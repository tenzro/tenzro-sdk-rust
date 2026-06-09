//! End-to-end synthetic-key smoke test for the passkey-first wallet RPCs.
//!
//! Drives the full happy path against a live node:
//!   1. enroll a synthetic passkey + ML-DSA-65 → smart account created
//!   2. get_smart_account → account materialized + WebAuthn validator installed
//!   3. add_guardian → guardian quorum config installed
//!   4. initiate_recovery → ceremony opened
//!   5. submit_recovery_signature × threshold → quorum reached
//!   6. finalize_recovery → account rotated to new passkey
//!   7. grant_session_key → session key installed
//!   8. revoke_session_key → session key removed
//!   9. set_spending_limit → limits installed
//!  10. add_hardware_signer → hardware validator installed
//!
//! Run against the live testnet:
//!
//! ```bash
//! cargo run --example passkey_e2e_smoke
//! ```
//!
//! The signing material is synthetic — software P-256 + software ML-DSA-65
//! generation, not a real platform authenticator. The point is to exercise
//! the RPC handlers and persistence end-to-end; the on-chain WebAuthn /
//! validator pipelines are covered by their own unit tests.

use std::time::{SystemTime, UNIX_EPOCH};
use tenzro_crypto::p256::P256KeyPair;
use tenzro_crypto::pq::MlDsaSigningKey;
use tenzro_crypto::{KeyPair, KeyType};
use tenzro_sdk::config::SdkConfig;
use tenzro_sdk::passkey_client::{
    AddGuardianParams, AddHardwareSignerParams, EnrollPasskeyParams, FinalizeRecoveryParams,
    GrantSessionKeyParams, InitiateRecoveryParams, RevokeSessionKeyParams,
    SetSpendingLimitParams, SubmitRecoverySignatureParams,
};
use tenzro_sdk::TenzroClient;

fn random_credential_id() -> Vec<u8> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut id = b"tenzro-smoke-".to_vec();
    id.extend_from_slice(&nanos.to_le_bytes());
    id
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::var("TENZRO_RPC_URL")
        .unwrap_or_else(|_| "https://rpc.tenzro.network".to_string());
    println!("===== passkey E2E smoke against {} =====", endpoint);

    let mut cfg = SdkConfig::testnet();
    cfg.endpoint = endpoint;
    let client = TenzroClient::connect(cfg).await?;
    let passkey = client.passkey_rpc();

    // -----------------------------------------------------------------
    // 1. enroll
    // -----------------------------------------------------------------
    println!("\n[1/10] enroll");
    let kp = P256KeyPair::generate();
    let pubkey_xy = kp.public_key_bytes(); // [u8; 64]
    let pq = MlDsaSigningKey::generate();
    let pq_vk_hex = hex::encode(pq.verifying_key_bytes());
    let credential_id = random_credential_id();
    let cred_hex = hex::encode(&credential_id);
    let pk_hex = hex::encode(pubkey_xy);

    let enroll = passkey
        .enroll(EnrollPasskeyParams {
            display_name: Some("smoke-test".into()),
            passkey_public_key_hex: pk_hex.clone(),
            credential_id_hex: cred_hex.clone(),
            ml_dsa_public_key_hex: pq_vk_hex.clone(),
            salt: 0,
        })
        .await?;
    println!("  did:                   {}", enroll.did);
    println!("  smart_account_address: {}", enroll.smart_account_address);
    println!("  credential_id_hex:     {}", enroll.credential_id_hex);
    println!("  webauthn_validator:    {}", enroll.webauthn_validator_address);
    println!("  installed_validators:  {:?}", enroll.installed_validators);
    let account_address = enroll.smart_account_address.clone();

    // -----------------------------------------------------------------
    // 2. get_smart_account
    // -----------------------------------------------------------------
    println!("\n[2/10] get_smart_account");
    let sa = passkey.get_smart_account(&account_address).await?;
    println!("  address:               {}", sa.address);
    println!("  is_deployed:           {}", sa.is_deployed);
    println!("  validators:            {}", sa.installed_validators.len());
    for v in &sa.installed_validators {
        println!("    - {} (type {} prio {})", v.module_address, v.type_id, v.priority);
    }
    assert!(
        !sa.installed_validators.is_empty(),
        "WebAuthnValidator should be installed after enroll"
    );

    // -----------------------------------------------------------------
    // 3. add_guardian
    // -----------------------------------------------------------------
    println!("\n[3/10] add_guardian");
    let guardian_ed = KeyPair::generate(KeyType::Ed25519).unwrap();
    let guardian_pq = MlDsaSigningKey::generate();
    let add = passkey
        .add_guardian(AddGuardianParams {
            account_address: account_address.clone(),
            guardian_ed25519_pubkey_hex: hex::encode(guardian_ed.public_key().as_bytes()),
            guardian_ml_dsa_pubkey_hex: hex::encode(guardian_pq.verifying_key_bytes()),
            label: Some("smoke-guardian-1".into()),
            threshold: Some(1),
        })
        .await?;
    println!(
        "  guardian_count: {}, threshold: {}",
        add.guardian_count, add.threshold
    );

    // -----------------------------------------------------------------
    // 4. initiate_recovery
    // -----------------------------------------------------------------
    println!("\n[4/10] initiate_recovery");
    let new_kp = P256KeyPair::generate();
    let new_pq = MlDsaSigningKey::generate();
    let new_cred = random_credential_id();
    let init = passkey
        .initiate_recovery(InitiateRecoveryParams {
            account_address: account_address.clone(),
            new_passkey_public_key_hex: hex::encode(new_kp.public_key_bytes()),
            new_credential_id_hex: hex::encode(&new_cred),
            new_ml_dsa_public_key_hex: hex::encode(new_pq.verifying_key_bytes()),
            ttl_secs: Some(3600),
        })
        .await?;
    println!("  recovery_id:           {}", init.recovery_id);
    println!("  recovery_op_hash_hex:  {}", init.recovery_op_hash_hex);
    println!(
        "  guardians_required:    {}/{}",
        init.guardians_required, init.guardians_total
    );

    // -----------------------------------------------------------------
    // 5. submit_recovery_signature (× threshold)
    // -----------------------------------------------------------------
    println!("\n[5/10] submit_recovery_signature");
    // Compose a 64-byte Ed25519 signature || 3309-byte ML-DSA-65 signature.
    // The signature bytes don't have to verify on this happy-path smoke —
    // the RPC stores the bytes and counts them; cryptographic enforcement
    // would happen in `finalize_recovery` when the validator actually
    // verifies the composite signatures. Today the handler stores them
    // and considers quorum reached when N entries land; that matches what
    // we want to smoke at this layer.
    let ed_sig = vec![0xABu8; 64];
    let pq_sig = vec![0xCDu8; 3309];
    let composite_hex = format!("{}{}", hex::encode(&ed_sig), hex::encode(&pq_sig));
    let sub = passkey
        .submit_recovery_signature(SubmitRecoverySignatureParams {
            recovery_id: init.recovery_id.clone(),
            guardian_index: 0,
            composite_signature_hex: composite_hex,
        })
        .await?;
    println!(
        "  collected: {}/{}, quorum_reached: {}",
        sub.guardian_signatures_collected, sub.guardians_required, sub.quorum_reached
    );

    // -----------------------------------------------------------------
    // 6. finalize_recovery
    // -----------------------------------------------------------------
    println!("\n[6/10] finalize_recovery");
    let fin = passkey
        .finalize_recovery(FinalizeRecoveryParams {
            recovery_id: init.recovery_id.clone(),
        })
        .await?;
    println!("  account_address:       {}", fin.account_address);
    println!("  new_credential_id_hex: {}", fin.new_credential_id_hex);
    println!("  installed_validators:  {}", fin.installed_validators.len());

    // -----------------------------------------------------------------
    // 7. grant_session_key
    // -----------------------------------------------------------------
    println!("\n[7/10] grant_session_key");
    let session_kp = KeyPair::generate(KeyType::Ed25519).unwrap();
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let grant = passkey
        .grant_session_key(GrantSessionKeyParams {
            account_address: account_address.clone(),
            session_pubkey_hex: hex::encode(session_kp.public_key().as_bytes()),
            allowed_selectors_hex: vec![
                "a9059cbb".into(), // ERC-20 transfer(address,uint256)
                "095ea7b3".into(), // ERC-20 approve(address,uint256)
            ],
            allowed_targets: vec![],
            max_value_per_call_wei: Some("1000000000000000000".into()), // 1 ETH cap
            max_total_value_wei: Some("10000000000000000000".into()),   // 10 ETH lifetime
            valid_after_unix: now_unix,
            valid_until_unix: now_unix + 30 * 86400, // 30 days
            label: Some("smoke-agent-key".into()),
        })
        .await?;
    println!("  session_pubkey_hex:    {}", grant.session_pubkey_hex);
    println!("  valid_until_unix:      {}", grant.valid_until_unix);

    // -----------------------------------------------------------------
    // 8. revoke_session_key
    // -----------------------------------------------------------------
    println!("\n[8/10] revoke_session_key");
    let rev = passkey
        .revoke_session_key(RevokeSessionKeyParams {
            account_address: account_address.clone(),
        })
        .await?;
    println!("  revoked: {}", rev.revoked);

    // -----------------------------------------------------------------
    // 9. set_spending_limit
    // -----------------------------------------------------------------
    println!("\n[9/10] set_spending_limit");
    let auth_pubkey = KeyPair::generate(KeyType::Ed25519).unwrap();
    let limit = passkey
        .set_spending_limit(SetSpendingLimitParams {
            account_address: account_address.clone(),
            per_tx_cap_wei: "5000000000000000000".into(), // 5 ETH per tx
            daily_cap_wei: "20000000000000000000".into(),  // 20 ETH per day
            authenticator_pubkey_hex: hex::encode(auth_pubkey.public_key().as_bytes()),
        })
        .await?;
    println!("  per_tx:  {}", limit.per_tx_cap_wei);
    println!("  daily:   {}", limit.daily_cap_wei);

    // -----------------------------------------------------------------
    // 10. add_hardware_signer
    // -----------------------------------------------------------------
    println!("\n[10/10] add_hardware_signer");
    // Synthetic Ledger pubkey: 33-byte SEC1 compressed secp256k1.
    let ledger_pk = {
        let mut v = vec![0x02u8];
        v.extend(vec![0xEEu8; 32]);
        v
    };
    let hw = passkey
        .add_hardware_signer(AddHardwareSignerParams {
            account_address: account_address.clone(),
            device_kind: "ledger".into(),
            public_key_hex: hex::encode(&ledger_pk),
            required_always: false,
            required_above_wei: Some("1000000000000000000".into()),
            label: Some("smoke-ledger".into()),
        })
        .await?;
    println!("  device_kind: {}", hw.device_kind);
    println!(
        "  validator_module_address: {}",
        hw.validator_module_address
    );

    // -----------------------------------------------------------------
    // Final verification
    // -----------------------------------------------------------------
    println!("\n===== Final state =====");
    let final_state = passkey.get_smart_account(&account_address).await?;
    println!("address:    {}", final_state.address);
    println!("validators: {}", final_state.installed_validators.len());
    for v in &final_state.installed_validators {
        println!("  - {} (type {} prio {})", v.module_address, v.type_id, v.priority);
    }

    let accounts_list = passkey.list_smart_accounts().await?;
    println!("\ntotal smart accounts on node: {}", accounts_list.count);

    println!("\n✓ ALL 10 RPCs SUCCEEDED");
    Ok(())
}
