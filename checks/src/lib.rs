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

#[path = "support.rs"]
pub mod check_support;

use flate2 as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use tar as _;
