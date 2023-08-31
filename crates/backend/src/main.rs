use backend::KasukuRuntime;
use types::test_config;

#[tokio::main]
async fn main() {
    let runtime = KasukuRuntime::new(test_config()).await;
    let _app = backend::app(3001, runtime).await;
}
