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
use context::{BackendPlugin, Context, Database, Debugger, Emitter, Fetcher, GlobalContext, Query};
use interface::{Plugin, PluginWrapper};
use kasuku_database::{prelude::Glue, KasukuDatabase};
use plugy::runtime::Runtime;

use std::{
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tower_http::cors::{Any, CorsLayer};
use types::{config::Config, UserInfo};
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
    pub async fn new(config: Config) -> Result<Self, types::Error> {
        use futures::FutureExt;
        let kasuku_database = KasukuDatabase::open(&config.internals.database_path)
            .then(|res| async {
                if res.is_err() {
                    let new_db = KasukuDatabase::new(&config.internals.database_path)
                        .await
                        .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
                    new_db
                        .save()
                        .await
                        .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
                    return Ok(new_db);
                }
                res.map_err(|err| types::Error::DatabaseError(err.to_string()))
            })
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
        let mut glue_db = Glue::new(kasuku_database);
        glue_db
            .execute(
                "DROP TABLE IF EXISTS subscriptions;
                DROP TABLE IF EXISTS vaults;
                DROP TABLE IF EXISTS entries;
                DROP TABLE IF EXISTS tasks;
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
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        let db = Arc::new(Mutex::new(glue_db));
        let ctx_actor = xtra::spawn_tokio(GlobalContext::new(db.clone()), Mailbox::bounded(100));
        let runtime = Runtime::new().unwrap();
        let runtime = runtime
            .context(Fetcher)
            .context(Emitter)
            .context(Database)
            .context(Debugger);
        for plugin in &config.plugins {
            let plugin: PluginWrapper<BackendPlugin, _> = runtime
                .load_with(BackendPlugin {
                    addr: ctx_actor.clone(),
                    name: plugin.name.clone(),
                    uri: plugin.uri.clone(),
                    meta: distribution::load_package(&plugin.uri)
                        .await
                        .map_err(|err| types::Error::PluginError(err.to_string()))?,
                })
                .await
                .unwrap();
            plugin.on_load(Context::acquire()).await?;
        }

        let act = ctx_actor.clone();
        let mv_act = ctx_actor.clone();
        for (vault, vault_config) in config.vaults.clone() {
            let mount = vault_config.mount.clone();
            let _res = act
                .send(Query(format!(
                    "INSERT INTO vaults(name, mount) VALUES ('{vault}', '{}')",
                    mount.to_str().ok_or(types::Error::Serialization(
                        "Invalid vault name".to_string()
                    ))?
                )))
                .await
                // .map_err(|err| types::Error::DatabaseError(err.to_string()))?
                .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
            let mv_act = mv_act.clone();
            tokio::spawn(async move {
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
                let mv_act = mv_act.clone();
                loop {
                    match entries.next().await {
                        Some(Ok(entry)) => {
                            let _res = mv_act
                                .send(Query(format!(
                                    "INSERT INTO entries(path, vault) VALUES('{}', '{}')",
                                    entry.path().display(),
                                    vault
                                )))
                                .await;
                        }
                        Some(Err(e)) => {
                            eprintln!("error: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
            });
        }
        Ok(KasukuRuntime {
            inner: Arc::new(runtime),
            config,
            database: db,
        })
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
