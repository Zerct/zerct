use crate::helpers::CheckResult;

#[derive(Clone, Copy, Debug)]
struct PublicCopyForbiddenTerm {
    value: &'static str,
    whole_word: bool,
}

pub(crate) const RETIRED_PUBLIC_NAME: &str = concat!("ze", "rct");
#[cfg(test)]
pub(crate) const RETIRED_PUBLIC_NAME_TITLE: &str = concat!("Ze", "rct");
pub(crate) const RETIRED_PUBLIC_ORG_SCOPE: &str = concat!("@", "ze", "rct");
#[cfg(test)]
pub(crate) const RETIRED_PUBLIC_MINTLIFY_SLUG: &str = concat!("ze", "rct", "-4cdab021");

pub(crate) fn reject_forbidden_public_copy_terms(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for term in public_copy_forbidden_terms() {
        if term.whole_word {
            if contains_ascii_word(lower.as_str(), term.value) {
                return Err(format!(
                    "{label} contains forbidden public positioning term: {}",
                    term.value
                ));
            }
            continue;
        }
        if lower.contains(term.value) {
            return Err(format!(
                "{label} contains forbidden public positioning term: {}",
                term.value
            ));
        }
    }
    Ok(())
}

fn public_copy_forbidden_terms() -> &'static [PublicCopyForbiddenTerm] {
    &[
        PublicCopyForbiddenTerm {
            value: "cloudflare",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "vercel",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "supabase",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "hetzner",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "serverless",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "edge",
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value: "cdn",
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value: "durable object",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "workers kv",
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value: "d1",
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value: "r2",
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value: "pages functions",
            whole_word: false,
        },
    ]
}

pub(crate) fn retired_public_names() -> &'static [&'static str] {
    &[RETIRED_PUBLIC_NAME, "xquik"]
}

fn contains_ascii_word(value: &str, forbidden_word: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| word.eq_ignore_ascii_case(forbidden_word))
}
