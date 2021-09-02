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
    let _ = CustomRoot::new();
    assert_eq!(my_test::bar::COUNTER, "my_test.bar.counter");
}
