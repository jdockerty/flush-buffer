use std::fmt::Debug;

use parking_lot::MutexGuard;

/// A trigger captures the behaviour of a predicate which should cause
/// a flush to the [`Sink`] to occur for the [`Buffer`].
pub trait Trigger: Debug {
    /// Criteria for a flush to occur.
    fn should_flush(
        &self,
        buffer: Option<&MutexGuard<Vec<u8>>>,
    ) -> Result<bool, Box<dyn std::error::Error>>;
}

#[derive(Debug)]
pub struct SizeTrigger(usize);

impl SizeTrigger {
    pub fn new(max_size: usize) -> Self {
        Self(max_size)
    }
}

impl Trigger for SizeTrigger {
    fn should_flush(
        &self,
        buffer: &MutexGuard<Vec<u8>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(buffer.len() >= self.0)
    }
}
