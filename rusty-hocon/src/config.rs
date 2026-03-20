use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Keys},
    ops::Deref,
};

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

type ObjMap<'a> = HashMap<Cow<'a, str>, ResolvedValue<'a>>;

pub enum ResolvedValue<'a> {
    String(Cow<'a, str>),
    Number(f64),
    Object(ObjMap<'a>),
    Array(Vec<ResolvedValue<'a>>),
    Boolean(bool),
    Null,
}

pub enum AccessError {
    Missing,
    ConvertionFailure,
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

    pub fn is_empty(&'a self) -> bool {
        self.fields.is_empty()
    }

    pub fn get_keys(&'a self) -> Keys<Cow<'a, str>, ResolvedValue<'a>> {
        self.fields.keys()
    }
}

pub fn resolve<'a>(ast: HoconValue<'a>) -> Result<Config<'a>, HoconError> {
    let fields = resolve_object(HashMap::new(), "", ast)?;
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
        HoconValue::Object(_) => ResolvedValue::Object(track_err(path, resolve_object(HashMap::new(), path, ast))?),
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
