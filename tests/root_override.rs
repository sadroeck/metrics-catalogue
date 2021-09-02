use metrics_catalogue::*;

#[derive(Catalogue)]
#[metric(root, "my_test")]
pub struct CustomRoot {
    bar: Bar,
}

#[derive(Catalogue)]
pub struct Bar {
    counter: Counter,
}

#[test]
fn root_override() {
    let t = CustomRoot::new();
    assert_eq!(my_test::bar::COUNTER, "my_test.bar.counter");
    let registered_counters = [(my_test::bar::COUNTER, &t.bar.counter)];
    for (key, field) in registered_counters {
        let pre = field.read();
        t.increment_counter(&Key::from_name(key), 1);
        assert_eq!(field.read(), pre + 1, "key {} did not update the counter", key);
    }
}
