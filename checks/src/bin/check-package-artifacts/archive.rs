//! Bounded archive readers and archive-member invariants.

use alloc::collections::{BTreeMap, BTreeSet};

use core::str::from_utf8;

use flate2::read::GzDecoder;

use std::{
    fs::{File, symlink_metadata},
    io::{BufReader, Read},
    path::{Component, Path},
};

use tar::Archive as TarArchive;

use tovuk_public_checks::check_support::CheckResult;

use super::{
    MemberKind,
    policy::{reject_sensitive_content, reject_sensitive_path},
};

/// Largest accepted compressed package archive.
pub(super) const MAX_ARCHIVE_BYTES: u64 = 0x100_0000;

/// Largest accepted unpacked archive member.
pub(super) const MAX_ENTRY_BYTES: u64 = 0x080_0000;

/// Largest accepted number of archive members.
pub(super) const MAX_ENTRY_COUNT: usize = 0x1000;

/// Largest accepted archive member path.
const MAX_PATH_BYTES: usize = 0x0200;

/// Largest accepted aggregate unpacked package size.
const MAX_TOTAL_BYTES: u64 = 0x400_0000;

/// Normalized regular files held in a bounded archive.
pub(super) type ArchiveFiles = BTreeMap<String, Vec<u8>>;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&insert_file),
    size_of_val(&open_archive),
    size_of_val(&read_tar_gz),
    size_of_val(&record_path),
    size_of_val(&validate_member_path),
    size_of_val(&PackageArchive::from_files),
];

/// Fully validated regular files from one bounded package archive.
#[derive(Debug)]
pub(super) struct PackageArchive {
    /// Normalized member paths and exact file contents.
    files: ArchiveFiles,
}

impl PackageArchive {
    /// Return one required regular file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file is absent.
    pub(super) fn file(&self, path: &str, label: &str) -> CheckResult<&[u8]> {
        return self
            .files
            .get(path)
            .map(Vec::as_slice)
            .ok_or_else(|| return format!("{label} archive is missing {path}"));
    }

    /// Return all normalized regular-file paths.
    pub(super) const fn files(&self) -> &ArchiveFiles {
        return &self.files;
    }

    /// Build an archive from fully validated regular files.
    pub(super) const fn from_files(files: ArchiveFiles) -> Self {
        return Self { files };
    }

    /// Require an exact regular-file set.
    ///
    /// # Errors
    ///
    /// Returns an error when a required file is absent or an extra file exists.
    pub(super) fn require_exact_files(&self, expected: &[String], label: &str) -> CheckResult {
        let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
        let actual_set = self.files.keys().cloned().collect::<BTreeSet<_>>();
        if actual_set == expected_set {
            return Ok(());
        }
        let missing = expected_set
            .difference(&actual_set)
            .cloned()
            .collect::<Vec<_>>();
        let unexpected = actual_set
            .difference(&expected_set)
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "{label} archive file set differs; missing: {}; unexpected: {}",
            missing.join(", "),
            unexpected.join(", ")
        ));
    }

    /// Require every file to live below one exact archive root.
    ///
    /// # Errors
    ///
    /// Returns an error when a file escapes or differs from the expected root.
    pub(super) fn require_root(&self, root: &str, label: &str) -> CheckResult {
        let prefix = format!("{root}/");
        return self
            .files
            .keys()
            .find(|path| return !path.starts_with(prefix.as_str()))
            .map_or(Ok(()), |path| {
                return Err(format!("{label} member {path} is outside {root}"));
            });
    }
}

/// Insert one validated, nonempty regular archive file.
///
/// # Errors
///
/// Returns an error when the file is empty, sensitive, or duplicated.
pub(super) fn insert_file(files: &mut ArchiveFiles, path: &str, contents: Vec<u8>) -> CheckResult {
    if contents.is_empty() {
        return Err(format!("package archive member {path} must not be empty"));
    }
    check_try!(reject_sensitive_content(path, contents.as_slice()));
    if files.insert(path.to_owned(), contents).is_some() {
        return Err(format!("package archive contains duplicate file {path}"));
    }
    return Ok(());
}

/// Open one bounded, regular archive file without following a symlink.
///
/// # Errors
///
/// Returns an error when the path is not a bounded regular file or cannot open.
pub(super) fn open_archive(path: &Path, label: &str) -> CheckResult<File> {
    let metadata = check_try!(
        symlink_metadata(path)
            .map_err(|error| return format!("stat {label} archive {}: {error}", path.display()))
    );
    if !metadata.is_file() || metadata.len() == u64::MIN || metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(format!(
            "{label} archive {} must be a nonempty regular file no larger than {MAX_ARCHIVE_BYTES} bytes",
            path.display()
        ));
    }
    return File::open(path)
        .map_err(|error| return format!("open {label} archive {}: {error}", path.display()));
}

/// Read one archive member within its declared and global limits.
///
/// # Errors
///
/// Returns an error when the member is empty, oversized, truncated, or grows.
pub(super) fn read_bounded<Reader>(
    reader: &mut Reader,
    declared_size: u64,
    total_size: &mut u64,
    path: &str,
) -> CheckResult<Vec<u8>>
where
    Reader: Read,
{
    if declared_size == u64::MIN || declared_size > MAX_ENTRY_BYTES {
        return Err(format!(
            "package archive member {path} must contain between 1 and {MAX_ENTRY_BYTES} bytes"
        ));
    }
    let next_total = check_try!(
        total_size
            .checked_add(declared_size)
            .ok_or_else(|| return "package archive unpacked size overflow".to_owned())
    );
    if next_total > MAX_TOTAL_BYTES {
        return Err(format!(
            "package archive exceeds the {MAX_TOTAL_BYTES}-byte unpacked limit"
        ));
    }
    let capacity = check_try!(
        usize::try_from(declared_size)
            .map_err(|error| return format!("measure package member {path}: {error}"))
    );
    let mut contents = Vec::with_capacity(capacity);
    let actual_size = check_try!(
        reader
            .take(check_try!(declared_size.checked_add(0x1).ok_or_else(
                || {
                    return format!("package member {path} size overflow");
                }
            )))
            .read_to_end(&mut contents)
            .map_err(|error| return format!("read package member {path}: {error}"))
    );
    if actual_size != capacity {
        return Err(format!(
            "package archive member {path} size changed: declared {declared_size}, read {actual_size}"
        ));
    }
    *total_size = next_total;
    return Ok(contents);
}

/// Read and validate a gzip-compressed tar package.
///
/// # Errors
///
/// Returns an error when the archive is malformed, unsafe, duplicated, or
/// exceeds a configured bound.
pub(super) fn read_tar_gz(path: &Path, label: &str) -> CheckResult<PackageArchive> {
    let file = check_try!(open_archive(path, label));
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = TarArchive::new(decoder);
    let entries = check_try!(
        archive
            .entries()
            .map_err(|error| return format!("read {label} archive {}: {error}", path.display()))
    );
    let (mut files, mut paths) = (ArchiveFiles::new(), BTreeSet::new());
    let mut total_size = u64::MIN;
    for entry_result in entries {
        let mut entry = check_try!(entry_result.map_err(|error| {
            return format!("read {label} archive {} entry: {error}", path.display());
        }));
        let entry_type = entry.header().entry_type();
        let kind = if entry_type.is_dir() {
            MemberKind::Directory
        } else {
            MemberKind::File
        };
        let member = check_try!(validate_member_path(entry.path_bytes().as_ref(), kind));
        check_try!(record_path(&mut paths, member.as_str()));
        if entry_type.is_dir() {
            continue;
        }
        if !entry_type.is_file() {
            return Err(format!(
                "{label} archive member {member} is not a regular file"
            ));
        }
        let declared_size = entry.size();
        let contents = check_try!(read_bounded(
            &mut entry,
            declared_size,
            &mut total_size,
            member.as_str(),
        ));
        check_try!(insert_file(&mut files, member.as_str(), contents));
    }
    return Ok(PackageArchive::from_files(files));
}

/// Record one unique archive member path.
///
/// # Errors
///
/// Returns an error when the archive has too many or duplicate members.
pub(super) fn record_path(paths: &mut BTreeSet<String>, path: &str) -> CheckResult {
    if paths.len() >= MAX_ENTRY_COUNT {
        return Err(format!(
            "package archive contains more than {MAX_ENTRY_COUNT} members"
        ));
    }
    return paths
        .insert(path.to_owned())
        .then_some(())
        .ok_or_else(|| return format!("package archive contains duplicate member {path}"));
}

/// Validate and normalize one UTF-8 relative archive path.
///
/// # Errors
///
/// Returns an error when the path is noncanonical, unsafe, or sensitive.
pub(super) fn validate_member_path(raw_path: &[u8], kind: MemberKind) -> CheckResult<String> {
    if raw_path.is_empty() || raw_path.len() > MAX_PATH_BYTES {
        return Err(format!(
            "package archive member path must contain between 1 and {MAX_PATH_BYTES} bytes"
        ));
    }
    let path = check_try!(
        from_utf8(raw_path)
            .map_err(|error| return format!("package archive member path is not UTF-8: {error}"))
    );
    let normalized = if kind == MemberKind::Directory {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    };
    if normalized.is_empty()
        || normalized.contains('\0')
        || normalized.contains(':')
        || normalized.contains('\\')
        || normalized.contains("//")
        || normalized.ends_with('/')
    {
        return Err(format!(
            "package archive member path {path:?} is not canonical"
        ));
    }
    for component in Path::new(normalized).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(format!("package archive member path {path:?} is unsafe"));
        }
    }
    check_try!(reject_sensitive_path(normalized));
    return Ok(normalized.to_owned());
}
