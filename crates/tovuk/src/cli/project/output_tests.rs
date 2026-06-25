use crate::cli::args::CliOptions;

use super::progress_message;

#[test]
fn json_output_suppresses_human_progress() {
    let mut cli = CliOptions::default();
    cli.output.json = true;

    assert_eq!(progress_message(&cli, "build job_1 running"), None);
}

#[test]
fn text_output_keeps_human_progress() {
    let cli = CliOptions::default();

    assert_eq!(
        progress_message(&cli, "build job_1 running"),
        Some("build job_1 running")
    );
}
