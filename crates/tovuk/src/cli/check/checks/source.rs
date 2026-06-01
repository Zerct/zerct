use super::super::report::QualityCheck;
use crate::cli::source_policy::backend_javascript_or_typescript_sources;
use std::path::Path;

pub(super) fn backend_javascript_source_check(project_dir: &Path, label: &str) -> QualityCheck {
    let matches = backend_javascript_or_typescript_sources(project_dir, label);
    QualityCheck {
        name: "rust worker js/ts server source".to_owned(),
        ok: matches.is_empty(),
        message: if matches.is_empty() {
            "none found".to_owned()
        } else {
            matches
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        },
        agent_instruction: if matches.is_empty() {
            None
        } else {
            Some("Move API routes, SSR handlers, middleware, and server logic to Rust. Keep JS/TS only in static frontend roots.".to_owned())
        },
    }
}
