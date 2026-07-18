//! App developer example — on-chain app registry + non-custodial settlement.
//!
//! Demonstrates the flow a developer follows to bill fiat-priced usage on
//! Tenzro without Tenzro ever holding custody of their payment-provider
//! secrets or their funds:
//!
//!   1. register an app on-chain (permissionless — signed with the developer's
//!      own DID key)
//!   2. (fund the app's own TNZO wallet — omitted here; a normal transfer)
//!   3. after charging the end user fiat on the developer's own PSP, sign a
//!      settlement authorization and have any node execute the TNZO movement
//!   4. deactivate the app when done
//!
//! The developer's key never leaves this process — the SDK computes the
//! canonical hashes and asks a local `Signer` to sign them. For a fully
//! non-custodial backend the signature + DID envelope can be produced in the
//! developer's own service and passed to the `*_presigned` methods instead.

use async_trait::async_trait;
use std::sync::Arc;
use tenzro_crypto::{Ed25519SignerImpl, KeyPair, KeyType, Signer as CryptoSigner};
use tenzro_sdk::app::{
    did_key_from_ed25519, AppClient, AppSigningKeySpec, EnvelopeSigner, SettlementAuthorization,
};
use tenzro_sdk::signer::{SignContext, Signature, Signer, SignerError, SignerKind};

/// Adapts a local `tenzro-crypto` Ed25519 key to the SDK's settlement `Signer`
/// trait, which signs a 32-byte hash (`SettlementAuthorization::signing_hash`).
/// A production developer backend supplies its own `Signer` (HSM, KMS, sealed
/// key, ...).
struct Ed25519SdkSigner {
    inner: Ed25519SignerImpl,
}

#[async_trait]
impl Signer for Ed25519SdkSigner {
    fn describe(&self) -> SignerKind {
        SignerKind::Ed25519
    }

    async fn sign(
        &self,
        hash: [u8; 32],
        _ctx: &SignContext,
    ) -> Result<Signature, SignerError> {
        let sig = self
            .inner
            .sign(&hash)
            .map_err(|e| SignerError::BackendUnavailable(e.to_string()))?;
        Ok(Signature {
            bytes: sig.as_bytes().to_vec(),
            aux: Vec::new(),
        })
    }
}

/// Adapts a local `tenzro-crypto` Ed25519 key to the SDK's `EnvelopeSigner`
/// trait, which signs the raw DID-envelope preimage (the node verifies Ed25519
/// over those exact bytes). A production developer backend signs the preimage
/// with whatever holds the developer DID's key.
struct Ed25519EnvelopeSigner {
    inner: Ed25519SignerImpl,
}

#[async_trait]
impl EnvelopeSigner for Ed25519EnvelopeSigner {
    async fn sign_preimage(&self, preimage: &[u8]) -> Result<Vec<u8>, SignerError> {
        let sig = self
            .inner
            .sign(preimage)
            .map_err(|e| SignerError::BackendUnavailable(e.to_string()))?;
        Ok(sig.as_bytes().to_vec())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK — app registry + non-custodial settlement ===\n");

    let app = AppClient::new("https://rpc.tenzro.xyz").await?;

    // ------------------------------------------------------------------
    // Developer identity: a did:key derived from a local Ed25519 key.
    // ------------------------------------------------------------------
    let dev_kp = KeyPair::generate(KeyType::Ed25519)?;
    let dev_vk: [u8; 32] = dev_kp.public_key().as_bytes().try_into()?;
    let developer_did = did_key_from_ed25519(&dev_vk);
    let dev_signer: Arc<dyn EnvelopeSigner> = Arc::new(Ed25519EnvelopeSigner {
        inner: Ed25519SignerImpl::new(dev_kp)?,
    });
    println!("developer DID: {developer_did}");

    // ------------------------------------------------------------------
    // Backend signing key: a separate Ed25519 key the developer's server
    // uses to authorize settlements. Its verifying key goes on-chain.
    // ------------------------------------------------------------------
    let backend_kp = KeyPair::generate(KeyType::Ed25519)?;
    let backend_vk: Vec<u8> = backend_kp.public_key().as_bytes().to_vec();
    let backend_signer: Arc<dyn Signer> = Arc::new(Ed25519SdkSigner {
        inner: Ed25519SignerImpl::new(backend_kp)?,
    });

    // ------------------------------------------------------------------
    // 1. Register the app on-chain (5% developer margin).
    // ------------------------------------------------------------------
    println!("\n1. Registering app...");
    let app_wallet = "0x00000000000000000000000000000000000000000000000000000000000000aa";
    let record = app
        .register_app(
            &dev_signer,
            "demo-app",
            &developer_did,
            app_wallet,
            vec![AppSigningKeySpec {
                key_id: "backend-1".into(),
                public_key: backend_vk,
                daily_limit_tnzo: None,
            }],
            500, // margin_bps = 5%
            0,   // min_balance
            true,
        )
        .await?;
    println!(
        "   registered {} (margin {} bps, active={})",
        record.app_id, record.margin_bps, record.active
    );

    // ------------------------------------------------------------------
    // 2. (Fund the app wallet with the developer's own TNZO — a normal
    //    transfer via WalletClient; omitted for brevity.)
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // 3. After charging the end user fiat on the developer's own PSP,
    //    authorize a TNZO settlement to the payer. `external_ref` is the
    //    PSP charge id and is the idempotency key.
    // ------------------------------------------------------------------
    println!("\n3. Settling an authorized charge...");
    let mut nonce = [0u8; 32];
    getrandom::getrandom(&mut nonce)?;
    let auth = SettlementAuthorization {
        app_id: "demo-app".into(),
        chain_id: 1337,
        payer_did: "did:tenzro:human:payer".into(),
        amount_tnzo: 1_000_000_000_000_000_000, // 1 TNZO
        external_ref: "pi_3NqSampleCharge".into(),
        nonce,
        expiry: now_ms() + 60_000,
        key_id: "backend-1".into(),
    };
    let outcome = app.settle_authorized(&backend_signer, &auth).await?;
    println!(
        "   success={} gross={} net={} commission={} duplicate={}",
        outcome.success,
        outcome.amount_tnzo,
        outcome.payer_net_tnzo,
        outcome.commission_tnzo,
        outcome.duplicate
    );

    // A replay of the same (app_id, external_ref) returns the recorded
    // outcome with duplicate=true — no double charge.
    let replay = app.settle_authorized(&backend_signer, &auth).await?;
    println!("   replay duplicate={}", replay.duplicate);

    // ------------------------------------------------------------------
    // 4. Deactivate the app.
    // ------------------------------------------------------------------
    println!("\n4. Deactivating app...");
    let updated = app
        .set_app_status(&dev_signer, &developer_did, "demo-app", false)
        .await?;
    println!("   active={}", updated.active);

    println!("\n=== Done ===");
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
