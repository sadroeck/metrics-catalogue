use metrics_catalogue::*;

#[derive(Catalogue)]
#[metric(root, "")]
pub struct EmptyRoot {
    bar: Bar,
}

#[derive(Catalogue)]
pub struct Bar {
    counter: Counter,
}

#[test]
fn empty_override() {
    let t = EmptyRoot::new();
    assert_eq!(empty_root::bar::COUNTER, "bar.counter");

    let registered_counters = [(empty_root::bar::COUNTER, &t.bar.counter)];
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
