use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, write::GzEncoder};
use reqwest::{Method, StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeSet,
    env, fmt, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};
use walkdir::{DirEntry, WalkDir};

mod api_commands;
mod args;
mod auth;
mod config;
mod constants;
mod deploy;
mod doctor;
mod errors;
mod frontend_checks;
mod help;
mod preview;
mod project;
mod runtime;
mod template_sources;
mod templates;

pub(crate) use api_commands::*;
pub(crate) use args::*;
pub(crate) use auth::*;
pub(crate) use config::*;
pub(crate) use constants::*;
pub(crate) use deploy::*;
pub(crate) use doctor::*;
pub(crate) use errors::*;
pub(crate) use frontend_checks::*;
pub(crate) use help::*;
pub(crate) use preview::*;
pub(crate) use project::*;
pub(crate) use runtime::*;
pub(crate) use template_sources::*;
pub(crate) use templates::*;

/// Runs the native Tovuk CLI.
pub(crate) fn entrypoint() -> ExitCode {
    runtime_entrypoint()
}
