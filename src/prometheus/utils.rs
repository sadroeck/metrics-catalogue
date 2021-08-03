// pub(super) fn write_help_line(buffer: &mut String, name: &str, desc: &str) {
//     buffer.push_str("# HELP ");
//     buffer.push_str(name);
//     buffer.push(' ');
//     buffer.push_str(desc);
//     buffer.push('\n');
// }

pub(super) fn write_type_line(buffer: &mut String, name: &str, metric_type: &str) {
    buffer.push_str("# TYPE ");
    buffer.push_str(name);
    buffer.push(' ');
    buffer.push_str(metric_type);
    buffer.push('\n');
}

pub(super) fn write_metric_line<T, T2>(
    buffer: &mut String,
    name: &str,
    suffix: Option<&'static str>,
    labels: &[String],
    additional_label: Option<(&'static str, T)>,
    value: T2,
) where
    T: std::fmt::Display,
    T2: std::fmt::Display,
{
    buffer.push_str(name);
    if let Some(suffix) = suffix {
        buffer.push('_');
        buffer.push_str(suffix)
    }

    if !labels.is_empty() || additional_label.is_some() {
        buffer.push('{');

        let mut first = true;
        for label in labels {
            if first {
                first = false;
            } else {
                buffer.push(',');
            }
            buffer.push_str(label);
        }

        if let Some((name, value)) = additional_label {
            if !first {
                buffer.push(',');
            }
            buffer.push_str(name);
            buffer.push_str("=\"");
            buffer.push_str(value.to_string().as_str());
            buffer.push('"');
        }

        buffer.push('}');
    }

    buffer.push(' ');
    buffer.push_str(value.to_string().as_str());
    buffer.push('\n');
}
