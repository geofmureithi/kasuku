use std::{ops::Deref, sync::Arc};

use context::{BackendPlugin, Context, Database, Debugger, Emitter, Fetcher, KasukuState};
use distribution::PluginAnnotation;
use interface::{Plugin, PluginWrapper};
use kasuku_database::KasukuDatabase;
use plugy::runtime::Runtime;
use types::config::Config;

pub type BoxedPlugin = Box<dyn Plugin>;

#[derive(Clone)]
pub struct KasukuRuntime {
    inner: Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>,
    pub state: Arc<KasukuState>,
}

impl Deref for KasukuRuntime {
    type Target = Arc<Runtime<BoxedPlugin, plugy::runtime::Plugin<BackendPlugin>>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl KasukuRuntime {
    pub async fn new(config: &Config) -> Result<Self, types::Error> {
        use futures::FutureExt;
        // let kasuku_database = KasukuDatabase::new(&config.internals.database_path)
        let kasuku_database = KasukuDatabase::new_from_memory()
            .then(|res| async {
                if res.is_err() {
                    let new_db = KasukuDatabase::new(&config.internals.database_path)
                        .await
                        .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
                    return Ok(new_db);
                }
                res.map_err(|err| types::Error::DatabaseError(err.to_string()))
            })
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
        kasuku_database
            .query_raw("PRAGMA journal_mode = WAL;")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA temp_store = 2")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA synchronous = NORMAL")
            .await
            .unwrap();
        kasuku_database
            .query_raw("PRAGMA cache_size = 64000")
            .await
            .unwrap();

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS plugins (
                    name TEXT NOT NULL PRIMARY KEY,
                    uri TEXT NOT NULL,
                    data TEXT
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS subscriptions (
                    event TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    plugin TEXT NOT NULL,
                    data TEXT,
                    FOREIGN KEY(plugin) REFERENCES plugins(name)
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS widgets (
                    name TEXT NOT NULL,
                    type TEXT NOT NULL,
                    plugin TEXT NOT NULL,
                    data TEXT,
                    FOREIGN KEY(plugin) REFERENCES plugins(name)
                );",
            )
            .await
            .map_err(|err| types::Error::DatabaseError(err.to_string()))?;

        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS vaults (
                    name TEXT NOT NULL PRIMARY KEY,
                    mount TEXT NOT NULL
                );",
            )
            .await
            .unwrap();
        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS entries (
                    path TEXT NOT NULL,
                    vault TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    last_modified INTEGER,
                    meta TEXT,
                    PRIMARY KEY (vault, path, filename),
                    FOREIGN KEY(vault) REFERENCES vaults(name)
                );",
            )
            .await
            .unwrap();
        kasuku_database
            .execute(
                "CREATE TABLE IF NOT EXISTS menu_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                label TEXT NOT NULL,
                action TEXT,
                icon TEXT,
                sort_order INTEGER,
                is_enabled INTEGER NOT NULL DEFAULT 1
            );
        ",
            )
            .await
            .unwrap();
        let state = Arc::new(KasukuState {
            database: kasuku_database.clone(),
            config: Arc::new(config.clone()),
            context: Default::default(),
        });
        let runtime = Runtime::new().unwrap();
        let runtime = runtime
            .context(Fetcher)
            .context(Emitter)
            .context(Database)
            .context(Debugger);
        for plugin in &config.plugins {
            let annotation = PluginAnnotation {
                wasm: std::fs::read(&plugin.uri).unwrap(),
                ..Default::default()
            };
            let plugin: PluginWrapper<BackendPlugin, _> = runtime
                .load_with(BackendPlugin {
                    addr: state.clone(),
                    name: plugin.name.clone(),
                    uri: plugin.uri.clone(),
                    meta: annotation,
                    // meta: distribution::load_package(&plugin.uri)
                    //     .await
                    //     .map_err(|err| types::Error::PluginError(err.to_string()))?,
                })
                .await
                .unwrap();

            plugin.on_load(&mut Context).await?;
            // TODO: insert plugin into db
        }

        for (vault, vault_config) in config.vaults.clone() {
            let mount = vault_config.mount.clone();
            let _res = state
                .database
                .execute(format!(
                    "INSERT INTO vaults(name, mount) VALUES ('{vault}', '{}')",
                    mount.to_str().ok_or(types::Error::Serialization(
                        "Invalid vault name".to_string()
                    ))?
                ))
                .await
                .map_err(|err| types::Error::DatabaseError(err.to_string()))?;
        }
        Ok(KasukuRuntime {
            inner: Arc::new(runtime),
            state,
        })
    }
}
