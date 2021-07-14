Metrics Catalogue
=============

[<img alt="github" src="https://img.shields.io/badge/github-metrics--catalogue-8da0cb?style=flat-square&labelColor=555555&logo=github" height="20">](https://github.com/sadroeck/metrics-catalogue)
[<img alt="crates.io" src="https://img.shields.io/crates/v/metrics-catalogue.svg?style=flat-square&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/metrics-catalogue)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-metrics--catalogue-66c2a5?style=flat-square&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/metrics-catalogue)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/sadroeck/metrics-catalogue/Rust/main?style=flat-square" height="20">](https://github.com/sadroeck/metrics-catalogue/actions?query=branch%3Amain)

This library provides a way to automatically derive a catalogue of [metrics](https://github.com/metrics-rs/metrics) from a struct definition.
It generates a hierarchical set of modules & metric keys following the same hierarchy from the root struct.
This allows users to interact with the `metrics` without the risk of typos or having to maintain an external list of available metric keys.
Additionally, the metrics can also be interacted with directly, without using the `metrics` framework.

[`metrics`]: https://doc.rust-lang.org/std/error/trait.Error.html

```toml
[dependencies]
metrics-catalogue = "0.1"
```

## Example

```rust
use metric_catalogue::{Metrics, Counter, Gauge};

#[derive(Catalogue)]
#[metric(root)]
struct Foo {
  my_counter: Counter,
  my_gauge: Gauge,
  my_bar: Bar,
}

#[derive(Catalogue)]
struct Bar {
  my_counter: Counter,
}
```

generates the following catalogue of metric keys:

```rust
#[allow(non_camel_case_types)]
pub mod catalogue {
  pub const MY_COUNTER: &str = "my_counter";
  pub const MY_GAUGE: &str = "my_gauge";
  
  #[allow(non_camel_case_types)]
  pub mod bar {
    pub const MY_COUNTER: &str = "bar.my_counter";
  }
}
```

allowing updates to the relevant metrics without potential typos:

```rust
fn my_function() {
  metrics::increment_counter!(catalogue::my_sub::MY_COUNTER);
}
```

## Supported metric types

Currently, the following metric types are supported:

- [`Counter`] A monotonically incrementing [`AtomicU64`] metric
- [`Gauge`] An [`AtomicU64`] based metric allowing arbitrary updated, increments & decrements using real units.
- [`DiscreteGauge`] An [`AtomicU64`] based metric allowing arbitrary updated, increments & decrements using discrete units.

## Metrics Registry

Adding the `Catalogue` derivation will implement the `Registry` trait for all relevant structs:

```rust
pub trait Registry {
    /// Find a registered counter under the provided name
    fn find_counter(&self, name: &str) -> Option<&Counter>;
    /// Find a registered gauge under the provided name
    fn find_gauge(&self, name: &str) -> Option<&Gauge>;
}
```

allowing an automatic hierarchical look-up of the generated catalogue.

## Metrics recorder

The `Catalogue` derivation will also implement the [`Recorder`] trait for the `root` struct:

```rust
impl Recorder for Foo {
        fn register_counter(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}
        fn register_gauge(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}
        fn register_histogram(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}
        fn record_histogram(&self, _key: &Key, _value: f64) {}
        fn increment_counter(&self, key: &Key, value: u64) {
            if let Some(metric) = self.find_counter(key.name()) {
                metric.increment(value);
            }
        }
        fn update_gauge(&self, key: &Key, value: GaugeValue) {
            if let Some(metric) = self.find_gauge(key.name()) {
                match value {
                    GaugeValue::Increment(val) => metric.increase(val),
                    GaugeValue::Decrement(val) => metric.decrease(val),
                    GaugeValue::Absolute(val) => metric.set(val),
                }
            }
        }
    }
```

[`Recorder`]: https://docs.rs/metrics/0.16.0/metrics/trait.Recorder.html

## Details

- A single `root` structure must be declared by using the `root` attribute. 
  Without a root, no catalogue will be generated.
  
- fields can be hidden from the catalogue by using the `skip` attribute.
  e.g.
  ```rust
  #[derive(Catalogue)]
  struct Foo {
    #[metric(skip)]
    my_hidden_field: Counter,
  }
  ```
  which will prevent generating the `MY_HIDDEN_FIELD -> "my_hidden_field"` key & the associated registry entries.

- the `Catalogue` macro is limited to `struct`s only.

#### License

Copyright 2021 Sam De Roeck

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
