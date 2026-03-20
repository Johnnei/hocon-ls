#[derive(Clone, Debug, PartialEq)]
pub enum HoconInclusion<'a> {
    File(&'a str),
    Url(&'a str),
    Classpath(&'a str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HoconField<'a> {
    Include(HoconInclusion<'a>),
    KeyValue(HoconString<'a>, HoconValue<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HoconString<'a> {
    Quoted(&'a str),
    Unquoted(&'a str),
}

impl <'a> HoconString<'a> {

    pub fn get_str(&self) -> &'a str {
        match self {
            Self::Quoted(v) => v,
            Self::Unquoted(v) => v,
        }
    }

}

/// Represents the concatenation of two hocon values while preserving the white space in between the
/// two values
#[derive(Clone, Debug, PartialEq)]
pub struct HoconConcatenation<'a> {
    pub a: HoconValue<'a>,
    pub whitespace: Option<&'a str>,
    pub b: HoconValue<'a>,
}

/// Represents a hocon value within the AST representation.
/// As this meant for AST the structure is kept close to the source document
#[derive(Clone, Debug, PartialEq)]
pub enum HoconValue<'a> {
    String(HoconString<'a>),
    Number(f64),
    Object(Vec<HoconField<'a>>),
    Array(Vec<HoconValue<'a>>),
    Boolean(bool),
    Null,
    Include(HoconInclusion<'a>),
    Concat(Box<HoconConcatenation<'a>>),
}
