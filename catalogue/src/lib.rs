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
    struct Test {
        my_b: Counter,
        my_g: Gauge,

        // Name override
        #[metric("my_counter_a")]
        my_a: Counter,

        /// Hidden metrics
        #[metric(hidden)]
        my_non_g: Counter,
        #[metric(hidden)]
        my_hidden_sub: SubTest,

        /// Subtypes
        my_test: SubTest,
    }

    #[derive(Metrics)]
    struct SubTest {
        my_t_a: Gauge,
        my_t_b: Counter,
        my_sub_sub: SubSubTest,
    }

    #[derive(Metrics)]
    struct SubSubTest {
        my_s_t_a: Gauge,
        my_s_t_b: Counter,
    }

    #[test]
    fn key_generation() {
        let known_names = [
            (catalogue::MY_B, "my_b"),
            (catalogue::MY_G, "my_g"),
            (catalogue::MY_COUNTER_A, "my_counter_a"),
            (catalogue::my_test::MY_T_A, "my_test.my_t_a"),
            (catalogue::my_test::MY_T_B, "my_test.my_t_b"),
            (
                catalogue::my_test::my_sub_sub::MY_S_T_A,
                "my_test.my_sub_sub.my_s_t_a",
            ),
            (
                catalogue::my_test::my_sub_sub::MY_S_T_B,
                "my_test.my_sub_sub.my_s_t_b",
            ),
        ];
        known_names.iter().for_each(|(k, v)| assert_eq!(k, v));
    }

    #[test]
    fn counters() {
        let t = Test::new();
        let registered_counters = [
            (catalogue::MY_B, &t.my_b),
            (catalogue::MY_COUNTER_A, &t.my_a),
            (catalogue::my_test::MY_T_B, &t.my_test.my_t_b),
            (
                catalogue::my_test::my_sub_sub::MY_S_T_B,
                &t.my_test.my_sub_sub.my_s_t_b,
            ),
        ];
        for (key, field) in registered_counters {
            let pre = field.read();
            t.increment_counter(&Key::from_name(key), 1);
            assert_eq!(
                field.read(),
                pre + 1,
                "key {} did not update the counter",
                key
            );
        }
    }

    #[test]
    fn hidden_counters() {
        let t = Test::new();
        let hidden_counters = [
            ("my_non_g", &t.my_non_g),
            ("my_hidden_sub.my_b", &t.my_hidden_sub.my_t_b),
            (
                "my_hidden_sub.my_sub_sub.my_s_t_b",
                &t.my_hidden_sub.my_sub_sub.my_s_t_b,
            ),
        ];
        for (key, field) in hidden_counters {
            let pre = field.read();
            t.increment_counter(&Key::from_name(key), 1);
            assert_eq!(field.read(), pre, "key {} updated the counter", key);
        }
    }
}
