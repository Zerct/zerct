use super::{flags, model::CliOptions};
use crate::cli::errors::{Result, agent_error};

pub(crate) fn parse_args(argv: &[String]) -> Result<CliOptions> {
    let mut cli = CliOptions::default();
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < argv.len() {
        let arg = argv[index].clone();
        if arg == "--" {
            positional.extend(argv.iter().skip(index + 1).cloned());
            break;
        }
        if let Some((name, inline)) = flags::parse_flag(&arg) {
            let consumed = flags::apply_flag(&mut cli, name, inline, argv, index)?;
            index += consumed;
        } else if arg.starts_with('-') {
            return Err(agent_error(
                "unknown_argument",
                format!("Unknown Tovuk option: {arg}."),
                "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
                cli.output.json,
            ));
        } else {
            positional.push(arg);
            index += 1;
        }
    }

    if let Some(command) = positional.first() {
        cli.command.clone_from(command);
        cli.args = positional.into_iter().skip(1).collect();
    }
    cli.api_url
        .truncate(cli.api_url.trim_end_matches('/').len());
    Ok(cli)
}
