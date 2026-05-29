//! Thin Cargo entrypoint for the npm Tovuk CLI source of truth.

use std::{
    env, fmt,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const VERSION: &str = "0.1.49";
const NPM_PACKAGE: &str = "tovuk";
const NPM_PACKAGE_VERSION: &str = "0.1.49";

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            error.print();
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, CliError> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if wants_version(&args) {
        println!("{VERSION}");
        return Ok(ExitCode::SUCCESS);
    }

    let delegate = delegate_command()?;
    let status = Command::new(&delegate.program)
        .args(&delegate.args)
        .args(args)
        .status()
        .map_err(|error| CliError::from_command(&error))?;

    Ok(exit_code(status.code()))
}

fn wants_version(args: &[String]) -> bool {
    args.len() == 1 && matches!(args[0].as_str(), "--version" | "-v" | "-V")
}

fn delegate_command() -> Result<DelegateCommand, CliError> {
    if let Some(path) = env::var_os("TOVUK_NPM_CLI").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(CliError::new(format!(
                "TOVUK_NPM_CLI does not point to a file: {}",
                path.display()
            )));
        }
        let tsx = local_tsx_command(&path)?;
        return Ok(DelegateCommand {
            program: tsx,
            args: vec![path.into_os_string().to_string_lossy().into_owned()],
        });
    }

    let Some(npx) = command_path("npx") else {
        return Err(CliError::new(
            "Node.js npm tooling is required for the Cargo Tovuk CLI.",
        ));
    };
    Ok(DelegateCommand {
        program: npx,
        args: vec!["-y".to_owned(), npm_package_spec()],
    })
}

fn npm_package_spec() -> String {
    format!("{NPM_PACKAGE}@{NPM_PACKAGE_VERSION}")
}

fn command_path(name: &str) -> Option<String> {
    if Path::new(name).components().count() > 1 {
        return Path::new(name).is_file().then(|| name.to_owned());
    }

    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .flat_map(|directory| executable_candidates(&directory, name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.into_os_string().to_string_lossy().into_owned())
}

fn local_tsx_command(cli_path: &Path) -> Result<String, CliError> {
    let Some(package_root) = cli_path.parent().and_then(Path::parent) else {
        return Err(CliError::new(
            "Could not locate the Tovuk npm package root from TOVUK_NPM_CLI.",
        ));
    };
    let local_bin = if cfg!(windows) {
        package_root
            .join("node_modules")
            .join(".bin")
            .join("tsx.cmd")
    } else {
        package_root.join("node_modules").join(".bin").join("tsx")
    };
    if local_bin.is_file() {
        return Ok(local_bin.into_os_string().to_string_lossy().into_owned());
    }

    command_path("tsx").ok_or_else(|| {
        CliError::new(
            "tsx is required to run the local TypeScript Tovuk npm CLI. Run npm install in packages/tovuk.",
        )
    })
}

fn executable_candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
    let base = directory.join(name);
    if cfg!(windows) {
        vec![base, directory.join(format!("{name}.cmd"))]
    } else {
        vec![base]
    }
}

fn exit_code(code: Option<i32>) -> ExitCode {
    let Some(code) = code else {
        return ExitCode::FAILURE;
    };
    let Ok(code) = u8::try_from(code) else {
        return ExitCode::FAILURE;
    };
    ExitCode::from(code)
}

struct DelegateCommand {
    program: String,
    args: Vec<String>,
}

#[derive(Debug)]
struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn from_command(error: &std::io::Error) -> Self {
        Self::new(format!("Could not start Tovuk npm CLI: {error}"))
    }

    fn print(&self) {
        eprintln!("{}", self.message);
        eprintln!(
            "agent_instruction: Install Node.js 18+ with npx, or set TOVUK_NPM_CLI to packages/tovuk/src/tovuk.ts after running npm install in packages/tovuk."
        );
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}
