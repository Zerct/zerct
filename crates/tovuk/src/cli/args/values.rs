use crate::cli::errors::{Result, agent_error};

const LONG_FLAG_PREFIX: &str = "\x2d\x2d";

pub(super) fn set_string_flag(
    target: &mut String,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<usize> {
    *target = flag_value(name, inline, argv, index, json_output)?;
    Ok(flag_consumed(argv, index))
}

pub(super) fn set_boolean_flag(
    inline: Option<&String>,
    mut set: impl FnMut(),
    name: &str,
    json_output: bool,
) -> Result<usize> {
    if inline.is_some() {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} does not accept a value."),
            format!("Use {name} without =value."),
            json_output,
        ));
    }
    set();
    Ok(1)
}

pub(super) fn flag_value(
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<String> {
    let value = if let Some(value) = inline {
        value
    } else {
        argv.get(index + 1).cloned().ok_or_else(|| {
            agent_error(
                "missing_argument",
                format!("{name} requires a value."),
                format!("Pass a value after {name}."),
                json_output,
            )
        })?
    };
    if value.is_empty()
        || (!arg_has_inline_value(argv, index) && value.starts_with(LONG_FLAG_PREFIX))
    {
        return Err(agent_error(
            "missing_argument",
            format!("{name} requires a value."),
            format!("Pass a value after {name}."),
            json_output,
        ));
    }
    Ok(value)
}

pub(super) fn flag_consumed(argv: &[String], index: usize) -> usize {
    if arg_has_inline_value(argv, index) {
        1
    } else {
        2
    }
}

fn arg_has_inline_value(argv: &[String], index: usize) -> bool {
    argv.get(index)
        .is_some_and(|arg| arg.starts_with(LONG_FLAG_PREFIX) && arg.contains('='))
}
