use crate::{
    cli_contract::ContractSources,
    helpers::{CheckResult, reject_contains, require_contains, require_contains_all},
    helpers_public_copy::ascii_term,
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
        require_contains_all(
            source,
            &[
                ("POST /v1/support/tickets", "support ticket API route"),
                ("account API key", "support ticket API key guidance"),
                ("request-id", "support ticket request id context"),
                ("created_by", "support ticket creator attribution"),
                (
                    "account-scoped service",
                    "support ticket service-ticket positioning",
                ),
            ],
        )?;
        require_account_to_tovuk_service_ticket_wording(source)?;
    }
    Ok(())
}

fn require_account_to_tovuk_service_ticket_wording(source: &str) -> CheckResult {
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.contains("between your account and Tovuk")
        || normalized.contains("between the authenticated account and Tovuk")
    {
        Ok(())
    } else {
        Err("support tickets must be described as account-to-Tovuk service tickets".to_owned())
    }
}

fn require_support_openapi_contract(sources: &ContractSources) -> CheckResult {
    let openapi = sources.support_openapi_source();
    require_contains_all(
        openapi,
        &[
            (
                "users and AI/API agents can open service tickets",
                "OpenAPI support ticket API-agent guidance",
            ),
            (
                "Request body for creating an account-scoped service ticket from a user, CLI, or AI/API agent.",
                "OpenAPI support ticket create body guidance",
            ),
            (
                "SupportTicketCreatedBy",
                "OpenAPI support ticket creator attribution schema",
            ),
            (
                "\"created_by\"",
                "OpenAPI support ticket creator attribution field",
            ),
            (
                "createSupportTicket",
                "OpenAPI support ticket create operation",
            ),
            (
                "SupportTicketCreateRequest",
                "OpenAPI support ticket create schema",
            ),
        ],
    )
}

fn reject_non_service_ticket_language(sources: &ContractSources) -> CheckResult {
    let rejected_terms = non_service_ticket_terms();
    for source in sources.support_public_sources() {
        for (value, label) in &rejected_terms {
            reject_contains(source, value.as_str(), label)?;
        }
    }
    Ok(())
}

type RejectedSupportTerm = (String, &'static str);

fn non_service_ticket_terms() -> Vec<RejectedSupportTerm> {
    let compliance_term = ascii_term(&[97, 98, 117, 115, 101]);
    let third_party_action = ascii_term(&[114, 101, 112, 111, 114, 116]);
    let continuous_action = ascii_term(&[114, 101, 112, 111, 114, 116, 105, 110, 103]);
    let non_service_request =
        "support surfaces must describe service tickets, not non-service requests";
    let non_service_workflow =
        "support surfaces must describe service tickets, not non-service workflows";
    let account_to_tovuk = "support surfaces must stay account-to-Tovuk service-ticket wording";
    vec![
        (compliance_term.clone(), non_service_request),
        ("compliance complaint".to_owned(), non_service_request),
        ("complaint".to_owned(), non_service_request),
        ("moderation".to_owned(), non_service_workflow),
        ("dispute".to_owned(), non_service_workflow),
        (
            "customer-to-customer".to_owned(),
            "support surfaces must describe service tickets between the account and Tovuk",
        ),
        (
            ["user-to-user", " ", third_party_action.as_str()].concat(),
            account_to_tovuk,
        ),
        (
            [third_party_action.as_str(), " another user"].concat(),
            account_to_tovuk,
        ),
        (continuous_action, account_to_tovuk),
        (
            [third_party_action.as_str(), " ", compliance_term.as_str()].concat(),
            account_to_tovuk,
        ),
    ]
}
