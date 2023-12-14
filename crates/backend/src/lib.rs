mod mutation;
pub mod query;

use async_graphql::{http::GraphiQLSource, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    http::Method,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Json, Router,
};
use plugy::runtime::Runtime;

use std::{net::SocketAddr, ops::Deref, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use types::{
    config::Config, BackendPlugin, Emitter, Fetcher, GlobalContext, Plugin, PluginContext, UserInfo,
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
}

impl Deref for KasukuRuntime {
    type Target = Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl KasukuRuntime {
    pub async fn new(config: Config) -> Self {
        let runtime = Runtime::new().unwrap();
        let runtime = runtime.context(Fetcher).context(Emitter);
        for plugin in &config.plugins {
            let plugin: types::PluginWrapper<BackendPlugin, _> = runtime
                .load_with(BackendPlugin {
                    addr: xtra::spawn_tokio(
                        PluginContext {
                            inner: GlobalContext::default(),
                        },
                        Mailbox::unbounded(),
                    ),
                    name: plugin.name.clone(),
                    uri: plugin.uri.clone(),
                })
                .await
                .unwrap();
            plugin.on_load(::types::Context::acquire()).await.unwrap();
        }
        KasukuRuntime {
            inner: Arc::new(runtime),
            config,
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
        // .route(
        //     "/api/v1/:namespace/:plugin/*path",
        //     get(getter).post(do_action),
        // )
        // .route("/api/v1/:namespace/:plugin", get(getter))
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
