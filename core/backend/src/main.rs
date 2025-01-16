use backend::{read_config, KasukuRuntime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    use tracing_subscriber::EnvFilter;

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_new("info").unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let config = read_config();
    // config.validate().expect("Invalid config");
    let runtime = KasukuRuntime::new(&config)
        .await
        .expect("Could not start the runtime");
    let _app = backend::app(config.server.port, runtime).await;
}
