#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Capability and sandbox policy primitives.

/// A permission that can be granted to a subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Capability {
    /// Read filesystem objects.
    ReadFile = 0,
    /// Write filesystem objects.
    WriteFile = 1,
    /// Open network sockets.
    Network = 2,
    /// Create processes or threads.
    SpawnProcess = 3,
    /// Access display services.
    Display = 4,
    /// Access input devices.
    Input = 5,
    /// Request privileged kernel diagnostics.
    KernelDiagnostics = 6,
}

/// A compact capability set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CapabilitySet(u64);

impl CapabilitySet {
    /// Creates an empty capability set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Creates a set containing one capability.
    pub const fn single(capability: Capability) -> Self {
        Self(1u64 << capability as u8)
    }

    /// Returns a new set with the capability included.
    pub const fn with(self, capability: Capability) -> Self {
        Self(self.0 | (1u64 << capability as u8))
    }

    /// Returns true when the capability is present.
    pub const fn contains(self, capability: Capability) -> bool {
        (self.0 & (1u64 << capability as u8)) != 0
    }

    /// Returns true when all capabilities from another set are present.
    pub const fn contains_all(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    /// Raw representation used for audit records.
    pub const fn bits(self) -> u64 {
        self.0
    }
}

/// Sandbox profile assigned to an application or service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SandboxProfile {
    /// Stable subject identifier.
    pub subject_id: [u8; 16],
    /// Capabilities granted to the subject.
    pub capabilities: CapabilitySet,
    /// Maximum memory budget in bytes.
    pub max_memory_bytes: u64,
}

impl SandboxProfile {
    /// Returns true when a capability is granted.
    pub const fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(capability)
    }
}

/// A request checked by the policy engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessRequest {
    /// Requested capability.
    pub capability: Capability,
    /// Optional memory impact in bytes.
    pub memory_bytes: u64,
}

/// Policy denial reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityError {
    /// The requested capability is not present in the subject profile.
    MissingCapability,
    /// The request exceeds the subject memory budget.
    MemoryLimitExceeded,
}

/// Checks a request against a sandbox profile.
pub fn authorize(profile: &SandboxProfile, request: AccessRequest) -> Result<(), SecurityError> {
    if !profile.allows(request.capability) {
        return Err(SecurityError::MissingCapability);
    }

    if request.memory_bytes > profile.max_memory_bytes {
        return Err(SecurityError::MemoryLimitExceeded);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_denies_missing_capability() {
        let profile = SandboxProfile {
            subject_id: [1; 16],
            capabilities: CapabilitySet::single(Capability::ReadFile),
            max_memory_bytes: 4096,
        };

        let request = AccessRequest {
            capability: Capability::Network,
            memory_bytes: 1024,
        };

        assert_eq!(
            authorize(&profile, request),
            Err(SecurityError::MissingCapability)
        );
    }

    #[test]
    fn policy_accepts_least_privilege_request() {
        let profile = SandboxProfile {
            subject_id: [2; 16],
            capabilities: CapabilitySet::empty().with(Capability::ReadFile),
            max_memory_bytes: 4096,
        };

        let request = AccessRequest {
            capability: Capability::ReadFile,
            memory_bytes: 2048,
        };

        assert_eq!(authorize(&profile, request), Ok(()));
    }
}
