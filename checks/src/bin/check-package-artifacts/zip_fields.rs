//! Checked ZIP central and local fixed-field readers.

use tovuk_public_checks::check_support::CheckResult;

use super::{
    CentralFields, CentralIdentity, CentralLayout, EndFields, LocalFields,
    zip_format::{checked_end, u16_at, u32_at},
};

/// ZIP local-file header length.
const LOCAL_HEADER_BYTES: usize = 0x001e;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000a] = [
    size_of_val(&checked_usize),
    size_of_val(&read_central_fields),
    size_of_val(&read_central_identity),
    size_of_val(&read_central_layout),
    size_of_val(&read_end_fields),
    size_of_val(&read_local_fields),
    size_of_val(&u16_field),
    size_of_val(&u32_field),
    size_of_val(&usize_field),
    size_of_val(&LOCAL_HEADER_BYTES),
];

/// Convert one non-ZIP64 `u32` field to the host index type.
///
/// # Errors
///
/// Returns an error when conversion fails or the ZIP64 sentinel is present.
fn checked_usize(value: u32, label: &str) -> CheckResult<usize> {
    if value == u32::MAX {
        return Err(format!("ZIP64 {label} is not supported"));
    }
    return usize::try_from(value).map_err(|error| return format!("convert ZIP {label}: {error}"));
}

/// Read all fixed central-directory fields.
///
/// # Errors
///
/// Returns an error when a field is out of bounds or ZIP64-sized.
pub(super) fn read_central_fields(source: &[u8], offset: usize) -> CheckResult<CentralFields> {
    return Ok(CentralFields {
        identity: check_try!(read_central_identity(source, offset)),
        layout: check_try!(read_central_layout(source, offset)),
    });
}

/// Read central identity and file-attribute fields.
///
/// # Errors
///
/// Returns an error when a field exceeds the input.
fn read_central_identity(source: &[u8], offset: usize) -> CheckResult<CentralIdentity> {
    return Ok(CentralIdentity {
        crc32: check_try!(u32_field(source, offset, 0x10, "CRC")),
        external: check_try!(u32_field(source, offset, 0x26, "attributes")),
        flags: check_try!(u16_field(source, offset, 0x8, "flags")),
        made_by: check_try!(u16_field(source, offset, 0x4, "creator")),
        method: check_try!(u16_field(source, offset, 0x0a, "method")),
    });
}

/// Read central variable-length layout fields.
///
/// # Errors
///
/// Returns an error when a field exceeds the input or is ZIP64-sized.
fn read_central_layout(source: &[u8], offset: usize) -> CheckResult<CentralLayout> {
    return Ok(CentralLayout {
        comment_length: usize::from(check_try!(u16_field(
            source,
            offset,
            0x20,
            "comment length"
        ))),
        compressed_size: check_try!(usize_field(source, offset, 0x14, "compressed size")),
        extra_length: usize::from(check_try!(u16_field(source, offset, 0x1e, "extra length"))),
        local_offset: check_try!(usize_field(source, offset, 0x2a, "local offset")),
        name_length: usize::from(check_try!(u16_field(source, offset, 0x1c, "name length"))),
        unpacked_size: u64::from(check_try!(u32_field(source, offset, 0x18, "unpacked size"))),
    });
}

/// Read all fixed end-of-central-directory fields.
///
/// # Errors
///
/// Returns an error when a field is out of bounds or ZIP64-sized.
pub(super) fn read_end_fields(source: &[u8], offset: usize) -> CheckResult<EndFields> {
    return Ok(EndFields {
        central_disk: check_try!(u16_field(source, offset, 0x6, "central disk")),
        central_size: check_try!(usize_field(source, offset, 0x0c, "central size")),
        central_start: check_try!(usize_field(source, offset, 0x10, "central offset")),
        comment_length: check_try!(u16_field(source, offset, 0x14, "comment length")),
        disk: check_try!(u16_field(source, offset, 0x4, "disk")),
        disk_entries: check_try!(u16_field(source, offset, 0x8, "disk entries")),
        total_entries: check_try!(u16_field(source, offset, 0x0a, "total entries")),
    });
}

/// Read fixed local-header identity and layout fields.
///
/// # Errors
///
/// Returns an error when a field exceeds the input.
pub(super) fn read_local_fields(source: &[u8], offset: usize) -> CheckResult<LocalFields> {
    return Ok(LocalFields {
        compressed_size: check_try!(usize_field(source, offset, 0x12, "local compressed size")),
        crc32: check_try!(u32_field(source, offset, 0x0e, "local CRC")),
        extra_length: usize::from(check_try!(u16_field(
            source,
            offset,
            0x1c,
            "local extra length"
        ))),
        flags: check_try!(u16_field(source, offset, 0x6, "local flags")),
        header_end: check_try!(checked_end(offset, LOCAL_HEADER_BYTES, "local header")),
        method: check_try!(u16_field(source, offset, 0x8, "local method")),
        name_length: usize::from(check_try!(u16_field(
            source,
            offset,
            0x1a,
            "local name length"
        ))),
        unpacked_size: u64::from(check_try!(u32_field(
            source,
            offset,
            0x16,
            "local unpacked size"
        ))),
    });
}

/// Read a checked little-endian `u16` field relative to a header.
///
/// # Errors
///
/// Returns an error when offset arithmetic or field reading fails.
fn u16_field(source: &[u8], header: usize, delta: usize, label: &str) -> CheckResult<u16> {
    let offset = check_try!(checked_end(header, delta, label));
    return u16_at(source, offset, label);
}

/// Read a checked little-endian `u32` field relative to a header.
///
/// # Errors
///
/// Returns an error when offset arithmetic or field reading fails.
fn u32_field(source: &[u8], header: usize, delta: usize, label: &str) -> CheckResult<u32> {
    let offset = check_try!(checked_end(header, delta, label));
    return u32_at(source, offset, label);
}

/// Read and convert a non-ZIP64 `u32` field relative to a header.
///
/// # Errors
///
/// Returns an error when reading, conversion, or ZIP64 validation fails.
fn usize_field(source: &[u8], header: usize, delta: usize, label: &str) -> CheckResult<usize> {
    let value = check_try!(u32_field(source, header, delta, label));
    return checked_usize(value, label);
}
