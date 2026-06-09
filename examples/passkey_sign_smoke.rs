//! Real-cryptography end-to-end test of `tenzro_signWithPasskey`.
//!
//! Constructs a valid WebAuthn assertion + ML-DSA-65 signature exactly
//! the way a real platform authenticator would, except the signing
//! material is software-generated so the test can run headless.
//!
//! Sequence:
//!   1. Generate P-256 keypair + ML-DSA-65 keypair (these are what a
//!      hardware authenticator would normally hold)
//!   2. Enroll a passkey-bound smart account via `tenzro_enrollPasskey`
//!   3. Pick a synthetic 32-byte UserOp hash
//!   4. Compute the WebAuthn challenge from the op hash
//!      (`base64url_no_pad(op_hash)`)
//!   5. Build clientDataJSON with the right type/challenge/origin
//!   6. Build authenticatorData with rpIdHash + UP+UV flags + signCount
//!   7. Sign the prehash `SHA-256(authData ‖ SHA-256(clientDataJSON))`
//!      with the P-256 key, and sign the raw op_hash with the ML-DSA
//!      key
//!   8. Submit to `tenzro_signWithPasskey` — expect verified=true
//!
//! Run:
//! ```bash
//! cargo run --example passkey_sign_smoke
//! ```

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use tenzro_crypto::p256::{P256KeyPair, P256Signer};
use tenzro_crypto::pq::MlDsaSigningKey;
use tenzro_sdk::config::SdkConfig;
use tenzro_sdk::passkey_client::{
    EnrollPasskeyParams, SignWithPasskeyParams,
};
use tenzro_sdk::TenzroClient;

fn build_client_data_json(challenge_b64url: &str, origin: &str) -> Vec<u8> {
    // Key order matches what the verifier expects when parsing
    // (it deserializes via serde, so order doesn't matter for verify;
    // but the bytes are hashed verbatim, so we keep a stable order).
    format!(
        r#"{{"type":"webauthn.get","challenge":"{}","origin":"{}","crossOrigin":false}}"#,
        challenge_b64url, origin,
    )
    .into_bytes()
}

fn build_authenticator_data(rp_id: &str, sign_count: u32) -> Vec<u8> {
    let rp_hash: [u8; 32] = Sha256::digest(rp_id.as_bytes()).into();
    let mut data = Vec::with_capacity(37);
    data.extend_from_slice(&rp_hash);
    // Flags: UP (0x01) + UV (0x04) = 0x05
    data.push(0x05);
    data.extend_from_slice(&sign_count.to_be_bytes());
    data
}

fn webauthn_prehash(authenticator_data: &[u8], client_data_json: &[u8]) -> [u8; 32] {
    let cdj_hash: [u8; 32] = Sha256::digest(client_data_json).into();
    let mut payload = Vec::with_capacity(authenticator_data.len() + 32);
    payload.extend_from_slice(authenticator_data);
    payload.extend_from_slice(&cdj_hash);
    Sha256::digest(&payload).into()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("TENZRO_RPC_URL")
        .unwrap_or_else(|_| "https://rpc.tenzro.network".to_string());
    println!("===== passkey SIGN smoke against {} =====", endpoint);

    let mut cfg = SdkConfig::testnet();
    cfg.endpoint = endpoint;
    let client = TenzroClient::connect(cfg).await?;
    let passkey = client.passkey_rpc();

    // 1. Generate signing material.
    let p256_kp = P256KeyPair::generate();
    let pq_kp = MlDsaSigningKey::generate();
    let pubkey_xy = p256_kp.public_key_bytes(); // [u8; 64]
    let pq_vk_hex = hex::encode(pq_kp.verifying_key_bytes());
    let credential_id = format!(
        "tenzro-sign-smoke-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
    .into_bytes();
    let cred_hex = hex::encode(&credential_id);
    let pk_hex = hex::encode(pubkey_xy);

    // 2. Enroll.
    println!("\n[1/3] enroll");
    let enroll = passkey
        .enroll(EnrollPasskeyParams {
            display_name: Some("sign-smoke".into()),
            passkey_public_key_hex: pk_hex,
            credential_id_hex: cred_hex,
            ml_dsa_public_key_hex: pq_vk_hex,
            salt: 0,
        })
        .await?;
    println!("  smart_account_address: {}", enroll.smart_account_address);

    // 3. Build a synthetic op hash that we'll "sign".
    let mut op_hash = [0u8; 32];
    for (i, b) in op_hash.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7);
    }
    let op_hash_hex = hex::encode(op_hash);
    println!("\n[2/3] construct WebAuthn assertion + ML-DSA signature");
    println!("  op_hash:               0x{}", op_hash_hex);

    // 4-7. Build the WebAuthn assertion exactly the way an authenticator
    // would, then sign it.
    let rp_id = "wallet.tenzro.network";
    let origin = "https://wallet.tenzro.network";
    let challenge_b64 = URL_SAFE_NO_PAD.encode(&op_hash);
    println!("  challenge_b64url:      {}", challenge_b64);

    let authenticator_data = build_authenticator_data(rp_id, 1);
    let client_data_json = build_client_data_json(&challenge_b64, origin);
    let prehash = webauthn_prehash(&authenticator_data, &client_data_json);
    let p256_signer = P256Signer::from_keypair(&p256_kp);
    let p256_sig = p256_signer.sign_prehash(&prehash);
    let ml_dsa_sig = pq_kp.sign(&op_hash);
    let ml_dsa_hex = hex::encode(&ml_dsa_sig);
    println!(
        "  P-256 signature:       {} bytes",
        p256_sig.as_bytes().len()
    );
    println!("  ML-DSA-65 signature:   {} bytes", ml_dsa_sig.len());

    // 8. Submit to the live RPC.
    println!("\n[3/3] tenzro_signWithPasskey");
    let assertion_json = serde_json::json!({
        "authenticator_data": authenticator_data,
        "client_data_json": client_data_json,
        "signature": p256_sig.as_bytes(),
        "user_handle": serde_json::Value::Null,
    });
    let sign = passkey
        .sign(SignWithPasskeyParams {
            account_address: enroll.smart_account_address.clone(),
            op_hash_hex,
            assertion: assertion_json,
            ml_dsa_signature_hex: Some(ml_dsa_hex),
        })
        .await?;
    println!("  verified: {}", sign.verified);
    println!("  validator: {}", sign.validator);

    if !sign.verified {
        eprintln!("\n✗ verification REJECTED");
        std::process::exit(1);
    }

    println!("\n✓ FULL HYBRID WEBAUTHN+ML-DSA SIGNATURE VERIFIED ON LIVE TESTNET");
    Ok(())
}
