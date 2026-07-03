# Security Notes

The initial code follows these rules:

- Safe Rust only; unsafe code is denied at the workspace level.
- Fixed-capacity structures are used in early-kernel crates to avoid hidden
  allocation.
- Access checks use explicit capabilities and deny missing permissions.
- Integrity measurements fail closed when baselines differ.
- TXVault does not implement a home-grown cipher. It defines a provider
  contract so a reviewed AES-256-GCM or XChaCha20-Poly1305 implementation can
  be wired in later.
- Checksums in TXFS are for corruption detection, not cryptographic integrity.

Milestone 1 should add signed boot artifacts, measured boot policy, and a real
cryptographic provider selected from reviewed implementations.

