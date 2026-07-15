//! Full ERC-4337 v0.8 UserOp signed with a passkey, submitted to the
//! live EntryPoint.
//!
//! This is the production flow:
//!   1. Generate passkey + ML-DSA-65 keypair, enroll the smart account
//!   2. Build a UserOperation
//!   3. Compute its EIP-712 hash (matching `UserOperation::hash`)
//!   4. Construct a WebAuthn assertion over `base64url_no_pad(op_hash)`
//!      and sign with both P-256 and ML-DSA-65
//!   5. Pack the hybrid signature into `userOp.signature`
//!   6. Call `eth_sendUserOperation([userOp, entryPoint])`
//!   7. Expect the validator chain to accept and return the op hash
//!
//! Run:
//! ```bash
//! cargo run --example passkey_userop_smoke
//! ```

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::json;
use sha2::{Digest, Sha256};
use tenzro_crypto::keccak256;
use tenzro_crypto::p256::{P256KeyPair, P256Signer};
use tenzro_crypto::pq::MlDsaSigningKey;
use tenzro_sdk::config::SdkConfig;
use tenzro_sdk::passkey_client::EnrollPasskeyParams;
use tenzro_sdk::TenzroClient;

// EIP-712 type hash for UserOperation v0.8 — copied bit-for-bit from
// `tenzro_vm::account_abstraction::user_operation_type_hash` so the
// smoke is independent of the public API.
fn user_operation_type_hash() -> [u8; 32] {
    let s = "UserOperation(address sender,uint256 nonce,address factory,bytes factoryData,bytes callData,uint256 callGasLimit,uint256 verificationGasLimit,uint256 preVerificationGas,uint256 maxFeePerGas,uint256 maxPriorityFeePerGas,address paymaster,uint256 paymasterVerificationGasLimit,uint256 paymasterPostOpGasLimit,bytes paymasterData)";
    let h = keccak256(s.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

// EIP-712 domain separator: keccak256("EIP712Domain(...)" + "Tenzro" + version 1
// + chain_id + entry_point).
//
// We mirror the implementation in `tenzro_vm::account_abstraction::
// eip712_domain_separator`. The smoke just needs the same bytes.
fn eip712_domain_separator(chain_id: u64, entry_point: &[u8]) -> [u8; 32] {
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
            .as_ref(),
    );
    let name_hash = keccak256(b"ERC4337".as_ref());
    let version_hash = keccak256(b"1".as_ref());
    let mut data = Vec::with_capacity(160);
    data.extend_from_slice(type_hash.as_bytes());
    data.extend_from_slice(name_hash.as_bytes());
    data.extend_from_slice(version_hash.as_bytes());
    let mut chain_buf = [0u8; 32];
    chain_buf[24..].copy_from_slice(&chain_id.to_be_bytes());
    data.extend_from_slice(&chain_buf);
    let mut addr_buf = [0u8; 32];
    let n = entry_point.len().min(20);
    addr_buf[32 - n..].copy_from_slice(&entry_point[..n]);
    data.extend_from_slice(&addr_buf);
    let h = keccak256(&data);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

fn encode_address(addr: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let n = addr.len().min(20);
    buf[32 - n..].copy_from_slice(&addr[..n]);
    buf
}

fn encode_u64_uint256(v: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&v.to_be_bytes());
    buf
}

fn encode_u128_uint256(v: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&v.to_be_bytes());
    buf
}

fn kbytes(b: &[u8]) -> [u8; 32] {
    let h = keccak256(b);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

#[derive(Default)]
struct UserOp {
    sender: Vec<u8>,
    nonce: u64,
    factory: Vec<u8>,
    factory_data: Vec<u8>,
    call_data: Vec<u8>,
    call_gas_limit: u64,
    verification_gas_limit: u64,
    pre_verification_gas: u64,
    max_fee_per_gas: u128,
    max_priority_fee_per_gas: u128,
    paymaster: Vec<u8>,
    paymaster_verification_gas_limit: u64,
    paymaster_post_op_gas_limit: u64,
    paymaster_data: Vec<u8>,
}

impl UserOp {
    fn struct_hash(&self) -> [u8; 32] {
        let mut data = Vec::with_capacity(32 * 15);
        data.extend_from_slice(&user_operation_type_hash());
        data.extend_from_slice(&encode_address(&self.sender));
        data.extend_from_slice(&encode_u64_uint256(self.nonce));
        data.extend_from_slice(&encode_address(&self.factory));
        data.extend_from_slice(&kbytes(&self.factory_data));
        data.extend_from_slice(&kbytes(&self.call_data));
        data.extend_from_slice(&encode_u64_uint256(self.call_gas_limit));
        data.extend_from_slice(&encode_u64_uint256(self.verification_gas_limit));
        data.extend_from_slice(&encode_u64_uint256(self.pre_verification_gas));
        data.extend_from_slice(&encode_u128_uint256(self.max_fee_per_gas));
        data.extend_from_slice(&encode_u128_uint256(self.max_priority_fee_per_gas));
        data.extend_from_slice(&encode_address(&self.paymaster));
        data.extend_from_slice(&encode_u64_uint256(self.paymaster_verification_gas_limit));
        data.extend_from_slice(&encode_u64_uint256(self.paymaster_post_op_gas_limit));
        data.extend_from_slice(&kbytes(&self.paymaster_data));
        let h = keccak256(&data);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    }

    fn hash(&self, chain_id: u64, entry_point: &[u8]) -> [u8; 32] {
        let dsep = eip712_domain_separator(chain_id, entry_point);
        let sh = self.struct_hash();
        let mut buf = Vec::with_capacity(66);
        buf.push(0x19);
        buf.push(0x01);
        buf.extend_from_slice(&dsep);
        buf.extend_from_slice(&sh);
        let h = keccak256(&buf);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    }
}

fn build_authenticator_data(rp_id: &str, sign_count: u32) -> Vec<u8> {
    let rp_hash: [u8; 32] = Sha256::digest(rp_id.as_bytes()).into();
    let mut data = Vec::with_capacity(37);
    data.extend_from_slice(&rp_hash);
    data.push(0x05); // UP | UV
    data.extend_from_slice(&sign_count.to_be_bytes());
    data
}

fn build_client_data_json(challenge_b64url: &str, origin: &str) -> Vec<u8> {
    format!(
        r#"{{"type":"webauthn.get","challenge":"{}","origin":"{}","crossOrigin":false}}"#,
        challenge_b64url, origin
    )
    .into_bytes()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("TENZRO_RPC_URL")
        .unwrap_or_else(|_| "https://rpc.tenzro.xyz".to_string());
    println!("===== ERC-4337 v0.8 UserOp + passkey smoke =====");
    println!("endpoint: {}", endpoint);

    let mut cfg = SdkConfig::testnet();
    cfg.endpoint = endpoint;
    let client = TenzroClient::connect(cfg).await?;
    let passkey = client.passkey_rpc();

    // ── 1. Generate keys + enroll ──
    let p256_kp = P256KeyPair::generate();
    let pq_kp = MlDsaSigningKey::generate();
    let cred = format!(
        "userop-smoke-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
    .into_bytes();
    println!("\n[1/4] enroll");
    let enroll = passkey
        .enroll(EnrollPasskeyParams {
            display_name: Some("userop-smoke".into()),
            passkey_public_key_hex: hex::encode(p256_kp.public_key_bytes()),
            credential_id_hex: hex::encode(&cred),
            ml_dsa_public_key_hex: hex::encode(pq_kp.verifying_key_bytes()),
            salt: 0,
        })
        .await?;
    println!("  smart_account: {}", enroll.smart_account_address);
    let sa_hex = enroll.smart_account_address.trim_start_matches("0x").to_string();
    let sa_bytes = hex::decode(&sa_hex)?;

    // ── 2. Build a no-op UserOperation ──
    println!("\n[2/4] build UserOp");
    let entry_point_hex = "4337084d9e255ff0702461cf8895ce9e3b5ff108";
    let entry_point_bytes = hex::decode(entry_point_hex)?;
    let chain_id: u64 = 1337;
    let user_op = UserOp {
        sender: sa_bytes.clone(),
        // 2-D nonce (EIP-4337 v0.8): key = 0, seq = 0 packs to uint256 zero
        nonce: 0,
        call_data: Vec::new(), // no inner call — just exercise validation
        call_gas_limit: 100_000,
        verification_gas_limit: 500_000,
        pre_verification_gas: 40_000,
        max_fee_per_gas: 1_500_000_000,
        max_priority_fee_per_gas: 1_500_000_000,
        ..Default::default()
    };
    let op_hash = user_op.hash(chain_id, &entry_point_bytes);
    println!("  op_hash: 0x{}", hex::encode(op_hash));

    // ── 3. Build hybrid signature ──
    println!("\n[3/4] sign op hash (passkey + ML-DSA-65)");
    let rp_id = "wallet.tenzro.xyz";
    let origin = "https://wallet.tenzro.xyz";
    let challenge = URL_SAFE_NO_PAD.encode(op_hash);
    let auth_data = build_authenticator_data(rp_id, 1);
    let client_data = build_client_data_json(&challenge, origin);
    let cdj_hash: [u8; 32] = Sha256::digest(&client_data).into();
    let mut payload = Vec::with_capacity(auth_data.len() + 32);
    payload.extend_from_slice(&auth_data);
    payload.extend_from_slice(&cdj_hash);
    let prehash: [u8; 32] = Sha256::digest(&payload).into();
    let p256_signer = P256Signer::from_keypair(&p256_kp);
    let p256_sig = p256_signer.sign_prehash(&prehash);
    let ml_dsa_sig = pq_kp.sign(&op_hash);

    // Pack into HybridWebAuthnSignature bincode (matches
    // tenzro_vm::HybridWebAuthnSignature::encode).
    #[derive(serde::Serialize)]
    struct WebAuthnAssertionWire {
        authenticator_data: Vec<u8>,
        client_data_json: Vec<u8>,
        signature: Vec<u8>,
        user_handle: Option<Vec<u8>>,
    }
    #[derive(serde::Serialize)]
    struct HybridSig {
        assertion: WebAuthnAssertionWire,
        ml_dsa_signature: Vec<u8>,
    }
    let hybrid = HybridSig {
        assertion: WebAuthnAssertionWire {
            authenticator_data: auth_data,
            client_data_json: client_data,
            signature: p256_sig.as_bytes().to_vec(),
            user_handle: None,
        },
        ml_dsa_signature: ml_dsa_sig,
    };
    let signature_bytes = bincode::serialize(&hybrid)?;
    println!("  hybrid signature: {} bytes", signature_bytes.len());

    // ── 4. Submit ──
    println!("\n[4/4] eth_sendUserOperation");
    let payload = json!([{
        "sender": format!("0x{}", sa_hex),
        "nonce": format!("0x{:x}", user_op.nonce),
        "factory": "0x",
        "factoryData": "0x",
        "callData": "0x",
        "callGasLimit": format!("0x{:x}", user_op.call_gas_limit),
        "verificationGasLimit": format!("0x{:x}", user_op.verification_gas_limit),
        "preVerificationGas": format!("0x{:x}", user_op.pre_verification_gas),
        "maxFeePerGas": format!("0x{:x}", user_op.max_fee_per_gas),
        "maxPriorityFeePerGas": format!("0x{:x}", user_op.max_priority_fee_per_gas),
        "paymaster": "0x",
        "paymasterVerificationGasLimit": "0x0",
        "paymasterPostOpGasLimit": "0x0",
        "paymasterData": "0x",
        "signature": format!("0x{}", hex::encode(&signature_bytes)),
    }, format!("0x{}", entry_point_hex)]);

    let rpc_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_sendUserOperation",
        "params": payload,
    });

    let url = std::env::var("TENZRO_RPC_URL")
        .unwrap_or_else(|_| "https://rpc.tenzro.xyz".to_string());
    let http = reqwest::Client::new();
    let resp: serde_json::Value = http
        .post(&url)
        .json(&rpc_req)
        .send()
        .await?
        .json()
        .await?;
    println!("  response: {}", serde_json::to_string_pretty(&resp)?);

    if resp.get("error").is_some() {
        eprintln!("\n✗ UserOp REJECTED");
        std::process::exit(1);
    }

    if let Some(result) = resp.get("result").and_then(|v| v.as_str()) {
        println!("\n✓ UserOp accepted — userOpHash={}", result);
    }

    Ok(())
}
