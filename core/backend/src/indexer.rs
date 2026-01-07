use runtime::KasukuRuntime;
use tokio::task::block_in_place;
use types::{config::VaultConfig, FilePath};

use futures::{
    channel::mpsc::{channel, Receiver},
    executor::block_on,
    future::join_all,
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub async fn run_indexer(runtime: &KasukuRuntime) {
    let mut listeners = Vec::new();
    for (vault, vault_cfg) in &runtime.state.config.vaults {
        listeners.push(vault_indexer(runtime, vault, vault_cfg));
    }
    join_all(listeners).await;
}

/// Compute a relative path from `base` to `target`.
///
/// Both `base` and `target` should be absolute paths on the same filesystem.
/// Returns a `PathBuf` that, when appended to `base`, should refer to the same
/// filesystem location as `target`.
fn make_relative(base: &Path, target: &Path) -> PathBuf {
    use std::path::{Component, PathBuf};
    // Collect components (e.g., ["root", "home", "user", "folder", ...])
    let base_components: Vec<Component> = base.components().collect();
    let target_components: Vec<Component> = target.components().collect();

    // Determine how many path components are shared (common prefix).
    let mut i = 0;
    while i < base_components.len()
        && i < target_components.len()
        && base_components[i] == target_components[i]
    {
        i += 1;
    }

    // Build the relative path.
    // Step 1: For each remaining component in the base path (after the common prefix),
    //         we need to go up one directory, i.e., "..".
    let mut result = PathBuf::new();
    for _ in i..base_components.len() {
        result.push("..");
    }

    // Step 2: For each remaining component in the target path (after the common prefix),
    //         push it onto our result.
    for comp in &target_components[i..] {
        result.push(comp.as_os_str());
    }

    result
}

async fn vault_indexer(
    runtime: &KasukuRuntime,
    vault: &str,
    cfg: &VaultConfig,
) -> notify::Result<()> {
    for entry in WalkDir::new(&cfg.mount) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Only consider regular files
        if entry.file_type().is_file() {
            let path = make_relative(cfg.mount.as_path(), entry.path());
            // Check if it ends in .md
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        runtime.state
                            .database
                            .execute_named_params(
                                "INSERT INTO entries(path, vault, filename) VALUES(:path, :vault, :filename)",
                                FilePath {
                                    filename: filename.to_owned(),
                                    path,
                                    vault: vault.to_owned(),
                                },
                            )
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }

    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(cfg.mount.as_ref(), RecursiveMode::Recursive)?;

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
