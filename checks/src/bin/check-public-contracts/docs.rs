use crate::{
    docs_api_contract::require_support_pricing_and_openapi,
    docs_navigation::{require_navigation_contract, require_navigation_pages_exist},
    docs_sources::{DocsSources, openapi_config_path, read_navigation_pages},
    helpers::{
        CheckResult, OutputChannel, read_json, read_text, reject_contains, require_contains,
        require_results, write_line,
    },
    retired_contracts::{
        RETIRED_OPENAPI_CONTRACTS, RETIRED_PUBLIC_COMMANDS, RETIRED_PUBLIC_DOCS_WORDING,
    },
    types::DocsJson,
};

/// Ecommerce output fields required by every public scraper-facing surface.
const ECOMMERCE_OUTPUT_FIELDS: &[&str] = &[
    "tags",
    "keywords",
    "questionSamples",
    "customerPhotoUrls",
    "minimumOrderQuantity",
    "rfqText",
    "resultPosition",
    "adPosition",
];

/// Ecommerce properties required in the public `OpenAPI` schema corpus.
const OPENAPI_ECOMMERCE_PROPERTIES: &[&str] = &[
    "\"tags\": {",
    "\"keywords\": {",
    "\"questionSamples\": {",
    "\"customerPhotoUrls\": {",
    "\"minimumOrderQuantity\": {",
    "\"rfqText\": {",
    "\"resultPosition\": {",
    "\"adPosition\": {",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&check),
    size_of_val(&print_openapi_path),
    size_of_val(&reject_retired_docs_contracts),
    size_of_val(&require_ecommerce_output_fields),
    size_of_val(&require_mintlify_exclusions),
    size_of_val(&require_output_fields_in_openapi),
    size_of_val(&require_scraper_examples),
    size_of_val(&require_scraper_examples_in_sources),
    size_of_val(&require_scraper_feature_coverage),
];

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    let pages = check_try!(read_navigation_pages());
    check_try!(require_navigation_pages_exist(&pages));
    check_try!(require_mintlify_exclusions());
    let sources = check_try!(DocsSources::load(&pages));
    check_try!(require_navigation_contract(&sources));
    check_try!(require_scraper_examples(&sources));
    check_try!(require_support_pricing_and_openapi(&sources));
    check_try!(reject_retired_docs_contracts(&sources));
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked scraper-only docs, package copy, and OpenAPI contract.",
    ));
    return Ok(());
}

/// Contract implementation for `print_openapi_path`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn print_openapi_path() -> CheckResult {
    let path = check_try!(openapi_config_path());
    check_try!(write_line(
        OutputChannel::Regular,
        path.display().to_string().as_str(),
    ));
    return Ok(());
}

/// Contract implementation for `reject_retired_docs_contracts`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_docs_contracts(sources: &DocsSources) -> CheckResult {
    for retired in RETIRED_OPENAPI_CONTRACTS {
        check_try!(reject_contains(
            sources.openapi.as_str(),
            retired,
            format!("retired public OpenAPI contract {retired}").as_str(),
        ));
    }

    for retired in RETIRED_PUBLIC_COMMANDS
        .iter()
        .copied()
        .chain(RETIRED_PUBLIC_DOCS_WORDING.iter().copied())
    {
        check_try!(reject_contains(
            sources.public_copy.as_str(),
            retired,
            format!("retired public docs wording {retired}").as_str(),
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_ecommerce_output_fields`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_ecommerce_output_fields(sources: &DocsSources) -> CheckResult {
    return require_output_fields_in_openapi(sources);
}

/// Require durable exclusion of retired documentation font assets.
///
/// # Errors
///
/// Returns an error when Mintlify can ingest the retired font subtree again.
fn require_mintlify_exclusions() -> CheckResult {
    let exclusions = check_try!(read_text("docs/.mintignore"));
    if !exclusions.lines().any(|line| return line == "fonts/") {
        return Err("docs/.mintignore must exclude the retired fonts/ subtree".to_owned());
    }
    let docs: DocsJson = check_try!(read_json("docs/docs.json"));
    if docs.seo.indexing != "navigable" {
        return Err("docs SEO indexing must be limited to navigable pages".to_owned());
    }
    if !docs.redirects.iter().any(|redirect| {
        return redirect.source == "/fonts/PROVENANCE" && redirect.destination == "/changelog";
    }) {
        return Err("the retired font page must redirect to /changelog".to_owned());
    }
    return Ok(());
}

/// Require ecommerce output schemas in the public `OpenAPI` document.
///
/// # Errors
///
/// Returns an error when a documented output field is missing.
fn require_output_fields_in_openapi(sources: &DocsSources) -> CheckResult {
    for field in ECOMMERCE_OUTPUT_FIELDS {
        for (name, text) in [
            ("scraper docs", sources.scrapers.as_str()),
            ("agents", sources.agents.as_str()),
            ("docs skill", sources.skill.as_str()),
            ("packaged skill", sources.packaged_skill.as_str()),
            ("llms", sources.llms.as_str()),
        ] {
            check_try!(require_contains(
                text,
                field,
                format!("{name} ecommerce output field {field}").as_str(),
            ));
        }
    }
    return require_results(OPENAPI_ECOMMERCE_PROPERTIES.iter().map(|field| {
        return require_contains(
            sources.openapi.as_str(),
            field,
            format!("OpenAPI ecommerce output field {field}").as_str(),
        );
    }));
}

/// Contract implementation for `require_scraper_examples`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_scraper_examples(sources: &DocsSources) -> CheckResult {
    check_try!(require_scraper_examples_in_sources(sources));
    check_try!(require_scraper_feature_coverage(sources));
    return require_ecommerce_output_fields(sources);
}

/// Require representative public scraper commands in every agent-facing surface.
///
/// # Errors
///
/// Returns an error when a representative command or boundary is missing.
fn require_scraper_examples_in_sources(sources: &DocsSources) -> CheckResult {
    for (name, text) in [
        ("README", sources.readme.as_str()),
        ("scraper docs", sources.scrapers.as_str()),
        ("agents", sources.agents.as_str()),
        ("packages", sources.packages.as_str()),
        ("llms", sources.llms.as_str()),
        ("docs skill", sources.skill.as_str()),
        ("packaged skill", sources.packaged_skill.as_str()),
    ] {
        for (scraper, label) in [
            ("tiktok", "TikTok"),
            ("github", "GitHub"),
            ("linkedin", "LinkedIn"),
            ("amazon", "Amazon"),
            ("google-maps", "Google Maps"),
        ] {
            check_try!(require_contains(
                text,
                format!("tovuk request create {scraper}").as_str(),
                format!("{name} {label} example").as_str(),
            ));
        }
        check_try!(require_contains(
            text,
            "public data only",
            format!("{name} public-data policy").as_str(),
        ));
    }
    return Ok(());
}

/// Require ecommerce feature coverage in docs and `OpenAPI`.
///
/// # Errors
///
/// Returns an error when feature coverage is absent from a public surface.
fn require_scraper_feature_coverage(sources: &DocsSources) -> CheckResult {
    check_try!(require_contains(
        sources.scrapers.as_str(),
        "featureCoverage",
        "scraper docs ecommerce feature coverage",
    ));
    check_try!(require_contains(
        sources.agents.as_str(),
        "featureCoverage",
        "agent docs ecommerce feature coverage",
    ));
    check_try!(require_contains(
        sources.skill.as_str(),
        "featureCoverage",
        "docs skill ecommerce feature coverage",
    ));
    check_try!(require_contains(
        sources.packaged_skill.as_str(),
        "featureCoverage",
        "packaged skill ecommerce feature coverage",
    ));
    check_try!(require_contains(
        sources.llms.as_str(),
        "featureCoverage",
        "llms ecommerce feature coverage",
    ));
    check_try!(require_contains(
        sources.openapi.as_str(),
        "featureCoverage",
        "OpenAPI ecommerce feature coverage",
    ));
    return Ok(());
}
