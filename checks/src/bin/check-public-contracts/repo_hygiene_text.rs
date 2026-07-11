/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&line_contains_private_repository_marker),
    size_of_val(&line_contains_retired_npm_runner_guidance),
    size_of_val(&line_has_ascii_path_pair),
    size_of_val(&line_has_ascii_word_pair),
    size_of_val(&line_has_local_root),
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
