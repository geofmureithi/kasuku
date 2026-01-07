use std::collections::BTreeMap;
use std::{f32, f64};

use crate::{RawValue, Table};
use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::{forward_to_deserialize_any, Deserializer};

use crate::Error;

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! forward_to_row_value_deserializer {
	($($fun:ident)*) => {
		$(
			fn $fun<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
				self.row_value().$fun(visitor)
			}
		)*
	}
}

pub fn from_table<D: serde::de::DeserializeOwned>(table: &Table) -> Result<Vec<D>> {
    let mut items = Vec::new();
    let columns: Vec<String> = table
        .rows
        .first()
        .map(|s| s.keys().map(|s| s.to_string()).collect())
        .unwrap_or(table.columns.clone());
    for row in table.rows.iter() {
        let item = D::deserialize(TableRowDeserializer::from_row_with_columns(row, &columns))?;
        items.push(item);
    }
    Ok(items)
}

/// Deserializer for `rusqlite::Row`
///
/// You shouldn't use it directly, but via the crate's `from_row()` function. Check the crate documentation for example.
pub struct TableRowDeserializer<'row, 'cols> {
    row: &'row BTreeMap<String, RawValue>,
    columns: &'cols [String],
}

impl<'row, 'cols> TableRowDeserializer<'row, 'cols> {
    pub fn from_row_with_columns(
        row: &'row BTreeMap<String, RawValue>,
        columns: &'cols [String],
    ) -> Self {
        Self { row, columns }
    }

    fn row_value(&self) -> TableRowValue<'row> {
        TableRowValue {
            row: self.row,
            idx: 0,
        }
    }
}

impl<'de> Deserializer<'de> for TableRowDeserializer<'de, '_> {
    type Error = Error;

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.row_value().deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self.row_value())
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(TableRowSeqAccess { idx: 0, de: self })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_map(TableRowMapAccess { idx: 0, de: self })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.row_value().deserialize_enum(name, variants, visitor)
    }

    forward_to_row_value_deserializer! {
        deserialize_bool
        deserialize_f32
        deserialize_f64
        deserialize_option
        deserialize_unit
        deserialize_any
        deserialize_byte_buf
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i64 u8 u16 u32 u64 char str string bytes
        seq tuple_struct identifier ignored_any
    }
}

struct TableRowValue<'row> {
    idx: usize,
    row: &'row BTreeMap<String, RawValue>,
}

impl<'row> TableRowValue<'row> {
    fn value(&self) -> Result<RawValue> {
        let keys: Vec<_> = self.row.keys().collect();
        let key = keys[self.idx];
        self.row
            .get(key)
            .cloned()
            .ok_or(Error::Serialization(format!(
                "Could not read TableRowValue for key {}. Check the type for deserialization",
                key
            )))
    }

    fn deserialize_any_helper<V: Visitor<'row>>(
        self,
        visitor: V,
        value: RawValue,
    ) -> Result<V::Value> {
        match value {
            RawValue::Null => visitor.visit_none(),
            RawValue::Integer(val) => visitor.visit_i64(val),
            RawValue::Real(val) => visitor.visit_f64(val),
            RawValue::Text(val) => visitor.visit_string(val),
            RawValue::Blob(val) => visitor.visit_seq(val.into_deserializer()),
        }
    }
}

impl<'de> Deserializer<'de> for TableRowValue<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let val = self.value()?;
        self.deserialize_any_helper(visitor, val)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value()? {
            RawValue::Integer(val) => visitor.visit_bool(val != 0),
            RawValue::Real(val) => visitor.visit_bool(val != 0.),
            val => self.deserialize_any_helper(visitor, val),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value()? {
            RawValue::Null => visitor.visit_f32(f32::NAN),
            val => self.deserialize_any_helper(visitor, val),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value()? {
            RawValue::Null => visitor.visit_f64(f64::NAN),
            val => self.deserialize_any_helper(visitor, val),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_byte_buf(match self.value()? {
            RawValue::Blob(vec) => vec,
            _ => unreachable!(),
        })
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value()? {
            RawValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value()? {
            RawValue::Null => visitor.visit_unit(),
            val => self.deserialize_any_helper(visitor, val),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        match self.value()? {
            RawValue::Text(ref val) if val == name => visitor.visit_unit(),
            val => self.deserialize_any_helper(visitor, val),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(TableRowEnumAccess(match self.value()? {
            RawValue::Text(s) => s,
            _ => unreachable!(),
        }))
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i64 u8 u16 u32 u64 char str string bytes
        newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct TableRowMapAccess<'row, 'cols> {
    idx: usize,
    de: TableRowDeserializer<'row, 'cols>,
}

impl<'de> MapAccess<'de> for TableRowMapAccess<'de, '_> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.idx >= self.de.columns.len() {
            Ok(None)
        } else {
            let column = self.de.columns[self.idx].as_str();
            seed.deserialize(column.into_deserializer())
                .map(Some)
                .map_err(|e| add_field_to_error(e, column))
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let out = seed
            .deserialize(TableRowValue {
                idx: self.idx,
                row: self.de.row,
            })
            .map_err(|e| add_field_to_error(e, &self.de.columns[self.idx]));
        self.idx += 1;
        out
    }
}

struct TableRowSeqAccess<'row, 'cols> {
    idx: usize,
    de: TableRowDeserializer<'row, 'cols>,
}

impl<'de> SeqAccess<'de> for TableRowSeqAccess<'de, '_> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        let out = seed
            .deserialize(TableRowValue {
                idx: self.idx,
                row: self.de.row,
            })
            .map(Some)
            .map_err(|e| add_field_to_error(e, &self.de.columns[self.idx]));
        self.idx += 1;
        out
    }
}

struct TableRowEnumAccess(String);

impl<'de> EnumAccess<'de> for TableRowEnumAccess {
    type Error = Error;
    type Variant = TableRowVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        seed.deserialize(self.0.into_deserializer())
            .map(|v| (v, TableRowVariantAccess))
    }
}

struct TableRowVariantAccess;

impl<'de> VariantAccess<'de> for TableRowVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> Result<T::Value> {
        Err(Error::de_unsupported("newtype_variant"))
    }
    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::de_unsupported("tuple_variant"))
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::de_unsupported("struct_variant"))
    }
}

fn add_field_to_error(mut error: Error, error_column: &str) -> Error {
    if let Error::Serialization(column) = &mut error {
        *column = error_column.to_string();
    }
    error
}
