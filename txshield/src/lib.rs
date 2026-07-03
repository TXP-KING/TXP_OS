#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXShield runtime integrity measurement primitives.

use txpos_crypto::constant_time_eq;

/// Measured component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Component {
    /// UEFI bootloader.
    Bootloader,
    /// Kernel image.
    Kernel,
    /// Memory manager.
    Memory,
    /// Scheduler.
    Scheduler,
    /// Security policy engine.
    Security,
}

/// A single integrity measurement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Measurement {
    /// Component measured.
    pub component: Component,
    /// Digest selected by the active boot policy.
    pub digest: [u8; 32],
}

/// Measurement log errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurementError {
    /// The log is full.
    Full,
    /// The supplied baseline does not match the log.
    BaselineMismatch,
}

/// Fixed-capacity measurement log.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementLog<const MAX: usize> {
    entries: [Option<Measurement>; MAX],
    len: usize,
}

impl<const MAX: usize> MeasurementLog<MAX> {
    /// Creates an empty log.
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX],
            len: 0,
        }
    }

    /// Records a measurement.
    pub fn record(&mut self, measurement: Measurement) -> Result<(), MeasurementError> {
        if self.len >= MAX {
            return Err(MeasurementError::Full);
        }

        self.entries[self.len] = Some(measurement);
        self.len += 1;
        Ok(())
    }

    /// Returns the number of recorded measurements.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no measurements are recorded.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a measurement by index.
    pub const fn get(&self, index: usize) -> Option<Measurement> {
        if index >= self.len {
            return None;
        }

        self.entries[index]
    }

    /// Verifies that the current log starts with the expected baseline.
    pub fn verify_prefix(&self, expected: &[Measurement]) -> Result<(), MeasurementError> {
        if expected.len() > self.len {
            return Err(MeasurementError::BaselineMismatch);
        }

        for (index, baseline) in expected.iter().enumerate() {
            let Some(actual) = self.entries[index] else {
                return Err(MeasurementError::BaselineMismatch);
            };

            if actual.component != baseline.component
                || !constant_time_eq(&actual.digest, &baseline.digest)
            {
                return Err(MeasurementError::BaselineMismatch);
            }
        }

        Ok(())
    }
}

impl<const MAX: usize> Default for MeasurementLog<MAX> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurement_log_verifies_baseline() {
        let measurement = Measurement {
            component: Component::Kernel,
            digest: [7; 32],
        };
        let mut log = MeasurementLog::<4>::new();
        log.record(measurement).unwrap();

        assert_eq!(log.verify_prefix(&[measurement]), Ok(()));
    }

    #[test]
    fn measurement_log_fails_closed_on_mismatch() {
        let mut log = MeasurementLog::<1>::new();
        log.record(Measurement {
            component: Component::Kernel,
            digest: [1; 32],
        })
        .unwrap();

        let expected = Measurement {
            component: Component::Kernel,
            digest: [2; 32],
        };

        assert_eq!(
            log.verify_prefix(&[expected]),
            Err(MeasurementError::BaselineMismatch)
        );
    }
}
