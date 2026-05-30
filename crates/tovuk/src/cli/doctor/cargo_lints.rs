use super::report::DoctorCheck;
use crate::cli::constants::RUST_STRICT_CLIPPY_DENY_LINTS;
use std::{fs, path::Path};

pub(super) fn cargo_lints(project_dir: &Path) -> DoctorCheck {
    let cargo_toml = project_dir.join("Cargo.toml");
    let source = match fs::read_to_string(&cargo_toml) {
        Ok(source) => source,
        Err(error) => {
            return DoctorCheck {
                name: "cargo lints".to_owned(),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(
                    "Create Cargo.toml with strict Rust lints, then retry.".to_owned(),
                ),
            };
        }
    };
    let required_clippy_lints = RUST_STRICT_CLIPPY_DENY_LINTS
        .iter()
        .map(|lint| lint.trim_start_matches("clippy::"))
        .collect::<Vec<_>>();
    let cargo_toml = match source.parse::<toml::Table>() {
        Ok(cargo_toml) => cargo_toml,
        Err(error) => {
            return DoctorCheck {
                name: "cargo lints".to_owned(),
                ok: false,
                message: error.to_string(),
                agent_instruction: Some(
                    "Fix Cargo.toml syntax, then add strict Rust lints and retry.".to_owned(),
                ),
            };
        }
    };
    let ok = cargo_lint_level(&cargo_toml, "rust", "unsafe_code") == Some("forbid")
        && cargo_lint_level(&cargo_toml, "rust", "warnings") == Some("deny")
        && required_clippy_lints
            .iter()
            .all(|lint| cargo_lint_level(&cargo_toml, "clippy", lint) == Some("deny"));
    DoctorCheck {
        name: "cargo lints".to_owned(),
        ok,
        message: if ok {
            "strict".to_owned()
        } else {
            "missing strict Rust or Clippy resource lints".to_owned()
        },
        agent_instruction: if ok {
            None
        } else {
            Some("Add `[lints.rust]` with `unsafe_code = \"forbid\"` and `warnings = \"deny\"`, plus `[lints.clippy]` deny entries for all, pedantic, panic/unwrap bans, and resource lints, then retry.".to_owned())
        },
    }
}

fn cargo_lint_level<'a>(
    cargo_toml: &'a toml::Table,
    lint_group: &str,
    lint_name: &str,
) -> Option<&'a str> {
    lint_group_table(cargo_toml, &["lints", lint_group])
        .or_else(|| lint_group_table(cargo_toml, &["workspace", "lints", lint_group]))
        .and_then(|table| table.get(lint_name))
        .and_then(lint_assignment_level)
}

fn lint_group_table<'a>(cargo_toml: &'a toml::Table, path: &[&str]) -> Option<&'a toml::Table> {
    let mut table = cargo_toml;
    for segment in path {
        table = table.get(*segment)?.as_table()?;
    }
    Some(table)
}

fn lint_assignment_level(value: &toml::Value) -> Option<&str> {
    if let Some(level) = value.as_str() {
        return Some(level);
    }
    value
        .as_table()
        .and_then(|table| table.get("level"))
        .and_then(toml::Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::{cargo_lint_level, lint_assignment_level};

    fn parse_cargo_toml(source: &str) -> toml::Table {
        match source.parse::<toml::Table>() {
            Ok(table) => table,
            Err(error) => {
                let message = error.to_string();
                assert!(message.is_empty(), "{message}");
                toml::Table::new()
            }
        }
    }

    #[test]
    fn cargo_lint_level_reads_string_and_inline_table_levels() {
        let cargo_toml = parse_cargo_toml(
            r#"
[lints.rust]
unsafe_code = "forbid"
warnings = "deny"

[lints.clippy]
unwrap_used = { level = "deny", priority = -1 }
"#,
        );

        assert_eq!(
            cargo_lint_level(&cargo_toml, "rust", "unsafe_code"),
            Some("forbid")
        );
        assert_eq!(
            cargo_lint_level(&cargo_toml, "clippy", "unwrap_used"),
            Some("deny")
        );
    }

    #[test]
    fn cargo_lint_level_falls_back_to_workspace_lints() {
        let cargo_toml = parse_cargo_toml(
            r#"
[workspace.lints.clippy]
panic = "deny"
"#,
        );

        assert_eq!(
            cargo_lint_level(&cargo_toml, "clippy", "panic"),
            Some("deny")
        );
    }

    #[test]
    fn lint_assignment_level_rejects_unstructured_values() {
        assert_eq!(lint_assignment_level(&toml::Value::Boolean(true)), None);
    }
}
