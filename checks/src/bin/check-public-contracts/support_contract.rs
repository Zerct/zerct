use crate::{
    cli_contract::ContractSources,
    helpers::{
        CheckResult, LabeledSnippet, reject_contains_any, require_contains, require_contains_all,
    },
};

/// Contract value named `ACCOUNT_TO_TOVUK_TICKET`.
const ACCOUNT_TO_TOVUK_TICKET: &str =
    "support surfaces must stay account-to-Tovuk service-ticket wording";

/// Contract value named `NON_SERVICE_REQUEST`.
const NON_SERVICE_REQUEST: &str =
    "support surfaces must describe service tickets, not non-service requests";

/// Contract value named `NON_SERVICE_TICKET_TERMS`.
const NON_SERVICE_TICKET_TERMS: &[LabeledSnippet] = &[
    ("abuse", NON_SERVICE_REQUEST),
    ("compliance complaint", NON_SERVICE_REQUEST),
    ("complaint", NON_SERVICE_REQUEST),
    ("moderation", NON_SERVICE_WORKFLOW),
    ("dispute", NON_SERVICE_WORKFLOW),
    (
        "customer-to-customer",
        "support surfaces must describe service tickets between the account and Tovuk",
    ),
    ("user-to-user report", ACCOUNT_TO_TOVUK_TICKET),
    ("report a user", ACCOUNT_TO_TOVUK_TICKET),
    ("report user", ACCOUNT_TO_TOVUK_TICKET),
    ("report another user", ACCOUNT_TO_TOVUK_TICKET),
    ("report customer", ACCOUNT_TO_TOVUK_TICKET),
    ("user report", ACCOUNT_TO_TOVUK_TICKET),
    ("customer report", ACCOUNT_TO_TOVUK_TICKET),
    ("reporting", ACCOUNT_TO_TOVUK_TICKET),
    ("report abuse", ACCOUNT_TO_TOVUK_TICKET),
];

/// Contract value named `NON_SERVICE_WORKFLOW`.
const NON_SERVICE_WORKFLOW: &str =
    "support surfaces must describe service tickets, not non-service workflows";

/// Contract value named `SUPPORT_COMMANDS`.
const SUPPORT_COMMANDS: &[&str] = &[
    "tovuk support create",
    "tovuk support list",
    "tovuk support resolve",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 6] = [
    size_of_val(&check),
    size_of_val(&reject_non_service_ticket_language),
    size_of_val(&require_account_to_tovuk_service_ticket_wording),
    size_of_val(&require_support_api_docs),
    size_of_val(&require_support_commands),
    size_of_val(&require_support_openapi_contract),
];

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check(sources: &ContractSources) -> CheckResult {
    check_try!(require_support_commands(sources));
    check_try!(require_support_api_docs(sources));
    check_try!(require_support_openapi_contract(sources));
    return reject_non_service_ticket_language(sources);
}

/// Contract implementation for `reject_non_service_ticket_language`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_non_service_ticket_language(sources: &ContractSources) -> CheckResult {
    for source in sources.support_public_sources() {
        check_try!(reject_contains_any(source, NON_SERVICE_TICKET_TERMS));
    }
    return Ok(());
}

/// Contract implementation for `require_account_to_tovuk_service_ticket_wording`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_account_to_tovuk_service_ticket_wording(source: &str) -> CheckResult {
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.contains("between your account and Tovuk")
        || normalized.contains("between the authenticated account and Tovuk")
    {
        return Ok(());
    }
    return Err("support tickets must be described as account-to-Tovuk service tickets".to_owned());
}

/// Contract implementation for `require_support_api_docs`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_support_api_docs(sources: &ContractSources) -> CheckResult {
    for source in sources.support_api_doc_sources() {
        check_try!(require_contains_all(
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
        ));
        check_try!(require_account_to_tovuk_service_ticket_wording(source));
    }
    return Ok(());
}

/// Contract implementation for `require_support_commands`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_support_commands(sources: &ContractSources) -> CheckResult {
    for source in sources.support_command_sources() {
        for snippet in SUPPORT_COMMANDS {
            check_try!(require_contains(
                source,
                snippet,
                format!("scraper-only public command {snippet}").as_str(),
            ));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_support_openapi_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_support_openapi_contract(sources: &ContractSources) -> CheckResult {
    let openapi = sources.support_openapi_source();
    return require_contains_all(
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
    );
}
