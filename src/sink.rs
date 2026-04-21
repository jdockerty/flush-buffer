use std::fmt::Debug;

/// The destination for flushes of the [`Buffer`].
pub trait Sink: Debug {
    fn flush(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}
