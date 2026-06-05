use super::super::args::CliOptions;

pub(crate) fn progress(cli: &CliOptions, message: &str) {
    if progress_message(cli, message).is_some() {
        println!("{message}");
    }
}

fn progress_message<'a>(cli: &CliOptions, message: &'a str) -> Option<&'a str> {
    if cli.output.json { None } else { Some(message) }
}

#[cfg(test)]
mod tests {
    use super::progress_message;
    use crate::cli::args::CliOptions;

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
}
