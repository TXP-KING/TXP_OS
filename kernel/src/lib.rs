#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXPOS kernel milestone-0 initialization facade.

use txpos_memory::{BumpFrameAllocator, FrameRange, MemoryError, PAGE_SIZE, PhysFrame};
use txpos_scheduler::{Priority, Scheduler, SchedulerError, TaskId};
use txpos_security::{AccessRequest, SandboxProfile, SecurityError, authorize};
use txpos_txshield::{Component, Measurement, MeasurementError, MeasurementLog};

/// Kernel initialization configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelConfig {
    /// First usable physical memory address.
    pub memory_start: usize,
    /// Usable physical memory length in bytes.
    pub memory_len: usize,
    /// Expected kernel measurement digest.
    pub kernel_digest: [u8; 32],
}

/// Kernel state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelState {
    /// Constructed but no tasks have run.
    Initialized,
    /// Scheduler has selected at least one task.
    Running,
}

/// Kernel errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelError {
    /// Memory configuration is invalid.
    Memory(MemoryError),
    /// Scheduler operation failed.
    Scheduler(SchedulerError),
    /// Integrity measurement failed.
    Measurement(MeasurementError),
    /// Security policy denied the operation.
    Security(SecurityError),
}

impl From<MemoryError> for KernelError {
    fn from(value: MemoryError) -> Self {
        Self::Memory(value)
    }
}

impl From<SchedulerError> for KernelError {
    fn from(value: SchedulerError) -> Self {
        Self::Scheduler(value)
    }
}

impl From<MeasurementError> for KernelError {
    fn from(value: MeasurementError) -> Self {
        Self::Measurement(value)
    }
}

impl From<SecurityError> for KernelError {
    fn from(value: SecurityError) -> Self {
        Self::Security(value)
    }
}

/// Core kernel object for milestone 0 tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Kernel<const MAX_TASKS: usize, const MAX_MEASUREMENTS: usize> {
    allocator: BumpFrameAllocator,
    scheduler: Scheduler<MAX_TASKS>,
    measurements: MeasurementLog<MAX_MEASUREMENTS>,
    state: KernelState,
}

impl<const MAX_TASKS: usize, const MAX_MEASUREMENTS: usize> Kernel<MAX_TASKS, MAX_MEASUREMENTS> {
    /// Initializes kernel services from configuration.
    pub fn init(config: KernelConfig) -> Result<Self, KernelError> {
        let start = PhysFrame::containing_address(config.memory_start);
        let frames = config.memory_len / PAGE_SIZE;
        let range = FrameRange::new(start, frames)?;
        let allocator = BumpFrameAllocator::new(range);
        let scheduler = Scheduler::new();
        let mut measurements = MeasurementLog::new();
        measurements.record(Measurement {
            component: Component::Kernel,
            digest: config.kernel_digest,
        })?;

        Ok(Self {
            allocator,
            scheduler,
            measurements,
            state: KernelState::Initialized,
        })
    }

    /// Creates the idle task.
    pub fn spawn_idle(&mut self) -> Result<TaskId, KernelError> {
        Ok(self.scheduler.spawn(Priority::LOW)?)
    }

    /// Runs one scheduler tick.
    pub fn tick(&mut self) -> Option<TaskId> {
        let selected = self.scheduler.schedule_next();
        if selected.is_some() {
            self.state = KernelState::Running;
        }
        selected
    }

    /// Authorizes a request against a sandbox profile.
    pub fn authorize(
        &self,
        profile: &SandboxProfile,
        request: AccessRequest,
    ) -> Result<(), KernelError> {
        Ok(authorize(profile, request)?)
    }

    /// Allocates one physical frame.
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocator.allocate()
    }

    /// Returns current kernel state.
    pub const fn state(&self) -> KernelState {
        self.state
    }

    /// Returns the integrity measurement log.
    pub const fn measurements(&self) -> &MeasurementLog<MAX_MEASUREMENTS> {
        &self.measurements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use txpos_security::{Capability, CapabilitySet};

    #[test]
    fn kernel_initializes_measurement_and_memory() {
        let mut kernel = Kernel::<4, 4>::init(KernelConfig {
            memory_start: 0x1000,
            memory_len: PAGE_SIZE * 2,
            kernel_digest: [5; 32],
        })
        .unwrap();

        assert_eq!(kernel.measurements().len(), 1);
        assert_eq!(kernel.allocate_frame(), Some(PhysFrame::from_number(1)));
    }

    #[test]
    fn kernel_spawns_and_schedules_idle() {
        let mut kernel = Kernel::<4, 4>::init(KernelConfig {
            memory_start: 0x1000,
            memory_len: PAGE_SIZE,
            kernel_digest: [5; 32],
        })
        .unwrap();
        let idle = kernel.spawn_idle().unwrap();

        assert_eq!(kernel.tick(), Some(idle));
        assert_eq!(kernel.state(), KernelState::Running);
    }

    #[test]
    fn kernel_delegates_security_policy() {
        let kernel = Kernel::<4, 4>::init(KernelConfig {
            memory_start: 0x1000,
            memory_len: PAGE_SIZE,
            kernel_digest: [5; 32],
        })
        .unwrap();
        let profile = SandboxProfile {
            subject_id: [1; 16],
            capabilities: CapabilitySet::empty().with(Capability::Display),
            max_memory_bytes: 4096,
        };
        let request = AccessRequest {
            capability: Capability::Display,
            memory_bytes: 128,
        };

        assert_eq!(kernel.authorize(&profile, request), Ok(()));
    }
}
