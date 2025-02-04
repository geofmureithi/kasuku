pub mod indexer;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use context::BackendPlugin;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use interface::PluginWrapper;
use markdown::{IsMatched, MarkdownEvent, MarkdownFile};
use runtime::KasukuRuntime;
use serde::{
    de::{self, DeserializeOwned},
    Deserialize,
};
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use types::{config::Config, FilePath, UserInfo};

pub async fn app(runtime: &KasukuRuntime) {
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
        .with_state(runtime.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], runtime.state.config.server.port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

async fn get_config(runtime: State<KasukuRuntime>) -> impl IntoResponse {
    Json(runtime.state.config.clone())
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
    State(runtime): State<KasukuRuntime>,
) -> impl IntoResponse {
    let tx = runtime.state.database.transaction().await;
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
        let mut ctx = runtime.state.context.write().await;
        md = plugin.process_file(&mut ctx, md).await.unwrap();
    }
    tx.commit().await;

    let mut buf = String::new();
    let md: MarkdownFile = (&md).try_into().unwrap();

    markdown::cmark::html::push_html(&mut buf, md.events.into_iter());

    Html(buf)
}

/// Handler that lists all `.md` files inside the vault.
async fn list_files(
    Path(vault): Path<String>,
    State(runtime): State<KasukuRuntime>,
) -> impl IntoResponse {
    let files: Vec<FilePath> = runtime
        .state
        .database
        .query_params("SELECT * FROM entries where vault = :1", [vault])
        .await
        .unwrap();

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
