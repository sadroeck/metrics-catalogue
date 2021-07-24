use crate::{Counter, GaugeMetric, HistogramMetric};

pub trait Registry {
    /// Find a registered counter under the provided name
    fn find_counter(&self, name: &str) -> Option<&Counter>;
    /// Find a registered gauge under the provided name
    fn find_gauge(&self, name: &str) -> Option<&dyn GaugeMetric>;
    /// Find a registered [`Histogram`] under the provided name
    fn find_histogram(&self, name: &str) -> Option<&dyn HistogramMetric>;
}
