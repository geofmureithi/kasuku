pub mod config;

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Event<T> {
    pub data: T,
    pub path: String,
}

pub type RenderEvent = Event<HashMap<String, String>>;

pub type HandleEvent = Event<FileEvent>;

pub type HandleError = serde_error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEvent {
    Create(Vec<u8>),
    Update(Vec<u8>),
    Delete,
    CustomAction(Vec<u8>),
}

#[plugy::macros::plugin]
pub trait Plugin: Send + Sync {
    /// Return true if should re-render
    fn handle(&self, msg: FileEvent) -> Result<bool, HandleError>;
    /// Return a rendered html version
    fn render(&self, evt: RenderEvent) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emit<Ev> {
    data: Ev,
    r#type: String,
}

impl<Ev: Serialize> Display for Emit<Ev> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "emit({})", serde_json_wasm::to_string(&self).unwrap())
    }
}

pub fn emit<Ev>(data: &Ev) -> Emit<&Ev> {
    Emit {
        data,
        r#type: std::any::type_name::<Ev>().to_owned(),
    }
}
