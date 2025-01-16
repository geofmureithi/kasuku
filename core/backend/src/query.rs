use ::types::Event;
use async_graphql::*;
use interface::PluginWrapper;
use markdown::IsMatched;
use markdown::MarkdownEvent;
use markdown::MarkdownFile;
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
                &::context::Context::default(),
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

        #[derive(Debug, Deserialize, Clone)]
        struct Subscription {
            data: String,
            plugin: String,
        }
        let subscriptions: Vec<Subscription> = runtime
            .database
            .query(
                "SELECT data, plugin FROM subscriptions WHERE event = 'markdown::MarkdownEvent';",
            )
            .await
            .unwrap();

        let md = tokio::fs::read_to_string(&path).await.unwrap();

        let file = markdown::parse(&md).unwrap();
        let plugins: Vec<String> = file
            .iter()
            .flat_map(move |event| {
                subscriptions
                    .clone()
                    .iter()
                    .filter(move |filter| {
                        serde_json::from_str::<MarkdownEvent>(&filter.data)
                            .unwrap()
                            .is_matched(event)
                            .unwrap()
                    })
                    .map(|c| c.plugin.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect::<Vec<String>>();
        let mut md: ::types::File = file.try_into().unwrap();
        for plugin in plugins {
            println!("{plugin:?}");
            let plugin: PluginWrapper<BackendPlugin, _> =
                runtime.get_plugin_by_name(&plugin).unwrap();
            md = plugin
                .process_file(&mut ::context::Context::default(), md)
                .await
                .unwrap();
        }
        let mut buf = String::new();
        let md: MarkdownFile = (&md).try_into().unwrap();
        pulldown_cmark_to_cmark::cmark(md.iter(), &mut buf).unwrap();

        serde_json::to_value(buf).unwrap()
    }
    async fn vaults(&self, ctx: &Context<'_>) -> Vec<Vault> {
        let runtime: &KasukuRuntime = ctx.data().unwrap();
        let vaults: Vec<Vault> = runtime
            .database
            .query("Select name, mount from vaults")
            .await
            .unwrap();
        vaults
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
        let res: Vec<File> = runtime
            .database
            .query(&format!(
                "Select path from entries WHERE vault = '{}' OFFSET {} LIMIT {};",
                self.name,
                offset.unwrap_or(0),
                limit.unwrap_or(0)
            ))
            .await
            .unwrap();
        res
    }
}
