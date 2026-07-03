#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Fixed-capacity scheduler primitives for the early kernel.

/// Identifier for a schedulable task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskId(u64);

impl TaskId {
    /// Creates a task identifier from a raw value.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw identifier.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Task priority. Lower numeric values run first within the same pass.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Priority(u8);

impl Priority {
    /// Lowest normal priority.
    pub const LOW: Self = Self(200);
    /// Default priority.
    pub const NORMAL: Self = Self(100);
    /// Highest non-realtime priority.
    pub const HIGH: Self = Self(10);

    /// Creates a priority.
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the numeric priority.
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Current task state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    /// Ready to run.
    Ready,
    /// Currently selected by the scheduler.
    Running,
    /// Waiting for an external event.
    Blocked,
    /// Task has exited and its slot can be reclaimed later.
    Exited,
}

/// Task metadata stored by the scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskControlBlock {
    /// Unique task id.
    pub id: TaskId,
    /// Scheduling priority.
    pub priority: Priority,
    /// Current state.
    pub state: TaskState,
}

/// Scheduler errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    /// No free task slot exists.
    Full,
    /// The task id is unknown.
    UnknownTask,
}

/// A deterministic fixed-capacity scheduler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduler<const MAX_TASKS: usize> {
    tasks: [Option<TaskControlBlock>; MAX_TASKS],
    cursor: usize,
    next_id: u64,
}

impl<const MAX_TASKS: usize> Scheduler<MAX_TASKS> {
    /// Creates an empty scheduler.
    pub const fn new() -> Self {
        Self {
            tasks: [None; MAX_TASKS],
            cursor: 0,
            next_id: 1,
        }
    }

    /// Adds a task and returns its id.
    pub fn spawn(&mut self, priority: Priority) -> Result<TaskId, SchedulerError> {
        for slot in &mut self.tasks {
            if slot.is_none() {
                let id = TaskId::from_raw(self.next_id);
                self.next_id = self.next_id.saturating_add(1);
                *slot = Some(TaskControlBlock {
                    id,
                    priority,
                    state: TaskState::Ready,
                });
                return Ok(id);
            }
        }

        Err(SchedulerError::Full)
    }

    /// Marks a task blocked.
    pub fn block(&mut self, id: TaskId) -> Result<(), SchedulerError> {
        self.set_state(id, TaskState::Blocked)
    }

    /// Wakes a blocked task.
    pub fn wake(&mut self, id: TaskId) -> Result<(), SchedulerError> {
        self.set_state(id, TaskState::Ready)
    }

    /// Marks a task exited.
    pub fn exit(&mut self, id: TaskId) -> Result<(), SchedulerError> {
        self.set_state(id, TaskState::Exited)
    }

    /// Selects the next ready task using priority-aware round-robin.
    pub fn schedule_next(&mut self) -> Option<TaskId> {
        self.demote_running_tasks();

        let best_priority = self
            .tasks
            .iter()
            .flatten()
            .filter(|task| task.state == TaskState::Ready)
            .map(|task| task.priority)
            .min()?;

        for offset in 0..MAX_TASKS {
            let index = (self.cursor + offset) % MAX_TASKS;
            if let Some(task) = &mut self.tasks[index] {
                if task.state == TaskState::Ready && task.priority == best_priority {
                    task.state = TaskState::Running;
                    self.cursor = (index + 1) % MAX_TASKS;
                    return Some(task.id);
                }
            }
        }

        None
    }

    /// Returns the task metadata for an id.
    pub fn task(&self, id: TaskId) -> Option<TaskControlBlock> {
        self.tasks
            .iter()
            .flatten()
            .copied()
            .find(|task| task.id == id)
    }

    /// Counts non-empty task slots.
    pub fn len(&self) -> usize {
        self.tasks.iter().filter(|task| task.is_some()).count()
    }

    /// Returns true when no task slots are occupied.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn set_state(&mut self, id: TaskId, state: TaskState) -> Result<(), SchedulerError> {
        for task in self.tasks.iter_mut().flatten() {
            if task.id == id {
                task.state = state;
                return Ok(());
            }
        }

        Err(SchedulerError::UnknownTask)
    }

    fn demote_running_tasks(&mut self) {
        for task in self.tasks.iter_mut().flatten() {
            if task.state == TaskState::Running {
                task.state = TaskState::Ready;
            }
        }
    }
}

impl<const MAX_TASKS: usize> Default for Scheduler<MAX_TASKS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_respects_priority() {
        let mut scheduler = Scheduler::<4>::new();
        let low = scheduler.spawn(Priority::LOW).unwrap();
        let high = scheduler.spawn(Priority::HIGH).unwrap();

        assert_eq!(scheduler.schedule_next(), Some(high));
        scheduler.block(high).unwrap();
        assert_eq!(scheduler.schedule_next(), Some(low));
    }

    #[test]
    fn scheduler_round_robins_equal_priority() {
        let mut scheduler = Scheduler::<4>::new();
        let a = scheduler.spawn(Priority::NORMAL).unwrap();
        let b = scheduler.spawn(Priority::NORMAL).unwrap();

        assert_eq!(scheduler.schedule_next(), Some(a));
        assert_eq!(scheduler.schedule_next(), Some(b));
        assert_eq!(scheduler.schedule_next(), Some(a));
    }
}
