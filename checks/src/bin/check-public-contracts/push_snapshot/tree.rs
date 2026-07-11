//! Bounded parser for immutable raw Git tree objects.

use crate::helpers::CheckResult;

use core::{
    cmp::{Ordering, max},
    str::from_utf8,
};

use std::path::Path;

use super::{ObjectKind, git};

/// Bit shift from one byte to its high hexadecimal nibble.
const NIBBLE_SHIFT: u32 = 0x0004;

/// One parsed entry and its exact consumed byte count.
#[derive(Debug, Eq, PartialEq)]
struct ParsedEntry {
    /// Number of raw bytes consumed by this entry.
    consumed: usize,
    /// Structurally decoded entry.
    entry: RawTreeEntry,
}

/// One structurally decoded raw Git tree entry.
#[derive(Debug, Eq, PartialEq)]
struct RawTreeEntry {
    /// Expected referenced object kind derived from the exact raw mode.
    kind: ObjectKind,
    /// Canonical UTF-8 basename.
    name: String,
    /// Lowercase hexadecimal referenced object identifier.
    object: String,
}

/// One decoded raw tree header and its binary object offset.
#[derive(Debug, Eq, PartialEq)]
struct RawTreeHeader {
    /// Expected referenced object kind derived from the mode.
    kind: ObjectKind,
    /// Canonical UTF-8 basename.
    name: String,
    /// Byte offset at which the binary object identifier begins.
    object_start: usize,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000c] = [
    size_of_val(&compare_entries),
    size_of_val(&comparison_byte),
    size_of_val(&decode_object),
    size_of_val(&object_byte_width),
    size_of_val(&parse_entry),
    size_of_val(&parse_header),
    size_of_val(&parse_mode),
    size_of_val(&parse_raw_tree),
    size_of_val(&validate_entry_kind),
    size_of_val(&validate_entry_order),
    size_of_val(&validate_name),
    size_of_val(&validate_raw_tree),
];

/// Compare raw names using Git's virtual slash for directory entries.
fn compare_entries(left: &RawTreeEntry, right: &RawTreeEntry) -> Ordering {
    let maximum = max(left.name.len(), right.name.len());
    for index in 0..=maximum {
        let ordering = comparison_byte(left, index).cmp(&comparison_byte(right, index));
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    return Ordering::Equal;
}

/// Return one name byte or Git's kind-specific virtual terminator.
fn comparison_byte(entry: &RawTreeEntry, index: usize) -> u8 {
    return entry
        .name
        .as_bytes()
        .get(index)
        .copied()
        .unwrap_or_else(|| {
            return if entry.kind == ObjectKind::Tree {
                b'/'
            } else {
                0x00
            };
        });
}

/// Encode one raw object identifier as canonical lowercase hexadecimal.
fn decode_object(bytes: &[u8]) -> String {
    const HEX: &[u8; 0x0010] = b"0123456789abcdef";
    let mut object = String::with_capacity(bytes.len().saturating_mul(0x0002));
    for byte in bytes {
        let high = HEX
            .get(usize::from(*byte >> NIBBLE_SHIFT))
            .copied()
            .unwrap_or(b'0');
        let low = HEX
            .get(usize::from(*byte & 0x000f))
            .copied()
            .unwrap_or(b'0');
        object.push(char::from(high));
        object.push(char::from(low));
    }
    return object;
}

/// Resolve the repository object format to its raw binary identifier width.
///
/// # Errors
///
/// Returns an error when Git reports an unsupported hexadecimal width.
fn object_byte_width(repository: &Path) -> CheckResult<usize> {
    return match check_try!(git::object_id_length(repository)) {
        0x0028 => Ok(0x0014),
        0x0040 => Ok(0x0020),
        width => Err(format!("unsupported Git hexadecimal object width {width}")),
    };
}

/// Parse one raw mode, basename, NUL, and binary object identifier.
///
/// # Errors
///
/// Returns an error when an entry is truncated or structurally noncanonical.
fn parse_entry(contents: &[u8], object_width: usize) -> CheckResult<ParsedEntry> {
    let header = check_try!(parse_header(contents));
    let after_name = contents.get(header.object_start..).unwrap_or_default();
    let object = check_try!(after_name.get(..object_width).ok_or_else(|| return format!(
        "raw Git tree entry {:?} has a truncated object ID",
        header.name
    )));
    let consumed = header.object_start.saturating_add(object_width);
    return Ok(ParsedEntry {
        consumed,
        entry: RawTreeEntry {
            kind: header.kind,
            name: header.name,
            object: decode_object(object),
        },
    });
}

/// Parse one raw mode and NUL-terminated basename header.
///
/// # Errors
///
/// Returns an error when mode or name separators are missing.
fn parse_header(contents: &[u8]) -> CheckResult<RawTreeHeader> {
    let mode_end = check_try!(
        contents
            .iter()
            .position(|byte| return *byte == b' ')
            .ok_or_else(|| return "raw Git tree entry lacks a mode separator".to_owned())
    );
    let mode = contents.get(..mode_end).unwrap_or_default();
    let after_mode = contents
        .get(mode_end.saturating_add(0x0001)..)
        .unwrap_or_default();
    let name_end = check_try!(
        after_mode
            .iter()
            .position(|byte| return *byte == 0x00)
            .ok_or_else(|| return "raw Git tree entry lacks a name terminator".to_owned())
    );
    let name = check_try!(validate_name(
        after_mode.get(..name_end).unwrap_or_default()
    ));
    let object_start = mode_end
        .saturating_add(0x0001)
        .saturating_add(name_end)
        .saturating_add(0x0001);
    let kind = check_try!(parse_mode(mode, name.as_str()));
    return Ok(RawTreeHeader {
        kind,
        name,
        object_start,
    });
}

/// Convert one exact canonical raw mode into its expected object kind.
///
/// # Errors
///
/// Returns an error when the mode is padded, abbreviated, or unsupported.
fn parse_mode(mode: &[u8], name: &str) -> CheckResult<ObjectKind> {
    return match mode {
        b"100644" | b"100755" => Ok(ObjectKind::Blob),
        b"40000" => Ok(ObjectKind::Tree),
        other => Err(format!(
            "raw Git tree entry {name:?} has invalid mode {other:?}"
        )),
    };
}

/// Parse a complete bounded raw tree and enforce Git's canonical ordering.
///
/// # Errors
///
/// Returns an error when any raw entry or ordering relation is invalid.
fn parse_raw_tree(contents: &[u8], object_width: usize) -> CheckResult<Vec<RawTreeEntry>> {
    if contents.is_empty() {
        return Err("raw Git tree must not be empty".to_owned());
    }
    let mut entries: Vec<RawTreeEntry> = Vec::new();
    let mut remainder = contents;
    while !remainder.is_empty() {
        let parsed = check_try!(parse_entry(remainder, object_width));
        if let Some(previous) = entries.last() {
            check_try!(validate_entry_order(previous, &parsed.entry));
        }
        remainder = remainder.get(parsed.consumed..).unwrap_or_default();
        entries.push(parsed.entry);
    }
    return Ok(entries);
}

/// Require the raw mode's referenced object kind to match the object database.
///
/// # Errors
///
/// Returns an error when an object is missing or its kind contradicts the mode.
fn validate_entry_kind(repository: &Path, tree: &str, entry: &RawTreeEntry) -> CheckResult {
    let actual = check_try!(git::object_kind(repository, entry.object.as_str()));
    if actual != entry.kind {
        return Err(format!(
            "raw Git tree {tree} entry {:?} mode expects {:?}, found {actual:?}",
            entry.name, entry.kind
        ));
    }
    return Ok(());
}

/// Require adjacent entries to have unique names in Git's canonical order.
///
/// # Errors
///
/// Returns an error for a duplicate or out-of-order raw basename.
fn validate_entry_order(previous: &RawTreeEntry, entry: &RawTreeEntry) -> CheckResult {
    if previous.name == entry.name {
        return Err(format!("raw Git tree repeats name {:?}", entry.name));
    }
    if compare_entries(previous, entry) != Ordering::Less {
        return Err(format!(
            "raw Git tree name {:?} is out of order",
            entry.name
        ));
    }
    return Ok(());
}

/// Decode and validate one canonical public Git basename.
///
/// # Errors
///
/// Returns an error for empty, special, non-UTF-8, or unsafe names.
fn validate_name(bytes: &[u8]) -> CheckResult<String> {
    let name = check_try!(
        from_utf8(bytes).map_err(|error| return format!("raw Git tree name is not UTF-8: {error}"))
    );
    let unsafe_byte = name
        .bytes()
        .any(|byte| return byte.is_ascii_control() || matches!(byte, b'/' | b'\\'));
    if name.is_empty() || matches!(name, "." | "..") || unsafe_byte {
        return Err(format!("raw Git tree contains unsafe name {name:?}"));
    }
    return Ok(name.to_owned());
}

/// Validate one raw tree's structure, order, modes, names, widths, and object kinds.
///
/// # Errors
///
/// Returns an error when the bounded raw tree violates canonical Git structure.
pub(super) fn validate_raw_tree(repository: &Path, tree: &str, contents: &[u8]) -> CheckResult {
    let width = check_try!(object_byte_width(repository));
    let entries = check_try!(parse_raw_tree(contents, width));
    for entry in &entries {
        check_try!(validate_entry_kind(repository, tree, entry));
    }
    return Ok(());
}
