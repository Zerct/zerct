use crate::helpers::{CheckResult, require_results};

use super::openapi::{
    OpenApi, openapi_schema, reject_schema_property, reject_schema_property_enum,
    require_json_response_example_string, require_named_schema_properties,
    require_schema_property_enum,
};

/// Required public login example values.
const LOGIN_EXAMPLE_REQUIREMENTS: &[ResponseExampleRequirement] = &[
    ResponseExampleRequirement {
        expected: "https://tovuk.com/login",
        label: "OpenAPI login start verification URI",
        path: &["verificationUri"],
        response: "LoginStarted",
    },
    ResponseExampleRequirement {
        expected: "pending",
        label: "OpenAPI login poll status",
        path: &["status"],
        response: "LoginPolled",
    },
];

/// Required public login schema shapes.
const LOGIN_SCHEMA_REQUIREMENTS: &[LoginSchemaRequirement] = &[
    LoginSchemaRequirement {
        fields: &[
            "loginUrl",
            "verificationUri",
            "deviceCode",
            "userCode",
            "intervalSeconds",
            "expiresInSeconds",
        ],
        label: "OpenAPI login start response",
        name: "LoginStartResponse",
    },
    LoginSchemaRequirement {
        fields: &[
            "status",
            "intervalSeconds",
            "accountId",
            "email",
            "provider",
            "token",
            "expiresAt",
        ],
        label: "OpenAPI login poll response",
        name: "LoginPollResponse",
    },
];

/// Retired snake-case login polling fields.
const RETIRED_POLL_FIELDS: &[&str] = &["expires_at", "account_id", "interval_seconds"];

/// Retired snake-case login start fields.
const RETIRED_START_FIELDS: &[&str] = &[
    "login_url",
    "verification_uri",
    "device_code",
    "expires_in",
    "interval",
];

/// Separable login policy facets applied to the public `OpenAPI` document.
trait LoginPolicy {
    /// Require public login response examples.
    ///
    /// # Errors
    ///
    /// Returns an error when a public login example is missing or stale.
    fn require_login_examples(&self) -> CheckResult;

    /// Reject every retired snake-case login field.
    ///
    /// # Errors
    ///
    /// Returns an error when a retired login field remains.
    fn require_login_retired_fields(&self) -> CheckResult;

    /// Require the public login schema shapes.
    ///
    /// # Errors
    ///
    /// Returns an error when a required login schema or property is missing.
    fn require_login_schema_shapes(&self) -> CheckResult;

    /// Require the canonical login polling status contract.
    ///
    /// # Errors
    ///
    /// Returns an error when the polling status enum is missing or stale.
    fn require_login_status(&self) -> CheckResult;
}

/// One required public login schema shape.
struct LoginSchemaRequirement {
    /// Required property names.
    fields: &'static [&'static str],
    /// Diagnostic label.
    label: &'static str,
    /// Component schema name.
    name: &'static str,
}

impl LoginPolicy for OpenApi {
    fn require_login_examples(&self) -> CheckResult {
        return require_results(LOGIN_EXAMPLE_REQUIREMENTS.iter().map(|requirement| {
            return require_json_response_example_string(
                self,
                (requirement.response, requirement.path),
                requirement.expected,
                requirement.label,
            );
        }));
    }

    fn require_login_retired_fields(&self) -> CheckResult {
        let start_schema = check_try!(openapi_schema(self, "LoginStartResponse"));
        for retired_property in RETIRED_START_FIELDS {
            check_try!(reject_schema_property(
                start_schema,
                retired_property,
                format!("OpenAPI login start retired {retired_property} field").as_str(),
            ));
        }
        let poll_schema = check_try!(openapi_schema(self, "LoginPollResponse"));
        for retired_property in RETIRED_POLL_FIELDS {
            check_try!(reject_schema_property(
                poll_schema,
                retired_property,
                format!("OpenAPI login poll retired {retired_property} field").as_str(),
            ));
        }
        return Ok(());
    }

    fn require_login_schema_shapes(&self) -> CheckResult {
        return require_results(LOGIN_SCHEMA_REQUIREMENTS.iter().map(|requirement| {
            return require_named_schema_properties(
                self,
                requirement.name,
                requirement.fields,
                requirement.label,
            );
        }));
    }

    fn require_login_status(&self) -> CheckResult {
        let poll_schema = check_try!(openapi_schema(self, "LoginPollResponse"));
        check_try!(require_schema_property_enum(
            poll_schema,
            "status",
            &["pending", "complete", "expired"],
            "OpenAPI login poll status enum",
        ));
        return reject_schema_property_enum(
            poll_schema,
            "status",
            "authorized",
            "OpenAPI retired authorized login poll status",
        );
    }
}

/// One required public login response example.
struct ResponseExampleRequirement {
    /// Required string value.
    expected: &'static str,
    /// Diagnostic label.
    label: &'static str,
    /// Nested example property path.
    path: &'static [&'static str],
    /// Response component name.
    response: &'static str,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&require_openapi_login_contract)];

/// Contract implementation for `require_openapi_login_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_login_contract(openapi: &OpenApi) -> CheckResult {
    check_try!(openapi.require_login_examples());
    check_try!(openapi.require_login_retired_fields());
    check_try!(openapi.require_login_schema_shapes());
    return openapi.require_login_status();
}
