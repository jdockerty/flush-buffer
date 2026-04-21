use parking_lot::Mutex;
use std::{io::Write, sync::Arc};

use crate::{Sink, Trigger};

/// A buffer which will flush to a defined [`Sink`] based
/// on a [`Trigger`].
pub struct Buffer<S, T> {
    inner: Arc<Mutex<Vec<u8>>>,
    sink: S,
    trigger: T,
}

impl<S, T> Buffer<S, T> {
    pub fn new(capacity: Option<usize>, sink: S, trigger: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::with_capacity(capacity.unwrap_or(1024)))),
            sink,
            trigger,
        }
    }
}

impl<S: Sink, T: Trigger> Buffer<S, T> {
    /// Write to the underlying buffer.
    ///
    /// The buffer MAY flush to the defined [`Sink`] depending on the predicate
    /// within the [`Trigger`].
    fn write(&self, data: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        {
            let mut guard = self.inner.lock();
            guard.write_all(data)?;

            if self.trigger.should_flush(&guard)? {
                self.sink.flush(&guard)?;
                guard.clear();
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod test {
    use std::{fmt::Debug, io::Write, sync::Arc};

    use crate::{Buffer, Sink, trigger::SizeTrigger};
    use parking_lot::Mutex;

    #[derive(Debug)]
    struct MemSink {
        inner: Arc<Mutex<Vec<u8>>>,
    }

    impl Sink for MemSink {
        fn flush(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
            self.inner.lock().write_all(data).unwrap();
            Ok(())
        }
    }

    #[test]
    fn flushes() {
        let sink = MemSink {
            inner: Arc::new(Mutex::new(Vec::new())),
        };
        let sink_inner = Arc::clone(&sink.inner);
        let trigger = SizeTrigger::new(10);

        let buffer = Buffer::new(None, sink, trigger);
        let buffer_inner = Arc::clone(&buffer.inner);

        assert!(
            !buffer.write(b"hello").unwrap(),
            "Flush should not have happened above configured max size",
        );
        assert_eq!(buffer_inner.lock().to_vec(), b"hello");
        assert!(
            sink_inner.lock().is_empty(),
            "Sink should be empty from no flush"
        );

        assert!(
            buffer.write(b" world").unwrap(),
            "Flush should have occurred"
        );
        assert!(
            buffer_inner.lock().is_empty(),
            "Buffer should be cleared after flush"
        );
        assert_eq!(sink_inner.lock().to_vec(), b"hello world");
    }
}
