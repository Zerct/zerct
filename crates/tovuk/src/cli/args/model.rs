use super::super::constants::DEFAULT_API_URL;

#[derive(Clone, Debug)]
pub(crate) struct CliOptions {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) api_url: String,
    pub(crate) limit: String,
    pub(crate) cursor: String,
    pub(crate) failing_command: String,
    pub(crate) first_log_line: String,
    pub(crate) token: String,
    pub(crate) severity: String,
    pub(crate) account: AccountOptions,
    pub(crate) abuse: AbuseOptions,
    pub(crate) output: OutputOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct AccountOptions {
    pub(crate) handle: String,
    pub(crate) display_name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AbuseOptions {
    pub(crate) operator: bool,
    pub(crate) category: String,
    pub(crate) reporter_email: String,
    pub(crate) reporter_name: String,
    pub(crate) evidence: String,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputOptions {
    pub(crate) json: bool,
    pub(crate) help: bool,
    pub(crate) version: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: "help".to_owned(),
            args: Vec::new(),
            api_url: DEFAULT_API_URL.to_owned(),
            limit: String::new(),
            cursor: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            token: String::new(),
            severity: String::new(),
            account: AccountOptions {
                handle: String::new(),
                display_name: String::new(),
            },
            abuse: AbuseOptions {
                operator: false,
                category: String::new(),
                reporter_email: String::new(),
                reporter_name: String::new(),
                evidence: String::new(),
            },
            output: OutputOptions {
                json: false,
                help: false,
                version: false,
            },
        }
    }
}
