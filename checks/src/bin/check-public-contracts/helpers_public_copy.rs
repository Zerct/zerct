use crate::helpers::CheckResult;

/// Forbidden implementation and infrastructure positioning terms.
const PUBLIC_COPY_FORBIDDEN_TERMS: &[PublicCopyForbiddenTerm] = &[
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "cloudflare",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "vercel",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "supabase",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "hosting provider",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "serverless",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::WholeWord,
        value: "edge",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::WholeWord,
        value: "cdn",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "durable object",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "workers kv",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::WholeWord,
        value: "d1",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::WholeWord,
        value: "r2",
    },
    PublicCopyForbiddenTerm {
        matching: TermMatching::Substring,
        value: "pages functions",
    },
];

#[cfg(test)]
pub(super) const RETIRED_PUBLIC_MINTLIFY_SLUG: &str = concat!("ze", "rct", "-4cdab021");

/// Contract value named `RETIRED_PUBLIC_NAME`.
pub(super) const RETIRED_PUBLIC_NAME: &str = concat!("ze", "rct");

#[cfg(test)]
pub(super) const RETIRED_PUBLIC_NAME_TITLE: &str = concat!("Ze", "rct");

/// Contract value named `RETIRED_PUBLIC_ORG_SCOPE`.
pub(super) const RETIRED_PUBLIC_ORG_SCOPE: &str = concat!("@", "ze", "rct");

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0003] = [
    size_of_val(&contains_ascii_word),
    size_of_val(&public_copy_forbidden_terms),
    size_of_val(&retired_public_names),
];

#[derive(Clone, Copy, Debug)]
/// Contract representation for `PublicCopyForbiddenTerm`.
pub(super) struct PublicCopyForbiddenTerm {
    /// Matching strategy for the forbidden term.
    matching: TermMatching,
    /// Forbidden lowercase public-copy term.
    value: &'static str,
}

/// Matching strategy for a forbidden public-copy term.
#[derive(Clone, Copy, Debug)]
enum TermMatching {
    /// Match the term as a substring.
    Substring,
    /// Match the term as a whole ASCII word.
    WholeWord,
}

/// Contract implementation for `contains_ascii_word`.
pub(super) fn contains_ascii_word(value: &str, forbidden_word: &str) -> bool {
    return value
        .split(|character: char| return !character.is_ascii_alphanumeric())
        .any(|word| return word.eq_ignore_ascii_case(forbidden_word));
}

/// Contract implementation for `public_copy_forbidden_terms`.
pub(super) const fn public_copy_forbidden_terms() -> &'static [PublicCopyForbiddenTerm] {
    return PUBLIC_COPY_FORBIDDEN_TERMS;
}

/// Contract implementation for `reject_forbidden_public_copy_terms`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_forbidden_public_copy_terms(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for term in public_copy_forbidden_terms() {
        let forbidden = match term.matching {
            TermMatching::Substring => lower.contains(term.value),
            TermMatching::WholeWord => contains_ascii_word(lower.as_str(), term.value),
        };
        if forbidden {
            return Err(format!(
                "{label} contains forbidden public positioning term: {}",
                term.value
            ));
        }
    }
    return Ok(());
}

/// Contract implementation for `retired_public_names`.
pub(super) const fn retired_public_names() -> &'static [&'static str] {
    return &[RETIRED_PUBLIC_NAME, "xquik"];
}
