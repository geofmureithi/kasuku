pub mod config;
pub mod table;

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Home,
    Menu,
    Commands,
    Main,
}

#[derive(Debug, Serialize, Deserialize, thiserror::Error)]
pub enum Error {
    #[error("ser/de failed: `{0}`")]
    Serialization(String),
    #[error("a render was requested but cannot be completed")]
    InvalidRender,
    #[error("parse sql error")]
    SqlParser,
    #[error("File not encoded correctly `{0}`")]
    FileCodec(String),
    #[error("Regex could not be parse `{0}`")]
    Regex(String),
    #[error("A plugin error occurred `{0}`")]
    PluginError(String),
    #[error("A database error occurred `{0}`")]
    DatabaseError(String),
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Serialization(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Serialization(msg.to_string())
    }
}

impl Error {
    /// Create the instance of `Unsupported` during serialization `Error`
    pub fn ser_unsupported(typ: &str) -> Self {
        Error::Serialization(format!("Serialization is not supported from type: {}", typ))
    }

    /// Create the instance of `Unsupported` during deserialization `Error`
    pub fn de_unsupported(typ: &str) -> Self {
        Error::Serialization(format!(
            "Deserialization is not supported into type: {}",
            typ
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewType {
    SideBar,
    Dashboard,
    FloatingMenu,
}

pub trait PluginEvent {
    type Plugin;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct Rsx(pub String);

/// A serializable version of a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct File {
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RawValue {
    /// The value is a `NULL` value.
    Null,
    /// The value is a signed integer.
    Integer(i64),
    /// The value is a floating point number.
    Real(f64),
    /// The value is a text string.
    Text(String),
    /// The value is a blob of data
    Blob(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, RawValue>>,
}
/// This is a plugin that does nothing and can be important for creating events that are multi-plugin.
/// It should not be invoked or used.
pub struct IdentityPlugin;

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginSubscription {
    pub data: String,
    pub event: String,
    pub event_type: String,
    pub plugin: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct FilePath {
    pub filename: String,
    pub vault: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub plugin: String,
}
