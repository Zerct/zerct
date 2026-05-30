use crate::cli::{doctor::DoctorCheck, source_policy::frontend_source_report};
use std::path::Path;

pub(super) fn frontend_source_checks(project_dir: &Path) -> Vec<DoctorCheck> {
    let report = frontend_source_report(project_dir);
    vec![
        DoctorCheck {
            name: "typescript source".to_owned(),
            ok: !report.typescript.is_empty(),
            message: if report.typescript.is_empty() {
                "missing".to_owned()
            } else {
                report
                    .typescript
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            agent_instruction: if report.typescript.is_empty() {
                Some("Add browser source as .ts or .tsx under src, app, pages, routes, or components, then retry.".to_owned())
            } else {
                None
            },
        },
        forbidden_source_check(
            "javascript source",
            &report.javascript,
            "Rename browser .js, .jsx, .mjs, or .cjs source files to .ts or .tsx and fix type errors before deploying.",
        ),
        forbidden_source_check(
            "frontend server routes",
            &report.server_routes,
            "Move API routes, SSR handlers, middleware, and server logic to the Rust worker; static frontend source may only contain browser code.",
        ),
    ]
}

fn forbidden_source_check(name: &str, files: &[String], instruction: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.to_owned(),
        ok: files.is_empty(),
        message: if files.is_empty() {
            "none found".to_owned()
        } else {
            files.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        },
        agent_instruction: if files.is_empty() {
            None
        } else {
            Some(instruction.to_owned())
        },
    }
}
