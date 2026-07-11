//! Checked little-endian ZIP field parsing.

use tovuk_public_checks::check_support::CheckResult;

/// Fourth little-endian byte shift.
const FOURTH_BYTE_SHIFT: u32 = 0x18;

/// Second little-endian byte shift.
const SECOND_BYTE_SHIFT: u32 = 0x8;

/// Third little-endian byte shift.
const THIRD_BYTE_SHIFT: u32 = 0x10;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&bytes_at),
    size_of_val(&checked_end),
    size_of_val(&u16_at),
    size_of_val(&u32_at),
];

/// Borrow an exact byte range with checked offset arithmetic.
///
/// # Errors
///
/// Returns an error when the range overflows or exceeds the input.
pub(super) fn bytes_at<'source>(
    source: &'source [u8],
    start: usize,
    length: usize,
    label: &str,
) -> CheckResult<&'source [u8]> {
    let end = check_try!(checked_end(start, length, label));
    return source
        .get(start..end)
        .ok_or_else(|| return format!("ZIP {label} exceeds the archive bounds"));
}

/// Add a byte length to an offset without overflow.
///
/// # Errors
///
/// Returns an error when the offset calculation overflows.
pub(super) fn checked_end(start: usize, length: usize, label: &str) -> CheckResult<usize> {
    return start
        .checked_add(length)
        .ok_or_else(|| return format!("ZIP {label} offset overflow"));
}

/// Read one little-endian `u16` from a checked ZIP offset.
///
/// # Errors
///
/// Returns an error when the field exceeds the input.
pub(super) fn u16_at(source: &[u8], offset: usize, label: &str) -> CheckResult<u16> {
    let bytes = check_try!(bytes_at(source, offset, 0x2, label));
    let array = check_try!(
        <[u8; 0x2]>::try_from(bytes).map_err(|error| return format!("read ZIP {label}: {error}"))
    );
    let [first, second] = array;
    return Ok(u16::from(first) | (u16::from(second) << SECOND_BYTE_SHIFT));
}

/// Read one little-endian `u32` from a checked ZIP offset.
///
/// # Errors
///
/// Returns an error when the field exceeds the input.
pub(super) fn u32_at(source: &[u8], offset: usize, label: &str) -> CheckResult<u32> {
    let bytes = check_try!(bytes_at(source, offset, 0x4, label));
    let array = check_try!(
        <[u8; 0x4]>::try_from(bytes).map_err(|error| return format!("read ZIP {label}: {error}"))
    );
    let [first, second, third, fourth] = array;
    return Ok(u32::from(first)
        | (u32::from(second) << SECOND_BYTE_SHIFT)
        | (u32::from(third) << THIRD_BYTE_SHIFT)
        | (u32::from(fourth) << FOURTH_BYTE_SHIFT));
}
