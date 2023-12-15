use ::types::{Emit, Event};
use async_graphql::*;
use kasuku_database::prelude::Payload;
use serde::Deserialize;
use serde::Serialize;

use crate::{BackendPlugin, KasukuRuntime};

pub struct QueryRoot;

#[derive(Debug, serde::Serialize, serde::Deserialize, SimpleObject)]
#[graphql(complex)]
pub struct File {
    path: String,
    // size: usize,
    // mime_type: String,
    // last_modified: String,
    // meta: Option<serde_json::Value>,
}

#[ComplexObject]
impl File {
    async fn render(&self, ctx: &Context<'_>, renderer: Option<String>) -> serde_json::Value {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let plugin: ::types::PluginWrapper<BackendPlugin, _> =
            runtime.get_plugin_by_name("tasks").unwrap();
        let res = plugin
            .render(
                ::types::Context::acquire(),
                Event {
                    path: "text".to_string(),
                    data: Emit {
                        data: vec![],
                        r#type: "Event".to_string(),
                    },
                },
            )
            .await
            .unwrap();
        serde_json::from_slice(&res.0).unwrap()
    }
}

#[Object]
impl QueryRoot {
    async fn render_file(
        &self,
        ctx: &Context<'_>,
        path: String,
        renderer: Option<String>,
    ) -> serde_json::Value {
        File { path }
            .render(ctx, renderer)
            .await
            .unwrap()
    }
    async fn vaults(&self, ctx: &Context<'_>) -> Vec<Vault> {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let mut lock = runtime.database.lock();
        let db = lock.as_mut();
        let database = db.unwrap();
        let res = database
            .execute(format!("Select name, mount from vaults"))
            .unwrap();
        let payload = res.get(0).unwrap();
        match payload {
            Payload::Select { rows, .. } => rows
                .into_iter()
                .map(|row| Vault {
                    name: row.get(0).unwrap().into(),
                    mount: row.get(1).unwrap().into(),
                })
                .collect(),
            _ => unreachable!(),
        }
    }

    async fn config(&self) -> u8 {
        28
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, async_graphql::SimpleObject)]
#[graphql(complex)]
pub struct Vault {
    pub name: String,
    pub mount: String,
    // pub plugins: Vec<String>,
}

#[ComplexObject]
impl Vault {
    async fn entries(
        &self,
        ctx: &Context<'_>,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Vec<File> {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let mut lock = runtime.database.lock();
        let db = lock.as_mut();
        let database = db.unwrap();
        let res = database
            .execute(format!(
                "Select path from entries WHERE vault = '{}' OFFSET {} LIMIT {};",
                self.name,
                offset.unwrap_or(0),
                limit.unwrap_or(0)
            ))
            .unwrap();
        let payload = res.get(0).unwrap();
        match payload {
            Payload::Select { rows, .. } => rows
                .into_iter()
                .map(|row| File {
                    path: row.get(0).unwrap().into(),
                })
                .collect(),
            _ => unreachable!(),
        }
    }
}
