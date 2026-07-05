use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_schema_property, require_operation_id,
    require_parameter_bounds, require_schema_properties,
};

pub(super) fn require_openapi_scraper_response_contract(openapi: &OpenApi) -> CheckResult {
    require_schema_properties(
        openapi_schema(openapi, "ScrapersResponse")?,
        &["scrapers", "nextActions"],
        "OpenAPI scraper catalog response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperSummary")?,
        &["inputSchema"],
        "OpenAPI scraper summary",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperRuntimeHealthResponse")?,
        &["scrapers", "nextActions"],
        "OpenAPI scraper runtime health response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperDetailsResponse")?,
        &["scraper", "runtimeHealth", "nextActions"],
        "OpenAPI scraper details response",
    )?;
    require_operation_id(
        openapi,
        "/v1/requests",
        "get",
        "listScrapeRequests",
        "OpenAPI request list operation",
    )?;
    require_parameter_bounds(
        openapi,
        "/v1/requests",
        "get",
        "limit",
        50,
        200,
        "OpenAPI request list page limit",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRequest")?,
        &[
            "resultCount",
            "estimatedCostUsdMicros",
            "costUsdMicros",
            "resultsUrl",
            "agentInstruction",
        ],
        "OpenAPI scraper request",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRequestResponse")?,
        &["request", "nextActions"],
        "OpenAPI scraper request response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeCancelResponse")?,
        &["requestId", "canceled"],
        "OpenAPI scraper cancel response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeResultsResponse")?,
        &["request", "records", "nextCursor", "nextActions"],
        "OpenAPI scraper results response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRecord")?,
        &["index", "sizeBytes"],
        "OpenAPI scrape record",
    )?;
    for schema_name in [
        "ScrapersResponse",
        "ScraperRuntimeHealthResponse",
        "ScraperDetailsResponse",
        "ScrapeRequestResponse",
        "ScrapeRequestsResponse",
        "ScrapeCancelResponse",
        "ScrapeResultsResponse",
    ] {
        reject_schema_property(
            openapi_schema(openapi, schema_name)?,
            "ok",
            format!("OpenAPI {schema_name} retired ok wrapper").as_str(),
        )?;
    }
    Ok(())
}
