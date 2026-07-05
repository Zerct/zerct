use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_json_response_example_check_name,
    require_json_response_example_check_name,
};

pub(super) fn require_openapi_status_checks(openapi: &OpenApi) -> CheckResult {
    openapi_schema(openapi, "StatusResponse")?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "control_plane_sqlite",
        "OpenAPI status control-plane SQLite check",
    )?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "redis",
        "OpenAPI status Redis check",
    )?;
    reject_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "database",
        "OpenAPI status must not expose generic database product wording",
    )
}
