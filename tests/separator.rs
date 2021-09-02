use metrics_catalogue::*;

#[derive(Catalogue)]
#[metric(root, separator = "-")]
pub struct CustomSeparatorFoo {
    bar: CustomSeparatorBar,
}

#[derive(Catalogue)]
pub struct CustomSeparatorBar {
    counter: Counter,
}

#[test]
fn custom_separator() {
    let _ = CustomSeparatorFoo::new();
    assert_eq!(
        custom_separator_foo::bar::COUNTER,
        "custom_separator_foo-bar-counter"
    );
}
