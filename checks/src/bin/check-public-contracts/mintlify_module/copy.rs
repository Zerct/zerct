use crate::helpers::{CheckResult, reject_forbidden_public_copy_terms, retired_public_names};

use crate::html_visible_copy::html_visible_copy;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&reject_retired_public_names_in_html)];

/// Contract implementation for `reject_retired_public_names`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_public_names(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for retired in retired_public_names() {
        if lower.contains(retired) {
            return Err(format!("{label} contains retired public branding"));
        }
    }
    return reject_forbidden_public_copy_terms(label, source);
}

/// Contract implementation for `reject_retired_public_names_in_html`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_public_names_in_html(label: &str, source: &str) -> CheckResult {
    let visible_copy = html_visible_copy(source);
    return reject_retired_public_names(label, visible_copy.as_str());
}
#[cfg(test)]
mod tests {
    use crate::helpers_public_copy::{RETIRED_PUBLIC_MINTLIFY_SLUG, RETIRED_PUBLIC_NAME_TITLE};

    use super::reject_retired_public_names_in_html;

    /// Verify generated Mintlify asset slugs do not enter visible-copy validation.
    ///
    /// # Panics
    ///
    /// Panics when generated hidden asset metadata is treated as visible branding.
    #[test]
    fn ignores_generated_mintlify_project_slug_in_html_assets() {
        let source = format!(
            r#"
            <!doctype html>
            <html>
              <head>
                <meta property="og:image" content="https://{RETIRED_PUBLIC_MINTLIFY_SLUG}.mintlify.app/og.png">
                <link rel="preload" href="/mintlify-assets/{RETIRED_PUBLIC_MINTLIFY_SLUG}/logo.svg">
              </head>
              <body>
                <main>
                  <h1>Tovuk</h1>
                  <p>Paid public-data scraper API.</p>
                </main>
              </body>
            </html>
        "#
        );

        assert!(
            reject_retired_public_names_in_html("/", source.as_str()).is_ok(),
            "Mintlify immutable internal slugs in asset URLs should not fail visible copy checks"
        );
    }

    /// Verify visible retired branding remains rejected.
    ///
    /// # Panics
    ///
    /// Panics when visible retired branding is accepted.
    #[test]
    fn rejects_visible_retired_branding_in_html() {
        let source = format!(
            "\n            <!doctype html>\n            <html>\n              <body>\n                <main>\n                  <h1>{RETIRED_PUBLIC_NAME_TITLE}</h1>\n                </main>\n              </body>\n            </html>\n        "
        );

        let result = reject_retired_public_names_in_html("/", source.as_str());
        assert!(
            matches!(result, Err(message) if message.contains("retired public branding")),
            "Visible retired public branding must still fail the docs readiness check"
        );
    }
}
