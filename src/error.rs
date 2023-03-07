use std::{fmt, io, string::FromUtf8Error};

use thiserror::Error as ThisError;
use strict_yaml_rust::{EmitError, ScanError};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Can't load YAML from string")]
    LoadYaml(#[from] ScanError),
    #[error("Manifest file is malformed")]
    ManifestMalformed,
    #[error("Manifest file is malformed, missing 'cmd' key")]
    MissingCmd,
    #[error("IO - {0}")]
    Io(#[from] io::Error),
    #[error("Manifest file doesn't exist - '{0}'")]
    ManifestFileDoesntExist(String),
    #[error("Problem converting from UTF-8")]
    ConvertUTF8(#[from] FromUtf8Error),
    #[error("Invalid path - '{0}'")]
    InvalidPath(String),
    #[error("Cloudn't update config")]
    SerializeYaml(#[from] EmitError),
    #[error("Cloudn't dump entry")]
    DumpEntry(#[from] fmt::Error),
}
