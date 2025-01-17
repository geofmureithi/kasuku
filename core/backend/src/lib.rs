mod indexer;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use context::{BackendPlugin, Context, Database, Debugger, Emitter, Fetcher, GlobalContext};
use distribution::PluginAnnotation;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use indexer::run_indexer;
use interface::{Plugin, PluginWrapper};
use kasuku_database::KasukuDatabase;
use markdown::{IsMatched, MarkdownEvent, MarkdownFile};
use plugy::runtime::Runtime;
use serde::{
    de::{self, DeserializeOwned},
    Deserialize, Serialize,
};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

use std::{fs, net::SocketAddr, ops::Deref, sync::Arc};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use types::{config::Config, UserInfo};

pub type BoxedPlugin = Box<dyn Plugin>;

#[derive(Debug, Clone)]
pub struct KasukuContext;

#[derive(Clone)]
pub struct KasukuRuntime {
    inner: Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>,
    config: Config,
    database: KasukuDatabase,
    context: Context,
}

impl Deref for KasukuRuntime {
    type Target = Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl KasukuRuntime {
    pub async fn new(config: &Config) -> Result<Self, types::Error> {
        use futures::FutureExt;
        // let kasuku_database = KasukuDatabase::new(&config.internals.database_path)
        let kasuku_database = KasukuDatabase::new_from_memory()
            .then(|res| async {
                if res.is_err() {
                    let new_db = KasukuDatabase::new(&config.internals.database_path)
                        .await
                        .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
                    return Ok(new_db);
                }
                res.map_err(|err| types::Error::DatabaseError(err.to_string()))
            })
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
        kasuku_database
            .query_raw("PRAGMA journal_mode = WAL;")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA temp_store = 2")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA synchronous = NORMAL")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA cache_size = 64000")
            .await
            .unwrap();

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS plugins (
                    name TEXT NOT NULL PRIMARY KEY,
                    uri TEXT NOT NULL,
                    data TEXT
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS subscriptions (
                    event TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    plugin TEXT NOT NULL,
                    data TEXT,
                    FOREIGN KEY(plugin) REFERENCES plugins(name)
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS widgets (
                    name TEXT NOT NULL,
                    type TEXT NOT NULL,
                    plugin TEXT NOT NULL,
                    data TEXT,
                    FOREIGN KEY(plugin) REFERENCES plugins(name)
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS vaults (
                    name TEXT NOT NULL PRIMARY KEY,
                    mount TEXT NOT NULL
                ); 
                CREATE TABLE IF NOT EXISTS entries (
                    path TEXT NOT NULL PRIMARY KEY,
                    vault TEXT NOT NULL,
                    last_modified INTEGER,
                    meta TEXT,
                    FOREIGN KEY(vault) REFERENCES vaults(name),
                );",
            )
            .await
            .unwrap();

        let ctx_actor = Arc::new(GlobalContext::new(kasuku_database.clone()));
        let runtime = Runtime::new().unwrap();
        let runtime = runtime
            .context(Fetcher)
            .context(Emitter)
            .context(Database)
            .context(Debugger);
        for plugin in &config.plugins {
            let annotation = PluginAnnotation {
                wasm: fs::read(&plugin.uri).unwrap(),
                ..Default::default()
            };
            let plugin: PluginWrapper<BackendPlugin, _> = runtime
                .load_with(BackendPlugin {
                    addr: ctx_actor.clone(),
                    name: plugin.name.clone(),
                    uri: plugin.uri.clone(),
                    meta: annotation,
                    // meta: distribution::load_package(&plugin.uri)
                    //     .await
                    //     .map_err(|err| types::Error::PluginError(err.to_string()))?,
                })
                .await
                .unwrap();

            plugin.on_load(&mut Context).await?;
        }

        let act = ctx_actor.clone();
        let mv_act = ctx_actor.clone();
        for (vault, vault_config) in config.vaults.clone() {
            let mount = vault_config.mount.clone();
            let _res = act
                .database
                .execute(format!(
                    "INSERT INTO vaults(name, mount) VALUES ('{vault}', '{}')",
                    mount.to_str().ok_or(types::Error::Serialization(
                        "Invalid vault name".to_string()
                    ))?
                ))
                .await
                .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
            let mv_act = mv_act.clone();
            tokio::spawn(async move {
                run_indexer(mv_act, vault, vault_config).await.unwrap();
            });
        }
        Ok(KasukuRuntime {
            inner: Arc::new(runtime),
            config: config.clone(),
            database: kasuku_database,
            context: Context,
        })
    }
}

pub async fn app(port: u16, data: KasukuRuntime) {
    let app = Router::new()
        .route("/user", get(user_handler))
        .route("/api/v1/config", get(get_config))
        .route(
            "/api/v1/vaults/{vault}/file/{*filename}",
            get(get_file).put(update_file),
        )
        .route("/api/v1/vaults/{vault}", get(list_files))
        .fallback_service(
            ServeDir::new("static").not_found_service(ServeFile::new("static/index.html")),
        )
        .with_state(data);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

async fn get_config(state: State<KasukuRuntime>) -> impl IntoResponse {
    Json(state.config.clone())
}

async fn user_handler() -> impl IntoResponse {
    let user = UserInfo {
        id: 1,
        name: "Backend user".to_owned(),
    };
    Json(user)
}

/// Payload for updating a file (we expect a JSON body with a "content" field)
#[derive(Debug, Deserialize)]
struct UpdateFile {
    content: String,
}

/// Handler that overwrites the contents of a specific `.md` file.
/// - `filename` is the name of the file
/// - Expects a JSON body with `{ "content": "...new markdown contents..." }`
async fn update_file(
    Path(filename): Path<String>,
    State(_state): State<KasukuRuntime>,
    Json(payload): Json<UpdateFile>,
) -> impl IntoResponse {
    // let file_path = state.base_dir.join(&filename);
    let file_path = filename;

    // Create the file if it doesn't exist, or truncate it if it does.
    match tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_path)
        .await
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(payload.content.as_bytes()).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write file: {}", e),
                )
                    .into_response();
            }
            (StatusCode::OK, "File updated successfully").into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create or open file: {}", e),
        )
            .into_response(),
    }
}

async fn get_file(
    Path((_vault, filename)): Path<(String, String)>,
    State(mut runtime): State<KasukuRuntime>,
) -> impl IntoResponse {
    let tx = runtime.database.transaction().await;
    #[derive(Debug, Deserialize, Clone)]
    struct Subscription {
        #[serde(deserialize_with = "deserialize_json_string")]
        data: MarkdownEvent,
        plugin: String,
    }

    fn deserialize_json_string<'de, D, T: DeserializeOwned>(deserializer: D) -> Result<T, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        serde_json::from_str(&s).map_err(de::Error::custom)
    }

    let md = tokio::fs::read_to_string(&filename).await.unwrap();
    let file = markdown::parse(&md).unwrap();

    let subscriptions: Vec<Subscription> = tx
        .query(
            "SELECT s.data, s.plugin FROM subscriptions s WHERE event = 'markdown::MarkdownEvent' ORDER BY s.rowid ASC",
        )
        .await
        .unwrap();

    let plugins: Vec<String> = file
        .iter()
        .flat_map(move |event| {
            subscriptions
                .iter()
                .filter(move |filter| filter.data.is_matched(event).unwrap())
                .map(|c| c.plugin.clone())
                .collect::<Vec<_>>()
        })
        .collect::<std::collections::HashSet<String>>()
        .into_iter()
        .collect::<Vec<String>>();
    let mut md: ::types::File = file.try_into().unwrap();
    for plugin in plugins.iter() {
        let plugin: PluginWrapper<BackendPlugin, _> = runtime.get_plugin_by_name(plugin).unwrap();
        md = plugin.process_file(&mut runtime.context, md).await.unwrap();
    }
    tx.commit().await;

    let mut buf = String::new();
    let md: MarkdownFile = (&md).try_into().unwrap();

    markdown::cmark::html::push_html(&mut buf, md.events.into_iter());

    Html(buf)
}

/// Handler that lists all `.md` files inside `base_dir`.
async fn list_files(
    Path(vault): Path<String>,
    State(state): State<KasukuRuntime>,
) -> impl IntoResponse {
    let mut files = Vec::new();

    #[derive(Debug, Serialize, Deserialize)]
    struct MarkdownFile {
        filename: String,
        path: String,
    }
    // Recursively walk through base_dir to find .md files
    for entry in WalkDir::new(&state.config.vaults.get(&vault).unwrap().mount) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Only consider regular files
        if entry.file_type().is_file() {
            let path = entry.path();
            // Check if it ends in .md
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        files.push(MarkdownFile {
                            filename: filename.to_string(),
                            path: path.to_str().unwrap_or("").replace("\\", "/"), // handle Windows backslashes
                        });
                    }
                }
            }
        }
    }

    Json(files)
}

pub fn read_config() -> Config {
    let config: Config = Figment::new()
        .merge(Toml::file("Kasuku.toml"))
        .merge(Env::prefixed("KASUKU_"))
        .extract()
        .unwrap();
    config
}
