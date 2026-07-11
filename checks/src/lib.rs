//! Shared support for Tovuk public repository check binaries.

/// Propagate a failed check without using the question-mark operator.
#[macro_export]
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

extern crate alloc;

#[path = "support.rs"]
pub mod check_support;
pub mod http_transport;

use flate2 as _;
use http as _;
use http_body_util as _;
use hyper as _;
use hyper_rustls as _;
use hyper_util as _;
use rustls as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use tar as _;
use tokio as _;
use url as _;
