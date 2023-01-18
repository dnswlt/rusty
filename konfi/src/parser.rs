use nom::{bytes::complete::take_while, error::ParseError, IResult};

// Whitespace parser.
fn sp<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let space_chars = " \t\r\n";
    take_while(move |c| space_chars.contains(c))(i)
}
