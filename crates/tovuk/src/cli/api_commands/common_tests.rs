use crate::cli::args::options_for_test;

use super::{joined_args, optional_trimmed_value, page_query, required_arg};

#[test]
/// Verifies joined positional arguments are normalized.
///
/// # Panics
///
/// Panics when blank values remain or non-blank values are not joined.
fn joined_args_trims_and_drops_empty_values() {
    let cli = options_for_test(&["test", "create", " first ", " ", "second"]);

    assert_eq!(joined_args(&cli, 0x0001), "first second");
}

#[test]
/// Verifies optional values reject blank input and trim valid input.
///
/// # Panics
///
/// Panics when optional value normalization changes.
fn optional_trimmed_value_drops_blank_input() {
    assert_eq!(optional_trimmed_value(" value "), Some("value".to_owned()));
    assert_eq!(optional_trimmed_value(" "), None);
}

#[test]
/// Verifies page queries encode pagination values.
///
/// # Panics
///
/// Panics when pagination output is not URL encoded.
fn page_query_encodes_limit_and_cursor() {
    let cli = options_for_test(&[
        "request",
        "list",
        "--limit",
        "25",
        "--cursor",
        "after value",
    ]);

    assert_eq!(page_query(&cli), "?limit=25&cursor=after%20value");
}

#[test]
/// Verifies required positional arguments use the requested offset.
///
/// # Panics
///
/// Panics when the requested argument is not returned.
fn required_arg_reads_requested_position() {
    let cli = options_for_test(&["test", "show", "request_123"]);

    assert_eq!(
        required_arg(&cli, 0b1, ("missing", "missing", "retry")).ok(),
        Some("request_123".to_owned())
    );
}
