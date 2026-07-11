/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&line_contains_retired_npm_runner_guidance),
    size_of_val(&line_has_ascii_word_pair),
];

/// Contract implementation for `line_contains_retired_npm_runner_guidance`.
pub(super) fn line_contains_retired_npm_runner_guidance(line: &str) -> bool {
    let runner = "npx";
    let command = "tovuk";
    return line_has_ascii_word_pair(line, runner, command);
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
