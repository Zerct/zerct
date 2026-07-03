//! Repository prose style checks for public Tovuk text files.

use std::{
    ffi::OsStr,
    fs,
    path::Path,
    process::{Command, ExitCode},
};

#[derive(Debug, PartialEq, Eq)]
struct Finding {
    file: String,
    line: usize,
    column: usize,
    message: &'static str,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [flag] if flag == "--self-test" => run_self_test(),
        [] => scan_repository(),
        _ => Err("usage: check-prose-style [--self-test]".to_owned()),
    }
}

fn run_self_test() -> Result<(), String> {
    let findings = line_findings("self-test", 0, "bad punctuation \u{2014} stop");
    match findings.as_slice() {
        [finding] if finding.column == 17 => {
            println!("Style checker self-test passed.");
            Ok(())
        }
        _ => Err("self-test em dash fixture failed".to_owned()),
    }
}

fn scan_repository() -> Result<(), String> {
    let mut text_file_count = 0usize;
    let mut findings = Vec::new();

    for file in git_files()? {
        if has_ignored_binary_extension(&file) {
            continue;
        }

        let contents = match fs::read(&file) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("read {file}: {error}")),
        };
        if contents.contains(&0) {
            continue;
        }

        let text = String::from_utf8_lossy(&contents);
        text_file_count += 1;
        for (line_index, line) in text.replace("\r\n", "\n").split('\n').enumerate() {
            findings.extend(line_findings(&file, line_index, line));
        }
    }

    if findings.is_empty() {
        println!("Checked {text_file_count} text files for em dashes.");
        return Ok(());
    }

    eprintln!("Style check failed.");
    eprintln!("Em dash is banned in every tracked text file.");
    for finding in findings {
        eprintln!(
            "{}:{}:{}: {}",
            finding.file, finding.line, finding.column, finding.message
        );
    }
    Err("prose style check failed".to_owned())
}

fn git_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "--cached"])
        .output()
        .map_err(|error| format!("run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err(format!("git ls-files failed with status {}", output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn line_findings(file: &str, line_index: usize, line: &str) -> Vec<Finding> {
    line.match_indices('\u{2014}')
        .map(|(index, _)| Finding {
            file: file.to_owned(),
            line: line_index + 1,
            column: index + 1,
            message: "em dash is not allowed in any tracked text file",
        })
        .collect()
}

fn has_ignored_binary_extension(file: &str) -> bool {
    matches!(
        Path::new(file).extension().and_then(OsStr::to_str),
        Some(
            "avif"
                | "gif"
                | "ico"
                | "jpeg"
                | "jpg"
                | "otf"
                | "pdf"
                | "png"
                | "ttf"
                | "webp"
                | "woff"
                | "woff2",
        )
    )
}
