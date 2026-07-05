use crate::helpers::{CheckResult, read_text, require_contains, require_contains_all};

const BOOTSTRAPPED_SCRIPTS: &[&str] = &["scripts/deploy-mintlify-docs.sh"];

pub(crate) fn check() -> CheckResult {
    require_check_script_bootstrap()?;
    require_rust_native_check_commands()?;
    require_vacuum_installer_contract()?;
    require_shell_style_contract()?;
    require_toml_style_contract()
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
            "self.run_check_bin(\"check-shell-style\", &[])?;",
            "Rust check-all must run shell style through the Rust checker binary",
        ),
        (
            "self.run_check_bin(\"check-toml-style\", &[])?;",
            "Rust check-all must run TOML style through the Rust checker binary",
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
    let source = read_text("checks/src/bin/check-openapi/vacuum.rs")?;
    require_contains_all(
        source.as_str(),
        &[
            (
                "fn vacuum_asset_sha256",
                "Vacuum installer must pin asset checksums",
            ),
            (
                "Sha256::digest(archive_bytes)",
                "Vacuum installer must verify SHA-256 before extraction",
            ),
            (
                "checksum mismatch",
                "Vacuum installer must fail on checksum mismatch",
            ),
            (
                "Archive::new(GzDecoder::new(archive_bytes))",
                "Vacuum installer must extract the pinned tarball in Rust",
            ),
        ],
    )
}

fn require_shell_style_contract() -> CheckResult {
    let source = read_text("checks/src/bin/check-shell-style.rs")?;
    require_contains_all(
        source.as_str(),
        &[
            (
                "collect_shell_files(repo_root, Path::new(\"scripts/lib\"))?",
                "public shell style check must include shared shell libraries",
            ),
            (
                "\"shellcheck\",",
                "public shell style check must run ShellCheck with sourced-file analysis",
            ),
            (
                "&[\"-i\", \"2\", \"-ci\", \"-d\"]",
                "public shell style check must run shfmt",
            ),
        ],
    )
}

fn require_toml_style_contract() -> CheckResult {
    let source = read_text("checks/src/bin/check-toml-style.rs")?;
    require_contains_all(
        source.as_str(),
        &[
            (
                "&[\"format\", \"--check\"]",
                "public TOML style check must run taplo format in check mode",
            ),
            (
                "&[\"lint\", \"--no-schema\"]",
                "public TOML style check must run taplo lint without schema downloads",
            ),
            (
                "matches!(name, \".git\" | \"target\" | \"node_modules\")",
                "public TOML style check must prune generated dependency directories",
            ),
        ],
    )
}
