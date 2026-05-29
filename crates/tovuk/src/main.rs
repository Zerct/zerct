//! Native Tovuk command line interface.

mod cli;

fn main() -> std::process::ExitCode {
    cli::entrypoint()
}
