#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Cryptographic support contracts and safe utility primitives.

/// Compares two byte slices without early exit on matching-length inputs.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    let min_len = if left.len() < right.len() {
        left.len()
    } else {
        right.len()
    };

    for index in 0..min_len {
        diff |= (left[index] ^ right[index]) as usize;
    }

    diff == 0
}

/// Fixed-size secret material with explicit clearing support.
#[derive(Debug, Eq, PartialEq)]
pub struct Secret<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> Secret<N> {
    /// Creates secret material from bytes.
    pub const fn new(bytes: [u8; N]) -> Self {
        Self { bytes }
    }

    /// Returns the secret bytes for cryptographic providers.
    pub const fn expose(&self) -> &[u8; N] {
        &self.bytes
    }

    /// Clears the secret bytes.
    pub fn clear(&mut self) {
        self.bytes.fill(0);
    }
}

impl<const N: usize> Drop for Secret<N> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Verification error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationError {
    /// The subject or signature did not match the pinned value.
    Invalid,
}

/// Signature verifier abstraction used by boot and package policy.
pub trait Verifier {
    /// Verifies a subject and signature.
    fn verify(&self, subject: &[u8], signature: &[u8]) -> Result<(), VerificationError>;
}

/// Verifier for pinned measurements or test vectors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PinnedVerifier<'a> {
    expected_subject: &'a [u8],
    expected_signature: &'a [u8],
}

impl<'a> PinnedVerifier<'a> {
    /// Creates a verifier for exact byte matches.
    pub const fn new(expected_subject: &'a [u8], expected_signature: &'a [u8]) -> Self {
        Self {
            expected_subject,
            expected_signature,
        }
    }
}

impl Verifier for PinnedVerifier<'_> {
    fn verify(&self, subject: &[u8], signature: &[u8]) -> Result<(), VerificationError> {
        if constant_time_eq(subject, self.expected_subject)
            && constant_time_eq(signature, self.expected_signature)
        {
            Ok(())
        } else {
            Err(VerificationError::Invalid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_checks_content_and_length() {
        assert!(constant_time_eq(b"txpos", b"txpos"));
        assert!(!constant_time_eq(b"txpos", b"txpoS"));
        assert!(!constant_time_eq(b"txpos", b"txpos!"));
    }

    #[test]
    fn pinned_verifier_requires_subject_and_signature() {
        let verifier = PinnedVerifier::new(b"kernel", b"signature");

        assert_eq!(verifier.verify(b"kernel", b"signature"), Ok(()));
        assert_eq!(
            verifier.verify(b"kernel", b"wrong"),
            Err(VerificationError::Invalid)
        );
    }
}
