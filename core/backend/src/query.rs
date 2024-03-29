use ::types::Event;
use async_graphql::*;
use interface::PluginWrapper;
use kasuku_database::prelude::Payload;
use markdown::AsMarkdown;
use markdown::IsMatched;
use markdown::MarkdownEvent;
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
    async fn preview(&self, ctx: &Context<'_>) -> serde_json::Value {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let plugin: PluginWrapper<BackendPlugin, _> = runtime.get_plugin_by_name("tasks").unwrap();
        let res = plugin
            .render(
                ::context::Context::acquire(),
                Event {
                    namespace: "text".to_string(),
                    data: vec![],
                },
            )
            .await
            .unwrap();
        serde_json::from_str(&res.0).unwrap()
    }
}

#[Object]
impl QueryRoot {
    async fn render_file(
        &self,
        ctx: &Context<'_>,
        path: String,
        _renderer: Option<String>,
    ) -> serde_json::Value {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let res = runtime
            .database
            .lock()
            .unwrap()
            .execute("SELECT * FROM subscriptions WHERE event = 'markdown::MarkdownEvent';")
            .unwrap();
        let subscriptions = res.get(0).unwrap();

        let render = match subscriptions {
            Payload::Select { rows, .. } => {
                let filters: Vec<(MarkdownEvent, String)> = rows
                    .iter()
                    .map(|row| {
                        (
                            match row.get(3).unwrap() {
                                kasuku_database::prelude::Value::Str(val) => {
                                    serde_json::from_str(val).unwrap()
                                }
                                _ => unreachable!(),
                            },
                            row.get(0)
                                .map(|s| match s {
                                    kasuku_database::prelude::Value::Str(txt) => txt.clone(),
                                    _ => unreachable!(),
                                })
                                .unwrap(),
                        )
                    })
                    .collect();
                let md = tokio::fs::read_to_string(&path).await.unwrap();

                let events = markdown::parse(&md).unwrap();
                let plugins: Vec<String> = events
                    .events
                    .iter()
                    .flat_map(move |event| {
                        filters
                            .clone()
                            .iter()
                            .filter(move |filter| filter.0.is_matched(event).unwrap())
                            .map(|c| c.1.clone())
                            .collect::<Vec<_>>()
                    })
                    .collect::<std::collections::HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<String>>();
                let mut md = {
                    ::types::File {
                        data: ::types::FileType::Markdown(bincode::serialize(&events).unwrap()),
                        path,
                    }
                };
                for plugin in plugins {
                    println!("{plugin:?}");
                    let plugin: PluginWrapper<BackendPlugin, _> =
                        runtime.get_plugin_by_name(&plugin).unwrap();
                    md = plugin
                        .process_file(::context::Context::acquire(), md)
                        .await
                        .unwrap();
                }
                let mut buf = String::new();
                let md_events = &md.data.to_markdown().unwrap().events;
                pulldown_cmark_to_cmark::cmark(md_events.iter(), &mut buf).unwrap();
                buf
            }
            _ => unreachable!(),
        };

        serde_json::to_value(render).unwrap()
    }
    async fn vaults(&self, ctx: &Context<'_>) -> Vec<Vault> {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let mut lock = runtime.database.lock();
        let db = lock.as_mut();
        let database = db.unwrap();
        let res = database.execute("Select name, mount from vaults").unwrap();
        let payload = res.get(0).unwrap();
        match payload {
            Payload::Select { rows, .. } => rows
                .iter()
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
                .iter()
                .map(|row| File {
                    path: row.get(0).unwrap().into(),
                })
                .collect(),
            _ => unreachable!(),
        }
    }
}
