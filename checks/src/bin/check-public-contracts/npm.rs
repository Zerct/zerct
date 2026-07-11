use crate::{
    helpers::CheckResult, npm_package::check_cli_package_contract,
    npm_runtime::check_native_runtime_contract,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&check_cli_package),
    size_of_val(&check_native_runtime),
];

/// Contract implementation for `check_cli_package`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_cli_package() -> CheckResult {
    return check_cli_package_contract();
}

/// Contract implementation for `check_native_runtime`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_native_runtime() -> CheckResult {
    return check_native_runtime_contract();
}
