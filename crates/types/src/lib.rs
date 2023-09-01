use std::{collections::HashMap, future::Future, pin::Pin, fmt::Display};

use plugy::core::PluginLoader;
use serde::{Deserialize, Serialize};
use solvent::DepGraph;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource<E> {
    CronJob(String),
    PluginEvent(E),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription<E> {
    pub source: EventSource<E>,
    pub event: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfig {
    name: String,
    path: String, // Path to the wasm or tar.gz file
}

impl PluginLoader for PluginConfig {
    fn name(&self) -> &'static str {
        Box::leak((self.name.clone()).into_boxed_str())
    }
    fn load(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, anyhow::Error>>>> {
        let file_path = self.path.clone();
        std::boxed::Box::pin(async {
            let res = std::fs::read(file_path)?;
            Ok(res)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub title: String,
    pub entry_point: String,
    pub plugins: Vec<String>,
    pub subscriptions: Vec<Subscription<String>>, // List of module subscriptions
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub plugins: Vec<PluginConfig>, // List of plugins
    pub modules: Vec<Module>,       // List of modules
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        let mut dep_graph: DepGraph<&str> = DepGraph::new();
        for mods in &self.modules {
            dep_graph
                .register_dependencies(&mods.title, mods.plugins.iter().map(|s| &**s).collect())
        }

        Ok(())
    }
}

pub fn test_config() -> Config {
    // Generate some sample data for Config
    Config {
        plugins: vec![
            PluginConfig {
                name: "tasks".to_owned(),
                path: "/home/geoff/Projects/kasuku/target/wasm32-unknown-unknown/debug/kasuku_tasks.wasm".to_string(),
            },
            // PluginConfig {
            //     name: "tasks".to_string(),
            //     path: "tasks.tar.gz".to_string(),
            // },
        ],
        modules: vec![
            Module {
                title: "Planning".to_string(),
                entry_point: "/home/geoff/Documents/kasuku/Tasks".to_string(),
                plugins: vec!["tasks".to_string()],
                subscriptions: vec![],
            },
        ],
    }
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
    CustomAction(serde_json::Value),
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
        write!(f, "emit({})", serde_json::to_string(&self).unwrap())
    }
}

pub fn emit<Ev>(data: &Ev) -> Emit<&Ev> {
    Emit {
        data,
        r#type: std::any::type_name::<Ev>().to_owned(),
    }
}
