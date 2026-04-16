use std::{
    fmt::Debug,
    io::Write,
    sync::{Arc, Mutex},
};

/// The destination for flushes of the [`Buffer`].
pub trait Sink: Debug {
    fn write(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}

/// A buffer which will flush to a defined [`Sink`].
pub struct Buffer<S> {
    inner: Arc<Mutex<Vec<u8>>>,
    sink: S,
    max_size: usize,
}

impl<S> Buffer<S> {
    pub fn new(capacity: Option<usize>, max_size: usize, sink: S) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::with_capacity(capacity.unwrap_or(1024)))),
            sink,
            max_size,
        }
    }
}

impl<S: Sink> Buffer<S> {
    fn write(&self, data: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        {
            let mut guard = self.inner.lock().expect("lock is not poisoned");
            guard.write_all(data)?;

            if guard.len() >= self.max_size {
                self.sink.write(&guard)?;
                guard.clear();
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod test {
    use std::{
        fmt::Debug,
        io::Write,
        sync::{Arc, Mutex},
    };

    use crate::{Buffer, Sink};

    #[derive(Debug)]
    struct MemSink {
        inner: Arc<Mutex<Vec<u8>>>,
    }

    impl Sink for MemSink {
        fn write(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
            self.inner.lock().unwrap().write_all(data).unwrap();
            Ok(())
        }
    }

    #[test]
    fn flushes() {
        let sink = MemSink {
            inner: Arc::new(Mutex::new(Vec::new())),
        };
        let sink_inner = Arc::clone(&sink.inner);

        let buffer = Buffer::new(None, 10, sink);
        let buffer_inner = Arc::clone(&buffer.inner);

        assert!(
            !buffer.write(b"hello").unwrap(),
            "Flush should not have happened above configured max size",
        );
        assert_eq!(buffer_inner.lock().unwrap().to_vec(), b"hello");
        assert!(
            sink_inner.lock().unwrap().is_empty(),
            "Sink should be empty from no flush"
        );

        assert!(
            buffer.write(b" world").unwrap(),
            "Flush should have occurred"
        );
        assert!(
            buffer_inner.lock().unwrap().is_empty(),
            "Buffer should be cleared after flush"
        );
        assert_eq!(sink_inner.lock().unwrap().to_vec(), b"hello world");
    }
}
