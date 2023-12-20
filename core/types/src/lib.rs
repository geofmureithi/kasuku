pub mod config;

use serde::{Deserialize, Serialize};
use std::{collections::HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub title: String,
    completed: bool,
    due: Option<String>,
    meta: Option<HashMap<String, String>>,
}

impl Task {
    pub fn new(text: String) -> Self {
        Self {
            title: text,
            completed: false,
            due: None,
            meta: None,
        }
    }
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
    DatabaseError(String)
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
    pub path: Option<String>,
    pub data: FileType, // The contents of the file serialized in bincode
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum FileType {
    #[default]
    Unknown,
    Markdown(Vec<u8>),
}

/// This is a plugin that does nothing and can be important for creating events that are multi-plugin.
/// It should not be invoked or used.
pub struct IdentityPlugin;
