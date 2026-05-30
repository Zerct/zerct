use super::super::args::CliOptions;

pub(crate) fn progress(cli: &CliOptions, message: &str) {
    if cli.output.json {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}
