mod mutation;
pub mod query;

use async_graphql::{http::GraphiQLSource, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use async_walkdir::{Filtering, WalkDir};
use axum::{
    http::Method,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Json, Router,
};
use kasuku_database::{prelude::Glue, KasukuDatabase};
use plugy::runtime::Runtime;

use std::{
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use types::{
    config::Config, BackendPlugin, Database, Emitter, Fetcher, GlobalContext, Plugin, UserInfo,
};
use xtra::Mailbox;

use crate::{mutation::MutationRoot, query::QueryRoot};

pub type BoxedPlugin = Box<dyn Plugin>;

#[derive(Debug, Clone)]
pub struct KasukuContext;

#[derive(Clone)]
pub struct KasukuRuntime {
    inner: Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>,
    config: Config,
    database: Arc<Mutex<Glue<KasukuDatabase>>>,
}

impl Deref for KasukuRuntime {
    type Target = Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl KasukuRuntime {
    pub async fn new(config: Config) -> Self {
        use futures::FutureExt;
        let ks = KasukuDatabase::open(&config.internals.database_path)
            .then(|res| async {
                if res.is_err() {
                    let new_db = KasukuDatabase::new(&config.internals.database_path)
                        .await
                        .unwrap();
                    new_db.save().await.unwrap();
                    return new_db;
                }
                res.unwrap()
            })
            .await;
        let mut db = Glue::new(ks);
        db.execute(
            "
                DROP TABLE subscriptions;
                DROP TABLE vaults;
                DROP TABLE entries;
                CREATE TABLE IF NOT EXISTS subscriptions (
                    plugin TEXT NOT NULL,
                    event TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    data TEXT
                );
                CREATE TABLE IF NOT EXISTS vaults (
                    name TEXT NOT NULL PRIMARY KEY,
                    mount TEXT NOT NULL,
                ); 
                CREATE TABLE IF NOT EXISTS entries (
                    path TEXT NOT NULL PRIMARY KEY,
                    vault TEXT NOT NULL,
                    last_modified INTEGER,
                    meta TEXT
                )",
        )
        .unwrap();

        let db = Arc::new(Mutex::new(db));
        let runtime = Runtime::new().unwrap();
        let runtime = runtime.context(Fetcher).context(Emitter).context(Database);
        for plugin in &config.plugins {
            let plugin: types::PluginWrapper<BackendPlugin, _> = runtime
                .load_with(BackendPlugin {
                    addr: xtra::spawn_tokio(GlobalContext::new(db.clone()), Mailbox::bounded(100)),
                    name: plugin.name.clone(),
                    uri: plugin.uri.clone(),
                })
                .await
                .unwrap();
            plugin.on_load(::types::Context::acquire()).await.unwrap();
        }

        for (vault, vault_config) in config.vaults.clone() {
            let mount = vault_config.mount.clone();
            // let db_clone = Arc::clone(&db);
            let _res = db.lock().as_mut().unwrap().execute(format!(
                "INSERT INTO vaults(name, mount) VALUES ('{vault}', '{}') ON CONFLICT(name) DO 
                         UPDATE SET mount = EXCLUDED.mount",
                mount.to_str().unwrap()
            ));

            // tokio::spawn(async move {
            use futures::stream::StreamExt;
            let mut entries = WalkDir::new(mount).filter(|entry| async move {
                if let Some(true) = entry
                    .path()
                    .file_name()
                    .map(|f| f.to_string_lossy().starts_with('.'))
                {
                    return Filtering::IgnoreDir;
                }
                if let Some(true) = entry
                    .path()
                    .file_name()
                    .map(|f| f.to_string_lossy().ends_with(".md"))
                {
                    return Filtering::Continue;
                }
                Filtering::Ignore
            });
            loop {
                match entries.next().await {
                    Some(Ok(entry)) => {
                        let _res = db.lock().unwrap().execute(format!(
                            "INSERT INTO entries(path, vault) VALUES('{}', '{}')",
                            entry.path().display(),
                            vault
                        ));
                    }
                    Some(Err(e)) => {
                        eprintln!("error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
            // });
        }
        KasukuRuntime {
            inner: Arc::new(runtime),
            config,
            database: db,
        }
    }
}

async fn graphiql() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/")
            .subscription_endpoint("/ws")
            .finish(),
    )
}

pub async fn app<D: Send + Sync + Clone + 'static>(port: u16, data: D) {
    // Setup graphql schema
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(data.clone())
        .finish();
    // Setup ws subscriptions

    let app = Router::new()
        .route("/user", get(user_handler))
        .route("/api/v1/config", get(get_config))
        .route(
            "/",
            get(graphiql).post_service(GraphQL::new(schema.clone())),
        )
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ]))
        .layer(Extension(data));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Backend is listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// async fn getter(
//     state: Extension<KasukuRuntime>,
//     Path((_module, plugin, _path)): Path<(String, String, String)>,
// ) -> impl IntoResponse {
//     let plugin = state.get_plugin_by_name(&plugin).unwrap();
//     let res = plugin.render(RenderEvent::default()).await;
//     Json(serde_json::from_slice::<serde_json::Value>(&res.0).unwrap())
// }

// async fn do_action(
//     state: Extension<KasukuRuntime>,
//     Path((_module, plugin, _path)): Path<(String, String, String)>,
//     Json(data): Json<serde_json::Value>,
// ) -> impl IntoResponse {
//     let plugin = state.get_plugin_by_name(&plugin).unwrap();
//     let _res = plugin
//         .handle(FileEvent::CustomAction(serde_json::to_vec(&data).unwrap()))
//         .await
//         .unwrap();
//     Html("res")
// }

async fn get_config(state: Extension<KasukuRuntime>) -> impl IntoResponse {
    // let dom = Dom::new();
    Json(state.config.clone())
}

async fn user_handler() -> impl IntoResponse {
    let user = UserInfo {
        id: 1,
        name: "Backend user".to_owned(),
    };
    Json(user)
}
