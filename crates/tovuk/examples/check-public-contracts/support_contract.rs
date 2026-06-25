use crate::{
    cli_contract::ContractSources,
    helpers::{CheckResult, require_contains},
};

const SUPPORT_COMMANDS: &[&str] = &[
    "tovuk support create",
    "tovuk support list",
    "tovuk support resolve",
];

pub(crate) fn check(sources: &ContractSources) -> CheckResult {
    require_support_commands(sources)?;
    require_support_api_docs(sources)
}

fn require_support_commands(sources: &ContractSources) -> CheckResult {
    for source in sources.support_command_sources() {
        for snippet in SUPPORT_COMMANDS {
            require_contains(
                source,
                snippet,
                format!("scraper-only public command {snippet}").as_str(),
            )?;
        }
    }
    Ok(())
}

fn require_support_api_docs(sources: &ContractSources) -> CheckResult {
    for source in sources.support_api_doc_sources() {
        require_contains(
            source,
            "POST /v1/support/tickets",
            "support ticket API route",
        )?;
        require_contains(source, "account API key", "support ticket API key guidance")?;
        require_contains(source, "request-id", "support ticket request id context")?;
    }
    Ok(())
}
