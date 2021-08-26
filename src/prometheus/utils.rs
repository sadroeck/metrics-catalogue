use std::fmt::Display;

// pub(super) fn write_help_line(buffer: &mut String, name: &str, desc: &str) {
//     buffer.push_str("# HELP ");
//     buffer.push_str(name);
//     buffer.push(' ');
//     buffer.push_str(desc);
//     buffer.push('\n');
// }

pub enum Label<T: Display> {
    #[allow(dead_code)]
    Key(String),
    KeyValue((&'static str, T)),
}

pub(super) fn write_type_line(buffer: &mut String, name: &str, metric_type: &str) {
    buffer.push_str("# TYPE ");
    buffer.push_str(name);
    buffer.push(' ');
    buffer.push_str(metric_type);
    buffer.push('\n');
}

pub(super) fn write_metric_line<L, V, LB>(
    buffer: &mut String,
    name: &str,
    suffix: Option<&'static str>,
    labels: LB,
    value: V,
) where
    L: Display,
    V: Display,
    LB: Iterator<Item = Label<L>>,
{
    buffer.push_str(name);
    if let Some(suffix) = suffix {
        buffer.push('_');
        buffer.push_str(suffix)
    }

    let mut first = true;
    for label in labels {
        if first {
            buffer.push('{');
            first = false;
        } else {
            buffer.push(',');
        }
        match label {
            Label::Key(label) => {
                buffer.push_str(&label);
            }
            Label::KeyValue((name, value)) => {
                buffer.push_str(name);
                buffer.push_str("=\"");
                buffer.push_str(value.to_string().as_str());
                buffer.push('"');
            }
        }
    }
    if !first {
        buffer.push('}');
    }

    buffer.push(' ');
    buffer.push_str(value.to_string().as_str());
    buffer.push('\n');
}
