use core::fmt;

use nom_language::error::VerboseError;
use serde::de::{self, DeserializeOwned, Deserializer, MapAccess, Visitor};

use crate::parser::{HoconError, HoconField, HoconString, HoconValue};

impl serde::de::Error for HoconError {
    fn custom<T: fmt::Display>(e: T) -> Self {
        HoconError::ParseError { msg: e.to_string() }
    }
}

struct HoconObjectIter<'a, 'de: 'a> {
    de: &'a mut HoconDeserializer<'de>,
    first: bool,
}

impl<'a, 'de> HoconObjectIter<'a, 'de> {
    pub fn new(de: &'a mut HoconDeserializer<'de>) -> Self {
        HoconObjectIter { de, first: true }
    }
}

impl<'de, 'a> MapAccess<'de> for HoconObjectIter<'a, 'de> {
    type Error = HoconError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match &mut self.de.input {
            HoconValue::HoconObject(elements) => {
                if !self.first {
                    elements.remove(0);
                } else {
                    self.first = false;
                }

                if elements.is_empty() {
                    Ok(None)
                } else {
                    seed.deserialize(&mut *self.de).map(Some)
                }
            }
            _ => Err(HoconError::ParseError {
                msg: "Expected object type".to_owned(),
            }),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match &mut self.de.input {
            HoconValue::HoconObject(map) => {
                if let Some(HoconField::KeyValue(_, value)) = map.first() {
                    let mut value_deser = HoconDeserializer {
                        input: value.to_owned(),
                    };
                    seed.deserialize(&mut value_deser)
                } else {
                    Err(HoconError::ParseError {
                        msg: "Expected non-empty map".to_owned(),
                    })
                }
            }
            _ => Err(HoconError::ParseError {
                msg: "Expcected object type".to_owned(),
            }),
        }
    }
}

pub struct HoconDeserializer<'de> {
    input: HoconValue<'de>,
}

impl<'de> HoconDeserializer<'de> {
    pub fn from_str(input: &'de str) -> Result<Self, HoconError> {
        let input = crate::parser::parse::<VerboseError<&'de str>>(input)?;
        Ok(HoconDeserializer { input })
    }
}

pub fn from_str<T>(s: &str) -> Result<T, HoconError>
where
    // TODO: Figure out why lifetime doesn't outlast the deserializer when not using the owned
    //       type.
    T: DeserializeOwned,
{
    let mut deserializer = HoconDeserializer::from_str(s)?;
    T::deserialize(&mut deserializer)
}

impl<'de, 'a> Deserializer<'de> for &'a mut HoconDeserializer<'de> {
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
        match self.input {
            HoconValue::HoconString(HoconString::Quoted(value)) => visitor.visit_borrowed_str(value),
            HoconValue::HoconString(HoconString::Unquoted(value)) => visitor.visit_borrowed_str(value),
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
        match &mut self.input {
            HoconValue::HoconObject(_) => {
                let object_iter = HoconObjectIter::new(self);
                visitor.visit_map(object_iter)
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
        match &mut self.input {
            HoconValue::HoconObject(ref mut map) => match map.first().take().map(|s| s.to_owned()) {
                Some(HoconField::KeyValue(HoconString::Quoted(key), _)) => visitor.visit_borrowed_str(key),
                Some(HoconField::KeyValue(HoconString::Unquoted(key), _)) => visitor.visit_borrowed_str(key),
                _ => Err(HoconError::ParseError {
                    msg: "Expected non-empty object".to_owned(),
                }),
            },
            _ => Err(HoconError::ParseError {
                msg: "Expected object type".to_owned(),
            }),
        }
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
