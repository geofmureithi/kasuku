use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use tokio::process::Command;
use types::config::{Config, PluginConfig, VaultConfig};

type DynError = Box<dyn std::error::Error>;

// TODO: add watching for plugins and backend
#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("dist") => dist().await?,
        Some("dev") => dev_server().await?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

dist            builds application
dev             runs the dev server
"
    )
}

async fn dist() -> Result<(), DynError> {
    let _ = fs::remove_dir_all(&dist_dir());
    fs::create_dir_all(&dist_dir())?;

    dist_backend_binary().await?;

    Ok(())
}

async fn dist_backend_binary() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut backend = project_root();
    backend.push("core/backend");
    let status = Command::new(cargo)
        .current_dir(backend)
        .args(&["build", "--release"])
        .status()
        .await?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    let dst = project_root().join("target/release/backend");

    fs::copy(&dst, dist_dir().join("backend"))?;

    if Command::new("strip")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
        .await
        .is_ok()
    {
        eprintln!("stripping the binary");
        let status = Command::new("strip").arg(&dst).status().await?;
        if !status.success() {
            Err("strip failed")?;
        }
    } else {
        eprintln!("no `strip` utility found")
    }

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn dist_dir() -> PathBuf {
    project_root().join("target/dist")
}

async fn dev_server() -> Result<(), DynError> {
    let _ = fs::remove_dir_all(&dev_dir());
    fs::create_dir_all(&dev_dir())?;

    let mut config = types::config::Config::default();

    config.internals.database_path = dev_dir().join("db.sqlite");

    config.server.port = 8080;

    config.vaults.insert(
        "Default".to_string(),
        VaultConfig {
            mount: dev_dir().join("vault"),
            ..Default::default()
        },
    );

    fs::create_dir_all(dev_dir().join("vault"))?;

    let test_markdown = r###"    
# Daily Task Template

**Date:** ___

## Schedule
| Time   | Task     |
|--------|----------|
| 6-7 AM |          |
| 7-8 AM |          |
| 8-9 AM |          |
| 9-10 AM|          |
| 10-11 AM|         |
| 11-12 PM|         |
| 12-1 PM|          |
| 1-2 PM |          |
| 2-3 PM |          |
| 3-4 PM |          |
| 4-5 PM |          |
| 5-6 PM |          |
| 6-7 PM |          |
| 7-8 PM |          |
| 8-9 PM |          |
| 9-10 PM|          |

## Main Tasks
- [ ] One
- [ ] Two
- [ ] Three

## Additional Tasks
- [ ] Minor
- [ ] Major
- [ ] semver

## Notes created today
```sql,dataview
SELECT * FROM sqlite_master
```

## Notes
- Blah blah blah 
    "###;

    fs::write(dev_dir().join("vault/test.md"), test_markdown)?;

    let plugins = ["tasks", "dataview"];
    let mut tasks = plugins.into_iter().fold(FuturesUnordered::new(), |fut, p| {
        fut.push(build_plugin(p.to_string()).boxed());
        config.plugins.push(PluginConfig {
            name: p.to_owned(),
            uri: dev_dir()
                .join(format!("{}.wasm", p))
                .to_str()
                .unwrap()
                .to_owned(),
        });
        fut
    });

    while let Some(result) = tasks.next().await {
        result?;
    }

    let toml = toml::to_string(&config)?;

    tokio::fs::write(dev_dir().join("Kasuku.toml"), toml).await?;

    if let Err(e) = tokio::try_join!(run_backend_dev(), run_frontend_dev(&config)) {
        eprintln!("Could not run the dev server: {e}");
        return Err(e);
    }

    Ok(())
}

async fn build_plugin<P: AsRef<str>>(plugin: P) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let plugin_path = project_root().join(format!("plugins/{}", plugin.as_ref()));
    let status = Command::new(cargo)
        .current_dir(plugin_path)
        .args(&["build", "--release", "--target=wasm32-unknown-unknown"])
        .status()
        .await?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    let from = project_root().join(format!(
        "target/wasm32-unknown-unknown/release/{}.wasm",
        plugin.as_ref()
    ));

    let dst = dev_dir().join(format!("{}.wasm", plugin.as_ref()));
    fs::copy(&from, &dst)?;
    Ok(())
}

async fn run_backend_dev() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let backend = project_root().join("core/backend");
    let status = Command::new(cargo)
        .current_dir(backend)
        .args(&["build"])
        .status()
        .await?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    let from = project_root().join("target/debug/backend");

    let dst = dev_dir().join("backend");
    fs::copy(&from, &dst)?;

    Command::new(dst)
        .current_dir(dev_dir())
        .args(&["build"])
        .status()
        .await?;
    Ok(())
}

async fn run_frontend_dev(_cfg: &Config) -> Result<(), DynError> {
    let frontend = project_root().join("core/frontend");
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Build {
        target: PathBuf,
        dist: PathBuf,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Proxy {
        rewrite: String,
        backend: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct TrunkConfig {
        build: Build,
        proxy: Vec<Proxy>,
    }

    let config = TrunkConfig {
        build: Build {
            dist: dev_dir().join("static"),
            target: frontend.join("index.html"),
        },
        proxy: vec![],
    };

    let toml = toml::to_string(&config)?;

    tokio::fs::write(dev_dir().join("Trunk.toml"), toml).await?;

    fs::create_dir_all(&dev_dir().join("static"))?;

    let status = Command::new("trunk")
        .current_dir(frontend)
        .args(&[
            "watch",
            "--config",
            dev_dir().join("Trunk.toml").to_str().unwrap(),
        ])
        .status()
        .await?;

    if !status.success() {
        Err("trunk serve failed")?;
    }

    Ok(())
}

fn dev_dir() -> PathBuf {
    project_root().join("target/dev")
}
