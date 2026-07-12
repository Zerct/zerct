use crate::helpers::CheckResult;

use core::str::from_utf8;

use sha2::{Digest as _, Sha256};

/// Maximum byte size of one tracked public text file.
pub(super) const MAX_TRACKED_TEXT_BYTES: u64 = 0x0008_0000;

/// Non-reversible fingerprints of private implementation terms forbidden in
/// every public path, file, and Git object.
const PRIVATE_IMPLEMENTATION_TERM_FINGERPRINTS: &[PrivateTermFingerprint; 0x0004] = &[
    PrivateTermFingerprint {
        digest: [
            0xbc, 0x00, 0xb5, 0x12, 0xce, 0xf8, 0x8d, 0x40, 0x59, 0xf4, 0xad, 0xe3, 0x6b, 0x42,
            0x6a, 0x22, 0x6a, 0x67, 0x9a, 0x72, 0xdb, 0x60, 0xfc, 0x8c, 0x83, 0xa0, 0xa2, 0xc5,
            0x79, 0xe5, 0x49, 0x48,
        ],
        length: 0x0008,
    },
    PrivateTermFingerprint {
        digest: [
            0x29, 0x4a, 0xa8, 0xd7, 0x54, 0x83, 0xb8, 0x33, 0x1e, 0x3b, 0xa6, 0xa7, 0xf2, 0x4a,
            0xea, 0x15, 0x20, 0x27, 0x47, 0xf3, 0x6d, 0xe6, 0x51, 0x97, 0xe7, 0xbc, 0x61, 0x94,
            0x88, 0x0b, 0x25, 0x58,
        ],
        length: 0x0007,
    },
    PrivateTermFingerprint {
        digest: [
            0x78, 0x27, 0xe8, 0xeb, 0x2c, 0xb3, 0xb9, 0x5f, 0x4d, 0x0f, 0xfa, 0xc3, 0x24, 0xb6,
            0x52, 0x08, 0xff, 0x81, 0x3b, 0x66, 0x88, 0x4d, 0x08, 0x32, 0x7a, 0xb0, 0xab, 0xb0,
            0x4f, 0xe7, 0x80, 0xfb,
        ],
        length: 0x000a,
    },
    PrivateTermFingerprint {
        digest: [
            0xe6, 0x84, 0xa3, 0xe9, 0x4e, 0xfd, 0x46, 0xf9, 0xc5, 0xfa, 0x0e, 0xa8, 0x3c, 0x60,
            0x0c, 0xba, 0xd6, 0xfb, 0x47, 0xdc, 0x9c, 0xf2, 0x62, 0x0a, 0x62, 0xf8, 0x3f, 0xbd,
            0x35, 0x52, 0xcb, 0xbe,
        ],
        length: 0x0008,
    },
];

/// One private-term digest and its exact normalized byte length.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrivateTermFingerprint {
    /// SHA-256 digest of the normalized private term.
    digest: [u8; 0x0020],
    /// Exact ASCII byte length hashed by this fingerprint.
    length: usize,
}

impl PrivateTermFingerprint {
    /// Return whether one case-folded candidate has this fingerprint.
    fn matches(self, candidate: &[u8]) -> bool {
        let mut digest = Sha256::new();
        for byte in candidate {
            digest.update([byte.to_ascii_lowercase()]);
        }
        let actual: [u8; 0x0020] = digest.finalize().into();
        return actual == self.digest;
    }
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&line_contains_private_repository_marker),
    size_of_val(&line_contains_retired_npm_runner_guidance),
    size_of_val(&line_has_ascii_path_pair),
    size_of_val(&line_has_ascii_word_pair),
    size_of_val(&line_has_local_root),
    size_of_val(&reject_private_implementation_terms),
    size_of_val(&validate_tracked_text),
];

/// Return whether a line exposes a developer-local or private-engine path.
pub(super) fn line_contains_private_repository_marker(line: &str) -> bool {
    let components = line
        .split(['/', '\\'])
        .filter(|component| return !component.is_empty())
        .collect::<Vec<_>>();
    let has_local_home = ["developer", "home", "users"]
        .iter()
        .any(|local| return line_has_local_root(line, local));
    return has_local_home
        || line_has_ascii_path_pair(components.as_slice(), "tovuk", "engine")
        || line_has_ascii_path_pair(components.as_slice(), "engine", "apps")
        || line_has_ascii_path_pair(components.as_slice(), "engine", "crates");
}

/// Contract implementation for `line_contains_retired_npm_runner_guidance`.
pub(super) fn line_contains_retired_npm_runner_guidance(line: &str) -> bool {
    let runner = "npx";
    let command = "tovuk";
    return line_has_ascii_word_pair(line, runner, command);
}

/// Return whether adjacent path components match a case-insensitive pair.
fn line_has_ascii_path_pair(components: &[&str], first: &str, second: &str) -> bool {
    return components.windows(0x0002).any(|pair| {
        let Some(left) = pair.first() else {
            return false;
        };
        let Some(right) = pair.get(0x0001) else {
            return false;
        };
        return left.eq_ignore_ascii_case(first) && right.eq_ignore_ascii_case(second);
    });
}

/// Contract implementation for `line_has_ascii_word_pair`.
fn line_has_ascii_word_pair(line: &str, first: &str, second: &str) -> bool {
    let words = line
        .split(|character: char| return !character.is_ascii_alphanumeric())
        .filter(|word| return !word.is_empty())
        .collect::<Vec<_>>();
    return words
        .windows(0x0002)
        .any(|pair| return pair.first() == Some(&first) && pair.get(0x0001) == Some(&second));
}

/// Return whether a line contains an absolute developer-home component.
fn line_has_local_root(line: &str, root: &str) -> bool {
    for (index, character) in line.char_indices() {
        if !matches!(character, '/' | '\\') {
            continue;
        }
        let prefix = line.get(..index).unwrap_or_default();
        let at_boundary = prefix.chars().next_back().is_none_or(|previous| {
            return previous.is_ascii_whitespace()
                || matches!(previous, '"' | '\'' | '(' | ':' | '=' | '`');
        });
        if !at_boundary {
            continue;
        }
        let remainder = line
            .get(index.saturating_add(character.len_utf8())..)
            .unwrap_or_default();
        let component = remainder.split(['/', '\\']).next().unwrap_or_default();
        if component.eq_ignore_ascii_case(root) {
            return true;
        }
    }
    return false;
}

/// Reject private provider or browser-automation implementation terms in
/// public bytes without storing those terms in source or diagnostics.
///
/// # Errors
///
/// Returns an error when input contains a fingerprinted private term.
pub(super) fn reject_private_implementation_terms(label: &str, contents: &[u8]) -> CheckResult {
    for fingerprint in PRIVATE_IMPLEMENTATION_TERM_FINGERPRINTS {
        let mut candidates = contents
            .split(|byte| return !byte.is_ascii_alphabetic())
            .filter(|word| return !word.is_empty())
            .flat_map(|word| return word.windows(fingerprint.length));
        if candidates.any(|candidate| return fingerprint.matches(candidate)) {
            return Err(format!(
                "{label} contains a forbidden private implementation term"
            ));
        }
    }
    return Ok(());
}

/// Require canonical UTF-8 and line endings for one tracked public text file.
///
/// # Errors
///
/// Returns an error for oversized, binary, non-UTF-8, CRLF, unterminated, or
/// trailing-whitespace content.
pub(super) fn validate_tracked_text(path: &str, contents: &[u8]) -> CheckResult {
    let byte_count = check_try!(
        u64::try_from(contents.len())
            .map_err(|error| return format!("measure tracked file {path}: {error}"))
    );
    if byte_count > MAX_TRACKED_TEXT_BYTES {
        return Err(format!(
            "{path} exceeds the {MAX_TRACKED_TEXT_BYTES}-byte tracked-file ceiling"
        ));
    }
    if contents.contains(&0x00) {
        return Err(format!("{path} contains a NUL byte"));
    }
    let source = check_try!(
        from_utf8(contents).map_err(|error| return format!("{path} is not UTF-8: {error}"))
    );
    if source.contains('\r') {
        return Err(format!(
            "{path} contains a carriage return; tracked text must use LF"
        ));
    }
    if !contents.is_empty() && !contents.ends_with(b"\n") {
        return Err(format!("{path} does not end with LF"));
    }
    if source.lines().any(|line| {
        return line
            .as_bytes()
            .last()
            .is_some_and(|byte| return matches!(byte, b' ' | b'\t'));
    }) {
        return Err(format!("{path} contains trailing whitespace"));
    }
    return Ok(());
}

#[cfg(test)]
#[path = "repo_hygiene_text_tests/verification.rs"]
mod tests;
