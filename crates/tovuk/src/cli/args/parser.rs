use super::{flags, model::CliOptions};
use crate::cli::errors::{Result, agent_error};
use std::env;

const END_OF_OPTIONS: &str = "\x2d\x2d";
const OUTPUT_ENV: &str = "TOVUK_OUTPUT";

pub(crate) fn parse_args(argv: &[String]) -> Result<CliOptions> {
    let mut cli = CliOptions::default();
    apply_output_env(&mut cli)?;
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < argv.len() {
        let arg = argv[index].clone();
        if arg == END_OF_OPTIONS {
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

fn apply_output_env(cli: &mut CliOptions) -> Result<()> {
    let Ok(value) = env::var(OUTPUT_ENV) else {
        return Ok(());
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    flags::set_output_format(cli, value, OUTPUT_ENV, false)
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    fn args(values: &[&str]) -> Vec<String> {
        values
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }

    #[test]
    fn output_json_enables_json_output() {
        let parsed = parse_args(&args(&["check", "--output", "json"]));

        assert!(
            parsed
                .as_ref()
                .is_ok_and(|cli| cli.output.json && cli.command == "check")
        );
    }

    #[test]
    fn output_text_overrides_json_flag() {
        let parsed = parse_args(&args(&["check", "--json", "--output=text"]));

        assert!(parsed.as_ref().is_ok_and(|cli| !cli.output.json));
    }

    #[test]
    fn invalid_output_value_is_rejected() {
        let parsed = parse_args(&args(&["check", "--output", "yaml"]));

        assert!(
            parsed
                .as_ref()
                .is_err_and(|error| error.payload().code == "invalid_argument")
        );
    }
}
