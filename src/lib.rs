mod metrics;
mod prometheus;
mod registry;

/// Export metrics types
pub use crate::metrics::*;
/// Re-export metrics crate
#[doc(hidden)]
pub use ::metrics::*;
/// Re-export declarative macros
pub use metrics_catalogue_macros::Catalogue;
/// Export registry trait
pub use registry::Registry;

#[cfg(feature = "prometheus")]
pub use prometheus::*;
