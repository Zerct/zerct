use super::{
    DocsCacheIdentity, ResponseConstraints, bounded_response_text, render_cache_path,
    validate_check_id, validate_revision,
};

use http::StatusCode;

/// Verify declared and chunked public docs bodies obey the hard ceiling.
///
/// # Panics
///
/// Panics when bounded response enforcement accepts an oversized body.
#[test]
fn enforces_declared_and_streamed_response_limits() {
    let mut declared_reader: &[u8] = b"small";
    let declared_error = bounded_response_text(
        &mut declared_reader,
        &ResponseConstraints {
            content_length: Some(0x0009),
            maximum: 0x0008,
            path: "/declared".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert!(
        declared_error.is_err(),
        "an oversized Content-Length must fail before body acceptance"
    );

    let mut chunked_reader: &[u8] = b"123456789";
    let chunked_error = bounded_response_text(
        &mut chunked_reader,
        &ResponseConstraints {
            content_length: None,
            maximum: 0x0008,
            path: "/chunked".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert!(
        chunked_error.is_err(),
        "an oversized chunked body must fail at the streaming ceiling"
    );
}

/// Verify a bounded public docs body is returned unchanged.
///
/// # Panics
///
/// Panics when a valid bounded response cannot be decoded.
#[test]
fn returns_bounded_utf8_body() {
    let mut reader: &[u8] = b"public docs";
    let result = bounded_response_text(
        &mut reader,
        &ResponseConstraints {
            content_length: Some(0x000b),
            maximum: 0x0010,
            path: "/docs".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert_eq!(
        result.map_err(|error| return error.message),
        Ok("public docs".to_owned()),
        "a bounded UTF-8 response must be preserved"
    );
}

/// Verify deployment attempts receive unique CDN cache keys.
///
/// # Panics
///
/// Panics when a valid deployment identity is rejected or two attempts share a key.
#[test]
fn uses_unique_deployment_attempt_keys() {
    let revision = "6c3159be79131fc71faa678d8e09a0ad31191615";
    let identity_result = DocsCacheIdentity::new(revision.to_owned(), "29157505718-1".to_owned());
    assert!(identity_result.is_ok(), "the cache identity must be valid");
    let Some(identity) = identity_result.ok() else {
        return;
    };
    assert_eq!(validate_revision(revision), Ok(()));
    assert_eq!(
        render_cache_path("/llms-full.txt", Some(&identity), 0),
        "/llms-full.txt?revision=6c3159be79131fc71faa678d8e09a0ad31191615&check=29157505718-1&attempt=0"
    );
    assert_ne!(
        render_cache_path("/llms-full.txt", Some(&identity), 0),
        render_cache_path("/llms-full.txt", Some(&identity), 0x0001),
        "each full readiness attempt must use a fresh CDN cache key"
    );
    assert_eq!(
        render_cache_path("/llms-full.txt", None, 0),
        "/llms-full.txt"
    );
}

/// Verify deployment cache identities reject query injection and noncanonical values.
///
/// # Panics
///
/// Panics when unsafe revision or workflow identity input is accepted.
#[test]
fn validates_unsafe_deployment_cache_identity_rejection() {
    let revision = "6c3159be79131fc71faa678d8e09a0ad31191615";
    assert_eq!(validate_revision(revision), Ok(()));
    assert!(
        validate_revision("6c3159be79131fc71faa678d8e09a0ad31191615&unsafe=true").is_err(),
        "query syntax must not be accepted as a deployment revision"
    );
    assert!(
        validate_revision("6C3159BE79131FC71FAA678D8E09A0AD31191615").is_err(),
        "deployment revisions must use their canonical lowercase representation"
    );
    assert_eq!(validate_check_id("29157505718-1"), Ok(()));
    assert!(
        validate_check_id("29157505718-1&unsafe=true").is_err(),
        "query syntax must not be accepted as a workflow check identity"
    );
}
