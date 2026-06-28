use std::{borrow::Cow, collections::HashMap, ops::Deref};

use thiserror::Error;

use crate::ast::{HoconConcatenation, HoconField, HoconValue};

/// Represents the various modes of failure while parsing or evaluating hocon files.
#[derive(Error, Debug, PartialEq)]
pub enum HoconError {
    // TODO Integrate better with nom error to get better parsing error docs
    #[error("Parse error")]
    ParseError { msg: String },
    #[error("Failed to resolve configuration")]
    ResolveError { path: Vec<String>, msg: String },
}

#[derive(Clone, Default)]
pub struct ObjMap<'a> {
    map: HashMap<Cow<'a, str>, ResolvedValue<'a>>,
}

impl<'a> ObjMap<'a> {
    pub fn get(&self, path: &str) -> Option<&ResolvedValue<'a>> {
        self.map.get(path)
    }

    pub fn insert(&mut self, path: Cow<'a, str>, value: ResolvedValue<'a>) {
        self.map.insert(path, value);
    }

    pub fn remove(&mut self, path: &Cow<'a, str>) -> Option<ResolvedValue<'a>> {
        self.map.remove(path)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<'a> IntoIterator for &'a ObjMap<'a> {
    type Item = (&'a Cow<'a, str>, &'a ResolvedValue<'a>);

    type IntoIter = std::collections::hash_map::Iter<'a, Cow<'a, str>, ResolvedValue<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a> IntoIterator for &'a mut ObjMap<'a> {
    type Item = (&'a Cow<'a, str>, &'a mut ResolvedValue<'a>);

    type IntoIter = std::collections::hash_map::IterMut<'a, Cow<'a, str>, ResolvedValue<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}

impl<'a> IntoIterator for ObjMap<'a> {
    type Item = (Cow<'a, str>, ResolvedValue<'a>);

    type IntoIter = std::collections::hash_map::IntoIter<Cow<'a, str>, ResolvedValue<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

#[derive(Clone)]
pub enum ResolvedValue<'a> {
    String(Cow<'a, str>),
    Number(f64),
    Object(ObjMap<'a>),
    Array(Vec<ResolvedValue<'a>>),
    Boolean(bool),
    Null,
}

#[derive(Error, Debug, PartialEq)]
pub enum AccessError {
    #[error("Path is missing")]
    Missing,
    #[error("Value cannot be converted to expected type")]
    ConvertionFailure,
    #[error("Value at path is of different type")]
    TypeMismatch,
}

enum Ref<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Ref::Borrowed(v) => v,
            Ref::Owned(v) => v,
        }
    }
}

impl<'a> From<Config<'a>> for ResolvedValue<'a> {
    fn from(value: Config<'a>) -> Self {
        match value.fields {
            // Can I avoid cloning?
            Ref::Borrowed(value) => ResolvedValue::Object(value.to_owned()),
            Ref::Owned(value) => ResolvedValue::Object(value),
        }
    }
}

pub struct Config<'a> {
    fields: Ref<'a, ObjMap<'a>>,
}

impl<'a> Config<'a> {
    pub fn get_config(&'a self, path: &str) -> Result<Config<'a>, AccessError> {
        match self.fields.get(path) {
            Some(ResolvedValue::Object(fields)) => Ok(Config {
                fields: Ref::Borrowed(fields),
            }),
            Some(_) => Err(AccessError::ConvertionFailure),
            None => Err(AccessError::Missing),
        }
    }

    #[inline]
    pub fn get<T: TryFrom<&'a ResolvedValue<'a>>>(&'a self, path: &str) -> T {
        self.try_get(path).unwrap()
    }

    pub fn try_get<T: TryFrom<&'a ResolvedValue<'a>>>(&'a self, path: &str) -> Result<T, AccessError> {
        match self.fields.get(path) {
            Some(value) => match T::try_from(value) {
                Ok(r) => Ok(r),
                Err(_) => Err(AccessError::TypeMismatch),
            },
            None => Err(AccessError::Missing),
        }
    }

    pub fn is_empty(&'a self) -> bool {
        self.fields.is_empty()
    }
}

pub fn resolve<'a>(ast: HoconValue<'a>) -> Result<Config<'a>, HoconError> {
    let fields = resolve_object(ObjMap::default(), "", ast)?;
    Ok(Config {
        fields: Ref::Owned(fields),
    })
}

fn resolve_object<'a>(mut obj: ObjMap<'a>, path: &str, ast: HoconValue<'a>) -> Result<ObjMap<'a>, HoconError> {
    match ast {
        HoconValue::Object(fields) => {
            for field in fields {
                match field {
                    HoconField::Include(_) => todo!(),
                    HoconField::KeyValue(key, value) => {
                        let key = key.get_str();
                        let value = track_err(key, resolve_value(key, value))?;
                        obj.insert(Cow::Borrowed(key), value);
                    }
                }
            }
            Ok(obj)
        }
        _ => Err(HoconError::ResolveError {
            path: vec![path.to_owned()],
            msg: format!("Expected object but found: {:?}", ast),
        }),
    }
}

fn resolve_value<'a>(path: &str, ast: HoconValue<'a>) -> Result<ResolvedValue<'a>, HoconError> {
    Ok(match ast {
        HoconValue::Null => ResolvedValue::Null,
        HoconValue::Boolean(b) => ResolvedValue::Boolean(b),
        HoconValue::Number(f) => ResolvedValue::Number(f),
        HoconValue::String(s) => ResolvedValue::String(Cow::Borrowed(s.get_str())),
        HoconValue::Array(array) => {
            let mut resolved = Vec::new();
            for item in array {
                resolved.push(track_err(path, resolve_value(path, item))?);
            }
            ResolvedValue::Array(resolved)
        }
        HoconValue::Object(_) => ResolvedValue::Object(track_err(path, resolve_object(ObjMap::default(), path, ast))?),
        HoconValue::Concat(c) => track_err(path, concat(path, *c))?,
        HoconValue::Include(_) => todo!(),
    })
}

fn concat<'a>(path: &str, concat: HoconConcatenation<'a>) -> Result<ResolvedValue<'a>, HoconError> {
    let a = track_err(path, resolve_value(path, concat.a))?;
    let b = track_err(path, resolve_value(path, concat.b))?;

    Ok(match (a, b) {
        (ResolvedValue::Object(mut a_fields), ResolvedValue::Object(b_fields)) => {
            for (key, value) in b_fields {
                a_fields.insert(key, value);
            }
            ResolvedValue::Object(a_fields)
        }
        (ResolvedValue::Array(mut a_values), ResolvedValue::Array(b_values)) => {
            a_values.extend(b_values);
            ResolvedValue::Array(a_values)
        }
        (a, b) => {
            // String promotion should fail if non-concatable types are merged
            let mut combined = track_err(path, promote_to_string(path, a))?;
            let b = track_err(path, promote_to_string(path, b))?;
            if let Some(whitespace) = concat.whitespace {
                combined.to_mut().push_str(whitespace);
            };
            combined.to_mut().push_str(&b);
            ResolvedValue::String(combined)
        }
    })
}

fn track_err<T>(current_path: &str, v: Result<T, HoconError>) -> Result<T, HoconError> {
    match v {
        Err(HoconError::ResolveError { mut path, msg }) => {
            path.push(current_path.to_owned());
            Err(HoconError::ResolveError { path, msg })
        }
        _ => v,
    }
}

fn promote_to_string<'a>(path: &str, value: ResolvedValue<'a>) -> Result<Cow<'a, str>, HoconError> {
    match value {
        ResolvedValue::Object(_) => Err(HoconError::ResolveError {
            path: vec![path.to_owned()],
            msg: "Can't stringify object for string concatenation".to_owned(),
        }),
        ResolvedValue::Array(_) => Err(HoconError::ResolveError {
            path: vec![path.to_owned()],
            msg: "Can't stringify array for string concatenation".to_owned(),
        }),
        ResolvedValue::String(string) => Ok(string),
        ResolvedValue::Boolean(b) => {
            if b {
                Ok(Cow::Owned(String::from("true")))
            } else {
                Ok(Cow::Owned(String::from("false")))
            }
        }
        ResolvedValue::Null => Ok(Cow::Owned(String::from("null"))),
        ResolvedValue::Number(n) => Ok(Cow::Owned(n.to_string())),
    }
}

impl<'a> TryFrom<&'a ResolvedValue<'a>> for i32 {
    type Error = AccessError;

    fn try_from(value: &'a ResolvedValue<'a>) -> Result<Self, Self::Error> {
        if let ResolvedValue::Number(n) = value {
            Ok(*n as i32)
        } else {
            Err(AccessError::TypeMismatch)
        }
    }
}

impl<'a> TryFrom<&'a ResolvedValue<'a>> for &'a str {
    type Error = AccessError;

    fn try_from(value: &'a ResolvedValue<'a>) -> Result<Self, Self::Error> {
        if let ResolvedValue::String(n) = value {
            Ok(n)
        } else {
            Err(AccessError::TypeMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{HoconConcatenation, HoconField, HoconString, HoconValue},
        config::resolve,
    };

    #[test]
    fn test_string() {
        let input = HoconValue::Object(vec![HoconField::KeyValue(
            HoconString::Unquoted("one"),
            HoconValue::String(HoconString::Unquoted("one")),
        )]);
        let conf = resolve(input).unwrap();
        assert_eq!(conf.get::<&str>("one"), "one");
    }

    #[test]
    fn test_i32() {
        let input = HoconValue::Object(vec![HoconField::KeyValue(
            HoconString::Unquoted("one"),
            HoconValue::Number(42f64),
        )]);
        let conf = resolve(input).unwrap();
        assert_eq!(conf.get::<i32>("one"), 42);
    }

    #[test]
    fn test_string_concat() {
        let input = HoconValue::Object(vec![HoconField::KeyValue(
            HoconString::Unquoted("one"),
            HoconValue::Concat(Box::new(HoconConcatenation {
                a: HoconValue::String(HoconString::Unquoted("one")),
                whitespace: Some(" "),
                b: HoconValue::String(HoconString::Quoted("two")),
            })),
        )]);
        let conf = resolve(input).unwrap();
        assert_eq!(conf.get::<&str>("one"), "one two");
    }
}
