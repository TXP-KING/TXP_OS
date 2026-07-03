#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXVault encrypted-storage plumbing.

use txpos_crypto::Secret;

/// Authenticated encryption provider contract.
pub trait Aead {
    /// Provider error.
    type Error;

    /// Seals plaintext into the caller-supplied output buffer.
    fn seal(
        &self,
        nonce: &[u8; 24],
        aad: &[u8],
        plaintext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Opens ciphertext into the caller-supplied output buffer.
    fn open(
        &self,
        nonce: &[u8; 24],
        aad: &[u8],
        ciphertext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;
}

/// Key state in the vault.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyState {
    /// Key is ready for use.
    Active,
    /// Key is retired and should only decrypt existing records.
    Retired,
    /// Key is revoked and must not be used.
    Revoked,
}

/// A vault key slot.
#[derive(Debug, Eq, PartialEq)]
pub struct KeySlot<const N: usize> {
    /// Key identifier.
    pub id: [u8; 16],
    /// Secret key bytes.
    pub secret: Secret<N>,
    /// Key lifecycle state.
    pub state: KeyState,
}

/// Record metadata authenticated with encrypted payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordMeta {
    /// Record identifier.
    pub id: [u8; 16],
    /// Format version.
    pub version: u32,
    /// Encryption nonce.
    pub nonce: [u8; 24],
}

impl RecordMeta {
    /// Serializes stable associated data for authenticated encryption.
    pub fn associated_data(&self) -> [u8; 20] {
        let mut aad = [0u8; 20];
        aad[..16].copy_from_slice(&self.id);
        aad[16..].copy_from_slice(&self.version.to_le_bytes());
        aad
    }
}

/// Vault wrapper around an authenticated encryption provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vault<C> {
    cipher: C,
}

impl<C: Aead> Vault<C> {
    /// Creates a vault around a cipher provider.
    pub const fn new(cipher: C) -> Self {
        Self { cipher }
    }

    /// Seals a record.
    pub fn seal_record(
        &self,
        meta: &RecordMeta,
        plaintext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, C::Error> {
        self.cipher
            .seal(&meta.nonce, &meta.associated_data(), plaintext, out)
    }

    /// Opens a record.
    pub fn open_record(
        &self,
        meta: &RecordMeta,
        ciphertext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, C::Error> {
        self.cipher
            .open(&meta.nonce, &meta.associated_data(), ciphertext, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestAead;

    impl Aead for TestAead {
        type Error = ();

        fn seal(
            &self,
            nonce: &[u8; 24],
            aad: &[u8],
            plaintext: &[u8],
            out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            if out.len() < plaintext.len() {
                return Err(());
            }

            for (index, byte) in plaintext.iter().enumerate() {
                out[index] = byte ^ nonce[0] ^ aad[0];
            }

            Ok(plaintext.len())
        }

        fn open(
            &self,
            nonce: &[u8; 24],
            aad: &[u8],
            ciphertext: &[u8],
            out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            self.seal(nonce, aad, ciphertext, out)
        }
    }

    #[test]
    fn vault_round_trips_through_provider_contract() {
        let vault = Vault::new(TestAead);
        let meta = RecordMeta {
            id: [9; 16],
            version: 1,
            nonce: [3; 24],
        };
        let mut sealed = [0u8; 16];
        let mut opened = [0u8; 16];

        let sealed_len = vault
            .seal_record(&meta, b"secret", &mut sealed)
            .expect("seal");
        let opened_len = vault
            .open_record(&meta, &sealed[..sealed_len], &mut opened)
            .expect("open");

        assert_eq!(&opened[..opened_len], b"secret");
    }
}
