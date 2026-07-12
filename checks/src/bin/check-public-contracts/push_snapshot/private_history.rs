//! Efficient private-term audit for every object reachable from local refs.

use alloc::collections::BTreeSet;

use crate::{
    helpers::{CheckResult, OutputChannel, write_line},
    repo_hygiene_text::{MAX_TRACKED_TEXT_BYTES, reject_private_implementation_terms},
};

use std::{
    io::{BufRead as _, BufReader, Read},
    path::Path,
    process::{ChildStdout, Stdio},
};

use super::{ObjectKind, git, git_command, graph};

/// Maximum allocation used while discarding one unreachable object body.
const SKIP_BUFFER_BYTES_U64: u64 = 0x1000;

/// One canonical header emitted by `git cat-file --batch`.
#[derive(Clone, Debug, Eq, PartialEq)]
struct BatchHeader {
    /// Stored Git object kind.
    kind: ObjectKind,
    /// Canonical lowercase object identifier.
    object: String,
    /// Exact following body size in bytes.
    size: u64,
}

const _: () = {
    _ = BatchHeader::parse;
    _ = BatchHeader::parse;
    _ = BatchHeader::parse_size;
    _ = BatchHeader::parse_size;
    _ = check;
    _ = check;
    _ = read_reachable_objects;
    _ = read_reachable_objects;
    _ = scan_batch_stream;
    _ = scan_batch_stream;
    _ = scan_reachable_objects;
    _ = scan_reachable_objects;
    _ = skip_chunk_size;
    _ = skip_chunk_size;
};

impl BatchHeader {
    /// Return a generic object label safe for public diagnostics.
    fn label(&self) -> String {
        return format!("reachable Git {:?} {}", self.kind, self.object);
    }

    /// Parse one complete canonical batch header.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, unknown, oversized, or noncanonical metadata.
    fn parse(line: &str, object_width: usize) -> CheckResult<Self> {
        let source = check_try!(
            line.strip_suffix('\n')
                .ok_or_else(|| return "Git batch header is not LF-terminated".to_owned())
        );
        if source.contains('\r') {
            return Err("Git batch header contains a carriage return".to_owned());
        }
        let fields = source.split_ascii_whitespace().collect::<Vec<_>>();
        let [object, kind_name, size_text] = *fields.as_slice() else {
            return Err("Git batch header must contain exactly three fields".to_owned());
        };
        if !git::valid_object_shape(object, object_width) {
            return Err("Git batch header contains an invalid object ID".to_owned());
        }
        let kind = match kind_name {
            "blob" => ObjectKind::Blob,
            "commit" => ObjectKind::Commit,
            "tag" => ObjectKind::Tag,
            "tree" => ObjectKind::Tree,
            other => return Err(format!("Git batch header has unknown kind {other:?}")),
        };
        let size = check_try!(Self::parse_size(size_text));
        let canonical = format!("{object} {kind_name} {size}");
        if source != canonical {
            return Err("Git batch header is not canonical".to_owned());
        }
        return Ok(Self {
            kind,
            object: object.to_owned(),
            size,
        });
    }

    /// Parse one canonical unsigned object size.
    ///
    /// # Errors
    ///
    /// Returns an error when the batch size is not an unsigned 64-bit integer.
    fn parse_size(size_text: &str) -> CheckResult<u64> {
        return size_text
            .parse::<u64>()
            .map_err(|error| return format!("Git batch object size is invalid: {error}"));
    }

    /// Read and validate the framing delimiter after one object body.
    ///
    /// # Errors
    ///
    /// Returns an error when the delimiter is truncated or malformed.
    fn read_body_delimiter(&self, reader: &mut impl Read) -> CheckResult {
        let mut delimiter = [0x00];
        check_try!(
            reader
                .read_exact(&mut delimiter)
                .map_err(|error| return format!("read {} delimiter: {error}", self.label()))
        );
        if delimiter != *b"\n" {
            return Err(format!("{} has an invalid batch delimiter", self.label()));
        }
        return Ok(());
    }

    /// Read the exact body and framing delimiter following this header.
    ///
    /// # Errors
    ///
    /// Returns an error when the body is truncated or its delimiter is malformed.
    fn read_contents(&self, reader: &mut impl Read) -> CheckResult<Vec<u8>> {
        if self.size > MAX_TRACKED_TEXT_BYTES {
            return Err(format!(
                "{} exceeds the {MAX_TRACKED_TEXT_BYTES}-byte public ceiling",
                self.label()
            ));
        }
        let size = check_try!(usize::try_from(self.size).map_err(|error| {
            return format!("{} size does not fit in memory: {error}", self.label());
        }));
        let mut contents = vec![0x00; size];
        check_try!(
            reader
                .read_exact(contents.as_mut_slice())
                .map_err(|error| return format!("read {} body: {error}", self.label()))
        );
        check_try!(self.read_body_delimiter(reader));
        return Ok(contents);
    }

    /// Consume an unreachable object body without allocating it.
    ///
    /// # Errors
    ///
    /// Returns an error when the body is truncated or its delimiter is malformed.
    fn skip_contents(&self, reader: &mut impl Read) -> CheckResult {
        let mut remaining = self.size;
        while remaining != 0x0000 {
            let (chunk, next_remaining) = check_try!(skip_chunk_size(remaining));
            let mut buffer = vec![0x00; chunk];
            check_try!(
                reader
                    .read_exact(buffer.as_mut_slice())
                    .map_err(|error| return format!("skip {} body: {error}", self.label()))
            );
            remaining = next_remaining;
        }
        return self.read_body_delimiter(reader);
    }

    /// Scan this body only when its object is reachable and not already seen.
    ///
    /// # Errors
    ///
    /// Returns an error for a duplicate reachable object or private-term match.
    fn verify_reachable_contents(
        &self,
        contents: &[u8],
        reachable: &BTreeSet<String>,
        remaining: &mut BTreeSet<String>,
    ) -> CheckResult {
        if !reachable.contains(self.object.as_str()) {
            return Ok(());
        }
        if !remaining.remove(self.object.as_str()) {
            return Err(format!("Git batch repeated {}", self.label()));
        }
        return reject_private_implementation_terms(self.label().as_str(), contents);
    }
}

/// Bounded allocation length and exact bytes remaining after one discard.
type SkipChunk = (usize, u64);

/// Scan private-term fingerprints across every object reachable from any local ref.
///
/// # Errors
///
/// Returns an error when history is incomplete, Git objects are unreadable, or
/// any reachable object exposes a private implementation term.
pub(super) fn check() -> CheckResult {
    let repository = Path::new(".");
    check_try!(graph::require_integrity(repository));
    let reachable = check_try!(read_reachable_objects(repository));
    let object_count = reachable.len();
    check_try!(scan_reachable_objects(repository, &reachable));
    return write_line(
        OutputChannel::Regular,
        format!(
            "Checked {object_count} objects reachable from all local refs for private implementation terms."
        )
        .as_str(),
    );
}

/// Read the exact object identifiers reachable from every local ref.
///
/// # Errors
///
/// Returns an error when Git cannot enumerate the complete local ref graph.
fn read_reachable_objects(repository: &Path) -> CheckResult<BTreeSet<String>> {
    return git::git_text(
        repository,
        &["rev-list", "--objects", "--all", "--no-object-names"],
        "git rev-list all refs",
    )
    .map(|objects| return objects.lines().map(str::to_owned).collect());
}

/// Read and classify every framed object emitted by one Git batch process.
///
/// # Errors
///
/// Returns an error when framing is malformed, a reachable object is repeated,
/// or reachable history exposes a private term.
fn scan_batch_stream(
    reader: &mut BufReader<ChildStdout>,
    object_width: usize,
    reachable: &BTreeSet<String>,
    remaining: &mut BTreeSet<String>,
) -> CheckResult {
    loop {
        let mut line = String::new();
        let bytes = check_try!(
            reader
                .read_line(&mut line)
                .map_err(|error| return format!("read Git batch header: {error}"))
        );
        if bytes == 0x0000 {
            return Ok(());
        }
        let header = check_try!(BatchHeader::parse(line.as_str(), object_width));
        if reachable.contains(header.object.as_str()) {
            let contents = check_try!(header.read_contents(reader));
            check_try!(
                header.verify_reachable_contents(contents.as_slice(), reachable, remaining,)
            );
        } else {
            check_try!(header.skip_contents(reader));
        }
    }
}

/// Stream every local object once and scan only objects reachable from refs.
///
/// # Errors
///
/// Returns an error when the batch process fails, emits malformed data, omits
/// a reachable object, or exposes a private term in reachable history.
fn scan_reachable_objects(repository: &Path, reachable: &BTreeSet<String>) -> CheckResult {
    let mut remaining = reachable.clone();
    let mut child = check_try!(
        git_command(repository)
            .args(["cat-file", "--batch", "--batch-all-objects", "--unordered"])
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| return format!("start Git batch object reader: {error}"))
    );
    let stdout = check_try!(
        child
            .stdout
            .take()
            .ok_or_else(|| return "Git batch object reader has no stdout".to_owned())
    );
    let object_width = check_try!(git::object_id_length(repository));
    let mut reader = BufReader::new(stdout);
    check_try!(scan_batch_stream(
        &mut reader,
        object_width,
        reachable,
        &mut remaining,
    ));
    let status = check_try!(
        child
            .wait()
            .map_err(|error| return format!("wait for Git batch object reader: {error}"))
    );
    if !status.success() {
        return Err(format!("Git batch object reader failed with {status}"));
    }
    if let Some(missing) = remaining.first() {
        return Err(format!("Git batch omitted reachable object {missing}"));
    }
    return Ok(());
}

/// Bound one unreachable-object discard allocation and preserve its exact width.
///
/// # Errors
///
/// Returns an error when the bounded chunk cannot fit in the platform `usize`.
fn skip_chunk_size(remaining: u64) -> CheckResult<SkipChunk> {
    let chunk_u64 = remaining.min(SKIP_BUFFER_BYTES_U64);
    let chunk = check_try!(usize::try_from(chunk_u64).map_err(|error| {
        return format!("Git batch skip size does not fit in memory: {error}");
    }));
    let next_remaining = check_try!(
        remaining
            .checked_sub(chunk_u64)
            .ok_or_else(|| return "Git batch skip size underflowed".to_owned())
    );
    return Ok((chunk, next_remaining));
}
