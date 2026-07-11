use crate::cli::errors::{CliError, OutputFormat, Result, agent_error};

/// Prefix used by long-form command-line flags.
const LONG_FLAG_PREFIX: &str = "\x2d\x2d";

#[derive(Debug)]
/// Validated value extracted for a command-line flag.
pub(super) struct FlagValue(String);

impl<'input>
    TryFrom<(
        String,
        Option<String>,
        &'input [String],
        usize,
        OutputFormat,
    )> for FlagValue
{
    type Error = CliError;

    fn try_from(
        value: (
            String,
            Option<String>,
            &'input [String],
            usize,
            OutputFormat,
        ),
    ) -> Result<Self> {
        let (name, inline, argv, index, output_format) = value;
        let parsed = inline
            .or_else(|| return argv.get(index.saturating_add(0b1)).cloned())
            .unwrap_or_default();
        if parsed.is_empty()
            || (!arg_has_inline_value(argv, index) && parsed.starts_with(LONG_FLAG_PREFIX))
        {
            return Err(agent_error(
                "missing_argument",
                format!("{} requires a value.", name.as_str()),
                format!("Pass a value after {}.", name.as_str()),
                output_format,
            ));
        }
        return Ok(Self(parsed));
    }
}

impl From<FlagValue> for String {
    #[inline]
    fn from(value: FlagValue) -> Self {
        return value.0;
    }
}

/// Reports whether the indexed argument contains an inline long-flag value.
pub(super) fn arg_has_inline_value(argv: &[String], index: usize) -> bool {
    return argv
        .get(index)
        .is_some_and(|arg| return arg.starts_with(LONG_FLAG_PREFIX) && arg.contains('='));
}

/// Applies a boolean flag after rejecting inline values.
///
/// # Errors
///
/// Returns an error when an inline value was supplied.
pub(super) fn set_boolean_flag(
    inline: Option<&String>,
    mut set: impl FnMut(),
    name: &str,
    output_format: OutputFormat,
) -> Result<usize> {
    if inline.is_some() {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} does not accept a value."),
            format!("Use {name} without =value."),
            output_format,
        ));
    }
    set();
    return Ok(0b1);
}
