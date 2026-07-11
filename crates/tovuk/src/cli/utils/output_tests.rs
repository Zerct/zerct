use crate::cli::args::parse_args;

use super::ProgressMessage;

#[test]
/// Verifies JSON output suppresses human progress messages.
///
/// # Panics
///
/// Panics when JSON arguments do not parse or progress remains visible.
fn json_output_suppresses_human_progress() {
    let arguments = ["--json".to_owned()];
    let parsed = parse_args(&arguments);
    assert!(parsed.is_ok(), "JSON arguments should parse");
    let Some(cli) = parsed.ok() else {
        return;
    };

    assert_eq!(ProgressMessage::from((&cli, "waiting for login")).0, None);
}

#[test]
/// Verifies text output retains human progress messages.
///
/// # Panics
///
/// Panics when empty arguments do not parse or progress is suppressed.
fn text_output_keeps_human_progress() {
    let parsed = parse_args(&[]);
    assert!(parsed.is_ok(), "empty arguments should parse");
    let Some(cli) = parsed.ok() else {
        return;
    };

    assert_eq!(
        ProgressMessage::from((&cli, "waiting for login")).0,
        Some("waiting for login")
    );
}
