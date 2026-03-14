use std::collections::HashMap;

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while, take_while_m_n},
    character::{anychar, complete::char, one_of},
    combinator::{all_consuming, map, not, opt, peek, recognize, value, verify},
    error::ParseError,
    multi::{many0, many1, separated_list0},
    number::complete::double,
    sequence::{delimited, preceded, terminated},
};
use nom_language::error::convert_error;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct HoconObject<'a> {
    pub data: HashMap<String, HoconValue<'a>>,
}

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
    HoconString(HoconString<'a>),
    HoconNumber(f64),
    HoconObject(Vec<HoconField<'a>>),
    HoconArray(Vec<HoconValue<'a>>),
    HoconBoolean(bool),
    HoconNull,
    HoconInclude(HoconInclusion<'a>),
    HoconConcat(Box<HoconConcatenation<'a>>),
}

/// Represents the various modes of failure while parsing or evaluating hocon files.
#[derive(Error, Debug, PartialEq)]
pub enum HoconError {
    // TODO Integrate better with nom error to get better parsing error docs
    #[error("Parse error")]
    ParseError { msg: String },
}

/// Parses the given input as a Hocon document into a Hocon AST.
pub fn parse<'a, E: ParseError<&'a str>>(input: &'a str) -> Result<HoconValue<'a>, HoconError> {
    let r = alt((empty_content, map((parse_root_object, opt(whitespace)), |(o, _)| o))).parse(input);
    match r {
        Ok(("", value)) => Ok(value),
        Ok((remainder, value)) => Err(HoconError::ParseError {
            msg: format!(
                "Failed to consume all data. parsed: {:?}, remainder: {}",
                value, remainder
            ),
        }),
        Err(nom::Err::Error(e)) => {
            let msg = convert_error(input, e);
            Err(HoconError::ParseError { msg })
        }
        _ => Err(HoconError::ParseError {
            msg: "Unknown error".to_string(),
        }),
    }
}

fn empty_content<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    map(all_consuming(whitespace), |_| HoconValue::HoconObject(vec![])).parse(input)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    let (input, _) = tag("null")(input)?;
    Ok((input, HoconValue::HoconNull))
}

fn boolean<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    let parse_true = value(HoconValue::HoconBoolean(true), tag("true"));
    let parse_false = value(HoconValue::HoconBoolean(false), tag("false"));
    alt((parse_true, parse_false)).parse(input)
}

fn is_hocon_whitespace(c: char) -> bool {
    c.is_whitespace()
        || c == '\t'
        || c == '\n'
        || c == '\u{000B}'
        || c == '\u{000C}'
        || c == '\r'
        || c == '\u{001C}'
        || c == '\u{001D}'
        || c == '\u{001E}'
        || c == '\u{001F}'
}

fn whitespace<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    let (input, _) = take_while(is_hocon_whitespace)(input)?;
    Ok((input, ()))
}

fn whitespace_same_line<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    let (input, ws) = take_while(|c| c != '\n' && is_hocon_whitespace(c)).parse(input)?;
    Ok((input, ws))
}

fn unquoted_string<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    recognize(many1((
        not(peek(tag("//"))),
        not(peek(verify(anychar, |&c| {
            is_hocon_whitespace(c)
                || c == '$'
                || c == '"'
                || c == '{'
                || c == '}'
                || c == '['
                || c == ']'
                || c == ':'
                || c == '='
                || c == ','
                || c == '+'
                || c == '#'
                || c == '`'
                || c == '^'
                || c == '?'
                || c == '!'
                || c == '@'
                || c == '*'
                || c == '&'
                || c == '\\'
        }))),
        anychar,
    )))
    .parse(input)
}

fn quoted_string<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    delimited(
        tag("\""),
        recognize(many0(alt((
            map(
                (tag("\\u"), take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit())),
                |_| (),
            ),
            map((tag("\\"), one_of("\"\\/bfnrt")), |_| ()),
            map((not(one_of("\"\\")), anychar), |_| ()),
        )))),
        tag("\""),
    )
    .parse(input)
}

fn parse_string<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconString<'a>, E> {
    alt((
        map(unquoted_string, HoconString::Unquoted),
        map(quoted_string, HoconString::Quoted),
    ))
    .parse(input)
}

fn number<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    map(double, HoconValue::HoconNumber).parse(input)
}

fn include<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconInclusion<'a>, E> {
    let (remainder, (_, _, (_, v))) = (
        tag("include"),
        whitespace,
        alt((
            (
                tag("url"),
                delimited(char('('), map(quoted_string, HoconInclusion::Url), char(')')),
            ),
            (
                tag("file"),
                delimited(char('('), map(quoted_string, HoconInclusion::File), char(')')),
            ),
            (
                tag("classpath"),
                delimited(char('('), map(quoted_string, HoconInclusion::Classpath), char(')')),
            ),
        )),
    )
        .parse(input)?;
    Ok((remainder, v))
}

fn parse_value<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    let (remainder, a) = alt((
        null,
        map(include, HoconValue::HoconInclude),
        boolean,
        number,
        array,
        parse_object,
        map(parse_string, HoconValue::HoconString),
    ))
    .parse(input)?;

    fn concat_whitespace<'a, 'b, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Option<&'a str>, E> {
        map(whitespace_same_line, |ws| if ws.is_empty() { None } else { Some(ws) }).parse(input)
    }

    let (remainder, maybe_concat) = opt((concat_whitespace, parse_value)).parse(remainder)?;
    let result = match maybe_concat {
        None => a,
        Some((ws, b)) => HoconValue::HoconConcat(Box::new(HoconConcatenation { a, whitespace: ws, b })),
    };

    Ok((remainder, result))
}

fn next_element_whitespace<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    map(
        opt((
            take_while(|c| c != '\n' && c != ',' && is_hocon_whitespace(c)),
            char(','),
        )),
        |_| (),
    )
    .parse(input)
}

fn key_value<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (HoconString<'a>, HoconValue<'a>), E> {
    fn separator<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
        map(alt((char(':'), char('='), peek(char('{')))), |_| ()).parse(input)
    }

    let (input, (_, path, _, _, _, value, _)) = (
        whitespace,
        parse_string,
        whitespace,
        separator,
        whitespace,
        parse_value,
        next_element_whitespace,
    )
        .parse(input)?;
    Ok((input, (path, value)))
}

fn object_field<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconField<'a>, E> {
    alt((
        map(include, HoconField::Include),
        map(key_value, |(k, v)| HoconField::KeyValue(k, v)),
    ))
    .parse(input)
}

fn array<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    fn array_element<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
        preceded(whitespace, parse_value).parse(input)
    }

    delimited(
        (char('['), whitespace),
        map(
            terminated(
                separated_list0((whitespace_same_line, alt((char(','), char('\n')))), array_element),
                opt((whitespace, char(','))),
            ),
            HoconValue::HoconArray,
        ),
        (whitespace, char(']')),
    )
    .parse(input)
}

fn parse_root_object<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    map(
        alt((
            delimited(char('{'), many0(object_field), (whitespace, char('}'))),
            many1(object_field),
        )),
        HoconValue::HoconObject,
    )
    .parse(input)
}

fn parse_object<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, HoconValue<'a>, E> {
    map(
        delimited(char('{'), many0(object_field), (whitespace, char('}'))),
        HoconValue::HoconObject,
    )
    .parse(input)
}

#[cfg(test)]
mod tests {

    use nom_language::error::VerboseError;
    use rstest::rstest;

    use super::*;
    use crate::parser::HoconValue;

    #[test]
    fn test_null() {
        assert_eq!(null::<VerboseError<&str>>("null"), Ok(("", HoconValue::HoconNull)));
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(
            boolean::<VerboseError<&str>>("true"),
            Ok(("", HoconValue::HoconBoolean(true)))
        );
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(
            boolean::<VerboseError<&str>>("false"),
            Ok(("", HoconValue::HoconBoolean(false)))
        );
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(whitespace::<VerboseError<&str>>("     test"), Ok(("test", ())));
    }

    #[test]
    fn test_unquoted_string() {
        assert_eq!(unquoted_string::<VerboseError<&str>>("test"), Ok(("", "test")));
    }

    #[test]
    fn test_unquoted_string_with_trailing_slash_comment() {
        assert_eq!(
            unquoted_string::<VerboseError<&str>>("test// hello"),
            Ok(("// hello", "test"))
        );
    }

    #[test]
    fn test_unquoted_string_with_trailing_pound_comment() {
        assert_eq!(
            unquoted_string::<VerboseError<&str>>("test# hello"),
            Ok(("# hello", "test"))
        );
    }

    #[rstest]
    #[case::unquote_unquote("one = two three", false, false)]
    #[case::quote_unquote("one =\"two\" three", true, false)]
    #[case::unquote_quote("one = two \"three\"", false, true)]
    #[case::quote_quote("one = \"two\" \"three\"", true, true)]
    fn test_concat_unquoted_strings(#[case] input: &str, #[case] a_quoted: bool, #[case] b_quoted: bool) {
        let a = if a_quoted {
            HoconString::Quoted("two")
        } else {
            HoconString::Unquoted("two")
        };
        let b = if b_quoted {
            HoconString::Quoted("three")
        } else {
            HoconString::Unquoted("three")
        };

        let expected = vec![HoconField::KeyValue(
            HoconString::Unquoted("one"),
            HoconValue::HoconConcat(Box::new(HoconConcatenation {
                a: HoconValue::HoconString(a),
                whitespace: Some(" "),
                b: HoconValue::HoconString(b),
            })),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(input),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[rstest]
    #[case::string_boolean(
        "one false",
        HoconValue::HoconString(HoconString::Unquoted("one")),
        HoconValue::HoconBoolean(false)
    )]
    #[case::string_number(
        "one 12",
        HoconValue::HoconString(HoconString::Unquoted("one")),
        HoconValue::HoconNumber(12.0)
    )]
    fn test_concat_simple_types(#[case] input: &str, #[case] a: HoconValue, #[case] b: HoconValue) {
        let expected = HoconValue::HoconConcat(Box::new(HoconConcatenation {
            a,
            whitespace: Some(" "),
            b,
        }));
        assert_eq!(parse_value::<VerboseError<&str>>(input), Ok(("", expected)));
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(quoted_string::<VerboseError<&str>>("\"test\""), Ok(("", "test")));
    }

    #[test]
    fn test_quoted_string_empty() {
        assert_eq!(quoted_string::<VerboseError<&str>>("\"\""), Ok(("", "")));
    }

    #[test]
    fn test_quoted_string_unicode_escape() {
        assert_eq!(quoted_string::<VerboseError<&str>>("\"\\u12AB\""), Ok(("", "\\u12AB")));
    }

    #[test]
    fn test_quoted_string_with_escaped_quote() {
        assert_eq!(
            quoted_string::<VerboseError<&str>>(r#""testy \" test""#),
            Ok(("", "testy \\\" test"))
        );
    }

    #[test]
    fn test_key_value() {
        assert_eq!(
            key_value::<VerboseError<&str>>("test = true"),
            Ok(("", (HoconString::Unquoted("test"), HoconValue::HoconBoolean(true))))
        );
    }

    #[test]
    fn test_key_value_multiple_fields() {
        let content = r#""hello": "world", "world": "hello""#;
        assert_eq!(
            key_value::<VerboseError<&str>>(content),
            Ok((
                r#" "world": "hello""#,
                (
                    HoconString::Quoted("hello"),
                    HoconValue::HoconString(HoconString::Quoted("world"))
                )
            ))
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(
            number::<VerboseError<&str>>("42"),
            Ok(("", HoconValue::HoconNumber(42f64)))
        );
    }

    #[rstest]
    #[case::no_whitespace("[1,2,3]")]
    #[case::dense_whitespace("[1, 2, 3]")]
    #[case::usual_whitespace("[ 1, 2, 3 ]")]
    #[case::max_whitespace("[ 1 , 2 , 3 ]")]
    #[case::trailing_whitespace("[ 1 , 2 , 3 , ]")]
    #[case::new_lines("[\n1 \n2 \n3\n]")]
    #[case::mix_separators("[\n1 \n2, 3]")]
    fn test_array(#[case] input: &str) {
        let expected_data = vec![
            HoconValue::HoconNumber(1f64),
            HoconValue::HoconNumber(2f64),
            HoconValue::HoconNumber(3f64),
        ];
        assert_eq!(
            array::<VerboseError<&str>>(input),
            Ok(("", HoconValue::HoconArray(expected_data)))
        );
    }

    #[test]
    fn test_array_trailing_comma() {
        assert_eq!(
            array::<VerboseError<&str>>("[1,2,3,]"),
            Ok((
                "",
                HoconValue::HoconArray(vec![
                    HoconValue::HoconNumber(1.0),
                    HoconValue::HoconNumber(2.0),
                    HoconValue::HoconNumber(3.0),
                ])
            ))
        );
    }

    #[test]
    fn test_array_new_lines_equate_commas() {
        assert_eq!(
            array::<VerboseError<&str>>("[1\n2\n3]"),
            array::<VerboseError<&str>>("[1,2,3]")
        );
    }

    #[test]
    fn parse_array_concat() {
        let content = "a : [ 1, 2 ] [ 3, 4 ]";
        let expected = vec![HoconField::KeyValue(
            HoconString::Unquoted("a"),
            HoconValue::HoconConcat(Box::new(HoconConcatenation {
                a: HoconValue::HoconArray(vec![HoconValue::HoconNumber(1.0), HoconValue::HoconNumber(2.0)]),
                whitespace: Some(" "),
                b: HoconValue::HoconArray(vec![HoconValue::HoconNumber(3.0), HoconValue::HoconNumber(4.0)]),
            })),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_basic_json_object() {
        let content = r#"{ "hello": "world" }"#;
        let expected = vec![HoconField::KeyValue(
            HoconString::Quoted("hello"),
            HoconValue::HoconString(HoconString::Quoted("world")),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_json_object_with_two_keys() {
        let content = r#"{ "hello": "world", "world": "hello" }"#;
        let expected = vec![
            HoconField::KeyValue(
                HoconString::Quoted("hello"),
                HoconValue::HoconString(HoconString::Quoted("world")),
            ),
            HoconField::KeyValue(
                HoconString::Quoted("world"),
                HoconValue::HoconString(HoconString::Quoted("hello")),
            ),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_json_object_with_two_quoted_keys_multiline() {
        let content = r#"{
            "hello": "world",
            "world": "hello"
        }"#;
        let expected = vec![
            HoconField::KeyValue(
                HoconString::Quoted("hello"),
                HoconValue::HoconString(HoconString::Quoted("world")),
            ),
            HoconField::KeyValue(
                HoconString::Quoted("world"),
                HoconValue::HoconString(HoconString::Quoted("hello")),
            ),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_hocon_object_with_two_unquoted_keys() {
        let content = r#"{
            hello: "world"
            world: "hello"
        }"#;
        let expected = vec![
            HoconField::KeyValue(
                HoconString::Unquoted("hello"),
                HoconValue::HoconString(HoconString::Quoted("world")),
            ),
            HoconField::KeyValue(
                HoconString::Unquoted("world"),
                HoconValue::HoconString(HoconString::Quoted("hello")),
            ),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_object_with_concatenation() {
        let content = "a : { b : 1 } { c : 2 }";
        let expected = vec![HoconField::KeyValue(
            HoconString::Unquoted("a"),
            HoconValue::HoconConcat(Box::new(HoconConcatenation {
                a: HoconValue::HoconObject(vec![HoconField::KeyValue(
                    HoconString::Unquoted("b"),
                    HoconValue::HoconNumber(1.0),
                )]),
                whitespace: Some(" "),
                b: HoconValue::HoconObject(vec![HoconField::KeyValue(
                    HoconString::Unquoted("c"),
                    HoconValue::HoconNumber(2.0),
                )]),
            })),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_inclusion() {
        let content = r#"include file("test.conf")"#;
        let expected = HoconInclusion::File("test.conf");
        assert_eq!(include::<VerboseError<&str>>(content), Ok(("", expected)));
    }

    #[test]
    fn parse_inclusion_merge() {
        let content = r#"include file("test.conf")
            hello = "world"
        "#;
        let expected = vec![
            HoconField::Include(HoconInclusion::File("test.conf")),
            HoconField::KeyValue(
                HoconString::Unquoted("hello"),
                HoconValue::HoconString(HoconString::Quoted("world")),
            ),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn pares_inclusion_value() {
        let content = r#"
            hello = include file("test.conf")
        "#;
        let expected = vec![HoconField::KeyValue(
            HoconString::Unquoted("hello"),
            HoconValue::HoconInclude(HoconInclusion::File("test.conf")),
        )];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn parse_empty_line() {
        assert_eq!(
            empty_content::<VerboseError<&str>>(""),
            Ok(("", HoconValue::HoconObject(vec![])))
        );
        assert_eq!(parse::<VerboseError<&str>>(""), Ok(HoconValue::HoconObject(vec![])));
    }

    #[test]
    fn parse_empty_line_whitespace() {
        assert_eq!(parse::<VerboseError<&str>>("   "), Ok(HoconValue::HoconObject(vec![])));
    }

    #[test]
    fn parse_empty_multiline() {
        let content = r#"

        "#;
        let expected = vec![];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }

    #[test]
    fn test_scenario_nested_object() {
        let content = r#"
        database: {
          hostname: "localhost"
          username: "user"
          password: "secret"
        }
        some_string: "1h"
        some_number: 1148
        "#;

        let expected = vec![
            HoconField::KeyValue(
                HoconString::Unquoted("database"),
                HoconValue::HoconObject(vec![
                    HoconField::KeyValue(
                        HoconString::Unquoted("hostname"),
                        HoconValue::HoconString(HoconString::Quoted("localhost")),
                    ),
                    HoconField::KeyValue(
                        HoconString::Unquoted("username"),
                        HoconValue::HoconString(HoconString::Quoted("user")),
                    ),
                    HoconField::KeyValue(
                        HoconString::Unquoted("password"),
                        HoconValue::HoconString(HoconString::Quoted("secret")),
                    ),
                ]),
            ),
            HoconField::KeyValue(
                HoconString::Unquoted("some_string"),
                HoconValue::HoconString(HoconString::Quoted("1h")),
            ),
            HoconField::KeyValue(HoconString::Unquoted("some_number"), HoconValue::HoconNumber(1148.0)),
        ];
        assert_eq!(
            parse::<VerboseError<&str>>(content),
            Ok(HoconValue::HoconObject(expected))
        );
    }
}
