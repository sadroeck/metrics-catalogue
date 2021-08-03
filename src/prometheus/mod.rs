use crate::prometheus::utils::{write_metric_line, write_type_line};
use crate::{Counter, DiscreteGauge, Gauge, GaugeMetric, Histogram, HistogramMetric};
use metrics_util::Summary;

mod utils;

const QUANTILES: [f64; 4] = [0.0, 0.5, 0.9, 0.99];

pub trait Render {
    fn render(&self, name: &str, s: &mut String);
}

impl Render for Counter {
    fn render(&self, name: &str, s: &mut String) {
        // TODO: Process description
        // if let Some(desc) = descriptions.get(name.as_str()) {
        //     write_help_line(&mut output, name.as_str(), desc);
        // }
        write_type_line(s, name, "counter");

        // TODO: process labels
        let labels = &[];
        write_metric_line::<&str, u64>(s, name, None, labels, None, self.read());
        s.push('\n');
    }
}

#[inline]
fn render_gauge<G: GaugeMetric>(g: &G, name: &str, s: &mut String) {
    // TODO: Process description
    // if let Some(desc) = descriptions.get(name.as_str()) {
    //     write_help_line(&mut output, name.as_str(), desc);
    // }

    write_type_line(s, name, "gauge");
    // TODO: process labels
    let labels = [];
    write_metric_line::<&str, f64>(s, name, None, &labels, None, g.read());
    s.push('\n');
}

impl Render for Gauge {
    fn render(&self, name: &str, s: &mut String) {
        render_gauge(self, name, s)
    }
}

impl Render for DiscreteGauge {
    fn render(&self, name: &str, s: &mut String) {
        render_gauge(self, name, s)
    }
}

#[inline]
fn render_histogram<H: HistogramMetric>(h: &H, name: &str, s: &mut String) {
    // TODO: Process description
    // if let Some(desc) = descriptions.get(name.as_str()) {
    //     write_help_line(&mut output, name.as_str(), desc);
    // }

    write_type_line(s, name, "histogram");
    // TODO: process labels
    let labels = [];
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
        write_metric_line(s, &name, None, &labels, Some(("quantile", q)), value);
    }
    write_metric_line::<&str, f64>(s, &name, Some("sum"), &labels, None, sum);
    write_metric_line::<&str, u64>(s, &name, Some("count"), &labels, None, count as u64);

    s.push('\n');
}

impl<const RETENTION: u64> Render for Histogram<RETENTION> {
    fn render(&self, name: &str, s: &mut String) {
        render_histogram(self, name, s)
    }
}
