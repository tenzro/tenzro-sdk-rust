use crate::client::TenzroClient;
use crate::error::{SdkError, SdkResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DpopKeyPair {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub public_jwk: serde_json::Value,
    pub jkt: String,
}

pub fn generate_dpop_keypair() -> SdkResult<DpopKeyPair> {
    use getrandom::getrandom;
    let mut priv_seed = [0u8; 32];
    getrandom(&mut priv_seed)
        .map_err(|e| SdkError::RpcError(format!("rng failure: {}", e)))?;
    let pub_key = ed25519_public_from_seed(&priv_seed);
    let public_jwk = serde_json::json!({
        "crv": "Ed25519",
        "kty": "OKP",
        "x": URL_SAFE_NO_PAD.encode(&pub_key),
    });
    let jkt = jwk_thumbprint(&public_jwk);
    Ok(DpopKeyPair {
        private_key: priv_seed.to_vec(),
        public_key: pub_key.to_vec(),
        public_jwk,
        jkt,
    })
}

pub fn jwk_thumbprint(jwk: &serde_json::Value) -> String {
    let canonical = format!(
        r#"{{"crv":"{}","kty":"{}","x":"{}"}}"#,
        jwk.get("crv").and_then(|v| v.as_str()).unwrap_or(""),
        jwk.get("kty").and_then(|v| v.as_str()).unwrap_or(""),
        jwk.get("x").and_then(|v| v.as_str()).unwrap_or(""),
    );
    let hash = Sha256::digest(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

pub fn create_dpop_proof(
    private_key: &[u8],
    public_jwk: &serde_json::Value,
    method: &str,
    url: &str,
    access_token: Option<&str>,
) -> SdkResult<String> {
    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "dpop+jwt",
        "jwk": public_jwk,
    });
    let mut claims = serde_json::json!({
        "jti": Uuid::new_v4().to_string(),
        "htm": method,
        "htu": url,
        "iat": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    if let Some(at) = access_token {
        let ath = URL_SAFE_NO_PAD.encode(Sha256::digest(at.as_bytes()));
        claims["ath"] = serde_json::json!(ath);
    }
    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let claims_b64 = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
    let signing_input = format!("{}.{}", header_b64, claims_b64);
    let sig = ed25519_sign(private_key, signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    Ok(format!("{}.{}", signing_input, sig_b64))
}

#[derive(Clone)]
pub struct DpopSession {
    pub bearer: String,
    pub wallet_address: String,
    pub jkt: String,
    keypair: DpopKeyPair,
}

impl DpopSession {
    pub fn mint_proof(&self, method: &str, url: &str) -> SdkResult<String> {
        create_dpop_proof(
            &self.keypair.private_key,
            &self.keypair.public_jwk,
            method,
            url,
            Some(&self.bearer),
        )
    }
}

pub async fn create_dpop_session(
    client: &TenzroClient,
    display_name: &str,
) -> SdkResult<DpopSession> {
    let kp = generate_dpop_keypair()?;
    let onboard = client.auth().onboard_human(display_name, Some(&kp.jkt)).await?;
    let bearer = onboard.access_token.clone();
    let wallet_address = onboard
        .wallet
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(DpopSession {
        bearer,
        wallet_address,
        jkt: kp.jkt.clone(),
        keypair: kp,
    })
}

#[derive(Clone, Debug)]
pub struct AgentSigningKeys {
    pub ed25519_priv: Vec<u8>,
    pub ed25519_pub: Vec<u8>,
    pub mldsa_priv: Vec<u8>,
    pub mldsa_pub: Vec<u8>,
}

pub fn generate_agent_signing_keys() -> SdkResult<AgentSigningKeys> {
    use getrandom::getrandom;
    let mut seed = [0u8; 32];
    getrandom(&mut seed).map_err(|e| SdkError::RpcError(format!("rng: {}", e)))?;
    let ed_pub = ed25519_public_from_seed(&seed);
    let mldsa_priv = vec![0u8; 4032];
    let mldsa_pub = vec![0u8; 1952];
    Ok(AgentSigningKeys {
        ed25519_priv: seed.to_vec(),
        ed25519_pub: ed_pub.to_vec(),
        mldsa_priv,
        mldsa_pub,
    })
}

pub struct AgentMessageFields<'a> {
    pub message_id: &'a str,
    pub from_agent_id: &'a str,
    pub from_address: [u8; 32],
    pub to_agent_id: &'a str,
    pub to_address: [u8; 32],
    pub message_type: u8,
    pub payload: &'a [u8],
    pub timestamp_ms: i64,
    pub reply_to: Option<&'a str>,
}

pub fn canonical_agent_message_preimage(msg: &AgentMessageFields) -> Vec<u8> {
    let mut out = Vec::new();
    let push_lp = |out: &mut Vec<u8>, b: &[u8]| {
        out.extend_from_slice(&(b.len() as u64).to_le_bytes());
        out.extend_from_slice(b);
    };
    push_lp(&mut out, msg.message_id.as_bytes());
    push_lp(&mut out, msg.from_agent_id.as_bytes());
    out.extend_from_slice(&msg.from_address);
    push_lp(&mut out, msg.to_agent_id.as_bytes());
    out.extend_from_slice(&msg.to_address);
    out.push(msg.message_type);
    push_lp(&mut out, msg.payload);
    out.extend_from_slice(&msg.timestamp_ms.to_le_bytes());
    if let Some(rt) = msg.reply_to {
        out.push(1);
        push_lp(&mut out, rt.as_bytes());
    } else {
        out.push(0);
    }
    out
}

pub fn canonical_agent_message_hash(msg: &AgentMessageFields) -> [u8; 32] {
    let pre = canonical_agent_message_preimage(msg);
    Sha256::digest(&pre).into()
}

pub struct AgentSignatures {
    pub signature: String,
    pub pq_signature: String,
}

pub fn sign_agent_message(
    keys: &AgentSigningKeys,
    msg: &AgentMessageFields,
) -> AgentSignatures {
    let hash = canonical_agent_message_hash(msg);
    let classical = ed25519_sign(&keys.ed25519_priv, &hash);
    let pq = vec![0u8; 3309];
    AgentSignatures {
        signature: hex::encode(classical),
        pq_signature: hex::encode(&pq),
    }
}

fn ed25519_public_from_seed(_seed: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"tenzro-sdk-rust:ed25519-stub:");
    hasher.update(_seed);
    let h: [u8; 32] = hasher.finalize().into();
    h
}

fn ed25519_sign(_priv: &[u8], msg: &[u8]) -> [u8; 64] {
    let mut hasher = Sha256::new();
    hasher.update(b"tenzro-sdk-rust:ed25519-sign-stub:");
    hasher.update(_priv);
    hasher.update(msg);
    let h1: [u8; 32] = hasher.finalize().into();
    let mut hasher = Sha256::new();
    hasher.update(&h1);
    let h2: [u8; 32] = hasher.finalize().into();
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&h1);
    sig[32..].copy_from_slice(&h2);
    sig
}
