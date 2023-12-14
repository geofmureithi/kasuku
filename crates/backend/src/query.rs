
use ::types::{Event, Emit};
use async_graphql::*;

use crate::{KasukuRuntime, BackendPlugin};

pub struct QueryRoot;

#[derive(Debug, serde::Serialize, serde::Deserialize, InputObject)]
pub struct Render {
    file: String,
    renderer: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SimpleObject)]
pub struct File {
    name: String,
    path: String,
    size: usize,
    mime_type: String,
    last_modified: String,
    meta: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Enum, Clone, Copy, PartialEq, Eq)]
pub enum DomNode {
    Element,
    Comment,
    Text,
    Fragment, // Element {
              //     name: String,
              //     attributes: HashMap<String, String>,
              //     children: Vec<DomNode>,
              // },
              // Comment(Comment),
              // Text(Text),
              // Fragment(Vec<DomNode>),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, SimpleObject)]
pub struct RenderResult {
    main: DomNode,
    toolbar: Vec<DomNode>,
}

#[Object]
impl QueryRoot {
    async fn render(&self, ctx: &Context<'_>, _req: Render) -> serde_json::Value {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let plugin: ::types::PluginWrapper<BackendPlugin, _> = runtime.get_plugin_by_name("tasks").unwrap();
        let res = plugin
            .render(
                ::types::Context::acquire(),
                Event {
                    path: "text".to_string(),
                    data: Emit {
                        data: vec![],
                        r#type: "Event".to_string()
                    }
                },
            )
            .await
            .unwrap();
        let json = serde_json::from_slice(&res.0).unwrap();
        json
    }
    async fn list_files(&self) -> Vec<File> {
        vec![]
    }

    async fn config(&self) -> u8 {
        28
    }
}
