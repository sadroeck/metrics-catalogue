use metrics_util::AtomicBucket;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

pub trait HistogramMetric {
    /// Adds a sample to the container
    fn insert(&self, val: f64);
    /// Returns a list of the current samples in the container
    fn read(&self) -> Vec<f64>;
}

/// Mimics a [`metrics-core`] histogram container for bucketed sample grouping.
/// Provides an automatic retention of samples
#[derive(Debug)]
pub struct Histogram<const RETENTION: u64> {
    // TODO: Migrate from a lazily initialized cell to a const initializable container
    bucket: OnceCell<AtomicBucket<f64>>,
    started: AtomicU64,
}

impl<const RETENTION: u64> Histogram<RETENTION> {
    pub const fn new() -> Self {
        Self {
            bucket: OnceCell::new(),
            started: AtomicU64::new(0),
        }
    }

    #[inline]
    fn clear_if_timeout(&self) {
        let started = self.started.load(Ordering::Acquire);
        let reached_window = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH + Duration::from_secs(started))
            .map(|duration| duration >= Duration::from_secs(RETENTION))
            .unwrap_or(false);
        if reached_window
            && self
                .started
                .compare_exchange(
                    started,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                )
                .is_ok()
        {
            if let Some(bucket) = self.bucket.get() {
                bucket.clear();
            }
        }
    }
}

impl<const RET: u64> HistogramMetric for Histogram<RET> {
    /// Adds a sample to the [`Histogram`]
    #[inline]
    fn insert(&self, val: f64) {
        self.clear_if_timeout();
        self.bucket.get_or_init(AtomicBucket::new).push(val)
    }

    /// Read the current state of the [`Histogram`]
    #[inline]
    fn read(&self) -> Vec<f64> {
        self.clear_if_timeout();
        self.bucket.get_or_init(AtomicBucket::new).data()
    }
}
