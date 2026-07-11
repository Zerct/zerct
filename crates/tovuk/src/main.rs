//! Native Tovuk command line interface.

/// Native Tovuk command-line implementation.
#[path = "cli/module_root.rs"]
pub mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    return cli::Run::run(cli::Application);
}
