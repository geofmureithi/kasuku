/// This code is adapted from https://github.com/twistedfall/serde_rusqlite and the main goal is to make it compatible with tokio_sqlite
use tokio_rusqlite::{params_from_iter, ParamsFromIter};

pub use de::{DeserRows, DeserRowsRef, RowDeserializer};
pub use error::{Error, Result};
pub use ser::{NamedParamSlice, NamedSliceSerializer, PositionalParams, PositionalSliceSerializer};

pub mod de;
pub mod error;
pub mod ser;
#[cfg(test)]
mod tests;

/// Returns column names of the statement the way `from_row_with_columns()` method expects them
///
/// This function is needed because by default `column_names()` returns `Vec<&str>` which
/// ties it to the lifetime of the `tokio_rusqlite::Statement`. This way we won't be able to run for example
/// `.query_map()` because it mutably borrows `tokio_rusqlite::Statement` and by that time it's already borrowed
/// for columns. So this function owns all column names to detach them from the lifetime of `tokio_rusqlite::Statement`.
#[inline]
pub fn columns_from_statement(stmt: &tokio_rusqlite::Statement) -> Vec<String> {
    stmt.column_names().into_iter().map(str::to_owned).collect()
}

/// Deserializes an instance of `D: serde::Deserialize` from `tokio_rusqlite::Row`
///
/// Calling this function incurs allocation and processing overhead because we need to fetch column names from the row.
/// So use with care when calling this function in a loop or check `from_row_with_columns()` to avoid that overhead.
///
/// You should supply this function to `query_map()`.
#[inline]
pub fn from_row<D: serde::de::DeserializeOwned>(row: &tokio_rusqlite::Row) -> Result<D> {
    let columns = row.as_ref().column_names();
    let columns_ref = columns.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    from_row_with_columns(row, &columns_ref)
}

/// Deserializes any instance of `D: serde::Deserialize` from `tokio_tokio_rusqlite::Row` with specified columns
///
/// Use this function over `from_row()` to avoid allocation and overhead for fetching column names. To get columns names
/// you can use `columns_from_statement()`.
///
/// You should use this function in the closure you supply to `query_map()`.
///
/// Note: `columns` is a slice of owned `String`s to be type compatible with what `columns_from_statement()`
/// returns. Most of the time the result of that function will be used as the argument so it makes little sense
/// to accept something like `&[impl AsRef<str>]` here. It will only make usage of the API less ergonomic. E.g.
/// There will be 2 generic type arguments to the `from_row_with_columns()` instead of one.
#[inline]
pub fn from_row_with_columns<D: serde::de::DeserializeOwned>(
    row: &tokio_rusqlite::Row,
    columns: &[String],
) -> Result<D> {
    D::deserialize(RowDeserializer::from_row_with_columns(row, columns))
}

/// Returns iterator that owns `tokio_rusqlite::Rows` and deserializes all records from it into instances of `D: serde::Deserialize`
///
/// Also see `from_row()` for some specific info.
///
/// This function covers most of the use cases and is easier to use than the alternative `from_rows_ref()`.
#[inline]
pub fn from_rows<D: serde::de::DeserializeOwned>(rows: tokio_rusqlite::Rows) -> DeserRows<D> {
    DeserRows::new(rows)
}

/// Returns iterator that borrows `tokio_rusqlite::Rows` and deserializes records from it into instances of `D: serde::Deserialize`
///
/// Use this function instead of `from_rows()` when you still need iterator with the remaining rows after deserializing some
/// of them.
#[inline]
pub fn from_rows_ref<'rows, 'stmt, D: serde::de::DeserializeOwned>(
    rows: &'rows mut tokio_rusqlite::Rows<'stmt>,
) -> DeserRowsRef<'rows, 'stmt, D> {
    DeserRowsRef::new(rows)
}

/// Serializes an instance of `S: serde::Serialize` into structure for positional bound query arguments
///
/// To get the slice suitable for supplying to `query()` or `execute()` call `to_slice()` on the `Ok` result and
/// borrow it.
#[inline]
pub fn to_params<S: serde::Serialize>(obj: S) -> Result<ParamsFromIter<PositionalParams>> {
    obj.serialize(PositionalSliceSerializer::default())
        .map(params_from_iter)
}

/// Serializes an instance of `S: serde::Serialize` into structure for named bound query arguments
///
/// To get the slice suitable for supplying to `query_named()` or `execute_named()` call `to_slice()` on the `Ok` result
/// and borrow it.
#[inline]
pub fn to_params_named<S: serde::Serialize>(obj: S) -> Result<NamedParamSlice> {
    obj.serialize(NamedSliceSerializer::default())
}

/// Serializes only the specified `fields` of an instance of `S: serde::Serialize` into structure
/// for named bound query arguments
///
/// To get the slice suitable for supplying to `query_named()` or `execute_named()` call `to_slice()` on the `Ok` result
/// and borrow it.
#[inline]
pub fn to_params_named_with_fields<S: serde::Serialize>(
    obj: S,
    fields: &[&str],
) -> Result<NamedParamSlice> {
    obj.serialize(NamedSliceSerializer::with_only_fields(fields))
}
