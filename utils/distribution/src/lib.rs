use std::collections::HashMap;

use oci_distribution::client::{Config, ImageLayer};
use oci_distribution::Client;
use oci_distribution::{
    manifest::{self, OciManifest},
    secrets::RegistryAuth,
    Reference,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginAnnotation {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub version: String,
    pub readme: String,
    pub dependencies: Vec<String>,
    pub vendor: String,
    pub wasm: Vec<u8>,
}

impl From<PluginAnnotation> for HashMap<String, String> {
    fn from(plugin: PluginAnnotation) -> Self {
        let mut hashmap = HashMap::new();
        hashmap.insert("org.opencontainers.image.title".to_string(), plugin.name);
        hashmap.insert("io.kasuku.plugin.icon".to_string(), plugin.icon);
        hashmap.insert(
            "org.opencontainers.image.description".to_string(),
            plugin.description,
        );
        hashmap.insert(
            "org.opencontainers.image.version".to_string(),
            plugin.version,
        );

        hashmap.insert("io.kasuku.plugin.version".to_string(), "0.1.0".to_string());
        hashmap.insert("org.opencontainers.image.url".to_string(), plugin.readme);
        let dependencies_str = plugin.dependencies.join(",");
        hashmap.insert(
            "io.kasuku.plugin.dependencies".to_string(),
            dependencies_str,
        );
        hashmap.insert("org.opencontainers.image.vendor".to_owned(), plugin.vendor);
        hashmap
    }
}

impl From<HashMap<String, String>> for PluginAnnotation {
    fn from(hashmap: HashMap<String, String>) -> Self {
        let name = hashmap
            .get("org.opencontainers.image.title")
            .cloned()
            .unwrap_or_default();
        let icon = hashmap
            .get("io.kasuku.plugin.icon")
            .cloned()
            .unwrap_or_default();
        let description = hashmap
            .get("org.opencontainers.image.description")
            .cloned()
            .unwrap_or_default();
        let version = hashmap
            .get("org.opencontainers.image.version")
            .cloned()
            .unwrap_or_default();
        let readme = hashmap
            .get("org.opencontainers.image.url")
            .cloned()
            .unwrap_or_default();

        let vendor = hashmap
            .get("org.opencontainers.image.vendor")
            .cloned()
            .unwrap_or_default();

        // Convert comma-separated String back to Vec<String>
        let dependencies = hashmap
            .get("io.kasuku.plugin.dependencies")
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        PluginAnnotation {
            name,
            icon,
            description,
            version,
            readme,
            dependencies,
            vendor,
            wasm: vec![],
        }
    }
}

#[allow(dead_code)]
pub struct PluginLock {
    uri: String,
    digest: String,
    manifest: OciManifest,
    version: String,
}

pub async fn load_package(reference: &str) -> PluginAnnotation {
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
    pull_wasm(&mut client, &reference).await
}

// Read the Plugin
async fn pull_wasm(client: &mut Client, reference: &Reference) -> PluginAnnotation {
    let image_data = client
        .pull(
            reference,
            &RegistryAuth::Anonymous,
            vec![manifest::WASM_LAYER_MEDIA_TYPE],
        )
        .await
        .expect("Cannot pull Wasm module");
    let layer = image_data.layers.into_iter().next().expect("No data found");
    let manifest = image_data.manifest.unwrap().annotations.unwrap_or_default();
    let mut plugin: PluginAnnotation = manifest.into();

    plugin.wasm = layer.data;
    plugin
}

pub async fn push_wasm(
    client: &mut Client,
    auth: &RegistryAuth,
    reference: &Reference,
    module: &str,
    annotations: PluginAnnotation,
) {
    let data = tokio::fs::read(module)
        .await
        .expect("Cannot read Wasm module from disk");

    let layers = vec![ImageLayer::new(
        data,
        manifest::WASM_LAYER_MEDIA_TYPE.to_string(),
        None,
    )];

    let config = Config {
        data: b"{}".to_vec(),
        media_type: manifest::WASM_CONFIG_MEDIA_TYPE.to_string(),
        annotations: None,
    };

    let image_manifest =
        manifest::OciImageManifest::build(&layers, &config, Some(annotations.into()));

    let response = client
        .push(&reference, &layers, config, &auth, Some(image_manifest))
        .await
        .map(|push_response| push_response.manifest_url)
        .expect("Cannot push Wasm module");

    println!("Wasm module successfully pushed {:?}", response);
}
