#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXFS metadata, checksums, and journal record primitives.

/// TXFS on-disk magic.
pub const TXFS_MAGIC: [u8; 8] = *b"TXFS0001";

/// Filesystem feature flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureFlags(u32);

impl FeatureFlags {
    /// Journaling enabled.
    pub const JOURNALING: Self = Self(1 << 0);
    /// Encryption enabled.
    pub const ENCRYPTION: Self = Self(1 << 1);
    /// Compression enabled.
    pub const COMPRESSION: Self = Self(1 << 2);
    /// Snapshots enabled.
    pub const SNAPSHOTS: Self = Self(1 << 3);

    /// Creates an empty flag set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns a new set containing another flag.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Raw bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns true when all flags in `other` are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// TXFS superblock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Superblock {
    /// Magic bytes.
    pub magic: [u8; 8],
    /// Filesystem format version.
    pub version: u32,
    /// Block size in bytes.
    pub block_size: u32,
    /// Total blocks.
    pub total_blocks: u64,
    /// Enabled features.
    pub features: FeatureFlags,
    /// Journal start block.
    pub journal_start: u64,
    /// Journal length in blocks.
    pub journal_blocks: u64,
    /// CRC32 over stable superblock fields.
    pub checksum: u32,
}

impl Superblock {
    /// Creates a superblock with a valid checksum.
    pub fn new(
        block_size: u32,
        total_blocks: u64,
        features: FeatureFlags,
        journal_start: u64,
        journal_blocks: u64,
    ) -> Self {
        let mut block = Self {
            magic: TXFS_MAGIC,
            version: 1,
            block_size,
            total_blocks,
            features,
            journal_start,
            journal_blocks,
            checksum: 0,
        };
        block.checksum = block.compute_checksum();
        block
    }

    /// Verifies the magic and checksum.
    pub fn verify(&self) -> bool {
        self.magic == TXFS_MAGIC && self.compute_checksum() == self.checksum
    }

    fn compute_checksum(&self) -> u32 {
        let mut crc = Crc32::new();
        crc.update(&self.magic);
        crc.update(&self.version.to_le_bytes());
        crc.update(&self.block_size.to_le_bytes());
        crc.update(&self.total_blocks.to_le_bytes());
        crc.update(&self.features.bits().to_le_bytes());
        crc.update(&self.journal_start.to_le_bytes());
        crc.update(&self.journal_blocks.to_le_bytes());
        crc.finish()
    }
}

/// Journal operation kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalOp {
    /// Begin a transaction.
    Begin,
    /// Write metadata or data block.
    Write,
    /// Commit a transaction.
    Commit,
    /// Abort a transaction.
    Abort,
}

/// TXFS journal record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JournalRecord {
    /// Transaction id.
    pub txid: u64,
    /// Operation.
    pub op: JournalOp,
    /// Affected block.
    pub block: u64,
    /// Payload checksum.
    pub payload_crc32: u32,
}

/// Streaming CRC32 implementation for TXFS checksums.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Crc32 {
    value: u32,
}

impl Crc32 {
    /// Creates a new CRC32 calculator.
    pub const fn new() -> Self {
        Self { value: 0xffff_ffff }
    }

    /// Updates the checksum with bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= *byte as u32;
            for _ in 0..8 {
                let mask = 0u32.wrapping_sub(self.value & 1);
                self.value = (self.value >> 1) ^ (0xedb8_8320 & mask);
            }
        }
    }

    /// Finishes and returns the checksum.
    pub const fn finish(self) -> u32 {
        !self.value
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        let mut crc = Crc32::new();
        crc.update(b"123456789");
        assert_eq!(crc.finish(), 0xcbf4_3926);
    }

    #[test]
    fn superblock_detects_tampering() {
        let mut block = Superblock::new(
            4096,
            1024,
            FeatureFlags::empty()
                .with(FeatureFlags::JOURNALING)
                .with(FeatureFlags::ENCRYPTION),
            8,
            16,
        );

        assert!(block.verify());
        block.total_blocks = 2048;
        assert!(!block.verify());
    }
}
