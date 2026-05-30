use super::super::constants::{DEFAULT_API_URL, DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS};

#[derive(Clone, Debug)]
pub(crate) struct CliOptions {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) api_url: String,
    pub(crate) app: String,
    pub(crate) build: String,
    pub(crate) deploy: String,
    pub(crate) limit: String,
    pub(crate) cursor: String,
    pub(crate) failing_command: String,
    pub(crate) first_log_line: String,
    pub(crate) token: String,
    pub(crate) template: String,
    pub(crate) severity: String,
    pub(crate) port: u16,
    pub(crate) deployment: DeploymentOptions,
    pub(crate) storage: StorageOptions,
    pub(crate) output: OutputOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct DeploymentOptions {
    pub(crate) database: bool,
    pub(crate) wait: bool,
    pub(crate) wait_timeout_seconds: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct StorageOptions {
    pub(crate) content_type: String,
    pub(crate) public_read: bool,
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
            app: String::new(),
            build: String::new(),
            deploy: String::new(),
            limit: String::new(),
            cursor: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            token: String::new(),
            template: String::new(),
            severity: String::new(),
            port: 0,
            deployment: DeploymentOptions {
                database: false,
                wait: false,
                wait_timeout_seconds: DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS,
            },
            storage: StorageOptions {
                content_type: String::new(),
                public_read: false,
            },
            output: OutputOptions {
                json: false,
                help: false,
                version: false,
            },
        }
    }
}
