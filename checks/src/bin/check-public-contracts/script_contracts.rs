use crate::helpers::{CheckResult, read_text, require_contains, require_contains_all};

const BOOTSTRAPPED_SCRIPTS: &[&str] = &[
    "scripts/check-openapi.sh",
    "scripts/check-shell-style.sh",
    "scripts/check-toml-style.sh",
    "scripts/install-vacuum.sh",
    "scripts/sync-native-release-targets.sh",
];

pub(crate) fn check() -> CheckResult {
    require_check_script_bootstrap()?;
    require_rust_native_check_commands()?;
    require_vacuum_installer_contract()?;
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
            require_contains(source.as_str(), snippet, format!("{path} {label}").as_str())?;
        }
    }
    Ok(())
}

fn require_rust_native_check_commands() -> CheckResult {
    let check_all = read_text("checks/src/bin/check-all.rs")?;
    for (snippet, label) in [
        (
            "self.run_public_contracts(&[\"docs\"])?;",
            "Rust check-all must run public contract docs through the Rust checker binary",
        ),
        (
            "self.run_check_bin(\"check-prose-style\", &[\"--self-test\"])?;",
            "Rust check-all must run prose self-test through the Rust checker binary",
        ),
        (
            "self.run_check_bin(\"check-github-actions\", &[])?;",
            "Rust check-all must run GitHub Actions policy through the Rust checker binary",
        ),
        (
            "self.run(\"typos\", &[\"--config\", \".typos.toml\", \".\"])?;",
            "Rust check-all must call the Rust-native typos checker directly",
        ),
    ] {
        require_contains(check_all.as_str(), snippet, label)?;
    }
    Ok(())
}

fn require_vacuum_installer_contract() -> CheckResult {
    let source = read_text("scripts/install-vacuum.sh")?;
    require_contains_all(
        source.as_str(),
        &[
            (
                "vacuum_asset_sha256",
                "Vacuum installer must pin asset checksums",
            ),
            (
                "shasum -a 256",
                "Vacuum installer must verify SHA-256 before extraction",
            ),
            (
                "checksum mismatch",
                "Vacuum installer must fail on checksum mismatch",
            ),
        ],
    )
}

fn require_shell_style_contract() -> CheckResult {
    let source = read_text("scripts/check-shell-style.sh")?;
    require_contains_all(
        source.as_str(),
        &[
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
        ],
    )
}
