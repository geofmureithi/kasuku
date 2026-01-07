use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{any::type_name, fmt, marker::PhantomData};

use types::{table::from_table, Error, PluginEvent, Table, ViewType};

#[cfg(feature = "backend")]
use distribution::PluginAnnotation;

#[cfg(feature = "backend")]
pub type Addr = std::sync::Arc<backend::KasukuState>;

#[derive(Debug, Clone)]
pub struct BackendPlugin {
    #[cfg(feature = "backend")]
    pub addr: Addr,
    pub name: String,
    pub uri: String,
    #[cfg(feature = "backend")]
    pub meta: PluginAnnotation,
}

#[cfg(feature = "backend")]
pub use backend::KasukuState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextState {
    Ref,
    RefMut,
}

#[cfg(feature = "backend")]
mod backend {

    use std::sync::Arc;

    use kasuku_database::KasukuDatabase;
    use plugy::core::PluginLoader;
    use tokio::sync::RwLock;
    use types::config::Config;

    use crate::{BackendPlugin, Context};

    impl From<BackendPlugin> for plugy::runtime::Plugin<BackendPlugin> {
        fn from(val: BackendPlugin) -> Self {
            Self {
                name: val.name().to_string(),
                data: val,
                plugin_type: std::any::type_name::<BackendPlugin>().to_string(),
            }
        }
    }

    impl PluginLoader for BackendPlugin {
        fn name(&self) -> &'static str {
            Box::leak((self.name.clone()).into_boxed_str())
        }
        async fn bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
            let data = self.meta.wasm.clone();

            Ok(data)
        }
    }

    #[derive(Debug, Clone)]
    pub struct KasukuState {
        pub database: KasukuDatabase,
        pub config: Arc<Config>,
        pub context: Arc<RwLock<Context>>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fetcher;

#[plugy::macros::context(data = BackendPlugin)]
impl Fetcher {
    pub async fn fetch(
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        url: String,
    ) -> String {
        url
    }
}

pub struct Debugger;

#[plugy::macros::context(data = BackendPlugin)]
impl Debugger {
    pub async fn debug(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        output: String,
    ) {
        let plugin = &caller.data().as_ref().unwrap().plugin.name;
        tracing::info!("[{plugin}] {output}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emitter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    event: String,
    event_type: String,
    data: String,
}

#[plugy::macros::context(data = BackendPlugin)]
impl Emitter {
    pub async fn subscribe(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        subscription: crate::Subscription,
    ) -> Result<usize, types::Error> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        let plugin = caller.data().as_ref().unwrap().plugin.name.to_owned();
        let Subscription {
            event,
            event_type,
            data,
        } = subscription;
        let sql =
            "INSERT INTO subscriptions(plugin, event, event_type, data) VALUES(:plugin, :event, :event_type, :data);".to_string();
        let res = addr
            .database
            .execute_named_params(
                sql,
                types::PluginSubscription {
                    event,
                    event_type,
                    data,
                    plugin,
                },
            )
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        Ok(res)
    }

    pub async fn emit(
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        url: String,
    ) -> String {
        url
    }

    // pub async fn ask(
    //     _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
    //     plugin: &str,
    //     message: Event,
    // ) -> Result<Event, types::Error> {
    // }
}

pub struct Database;

#[plugy::macros::context(data = BackendPlugin)]
impl Database {
    pub async fn query(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        sql: String,
    ) -> Result<::types::Table, types::Error> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        addr.database
            .query_raw(&sql)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    pub async fn execute(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        sql: String,
    ) -> Result<usize, types::Error> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        addr.database
            .execute(sql)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    pub async fn execute_params(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        sql: String,
        params: Vec<String>,
    ) -> Result<usize, types::Error> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        addr.database
            .execute_params(sql, params)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }
}

impl Context {
    pub fn debug(output: &str) {
        debugger::sync::Debugger::debug(output.to_string());
    }
    pub fn fetch(url: &str) -> String {
        fetcher::sync::Fetcher::fetch(url.to_string())
    }
    pub fn register_view<W>(&mut self, _view: ViewType, _widget: W) {
        todo!()
    }

    pub fn append_script<Script>(&self, _script: Script) {
        todo!()
    }

    // Kasuku uses unocss presets?
    pub fn register_preset<Preset>(&self, _preset: Preset) {
        todo!()
    }

    pub fn subscribe<E: PluginEvent + Serialize>(&mut self, event: &E) -> Result<(), Error> {
        emitter::sync::Emitter::subscribe(Subscription {
            event: type_name::<E>().to_owned(),
            event_type: type_name::<E::Plugin>().to_owned(),
            data: serde_json::to_string(&event).map_err(|e| Error::Serialization(e.to_string()))?,
        })?;
        Ok(())
    }

    pub fn query<Res: DeserializeOwned>(&self, sql: &str) -> Result<Vec<Res>, Error> {
        debug!("{}", sql);
        let table = database::sync::Database::query(sql.to_owned())?;
        let items = from_table(&table)?;
        Ok(items)
    }

    pub fn query_raw(&self, sql: &str) -> Result<Table, Error> {
        debug!("{}", sql);
        let table = database::sync::Database::query(sql.to_owned())?;
        Ok(table)
    }

    pub fn execute(&mut self, sql: &str) -> Result<usize, Error> {
        debug!("{}", sql);
        let count = database::sync::Database::execute(sql.to_owned())?;
        Ok(count)
    }

    pub fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    pub fn execute_params(&mut self, sql: &str, params: Vec<String>) -> Result<usize, Error> {
        debug!("{}", sql);
        let count = database::sync::Database::execute_params(sql.to_owned(), params)?;
        Ok(count)
    }
}

impl<'de> Deserialize<'de> for &mut Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'a> Visitor<'_> for ContextVisitor<'a> {
            type Value = &'a mut Context;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("&mut Context")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Box::leak(Box::new(Context)))
            }
        }
        deserializer.deserialize_unit(ContextVisitor(PhantomData))
    }
}

impl<'de> Deserialize<'de> for &Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'a> Visitor<'_> for ContextVisitor<'a> {
            type Value = &'a Context;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("&Context")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(&Context)
            }
        }
        deserializer.deserialize_unit(ContextVisitor(PhantomData))
    }
}

#[derive(Debug, Default)]
pub struct Context;

impl Serialize for &Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_unit()
    }
}

impl Serialize for &mut Context {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_unit()
    }
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {{
        Context::debug(&format!($($args)*));
    }};
}
