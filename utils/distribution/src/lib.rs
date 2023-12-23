use std::collections::HashMap;

use oci_distribution::client::{ImageLayer, PushResponse};
use oci_distribution::errors::OciDistributionError;
use oci_distribution::Client;
use oci_distribution::{
    manifest::{self, OciManifest},
    secrets::RegistryAuth,
    Reference,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
    // authors: Vec<String>,
    pub description: String,
    pub metadata: Meta,
    pub version: String,
    pub license: Option<String>,
    pub repository: Option<String>,
}
#[derive(Deserialize)]
pub struct Meta {
    pub kasuku: Kasuku,
}

#[derive(Deserialize)]
pub struct Kasuku {
    pub name: String,
    pub readme: String,
    pub icon: String,
    pub identifier: String,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct Config {
    pub package: Package,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginAnnotation {
    pub name: String,
    pub identifier: String,
    pub icon: String,
    pub description: String,
    pub version: String,
    pub readme: String,
    pub dependencies: Vec<String>,
    pub vendor: String,
    pub licenses: Option<String>,
    pub source: Option<String>,
    pub wasm: Vec<u8>,
}

impl From<PluginAnnotation> for HashMap<String, String> {
    fn from(plugin: PluginAnnotation) -> Self {
        let mut hashmap = HashMap::new();
        hashmap.insert("org.opencontainers.image.title".to_string(), plugin.name);
        hashmap.insert("io.kasuku.plugin.identifier".to_string(), plugin.identifier);
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
        if plugin.source.is_some() {
            hashmap.insert(
                "org.opencontainers.image.source".to_owned(),
                plugin.source.unwrap(),
            );
        }

        if plugin.licenses.is_some() {
            hashmap.insert(
                "org.opencontainers.image.licenses".to_owned(),
                plugin.licenses.unwrap(),
            );
        }

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
        let identifier = hashmap.get("io.kasuku.plugin.identifier").cloned().expect(
            &("Invalid plugin config. Missing plugin identifier key `io.kasuku.plugin.identifier` in "
                .to_owned() + &name),
        );
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
        let licenses = hashmap.get("org.opencontainers.image.licenses").cloned();

        let source = hashmap.get("org.opencontainers.image.source").cloned();

        // Convert comma-separated String back to Vec<String>
        let dependencies = hashmap
            .get("io.kasuku.plugin.dependencies")
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        PluginAnnotation {
            name,
            identifier,
            icon,
            description,
            version,
            readme,
            dependencies,
            vendor,
            licenses,
            source,
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

pub async fn load_package(reference: &str) -> Result<PluginAnnotation, OciDistributionError> {
    let mut client = Client::new(oci_distribution::client::ClientConfig {
        protocol: oci_distribution::client::ClientProtocol::Http,
        // accept_invalid_certificates: true,
        ..Default::default()
    });
    let reference: Reference = reference.parse().map_err(|e| {
        OciDistributionError::ManifestParsingError(format!("Could not read the reference {e}"))
    })?;
    let (_manifest, _) = client
        .pull_manifest(&reference, &RegistryAuth::Anonymous)
        .await?;
    pull_wasm(&mut client, &reference).await
}

// Read the Plugin
pub async fn pull_wasm(
    client: &mut Client,
    reference: &Reference,
) -> Result<PluginAnnotation, OciDistributionError> {
    let image_data = client
        .pull(
            reference,
            &RegistryAuth::Anonymous,
            vec![manifest::WASM_LAYER_MEDIA_TYPE],
        )
        .await?;
    let layer =
        image_data
            .layers
            .into_iter()
            .next()
            .ok_or(OciDistributionError::ManifestParsingError(
                "Could not find required layers".to_owned(),
            ))?;
    let manifest = image_data
        .manifest
        .ok_or(OciDistributionError::ManifestParsingError(
            "Could not find required annotations".to_owned(),
        ))?
        .annotations
        .ok_or(OciDistributionError::ManifestParsingError(
            "Could not find required annotations".to_owned(),
        ))?;
    let mut plugin: PluginAnnotation = manifest.into();

    plugin.wasm = layer.data;
    Ok(plugin)
}

pub async fn push_wasm(
    client: &mut Client,
    auth: &RegistryAuth,
    reference: &Reference,
    module: &str,
    annotations: PluginAnnotation,
) -> Result<PushResponse, OciDistributionError> {
    let data = tokio::fs::read(module)
        .await
        .expect("Cannot read Wasm module from disk");

    let layers = vec![ImageLayer::new(
        data,
        manifest::WASM_LAYER_MEDIA_TYPE.to_string(),
        None,
    )];

    let config = oci_distribution::client::Config {
        data: b"{}".to_vec(),
        media_type: manifest::WASM_CONFIG_MEDIA_TYPE.to_string(),
        annotations: None,
    };

    let image_manifest =
        manifest::OciImageManifest::build(&layers, &config, Some(annotations.into()));

    client
        .push(reference, &layers, config, auth, Some(image_manifest))
        .await
}
