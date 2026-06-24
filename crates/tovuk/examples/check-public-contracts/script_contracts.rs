use crate::helpers::{CheckResult, read_text};

const BOOTSTRAPPED_SCRIPTS: &[&str] = &[
    "scripts/check-all.sh",
    "scripts/check-github-actions.sh",
    "scripts/check-openapi.sh",
    "scripts/check-prose-style.sh",
    "scripts/check-public-contracts.sh",
    "scripts/check-shell-style.sh",
    "scripts/check-toml-style.sh",
    "scripts/check-typos.sh",
    "scripts/install-vacuum.sh",
];

pub(crate) fn check() -> CheckResult {
    require_check_script_bootstrap()?;
    require_shell_style_contract()
}

fn require_check_script_bootstrap() -> CheckResult {
    for path in BOOTSTRAPPED_SCRIPTS {
        let source = read_text(path)?;
        for (snippet, label) in [
            (
                "scripts/lib/repo-root.sh",
                "must use the tracked repo root helper",
            ),
            (
                "scripts/lib/tool-path.sh",
                "must use the tracked tool path helper",
            ),
            (
                "tovuk_prepend_tool_path",
                "must prepend the trusted runner tool path",
            ),
        ] {
            if !source.contains(snippet) {
                return Err(format!("{path} {label}"));
            }
        }
    }
    Ok(())
}

fn require_shell_style_contract() -> CheckResult {
    let source = read_text("scripts/check-shell-style.sh")?;
    for (snippet, label) in [
        (
            "shell_sources=(scripts/*.sh scripts/lib/*.sh)",
            "public shell style check must include shared shell libraries",
        ),
        (
            "shellcheck -x",
            "public shell style check must run ShellCheck with sourced-file analysis",
        ),
        (
            "shfmt -i 2 -ci -d",
            "public shell style check must run shfmt",
        ),
    ] {
        if !source.contains(snippet) {
            return Err(label.to_owned());
        }
    }
    Ok(())
}
