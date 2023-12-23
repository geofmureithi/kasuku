use distribution::{push_wasm, Config, PluginAnnotation};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use oci_distribution::{secrets::RegistryAuth, Client};

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
        identifier: config.package.metadata.kasuku.identifier,
        licenses: config.package.license,
        source: config.package.repository,
        dependencies: config
            .package
            .metadata
            .kasuku
            .dependencies
            .unwrap_or_default(),
        wasm: vec![],
    };
    let res = push_wasm(
        &mut client,
        &RegistryAuth::Anonymous,
        &reference,
        &module,
        annotations,
    )
    .await
    .expect("Could not push plugin to oci registry");

    println!("Image successfully pushed: Manifest: {}", res.manifest_url);
}
