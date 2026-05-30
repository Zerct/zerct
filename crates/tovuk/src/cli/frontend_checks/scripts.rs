use super::{
    layout::frontend_package_manager,
    predicates::{
        referenced_package_scripts, uses_javascript_linter, uses_native_dead_code_checker,
        uses_native_duplicate_checker, uses_native_frontend_linter, uses_native_health_checker,
        uses_strict_frontend_typechecker,
    },
};
use crate::cli::{
    doctor::{DoctorCheck, doctor_check, first_output_line},
    project::read_package_json,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn frontend_script_checks(project_dir: &Path, run_scripts: bool) -> Vec<DoctorCheck> {
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

fn package_script_exists_check(script: &str, command: &str) -> DoctorCheck {
    doctor_check(
        &format!("package script {script}"),
        !command.is_empty(),
        "found",
        "missing",
        &format!("Add a non-empty \"{script}\" script to package.json, then retry."),
    )
}

fn strict_typecheck_check(command: &str) -> DoctorCheck {
    doctor_check(
        "strict frontend typecheck",
        uses_strict_frontend_typechecker(command),
        "accepted",
        "native typecheck missing",
        "Set package.json `typecheck` to `oxlint src vite.config.ts --deny-warnings --type-aware --type-check --tsconfig tsconfig.json`, then retry.",
    )
}

fn native_lint_check(manifest: Option<&Value>) -> DoctorCheck {
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

fn native_quality_gate_check(manifest: Option<&Value>) -> DoctorCheck {
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

fn package_script_value(manifest: Option<&Value>, script: &str) -> String {
    manifest
        .and_then(|manifest| manifest.get("scripts"))
        .and_then(|scripts| scripts.get(script))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn package_script_tree_uses(
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

fn package_script_check(project_dir: &Path, script: &str) -> DoctorCheck {
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
