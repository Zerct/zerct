use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_schema_property, reject_schema_property_enum,
    require_json_response_example_string, require_schema_properties, require_schema_property_enum,
};

pub(super) fn require_openapi_login_contract(openapi: &OpenApi) -> CheckResult {
    let start_schema = openapi_schema(openapi, "LoginStartResponse")?;
    require_schema_properties(
        start_schema,
        &[
            "loginUrl",
            "verificationUri",
            "deviceCode",
            "userCode",
            "intervalSeconds",
            "expiresInSeconds",
        ],
        "OpenAPI login start response",
    )?;
    for retired_property in [
        "login_url",
        "verification_uri",
        "device_code",
        "expires_in",
        "interval",
    ] {
        reject_schema_property(
            start_schema,
            retired_property,
            format!("OpenAPI login start retired {retired_property} field").as_str(),
        )?;
    }

    let poll_schema = openapi_schema(openapi, "LoginPollResponse")?;
    require_schema_properties(
        poll_schema,
        &[
            "status",
            "intervalSeconds",
            "accountId",
            "email",
            "provider",
            "token",
            "expiresAt",
        ],
        "OpenAPI login poll response",
    )?;
    require_schema_property_enum(
        poll_schema,
        "status",
        &["pending", "complete", "expired"],
        "OpenAPI login poll status enum",
    )?;
    reject_schema_property_enum(
        poll_schema,
        "status",
        "authorized",
        "OpenAPI retired authorized login poll status",
    )?;
    for retired_property in ["expires_at", "account_id", "interval_seconds"] {
        reject_schema_property(
            poll_schema,
            retired_property,
            format!("OpenAPI login poll retired {retired_property} field").as_str(),
        )?;
    }
    require_json_response_example_string(
        openapi,
        "LoginStarted",
        &["verificationUri"],
        "https://tovuk.com/login",
        "OpenAPI login start verification URI",
    )?;
    require_json_response_example_string(
        openapi,
        "LoginPolled",
        &["status"],
        "pending",
        "OpenAPI login poll status",
    )
}
