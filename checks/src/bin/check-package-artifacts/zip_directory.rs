//! ZIP central-directory and local-header validation.

use core::ops::Range;

use tovuk_public_checks::check_support::CheckResult;

use super::{
    CentralFields, CentralIdentity, EndFields, LocalFields, MemberKind, ZipMember,
    archive::{MAX_ENTRY_BYTES, MAX_ENTRY_COUNT, validate_member_path},
    zip_fields::{read_central_fields, read_end_fields, read_local_fields},
    zip_format::{bytes_at, checked_end, u32_at},
};

/// ZIP central-directory header length.
const CENTRAL_HEADER_BYTES: usize = 0x002e;

/// ZIP central-directory header signature.
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;

/// Creator-platform shift in the version-made-by field.
const CREATOR_PLATFORM_SHIFT: u32 = 0x8;

/// ZIP end-of-central-directory record length without a comment.
const END_RECORD_BYTES: usize = 0x0016;

/// ZIP end-of-central-directory record signature.
const END_SIGNATURE: u32 = 0x0605_4b50;

/// General-purpose flags accepted from ordinary UTF-8 wheel entries.
const FLAG_MASK: u16 = 0x0800;

/// ZIP local-file header signature.
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;

/// Deflate ZIP compression method.
const METHOD_DEFLATE: u16 = 0x0008;

/// Stored ZIP compression method.
const METHOD_STORED: u16 = 0x0000;

/// Unix directory file type bits.
const MODE_DIRECTORY: u32 = 0x4000;

/// Unix file type mask.
const MODE_FILE_TYPE: u32 = 0xf000;

/// Unix regular-file type bits.
const MODE_REGULAR: u32 = 0x8000;

/// External-attribute shift for Unix mode bits.
const MODE_SHIFT: u32 = 0x10;

/// Unix symbolic-link type bits.
const MODE_SYMLINK: u32 = 0xa000;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000b] = [
    size_of_val(&central_kind),
    size_of_val(&central_path),
    size_of_val(&parse_central_member),
    size_of_val(&parse_end_record),
    size_of_val(&parse_local_range),
    size_of_val(&read_directory),
    size_of_val(&require_central_fields),
    size_of_val(&require_end_fields),
    size_of_val(&require_local_coverage),
    size_of_val(&require_local_fields),
    size_of_val(&validate_mode),
];

/// Parsed central member name and next directory offset.
#[derive(Debug)]
struct CentralPath {
    /// Normalized member kind.
    kind: MemberKind,
    /// Raw central member name.
    name: Vec<u8>,
    /// Next central-directory offset.
    next_central: usize,
    /// Canonical member path.
    path: String,
}

/// Validated central-directory bounds and entry count.
#[derive(Debug)]
struct EndRecord {
    /// Byte offset immediately after the central directory.
    central_end: usize,
    /// Byte offset at the start of the central directory.
    central_start: usize,
    /// Number of central-directory entries.
    entry_count: usize,
}

/// Central values that a local header must match.
#[derive(Debug)]
struct LocalExpectation {
    /// Start of the central directory member.
    central_offset: usize,
    /// Declared compressed size.
    compressed_size: usize,
    /// Declared CRC-32.
    crc32: u32,
    /// General-purpose flags.
    flags: u16,
    /// Compression method.
    method: u16,
    /// Raw member name.
    name: Vec<u8>,
    /// Declared unpacked size.
    unpacked_size: u64,
}

/// Complete and compressed local ZIP ranges.
#[derive(Debug)]
struct LocalRanges {
    /// Complete local header, path, and data range.
    archive: Range<usize>,
    /// Compressed file-data range.
    compressed: Range<usize>,
}

/// Parsed central member and its following directory offset.
#[derive(Debug)]
struct ParsedMember {
    /// Validated member.
    member: ZipMember,
    /// Next central-directory offset.
    next_offset: usize,
}

/// Classify a canonical ZIP member name.
fn central_kind(name: &[u8]) -> MemberKind {
    if name.ends_with(b"/") {
        return MemberKind::Directory;
    }
    return MemberKind::File;
}

/// Read and normalize one central member name and variable-length tail.
///
/// # Errors
///
/// Returns an error when bounds, path safety, or Unix file type differ.
fn central_path(
    source: &[u8],
    central_offset: usize,
    central_end: usize,
    fields: &CentralFields,
) -> CheckResult<CentralPath> {
    let header_end = check_try!(checked_end(
        central_offset,
        CENTRAL_HEADER_BYTES,
        "central header"
    ));
    let name_end = check_try!(checked_end(
        header_end,
        fields.layout.name_length,
        "central name"
    ));
    let extra_end = check_try!(checked_end(
        name_end,
        fields.layout.extra_length,
        "central extra"
    ));
    let next_central = check_try!(checked_end(
        extra_end,
        fields.layout.comment_length,
        "central comment"
    ));
    if next_central > central_end {
        return Err("ZIP central member exceeds the central directory".to_owned());
    }
    let name = check_try!(bytes_at(
        source,
        header_end,
        fields.layout.name_length,
        "central name"
    ));
    let kind = central_kind(name);
    let path = check_try!(validate_member_path(name, kind));
    check_try!(validate_mode(&fields.identity, kind, path.as_str()));
    return Ok(CentralPath {
        kind,
        name: name.to_vec(),
        next_central,
        path,
    });
}

/// Parse one central member and its corresponding local data range.
///
/// # Errors
///
/// Returns an error when any central or local invariant fails.
fn parse_central_member(
    source: &[u8],
    central_offset: usize,
    central_end: usize,
) -> CheckResult<ParsedMember> {
    if check_try!(u32_at(source, central_offset, "central signature")) != CENTRAL_SIGNATURE {
        return Err(format!(
            "ZIP central directory has an invalid signature at {central_offset}"
        ));
    }
    let fields = check_try!(read_central_fields(source, central_offset));
    check_try!(require_central_fields(&fields));
    let central = check_try!(central_path(source, central_offset, central_end, &fields));
    let expected = LocalExpectation {
        central_offset,
        compressed_size: fields.layout.compressed_size,
        crc32: fields.identity.crc32,
        flags: fields.identity.flags,
        method: fields.identity.method,
        name: central.name,
        unpacked_size: fields.layout.unpacked_size,
    };
    let ranges = check_try!(parse_local_range(
        source,
        fields.layout.local_offset,
        &expected,
    ));
    return Ok(ParsedMember {
        member: ZipMember {
            archive_range: ranges.archive,
            compressed_range: ranges.compressed,
            crc32: fields.identity.crc32,
            kind: central.kind,
            method: fields.identity.method,
            path: central.path,
            unpacked_size: fields.layout.unpacked_size,
        },
        next_offset: central.next_central,
    });
}

/// Parse the single-disk, non-ZIP64 end record.
///
/// # Errors
///
/// Returns an error when the end record or central bounds are invalid.
fn parse_end_record(source: &[u8]) -> CheckResult<EndRecord> {
    let end_offset = check_try!(source.len().checked_sub(END_RECORD_BYTES).ok_or_else(|| {
        return "ZIP archive is shorter than its end record".to_owned();
    }));
    if check_try!(u32_at(source, end_offset, "end signature")) != END_SIGNATURE {
        return Err("ZIP archive must end with an uncommented central directory record".to_owned());
    }
    let fields = check_try!(read_end_fields(source, end_offset));
    check_try!(require_end_fields(&fields));
    let central_end = check_try!(checked_end(
        fields.central_start,
        fields.central_size,
        "central directory"
    ));
    if central_end != end_offset {
        return Err("ZIP central directory bounds do not meet the end record".to_owned());
    }
    return Ok(EndRecord {
        central_end,
        central_start: fields.central_start,
        entry_count: usize::from(fields.total_entries),
    });
}

/// Parse and validate one local-file data range.
///
/// # Errors
///
/// Returns an error when local headers differ or data overlaps the directory.
fn parse_local_range(
    source: &[u8],
    local_offset: usize,
    expected: &LocalExpectation,
) -> CheckResult<LocalRanges> {
    if check_try!(u32_at(source, local_offset, "local signature")) != LOCAL_SIGNATURE {
        return Err("ZIP local header has an invalid signature".to_owned());
    }
    let fields = check_try!(read_local_fields(source, local_offset));
    check_try!(require_local_fields(&fields, expected));
    let name = check_try!(bytes_at(
        source,
        fields.header_end,
        fields.name_length,
        "local name"
    ));
    if name != expected.name {
        return Err("ZIP local and central member names disagree".to_owned());
    }
    let name_end = check_try!(checked_end(
        fields.header_end,
        fields.name_length,
        "local name"
    ));
    let data_start = check_try!(checked_end(
        name_end,
        fields.extra_length,
        "local extra data"
    ));
    let data_end = check_try!(checked_end(
        data_start,
        expected.compressed_size,
        "compressed data"
    ));
    if data_end > expected.central_offset {
        return Err("ZIP local member overlaps the central directory".to_owned());
    }
    return Ok(LocalRanges {
        archive: local_offset..data_end,
        compressed: data_start..data_end,
    });
}

/// Parse every validated central-directory member.
///
/// # Errors
///
/// Returns an error when directory bounds or entry counts disagree.
pub(super) fn read_directory(source: &[u8]) -> CheckResult<Vec<ZipMember>> {
    let end_record = check_try!(parse_end_record(source));
    let mut central_offset = end_record.central_start;
    let mut members = Vec::with_capacity(end_record.entry_count);
    for _entry_index in 0..end_record.entry_count {
        let parsed = check_try!(parse_central_member(
            source,
            central_offset,
            end_record.central_end,
        ));
        members.push(parsed.member);
        central_offset = parsed.next_offset;
    }
    if central_offset != end_record.central_end {
        return Err("ZIP central directory entry count does not match its bounds".to_owned());
    }
    check_try!(require_local_coverage(
        members.as_mut_slice(),
        end_record.central_start,
    ));
    return Ok(members);
}

/// Require supported flags, methods, and unpacked member bounds.
///
/// # Errors
///
/// Returns an error when central member fields use unsupported features.
fn require_central_fields(fields: &CentralFields) -> CheckResult {
    if fields.identity.flags & !FLAG_MASK != u16::MIN {
        return Err("ZIP member uses unsupported or encrypted flags".to_owned());
    }
    if !matches!(fields.identity.method, METHOD_DEFLATE | METHOD_STORED) {
        return Err("ZIP member uses an unsupported compression method".to_owned());
    }
    if fields.layout.unpacked_size > MAX_ENTRY_BYTES {
        return Err("ZIP package member exceeds the unpacked size limit".to_owned());
    }
    if fields.layout.extra_length != usize::MIN || fields.layout.comment_length != usize::MIN {
        return Err("ZIP central extra data and comments must be empty".to_owned());
    }
    return Ok(());
}

/// Require a nonempty single-disk, uncommented central directory.
///
/// # Errors
///
/// Returns an error when end fields request unsupported ZIP features.
fn require_end_fields(fields: &EndFields) -> CheckResult {
    if fields.disk != u16::MIN
        || fields.central_disk != u16::MIN
        || fields.disk_entries != fields.total_entries
        || fields.comment_length != u16::MIN
    {
        return Err("ZIP archive must be single-disk and have no comment".to_owned());
    }
    let count = usize::from(fields.total_entries);
    if count == usize::MIN || count > MAX_ENTRY_COUNT {
        return Err(format!(
            "ZIP archive must contain between 1 and {MAX_ENTRY_COUNT} members"
        ));
    }
    return Ok(());
}

/// Require local records to cover exactly the prefix before the directory.
///
/// # Errors
///
/// Returns an error when local records overlap or leave unscanned bytes.
fn require_local_coverage(members: &mut [ZipMember], central_start: usize) -> CheckResult {
    members.sort_unstable_by_key(|member| return member.archive_range.start);
    let mut expected_start = usize::MIN;
    for member in members {
        if member.archive_range.start != expected_start {
            return Err("ZIP local records must form a canonical gapless prefix".to_owned());
        }
        expected_start = member.archive_range.end;
    }
    return (expected_start == central_start)
        .then_some(())
        .ok_or_else(|| return "ZIP local records must end at the central directory".to_owned());
}

/// Require local identity fields and an empty extra-data area.
///
/// # Errors
///
/// Returns an error when local and central fields differ or extras are present.
fn require_local_fields(fields: &LocalFields, expected: &LocalExpectation) -> CheckResult {
    if fields.flags != expected.flags || fields.method != expected.method {
        return Err("ZIP local and central headers disagree".to_owned());
    }
    if fields.crc32 != expected.crc32
        || fields.compressed_size != expected.compressed_size
        || fields.unpacked_size != expected.unpacked_size
    {
        return Err("ZIP local and central sizes or CRC-32 disagree".to_owned());
    }
    return (fields.extra_length == usize::MIN)
        .then_some(())
        .ok_or_else(|| return "ZIP local extra data must be empty".to_owned());
}

/// Reject Unix symlinks and file-type mismatches from external attributes.
///
/// # Errors
///
/// Returns an error when attributes name a symlink or contradict the path.
fn validate_mode(identity: &CentralIdentity, kind: MemberKind, path: &str) -> CheckResult {
    if identity.made_by >> CREATOR_PLATFORM_SHIFT != 0x3 {
        return Ok(());
    }
    let file_type = (identity.external >> MODE_SHIFT) & MODE_FILE_TYPE;
    if file_type == MODE_SYMLINK {
        return Err(format!("ZIP member {path} must not be a symlink"));
    }
    let expected = match kind {
        MemberKind::Directory => MODE_DIRECTORY,
        MemberKind::File => MODE_REGULAR,
    };
    let valid = file_type == u32::MIN || file_type == expected;
    return valid
        .then_some(())
        .ok_or_else(|| return format!("ZIP member {path} file type disagrees with its name"));
}
