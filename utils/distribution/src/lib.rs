use oci_distribution::Client;
use oci_distribution::{
    manifest::{self, OciManifest},
    secrets::RegistryAuth,
    Reference,
};

#[allow(dead_code)]
pub struct PluginLock {
    uri: String,
    digest: String,
    manifest: OciManifest,
    version: String,
}

pub async fn load_package(reference: &str) -> Vec<u8> {
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
