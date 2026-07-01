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
#[path = "output_tests.rs"]
mod tests;
