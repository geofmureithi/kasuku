use distribution::{push_wasm, PluginAnnotation};
use oci_distribution::{secrets::RegistryAuth, Client};
use serde::Deserialize;

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};

#[derive(Deserialize)]
struct Package {
    name: String,
    // authors: Vec<String>,
    description: String,
    metadata: Meta,
    version: String,
}
#[derive(Deserialize)]
struct Meta {
    kasuku: Kasuku,
}

#[derive(Deserialize)]
struct Kasuku {
    name: String,
    readme: String,
    icon: String,
    // compatibility: String,
}

#[derive(Deserialize)]
struct Config {
    package: Package,
}

#[tokio::main]
pub async fn main() {
    let mut args = std::env::args().collect::<Vec<_>>().into_iter();
    let path = args.nth(1).expect("missing the path to cargo.toml");
    let config: Config = Figment::new()
        .merge(Toml::file(path + "/Cargo.toml"))
        .merge(Env::prefixed("CARGO_"))
        .extract()
        .unwrap();
    let module = config.package.name;
    let mut client = Client::new(oci_distribution::client::ClientConfig {
        protocol: oci_distribution::client::ClientProtocol::Http,
        // accept_invalid_certificates: true,
        ..Default::default()
    });
    let version = &config.package.version;
    let reference = format!("localhost:5000/{module}:{version}")
        .parse()
        .unwrap();
    let module = format!("target/wasm32-unknown-unknown/release/{module}.wasm");
    let annotations = PluginAnnotation {
        name: config.package.metadata.kasuku.name,
        icon: config.package.metadata.kasuku.icon,
        readme: config.package.metadata.kasuku.readme,
        description: config.package.description,
        version: config.package.version,
        vendor: "Kasuku Core".to_owned(),
        ..Default::default()
    };
    push_wasm(
        &mut client,
        &RegistryAuth::Anonymous,
        &reference,
        &module,
        annotations,
    )
    .await;
}
