use super::npm_package_lock_versions;

/// Verify both npm lockfile version copies are returned independently.
///
/// # Panics
///
/// Panics when either extracted version differs from the fixture.
#[test]
fn reads_both_package_lock_versions() {
    let source = r#"{
        "version": "1.2.3",
        "packages": {
            "": { "version": "4.5.6" }
        }
    }"#;
    let actual = npm_package_lock_versions(source);
    let expected = Ok([String::from("1.2.3"), String::from("4.5.6")]);
    assert_eq!(actual, expected, "both lockfile versions must be preserved");
}

/// Verify malformed or incomplete lockfile version metadata is rejected.
///
/// # Panics
///
/// Panics when invalid version metadata is accepted or misclassified.
#[test]
fn rejects_invalid_package_lock_versions() {
    for (source, expected) in [
        (
            r#"{"packages":{"":{"version":"1.2.3"}}}"#,
            "npm package lock version must be a string",
        ),
        (
            r#"{"version":123,"packages":{"":{"version":"1.2.3"}}}"#,
            "npm package lock version must be a string",
        ),
        (
            r#"{"version":"1.2.3","packages":{}}"#,
            "npm package lock packages[\"\"] must exist",
        ),
        (
            r#"{"version":"1.2.3","packages":{"":{}}}"#,
            "npm package lock packages[\"\"] version must be a string",
        ),
        (
            r#"{"version":"1.2.3","packages":{"":{"version":456}}}"#,
            "npm package lock packages[\"\"] version must be a string",
        ),
    ] {
        assert_eq!(
            npm_package_lock_versions(source),
            Err(expected.to_owned()),
            "invalid npm lockfile metadata must fail closed: {source}"
        );
    }
}
