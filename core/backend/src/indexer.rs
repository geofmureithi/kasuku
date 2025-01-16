use context::GlobalContext;
use tokio::task::block_in_place;
use types::config::VaultConfig;

use futures::{
    channel::mpsc::{channel, Receiver},
    executor::block_on,
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;

pub async fn run_indexer(
    _ctx: Arc<GlobalContext>,
    _vault: String,
    cfg: VaultConfig,
) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&cfg.mount.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => println!("changed: {:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
    Ok(())
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            block_in_place(|| {
                block_on(async {
                    tx.send(res).await.unwrap();
                })
            });
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}
