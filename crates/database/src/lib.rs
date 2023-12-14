use std::collections::HashMap;
use std::io::ErrorKind;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use gluesql_core::data::{Schema, Key};
use gluesql_core::error::Error;
use gluesql_core::store::{
    AlterTable, CustomFunction, CustomFunctionMut, DataRow, Index, IndexMut, Metadata, RowIter,
    Store, StoreMut, Transaction,
};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use baildon::btree::Baildon;
use baildon::btree::Direction;

type Result<T, E = Error> = std::result::Result<T, E>;

pub mod prelude {
    pub use gluesql_core::prelude::*;
    pub use super::KasukuDatabase as Database;
}


pub struct KasukuDatabase {
    pub schemas: Baildon<String, Schema>,
    config: BaildonConfig,
    tables: Mutex<HashMap<String, Arc<Baildon<Key, DataRow>>>>,
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct BaildonConfig {
    pub index: AtomicI64,
    pub name: String,
    pub path: String,
}

impl KasukuDatabase {
    pub async fn new(path: &str) -> Result<Self> {
        // Create our path
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        // Get a canonical representation to store in config
        let mut canonical_path = tokio::fs::canonicalize(path)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;

        let config_path = canonical_path.display().to_string();
        let config_name = canonical_path
            .components()
            .last()
            .expect("must be a last element")
            .as_os_str()
            .to_string_lossy()
            .to_string();

        canonical_path.push("schema");
        canonical_path.set_extension("db");
        if tokio::fs::try_exists(&canonical_path)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?
        {
            return Err(Error::StorageMsg(format!(
                "database '{path}' already exists"
            )));
        }
        let schemas: Baildon<String, Schema> = Baildon::try_new(&canonical_path, 13)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        let config = BaildonConfig {
            path: config_path,
            name: config_name,
            index: AtomicI64::new(0),
        };

        Ok(KasukuDatabase {
            schemas,
            config,
            tables: Mutex::new(HashMap::new()),
        })
    }

    pub async fn open(path: &str) -> Result<Self> {
        let mut db_file = PathBuf::from(path);
        db_file.push("schema");
        db_file.set_extension("db");
        let schemas: Baildon<String, Schema> = Baildon::try_open(&db_file)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;

        // let mut f_path = PathBuf::from(db_file);
        db_file.set_extension("cfg");
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(&db_file)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        let mut s_cfg = String::new();
        let _ = file
            .read_to_string(&mut s_cfg)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        let config: BaildonConfig =
            serde_json::from_str(&s_cfg).map_err(|e| Error::StorageMsg(e.to_string()))?;
        Ok(KasukuDatabase {
            schemas,
            config,
            tables: Mutex::new(HashMap::new()),
        })
    }

    pub async fn save(&self) -> Result<()> {
        let mut f_path = PathBuf::from(&self.config.path);
        f_path.push("schema");
        f_path.set_extension("cfg");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&f_path)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        let s_config =
            serde_json::to_string(&self.config).map_err(|e| Error::StorageMsg(e.to_string()))?;
        file.write_all(s_config.as_bytes())
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))
    }

    async fn get_table(&self, name: &str) -> Result<Arc<Baildon<Key, DataRow>>> {
        let mut table_lock = self.tables.lock().await;

        let t_name = name.to_string();
        match table_lock.get(&t_name) {
            Some(db) => Ok(db.clone()),
            None => {
                if self.fetch_schema(&t_name).await?.is_none() {
                    return Err(Error::StorageMsg(format!(
                        "schema '{t_name}' does not exist"
                    )));
                }
                let mut table_file = PathBuf::from(self.config.path.clone());
                table_file.push(&t_name);
                table_file.set_extension("db");
                // First try to open, if we can open add it to the HashMap and return
                let table: Baildon<Key, DataRow> = match Baildon::try_open(table_file.clone()).await
                {
                    Ok(tbl) => tbl,
                    Err(err) => {
                        if let Some(io_error) = err.downcast_ref::<std::io::Error>() {
                            if io_error.kind() == ErrorKind::NotFound {
                                Baildon::try_new(table_file, 13)
                                    .await
                                    .map_err(|e| Error::StorageMsg(e.to_string()))?
                            } else {
                                return Err(Error::StorageMsg(err.to_string()));
                            }
                        } else {
                            return Err(Error::StorageMsg(err.to_string()));
                        }
                    }
                };
                table_lock.insert(t_name.clone(), Arc::new(table));
                Ok(table_lock.get(&t_name).expect("MUST BE THERE").clone())
            }
        }
    }

    pub async fn print_tables(&self) -> Result<()> {
        let mut streamer = self.schemas.keys(Direction::Ascending).await;
        while let Some(table) = streamer.next().await {
            self.print_table(&table).await?;
        }
        Ok(())
    }

    pub async fn print_table(&self, table_name: &str) -> Result<()> {
        let table = self.get_table(table_name).await?;
        let mut sep = "";
        let callback = |(key, value)| {
            print!("{sep}{key:?}:{value:?}");
            sep = ", ";
            ControlFlow::Continue(())
        };
        table.traverse_entries(Direction::Ascending, callback).await;
        println!("\nutilization: {}", table.utilization().await);
        println!();
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl Store for KasukuDatabase {
    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {
        Ok(self
            .schemas
            .values(Direction::Ascending)
            .await
            .collect::<Vec<Schema>>()
            .await)
    }

    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        let t_name = table_name.to_string();
        self.schemas.get(&t_name).await.map(Ok).transpose()
    }

    async fn fetch_data(&self, table_name: &str, key: &Key) -> Result<Option<DataRow>> {
        let table = self.get_table(table_name).await?;
        table.get(key).await.map(Ok).transpose()
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        let table = self.get_table(table_name).await?;
        Ok(Box::new(
            table
                .entries(Direction::Ascending)
                .await
                .collect::<Vec<(Key, DataRow)>>()
                .await
                .into_iter()
                .map(Ok),
        ))
    }
}

#[async_trait::async_trait(?Send)]
impl StoreMut for KasukuDatabase {
    async fn insert_schema(&mut self, schema: &Schema) -> Result<()> {
        let t_name = schema.table_name.clone();
        let s = schema.clone();
        // Insert it into our schemas table
        self.schemas
            .insert(t_name, s)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        Ok(())
    }

    async fn delete_schema(&mut self, table_name: &str) -> Result<()> {
        let mut db_file = PathBuf::from(table_name);
        let t_name = table_name.to_string();
        db_file.set_extension("db");
        let _ = tokio::fs::remove_file(&db_file)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()));
        db_file.set_extension("wal");
        let _ = tokio::fs::remove_file(&db_file)
            .await
            .map_err(|e| Error::StorageMsg(e.to_string()));
        self.schemas
            .delete(&t_name)
            .await
            .map(|_v| ())
            .map_err(|e| Error::StorageMsg(e.to_string()))?;
        Ok(())
    }

    async fn append_data(&mut self, table_name: &str, rows: Vec<DataRow>) -> Result<()> {
        let table = self.get_table(table_name).await?;
        for row in rows {
            let idx = self.config.index.fetch_add(1, Ordering::SeqCst);
            table
                .insert(Key::I64(idx), row)
                .await
                .map_err(|e| Error::StorageMsg(e.to_string()))?;
        }
        Ok(())
    }

    async fn insert_data(&mut self, table_name: &str, rows: Vec<(Key, DataRow)>) -> Result<()> {
        let table = self.get_table(table_name).await?;
        for (key, row) in rows {
            table
                .insert(key, row)
                .await
                .map_err(|e| Error::StorageMsg(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete_data(&mut self, table_name: &str, keys: Vec<Key>) -> Result<()> {
        let table = self.get_table(table_name).await?;
        for key in keys {
            table
                .delete(&key)
                .await
                .map_err(|e| Error::StorageMsg(e.to_string()))?;
        }
        Ok(())
    }
}

impl Index for KasukuDatabase {}

impl IndexMut for KasukuDatabase {}

impl AlterTable for KasukuDatabase {}

impl Transaction for KasukuDatabase {}

impl Metadata for KasukuDatabase {}

impl CustomFunction for KasukuDatabase {}

impl CustomFunctionMut for KasukuDatabase {}
