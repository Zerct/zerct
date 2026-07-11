//! Python wheel and source-distribution artifact policy.

use alloc::collections::BTreeSet;

use core::str::from_utf8;

use std::path::Path;

use tovuk_public_checks::check_support::CheckResult;

use super::{
    WrapperEvidence,
    archive::{PackageArchive, read_tar_gz},
    policy::{
        require_file_name, require_license, require_metadata, require_native_targets,
        require_python_project, require_python_version, require_wheel_file_name,
    },
    zip_archive::read_zip,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000a] = [
    size_of_val(&require_entry_points),
    size_of_val(&require_metadata_contract),
    size_of_val(&require_record),
    size_of_val(&require_record_row),
    size_of_val(&require_wheel_contract),
    size_of_val(&require_wheel_metadata),
    size_of_val(&sdist_files),
    size_of_val(&validate_sdist),
    size_of_val(&validate_wheel),
    size_of_val(&wheel_files),
];

/// Require the exact public console-script mapping.
///
/// # Errors
///
/// Returns an error when the file is not UTF-8 or maps another command.
fn require_entry_points(contents: &[u8]) -> CheckResult {
    let text = check_try!(
        from_utf8(contents)
            .map_err(|error| return format!("wheel entry_points.txt is not UTF-8: {error}"))
    );
    return (text.trim() == "[console_scripts]\ntovuk = tovuk.cli:main")
        .then_some(())
        .ok_or_else(|| {
            return "wheel entry_points.txt has an unexpected command mapping".to_owned();
        });
}

/// Require extended Python metadata fields and no runtime dependency metadata.
///
/// # Errors
///
/// Returns an error when core metadata is incomplete or declares dependencies.
fn require_metadata_contract(contents: &[u8], version: &str, label: &str) -> CheckResult {
    check_try!(require_metadata(contents, version, label));
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    for required in ["License-File: LICENSE", "Requires-Python: >=3.11"] {
        if !text.lines().any(|line| return line == required) {
            return Err(format!("{label} is missing {required}"));
        }
    }
    if text
        .lines()
        .any(|line| return line.starts_with("Requires-Dist:"))
    {
        return Err(format!("{label} must not declare runtime dependencies"));
    }
    return Ok(());
}

/// Require a complete wheel `RECORD` with one row per packaged file.
///
/// # Errors
///
/// Returns an error when rows are malformed, duplicated, or incomplete.
fn require_record(contents: &[u8], expected_files: &[String], record_path: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("wheel RECORD is not UTF-8: {error}"))
    );
    let mut recorded = BTreeSet::new();
    for line in text.lines() {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.len() != 0x3 {
            return Err(format!("wheel RECORD row {line:?} must have three fields"));
        }
        let path = check_try!(
            fields
                .first()
                .copied()
                .filter(|value| return !value.is_empty())
                .ok_or_else(|| return "wheel RECORD contains an empty path".to_owned())
        );
        if !recorded.insert(path.to_owned()) {
            return Err(format!("wheel RECORD contains duplicate path {path}"));
        }
        let digest = check_try!(
            fields
                .get(0x1)
                .copied()
                .ok_or_else(|| return format!("wheel RECORD row {path} lacks a digest"))
        );
        let size = check_try!(
            fields
                .get(0x2)
                .copied()
                .ok_or_else(|| return format!("wheel RECORD row {path} lacks a size"))
        );
        check_try!(require_record_row(path, digest, size, record_path));
    }
    let expected = expected_files.iter().cloned().collect::<BTreeSet<_>>();
    return (recorded == expected)
        .then_some(())
        .ok_or_else(|| return "wheel RECORD paths must match archive files exactly".to_owned());
}

/// Require one wheel `RECORD` row's hash and size shape.
///
/// # Errors
///
/// Returns an error when the record self-row is nonempty or another row lacks
/// its SHA-256 and numeric size.
fn require_record_row(path: &str, digest: &str, size: &str, record_path: &str) -> CheckResult {
    if path == record_path {
        return (digest.is_empty() && size.is_empty())
            .then_some(())
            .ok_or_else(|| {
                return "wheel RECORD must leave its own digest and size empty".to_owned();
            });
    }
    let valid = digest.starts_with("sha256=")
        && !size.is_empty()
        && size.bytes().all(|byte| return byte.is_ascii_digit());
    return valid.then_some(()).ok_or_else(|| {
        return format!("wheel RECORD row {path} must contain a SHA-256 digest and numeric size");
    });
}

/// Require the pure-Python universal wheel metadata contract.
///
/// # Errors
///
/// Returns an error when wheel metadata is not UTF-8 or is not universal.
fn require_wheel_contract(contents: &[u8]) -> CheckResult {
    let text = check_try!(
        from_utf8(contents)
            .map_err(|error| return format!("wheel WHEEL metadata is not UTF-8: {error}"))
    );
    for required in [
        "Generator: uv 0.11.28",
        "Wheel-Version: 1.0",
        "Root-Is-Purelib: true",
        "Tag: py3-none-any",
    ] {
        if !text.lines().any(|line| return line == required) {
            return Err(format!("wheel WHEEL metadata is missing {required}"));
        }
    }
    return Ok(());
}

/// Require generated wheel metadata, entry points, and record consistency.
///
/// # Errors
///
/// Returns an error when any generated wheel metadata file differs.
fn require_wheel_metadata(
    archive: &PackageArchive,
    information_root: &str,
    expected: &[String],
    version: &str,
) -> CheckResult {
    let metadata_path = format!("{information_root}/METADATA");
    check_try!(require_metadata_contract(
        check_try!(archive.file(metadata_path.as_str(), "Python wheel")),
        version,
        "wheel METADATA",
    ));
    let wheel_path = format!("{information_root}/WHEEL");
    check_try!(require_wheel_contract(check_try!(
        archive.file(wheel_path.as_str(), "Python wheel")
    )));
    let entry_path = format!("{information_root}/entry_points.txt");
    check_try!(require_entry_points(check_try!(
        archive.file(entry_path.as_str(), "Python wheel")
    )));
    let record_path = format!("{information_root}/RECORD");
    return require_record(
        check_try!(archive.file(record_path.as_str(), "Python wheel")),
        expected,
        record_path.as_str(),
    );
}

/// Return the exact Python source-distribution regular-file set.
fn sdist_files(root: &str) -> Vec<String> {
    return [
        "LICENSE",
        "PKG-INFO",
        "README.md",
        "pyproject.toml",
        "src/tovuk/__init__.py",
        "src/tovuk/__main__.py",
        "src/tovuk/cli.py",
        "src/tovuk/native_release_targets.json",
        "tests/__init__.py",
        "tests/test_cli.py",
    ]
    .map(|relative| return format!("{root}/{relative}"))
    .to_vec();
}

/// Validate one Python source-distribution artifact.
///
/// # Errors
///
/// Returns an error when the archive name, root, files, project metadata,
/// license, runtime version, or native target manifest differs.
pub(super) fn validate_sdist(path: &Path, version: &str) -> CheckResult<WrapperEvidence> {
    check_try!(require_file_name(
        path,
        format!("tovuk-{version}.tar.gz").as_str(),
        "Python sdist",
    ));
    let archive = check_try!(read_tar_gz(path, "Python sdist"));
    let root = format!("tovuk-{version}");
    check_try!(archive.require_root(root.as_str(), "Python sdist"));
    check_try!(archive.require_exact_files(sdist_files(root.as_str()).as_slice(), "Python sdist",));
    let metadata_path = format!("{root}/PKG-INFO");
    check_try!(require_metadata_contract(
        check_try!(archive.file(metadata_path.as_str(), "Python sdist")),
        version,
        "sdist PKG-INFO",
    ));
    let project_path = format!("{root}/pyproject.toml");
    check_try!(require_python_project(
        check_try!(archive.file(project_path.as_str(), "Python sdist")),
        version,
        "sdist pyproject.toml",
    ));
    let init_path = format!("{root}/src/tovuk/__init__.py");
    check_try!(require_python_version(
        check_try!(archive.file(init_path.as_str(), "Python sdist")),
        version,
        "sdist tovuk/__init__.py",
    ));
    let license_path = format!("{root}/LICENSE");
    let license = check_try!(archive.file(license_path.as_str(), "Python sdist"));
    check_try!(require_license(license, "Python sdist"));
    let targets_path = format!("{root}/src/tovuk/native_release_targets.json");
    let native_targets = check_try!(archive.file(targets_path.as_str(), "Python sdist"));
    check_try!(require_native_targets(
        native_targets,
        "sdist native target manifest",
    ));
    return Ok(WrapperEvidence {
        license: license.to_vec(),
        native_targets: native_targets.to_vec(),
    });
}

/// Validate one Python wheel artifact.
///
/// # Errors
///
/// Returns an error when the filename, exact file set, metadata, record,
/// license, runtime version, or native target manifest differs.
pub(super) fn validate_wheel(path: &Path, version: &str) -> CheckResult<WrapperEvidence> {
    check_try!(require_wheel_file_name(path, version));
    let archive = check_try!(read_zip(path, "Python wheel"));
    let information_root = format!("tovuk-{version}.dist-info");
    let expected = wheel_files(information_root.as_str());
    check_try!(archive.require_exact_files(expected.as_slice(), "Python wheel"));
    check_try!(require_wheel_metadata(
        &archive,
        information_root.as_str(),
        expected.as_slice(),
        version,
    ));
    check_try!(require_python_version(
        check_try!(archive.file("tovuk/__init__.py", "Python wheel")),
        version,
        "wheel tovuk/__init__.py",
    ));
    let license_path = format!("{information_root}/licenses/LICENSE");
    let license = check_try!(archive.file(license_path.as_str(), "Python wheel"));
    check_try!(require_license(license, "Python wheel"));
    let native_targets =
        check_try!(archive.file("tovuk/native_release_targets.json", "Python wheel",));
    check_try!(require_native_targets(
        native_targets,
        "wheel native target manifest",
    ));
    return Ok(WrapperEvidence {
        license: license.to_vec(),
        native_targets: native_targets.to_vec(),
    });
}

/// Return the exact Python wheel regular-file set.
fn wheel_files(information_root: &str) -> Vec<String> {
    return [
        "tovuk/__init__.py".to_owned(),
        "tovuk/__main__.py".to_owned(),
        "tovuk/cli.py".to_owned(),
        "tovuk/native_release_targets.json".to_owned(),
        format!("{information_root}/METADATA"),
        format!("{information_root}/RECORD"),
        format!("{information_root}/WHEEL"),
        format!("{information_root}/entry_points.txt"),
        format!("{information_root}/licenses/LICENSE"),
    ]
    .to_vec();
}
