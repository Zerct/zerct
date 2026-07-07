use crate::{
    docs_api_contract::require_support_pricing_and_openapi,
    docs_navigation::{require_navigation_contract, require_navigation_pages_exist},
    docs_sources::{DocsSources, openapi_config_path, read_navigation_pages},
    helpers::{CheckResult, reject_contains, require_contains},
    retired_contracts::{
        RETIRED_OPENAPI_CONTRACTS, RETIRED_PUBLIC_COMMANDS, RETIRED_PUBLIC_DOCS_WORDING,
    },
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
            "tovuk request create amazon",
            format!("{name} Amazon example").as_str(),
        )?;
        require_contains(
            text,
            "tovuk request create google-maps",
            format!("{name} Google Maps example").as_str(),
        )?;
        require_contains(
            text,
            "public data only",
            format!("{name} public-data policy").as_str(),
        )?;
    }
    require_contains(
        sources.scrapers.as_str(),
        "featureCoverage",
        "scraper docs ecommerce feature coverage",
    )?;
    require_contains(
        sources.agents.as_str(),
        "featureCoverage",
        "agent docs ecommerce feature coverage",
    )?;
    require_contains(
        sources.skill.as_str(),
        "featureCoverage",
        "docs skill ecommerce feature coverage",
    )?;
    require_contains(
        sources.packaged_skill.as_str(),
        "featureCoverage",
        "packaged skill ecommerce feature coverage",
    )?;
    require_contains(
        sources.llms.as_str(),
        "featureCoverage",
        "llms ecommerce feature coverage",
    )?;
    require_contains(
        sources.openapi.as_str(),
        "featureCoverage",
        "OpenAPI ecommerce feature coverage",
    )?;
    require_ecommerce_output_fields(sources)?;
    Ok(())
}

fn require_ecommerce_output_fields(sources: &DocsSources) -> CheckResult {
    for field in [
        "tags",
        "keywords",
        "questionSamples",
        "customerPhotoUrls",
        "minimumOrderQuantity",
        "rfqText",
        "resultPosition",
        "adPosition",
    ] {
        for (name, text) in [
            ("scraper docs", sources.scrapers.as_str()),
            ("agents", sources.agents.as_str()),
            ("docs skill", sources.skill.as_str()),
            ("packaged skill", sources.packaged_skill.as_str()),
            ("llms", sources.llms.as_str()),
        ] {
            require_contains(
                text,
                field,
                format!("{name} ecommerce output field {field}").as_str(),
            )?;
        }
    }
    for field in [
        "\"tags\": {",
        "\"keywords\": {",
        "\"questionSamples\": {",
        "\"customerPhotoUrls\": {",
        "\"minimumOrderQuantity\": {",
        "\"rfqText\": {",
        "\"resultPosition\": {",
        "\"adPosition\": {",
    ] {
        require_contains(
            sources.openapi.as_str(),
            field,
            format!("OpenAPI ecommerce output field {field}").as_str(),
        )?;
    }
    Ok(())
}

fn reject_retired_docs_contracts(sources: &DocsSources) -> CheckResult {
    for retired in RETIRED_OPENAPI_CONTRACTS {
        reject_contains(
            sources.openapi.as_str(),
            retired,
            format!("retired public OpenAPI contract {retired}").as_str(),
        )?;
    }

    for retired in RETIRED_PUBLIC_COMMANDS
        .iter()
        .copied()
        .chain(RETIRED_PUBLIC_DOCS_WORDING.iter().copied())
    {
        reject_contains(
            sources.public_copy.as_str(),
            retired,
            format!("retired public docs wording {retired}").as_str(),
        )?;
    }
    Ok(())
}
