use crate::ast;
use crate::strings::parse_string;
use std::num::ParseIntError;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, multispace0, one_of},
    combinator::{cut, map, map_res, opt, recognize},
    error::{FromExternalError, ParseError},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};

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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum BinopPrecedence {
    Multiplicative, // * /
    Additive,       // + -
    Shift,          // >> <<
    Relational,     // < > <= >=
    Equality,       // == !=
    LogicalAnd,     // &&
    LogicalOr,      // ||
}

impl BinopPrecedence {
    pub fn is_terminal(&self) -> bool {
        *self == Self::Multiplicative
    }
    pub fn next(&self) -> Self {
        match *self {
            Self::LogicalOr => Self::LogicalAnd,
            Self::LogicalAnd => Self::Equality,
            Self::Equality => Self::Relational,
            Self::Relational => Self::Shift,
            Self::Shift => Self::Additive,
            Self::Additive => Self::Multiplicative,
            Self::Multiplicative => {
                panic!("Called previous on {:?} which has no predecessor", self)
            }
        }
    }
}

fn binop<'a, E>(lvl: BinopPrecedence, input: &'a str) -> IResult<&str, ast::BinOp, E>
where
    E: ParseError<&'a str>,
{
    match lvl {
        BinopPrecedence::Multiplicative => alt((
            map(tag("*"), |_| ast::BinOp::Times),
            map(tag("/"), |_| ast::BinOp::Div),
        ))(input),
        BinopPrecedence::Additive => alt((
            map(tag("+"), |_| ast::BinOp::Plus),
            map(tag("-"), |_| ast::BinOp::Minus),
        ))(input),
        BinopPrecedence::Shift => alt((
            map(tag(">>"), |_| ast::BinOp::ShiftLeft),
            map(tag("<<"), |_| ast::BinOp::ShiftRight),
        ))(input),
        BinopPrecedence::Relational => alt((
            map(tag("<="), |_| ast::BinOp::LessEq),
            map(tag(">="), |_| ast::BinOp::GreaterEq),
            map(tag("<"), |_| ast::BinOp::LessThan),
            map(tag(">"), |_| ast::BinOp::GreaterThan),
        ))(input),
        BinopPrecedence::Equality => alt((
            map(tag("=="), |_| ast::BinOp::Eq),
            map(tag("!="), |_| ast::BinOp::NotEq),
        ))(input),
        BinopPrecedence::LogicalAnd => map(tag("&&"), |_| ast::BinOp::LogicalAnd)(input),
        BinopPrecedence::LogicalOr => map(tag("||"), |_| ast::BinOp::LogicalOr)(input),
    }
}

fn atom<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    let (r1, e) = alt((
        rec,
        delimited(char('('), cut(ws(expr)), char(')')),
        map(parse_string, |s| {
            Box::new(ast::Expr::Literal(ast::Literal::Str(s)))
        }),
        map(int_literal, |l| Box::new(ast::Expr::Literal(l))),
        map(var, |v| Box::new(ast::Expr::Var(v))),
    ))(input)?;
    // Try to parse a field access suffix.
    match pair(ws(char::<&'a str, E>('.')), var)(r1) {
        Ok((r2, (_, v))) => Ok((r2, Box::new(ast::Expr::FieldAcc(e, v.name)))),
        _ => Ok((r1, e)),
    }
}

pub fn expr<'a, E>(input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    gen_expr::<E>(BinopPrecedence::LogicalOr, input)
}

// Binary operators have different precedence ('*' binds more tightly than '+').
// BinopPrecedence encodes the precedence of all binary operators and is used
// here to obtain a generic recursive parser for all binary operators without the
// usual expr=>term=>factor=>atom hierarchy.
fn gen_expr<'a, E>(lvl: BinopPrecedence, input: &'a str) -> IResult<&str, Box<ast::Expr>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError> + 'a,
{
    let expr_binop = move |input| binop::<E>(lvl, input);
    // Parse first subterm.
    let (r1, a) = if lvl.is_terminal() {
        atom(input)
    } else {
        gen_expr::<E>(lvl.next(), input)
    }?;
    // Try to parse a binary operator and, if successful, the second term.
    // If no suitable operator follows, just return the first term.
    match ws(expr_binop)(r1) {
        Ok((r2, op)) => {
            let (r2, b) = gen_expr::<E>(lvl, r2)?;
            Ok((r2, Box::new(ast::Expr::BinExpr(a, op, b))))
        }
        _ => Ok((r1, a)),
    }
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
            // many0(delimited(multispace0, rec_field, eol)), //
            separated_list0(eol, preceded(multispace0, rec_field)),
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

    use super::*;
    use nom::combinator::all_consuming;
    use nom::Finish;

    macro_rules! assert_parse {
        ($f:ident, $e:expr) => {
            let input = $e;
            if let Err(e) = all_consuming($f::<nom::error::VerboseError<&str>>)(input).finish() {
                assert!(
                    false,
                    "Could not parse: {}",
                    nom::error::convert_error(input, e)
                );
            }
        };
    }
    macro_rules! assert_finish {
        ($e:literal, $f:ident, $v:expr) => {
            let input = $e;
            match $f::<nom::error::VerboseError<&str>>(input).finish() {
                Ok((i, r)) => {
                    assert_eq!(i, "", "Input not fully processed.");
                    assert_eq!(r, $v);
                }
                Err(e) => {
                    assert!(
                        false,
                        "Could not parse: {}",
                        nom::error::convert_error(input, e)
                    );
                }
            }
        };
    }

    // Helper functions to build expressions.
    mod h {
        use crate::ast;

        pub fn ilit(i: i64) -> ast::Literal {
            ast::Literal::Int(i)
        }
        pub fn ilit_expr(i: i64) -> Box<ast::Expr> {
            Box::new(ast::Expr::Literal(ilit(i)))
        }

        pub fn slit_expr(t: &str) -> Box<ast::Expr> {
            Box::new(ast::Expr::Literal(ast::Literal::Str(String::from(t))))
        }

        pub fn var(s: &str) -> ast::Var {
            ast::Var {
                name: String::from(s),
            }
        }

        pub fn var_expr(s: &str) -> Box<ast::Expr> {
            Box::new(ast::Expr::Var(var(s)))
        }

        pub fn binexpr(a: Box<ast::Expr>, op: ast::BinOp, b: Box<ast::Expr>) -> Box<ast::Expr> {
            Box::new(ast::Expr::BinExpr(a, op, b))
        }

        pub fn rec_expr(fields: Vec<(&str, Box<ast::Expr>)>) -> Box<ast::Expr> {
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

        pub fn acc_expr(e: Box<ast::Expr>, f: &str) -> Box<ast::Expr> {
            Box::new(ast::Expr::FieldAcc(e, String::from(f)))
        }
    }

    #[test]
    fn i64_works() {
        assert_finish!("123", int_literal, h::ilit(123));
        assert_finish!("+1", int_literal, h::ilit(1));
        assert_finish!("-2", int_literal, h::ilit(-2));
    }

    #[test]
    fn var_works() {
        assert_eq!(var::<nom::error::Error<&str>>("y"), Ok(("", h::var("y"))));
        assert_eq!(var::<nom::error::Error<&str>>("_"), Ok(("", h::var("_"))));
        assert_eq!(var::<nom::error::Error<&str>>("_1"), Ok(("", h::var("_1"))));
        assert_eq!(
            var::<nom::error::Error<&str>>("foo_1.x"),
            Ok((".x", h::var("foo_1")))
        );
        assert!(var::<nom::error::Error<&str>>("1").is_err());
    }

    #[test]
    fn expr_works() {
        use ast::BinOp::{Plus, Times};
        let v = h::var_expr;
        let l = h::ilit_expr;
        let bin = h::binexpr;
        assert_finish!("x+y *3", expr, bin(v("x"), Plus, bin(v("y"), Times, l(3))));
        assert_finish!(
            "x * y + 3",
            expr,
            bin(bin(v("x"), Times, v("y")), Plus, l(3))
        );
        assert_finish!(
            "(x + y ) *3",
            expr,
            bin(bin(v("x"), Plus, v("y")), Times, l(3))
        );
        // Right-associative expr parsing:
        let right_assoc_add = bin(v("x"), Plus, bin(v("y"), Plus, v("z")));
        assert_finish!("x+y+z", expr, right_assoc_add);
        assert_finish!("x+(y+z)", expr, right_assoc_add);
    }

    #[test]
    fn expr_long_chain() {
        // Ensure our parser does not suffer from a combinatorial explosion
        // when parsing long chains of expressions.
        let args = vec!["a"; 1000];
        assert_parse!(expr, &args.join("*")[..]);
        assert_parse!(expr, &args.join("||")[..]);
        assert_parse!(expr, &args.join(">>")[..]);
    }

    #[test]
    fn expr_rec_field_access() {
        let l = h::ilit_expr;
        let r = h::rec_expr;
        let get = h::acc_expr;
        assert_finish!("{x: 7}.x", expr, get(r(vec![("x", l(7))]), "x"));
    }

    #[test]
    fn rec_works() {
        let l = h::ilit_expr;
        let s = h::slit_expr;
        let r = h::rec_expr;
        assert_finish!("{}", rec, r(vec![]));
        assert_finish!("{}", rec, r(vec![]));
        assert_finish!(
            r#"{
            x: 7
            y: 10
        }"#,
            rec,
            r(vec![("x", l(7)), ("y", l(10))])
        );
        assert_finish!(
            r#"{
            x: {
                y: {
                    z: "foo"
                }
            }
        }"#,
            rec,
            r(vec![("x", r(vec![("y", r(vec![("z", s("foo"))]))]))])
        );
    }
}
