#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Memory primitives for the earliest TXPOS kernel milestones.

/// Standard x86-64 page size used by milestone 0.
pub const PAGE_SIZE: usize = 4096;

/// A physical frame number.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PhysFrame {
    number: usize,
}

impl PhysFrame {
    /// Creates a frame from a frame number.
    pub const fn from_number(number: usize) -> Self {
        Self { number }
    }

    /// Returns the frame containing a physical address.
    pub const fn containing_address(address: usize) -> Self {
        Self {
            number: address / PAGE_SIZE,
        }
    }

    /// Returns the frame number.
    pub const fn number(self) -> usize {
        self.number
    }

    /// Returns the starting physical address for this frame.
    pub const fn start_address(self) -> usize {
        self.number * PAGE_SIZE
    }
}

/// Memory-management errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryError {
    /// The requested range is empty.
    EmptyRange,
    /// The requested range overflows addressable memory.
    RangeOverflow,
}

/// A contiguous range of physical frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameRange {
    start: PhysFrame,
    len: usize,
}

impl FrameRange {
    /// Creates a frame range.
    pub const fn new(start: PhysFrame, len: usize) -> Result<Self, MemoryError> {
        if len == 0 {
            return Err(MemoryError::EmptyRange);
        }

        if start.number().checked_add(len).is_none() {
            return Err(MemoryError::RangeOverflow);
        }

        Ok(Self { start, len })
    }

    /// Start frame.
    pub const fn start(self) -> PhysFrame {
        self.start
    }

    /// Number of frames in the range.
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns true when the range contains no frames.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// End frame number, exclusive.
    pub const fn end_number_exclusive(self) -> usize {
        self.start.number() + self.len
    }

    /// Returns whether a frame is inside the range.
    pub const fn contains(self, frame: PhysFrame) -> bool {
        frame.number() >= self.start.number() && frame.number() < self.end_number_exclusive()
    }
}

/// A simple bump allocator for early physical frame allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BumpFrameAllocator {
    range: FrameRange,
    next: usize,
    allocated: usize,
}

impl BumpFrameAllocator {
    /// Creates an allocator over the supplied frame range.
    pub const fn new(range: FrameRange) -> Self {
        Self {
            range,
            next: range.start().number(),
            allocated: 0,
        }
    }

    /// Allocates one physical frame.
    pub fn allocate(&mut self) -> Option<PhysFrame> {
        if self.next >= self.range.end_number_exclusive() {
            return None;
        }

        let frame = PhysFrame::from_number(self.next);
        self.next += 1;
        self.allocated += 1;
        Some(frame)
    }

    /// Number of frames already allocated.
    pub const fn allocated(&self) -> usize {
        self.allocated
    }

    /// Number of frames still available.
    pub const fn remaining(&self) -> usize {
        self.range.end_number_exclusive() - self.next
    }

    /// Returns the original allocation range.
    pub const fn range(&self) -> FrameRange {
        self.range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_addresses_are_page_aligned() {
        let frame = PhysFrame::containing_address(0x2345);
        assert_eq!(frame.number(), 2);
        assert_eq!(frame.start_address(), 0x2000);
    }

    #[test]
    fn bump_allocator_walks_range_once() {
        let range = FrameRange::new(PhysFrame::from_number(10), 2).unwrap();
        let mut allocator = BumpFrameAllocator::new(range);

        assert_eq!(allocator.allocate(), Some(PhysFrame::from_number(10)));
        assert_eq!(allocator.allocate(), Some(PhysFrame::from_number(11)));
        assert_eq!(allocator.allocate(), None);
        assert_eq!(allocator.allocated(), 2);
        assert_eq!(allocator.remaining(), 0);
    }
}
