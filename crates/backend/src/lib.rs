use axum::{
    extract::Path,
    http::Method,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Json, Router,
};
use oci_distribution::{manifest::{self, OciManifest}, secrets::RegistryAuth, Client, Reference};
use plugy::{core::PluginLoader, runtime::Runtime};
use std::future::Future;
use std::pin::Pin;
use std::{net::SocketAddr, ops::Deref, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use types::{
    config::{Config, PluginConfig},
    FileEvent, Plugin, RenderEvent, UserInfo,
};

pub type BoxedPlugin = Box<dyn Plugin>;

#[derive(Debug, Clone)]
pub struct KasukuContext;

#[derive(Clone)]
pub struct KasukuRuntime {
    inner: Arc<Runtime<BoxedPlugin>>,
    config: Config,
}

impl Deref for KasukuRuntime {
    type Target = Arc<Runtime<BoxedPlugin>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl KasukuRuntime {
    pub async fn new(config: Config) -> Self {
        let runtime = Arc::new(Runtime::new().unwrap());
        for plugin in &config.plugins {
            runtime
                .load(BackendPlugin {
                    inner: plugin.clone(),
                })
                .await
                .unwrap();
        }
        KasukuRuntime {
            inner: runtime,
            config,
        }
    }
}

pub struct BackendPlugin {
    inner: PluginConfig,
}

#[allow(dead_code)]
pub struct PluginLock {
    uri: String,
    digest: String,
    manifest: OciManifest,
    version: String,
}

impl PluginLoader for BackendPlugin {
    fn name(&self) -> &'static str {
        Box::leak((self.inner.name.clone()).into_boxed_str())
    }
    fn load(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, anyhow::Error>>>> {
        let reference = self.inner.uri.clone();
        std::boxed::Box::pin(async move {

            // TODO: Make client reusable
            let mut client = Client::new(oci_distribution::client::ClientConfig {
                protocol: oci_distribution::client::ClientProtocol::Https,
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
        .map(|layer| layer)
        .expect("No data found");
    println!("Annotations: {:?}", image_content.annotations);
    image_content.data
}

pub async fn app<D: Send + Sync + Clone + 'static>(port: u16, data: D) {
    let app = Router::new()
        .route("/user", get(user_handler))
        .route("/api/v1/config", get(get_config))
        .route(
            "/api/v1/:namespace/:plugin/*path",
            get(getter).post(do_action),
        )
        .route("/api/v1/:namespace/:plugin", get(getter))
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

async fn getter(
    state: Extension<KasukuRuntime>,
    Path((_module, plugin, _path)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let plugin = state.get_plugin_by_name(&plugin).unwrap();
    let res = plugin.render(RenderEvent::default()).await;
    Html(res)
}

async fn do_action(
    state: Extension<KasukuRuntime>,
    Path((_module, plugin, _path)): Path<(String, String, String)>,
    Json(data): Json<serde_json::Value>,
) -> impl IntoResponse {
    let plugin = state.get_plugin_by_name(&plugin).unwrap();
    let _res = plugin
        .handle(FileEvent::CustomAction(serde_json::to_vec(&data).unwrap()))
        .await
        .unwrap();
    Html("res")
}

async fn get_config(state: Extension<KasukuRuntime>) -> impl IntoResponse {
    Json(state.config.clone())
}

async fn user_handler() -> impl IntoResponse {
    let user = UserInfo {
        id: 1,
        name: "Backend user".to_owned(),
    };
    Json(user)
}
