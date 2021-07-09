mod metrics;
mod registry;

/// Export metrics types
pub use crate::metrics::*;
/// Re-export metrics crate
pub use ::metrics::*;
/// Re-export declarative macros
pub use metrics_catalogue_macros::Metrics;
/// Export registry trait
pub use registry::Registry;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Metrics)]
    pub struct Test {
        my_b: Counter,
        my_g: Gauge,

        // Name override
        #[metric("my_counter_a")]
        my_a: Counter,

        /// Hidden metrics
        #[metric(hidden)]
        pub my_non_g: Gauge,
        #[metric(hidden)]
        pub my_hidden_sub: SubTest,

        /// Subtypes
        pub my_test: SubTest,
    }

    #[derive(Metrics)]
    pub struct SubTest {
        my_t_a: Gauge,
        pub my_sub_sub: SubSubTest,
    }

    #[derive(Metrics)]
    pub struct SubSubTest {
        my_s_t_a: Gauge,
    }
}
