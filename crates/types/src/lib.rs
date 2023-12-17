pub mod config;
pub mod node;
#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;
use hirola::prelude::EventListener;
#[cfg(not(target_arch = "wasm32"))]
use kasuku_database::{prelude::Glue, KasukuDatabase};
use node::Node;
#[cfg(not(target_arch = "wasm32"))]
use oci_distribution::Client;
pub use pulldown_cmark::{Alignment, CowStr, Event as PulldownEvent, HeadingLevel, LinkType};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use sqlparser::{ast::Statement, dialect::PostgreSqlDialect, parser::Parser};
use std::{
    collections::HashMap,
    fmt::{self},
    future::Future,
    marker::PhantomData,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
};
#[cfg(not(target_arch = "wasm32"))]
use xtra::Address;
#[cfg(not(target_arch = "wasm32"))]
use xtra::Handler;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct MarkdownFile<D, C> {
    pub frontmatter: D,
    pub content: C,
}

impl MarkdownFile<Vec<u8>, Vec<u8>> {
    pub fn get_contents<'a: 'de, 'de, C: Deserialize<'de>>(&'a self) -> C {
        serde_json::from_slice(&self.content).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarkdownEvent {
    #[serde(rename = "MarkdownEvent::Tag")]
    Tag(Tag),
    Text(String),
    InlineCode(String),
    FootNote(String),
    TaskList,
}

/// Codeblock kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CodeBlockKind {
    Indented,
    Fenced(String),
}

impl CodeBlockKind {
    pub fn is_indented(&self) -> bool {
        matches!(*self, CodeBlockKind::Indented)
    }

    pub fn is_fenced(&self) -> bool {
        matches!(*self, CodeBlockKind::Fenced(_))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Tag {
    /// A paragraph of text and other inline elements.
    Paragraph,

    /// A heading. The first field indicates the level of the heading,
    /// the second the fragment identifier, and the third the classes.
    Heading(HeadingLevel, Option<String>, Vec<String>),

    BlockQuote,
    /// A code block.
    CodeBlock(CodeBlockKind),

    List,
    /// A list item.
    Item,
    /// A footnote definition. The value contained is the footnote's label by which it can
    /// be referred to.
    FootnoteDefinition(String),

    /// A table. Contains a vector describing the text-alignment for each of its columns.
    Table(Vec<Alignment>),
    /// A table header. Contains only `TableCell`s. Note that the table body starts immediately
    /// after the closure of the `TableHead` tag. There is no `TableBody` tag.
    TableHead,
    /// A table row. Is used both for header rows as body rows. Contains only `TableCell`s.
    TableRow,
    TableCell,

    // span-level tags
    Emphasis,
    Strong,
    Strikethrough,

    /// A link. The first field is the link type, the second the destination URL and the third is a title.
    Link(LinkType, String, String),

    /// An image. The first field is the link type, the second the destination URL and the third is a title.
    Image(LinkType, String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum File {
    Markdown(MarkdownFile<Vec<u8>, Vec<u8>>),
}

#[cfg(not(target_arch = "wasm32"))]
impl File {
    pub async fn read<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.is_dir() {
            panic!("Trying read dir")
        }
        match path.extension().unwrap().to_str().unwrap() {
            "md" => {
                let file = tokio::fs::read_to_string(path).await.unwrap();
                let options = pulldown_cmark::Options::all();
                let parser = pulldown_cmark::Parser::new_ext(&file, options);
                let events: Vec<pulldown_cmark::Event<'_>> = parser.collect();
                Ok(File::Markdown(MarkdownFile {
                    frontmatter: Default::default(),
                    content: serde_json::to_vec(&events).unwrap(),
                }))
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub data: Emit<Vec<u8>>,
    pub path: String,
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
    title: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entity {
    Task(Task),
    Event,
    // Reminder,
}

#[derive(Debug, Serialize, Deserialize, thiserror::Error)]
pub enum Error {
    #[error("ser/de failed: `{0}`")]
    Serde(#[from] serde_error::Error),
    #[error("a render was requested but cannot be completed")]
    InvalidRender,
    #[error("parse sql error")]
    SqlParser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rsx(pub Vec<u8>);

impl TryFrom<Node> for Rsx {
    type Error = Error;

    fn try_from(dom: Node) -> Result<Self, Self::Error> {
        Ok(Rsx(
            serde_json::to_vec(&dom).map_err(|e| Error::Serde(serde_error::Error::new(&e)))?
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewType {
    SideBar,
    Dashboard,
    FloatingMenu,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(xtra::Actor)]
pub struct GlobalContext {
    database: Arc<Mutex<Glue<KasukuDatabase>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl GlobalContext {
    pub fn new(database: Arc<Mutex<Glue<KasukuDatabase>>>) -> Self {
        GlobalContext { database }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fetcher;

#[plugy::macros::context(data = BackendPlugin)]
impl Fetcher {
    pub async fn fetch(
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        url: String,
    ) -> String {
        url
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emitter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    event: String,
    event_type: String,
    data: String,
}

#[plugy::macros::context(data = BackendPlugin)]
impl Emitter {
    pub async fn subscribe(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        subscription: crate::Subscription,
    ) -> Result<String, ()> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        let plugin = &caller.data().as_ref().unwrap().plugin.name;
        let Subscription {
            event,
            event_type,
            data,
        } = subscription;
        let sql =
            format!("INSERT INTO subscriptions(plugin, event, event_type, data) VALUES('{plugin}','{event}','{event_type}','{data}');");
        addr.send(Query(sql)).await.map_err(|_| ())
    }

    pub async fn emit(
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        url: String,
    ) -> String {
        url
    }
}

pub struct Database;

#[plugy::macros::context(data = BackendPlugin)]
impl Database {
    pub async fn query(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        sql: String,
    ) -> String {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        addr.send(Query(sql)).await.unwrap()
    }
}

pub struct Context;

#[cfg(not(target_arch = "wasm32"))]
impl Context {
    pub fn acquire() -> &'static mut Self {
        Box::leak(Box::new(Context))
    }
}

impl Serialize for &Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_unit()
    }
}

impl Serialize for &mut Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de, 'a> Deserialize<'de> for &'a mut Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'de, 'a> Visitor<'de> for ContextVisitor<'a> {
            type Value = &'a mut Context;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("&mut Context")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Box::leak(Box::new(Context)))
            }
        }
        deserializer.deserialize_unit(ContextVisitor(PhantomData))
    }
}

impl<'de, 'a> Deserialize<'de> for &'a Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'de, 'a> Visitor<'de> for ContextVisitor<'a> {
            type Value = &'a Context;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("&Context")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Box::leak(Box::new(Context)))
            }
        }
        deserializer.deserialize_unit(ContextVisitor(PhantomData))
    }
}

pub struct Core;

impl Plugin for Core {
    fn on_load(&self, _ctx: &mut Context) -> Result<(), Error> {
        unreachable!()
    }
}

impl PluginEvent for MarkdownEvent {
    type Plugin = Core;
}

impl Context {
    pub fn fetch(&self, url: &str) -> String {
        fetcher::sync::Fetcher::fetch(url.to_string())
    }
    pub fn register_view<W>(&mut self, _view: ViewType, _widget: W) {
        todo!()
    }

    pub fn subscribe<E: PluginEvent + Serialize>(&mut self, event: &E) -> Result<(), String> {
        let value = serde_json::to_value(event).unwrap();
        let raw_event_type: Vec<&str> = value["type"].as_str().unwrap().split("::").collect();
        let event = raw_event_type.first().unwrap().to_string();
        let event_type = raw_event_type.get(1).unwrap().to_string();
        let data = serde_json::to_string(&value).unwrap();
        emitter::sync::Emitter::subscribe(Subscription {
            data,
            event,
            event_type,
        })
        .unwrap();
        Ok(())
    }

    pub fn query(&self, sql: &str) -> String {
        let req = parse(sql).unwrap();
        if req
            .iter()
            .any(|r| !matches!(r, sqlparser::ast::Statement::Query(_)))
        {
            panic!("Tried to modify database in non-mutable context. Please use execute()")
        }
        let res = database::sync::Database::query(sql.to_owned());
        serde_json::to_string(&res).unwrap()
    }

    pub fn execute(&mut self, sql: &str) -> String {
        let res = database::sync::Database::query(sql.to_owned());
        serde_json::to_string(&res).unwrap()
    }

    pub fn add_task(&self, task: &Task) {
        let text = &task.title;
        database::sync::Database::query(format!("INSERT INTO tasks(text) VALUES ('{text}') "));
    }
}

#[allow(unused_variables)]
#[plugy::macros::plugin]
pub trait Plugin: Send + Sync {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error>;

    fn process_file(&self, ctx: &Context, file: File) -> Result<File, Error> {
        Ok(file)
    }

    fn on_event(&self, ctx: &Context, ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn on_unload(&self, ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, view: Event) -> Result<Rsx, Error> {
        Err(Error::InvalidRender)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emit<Ev> {
    pub data: Ev,
    pub r#type: String,
}

pub fn emit<Ev: Serialize + PluginEvent>(data: &Ev) -> CompactEvent {
    CompactEvent::Plugin {
        plugin: std::any::type_name::<Ev::Plugin>().to_owned(),
        data: Emit {
            data: serde_json::to_vec(data).unwrap(),
            r#type: std::any::type_name::<Ev>().to_owned(),
        },
    }
}

impl EventListener for Node {
    type Handler = CompactEvent;
    fn event(&self, _name: &str, _handler: Self::Handler) {
        // self.event_handlers.borrow_mut().push(closure);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactEvent {
    Plugin { plugin: String, data: Emit<Vec<u8>> },
    Global(Emit<Vec<u8>>),
    Local(Emit<Vec<u8>>),
}

pub trait PluginEvent {
    type Plugin: Plugin;
}

#[cfg(not(target_arch = "wasm32"))]
pub type Addr = Address<GlobalContext>;

#[cfg(not(target_arch = "wasm32"))]
struct Query(String);

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Handler<Query> for GlobalContext {
    type Return = String;

    async fn handle(&mut self, sql: Query, _ctx: &mut xtra::Context<Self>) -> String {
        let conn = &mut self.database;
        let mut res = conn.lock().unwrap();
        let res = res.execute(sql.0).unwrap();
        serde_json::to_string(&res).unwrap()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct BackendPlugin {
    pub addr: Addr,
    pub name: String,
    pub uri: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<BackendPlugin> for plugy::runtime::Plugin<BackendPlugin> {
    fn from(val: BackendPlugin) -> Self {
        Self {
            name: val.name().to_string(),
            data: val,
            plugin_type: std::any::type_name::<BackendPlugin>().to_string(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use oci_distribution::{
    manifest::{self, OciManifest},
    secrets::RegistryAuth,
    Reference,
};
#[cfg(not(target_arch = "wasm32"))]
use plugy::core::PluginLoader;

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub struct PluginLock {
    uri: String,
    digest: String,
    manifest: OciManifest,
    version: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl PluginLoader for BackendPlugin {
    fn name(&self) -> &'static str {
        Box::leak((self.name.clone()).into_boxed_str())
    }
    fn bytes(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, anyhow::Error>>>> {
        let reference = self.uri.clone();
        std::boxed::Box::pin(async move {
            // TODO: Make client reusable
            let mut client = Client::new(oci_distribution::client::ClientConfig {
                protocol: oci_distribution::client::ClientProtocol::Http,
                // accept_invalid_certificates: true,
                ..Default::default()
            });
            let reference: Reference = reference.parse().expect("Not a valid image reference");
            let (_manifest, _) = client
                .pull_manifest(&reference, &RegistryAuth::Anonymous)
                .await
                .expect("Cannot pull manifest");
            let wasm = pull_wasm(&mut client, &reference).await;
            Ok(wasm)
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
// Read the Plugin
async fn pull_wasm(client: &mut Client, reference: &Reference) -> Vec<u8> {
    let image_content = client
        .pull(
            reference,
            &RegistryAuth::Anonymous,
            vec![manifest::WASM_LAYER_MEDIA_TYPE],
        )
        .await
        .expect("Cannot pull Wasm module")
        .layers
        .into_iter()
        .next()
        .expect("No data found");
    println!("Annotations: {:?}", image_content.annotations);
    image_content.data
}

const DIALECT: PostgreSqlDialect = PostgreSqlDialect {};

pub fn parse<Sql: AsRef<str>>(sql: Sql) -> Result<Vec<Statement>, Error> {
    Parser::parse_sql(&DIALECT, sql.as_ref()).map_err(|_e| Error::SqlParser)
}
