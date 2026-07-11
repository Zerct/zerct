use crate::helpers::CheckResult;

use super::{render_public_tree_policy, require_public_tree_policy_bytes, reviewed_tracked_paths};

/// Verify inert policy data permits one bounded documentation addition.
///
/// # Errors
///
/// Returns an error when the current policy or synthetic addition is invalid.
#[test]
fn accepts_safe_public_path_addition() -> CheckResult {
    let mut paths = check_try!(reviewed_tracked_paths());
    if paths.contains("docs/new-public-page.mdx") {
        return Err("safe-add fixture path unexpectedly exists".to_owned());
    }
    paths.extend(["docs/new-public-page.mdx".to_owned()]);
    let rendered = check_try!(render_public_tree_policy(&paths));
    return require_public_tree_policy_bytes(rendered.as_bytes(), &paths);
}

/// Verify inert policy data permits removal of a nonessential public page.
///
/// # Errors
///
/// Returns an error when the current policy or synthetic removal is invalid.
#[test]
fn accepts_safe_public_path_removal() -> CheckResult {
    let mut paths = check_try!(reviewed_tracked_paths());
    if !paths.remove("docs/changelog.mdx") {
        return Err("safe-removal fixture path is absent".to_owned());
    }
    let rendered = check_try!(render_public_tree_policy(&paths));
    return require_public_tree_policy_bytes(rendered.as_bytes(), &paths);
}

/// Verify policy bytes cannot bless a different tracked path set.
///
/// # Errors
///
/// Returns an error when fixture setup cannot read current policy.
#[test]
fn rejects_public_tree_policy_mismatch() -> CheckResult {
    let mut paths = check_try!(reviewed_tracked_paths());
    let rendered = check_try!(render_public_tree_policy(&paths));
    paths.extend(["docs/unbound-page.mdx".to_owned()]);
    if require_public_tree_policy_bytes(rendered.as_bytes(), &paths).is_ok() {
        return Err("public-tree policy accepted an unbound path set".to_owned());
    }
    return Ok(());
}
