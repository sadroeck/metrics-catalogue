use metrics_catalogue::{
    Catalogue, Counter, DiscreteGauge, Gauge, Histogram, HistogramMetric, Key, Recorder,
};

#[derive(Catalogue)]
#[metric(root)]
struct Test {
    my_b: Counter,
    my_g: Gauge,
    my_h_60: Histogram<60>,
    my_discrete_g: DiscreteGauge,

    // Name override
    #[metric("my_counter_a")]
    my_a: Counter,

    /// Hidden metrics
    #[metric(skip)]
    _my_non_g: Counter,
    #[metric(skip)]
    _my_hidden_sub: SubTest,

    /// Subtypes
    my_test: SubTest,
    my_second_test: SubTest,

    // Fully-qualified types
    my_full_counter: ::metrics_catalogue::Counter,
    my_h_30: ::metrics_catalogue::Histogram<30>,
}

#[derive(Catalogue)]
struct SubTest {
    my_t_a: Gauge,
    my_t_b: Counter,
    my_t_h: Histogram<5>,
    my_sub_sub: SubSubTest,
}

#[derive(Catalogue)]
struct SubSubTest {
    my_s_t_a: Gauge,
    my_s_t_b: Counter,
}

#[test]
fn key_generation() {
    let known_names = [
        (catalogue::MY_B, "test.my_b"),
        (catalogue::MY_G, "test.my_g"),
        (catalogue::MY_H_60, "test.my_h_60"),
        (catalogue::MY_COUNTER_A, "test.my_counter_a"),
        (catalogue::my_test::MY_T_A, "test.my_test.my_t_a"),
        (catalogue::my_test::MY_T_B, "test.my_test.my_t_b"),
        (catalogue::my_test::MY_T_H, "test.my_test.my_t_h"),
        (
            catalogue::my_test::my_sub_sub::MY_S_T_A,
            "test.my_test.my_sub_sub.my_s_t_a",
        ),
        (
            catalogue::my_test::my_sub_sub::MY_S_T_B,
            "test.my_test.my_sub_sub.my_s_t_b",
        ),
        (
            catalogue::my_second_test::MY_T_A,
            "test.my_second_test.my_t_a",
        ),
        (
            catalogue::my_second_test::MY_T_B,
            "test.my_second_test.my_t_b",
        ),
        (
            catalogue::my_second_test::MY_T_H,
            "test.my_second_test.my_t_h",
        ),
        (
            catalogue::my_second_test::my_sub_sub::MY_S_T_A,
            "test.my_second_test.my_sub_sub.my_s_t_a",
        ),
        (
            catalogue::my_second_test::my_sub_sub::MY_S_T_B,
            "test.my_second_test.my_sub_sub.my_s_t_b",
        ),
        (catalogue::MY_FULL_COUNTER, "test.my_full_counter"),
        (catalogue::MY_H_30, "test.my_h_30"),
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
        (catalogue::my_second_test::MY_T_B, &t.my_second_test.my_t_b),
        (
            catalogue::my_second_test::my_sub_sub::MY_S_T_B,
            &t.my_second_test.my_sub_sub.my_s_t_b,
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
        ("test.my_non_g", &t._my_non_g),
        ("test.my_hidden_sub.my_b", &t._my_hidden_sub.my_t_b),
        (
            "test.my_hidden_sub.my_sub_sub.my_s_t_b",
            &t._my_hidden_sub.my_sub_sub.my_s_t_b,
        ),
    ];
    for (key, field) in hidden_counters {
        let pre = field.read();
        t.increment_counter(&Key::from_name(key), 1);
        assert_eq!(field.read(), pre, "key {} updated the counter", key);
    }
}

#[test]
fn histograms() {
    let t = Test::new();
    let registered_histograms = [
        (catalogue::MY_H_60, &t.my_h_60 as &dyn HistogramMetric),
        (catalogue::MY_H_30, &t.my_h_30 as &dyn HistogramMetric),
        (
            catalogue::my_test::MY_T_H,
            &t.my_test.my_t_h as &dyn HistogramMetric,
        ),
        (
            catalogue::my_second_test::MY_T_H,
            &t.my_second_test.my_t_h as &dyn HistogramMetric,
        ),
    ];
    for (key, field) in registered_histograms {
        assert!(field.read().is_empty(), "Histogram was not empty initially");
        t.record_histogram(&Key::from_name(key), 1.0);
        let samples = field.read();
        assert_eq!(
            samples,
            vec![1.0f64],
            "key {} did not update the histogram",
            key
        );
    }
}
