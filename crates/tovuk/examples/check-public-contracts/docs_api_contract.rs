use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, reject_contains, require_contains, require_contains_all},
};

pub(crate) fn require_support_pricing_and_openapi(sources: &DocsSources) -> CheckResult {
    require_contains_all(
        sources.status.as_str(),
        &[("tovuk scraper health --json", "status scraper health docs")],
    )?;
    require_contains_all(
        sources.support.as_str(),
        &[
            ("tovuk support create", "support create docs"),
            ("POST /v1/support/tickets", "support API create docs"),
            ("account API key", "support API key docs"),
            ("request_id", "support API request id docs"),
        ],
    )?;
    require_pricing_contract(sources.pricing.as_str())?;
    require_openapi_paths(sources.openapi.as_str())?;
    require_openapi_account_profile_contract(sources.openapi.as_str())?;
    require_openapi_api_key_contract(sources.openapi.as_str())?;
    require_openapi_billing_contract(sources.openapi.as_str())?;
    require_openapi_status_checks(sources.openapi.as_str())?;
    require_contains_all(
        sources.openapi.as_str(),
        &[
            (
                "users and AI/API agents can open service tickets",
                "OpenAPI support agent create description",
            ),
            (
                r#""created_by""#,
                "OpenAPI support creator attribution field",
            ),
            (r#""request_id""#, "OpenAPI support request id context"),
            (
                r#""linkedinPostSearch""#,
                "OpenAPI LinkedIn post search example",
            ),
            (
                r#""author_company_urns""#,
                "OpenAPI LinkedIn author company filter",
            ),
            (
                r#""linkedinCompanyEmployees""#,
                "OpenAPI LinkedIn company employees example",
            ),
        ],
    )
}

fn require_pricing_contract(pricing: &str) -> CheckResult {
    require_contains_all(
        pricing,
        &[
            (
                "There is no free scraper tier",
                "pricing paid-only scraper docs",
            ),
            ("| Plus | `$20/month` | `$20`", "pricing Plus balance docs"),
            ("| Pro | `$100/month` | `$120`", "pricing Pro balance docs"),
            ("| Max | `$200/month` | `$300`", "pricing Max balance docs"),
            (
                "deducts from that balance for each successful stored",
                "pricing balance debit docs",
            ),
            (
                "`priceEvents[].usdMicros`",
                "pricing scraper event price docs",
            ),
            (
                "| Google Maps Scraper | place | `$2.10` |",
                "pricing Google Maps per-result docs",
            ),
            (
                "| TikTok Scraper | record | `$1.70` |",
                "pricing TikTok per-result docs",
            ),
            (
                "| Instagram Scraper | record | `$0.80` |",
                "pricing Instagram per-result docs",
            ),
        ],
    )
}

fn require_openapi_account_profile_contract(openapi: &str) -> CheckResult {
    require_contains_all(
        openapi,
        &[
            (
                r#""AccountProfileResponse""#,
                "OpenAPI account profile schema",
            ),
            (r#""accountId""#, "OpenAPI account id profile field"),
            (r#""email""#, "OpenAPI account email profile field"),
            (r#""provider""#, "OpenAPI account provider profile field"),
            (r#""plan""#, "OpenAPI account plan profile field"),
            (r#""unpaid""#, "OpenAPI unpaid account plan enum"),
            (r#""plus""#, "OpenAPI Plus account plan enum"),
            (r#""pro""#, "OpenAPI Pro account plan enum"),
            (r#""max""#, "OpenAPI Max account plan enum"),
            (r#""displayName""#, "OpenAPI display name profile field"),
        ],
    )?;
    for retired in [r#""free""#, r#""handle""#, r#""billingActive""#] {
        reject_contains(
            openapi,
            retired,
            format!("retired account profile field or plan value {retired}").as_str(),
        )?;
    }
    Ok(())
}

fn require_openapi_paths(openapi: &str) -> CheckResult {
    for path in [
        r#""/health""#,
        r#""/healthz""#,
        r#""/v1/status""#,
        r#""/v1/login/device""#,
        r#""/v1/login/device/{device_code}""#,
        r#""/v1/account""#,
        r#""/v1/account/activity""#,
        r#""/v1/account/api-keys""#,
        r#""/v1/account/api-keys/{key_id}""#,
        r#""/v1/scrapers""#,
        r#""/v1/scrapers/health""#,
        r#""/v1/scrapers/{scraper}""#,
        r#""/v1/requests""#,
        r#""/v1/requests/{request_id}""#,
        r#""/v1/requests/{request_id}/cancel""#,
        r#""/v1/requests/{request_id}/results""#,
        r#""/v1/usage""#,
        r#""/v1/billing/checkout""#,
        r#""/v1/billing/portal""#,
        r#""/v1/support/tickets""#,
        r#""/v1/support/tickets/{ticket_id}/resolve""#,
    ] {
        require_contains(
            openapi,
            path,
            format!("OpenAPI scraper-only path {path}").as_str(),
        )?;
    }
    Ok(())
}

fn require_openapi_api_key_contract(openapi: &str) -> CheckResult {
    require_contains_all(
        openapi,
        &[
            (
                r#""AccountApiKeySummary""#,
                "OpenAPI API key summary schema",
            ),
            (
                r#""AccountApiKeysResponse""#,
                "OpenAPI API key list response schema",
            ),
            (
                r#""AccountApiKeyCreateRequest""#,
                "OpenAPI API key create request schema",
            ),
            (
                r#""AccountApiKeyCreateResponse""#,
                "OpenAPI API key create response schema",
            ),
            (
                r#""AccountApiKeyRevokeResponse""#,
                "OpenAPI API key revoke response schema",
            ),
            (r#""tokenPrefix""#, "OpenAPI API key token prefix field"),
            (
                r#""currentDayRequestCount""#,
                "OpenAPI API key day usage field",
            ),
            (
                r#""currentMonthRequestCount""#,
                "OpenAPI API key month usage field",
            ),
            (
                r#""operationId": "listAccountApiKeys""#,
                "OpenAPI API key list operation",
            ),
            (
                r#""operationId": "createAccountApiKey""#,
                "OpenAPI API key create operation",
            ),
            (
                r#""operationId": "revokeAccountApiKey""#,
                "OpenAPI API key revoke operation",
            ),
        ],
    )
}

fn require_openapi_billing_contract(openapi: &str) -> CheckResult {
    require_contains_all(
        openapi,
        &[
            (
                r#""BillingCheckoutResponse""#,
                "OpenAPI billing checkout response schema",
            ),
            (r#""target_plan""#, "OpenAPI billing checkout plan field"),
            (
                r#""top_up_usd_cents""#,
                "OpenAPI billing checkout top-up field",
            ),
            (
                r#""checkout""#,
                "OpenAPI billing response checkout envelope",
            ),
            ("Manage Tovuk billing.", "OpenAPI portal response reason"),
        ],
    )?;
    reject_contains(
        openapi,
        r#""BillingPortalResponse""#,
        "OpenAPI must not document retired portal response envelope",
    )?;
    reject_contains(
        openapi,
        r#""portal":"#,
        "OpenAPI billing portal must use the checkout envelope returned by the API",
    )?;
    reject_contains(
        openapi,
        r#""portal": "#,
        "OpenAPI billing portal must use the checkout envelope returned by the API",
    )
}

fn require_openapi_status_checks(openapi: &str) -> CheckResult {
    require_contains_all(
        openapi,
        &[
            (
                r#""name": "control_plane_sqlite""#,
                "OpenAPI status control-plane SQLite check",
            ),
            (r#""name": "redis""#, "OpenAPI status Redis check"),
        ],
    )?;
    reject_contains(
        openapi,
        r#""name": "database""#,
        "OpenAPI status must not expose generic database product wording",
    )
}
