use super::super::constants::DEFAULT_API_URL;

#[derive(Clone, Debug)]
pub(crate) struct CliOptions {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) api_url: String,
    pub(crate) limit: String,
    pub(crate) cursor: String,
    pub(crate) top_up_usd_cents: String,
    pub(crate) failing_command: String,
    pub(crate) first_log_line: String,
    pub(crate) request_id: String,
    pub(crate) scraper_id: String,
    pub(crate) token: String,
    pub(crate) severity: String,
    pub(crate) output: OutputOptions,
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
            top_up_usd_cents: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            request_id: String::new(),
            scraper_id: String::new(),
            token: String::new(),
            severity: String::new(),
            output: OutputOptions {
                json: false,
                help: false,
                version: false,
            },
        }
    }
}
