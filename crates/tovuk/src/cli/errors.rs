use super::{Deserialize, Serialize, Value, fmt};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AgentErrorPayload {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) agent_instruction: Option<String>,
    pub(crate) docs_url: Option<String>,
    pub(crate) checkout_url: Option<String>,
}

#[derive(Debug)]
pub(crate) struct CliFailure {
    pub(crate) payload: AgentErrorPayload,
    pub(crate) json: bool,
    pub(crate) exit_code: u8,
}

#[derive(Debug)]
pub(crate) struct CliError(Box<CliFailure>);

pub(crate) type Result<T> = std::result::Result<T, CliError>;

impl CliError {
    pub(crate) fn new(failure: CliFailure) -> Self {
        Self(Box::new(failure))
    }

    pub(crate) fn print(&self) {
        print_agent_error(&self.0.payload, self.0.json);
    }

    pub(crate) fn exit_code(&self) -> u8 {
        self.0.exit_code
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.payload.message)
    }
}

impl std::error::Error for CliError {}

pub(crate) fn agent_error(
    code: impl Into<String>,
    message: impl Into<String>,
    agent_instruction: impl Into<String>,
    json_output: bool,
) -> CliError {
    CliError::new(CliFailure {
        payload: AgentErrorPayload {
            code: code.into(),
            message: message.into(),
            agent_instruction: Some(agent_instruction.into()),
            docs_url: None,
            checkout_url: None,
        },
        json: json_output,
        exit_code: 1,
    })
}

pub(crate) fn internal_error(message: impl Into<String>) -> CliError {
    agent_error(
        "internal_error",
        message.into(),
        "Retry the command. If it keeps failing, create a Tovuk support ticket with command output.",
        false,
    )
}

pub(crate) fn print_agent_error(payload: &AgentErrorPayload, json_output: bool) {
    if json_output {
        match serde_json::to_string_pretty(payload) {
            Ok(source) => eprintln!("{source}"),
            Err(error) => eprintln!("Tovuk command failed: {error}"),
        }
        return;
    }

    eprintln!("{}", payload.message);
    if let Some(instruction) = payload
        .agent_instruction
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("agent_instruction: {instruction}");
    }
    if let Some(docs_url) = payload
        .docs_url
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("docs: {docs_url}");
    }
    if let Some(checkout_url) = payload
        .checkout_url
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        eprintln!("checkout: {checkout_url}");
    }
}

pub(crate) fn print_json(value: &Value) -> Result<()> {
    let source =
        serde_json::to_string_pretty(value).map_err(|error| internal_error(error.to_string()))?;
    println!("{source}");
    Ok(())
}
