use crate::{
    helpers::CheckResult, npm_package::check_cli_package_contract,
    npm_runtime::check_native_runtime_contract,
};

pub(crate) fn check_cli_package() -> CheckResult {
    check_cli_package_contract()
}

pub(crate) fn check_native_runtime() -> CheckResult {
    check_native_runtime_contract()
}
