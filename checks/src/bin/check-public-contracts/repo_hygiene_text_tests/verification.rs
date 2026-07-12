use super::{reject_private_implementation_terms, validate_tracked_text};

/// Verify canonical tracked text satisfies every byte-level invariant.
///
/// # Panics
///
/// Panics when ordinary LF-terminated UTF-8 is rejected.
#[test]
fn accepts_canonical_tracked_text() {
    assert_eq!(
        validate_tracked_text("canonical.txt", b"first\nsecond\n"),
        Ok(())
    );
}

/// Verify ordinary public terminology and binary separators remain accepted.
///
/// # Panics
///
/// Panics when the private-term fingerprint scan produces a false positive.
#[test]
fn accepts_public_repository_terms() {
    for contents in [
        b"public browser automation provider".as_slice(),
        b"\xffpublic-repository\xfe".as_slice(),
    ] {
        assert_eq!(
            reject_private_implementation_terms("public fixture", contents),
            Ok(()),
        );
    }
}

/// Verify every noncanonical tracked-text class is rejected.
///
/// # Panics
///
/// Panics when malformed text or an oversized file is accepted.
#[test]
fn rejects_noncanonical_tracked_text() {
    for contents in [
        b"nul\0byte\n".as_slice(),
        b"crlf\r\n".as_slice(),
        b"missing final LF".as_slice(),
        b"space \n".as_slice(),
        b"tab\t\n".as_slice(),
        &[0xff],
    ] {
        assert!(validate_tracked_text("invalid.txt", contents).is_err());
    }
    let oversized = vec![b'a'; 0x0008_0001];
    assert!(validate_tracked_text("oversized.txt", oversized.as_slice()).is_err());
}
