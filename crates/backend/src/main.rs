use backend::KasukuRuntime;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use types::config::Config;

#[tokio::main]
async fn main() {
    let config: Config = Figment::new()
        .merge(Toml::file("Kasuku.toml"))
        .merge(Env::prefixed("KASUKU_"))
        .extract()
        .unwrap();
    config.validate().expect("Invalid config");
    let runtime = KasukuRuntime::new(config).await;
    let _app = backend::app(3001, runtime).await;
}

