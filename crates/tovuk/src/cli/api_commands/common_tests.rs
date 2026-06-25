use crate::cli::args::CliOptions;

use super::{joined_args, optional_trimmed_value, required_arg};

#[test]
fn joined_args_trims_and_drops_empty_values() {
    let cli = CliOptions {
        args: vec![
            "create".to_owned(),
            " first ".to_owned(),
            " ".to_owned(),
            "second".to_owned(),
        ],
        ..CliOptions::default()
    };

    assert_eq!(joined_args(&cli, 1), "first second");
}

#[test]
fn optional_trimmed_value_drops_blank_input() {
    assert_eq!(optional_trimmed_value(" value "), Some("value".to_owned()));
    assert_eq!(optional_trimmed_value(" "), None);
}

#[test]
fn required_arg_reads_requested_position() {
    let cli = CliOptions {
        args: vec!["show".to_owned(), "request_123".to_owned()],
        ..CliOptions::default()
    };

    assert_eq!(
        required_arg(&cli, 1, "missing", "missing", "retry").ok(),
        Some("request_123".to_owned())
    );
}
