use crate::prometheus::utils::{write_metric_line, write_type_line, Label};
use crate::{Counter, DiscreteGauge, Gauge, GaugeMetric, Histogram, HistogramMetric};
use metrics_util::Summary;
use std::iter::{empty, once};

mod utils;

const QUANTILES: [f64; 4] = [0.0, 0.5, 0.9, 0.99];

pub trait StringRender {
    fn render(&self, prefix: &str, name: &str, s: &mut String);
}

impl StringRender for Counter {
    fn render(&self, prefix: &str, name: &str, s: &mut String) {
        // TODO: Process description
        // if let Some(desc) = descriptions.get(name.as_str()) {
        //     write_help_line(&mut output, name.as_str(), desc);
        // }
        write_type_line(s, prefix, name, "counter");

        // TODO: process labels
        let labels = empty();
        write_metric_line::<&str, u64, _>(s, prefix, name, None, labels, self.read());
        s.push('\n');
    }
}

#[inline]
fn render_gauge<G: GaugeMetric>(g: &G, prefix: &str, name: &str, s: &mut String) {
    // TODO: Process description
    // if let Some(desc) = descriptions.get(name.as_str()) {
    //     write_help_line(&mut output, name.as_str(), desc);
    // }

    write_type_line(s, prefix, name, "gauge");
    // TODO: process labels
    let labels = empty();
    write_metric_line::<&str, f64, _>(s, prefix, name, None, labels, g.read());
    s.push('\n');
}

impl StringRender for Gauge {
    #[inline]
    fn render(&self, prefix: &str, name: &str, s: &mut String) {
        render_gauge(self, prefix, name, s)
    }
}

impl StringRender for DiscreteGauge {
    #[inline]
    fn render(&self, prefix: &str, name: &str, s: &mut String) {
        render_gauge(self, prefix, name, s)
    }
}

#[inline]
fn render_histogram<H: HistogramMetric>(h: &H, prefix: &str, name: &str, s: &mut String) {
    // TODO: Process description
    // if let Some(desc) = descriptions.get(name.as_str()) {
    //     write_help_line(&mut output, name.as_str(), desc);
    // }

    write_type_line(s, prefix, name, "histogram");
    // TODO: process labels
    let labels = empty();
    let mut summary = Summary::with_defaults();
    let samples = h.read();
    let mut sum = 0.0;
    let count = samples.len();
    for sample in samples {
        summary.add(sample);
        sum += sample;
    }
    for q in QUANTILES {
        let value = summary.quantile(q).unwrap_or(0.0);
        write_metric_line(
            s,
            prefix,
            name,
            None,
            labels.clone().chain(once(Label::KeyValue(("quantile", q)))),
            value,
        );
    }
    write_metric_line(s, prefix, name, Some("sum"), empty::<Label<usize>>(), sum);
    write_metric_line(
        s,
        prefix,
        name,
        Some("count"),
        empty::<Label<usize>>(),
        count as u64,
    );

    s.push('\n');
}

impl<const RETENTION: u64> StringRender for Histogram<RETENTION> {
    #[inline]
    fn render(&self, prefix: &str, name: &str, s: &mut String) {
        render_histogram(self, prefix, name, s)
    }
}
