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
    require_support_openapi_contract(sources)?;
    reject_non_service_ticket_language(sources)
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
        require_contains(
            source,
            "account-scoped service",
            "support ticket service-ticket positioning",
        )?;
    }
    Ok(())
}

fn require_support_openapi_contract(sources: &ContractSources) -> CheckResult {
    let openapi = sources.support_openapi_source();
    for (snippet, label) in [
        (
            "users and API agents can open service tickets",
            "OpenAPI support ticket API-agent guidance",
        ),
        (
            "Request body for creating an account-scoped service ticket from a user, CLI, or API agent.",
            "OpenAPI support ticket create body guidance",
        ),
        (
            "createSupportTicket",
            "OpenAPI support ticket create operation",
        ),
        (
            "SupportTicketCreateRequest",
            "OpenAPI support ticket create schema",
        ),
    ] {
        require_contains(openapi, snippet, label)?;
    }
    Ok(())
}

fn reject_non_service_ticket_language(sources: &ContractSources) -> CheckResult {
    let banned_compliance_term = banned_compliance_term()?;
    let banned_user_workflow_term = ["user-to-user", " ", "report"].concat();
    let banned_direct_report_term = ["report", " ", "another", " ", "user"].concat();
    let banned_compliance_report_term = ["report", " ", banned_compliance_term.as_str()].concat();
    for source in sources.support_public_sources() {
        for (term, label) in [
            (
                banned_compliance_term.as_str(),
                "support surfaces must describe service tickets, not non-service requests",
            ),
            (
                "compliance complaint",
                "support surfaces must describe service tickets, not non-service requests",
            ),
            (
                "complaint",
                "support surfaces must describe service tickets, not non-service requests",
            ),
            (
                "moderation",
                "support surfaces must describe service tickets, not non-service workflows",
            ),
            (
                "dispute",
                "support surfaces must describe service tickets, not non-service workflows",
            ),
            (
                "customer-to-customer",
                "support surfaces must describe service tickets between the account and Tovuk",
            ),
            (
                banned_user_workflow_term.as_str(),
                "support surfaces must stay account-to-Tovuk service-ticket wording",
            ),
            (
                banned_direct_report_term.as_str(),
                "support surfaces must stay account-to-Tovuk service-ticket wording",
            ),
            (
                "reporting",
                "support surfaces must stay account-to-Tovuk service-ticket wording",
            ),
            (
                banned_compliance_report_term.as_str(),
                "support surfaces must stay account-to-Tovuk service-ticket wording",
            ),
        ] {
            reject_contains(source, term, label)?;
        }
    }
    Ok(())
}

fn banned_compliance_term() -> CheckResult<String> {
    String::from_utf8(vec![97, 98, 117, 115, 101])
        .map_err(|error| format!("invalid support service-ticket term check: {error}"))
}
