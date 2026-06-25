use std::path::Path;

use crate::{
    docs_sources::{DocsSources, openapi_config_path, read_navigation_pages},
    helpers::{CheckResult, file_exists, reject_contains, require_contains},
};

pub(crate) fn check() -> CheckResult {
    let pages = read_navigation_pages()?;
    require_navigation_pages_exist(&pages)?;
    let sources = DocsSources::load(&pages)?;
    require_navigation_contract(&sources)?;
    require_scraper_examples(&sources)?;
    require_support_pricing_and_openapi(&sources)?;
    reject_retired_docs_contracts(&sources)?;
    println!("Checked scraper-only docs, package copy, and OpenAPI contract.");
    Ok(())
}

pub(crate) fn print_openapi_path() -> CheckResult {
    let path = openapi_config_path()?;
    println!("{}", path.display());
    Ok(())
}

fn require_navigation_pages_exist(pages: &[String]) -> CheckResult {
    let mut missing_pages = Vec::new();
    for page in pages {
        if page.starts_with("http://") || page.starts_with("https://") {
            continue;
        }
        let page_path = Path::new("docs").join(format!("{page}.mdx"));
        if !file_exists(page_path.as_path()) {
            missing_pages.push(page_path.display().to_string());
        }
    }
    if missing_pages.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Missing Mintlify pages:\n{}",
            missing_pages.join("\n")
        ))
    }
}

fn require_navigation_contract(sources: &DocsSources) -> CheckResult {
    for page in [
        "index",
        "quickstart",
        "scrapers",
        "agents",
        "pricing",
        "status",
        "support",
        "changelog",
        "reference/packages",
    ] {
        require_contains(
            sources.nav_pages.as_str(),
            page,
            format!("Mintlify scraper-only navigation {page}").as_str(),
        )?;
    }
    for page in [
        "deploy",
        "templates",
        "production-readiness",
        "reference/project-contract",
        "reference/workers",
        "reference/resources",
        "reference/sqlite",
        "reference/state",
        "reference/kv",
        "reference/secrets",
        "reference/storage",
        "reference/queues",
        "reference/cron",
        "reference/bindings",
        "reference/domains",
        "reference/logs-builds",
        "reference/usage-caps",
    ] {
        reject_contains(
            sources.nav_pages.as_str(),
            page,
            format!("retired Mintlify navigation {page}").as_str(),
        )?;
    }
    Ok(())
}

fn require_scraper_examples(sources: &DocsSources) -> CheckResult {
    for (name, text) in [
        ("README", sources.readme.as_str()),
        ("scraper docs", sources.scrapers.as_str()),
        ("agents", sources.agents.as_str()),
        ("packages", sources.packages.as_str()),
        ("llms", sources.llms.as_str()),
        ("docs skill", sources.skill.as_str()),
        ("packaged skill", sources.packaged_skill.as_str()),
    ] {
        require_contains(
            text,
            "tovuk request create tiktok",
            format!("{name} TikTok example").as_str(),
        )?;
        require_contains(
            text,
            "tovuk request create github",
            format!("{name} GitHub example").as_str(),
        )?;
        require_contains(
            text,
            "tovuk request create linkedin",
            format!("{name} LinkedIn example").as_str(),
        )?;
        require_contains(
            text,
            "public data only",
            format!("{name} public-data policy").as_str(),
        )?;
    }
    Ok(())
}

fn require_support_pricing_and_openapi(sources: &DocsSources) -> CheckResult {
    require_contains(
        sources.status.as_str(),
        "tovuk scraper health --json",
        "status scraper health docs",
    )?;
    require_contains(
        sources.support.as_str(),
        "tovuk support create",
        "support create docs",
    )?;
    require_pricing_contract(sources.pricing.as_str())?;
    require_openapi_paths(sources.openapi.as_str())?;
    require_openapi_status_checks(sources.openapi.as_str())?;
    require_contains(
        sources.openapi.as_str(),
        r#""linkedinPostSearch""#,
        "OpenAPI LinkedIn post search example",
    )?;
    require_contains(
        sources.openapi.as_str(),
        r#""author_company_urns""#,
        "OpenAPI LinkedIn author company filter",
    )?;
    require_contains(
        sources.openapi.as_str(),
        r#""linkedinCompanyEmployees""#,
        "OpenAPI LinkedIn company employees example",
    )
}

fn require_pricing_contract(pricing: &str) -> CheckResult {
    for (snippet, label) in [
        (
            "There is no free scraper tier",
            "pricing paid-only scraper docs",
        ),
        ("| Pro | `$20/month` | `$20`", "pricing Pro balance docs"),
        (
            "| Business | `$100/month` | `$125`",
            "pricing Business balance docs",
        ),
        (
            "| Scale | `$200/month` | `$300`",
            "pricing Scale balance docs",
        ),
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
    ] {
        require_contains(pricing, snippet, label)?;
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
    require_contains(
        openapi,
        r#""name": "control_plane_sqlite""#,
        "OpenAPI status control-plane SQLite check",
    )?;
    require_contains(openapi, r#""name": "redis""#, "OpenAPI status Redis check")?;
    reject_contains(
        openapi,
        r#""name": "database""#,
        "OpenAPI status must not expose generic database product wording",
    )
}

fn reject_retired_docs_contracts(sources: &DocsSources) -> CheckResult {
    for retired in [
        r#""/v1/apps""#,
        r#""/v1/deploy""#,
        r#""/v1/deploys""#,
        r#""/v1/services""#,
        r#""/v1/builds""#,
        r#""/v1/capabilities""#,
        r#""/v1/usage/caps"#,
        r#""/v1/abuse"#,
        r#""/v1/operator/abuse"#,
        "DeployRequest",
        "DeployResponse",
        "ServicesResponse",
        "ServiceOverviewResponse",
        "StorageObjectsResponse",
        "SqliteQueryResponse",
        "QueueMessageSendRequest",
        "CronTrigger",
        "UsageCap",
        "TovukConfig",
    ] {
        reject_contains(
            sources.openapi.as_str(),
            retired,
            format!("retired public OpenAPI contract {retired}").as_str(),
        )?;
    }

    for retired in [
        "tovuk deploy",
        "tovuk service",
        "tovuk storage",
        "tovuk sqlite",
        "tovuk kv",
        "tovuk queue",
        "tovuk cron",
        "tovuk secrets",
        "tovuk domains",
        "tovuk limits",
        "tovuk nodes",
        "tovuk abuse",
        "tovuk.toml",
        "full-stack",
        "static frontend",
    ] {
        reject_contains(
            sources.public_copy.as_str(),
            retired,
            format!("retired public docs wording {retired}").as_str(),
        )?;
    }
    Ok(())
}
