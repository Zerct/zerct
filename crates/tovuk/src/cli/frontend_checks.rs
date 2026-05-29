use super::{
    command_policy::{command_name_from_token, command_tokens},
    constants::{
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND,
        FRONTEND_INSTALL_COMMANDS, FRONTEND_PACKAGE_MANAGERS, JAVASCRIPT_LINTERS,
    },
    doctor::{DoctorCheck, doctor_check, first_output_line},
    project::read_package_json,
    source_policy::frontend_source_report,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::Path,
    process::{Command, Stdio},
};

pub(crate) fn static_frontend_checks(project_dir: &Path, run_scripts: bool) -> Vec<DoctorCheck> {
    if is_plain_static_frontend(project_dir) {
        return Vec::new();
    }
    let mut checks = vec![frontend_lockfile_check(project_dir)];
    checks.extend(frontend_source_checks(project_dir));
    checks.extend(frontend_script_checks(project_dir, run_scripts));
    checks
}

pub(crate) fn frontend_lockfile_check(project_dir: &Path) -> DoctorCheck {
    doctor_check(
        "frontend lockfile",
        frontend_lockfile_exists(project_dir),
        "found",
        "missing",
        "Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.",
    )
}

pub(crate) fn frontend_lockfile_exists(project_dir: &Path) -> bool {
    [
        "package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
    ]
    .iter()
    .any(|file| project_dir.join(file).exists())
}

pub(crate) fn is_plain_static_frontend(project_dir: &Path) -> bool {
    !project_dir.join("package.json").exists() && project_dir.join("index.html").exists()
}

pub(crate) fn frontend_package_manager(project_dir: &Path) -> &'static str {
    if project_dir.join("bun.lock").exists() || project_dir.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    }
}

pub(crate) fn frontend_check_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        DEFAULT_NPM_FRONTEND_CHECK_COMMAND.to_owned()
    }
}

pub(crate) fn frontend_build_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        "bun run build".to_owned()
    } else {
        "npm run build".to_owned()
    }
}

pub(crate) fn frontend_script_checks(project_dir: &Path, run_scripts: bool) -> Vec<DoctorCheck> {
    let manifest = read_package_json(project_dir);
    let typecheck = package_script_value(manifest.as_ref(), "typecheck");
    let lint = package_script_value(manifest.as_ref(), "lint");
    let mut checks = vec![
        package_script_exists_check("typecheck", &typecheck),
        package_script_exists_check("lint", &lint),
        strict_typecheck_check(&typecheck),
        native_lint_check(manifest.as_ref()),
        native_quality_gate_check(manifest.as_ref()),
    ];
    if run_scripts && checks.iter().all(|check| check.ok) {
        checks.push(package_script_check(project_dir, "typecheck"));
        checks.push(package_script_check(project_dir, "lint"));
    }
    checks
}

pub(crate) fn package_script_exists_check(script: &str, command: &str) -> DoctorCheck {
    doctor_check(
        &format!("package script {script}"),
        !command.is_empty(),
        "found",
        "missing",
        &format!("Add a non-empty \"{script}\" script to package.json, then retry."),
    )
}

pub(crate) fn strict_typecheck_check(command: &str) -> DoctorCheck {
    doctor_check(
        "strict frontend typecheck",
        uses_strict_frontend_typechecker(command),
        "accepted",
        "native typecheck missing",
        "Set package.json `typecheck` to `oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json`, then retry.",
    )
}

pub(crate) fn native_lint_check(manifest: Option<&Value>) -> DoctorCheck {
    let ok = !package_script_tree_uses(
        manifest,
        "lint",
        uses_javascript_linter,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_frontend_linter,
        &mut BTreeSet::new(),
    );
    doctor_check(
        "native frontend lint",
        ok,
        "accepted",
        "native linter missing",
        "Replace the lint script with native tooling such as `oxlint src vite.config.ts --deny-warnings`, `biome check .`, or `deno lint`, then retry.",
    )
}

pub(crate) fn native_quality_gate_check(manifest: Option<&Value>) -> DoctorCheck {
    let ok = package_script_tree_uses(
        manifest,
        "lint",
        uses_native_dead_code_checker,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_duplicate_checker,
        &mut BTreeSet::new(),
    ) && package_script_tree_uses(
        manifest,
        "lint",
        uses_native_health_checker,
        &mut BTreeSet::new(),
    );
    doctor_check(
        "native frontend quality gates",
        ok,
        "accepted",
        "dead-code, duplicate-code, or health gate missing",
        "Add Fallow checks for `dead-code`, semantic `dupes`, and `health` to package.json `lint`, then retry.",
    )
}

pub(crate) fn frontend_source_checks(project_dir: &Path) -> Vec<DoctorCheck> {
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
            "Move API routes, SSR handlers, middleware, and server logic to the Rust backend; static frontend source may only contain browser code.",
        ),
    ]
}

pub(crate) fn forbidden_source_check(
    name: &str,
    files: &[String],
    instruction: &str,
) -> DoctorCheck {
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

pub(crate) fn package_script_value(manifest: Option<&Value>, script: &str) -> String {
    manifest
        .and_then(|manifest| manifest.get("scripts"))
        .and_then(|scripts| scripts.get(script))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

pub(crate) fn uses_javascript_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        JAVASCRIPT_LINTERS.contains(&command_name.as_str())
            || (command_name == "next"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

pub(crate) fn uses_strict_frontend_typechecker(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        (command_name == "oxlint"
            && tokens.iter().any(|value| value == "--type-aware")
            && tokens.iter().any(|value| value == "--type-check"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "check"))
    })
}

pub(crate) fn uses_native_frontend_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        command_name == "oxlint"
            || (command_name == "biome"
                && tokens
                    .get(index + 1)
                    .is_some_and(|value| value == "check" || value == "lint"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

pub(crate) fn uses_native_dead_code_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dead-code")
}

pub(crate) fn uses_native_duplicate_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dupes")
}

pub(crate) fn uses_native_health_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "health")
}

pub(crate) fn uses_fallow_subcommand(command: &str, subcommand: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        command_name_from_token(token) == "fallow"
            && tokens
                .get(index + 1)
                .is_some_and(|value| value == subcommand)
    })
}

pub(crate) fn package_script_tree_uses(
    manifest: Option<&Value>,
    script: &str,
    predicate: fn(&str) -> bool,
    seen: &mut BTreeSet<String>,
) -> bool {
    if !seen.insert(script.to_owned()) {
        return false;
    }
    let command = package_script_value(manifest, script);
    if command.is_empty() {
        return false;
    }
    if predicate(&command) {
        return true;
    }
    referenced_package_scripts(&command)
        .iter()
        .any(|referenced| package_script_tree_uses(manifest, referenced, predicate, seen))
}

pub(crate) fn referenced_package_scripts(command: &str) -> Vec<String> {
    let tokens = command_tokens(command);
    let mut scripts = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            continue;
        }
        if let Some(script) = script_name_after_run(&tokens, index + 2) {
            scripts.push(script);
        }
    }
    scripts
}

pub(crate) fn script_name_after_run(tokens: &[String], start: usize) -> Option<String> {
    let mut index = start;
    while tokens
        .get(index)
        .is_some_and(|token| token.starts_with('-'))
    {
        index += 1;
    }
    tokens.get(index).cloned()
}

pub(crate) fn has_frontend_install_command(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        FRONTEND_INSTALL_COMMANDS.contains(
            &format!(
                "{} {}",
                command_name_from_token(token),
                tokens.get(index + 1).map_or("", String::as_str)
            )
            .as_str(),
        )
    })
}

pub(crate) fn has_frontend_script_run(tokens: &[String], script: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            return false;
        }
        tokens.get(index + 2).is_some_and(|value| value == script)
            || (tokens
                .get(index + 2)
                .is_some_and(|value| value.starts_with('-'))
                && tokens.get(index + 3).is_some_and(|value| value == script))
    })
}

pub(crate) fn package_script_check(project_dir: &Path, script: &str) -> DoctorCheck {
    let manager = frontend_package_manager(project_dir);
    let args = if manager == "bun" {
        vec!["run", script]
    } else {
        vec!["run", "--silent", script]
    };
    let result = Command::new(manager)
        .args(args)
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .output();
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            return DoctorCheck {
                name: format!("{manager} run {script}"),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(format!(
                    "Install {}, then run `{manager} run {script}` before deploying.",
                    if manager == "bun" {
                        "Bun"
                    } else {
                        "Node.js and npm"
                    }
                )),
            };
        }
    };
    DoctorCheck {
        name: format!("{manager} run {script}"),
        ok: output.status.success(),
        message: if output.status.success() {
            "passed".to_owned()
        } else {
            first_output_line(
                &output.stderr,
                &output.stdout,
                &format!("{manager} run {script}"),
            )
        },
        agent_instruction: if output.status.success() {
            None
        } else {
            Some(format!(
                "Run `{manager} run {script}`, fix every error, then redeploy."
            ))
        },
    }
}
