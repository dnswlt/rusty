use crate::ast;
use std::num::ParseIntError;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace0, one_of},
    combinator::{map, map_res, opt, recognize},
    error::{FromExternalError, ParseError, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    Finish, IResult,
};

// Whitespace parser.
fn sp<'a, E>(i: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str>,
{
    let space_chars = " \t\r\n";
    take_while(move |c| space_chars.contains(c))(i)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    E: ParseError<&'a str> + 'a,
    F: Fn(&'a str) -> IResult<&'a str, O, E> + 'a,
{
    delimited(multispace0, inner, multispace0)
}

// Parse whitespace including at least one newline (for record fields).
fn eol<'a, E>(i: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str>,
{
    let (i, _) = many0(one_of("\t "))(i)?;
    alt((tag("\r\n"), tag("\n")))(i)
}

fn int_literal<'a, E>(input: &'a str) -> IResult<&str, ast::Literal, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>,
{
    map_res(
        recognize(pair(
            opt(one_of("+-")),
            many1(terminated(one_of("0123456789"), many0(char('_')))),
        )),
        |res: &str| i64::from_str_radix(&str::replace(&res, "_", ""), 10).map(ast::Literal::Int),
    )(input)
}

fn var<'a, E>(input: &'a str) -> IResult<&str, ast::Var, E>
where
    E: ParseError<&'a str>,
{
    map(
        recognize(pair(
            take_while1(|c: char| c.is_alphabetic() || c == '_'),
            take_while(|c: char| c.is_alphanumeric() || c == '_'),
        )),
        |v: &str| ast::Var {
            name: String::from(v),
        },
    )(input)
}

fn binop<'a, E>(input: &'a str) -> IResult<&str, ast::BinOp, E>
where
    E: ParseError<&'a str>,
{
    alt((
        map(one_of("+-*/"), |c| match c {
            '+' => ast::BinOp::Plus,
            '-' => ast::BinOp::Minus,
            '*' => ast::BinOp::Times,
            '/' => ast::BinOp::Div,
            _ => unreachable!("Not all binop characters covered"),
        }),
        map(tag("&&"), |_| ast::BinOp::LogicalAnd),
        map(tag("||"), |_| ast::BinOp::LogicalOr),
    ))(input)
}

fn atom<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    alt((
        delimited(char('('), ws(expr), char(')')),
        map(var, |v| Box::new(ast::Expr::Var(v))),
        map(int_literal, |l| Box::new(ast::Expr::Literal(l))),
    ))(input)
}

fn factor<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    alt((
        map(tuple((atom, ws(binop), factor)), |(a, op, c)| {
            Box::new(ast::Expr::BinExpr(a, op, c))
        }),
        atom,
    ))(input)
}

fn expr<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    alt((
        map(tuple((factor, ws(binop), expr)), |(a, op, c)| {
            Box::new(ast::Expr::BinExpr(a, op, c))
        }),
        factor,
    ))(input)
}

fn rec_field<'a, E>(input: &'a str) -> IResult<&str, ast::Field, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    map(pair(terminated(var, ws(char(':'))), expr), |(v, e)| {
        ast::Field { name: v, value: e }
    })(input)
}

fn rec<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    map(
        delimited(
            terminated(char('{'), multispace0),
            many0(delimited(multispace0, rec_field, eol)), //
            preceded(multispace0, char('}')),
        ),
        |fs| {
            Box::new(ast::Expr::Rec(ast::Rec {
                let_vars: vec![],
                fields: fs,
            }))
        },
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{
        error::{convert_error, VerboseError},
        Finish,
    };

    use super::*;

    #[test]
    fn sp_works() {
        assert_eq!(sp::<nom::error::Error<&str>>("  \t x"), Ok(("x", "  \t ")));
    }

    #[test]
    fn i64_works() {
        assert_eq!(
            int_literal::<nom::error::Error<&str>>("123"),
            Ok(("", ast::Literal::Int(123)))
        );
        assert_eq!(
            int_literal::<nom::error::Error<&str>>("+123"),
            Ok(("", ast::Literal::Int(123)))
        );
        assert_eq!(
            int_literal::<nom::error::Error<&str>>("-123"),
            Ok(("", ast::Literal::Int(-123)))
        );
    }

    fn mk_var(s: &str) -> ast::Var {
        ast::Var {
            name: String::from(s),
        }
    }

    fn mk_binop(a: Box<ast::Expr>, op: ast::BinOp, b: Box<ast::Expr>) -> Box<ast::Expr> {
        Box::new(ast::Expr::BinExpr(a, op, b))
    }

    #[test]
    fn var_works() {
        assert_eq!(var::<nom::error::Error<&str>>("y"), Ok(("", mk_var("y"))));
        assert_eq!(var::<nom::error::Error<&str>>("_"), Ok(("", mk_var("_"))));
        assert_eq!(var::<nom::error::Error<&str>>("_1"), Ok(("", mk_var("_1"))));
        assert_eq!(
            var::<nom::error::Error<&str>>("foo_1.x"),
            Ok((".x", mk_var("foo_1")))
        );
        assert!(var::<nom::error::Error<&str>>("1").is_err());
    }

    #[test]
    fn expr_works() {
        let v = |x| Box::new(ast::Expr::Var(mk_var(x)));
        let l = |i| Box::new(ast::Expr::Literal(ast::Literal::Int(i)));
        let plus = ast::BinOp::Plus;
        let times = ast::BinOp::Times;
        let bin = |a, b, c| mk_binop(a, b, c);
        match expr::<nom::error::Error<&str>>("x+y *3") {
            Ok(("", e)) => {
                assert_eq!(e, bin(v("x"), plus, bin(v("y"), times, l(3))));
            }
            err => {
                panic!("{:?}", err)
            }
        }
        match expr::<nom::error::Error<&str>>("(x + y ) *3") {
            Ok(("", e)) => {
                assert_eq!(e, bin(bin(v("x"), plus, v("y")), times, l(3)));
            }
            err => {
                panic!("Could not parse: {:?}", err);
            }
        }
    }

    fn mk_rec(fields: Vec<(&str, Box<ast::Expr>)>) -> Box<ast::Expr> {
        let mut fs = Vec::new();
        for (f, e) in fields.into_iter() {
            fs.push(ast::Field {
                name: ast::Var {
                    name: String::from(f),
                },
                value: e,
            });
        }
        Box::new(ast::Expr::Rec(ast::Rec {
            let_vars: vec![],
            fields: fs,
        }))
    }

    #[test]
    fn rec_empty() {
        let l = |i| Box::new(ast::Expr::Literal(ast::Literal::Int(i)));
        let input = r#"{
            x: 7
            y: 10
        }"#;
        match rec::<VerboseError<&str>>(input).finish() {
            Ok((rem, e)) => {
                assert!(
                    rem.is_empty(),
                    "Expected to consume all output. Remainder: {}",
                    rem
                );
                assert_eq!(e, mk_rec(vec![("x", l(7)),("y", l(10))]));
            }
            Err(e) => {
                panic!("Could not parse: {}", convert_error(input, e));
            }
        }
    }
}
