use core::fmt;

use nom_language::error::VerboseError;
use serde::{
    Deserialize,
    de::{self, Deserializer, MapAccess, Visitor},
};

use crate::config::{HoconError, ObjMap, ResolvedValue};

impl serde::de::Error for HoconError {
    fn custom<T: fmt::Display>(e: T) -> Self {
        HoconError::ParseError { msg: e.to_string() }
    }
}

pub struct HoconDeserializer<'a> {
    input: ResolvedValue<'a>,
}

pub fn from_str<'a: 'de, 'de, T>(s: &'a str) -> Result<T, HoconError>
where
    T: Deserialize<'de>,
{
    let deserializer = HoconDeserializer::from_str(s)?;
    T::deserialize(deserializer)
}

struct HoconObjectIter<'de> {
    iter: <ObjMap<'de> as IntoIterator>::IntoIter,
    item: Option<ResolvedValue<'de>>,
}

impl<'de> HoconObjectIter<'de> {
    pub fn new(input: ObjMap<'de>) -> Self {
        let iter = input.into_iter();
        HoconObjectIter { iter, item: None }
    }
}

impl<'de> MapAccess<'de> for HoconObjectIter<'de> {
    type Error = HoconError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            None => Ok(None),
            Some((path, value)) => {
                self.item = Some(value);
                let de: HoconDeserializer<'de> = HoconDeserializer {
                    input: ResolvedValue::String(path.clone()),
                };
                seed.deserialize(de).map(Some)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.item.take() {
            Some(value) => {
                // TODO: Avoid cloning
                let de = HoconDeserializer { input: value.clone() };
                seed.deserialize(de)
            }
            _ => Err(HoconError::ParseError {
                msg: "Invalid deser state, expected to have reference to map element".to_owned(),
            }),
        }
    }
}

impl<'a> HoconDeserializer<'a> {
    // By serde convetion we overlap with common from_str from methods
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'a str) -> Result<Self, HoconError> {
        let input = crate::parser::parse::<VerboseError<&'a str>>(input)?;
        let resolved = crate::config::resolve(input)?;
        Ok(HoconDeserializer { input: resolved.into() })
    }
}

impl<'a: 'de, 'de> Deserializer<'de> for HoconDeserializer<'a> {
    type Error = HoconError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.input {
            ResolvedValue::String(value) => visitor.visit_str(value),
            _ => Err(HoconError::ParseError {
                msg: "Expected string type".to_owned(),
            }),
        }
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ResolvedValue::Object(obj) => {
                let iter = HoconObjectIter::new(obj);
                visitor.visit_map(iter)
            }
            _ => Err(HoconError::ParseError {
                msg: "Expected object type".to_owned(),
            }),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestStruct {
        hello: String,
        world: String,
    }

    #[test]
    fn test_deserialize() {
        let s = r#"{ hello = "world", world = "hello" }"#;
        let t: TestStruct = super::from_str(s).unwrap();
        assert_eq!(
            t,
            TestStruct {
                hello: "world".to_string(),
                world: "hello".to_string()
            }
        );
    }
}
