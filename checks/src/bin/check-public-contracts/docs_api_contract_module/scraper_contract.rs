use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_schema_property, require_operation_id,
    require_parameter_bounds, require_schema_properties,
};

/// Response schemas that must not retain a legacy `ok` wrapper.
const RETIRED_OK_WRAPPER_SCHEMAS: &[&str] = &[
    "ScrapersResponse",
    "ScraperRuntimeHealthResponse",
    "ScraperDetailsResponse",
    "ScrapeRequestResponse",
    "ScrapeRequestsResponse",
    "ScrapeCancelResponse",
    "ScrapeResultsResponse",
];

/// Required public scraper schema shapes.
const SCRAPER_SCHEMA_REQUIREMENTS: &[SchemaRequirement] = &[
    SchemaRequirement {
        fields: &["scrapers", "nextActions"],
        label: "OpenAPI scraper catalog response",
        name: "ScrapersResponse",
    },
    SchemaRequirement {
        fields: &["inputSchema"],
        label: "OpenAPI scraper summary",
        name: "ScraperSummary",
    },
    SchemaRequirement {
        fields: &["scrapers", "nextActions"],
        label: "OpenAPI scraper runtime health response",
        name: "ScraperRuntimeHealthResponse",
    },
    SchemaRequirement {
        fields: &["scraper", "runtimeHealth", "nextActions"],
        label: "OpenAPI scraper details response",
        name: "ScraperDetailsResponse",
    },
    SchemaRequirement {
        fields: &[
            "resultCount",
            "estimatedCostUsdMicros",
            "costUsdMicros",
            "resultsUrl",
            "agentInstruction",
        ],
        label: "OpenAPI scraper request",
        name: "ScrapeRequest",
    },
    SchemaRequirement {
        fields: &["request", "nextActions"],
        label: "OpenAPI scraper request response",
        name: "ScrapeRequestResponse",
    },
    SchemaRequirement {
        fields: &["requestId", "canceled"],
        label: "OpenAPI scraper cancel response",
        name: "ScrapeCancelResponse",
    },
    SchemaRequirement {
        fields: &["request", "records", "nextCursor", "nextActions"],
        label: "OpenAPI scraper results response",
        name: "ScrapeResultsResponse",
    },
    SchemaRequirement {
        fields: &["index", "sizeBytes"],
        label: "OpenAPI scrape record",
        name: "ScrapeRecord",
    },
];

/// One required public `OpenAPI` schema shape.
struct SchemaRequirement {
    /// Required property names.
    fields: &'static [&'static str],
    /// Diagnostic label.
    label: &'static str,
    /// Component schema name.
    name: &'static str,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&require_openapi_scraper_response_contract)];

/// Contract implementation for `require_openapi_scraper_response_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_scraper_response_contract(openapi: &OpenApi) -> CheckResult {
    for requirement in SCRAPER_SCHEMA_REQUIREMENTS {
        check_try!(require_schema_properties(
            check_try!(openapi_schema(openapi, requirement.name)),
            requirement.fields,
            requirement.label,
        ));
    }
    check_try!(require_operation_id(
        openapi,
        ("/v1/requests", "get"),
        "listScrapeRequests",
        "OpenAPI request list operation",
    ));
    check_try!(require_parameter_bounds(
        openapi,
        ("/v1/requests", "get", "limit"),
        (50, 200),
        "OpenAPI request list page limit",
    ));
    for schema_name in RETIRED_OK_WRAPPER_SCHEMAS {
        check_try!(reject_schema_property(
            check_try!(openapi_schema(openapi, schema_name)),
            "ok",
            format!("OpenAPI {schema_name} retired ok wrapper").as_str(),
        ));
    }
    return Ok(());
}
