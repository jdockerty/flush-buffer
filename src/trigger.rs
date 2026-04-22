use std::{
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread::JoinHandle,
    time::Duration,
};

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
        buffer: Option<&MutexGuard<Vec<u8>>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(buffer.expect("buffer is required to know size limit").len() >= self.0)
    }
}

#[derive(Debug)]
pub struct DurationTrigger {
    duration: Duration,
    monitor_task: JoinHandle<()>,
    flush_tx: Sender<()>,
    flush_rx: Receiver<()>,
}

impl DurationTrigger {
    pub fn new(duration: Duration) -> Self {
        let duration = duration.clone();
        let (flush_tx, flush_rx) = std::sync::mpsc::channel();
        let flush_tx_captured = flush_tx.clone();
        let monitor_task = std::thread::spawn(move || {
            loop {
                std::thread::sleep(duration);
                match flush_tx_captured.send(()) {
                    Ok(_) => {}
                    Err(_) => {
                        eprintln!("attempting to send duration trigger, but receiver was dropped");
                        break;
                    }
                }
            }
        });

        Self {
            duration,
            monitor_task,
            flush_tx,
            flush_rx,
        }
    }
}

impl Trigger for DurationTrigger {
    fn should_flush(
        &self,
        _buffer: Option<&MutexGuard<Vec<u8>>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Intentional use of `try_recv` here to avoid blocking
        Ok(self.flush_rx.try_recv().is_ok())
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        time::{Duration, Instant},
    };

    use crate::{Trigger, trigger::DurationTrigger};

    #[test]
    fn duration_trigger() {
        let duration = Duration::from_millis(100);
        let trigger = DurationTrigger::new(duration);
        let mut observations: HashMap<bool, usize> = HashMap::new();

        let start = Instant::now();
        loop {
            if start.elapsed() >= duration * 3 {
                break;
            }

            match trigger.flush_rx.try_recv() {
                Ok(_) => {
                    observations
                        .entry(true)
                        .and_modify(|v| *v += 1)
                        .or_default();
                }
                _ => {
                    observations
                        .entry(false)
                        .and_modify(|v| *v += 1)
                        .or_default();
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(*observations.get(&true).unwrap() >= 1);
        assert!(*observations.get(&false).unwrap() >= 1);

        let duration = Duration::from_millis(50);
        let trigger = DurationTrigger::new(duration);
        std::thread::sleep(duration * 2);
        assert!(trigger.should_flush(None).unwrap());
    }
}
