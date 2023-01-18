use nom::branch::alt;
use nom::bytes::streaming::{is_not, take_while_m_n};
use nom::character::streaming::{char, multispace1};
use nom::combinator::{map, map_opt, map_res, value, verify};
use nom::error::{FromExternalError, ParseError};
use nom::multi::fold_many0;
use nom::sequence::{delimited, preceded};
use nom::IResult;

// This code is essentially a copy of
// https://github.com/rust-bakery/nom/blob/main/examples/string.rs.

/// Parse a unicode sequence, of the form u{XXXX}, where XXXX is 1 to 6
/// hexadecimal numerals.
fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());
    let parse_delimited_hex = preceded(char('u'), delimited(char('{'), parse_hex, char('}')));
    let parse_u32 = map_res(parse_delimited_hex, move |hex| u32::from_str_radix(hex, 16));

    map_opt(parse_u32, std::char::from_u32)(input)
}

/// Parse an escaped character: \n, \t, \r, \u{00AC}, etc.
fn parse_escaped_char<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    preceded(
        char('\\'),
        alt((
            parse_unicode,
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
            value('\u{08}', char('b')),
            value('\u{0C}', char('f')),
            value('\\', char('\\')),
            value('\'', char('\'')),
            value('/', char('/')),
            value('"', char('"')),
        )),
    )(input)
}

/// Parse a backslash, followed by any amount of whitespace, including line breaks.
fn parse_escaped_whitespace<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    preceded(char('\\'), multispace1)(input)
}

/// Parse a non-empty block of text that doesn't include \ or "
fn parse_literal<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    verify(is_not("\"\\"), |s: &str| !s.is_empty())(input)
}

/// A string fragment contains a fragment of a string being parsed: either
/// a non-empty Literal (a series of non-escaped characters), a single
/// parsed escaped character, or a block of escaped whitespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    Literal(&'a str),
    EscapedChar(char),
    EscapedWS,
}

/// Combine parse_literal, parse_escaped_whitespace, and parse_escaped_char
/// into a StringFragment.
fn parse_fragment<'a, E>(input: &'a str) -> IResult<&'a str, StringFragment<'a>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((
        map(parse_literal, StringFragment::Literal),
        map(parse_escaped_char, StringFragment::EscapedChar),
        value(StringFragment::EscapedWS, parse_escaped_whitespace),
    ))(input)
}

/// Parse a string.
pub fn parse_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    let build_string = fold_many0(
        parse_fragment,
        String::new,
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedChar(c) => string.push(c),
                StringFragment::EscapedWS => {}
            }
            string
        },
    );

    delimited(char('"'), build_string, char('"'))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_string_simple() {
        let p = |s| parse_string::<nom::error::Error<&str>>(s);
        assert_eq!(p("\"foo\""), Ok(("", String::from("foo"))));
        assert_eq!(p("\"   \""), Ok(("", String::from("   "))));
        // Like in Rust, ' can be, but doesn't have to be escaped:
        assert_eq!(p(r#""O\'Hare""#), Ok(("", String::from("O\'Hare"))));
        assert_eq!(p(r#""O'Hare""#), Ok(("", String::from("O'Hare"))));
        assert_eq!(
            p(r#""123\n456\n789\n""#),
            Ok(("", String::from("123\n456\n789\n")))
        );
        assert_eq!(p(r#""\\begin{foo}""#), Ok(("", String::from("\\begin{foo}"))));
    }

    #[test]
    fn parse_string_escaped_unicode() {
        let p = |s| parse_string::<nom::error::Error<&str>>(s);
        assert_eq!(p(r#""S\u{00F6}gestra\u{df}e""#), Ok(("", String::from("Sögestraße"))));

    }

    #[test]
    fn parse_string_escaped_whitespace() {
        let s = "foo\
        bar";
        assert_eq!(
            parse_string::<nom::error::Error<&str>>(&format!("\"{}\"", s)),
            Ok(("", String::from("foobar")))
        );
        assert_eq!(
            parse_string::<nom::error::Error<&str>>("\"abc\\   \n   def\\   ghi \""),
            Ok(("", String::from("abcdefghi ")))
        );
    }
}
