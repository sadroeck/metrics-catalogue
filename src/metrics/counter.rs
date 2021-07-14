use std::sync::atomic::{AtomicU64, Ordering};

/// Mimics a [`metrics-core`] monotonically increasing [`Counter`] type
pub struct Counter(AtomicU64);

impl Counter {
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Increases the value of the [`Counter`] by a discrete amount
    #[inline]
    pub fn increment(&self, val: u64) {
        self.0.fetch_add(val, Ordering::Release);
    }

    /// Read the current state of the [`Counter`]
    #[inline]
    pub fn read(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}
