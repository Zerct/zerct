# Public ring compatibility patch

This directory starts from the public `ring` 0.17.14 crate published on
crates.io with checksum
`a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7`.

The patch updates `getrandom` from 0.2 to 0.4, updates the Windows dependency
from `windows-sys` 0.52 to 0.61, uses the renamed `getrandom` APIs, and makes
lifetimes explicit where Rust 1.97 reports ambiguous lifetime syntax. These
changes follow the public upstream `ring` development line while retaining the
released 0.17.14 implementation and package identity.

Upstream license expression: `Apache-2.0 AND ISC`.

The exact changed upstream files are:

- `Cargo.toml`
- `Cargo.toml.orig`
- `src/arithmetic/bigint/modulus.rs`
- `src/arithmetic/limbs512/storage.rs`
- `src/pkcs8.rs`
- `src/polyfill/slice/as_chunks.rs`
- `src/polyfill/slice/as_chunks_mut.rs`
- `src/rand.rs`
- `src/rsa/public_modulus.rs`

This `TOVUK-PATCH.md` file is added by the public repository, and the
registry extraction marker `.cargo-ok` is intentionally not tracked.

The public repository carries this narrow patch because Cargo locks optional
dependency metadata and maximum Clippy rejects duplicate dependency versions,
even when the older optional provider is not selected at runtime. Do not add
Tovuk product code or private configuration to this directory.
