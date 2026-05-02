//! Tenzro Cross-VM (SVM-native) program — instruction builders.
//!
//! The Tenzro Cross-VM program is a native SVM program (no SBF ELF) that lets
//! SVM transactions initiate cross-VM token transfers. The SVM executor
//! recognizes [`TENZRO_CROSS_VM_PROGRAM_ID`] and dispatches to the native
//! handlers in `tenzro_vm::svm::cross_vm`.
//!
//! Authoritative source: `crates/tenzro-vm/src/svm/cross_vm.rs`. Constants here
//! MUST stay byte-identical to the workspace constants. The SDK ships its own
//! copy so downstream Rust consumers can use these without taking a dependency
//! on the heavy `tenzro-vm` crate.
//!
//! - **Program ID**: `SHA-256("tenzro/svm/program/cross_vm/v1")`
//!   - Hex: `918f858b6b0dd134e9a1fcb73002428c5197093e76e536badc60382bb9f8ac78`
//!   - Base58: `AoD3kebB2bYjLKyJtaqkyXqwJy4oQ949SnVhMwEYzGXR`
//!
//! - **Instruction discriminators**: 8-byte Anchor-style
//!   `SHA-256("global:<snake_case_name>")[..8]`.
//!
//! - **Instruction data layout**: `[ discriminator (8) | payload (n) ]`. All
//!   integers are little-endian; byte arrays are inlined.

/// Canonical Tenzro Cross-VM program ID (32 bytes).
pub const TENZRO_CROSS_VM_PROGRAM_ID: [u8; 32] = [
    0x91, 0x8f, 0x85, 0x8b, 0x6b, 0x0d, 0xd1, 0x34,
    0xe9, 0xa1, 0xfc, 0xb7, 0x30, 0x02, 0x42, 0x8c,
    0x51, 0x97, 0x09, 0x3e, 0x76, 0xe5, 0x36, 0xba,
    0xdc, 0x60, 0x38, 0x2b, 0xb9, 0xf8, 0xac, 0x78,
];

/// Hex encoding of [`TENZRO_CROSS_VM_PROGRAM_ID`] (lowercase, no 0x prefix).
pub const TENZRO_CROSS_VM_PROGRAM_ID_HEX: &str =
    "918f858b6b0dd134e9a1fcb73002428c5197093e76e536badc60382bb9f8ac78";

/// Base58 encoding of [`TENZRO_CROSS_VM_PROGRAM_ID`].
pub const TENZRO_CROSS_VM_PROGRAM_ID_BASE58: &str =
    "AoD3kebB2bYjLKyJtaqkyXqwJy4oQ949SnVhMwEYzGXR";

/// Domain string used to derive [`TENZRO_CROSS_VM_PROGRAM_ID`].
pub const PROGRAM_ID_DERIVATION_DOMAIN: &str = "tenzro/svm/program/cross_vm/v1";

/// 8-byte Anchor-style instruction discriminators.
///
/// Each is `SHA-256("global:<snake_case_name>")[..8]`.
pub mod discriminators {
    pub const BRIDGE_TO_EVM: [u8; 8] = [0x92, 0xa8, 0xa4, 0x5c, 0x33, 0x22, 0x5f, 0x25];
    pub const BRIDGE_FROM_EVM: [u8; 8] = [0x30, 0x38, 0x73, 0x32, 0x89, 0xf4, 0xcd, 0x75];
    pub const REGISTER_TOKEN_POINTER: [u8; 8] = [0x9a, 0x8e, 0x01, 0x39, 0x0f, 0x99, 0x45, 0x22];
    pub const TRANSFER_CROSS_VM: [u8; 8] = [0xbc, 0x68, 0x41, 0x68, 0xab, 0xa7, 0xab, 0xb9];
}

/// Payload sizes (excluding the 8-byte discriminator).
pub const BRIDGE_TO_EVM_PAYLOAD_SIZE: usize = 32 + 20 + 8 + 8; // 68
pub const BRIDGE_FROM_EVM_PAYLOAD_SIZE: usize = 32 + 32 + 8 + 8; // 80
pub const REGISTER_TOKEN_POINTER_PAYLOAD_SIZE: usize = 32 + 20 + 32; // 84
pub const TRANSFER_CROSS_VM_PAYLOAD_SIZE: usize = 32 + 1 + 32 + 8 + 8; // 81

/// VM type tags (matches `cross_vm_bridge::VM_TYPE_*`).
pub mod vm_types {
    pub const NATIVE: u8 = 0;
    pub const EVM: u8 = 1;
    pub const SVM: u8 = 2;
    pub const DAML: u8 = 3;
}

/// Encode `bridge_to_evm(mint, evm_dest, amount, nonce)`.
///
/// Burns `amount` of `mint` on SVM and credits the EVM-side balance for
/// `evm_dest`. Returns 76 bytes (`8 + 68`).
pub fn encode_bridge_to_evm(
    mint: &[u8; 32],
    evm_dest: &[u8; 20],
    amount: u64,
    nonce: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + BRIDGE_TO_EVM_PAYLOAD_SIZE);
    out.extend_from_slice(&discriminators::BRIDGE_TO_EVM);
    out.extend_from_slice(mint);
    out.extend_from_slice(evm_dest);
    out.extend_from_slice(&amount.to_le_bytes());
    out.extend_from_slice(&nonce.to_le_bytes());
    out
}

/// Encode `bridge_from_evm(mint, svm_dest, amount, nonce)`.
///
/// Mints `amount` of `mint` on SVM after the EVM side has burned. Returns 88
/// bytes (`8 + 80`).
pub fn encode_bridge_from_evm(
    mint: &[u8; 32],
    svm_dest: &[u8; 32],
    amount: u64,
    nonce: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + BRIDGE_FROM_EVM_PAYLOAD_SIZE);
    out.extend_from_slice(&discriminators::BRIDGE_FROM_EVM);
    out.extend_from_slice(mint);
    out.extend_from_slice(svm_dest);
    out.extend_from_slice(&amount.to_le_bytes());
    out.extend_from_slice(&nonce.to_le_bytes());
    out
}

/// Encode `register_token_pointer(mint, evm_token_address, token_id)`.
///
/// Registers a cross-VM pointer mapping. Returns 92 bytes (`8 + 84`).
pub fn encode_register_token_pointer(
    mint: &[u8; 32],
    evm_token_address: &[u8; 20],
    token_id: &[u8; 32],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + REGISTER_TOKEN_POINTER_PAYLOAD_SIZE);
    out.extend_from_slice(&discriminators::REGISTER_TOKEN_POINTER);
    out.extend_from_slice(mint);
    out.extend_from_slice(evm_token_address);
    out.extend_from_slice(token_id);
    out
}

/// Encode `transfer_cross_vm(mint, dest_vm, dest_address, amount, nonce)`.
///
/// Generic cross-VM transfer with runtime-selected destination VM. Returns 89
/// bytes (`8 + 81`).
pub fn encode_transfer_cross_vm(
    mint: &[u8; 32],
    dest_vm: u8,
    dest_address: &[u8; 32],
    amount: u64,
    nonce: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + TRANSFER_CROSS_VM_PAYLOAD_SIZE);
    out.extend_from_slice(&discriminators::TRANSFER_CROSS_VM);
    out.extend_from_slice(mint);
    out.push(dest_vm);
    out.extend_from_slice(dest_address);
    out.extend_from_slice(&amount.to_le_bytes());
    out.extend_from_slice(&nonce.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_id_hex_matches_bytes() {
        let hex = TENZRO_CROSS_VM_PROGRAM_ID_HEX;
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        assert_eq!(bytes, TENZRO_CROSS_VM_PROGRAM_ID);
    }

    #[test]
    fn bridge_to_evm_size() {
        let bytes = encode_bridge_to_evm(&[1u8; 32], &[2u8; 20], 1_000, 42);
        assert_eq!(bytes.len(), 8 + BRIDGE_TO_EVM_PAYLOAD_SIZE);
        assert_eq!(&bytes[..8], &discriminators::BRIDGE_TO_EVM);
    }

    #[test]
    fn bridge_from_evm_size() {
        let bytes = encode_bridge_from_evm(&[1u8; 32], &[2u8; 32], 1_000, 42);
        assert_eq!(bytes.len(), 8 + BRIDGE_FROM_EVM_PAYLOAD_SIZE);
    }

    #[test]
    fn register_token_pointer_size() {
        let bytes = encode_register_token_pointer(&[1u8; 32], &[2u8; 20], &[3u8; 32]);
        assert_eq!(bytes.len(), 8 + REGISTER_TOKEN_POINTER_PAYLOAD_SIZE);
    }

    #[test]
    fn transfer_cross_vm_size() {
        let bytes = encode_transfer_cross_vm(&[1u8; 32], vm_types::EVM, &[2u8; 32], 1_000, 42);
        assert_eq!(bytes.len(), 8 + TRANSFER_CROSS_VM_PAYLOAD_SIZE);
    }
}
