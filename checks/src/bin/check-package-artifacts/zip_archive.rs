//! Bounded wheel ZIP archive decoding and CRC verification.

use alloc::collections::{BTreeMap, BTreeSet};

use flate2::read::DeflateDecoder;

use std::{io::Read as _, path::Path};

use tovuk_public_checks::check_support::CheckResult;

use super::{
    MemberKind, ZipMember,
    archive::{
        MAX_ARCHIVE_BYTES, PackageArchive, insert_file, open_archive, read_bounded, record_path,
    },
    zip_directory::read_directory,
};

/// Bits processed per CRC-32 input byte.
const CRC_BIT_COUNT: usize = 0x8;

/// Single-bit CRC-32 shift.
const CRC_BIT_SHIFT: u32 = 0x1;

/// Deflate ZIP compression method.
const METHOD_DEFLATE: u16 = 0x0008;

/// Stored ZIP compression method.
const METHOD_STORED: u16 = 0x0000;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&crc32),
    size_of_val(&crc32_polynomial),
    size_of_val(&decode_deflated),
    size_of_val(&decode_member),
    size_of_val(&decode_stored),
    size_of_val(&read_archive_bytes),
    size_of_val(&read_zip),
];

/// Compute an IEEE CRC-32 for one unpacked ZIP member.
pub(super) fn crc32(contents: &[u8]) -> u32 {
    let mut checksum = u32::MAX;
    for byte in contents {
        checksum ^= u32::from(*byte);
        for _round in usize::MIN..CRC_BIT_COUNT {
            let polynomial = crc32_polynomial(checksum);
            checksum = (checksum >> CRC_BIT_SHIFT) ^ polynomial;
        }
    }
    return !checksum;
}

/// Return the reflected CRC-32 polynomial for the current low bit.
const fn crc32_polynomial(checksum: u32) -> u32 {
    if checksum & 0x1 == 0x1 {
        return 0xedb8_8320;
    }
    return u32::MIN;
}

/// Decode one raw deflate stream and require complete compressed consumption.
///
/// # Errors
///
/// Returns an error when decompression fails, sizes differ, or data trails.
fn decode_deflated(
    compressed: &[u8],
    member: &ZipMember,
    total_size: &mut u64,
) -> CheckResult<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(compressed);
    let unpacked = check_try!(read_bounded(
        &mut decoder,
        member.unpacked_size,
        total_size,
        member.path.as_str(),
    ));
    let compressed_size = check_try!(u64::try_from(compressed.len()).map_err(|error| {
        return format!("measure compressed ZIP member {}: {error}", member.path);
    }));
    if decoder.total_in() != compressed_size {
        return Err(format!(
            "ZIP member {} has trailing or incomplete deflate data",
            member.path
        ));
    }
    return Ok(unpacked);
}

/// Decode and CRC-check one regular ZIP member.
///
/// # Errors
///
/// Returns an error when member data is out of bounds, unsupported, or corrupt.
fn decode_member(source: &[u8], member: &ZipMember, total_size: &mut u64) -> CheckResult<Vec<u8>> {
    let compressed = check_try!(
        source
            .get(member.compressed_range.clone())
            .ok_or_else(|| return format!("ZIP member {} data is out of bounds", member.path))
    );
    let contents = match member.method {
        METHOD_DEFLATE => check_try!(decode_deflated(compressed, member, total_size)),
        METHOD_STORED => check_try!(decode_stored(compressed, member, total_size)),
        _ => {
            return Err(format!(
                "ZIP member {} uses unsupported compression",
                member.path
            ));
        }
    };
    if crc32(contents.as_slice()) != member.crc32 {
        return Err(format!("ZIP member {} CRC-32 mismatch", member.path));
    }
    return Ok(contents);
}

/// Decode one stored ZIP member with exact declared sizing.
///
/// # Errors
///
/// Returns an error when compressed and unpacked sizes differ.
fn decode_stored(
    compressed: &[u8],
    member: &ZipMember,
    total_size: &mut u64,
) -> CheckResult<Vec<u8>> {
    let actual_size = check_try!(u64::try_from(compressed.len()).map_err(|error| {
        return format!("measure stored ZIP member {}: {error}", member.path);
    }));
    if actual_size != member.unpacked_size {
        return Err(format!("stored ZIP member {} size differs", member.path));
    }
    let mut reader = compressed;
    return read_bounded(
        &mut reader,
        member.unpacked_size,
        total_size,
        member.path.as_str(),
    );
}

/// Read a bounded ZIP archive into memory without trusting a growing file.
///
/// # Errors
///
/// Returns an error when the archive cannot open, read, or remain bounded.
fn read_archive_bytes(path: &Path, label: &str) -> CheckResult<Vec<u8>> {
    let file = check_try!(open_archive(path, label));
    let limit = check_try!(MAX_ARCHIVE_BYTES.checked_add(0x1).ok_or_else(|| {
        return "ZIP archive limit overflow".to_owned();
    }));
    let maximum = check_try!(
        usize::try_from(MAX_ARCHIVE_BYTES)
            .map_err(|error| return format!("convert ZIP archive limit: {error}"))
    );
    let mut source = Vec::new();
    let read_size = check_try!(
        file.take(limit)
            .read_to_end(&mut source)
            .map_err(|error| return format!("read {label} ZIP {}: {error}", path.display()))
    );
    if read_size != source.len() {
        return Err(format!("{label} ZIP changed while it was read"));
    }
    if source.len() > maximum {
        return Err(format!("{label} ZIP grew beyond {MAX_ARCHIVE_BYTES} bytes"));
    }
    return Ok(source);
}

/// Read and validate a wheel ZIP package.
///
/// # Errors
///
/// Returns an error when the ZIP is malformed, unsafe, duplicated, corrupted,
/// encrypted, ZIP64, multi-disk, or exceeds a configured bound.
pub(super) fn read_zip(path: &Path, label: &str) -> CheckResult<PackageArchive> {
    let source = check_try!(read_archive_bytes(path, label));
    let members = check_try!(read_directory(source.as_slice()));
    let mut files = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut total_size = u64::MIN;
    for member in members {
        check_try!(record_path(&mut paths, member.path.as_str()));
        if member.kind == MemberKind::Directory {
            continue;
        }
        let contents = check_try!(decode_member(source.as_slice(), &member, &mut total_size));
        check_try!(insert_file(&mut files, member.path.as_str(), contents));
    }
    return Ok(PackageArchive::from_files(files));
}
