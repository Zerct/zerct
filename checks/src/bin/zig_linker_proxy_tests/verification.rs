//! Zig linker proxy verification.

use std::{ffi::OsString, fs::read, path::Path};

use super::{
    DEPRECATED_LINKER_OPTIMIZATION, PreparedArguments, ProxyResult, TemporaryResponseFile,
    create_temporary_response, prepare_arguments, response_argument,
};

/// Compile-time references preserve the single-use helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&delegated_response_path),
    size_of_val(&prepare_response_fixture),
];

/// Source and sanitized arguments for one response-file fixture.
#[derive(Debug)]
struct ResponseFixture {
    /// Arguments prepared for the real Zig process.
    prepared: PreparedArguments,
    /// Unmodified source response file.
    source: TemporaryResponseFile,
}

/// Find the sanitized response path delegated by the proxy.
fn delegated_response_path<'fixture>(
    prepared: &'fixture PreparedArguments,
    source_path: &Path,
) -> Option<&'fixture Path> {
    for argument in &prepared.arguments {
        let Some(text) = argument.to_str() else {
            continue;
        };
        let Some(path) = text.strip_prefix('@') else {
            continue;
        };
        let candidate = Path::new(path);
        if candidate != source_path {
            return Some(candidate);
        }
    }
    return None;
}

/// Verify that combined or decorated option forms fail closed.
///
/// # Errors
///
/// Returns an error when an ambiguous direct linker option is accepted.
#[test]
fn direct_ambiguous_option_is_rejected() -> Result<(), String> {
    let result = prepare_arguments(
        ["cc", "-Wl,-O1,--gc-sections"]
            .into_iter()
            .map(OsString::from)
            .collect(),
    );
    return require_error_contains(result, "ambiguous linker option");
}

/// Verify that only the exact direct linker option is removed.
///
/// # Errors
///
/// Returns an error when direct argument preparation or cleanup fails.
#[test]
fn direct_exact_option_is_removed() -> Result<(), String> {
    let prepared = check_try!(prepare_arguments(
        [
            "cc",
            "input.o",
            DEPRECATED_LINKER_OPTIMIZATION,
            "-o",
            "tovuk",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
    ));
    let expected = ["cc", "input.o", "-o", "tovuk"]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    if prepared.arguments != expected {
        return Err("the exact deprecated direct option was not removed".to_owned());
    }
    if !prepared.temporary_files.is_empty() {
        return Err("direct filtering unexpectedly created a response file".to_owned());
    }
    return prepared.cleanup();
}

/// Verify that non-compiler Zig commands delegate byte-for-byte.
///
/// # Errors
///
/// Returns an error when non-compiler argument preparation or cleanup fails.
#[test]
fn non_compiler_command_is_unchanged() -> Result<(), String> {
    let original = ["version", DEPRECATED_LINKER_OPTIMIZATION]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    let prepared = check_try!(prepare_arguments(original.clone()));
    if prepared.arguments != original {
        return Err("non-compiler Zig arguments changed".to_owned());
    }
    return prepared.cleanup();
}

/// Create one source response and prepare its sanitized argument.
///
/// # Errors
///
/// Returns an error when fixture creation or argument preparation fails.
fn prepare_response_fixture(contents: &[u8]) -> ProxyResult<ResponseFixture> {
    let source = check_try!(create_temporary_response(contents));
    let prepared_result = prepare_arguments(vec![
        OsString::from("cc"),
        response_argument(source.path.as_path()),
    ]);
    return match prepared_result {
        Ok(prepared) => Ok(ResponseFixture { prepared, source }),
        Err(prepare_error) => match source.cleanup() {
            Ok(()) => Err(prepare_error),
            Err(cleanup_error) => Err(format!("{prepare_error}; {cleanup_error}")),
        },
    };
}

/// Require one operation to fail with the selected diagnostic.
///
/// # Errors
///
/// Returns an error when the operation succeeds or reports another failure.
fn require_error_contains<Value>(result: ProxyResult<Value>, expected: &str) -> ProxyResult<()> {
    let Err(message) = result else {
        return Err(format!(
            "operation unexpectedly succeeded; expected {expected}"
        ));
    };
    if !message.contains(expected) {
        return Err(format!("unexpected proxy error: {message}"));
    }
    return Ok(());
}

/// Verify ambiguous response-file tokens fail closed.
///
/// # Errors
///
/// Returns an error when fixture creation or cleanup fails, or an ambiguous
/// response is accepted.
#[test]
fn response_ambiguous_option_is_rejected() -> Result<(), String> {
    let source = check_try!(create_temporary_response(b"input.o\n\"-Wl,-O1\"\n"));
    let result = prepare_arguments(vec![
        OsString::from("c++"),
        response_argument(source.path.as_path()),
    ]);
    let validation = require_error_contains(result, "ambiguous response-file linker option");
    check_try!(source.cleanup());
    return validation;
}

/// Verify response filtering preserves the source and all unrelated bytes.
///
/// # Errors
///
/// Returns an error when preparation, reading, cleanup, or byte preservation
/// fails.
#[test]
fn response_exact_lines_are_removed_from_a_copy() -> Result<(), String> {
    let original = b"input.o\n-Wl,-O1\r\n-Wl,--gc-sections\n-Wl,-O1";
    let fixture = check_try!(prepare_response_fixture(original));
    let source_path = fixture.source.path.clone();
    let delegated_path = check_try!(
        delegated_response_path(&fixture.prepared, source_path.as_path())
            .ok_or_else(|| return "sanitized response argument was not created".to_owned())
    );
    let sanitized = check_try!(
        read(delegated_path)
            .map_err(|error| return format!("read {}: {error}", delegated_path.display()))
    );
    let unchanged_source = check_try!(
        read(source_path.as_path())
            .map_err(|error| return format!("read {}: {error}", source_path.display()))
    );
    if sanitized != b"input.o\n-Wl,--gc-sections\n" {
        return Err(format!(
            "unexpected sanitized response: {}",
            String::from_utf8_lossy(sanitized.as_slice())
        ));
    }
    if unchanged_source != original {
        return Err("cargo-zigbuild source response was modified".to_owned());
    }
    check_try!(fixture.prepared.cleanup());
    return fixture.source.cleanup();
}

/// Verify invalid response-file encodings fail closed.
///
/// # Errors
///
/// Returns an error when fixture creation or cleanup fails, or invalid UTF-8 is
/// accepted.
#[test]
fn response_invalid_utf8_is_rejected() -> Result<(), String> {
    let source = check_try!(create_temporary_response(&[0xff, 0xfe, 0x00]));
    let result = prepare_arguments(vec![
        OsString::from("cc"),
        response_argument(source.path.as_path()),
    ]);
    let validation = require_error_contains(result, "valid UTF-8");
    check_try!(source.cleanup());
    return validation;
}
