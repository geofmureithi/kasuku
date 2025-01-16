use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::sqlite::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

use transaction::Transaction;
use types::{RawValue, Table};
type Result<T, E = crate::sqlite::Error> = std::result::Result<T, E>;

pub mod sqlite;
pub mod transaction;

pub mod prelude {
    pub use super::KasukuDatabase as Database;
    pub use crate::sqlite::Error;
    pub use sqlparser::ast::Statement;
    use sqlparser::dialect::SQLiteDialect;
    use sqlparser::parser::{Parser, ParserError};
    pub fn parse<Sql: AsRef<str>>(sql: Sql) -> Result<Vec<Statement>, ParserError> {
        Parser::parse_sql(&SQLiteDialect {}, sql.as_ref())
    }
    pub use crate::sqlite;
}

#[derive(Debug, Clone)]
pub struct KasukuDatabase {
    inner: Arc<tokio_rusqlite::Connection>,
}

impl KasukuDatabase {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(KasukuDatabase {
            inner: Arc::new(tokio_rusqlite::Connection::open(path).await?),
        })
    }

    pub async fn new_from_memory() -> Result<Self> {
        Ok(KasukuDatabase {
            inner: Arc::new(tokio_rusqlite::Connection::open_in_memory().await?),
        })
    }

    pub async fn execute<S: AsRef<str> + Send + 'static>(&self, query: S) -> Result<usize> {
        Ok(self
            .inner
            .call(move |conn| {
                conn.execute(query.as_ref(), [])
                    .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))
            })
            .await?)
    }

    pub async fn execute_params<
        S: AsRef<str> + Send + 'static,
        Params: Serialize + Send + 'static,
    >(
        &self,
        query: S,
        params: Params,
    ) -> Result<usize> {
        Ok(self
            .inner
            .call(move |conn| {
                conn.execute(
                    query.as_ref(),
                    to_params(&params).map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))?,
                )
                .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))
            })
            .await?)
    }

    pub async fn execute_named_params<
        S: AsRef<str> + Send + 'static,
        Params: Serialize + Send + 'static,
    >(
        &self,
        query: S,
        params: Params,
    ) -> Result<usize> {
        Ok(self
            .inner
            .call(move |conn| {
                conn.execute(
                    query.as_ref(),
                    to_params_named(&params)
                        .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))?
                        .to_slice()
                        .as_slice(),
                )
                .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))
            })
            .await?)
    }

    pub async fn query<Res: DeserializeOwned + Send + 'static>(
        &self,
        query: &str,
    ) -> Result<Vec<Res>> {
        let query = query.to_owned();
        Ok(self
            .inner
            .call(move |conn| {
                let mut statement = conn.prepare(&query)?;
                let columns = columns_from_statement(&statement);
                let rows = statement
                    .query_and_then([], |row| {
                        from_row_with_columns::<Res>(row, &columns)
                            .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))
                    })
                    .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?;
                rows.collect()
            })
            .await?)
    }

    pub async fn query_params<
        Res: DeserializeOwned + Send + 'static,
        Params: Serialize + Send + 'static,
    >(
        &self,
        query: &str,
        params: Params,
    ) -> Result<Vec<Res>> {
        let query = query.to_owned();
        Ok(self
            .inner
            .call(move |conn| {
                let mut statement = conn.prepare(&query)?;
                let columns = columns_from_statement(&statement);
                let rows = statement
                    .query_and_then(
                        to_params(&params)
                            .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))?,
                        |row| {
                            from_row_with_columns::<Res>(row, &columns)
                                .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))
                        },
                    )
                    .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?;
                rows.collect()
            })
            .await?)
    }

    pub async fn query_named_params<
        Res: DeserializeOwned + Send + 'static,
        Params: Serialize + Send + 'static,
    >(
        &self,
        query: &str,
        params: Params,
    ) -> Result<Vec<Res>> {
        let query = query.to_owned();
        Ok(self
            .inner
            .call(move |conn| {
                let mut statement = conn.prepare(&query)?;
                let columns = columns_from_statement(&statement);
                let rows = statement
                    .query_and_then(
                        to_params_named(&params)
                            .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))?
                            .to_slice()
                            .as_slice(),
                        |row| {
                            from_row_with_columns::<Res>(row, &columns)
                                .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))
                        },
                    )
                    .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?;
                rows.collect()
            })
            .await?)
    }

    pub async fn query_raw(&self, query: &str) -> Result<Table> {
        let query = query.to_owned();
        Ok(self
            .inner
            .call(move |conn| {
                let mut statement = conn.prepare(&query)?;
                let columns = columns_from_statement(&statement);
                let rows: Vec<BTreeMap<String, RawValue>> = statement
                    .query_and_then([], |row| {
                        from_row_with_columns::<BTreeMap<String, RawValue>>(row, &columns)
                            .map_err(|e| tokio_rusqlite::Error::Other(Box::new(e)))
                    })
                    .map_err(|e| tokio_rusqlite::Error::Rusqlite(e))?
                    .map(|res| res.unwrap())
                    .collect();
                Ok(Table { columns, rows })
            })
            .await?)
    }

    pub async fn transaction(&self) -> Transaction<'_> {
        Transaction::new(&self).await
    }
}
