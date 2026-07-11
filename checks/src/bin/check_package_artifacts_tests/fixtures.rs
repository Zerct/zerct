//! Synthetic package archive builders.

use core::fmt::Write as _;

use flate2::{Compression, write::GzEncoder};

use std::{
    env,
    fs::{File, create_dir_all, remove_dir_all, write},
    path::{Path, PathBuf},
    process,
};

use tar::{Builder as TarBuilder, Header as TarHeader};

use tovuk_public_checks::check_support::{CheckResult, command, repo_root, tool_path};

use super::{ArtifactRequest, cargo_package::PACKAGED_CARGO_CONFIG, zip_archive::crc32};

/// Mask for one serialized byte.
const BYTE_MASK: u32 = 0x00ff;
/// ZIP central-directory header signature.
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
/// ZIP end-of-central-directory record signature.
const END_SIGNATURE: u32 = 0x0605_4b50;
/// Fourth little-endian byte shift.
const FOURTH_BYTE_SHIFT: u32 = 0x18;
/// Synthetic complete MIT license text.
const LICENSE: &str = "MIT License\nPermission is hereby granted\nTHE SOFTWARE IS PROVIDED\n";
/// ZIP local-file header signature.
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
/// Synthetic generated native-target manifest.
const NATIVE_TARGETS: &str = "{\"targets\":[{\"triple\":\"test-target\"}]}\n";
/// Second little-endian byte shift.
const SECOND_BYTE_SHIFT: u32 = 0x8;

/// Third little-endian byte shift.
const THIRD_BYTE_SHIFT: u32 = 0x10;

/// Synthetic synchronized package version.
pub(super) const VERSION: &str = "1.2.3";

/// Compile-time references preserve the named fixture-helper boundaries.
const _: [usize; 0x0011] = [
    size_of_val(&artifact_request),
    size_of_val(&current_head),
    size_of_val(&push_central_record),
    size_of_val(&push_local_record),
    size_of_val(&push_u16),
    size_of_val(&push_u32),
    size_of_val(&symlink_entry),
    size_of_val(&test_directory),
    size_of_val(&text_entry),
    size_of_val(&wheel_paths),
    size_of_val(&wheel_record),
    size_of_val(&write_cargo),
    size_of_val(&write_npm),
    size_of_val(&write_sdist),
    size_of_val(&write_tar),
    size_of_val(&write_wheel),
    size_of_val(&write_zip),
];

/// One synthetic archive member.
#[derive(Debug)]
pub(super) struct ArchiveEntry {
    /// Exact file contents.
    contents: Vec<u8>,
    /// Archive-relative path.
    path: String,
    /// Optional Unix mode embedded in a ZIP central header.
    unix_mode: Option<u32>,
}

/// Metadata retained while constructing a ZIP central directory.
#[derive(Debug)]
struct ZipRecord {
    /// Stored file size.
    compressed_size: u32,
    /// File CRC-32.
    crc32: u32,
    /// Local-header offset.
    local_offset: u32,
    /// UTF-8 path byte length.
    name_length: u16,
    /// Archive-relative path.
    path: String,
    /// Optional Unix mode.
    unix_mode: Option<u32>,
}

/// Build every valid synthetic package artifact.
/// # Errors
/// Returns an error when Git or fixture I/O fails.
pub(super) fn artifact_request(directory: &Path) -> CheckResult<ArtifactRequest> {
    let cargo_archive = directory.join(format!("tovuk-{VERSION}.crate"));
    let npm_archive = directory.join(format!("tovuk-{VERSION}.tgz"));
    let python_wheel = directory.join(format!("tovuk-{VERSION}-py3-none-any.whl"));
    let python_sdist = directory.join(format!("tovuk-{VERSION}.tar.gz"));
    let head = check_try!(current_head());
    check_try!(write_cargo(cargo_archive.as_path(), VERSION, head.as_str()));
    check_try!(write_npm(npm_archive.as_path(), VERSION, VERSION));
    check_try!(write_sdist(python_sdist.as_path(), VERSION));
    check_try!(write_wheel(python_wheel.as_path(), VERSION));
    return Ok(ArtifactRequest {
        cargo_archive,
        npm_archive,
        python_sdist,
        python_wheel,
        version: VERSION.to_owned(),
    });
}

/// Read the current lowercase Git commit identifier.
/// # Errors
/// Returns an error when repository discovery or Git fails.
fn current_head() -> CheckResult<String> {
    let repository = check_try!(repo_root());
    let output = check_try!(
        command(repository.as_path(), tool_path().as_os_str(), "git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|error| return format!("run git rev-parse HEAD: {error}"))
    );
    if !output.status.success() {
        return Err(format!("git rev-parse HEAD failed: {}", output.status));
    }
    return String::from_utf8(output.stdout)
        .map(|head| return head.trim().to_owned())
        .map_err(|error| return format!("git HEAD is not UTF-8: {error}"));
}

/// Append one stored ZIP central-directory record.
fn push_central_record(output: &mut Vec<u8>, record: &ZipRecord) {
    push_u32(output, CENTRAL_SIGNATURE);
    push_u16(
        output,
        record.unix_mode.map_or(0x0014, |_mode| return 0x0314),
    );
    push_u16(output, 0x0014);
    for value in [u16::MIN; 0x4] {
        push_u16(output, value);
    }
    push_u32(output, record.crc32);
    push_u32(output, record.compressed_size);
    push_u32(output, record.compressed_size);
    push_u16(output, record.name_length);
    for value in [u16::MIN; 0x4] {
        push_u16(output, value);
    }
    let attributes = record
        .unix_mode
        .map_or(u32::MIN, |mode| return mode << THIRD_BYTE_SHIFT);
    push_u32(output, attributes);
    push_u32(output, record.local_offset);
    output.extend_from_slice(record.path.as_bytes());
}

/// Append one stored ZIP local header and return its central metadata.
/// # Errors
/// Returns an error when a path or content length exceeds classic ZIP fields.
fn push_local_record(output: &mut Vec<u8>, entry: &ArchiveEntry) -> CheckResult<ZipRecord> {
    let local_offset = check_try!(
        u32::try_from(output.len())
            .map_err(|error| return format!("convert local ZIP offset: {error}"))
    );
    let name_length = check_try!(
        u16::try_from(entry.path.len())
            .map_err(|error| return format!("convert ZIP path length: {error}"))
    );
    let compressed_size = check_try!(
        u32::try_from(entry.contents.len())
            .map_err(|error| return format!("convert ZIP content length: {error}"))
    );
    let checksum = crc32(entry.contents.as_slice());
    push_u32(output, LOCAL_SIGNATURE);
    push_u16(output, 0x0014);
    for value in [u16::MIN; 0x4] {
        push_u16(output, value);
    }
    push_u32(output, checksum);
    push_u32(output, compressed_size);
    push_u32(output, compressed_size);
    push_u16(output, name_length);
    push_u16(output, u16::MIN);
    output.extend_from_slice(entry.path.as_bytes());
    output.extend_from_slice(entry.contents.as_slice());
    return Ok(ZipRecord {
        compressed_size,
        crc32: checksum,
        local_offset,
        name_length,
        path: entry.path.clone(),
        unix_mode: entry.unix_mode,
    });
}

/// Append a little-endian `u16` without host-endian conversion helpers.
fn push_u16(output: &mut Vec<u8>, value: u16) {
    let expanded = u32::from(value);
    output.push(u8::try_from(expanded & BYTE_MASK).unwrap_or_default());
    output.push(u8::try_from((expanded >> SECOND_BYTE_SHIFT) & BYTE_MASK).unwrap_or_default());
}

/// Append a little-endian `u32` without host-endian conversion helpers.
fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.push(u8::try_from(value & BYTE_MASK).unwrap_or_default());
    output.push(u8::try_from((value >> SECOND_BYTE_SHIFT) & BYTE_MASK).unwrap_or_default());
    output.push(u8::try_from((value >> THIRD_BYTE_SHIFT) & BYTE_MASK).unwrap_or_default());
    output.push(u8::try_from((value >> FOURTH_BYTE_SHIFT) & BYTE_MASK).unwrap_or_default());
}

/// Build one symbolic-link ZIP member.
pub(super) fn symlink_entry(path: &str, target: &str) -> ArchiveEntry {
    return ArchiveEntry {
        contents: target.as_bytes().to_vec(),
        path: path.to_owned(),
        unix_mode: Some(0o120_777),
    };
}

/// Return an isolated deterministic test directory.
/// # Errors
/// Returns an error when stale cleanup or directory creation fails.
pub(super) fn test_directory(label: &str) -> CheckResult<PathBuf> {
    let directory =
        env::temp_dir().join(format!("tovuk-package-artifacts-{}-{label}", process::id()));
    if directory.exists() {
        check_try!(
            remove_dir_all(directory.as_path())
                .map_err(|error| return format!("remove {}: {error}", directory.display()))
        );
    }
    check_try!(
        create_dir_all(directory.as_path())
            .map_err(|error| return format!("create {}: {error}", directory.display()))
    );
    return Ok(directory);
}

/// Build one regular text archive member.
pub(super) fn text_entry(path: &str, contents: &str) -> ArchiveEntry {
    return ArchiveEntry {
        contents: contents.as_bytes().to_vec(),
        path: path.to_owned(),
        unix_mode: None,
    };
}

/// Return the exact synthetic wheel paths.
fn wheel_paths(version: &str) -> Vec<String> {
    let information = format!("tovuk-{version}.dist-info");
    return [
        "tovuk/__init__.py".to_owned(),
        "tovuk/__main__.py".to_owned(),
        "tovuk/cli.py".to_owned(),
        "tovuk/native_release_targets.json".to_owned(),
        format!("{information}/METADATA"),
        format!("{information}/RECORD"),
        format!("{information}/WHEEL"),
        format!("{information}/entry_points.txt"),
        format!("{information}/licenses/LICENSE"),
    ]
    .to_vec();
}

/// Build a complete synthetic wheel `RECORD`.
/// # Errors
/// Returns an error when writing to the in-memory string fails.
fn wheel_record(paths: &[String]) -> CheckResult<String> {
    let mut record = String::new();
    for path in paths {
        let result = if path.ends_with("/RECORD") {
            writeln!(record, "{path},,")
        } else {
            writeln!(record, "{path},sha256=test,1")
        };
        check_try!(result.map_err(|error| return format!("write wheel RECORD: {error}")));
    }
    return Ok(record);
}

/// Write a synthetic Cargo package.
/// # Errors
/// Returns an error when archive construction fails.
fn write_cargo(path: &Path, version: &str, head: &str) -> CheckResult {
    let root = format!("tovuk-{version}");
    let manifest =
        format!("[package]\nname = \"tovuk\"\nversion = \"{version}\"\nlicense = \"MIT\"\n");
    let lock = format!(
        "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"registry-dependency\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n\n[[package]]\nname = \"tovuk\"\nversion = \"{version}\"\n"
    );
    let vcs = format!("{{\"git\":{{\"sha1\":\"{head}\"}},\"path_in_vcs\":\"crates/tovuk\"}}");
    let entries = vec![
        text_entry(
            format!("{root}/.cargo/config.toml").as_str(),
            PACKAGED_CARGO_CONFIG,
        ),
        text_entry(
            format!("{root}/.cargo_vcs_info.json").as_str(),
            vcs.as_str(),
        ),
        text_entry(format!("{root}/Cargo.lock").as_str(), lock.as_str()),
        text_entry(format!("{root}/Cargo.toml").as_str(), manifest.as_str()),
        text_entry(
            format!("{root}/Cargo.toml.orig").as_str(),
            manifest.as_str(),
        ),
        text_entry(format!("{root}/LICENSE").as_str(), LICENSE),
        text_entry(format!("{root}/README.md").as_str(), "# Tovuk\n"),
        text_entry(format!("{root}/src/main.rs").as_str(), "fn main() {}\n"),
    ];
    return write_tar(path, entries.as_slice());
}

/// Write a synthetic npm package with independently selected metadata version.
///
/// # Errors
///
/// Returns an error when archive construction fails.
pub(super) fn write_npm(path: &Path, archive_version: &str, metadata_version: &str) -> CheckResult {
    let package_json = format!(
        "{{\"name\":\"tovuk\",\"version\":\"{metadata_version}\",\"license\":\"MIT\",\"type\":\"module\",\"bin\":{{\"tovuk\":\"bin/tovuk.mjs\"}},\"scripts\":{{\"postinstall\":\"node install.mjs\"}}}}"
    );
    let entries = vec![
        text_entry("package/LICENSE", LICENSE),
        text_entry("package/README.md", "# Tovuk\n"),
        text_entry("package/bin/tovuk.mjs", "process.exitCode = 0\n"),
        text_entry("package/install-policy.mjs", "export default {}\n"),
        text_entry("package/install.mjs", "await Promise.resolve()\n"),
        text_entry("package/native-release-targets.json", NATIVE_TARGETS),
        text_entry("package/package.json", package_json.as_str()),
    ];
    let expected = format!("tovuk-{archive_version}.tgz");
    if path.file_name().and_then(|name| return name.to_str()) != Some(expected.as_str()) {
        return Err("synthetic npm archive filename differs".to_owned());
    }
    return write_tar(path, entries.as_slice());
}

/// Write a synthetic Python source distribution.
///
/// # Errors
///
/// Returns an error when archive construction fails.
fn write_sdist(path: &Path, version: &str) -> CheckResult {
    let root = format!("tovuk-{version}");
    let metadata = format!(
        "Metadata-Version: 2.4\nName: tovuk\nVersion: {version}\nLicense-Expression: MIT\nLicense-File: LICENSE\nRequires-Python: >=3.11\n\n# Tovuk\n"
    );
    let project =
        format!("[project]\nname = \"tovuk\"\nversion = \"{version}\"\nlicense = \"MIT\"\n");
    let init = format!("__version__ = \"{version}\"\n");
    let entries = vec![
        text_entry(format!("{root}/LICENSE").as_str(), LICENSE),
        text_entry(format!("{root}/PKG-INFO").as_str(), metadata.as_str()),
        text_entry(format!("{root}/README.md").as_str(), "# Tovuk\n"),
        text_entry(format!("{root}/pyproject.toml").as_str(), project.as_str()),
        text_entry(
            format!("{root}/src/tovuk/__init__.py").as_str(),
            init.as_str(),
        ),
        text_entry(
            format!("{root}/src/tovuk/__main__.py").as_str(),
            "from .cli import main\n",
        ),
        text_entry(
            format!("{root}/src/tovuk/cli.py").as_str(),
            "def main():\n    return None\n",
        ),
        text_entry(
            format!("{root}/src/tovuk/native_release_targets.json").as_str(),
            NATIVE_TARGETS,
        ),
        text_entry(
            format!("{root}/tests/__init__.py").as_str(),
            "\"\"\"Tests.\"\"\"\n",
        ),
        text_entry(
            format!("{root}/tests/test_cli.py").as_str(),
            "def test_cli():\n    pass\n",
        ),
    ];
    return write_tar(path, entries.as_slice());
}

/// Write a gzip-compressed tar fixture.
///
/// # Errors
///
/// Returns an error when the output or any member cannot be written.
pub(super) fn write_tar(path: &Path, entries: &[ArchiveEntry]) -> CheckResult {
    let file = check_try!(
        File::create(path).map_err(|error| return format!("create {}: {error}", path.display()))
    );
    let mut archive = TarBuilder::new(GzEncoder::new(file, Compression::default()));
    for entry in entries {
        let mut header = TarHeader::new_gnu();
        header.set_mode(0o644);
        header.set_size(check_try!(
            u64::try_from(entry.contents.len())
                .map_err(|error| return format!("measure tar member: {error}"))
        ));
        header.set_cksum();
        check_try!(
            archive
                .append_data(&mut header, entry.path.as_str(), entry.contents.as_slice())
                .map_err(|error| return format!("append {}: {error}", entry.path))
        );
    }
    let encoder = check_try!(
        archive
            .into_inner()
            .map_err(|error| return format!("finish tar {}: {error}", path.display()))
    );
    drop(check_try!(encoder.finish().map_err(
        |error| return format!("finish gzip {}: {error}", path.display())
    )));
    return Ok(());
}

/// Write a synthetic pure-Python wheel.
///
/// # Errors
///
/// Returns an error when record or ZIP construction fails.
fn write_wheel(path: &Path, version: &str) -> CheckResult {
    let information = format!("tovuk-{version}.dist-info");
    let paths = wheel_paths(version);
    let record = check_try!(wheel_record(paths.as_slice()));
    let metadata = format!(
        "Metadata-Version: 2.4\nName: tovuk\nVersion: {version}\nLicense-Expression: MIT\nLicense-File: LICENSE\nRequires-Python: >=3.11\n\n# Tovuk\n"
    );
    let init = format!("__version__ = \"{version}\"\n");
    let entries = vec![
        text_entry("tovuk/__init__.py", init.as_str()),
        text_entry("tovuk/__main__.py", "from .cli import main\n"),
        text_entry("tovuk/cli.py", "def main():\n    return None\n"),
        text_entry("tovuk/native_release_targets.json", NATIVE_TARGETS),
        text_entry(
            format!("{information}/METADATA").as_str(),
            metadata.as_str(),
        ),
        text_entry(format!("{information}/RECORD").as_str(), record.as_str()),
        text_entry(
            format!("{information}/WHEEL").as_str(),
            "Wheel-Version: 1.0\nGenerator: uv 0.11.28\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
        ),
        text_entry(
            format!("{information}/entry_points.txt").as_str(),
            "[console_scripts]\ntovuk = tovuk.cli:main\n",
        ),
        text_entry(format!("{information}/licenses/LICENSE").as_str(), LICENSE),
    ];
    return write_zip(path, entries.as_slice());
}

/// Write a classic stored ZIP fixture.
///
/// # Errors
///
/// Returns an error when a field exceeds classic ZIP bounds or writing fails.
pub(super) fn write_zip(path: &Path, entries: &[ArchiveEntry]) -> CheckResult {
    let mut output = Vec::new();
    let mut records = Vec::new();
    for entry in entries {
        records.push(check_try!(push_local_record(&mut output, entry)));
    }
    let central_offset = check_try!(
        u32::try_from(output.len())
            .map_err(|error| return format!("convert central ZIP offset: {error}"))
    );
    for record in &records {
        push_central_record(&mut output, record);
    }
    let central_end = check_try!(
        u32::try_from(output.len())
            .map_err(|error| return format!("convert central ZIP end: {error}"))
    );
    let central_size = check_try!(
        central_end
            .checked_sub(central_offset)
            .ok_or_else(|| return "central ZIP size underflow".to_owned())
    );
    let entry_count = check_try!(
        u16::try_from(records.len())
            .map_err(|error| return format!("convert ZIP entry count: {error}"))
    );
    push_u32(&mut output, END_SIGNATURE);
    push_u16(&mut output, u16::MIN);
    push_u16(&mut output, u16::MIN);
    push_u16(&mut output, entry_count);
    push_u16(&mut output, entry_count);
    push_u32(&mut output, central_size);
    push_u32(&mut output, central_offset);
    push_u16(&mut output, u16::MIN);
    return write(path, output)
        .map_err(|error| return format!("write {}: {error}", path.display()));
}
