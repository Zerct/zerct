use crate::helpers::CheckResult;

use core::str::from_utf8;

/// Maximum byte size of one tracked public text file.
pub(super) const MAX_TRACKED_TEXT_BYTES: u64 = 0x0008_0000;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&line_contains_private_repository_marker),
    size_of_val(&line_contains_retired_npm_runner_guidance),
    size_of_val(&line_has_ascii_path_pair),
    size_of_val(&line_has_ascii_word_pair),
    size_of_val(&line_has_local_root),
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
