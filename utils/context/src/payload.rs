use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::IpAddr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Str(String),
    Bytea(Vec<u8>),
    Inet(IpAddr),
    Uuid(u128),
    Map(HashMap<String, Value>),
    List(Vec<Value>),
    Point { x: f64, y: f64 },
    Null,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
    ShowColumns(Vec<(String, DataType)>),
    Create,
    Insert(usize),
    Select {
        labels: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
    SelectMap(Vec<HashMap<String, Value>>),
    Delete(usize),
    Update(usize),
    DropTable,
    DropFunction,
    AlterTable,
    CreateIndex,
    DropIndex,
    StartTransaction,
    Commit,
    Rollback,
    ShowVariable(PayloadVariable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    Int8,
    Int16,
    Int32,
    Int,
    Int128,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,
    Float32,
    Float,
    Text,
    Bytea,
    Inet,
    Date,
    Timestamp,
    Time,
    Interval,
    Uuid,
    Map,
    List,
    Decimal,
    Point,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PayloadVariable {
    Tables(Vec<String>),
    Functions(Vec<String>),
    Version(String),
}
#[cfg(feature = "backend")]
pub mod from_impl {
    use super::*;
    impl From<kasuku_database::prelude::Value> for Value {
        fn from(other: kasuku_database::prelude::Value) -> Self {
            match other {
                kasuku_database::prelude::Value::Bool(b) => Value::Bool(b),
                kasuku_database::prelude::Value::I8(i) => Value::I8(i),
                kasuku_database::prelude::Value::I16(i) => Value::I16(i),
                kasuku_database::prelude::Value::I32(i) => Value::I32(i),
                kasuku_database::prelude::Value::I64(i) => Value::I64(i),
                kasuku_database::prelude::Value::I128(i) => Value::I128(i),
                kasuku_database::prelude::Value::U8(u) => Value::U8(u),
                kasuku_database::prelude::Value::U16(u) => Value::U16(u),
                kasuku_database::prelude::Value::U32(u) => Value::U32(u),
                kasuku_database::prelude::Value::U64(u) => Value::U64(u),
                kasuku_database::prelude::Value::U128(u) => Value::U128(u),
                kasuku_database::prelude::Value::F32(f) => Value::F32(f),
                kasuku_database::prelude::Value::F64(f) => Value::F64(f),
                kasuku_database::prelude::Value::Str(s) => Value::Str(s),
                kasuku_database::prelude::Value::Bytea(b) => Value::Bytea(b),
                kasuku_database::prelude::Value::Inet(ip) => Value::Inet(ip),
                kasuku_database::prelude::Value::Uuid(uuid) => Value::Uuid(uuid),
                kasuku_database::prelude::Value::Map(map) => {
                    Value::Map(map.into_iter().map(|(k, v)| (k, v.into())).collect())
                }
                kasuku_database::prelude::Value::List(list) => {
                    Value::List(list.into_iter().map(|v| v.into()).collect())
                }
                kasuku_database::prelude::Value::Point(point) => Value::Point {
                    x: point.x,
                    y: point.y,
                },
                kasuku_database::prelude::Value::Null => Value::Null,
                _ => unimplemented!(
                    "Some parts especially date features are not yet fully implemented"
                ),
            }
        }
    }

    impl From<kasuku_database::prelude::Payload> for Payload {
        fn from(other: kasuku_database::prelude::Payload) -> Self {
            match other {
                kasuku_database::prelude::Payload::ShowColumns(columns) => Payload::ShowColumns(
                    columns.into_iter().map(|(s, dt)| (s, dt.into())).collect(),
                ),
                kasuku_database::prelude::Payload::Create => Payload::Create,
                kasuku_database::prelude::Payload::Insert(i) => Payload::Insert(i),
                kasuku_database::prelude::Payload::Select { labels, rows } => Payload::Select {
                    labels,
                    rows: rows
                        .into_iter()
                        .map(|r| r.into_iter().map(|v| v.into()).collect())
                        .collect(),
                },
                kasuku_database::prelude::Payload::SelectMap(map) => Payload::SelectMap(
                    map.into_iter()
                        .map(|m| m.into_iter().map(|(k, v)| (k, v.into())).collect())
                        .collect(),
                ),
                kasuku_database::prelude::Payload::Delete(i) => Payload::Delete(i),
                kasuku_database::prelude::Payload::Update(i) => Payload::Update(i),
                kasuku_database::prelude::Payload::DropTable => Payload::DropTable,
                kasuku_database::prelude::Payload::DropFunction => Payload::DropFunction,
                kasuku_database::prelude::Payload::AlterTable => Payload::AlterTable,
                kasuku_database::prelude::Payload::CreateIndex => Payload::CreateIndex,
                kasuku_database::prelude::Payload::DropIndex => Payload::DropIndex,
                kasuku_database::prelude::Payload::StartTransaction => Payload::StartTransaction,
                kasuku_database::prelude::Payload::Commit => Payload::Commit,
                kasuku_database::prelude::Payload::Rollback => Payload::Rollback,
                kasuku_database::prelude::Payload::ShowVariable(var) => {
                    Payload::ShowVariable(var.into())
                }
            }
        }
    }

    impl From<kasuku_database::prelude::DataType> for DataType {
        fn from(other: kasuku_database::prelude::DataType) -> Self {
            match other {
                kasuku_database::prelude::DataType::Boolean => DataType::Boolean,
                kasuku_database::prelude::DataType::Int8 => DataType::Int8,
                kasuku_database::prelude::DataType::Int16 => DataType::Int16,
                kasuku_database::prelude::DataType::Int32 => DataType::Int32,
                kasuku_database::prelude::DataType::Int => DataType::Int,
                kasuku_database::prelude::DataType::Int128 => DataType::Int128,
                kasuku_database::prelude::DataType::Uint8 => DataType::Uint8,
                kasuku_database::prelude::DataType::Uint16 => DataType::Uint16,
                kasuku_database::prelude::DataType::Uint32 => DataType::Uint32,
                kasuku_database::prelude::DataType::Uint64 => DataType::Uint64,
                kasuku_database::prelude::DataType::Uint128 => DataType::Uint128,
                kasuku_database::prelude::DataType::Float32 => DataType::Float32,
                kasuku_database::prelude::DataType::Float => DataType::Float,
                kasuku_database::prelude::DataType::Text => DataType::Text,
                kasuku_database::prelude::DataType::Bytea => DataType::Bytea,
                kasuku_database::prelude::DataType::Inet => DataType::Inet,
                kasuku_database::prelude::DataType::Date => DataType::Date,
                kasuku_database::prelude::DataType::Timestamp => DataType::Timestamp,
                kasuku_database::prelude::DataType::Time => DataType::Time,
                kasuku_database::prelude::DataType::Interval => DataType::Interval,
                kasuku_database::prelude::DataType::Uuid => DataType::Uuid,
                kasuku_database::prelude::DataType::Map => DataType::Map,
                kasuku_database::prelude::DataType::List => DataType::List,
                kasuku_database::prelude::DataType::Decimal => DataType::Decimal,
                kasuku_database::prelude::DataType::Point => DataType::Point,
            }
        }
    }

    impl From<kasuku_database::prelude::PayloadVariable> for PayloadVariable {
        fn from(other: kasuku_database::prelude::PayloadVariable) -> Self {
            match other {
                kasuku_database::prelude::PayloadVariable::Tables(tables) => {
                    PayloadVariable::Tables(tables)
                }
                kasuku_database::prelude::PayloadVariable::Functions(functions) => {
                    PayloadVariable::Functions(functions)
                }
                kasuku_database::prelude::PayloadVariable::Version(version) => {
                    PayloadVariable::Version(version)
                }
            }
        }
    }
}
