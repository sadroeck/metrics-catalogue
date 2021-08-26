#[cfg(feature = "prometheus")]
use metrics_catalogue::prometheus::StringRender;
use metrics_catalogue::*;

#[cfg(feature = "prometheus")]
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

    my_full_counter: ::metrics_catalogue::Counter,
    my_h_30: ::metrics_catalogue::Histogram<30>,
}

#[cfg(feature = "prometheus")]
#[derive(Catalogue)]
struct SubTest {
    my_t_a: Gauge,
    my_t_b: Counter,
    my_t_h: Histogram<5>,
}

#[cfg(feature = "prometheus")]
#[test]
fn prometheus_render() {
    use utils::*;
    let t = Test::new();
    let mut s = String::new();
    t.render("", "", &mut s);
    let lines = s
        .split('#')
        .flat_map(|x| x.split('\n'))
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>();

    // Metric types
    assert_type_in_output(&lines, "my_b", "counter");
    assert_type_in_output(&lines, "my_g", "gauge");
    assert_type_in_output(&lines, "my_discrete_g", "gauge");
    assert_type_in_output(&lines, "my_h_60", "histogram");

    // Metric values
    assert_value_in_output(&lines, "my_b");
    assert_value_in_output(&lines, "my_g");
    assert_quantile_in_output(&lines, "my_h_60");

    // Hidden metrics
    assert!(lines.iter().all(|l| !l.contains("_my_non_g")));
    assert!(lines.iter().all(|l| !l.contains("_my_hidden_sub")));

    // Sub metrics
    assert_type_in_output(&lines, "my_test.my_t_a", "gauge");
    assert_type_in_output(&lines, "my_test.my_t_b", "counter");
    assert_type_in_output(&lines, "my_test.my_t_h", "histogram");
}

#[cfg(feature = "prometheus")]
mod utils {
    #[inline]
    pub fn assert_type_in_output(lines: &[&str], name: &str, expected_type: &str) {
        let s = format!("TYPE {} {}", name, expected_type);
        assert!(
            lines.iter().any(|l| s == *l),
            "No {} {} in output",
            expected_type,
            name,
        );
    }

    #[inline]
    pub fn assert_value_in_output(lines: &[&str], name: &str) {
        let s = format!("{} 0", name);
        assert!(lines.iter().any(|l| s == *l), "No value {} in output", name);
    }

    #[inline]
    pub fn assert_quantile_in_output(lines: &[&str], name: &str) {
        for quantile in [0.0, 0.5, 0.9, 0.99] {
            let s = format!("{}{{quantile=\"{}\"}} 0", name, quantile);
            assert!(
                lines.iter().any(|l| s == *l),
                "No histogram {}(q={}) in output",
                name,
                quantile
            );
        }
    }
}
