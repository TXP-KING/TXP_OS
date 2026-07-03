#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Early boot contracts shared by the future UEFI loader and kernel.

/// Boot phase reached by the loader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootStage {
    /// Firmware services are still available.
    FirmwareEntry,
    /// Kernel image was located.
    KernelLocated,
    /// Kernel image measurement was recorded.
    KernelMeasured,
    /// Control is ready to move to the kernel entry point.
    ReadyForKernel,
}

/// Framebuffer details passed to the kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameBufferInfo {
    /// Physical base address of the framebuffer.
    pub base: u64,
    /// Buffer length in bytes.
    pub length: u64,
    /// Horizontal resolution.
    pub width: u32,
    /// Vertical resolution.
    pub height: u32,
    /// Bytes per scanline.
    pub stride: u32,
}

/// Boot information passed to the kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootInfo {
    /// Offset used for direct physical memory mapping.
    pub physical_memory_offset: u64,
    /// Start address of the firmware memory map.
    pub memory_map_start: u64,
    /// Length of the firmware memory map in bytes.
    pub memory_map_len: u64,
    /// Optional ACPI RSDP address.
    pub rsdp_addr: Option<u64>,
    /// Optional framebuffer information.
    pub framebuffer: Option<FrameBufferInfo>,
}

impl BootInfo {
    /// Returns true when the boot information has the minimum kernel inputs.
    pub const fn is_minimal_valid(&self) -> bool {
        self.memory_map_start != 0 && self.memory_map_len != 0
    }
}

/// Measurement captured by the loader before kernel transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootMeasurement {
    /// Component digest. The digest algorithm is selected by the boot policy.
    pub digest: [u8; 32],
    /// Monotonic measurement index.
    pub index: u32,
}

/// Loader errors that must fail closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootError {
    /// Required firmware data was absent or malformed.
    InvalidFirmwareState,
    /// Kernel image verification failed.
    KernelVerificationFailed,
    /// Memory map could not be trusted.
    InvalidMemoryMap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_boot_info_requires_memory_map() {
        let missing = BootInfo {
            physical_memory_offset: 0,
            memory_map_start: 0,
            memory_map_len: 4096,
            rsdp_addr: None,
            framebuffer: None,
        };

        let valid = BootInfo {
            memory_map_start: 0x1000,
            ..missing
        };

        assert!(!missing.is_minimal_valid());
        assert!(valid.is_minimal_valid());
    }
}
