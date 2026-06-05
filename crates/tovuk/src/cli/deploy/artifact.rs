use super::super::{
    check::first_output_line, command_policy::command_tokens, config::TovukConfig,
    project::number_field, project_kind::ProjectKind,
};
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) fn artifact_check(
    project_dir: &Path,
    config: Option<&TovukConfig>,
    limits: &Value,
    requested: bool,
    checks_ok: bool,
) -> Value {
    if !requested {
        return json!({
            "requested": false,
            "status": "skipped",
            "agent_instruction": "Run `tovuk deploy --dry-run --build-artifact --json` after adding or upgrading Rust dependencies to build the local release binary and check compressed worker size before deploy.",
        });
    }

    if !checks_ok {
        return json!({
            "requested": true,
            "status": "skipped_failed_checks",
            "ok": false,
            "message": "Artifact build was skipped because normal dry-run checks failed first.",
            "agent_instruction": "Fix the first failed quality check, then rerun `tovuk deploy --dry-run --build-artifact --json`.",
        });
    }

    let Some(config) = config else {
        return artifact_failure(
            "missing_config",
            "tovuk.toml must pass validation before artifact size can be checked.",
            "Fix tovuk.toml, rerun `tovuk check`, then rerun `tovuk deploy --dry-run --build-artifact --json`.",
        );
    };

    let Some(target) = worker_artifact_target(project_dir, config) else {
        return json!({
            "requested": true,
            "status": "not_applicable",
            "ok": true,
            "message": "No Rust worker artifact is required for this service kind.",
        });
    };

    run_artifact_check(&target, limits)
}

fn run_artifact_check(target: &WorkerArtifactTarget, limits: &Value) -> Value {
    let build = shell_command(&target.build_command)
        .current_dir(&target.build_dir)
        .stdin(Stdio::null())
        .output();
    let output = match build {
        Ok(output) => output,
        Err(error) => {
            return artifact_failure(
                "build_command_failed",
                &format!("Could not run worker build command: {error}"),
                &format!(
                    "Install the Rust toolchain, run `{}` in `{}`, fix the build, then rerun artifact dry-run.",
                    target.build_command,
                    target.build_dir.display()
                ),
            );
        }
    };
    if !output.status.success() {
        return artifact_failure(
            "build_failed",
            &first_output_line(&output.stderr, &output.stdout, "worker build failed"),
            &format!(
                "Run `{}` in `{}`, fix the first build error, then rerun artifact dry-run.",
                target.build_command,
                target.build_dir.display()
            ),
        );
    }

    let uncompressed_bytes = match fs::metadata(&target.binary_path) {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            return artifact_failure(
                "binary_missing",
                &format!("Worker binary was not found after build: {error}"),
                &format!(
                    "Ensure the worker runtime command points to the release binary built by `{}`, then rerun artifact dry-run.",
                    target.build_command
                ),
            );
        }
    };
    let compressed_bytes = match gzip_file_size(&target.binary_path) {
        Ok(size) => size,
        Err(error) => {
            return artifact_failure(
                "gzip_failed",
                &format!("Could not gzip worker binary: {error}"),
                "Check the release binary path and file permissions, then rerun artifact dry-run.",
            );
        }
    };
    let compressed_limit_bytes = worker_compressed_limit_bytes(limits);
    let ok = compressed_limit_bytes == 0 || compressed_bytes <= compressed_limit_bytes;
    json!({
        "requested": true,
        "status": if ok { "passed" } else { "failed" },
        "ok": ok,
        "kind": "rust_worker_binary",
        "buildCommand": target.build_command,
        "buildCwd": target.build_dir.display().to_string(),
        "binaryPath": target.binary_path.display().to_string(),
        "platform": "local",
        "uncompressedBytes": uncompressed_bytes,
        "compressedBytes": compressed_bytes,
        "compressedLimitBytes": compressed_limit_bytes,
        "limitSource": "usage.limits.workerCompressedSizeMib",
        "message": artifact_message(ok, compressed_bytes, compressed_limit_bytes),
        "agent_instruction": artifact_instruction(ok, compressed_bytes, compressed_limit_bytes),
    })
}

fn artifact_message(ok: bool, compressed_bytes: u64, compressed_limit_bytes: u64) -> String {
    if compressed_limit_bytes == 0 {
        return format!("Local worker artifact gzip size is {compressed_bytes} bytes.");
    }
    if ok {
        return format!(
            "Local worker artifact gzip size is {compressed_bytes} bytes, below the {compressed_limit_bytes} byte account limit."
        );
    }
    format!(
        "Local worker artifact gzip size is {compressed_bytes} bytes, above the {compressed_limit_bytes} byte account limit."
    )
}

fn artifact_instruction(
    ok: bool,
    compressed_bytes: u64,
    compressed_limit_bytes: u64,
) -> Option<String> {
    if ok {
        return None;
    }
    Some(format!(
        "Reduce Rust artifact size before deploy. Add or tighten [profile.release] with strip=true, lto=\"fat\", codegen-units=1, panic=\"abort\", and opt-level=\"z\", remove heavyweight dependencies, then rerun `tovuk deploy --dry-run --build-artifact --json`. Current gzip size is {compressed_bytes} bytes; limit is {compressed_limit_bytes} bytes."
    ))
}

fn artifact_failure(code: &str, message: &str, instruction: &str) -> Value {
    json!({
        "requested": true,
        "status": "failed",
        "ok": false,
        "code": code,
        "message": message,
        "agent_instruction": instruction,
    })
}

fn worker_artifact_target(
    project_dir: &Path,
    config: &TovukConfig,
) -> Option<WorkerArtifactTarget> {
    match config.kind {
        ProjectKind::Fullstack => {
            let worker_root = config.backend.root.as_deref().unwrap_or(".");
            let build_dir = project_dir.join(worker_root);
            let build_command = config.backend.build.as_ref()?;
            let run_command = config.backend.command.as_ref()?;
            Some(WorkerArtifactTarget::new(
                &build_dir,
                build_command,
                run_command,
            ))
        }
        ProjectKind::RustWorker => {
            let run_command = config.run.command.as_ref()?;
            Some(WorkerArtifactTarget::new(
                project_dir,
                &config.build.command,
                run_command,
            ))
        }
        ProjectKind::StaticFrontend => None,
    }
}

struct WorkerArtifactTarget {
    binary_path: PathBuf,
    build_command: String,
    build_dir: PathBuf,
}

impl WorkerArtifactTarget {
    fn new(build_dir: &Path, build_command: &str, run_command: &str) -> Self {
        let binary = worker_binary_token(run_command).unwrap_or_else(|| run_command.to_owned());
        Self {
            binary_path: build_dir.join(binary),
            build_command: build_command.to_owned(),
            build_dir: build_dir.to_path_buf(),
        }
    }
}

fn worker_binary_token(run_command: &str) -> Option<String> {
    command_tokens(run_command)
        .into_iter()
        .find(|token| token.contains("target/release/"))
}

fn gzip_file_size(path: &Path) -> io::Result<u64> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    let bytes = fs::read(path)?;
    encoder.write_all(&bytes)?;
    let compressed = encoder.finish()?;
    Ok(compressed.len() as u64)
}

fn worker_compressed_limit_bytes(limits: &Value) -> u64 {
    number_field(limits, "workerCompressedSizeMib") * 1024 * 1024
}

fn shell_command(source: &str) -> Command {
    if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", source]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-c", source]);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::{worker_binary_token, worker_compressed_limit_bytes};
    use serde_json::json;

    #[test]
    fn worker_compressed_limit_uses_usage_limits() {
        assert_eq!(
            worker_compressed_limit_bytes(&json!({ "workerCompressedSizeMib": 3 })),
            3 * 1024 * 1024
        );
    }

    #[test]
    fn worker_binary_token_reads_release_binary() {
        assert_eq!(
            worker_binary_token("RUST_LOG=info ./target/release/api --serve").as_deref(),
            Some("./target/release/api")
        );
    }
}
