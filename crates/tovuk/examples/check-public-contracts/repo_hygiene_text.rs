pub(crate) fn line_contains_retired_npm_runner_guidance(line: &str) -> bool {
    let runner = "npx";
    let command = "tovuk";
    line_has_ascii_word_pair(line, runner, command)
}

pub(crate) fn line_contains_forbidden_go_toolchain(line: &str) -> bool {
    const FORBIDDEN_PATTERNS: &[&str] = &[
        "actions/setup-go",
        "go.dev/dl",
        "/opt/tovuk/go/bin",
        "/opt/tovuk/go-tools",
        "setup-go",
    ];

    if line_has_ascii_word_pair(line, "go", "install") {
        return true;
    }
    FORBIDDEN_PATTERNS
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn line_has_ascii_word_pair(line: &str, first: &str, second: &str) -> bool {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words
        .windows(2)
        .any(|pair| pair.first() == Some(&first) && pair.get(1) == Some(&second))
}
