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
