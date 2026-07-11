use crate::helpers_public_copy::RETIRED_PUBLIC_NAME_TITLE;

use super::html_visible_copy;

/// Verify generated hidden HTML does not enter the visible-copy scan.
///
/// # Panics
///
/// Panics when visible body copy is lost or hidden retired branding is exposed.
#[test]
fn visible_copy_ignores_head_assets_and_hidden_tags() {
    let source = format!(
        "\n            <html>\n              <head><title>{RETIRED_PUBLIC_NAME_TITLE}</title></head>\n              <body>\n                <h1>Tovuk &amp; agents</h1>\n                <script>{RETIRED_PUBLIC_NAME_TITLE}</script>\n                <svg><text>{RETIRED_PUBLIC_NAME_TITLE}</text></svg>\n              </body>\n            </html>\n        "
    );

    let visible = html_visible_copy(source.as_str());
    assert!(
        visible.contains("Tovuk & agents"),
        "visible body copy must remain available"
    );
    assert!(
        !visible.contains(RETIRED_PUBLIC_NAME_TITLE),
        "hidden retired branding must not enter visible copy"
    );
}
