use super::validate_commit_text;

/// A GitHub-style signed merge commit uses a single-space continuation line.
const SIGNED_COMMIT: &[u8] = b"tree 0123456789012345678901234567890123456789\n\
parent 1111111111111111111111111111111111111111\n\
parent 2222222222222222222222222222222222222222\n\
author Public Author <author@example.com> 1 +0000\n\
committer GitHub <noreply@github.com> 1 +0000\n\
gpgsig -----BEGIN PGP SIGNATURE-----\n\
 \n\
 signature-data\n\
 =checksum\n\
 -----END PGP SIGNATURE-----\n\
 \n\
\n\
Merge public candidate\n";

/// Verify Git's canonical signature continuation framing remains accepted.
///
/// # Panics
///
/// Panics when a canonical signed commit is rejected.
#[test]
fn accepts_signed_commit_continuation_blank_line() {
    assert_eq!(validate_commit_text("signed commit", SIGNED_COMMIT), Ok(()));
}

/// Verify malformed commit whitespace and framing remain rejected.
///
/// # Panics
///
/// Panics when noncanonical raw commit bytes are accepted.
#[test]
fn rejects_noncanonical_commit_text() {
    for contents in [
        b"tree object \n\nmessage\n".as_slice(),
        b"tree object\n continuation \n\nmessage\n".as_slice(),
        b"tree object\n\nmessage \n".as_slice(),
        b" continuation\n\nmessage\n".as_slice(),
        b"tree object\nmessage\n".as_slice(),
        b"tree object\r\n\r\nmessage\r\n".as_slice(),
        b"tree object\n\nmessage".as_slice(),
        b"tree object\n\nmessage\0\n".as_slice(),
        &[0xff],
    ] {
        assert!(
            validate_commit_text("invalid commit", contents).is_err(),
            "noncanonical commit bytes must be rejected",
        );
    }
}
