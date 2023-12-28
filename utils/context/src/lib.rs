pub mod payload;

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{any::type_name, fmt, marker::PhantomData};

use types::{Error, PluginEvent, ViewType};

#[cfg(feature = "backend")]
use distribution::PluginAnnotation;

#[cfg(feature = "backend")]
pub type Addr = xtra::Address<backend::GlobalContext>;

#[derive(Debug, Clone)]
pub struct BackendPlugin {
    #[cfg(feature = "backend")]
    pub addr: Addr,
    pub name: String,
    pub uri: String,
    #[cfg(feature = "backend")]
    pub meta: PluginAnnotation,
}
#[derive(Debug)]
pub struct Query(pub String);

#[cfg(feature = "backend")]
pub use backend::GlobalContext;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub enum ContextState {
    Ref = 1,
    RefMut = 2,
}

#[cfg(feature = "backend")]
mod backend {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use kasuku_database::{prelude::Glue, KasukuDatabase};
    use plugy::core::PluginLoader;
    use xtra::Handler;

    use crate::{payload::Payload, BackendPlugin, Query};

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
        fn bytes(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, anyhow::Error>>>>
        {
            let data = self.meta.wasm.clone();

            Box::pin(async move { Ok(data) })
        }
    }

    #[async_trait]
    impl Handler<Query> for GlobalContext {
        type Return = Result<Vec<Payload>, kasuku_database::prelude::Error>;

        async fn handle(
            &mut self,
            sql: Query,
            _ctx: &mut xtra::Context<Self>,
        ) -> Result<Vec<Payload>, kasuku_database::prelude::Error> {
            let conn = &mut self.database;
            let mut res = conn.lock().unwrap();
            Ok(res.execute(sql.0)?.into_iter().map(Into::into).collect())
        }
    }

    #[derive(xtra::Actor)]
    pub struct GlobalContext {
        database: Arc<Mutex<Glue<KasukuDatabase>>>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl GlobalContext {
        pub fn new(database: Arc<Mutex<Glue<KasukuDatabase>>>) -> Self {
            GlobalContext { database }
        }
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
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        output: String,
    ) {
        println!("{output}")
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
    ) -> Result<Vec<crate::payload::Payload>, types::Error> {
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        let plugin = &caller.data().as_ref().unwrap().plugin.name;
        let Subscription {
            event,
            event_type,
            data,
        } = subscription;
        let sql =
            format!("INSERT INTO subscriptions(plugin, event, event_type, data) VALUES('{plugin}','{event}','{event_type}','{data}');");
        addr.send(Query(sql))
            .await
            .map_err(|e| Error::PluginError(e.to_string()))?
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    pub async fn emit(
        _caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        url: String,
    ) -> String {
        url
    }
}

pub struct Database;

#[plugy::macros::context(data = BackendPlugin)]
impl Database {
    pub async fn query(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<BackendPlugin>>,
        sql: String,
        ctx_state: crate::ContextState,
    ) -> Result<Vec<crate::payload::Payload>, types::Error> {
        use kasuku_database::prelude::parse;
        println!("{sql}");
        if let ContextState::Ref = ctx_state {
            let req = parse(&sql).map_err(|e| Error::DatabaseError(e.to_string()))?;
            if req
                .iter()
                .any(|r| !matches!(r, sqlparser::ast::Statement::Query(_)))
            {
                return Err(Error::DatabaseError(
                    "Tried to modify database in non-mutable context. Please use execute()"
                        .to_owned(),
                ));
            }
        }
        let addr = caller.data().as_ref().unwrap().plugin.data.addr.clone();
        Ok(addr
            .send(Query(sql))
            .await
            .map_err(|e| Error::PluginError(e.to_string()))?
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

impl Context {
    pub fn debug(&self, output: &str) {
        debugger::sync::Debugger::debug(output.to_string());
    }
    pub fn fetch(&self, url: &str) -> String {
        fetcher::sync::Fetcher::fetch(url.to_string())
    }
    pub fn register_view<W>(&mut self, _view: ViewType, _widget: W) {
        todo!()
    }

    pub fn subscribe<E: PluginEvent + Serialize>(&mut self, event: &E) -> Result<(), Error> {
        emitter::sync::Emitter::subscribe(Subscription {
            event: type_name::<E>().to_owned(),
            event_type: type_name::<E::Plugin>().to_owned(),
            data: serde_json_wasm::to_string(&event)
                .map_err(|e| Error::Serialization(e.to_string()))?,
        })?;
        Ok(())
    }

    pub fn query(&self, sql: &str) -> Result<Vec<crate::payload::Payload>, Error> {
        let res = database::sync::Database::query(sql.to_owned(), ContextState::Ref)?;
        Ok(res)
    }

    pub fn execute(&mut self, sql: &str) -> Result<Vec<crate::payload::Payload>, Error> {
        let res = database::sync::Database::query(sql.to_owned(), ContextState::RefMut)?;
        Ok(res)
    }
}

pub struct Context;

#[cfg(not(target_arch = "wasm32"))]
impl Context {
    pub fn acquire() -> &'static mut Self {
        Box::leak(Box::new(Context))
    }
}

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

impl<'de, 'a> Deserialize<'de> for &'a mut Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'de, 'a> Visitor<'de> for ContextVisitor<'a> {
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

impl<'de, 'a> Deserialize<'de> for &'a Context {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContextVisitor<'a>(PhantomData<&'a ()>);

        impl<'de, 'a> Visitor<'de> for ContextVisitor<'a> {
            type Value = &'a Context;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("&Context")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Box::leak(Box::new(Context)))
            }
        }
        deserializer.deserialize_unit(ContextVisitor(PhantomData))
    }
}
