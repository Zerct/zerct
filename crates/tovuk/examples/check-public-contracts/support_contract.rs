use crate::{
    cli_contract::ContractSources,
    helpers::{CheckResult, reject_contains, require_contains},
};

const SUPPORT_COMMANDS: &[&str] = &[
    "tovuk support create",
    "tovuk support list",
    "tovuk support resolve",
];

pub(crate) fn check(sources: &ContractSources) -> CheckResult {
    require_support_commands(sources)?;
    require_support_api_docs(sources)?;
    reject_support_retired_language(sources)
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

fn reject_support_retired_language(sources: &ContractSources) -> CheckResult {
    let retired_moderation_term = retired_moderation_term()?;
    let retired_user_workflow_term = ["user-to-user", " ", "report"].concat();
    let retired_direct_report_term = ["report", " ", "another", " ", "user"].concat();
    for source in sources.support_public_sources() {
        reject_contains(
            source,
            retired_moderation_term.as_str(),
            "support surfaces must not mention retired moderation workflow wording",
        )?;
        reject_contains(
            source,
            retired_user_workflow_term.as_str(),
            "support surfaces must not use retired customer-to-customer workflow wording",
        )?;
        reject_contains(
            source,
            retired_direct_report_term.as_str(),
            "support surfaces must not describe reporting another user",
        )?;
    }
    Ok(())
}

fn retired_moderation_term() -> CheckResult<String> {
    String::from_utf8(vec![97, 98, 117, 115, 101])
        .map_err(|error| format!("invalid retired support term check: {error}"))
}
