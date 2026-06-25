use crate::helpers::CheckResult;

#[derive(Clone, Copy, Debug)]
struct PublicCopyForbiddenTerm {
    value_bytes: &'static [u8],
    whole_word: bool,
}

pub(crate) fn reject_forbidden_public_copy_terms(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for term in public_copy_forbidden_terms() {
        let value = ascii_term(term.value_bytes);
        if term.whole_word {
            if contains_ascii_word(lower.as_str(), &[value.as_str()]) {
                return Err(format!(
                    "{label} contains forbidden public positioning term: {value}"
                ));
            }
            continue;
        }
        if lower.contains(value.as_str()) {
            return Err(format!(
                "{label} contains forbidden public positioning term: {value}"
            ));
        }
    }
    Ok(())
}

fn public_copy_forbidden_terms() -> Vec<PublicCopyForbiddenTerm> {
    vec![
        PublicCopyForbiddenTerm {
            value_bytes: &[99, 108, 111, 117, 100, 102, 108, 97, 114, 101],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[118, 101, 114, 99, 101, 108],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[115, 117, 112, 97, 98, 97, 115, 101],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[104, 101, 116, 122, 110, 101, 114],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[115, 101, 114, 118, 101, 114, 108, 101, 115, 115],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[101, 100, 103, 101],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[99, 100, 110],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[
                100, 117, 114, 97, 98, 108, 101, 32, 111, 98, 106, 101, 99, 116,
            ],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[119, 111, 114, 107, 101, 114, 115, 32, 107, 118],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[100, 49],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[114, 50],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[
                112, 97, 103, 101, 115, 32, 102, 117, 110, 99, 116, 105, 111, 110, 115,
            ],
            whole_word: false,
        },
    ]
}

pub(crate) fn retired_public_names() -> Vec<String> {
    vec![
        ascii_term(&[122, 101, 114, 99, 116]),
        ascii_term(&[120, 113, 117, 105, 107]),
    ]
}

pub(crate) fn ascii_term(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn contains_ascii_word(value: &str, forbidden_words: &[&str]) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| {
            forbidden_words
                .iter()
                .any(|forbidden| word.eq_ignore_ascii_case(forbidden))
        })
}
