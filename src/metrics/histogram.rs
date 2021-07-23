use metrics_util::AtomicBucket;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

const BUCKET_RETENTION_PERIOD: Duration = Duration::from_secs(60);

pub struct Histogram {
    bucket: OnceCell<AtomicBucket<f64>>,
    started: AtomicU64,
}

impl Histogram {
    pub const fn new() -> Self {
        Self {
            bucket: OnceCell::new(),
            started: AtomicU64::new(0),
        }
    }

    /// Adds a sample to the [`Histogram`]
    #[inline]
    pub fn insert(&self, val: f64) {
        self.clear_if_timeout();
        self.bucket.get_or_init(AtomicBucket::new).push(val)
    }

    /// Read the current state of the [`Histogram`]
    #[inline]
    pub fn read(&self) -> Vec<f64> {
        self.clear_if_timeout();
        self.bucket.get_or_init(AtomicBucket::new).data()
    }

    #[inline]
    fn clear_if_timeout(&self) {
        let started = self.started.load(Ordering::Acquire);
        if SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH + Duration::from_nanos(started))
            .map(|duration| duration >= BUCKET_RETENTION_PERIOD)
            .unwrap_or(false)
        {
            if self
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
}
