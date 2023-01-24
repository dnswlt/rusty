// Evaluate ast::Expr and friends and turn them into actual values.

use crate::ast;
use chrono::Duration;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

type UtcTimestamp = chrono::offset::Utc;

#[derive(PartialEq, Debug, Clone)]
pub enum Val {
    Nil,
    Rec(Rc<RefCell<Rec>>),
    Bool(bool),
    Int(i64),
    Double(f64),
    Str(String),
    Timestamp(UtcTimestamp),
    Duration(Duration),
}

impl Val {
    pub fn typ(&self) -> &str {
        match self {
            Val::Nil => "nil",
            Val::Rec(_) => "rec",
            Val::Int(_) => "int",
            Val::Double(_) => "double",
            Val::Str(_) => "str",
            Val::Timestamp(_) => "timestamp",
            Val::Duration(_) => "duration",
            Val::Bool(_) => "bool",
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Val::Nil => false,
            Val::Rec(r) => !r.borrow().is_empty(),
            Val::Bool(b) => *b,
            Val::Int(i) => *i != 0,
            Val::Double(d) => *d != 0.0,
            Val::Str(s) => !s.is_empty(),
            Val::Timestamp(_) => todo!(),
            Val::Duration(_) => todo!(),
        }
    }
}

impl Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Nil => write!(f, "nil"),
            Val::Rec(_) => todo!(),
            Val::Bool(b) => write!(f, "{b}"),
            Val::Int(i) => write!(f, "{i}"),
            Val::Double(d) => write!(f, "{d}"),
            Val::Str(s) => write!(f, "\"{s}\""),
            Val::Timestamp(_) => todo!(),
            Val::Duration(_) => todo!(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Rec {
    pub fields: HashMap<String, Val>,
}

impl Rec {
    pub fn new() -> Self {
        Rec {
            fields: HashMap::new(),
        }
    }
    pub fn getattr(&self, f: &str) -> Option<Val> {
        self.fields.get(f).map(|v| v.clone())
    }
    pub fn setattr(&mut self, f: &str, val: Val) {
        self.fields.insert(f.to_string(), val);
    }
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct EvalError {
    pub message: String,
}

type EvalResult<T> = Result<T, EvalError>;

// Evaluation context.
pub struct Ctx<'a> {
    rec: Rc<RefCell<Rec>>,
    rec_expr: &'a ast::Rec,
    parent: Option<Rc<Ctx<'a>>>,
}

static GLOBAL_DUMMY_REC: ast::Rec = ast::Rec {
    let_vars: vec![],
    fields: vec![],
};

impl<'a> Ctx<'a> {
    pub fn global() -> Rc<Ctx<'a>> {
        Rc::new(Ctx {
            rec: Rc::new(RefCell::new(Rec::new())),
            rec_expr: &GLOBAL_DUMMY_REC,
            parent: None,
        })
    }
    pub fn child_of(parent: Rc<Ctx<'a>>, r: Rc<RefCell<Rec>>, re: &'a ast::Rec) -> Rc<Ctx<'a>> {
        Rc::new(Ctx {
            rec: r,
            rec_expr: re,
            parent: Some(parent),
        })
    }

    pub fn getval(&self, var: &str) -> Option<Val> {
        let mut c = self;
        loop {
            if let Some(v) = self.rec.borrow().getattr(var) {
                return Some(v);
            }
            if let Some(p) = &c.parent {
                c = p;
            } else {
                return None;
            }
        }
    }

    fn getfield(&self, field: &str) -> Option<&'a ast::Field> {
        return self.rec_expr.fields.iter().find(|&fld| fld.name == field);
    }

    pub fn for_var(ctx: Rc<Ctx<'a>>, field: &str) -> Option<(Rc<Ctx<'a>>, &'a ast::Field)> {
        if let Some(f) = ctx.getfield(field) {
            return Some((ctx, f));
        }
        match &ctx.parent {
            Some(p) => Self::for_var(Rc::clone(p), field),
            None => None,
        }
    }
}

macro_rules! numeric_binexpr {
    ($lv:expr, $op:tt, $rv:expr) => {
        match (&$lv, &$rv) {
            (Val::Int(a), Val::Int(b)) => Ok(Val::Int(a $op b)),
            (Val::Int(a), Val::Double(b)) => Ok(Val::Double((*a as f64) $op b)),
            (Val::Double(a), Val::Int(b)) => Ok(Val::Double(a $op (*b as f64))),
            (Val::Double(a), Val::Double(b)) => Ok(Val::Double(a $op b)),
            (_, _) => Err(EvalError {
                message: format!("Invalid types for arithmetic operation '{}': {} and {}",
                    stringify!($op), $lv.typ(), $rv.typ()),
            }),
        }
    };
}

macro_rules! comp_expr {
    ($lv:expr, $op:tt, $rv:expr) => {
        match (&$lv, &$rv) {
            (Val::Int(a), Val::Int(b)) => Ok(Val::Bool(*a $op *b)),
            (Val::Int(a), Val::Double(b)) => Ok(Val::Bool((*a as f64) $op *b)),
            (Val::Double(a), Val::Int(b)) => Ok(Val::Bool(*a $op (*b as f64))),
            (Val::Double(a), Val::Double(b)) => Ok(Val::Bool(*a $op *b)),
            (Val::Str(a), Val::Str(b)) => Ok(Val::Bool(a $op b)),
            (Val::Bool(a), Val::Bool(b)) => Ok(Val::Bool(*a $op *b)),
            (_, _) => Err(EvalError {
                message: format!("Invalid types for arithmetic operation '{}': {} and {}",
                    stringify!($op), $lv.typ(), $rv.typ()),
            }),
        }
    };
}

pub fn eval(e: &ast::Expr, ctx: Rc<Ctx>) -> EvalResult<Val> {
    match e {
        ast::Expr::Literal(i) => match i {
            ast::Literal::Nil => Ok(Val::Nil),
            ast::Literal::Int(i) => Ok(Val::Int(*i)),
            ast::Literal::Double(d) => Ok(Val::Double(*d)),
            ast::Literal::Str(s) => Ok(Val::Str(s.clone())),
        },
        ast::Expr::Var(v) => match ctx.getval(&v.name) {
            Some(r) => Ok(r),
            None => match Ctx::for_var(ctx, &v.name) {
                Some((ctx2, fld)) => {
                    // Evaluate `fld`, store its value, and return it.
                    eval_field(fld, Rc::clone(&ctx2))
                }
                None => Err(EvalError {
                    message: format!("Unbound variable '{}'", v.name),
                }),
            },
        },
        ast::Expr::FieldAcc(re, f) => match eval(re, ctx)? {
            Val::Rec(r) => r.borrow().getattr(f).ok_or_else(|| EvalError {
                message: format!("Field does not exist '{}'", f),
            }),
            v => Err(EvalError {
                message: format!("Invalid field access on value type '{}'", v.typ()),
            }),
        },
        ast::Expr::UnExpr(op, e) => {
            let val = eval(e, Rc::clone(&ctx))?;
            match op {
                ast::UnOp::UnPlus => Ok(val),
                ast::UnOp::UnMinus => match &val {
                    Val::Int(i) => Ok(Val::Int(-i)),
                    Val::Double(d) => Ok(Val::Double(-d)),
                    _ => Err(EvalError {
                        message: format!("Cannot apply unary minus to type '{}'", val.typ()),
                    }),
                },
                ast::UnOp::Not => Ok(Val::Bool(!val.to_bool())),
            }
        }
        ast::Expr::BinExpr(le, op, re) => {
            let lv = eval(le, Rc::clone(&ctx))?;
            // Let's make && || lazy later. For now all ops are eager.
            let rv = eval(re, ctx)?;
            match op {
                ast::BinOp::Times => numeric_binexpr!(lv, *, rv),
                ast::BinOp::Div => numeric_binexpr!(lv, /, rv),
                ast::BinOp::Plus => numeric_binexpr!(lv, +, rv),
                ast::BinOp::Minus => numeric_binexpr!(lv, -, rv),
                ast::BinOp::ShiftLeft => todo!(),
                ast::BinOp::ShiftRight => todo!(),
                ast::BinOp::LessThan => comp_expr!(lv, <, rv),
                ast::BinOp::GreaterThan => comp_expr!(lv, >, rv),
                ast::BinOp::LessEq => comp_expr!(lv, <=, rv),
                ast::BinOp::GreaterEq => comp_expr!(lv, >=, rv),
                ast::BinOp::Eq => comp_expr!(lv, ==, rv),
                ast::BinOp::NotEq => comp_expr!(lv, !=, rv),
                ast::BinOp::LogicalAnd => Ok(Val::Bool(lv.to_bool() && rv.to_bool())),
                ast::BinOp::LogicalOr => Ok(Val::Bool(lv.to_bool() || rv.to_bool())),
            }
        }
        ast::Expr::Rec(re) => {
            let r = eval_rec(re, ctx)?;
            Ok(Val::Rec(r))
        }
        ast::Expr::Call(_) => todo!(),
        ast::Expr::Fun(_) => todo!(),
    }
}

fn eval_rec(re: &ast::Rec, ctx: Rc<Ctx>) -> EvalResult<Rc<RefCell<Rec>>> {
    {
        let record = Rc::new(RefCell::new(Rec::new()));
        let rec_ctx = Ctx::child_of(ctx, Rc::clone(&record), re);
        {
            for fld in re.fields.iter() {
                if record.borrow().fields.contains_key(&fld.name) {
                    // We already set this field while evaluating other fields
                    // of this (or a child/parent/sibling) record.
                    continue;
                }
                let v = eval(&fld.value, Rc::clone(&rec_ctx))?;
                (*record).borrow_mut().setattr(&fld.name, v);
            }
        }
        Ok(record)
    }
}

// Evaluate a single field, storing the result in the context's active record.
fn eval_field(field: &ast::Field, ctx: Rc<Ctx>) -> EvalResult<Val> {
    let val = eval(&field.value, Rc::clone(&ctx))?;
    let mut m = (*ctx.rec).borrow_mut();
    m.setattr(&field.name, val.clone());
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    mod h {
        use crate::eval::*;
        use crate::parser;
        use crate::ast;
        pub fn force_parse(s: &str) -> Box<ast::Expr> {
            parser::expr_opt(s).expect(&format!("Expected being able to parse: {}", s))
        }
        pub fn eval_global(s: &str) -> EvalResult<Val> {
            eval(&force_parse(s), Ctx::global())
        }
    }

    #[test]
    fn eval_truthy() {
        let e = h::eval_global;
        let r = |b| Ok(Val::Bool(b));
        assert_eq!(e("!!7"), r(true));
        assert_eq!(e("!\"foo\""), r(false));
        assert_eq!(e("!!{}"), r(false));
        assert_eq!(e("!!{}"), r(false));
    }

    #[test]
    fn eval_comp() {
        let e = h::eval_global;
        let r = |b| Ok(Val::Bool(b));
        assert_eq!(e("1 == 2"), r(false));
        assert_eq!(e("1 != 2"), r(true));
        assert_eq!(e("1 < 2 && 2 < 3"), r(true));
        // Tests that && binds more tightly than ||
        assert_eq!(e("1 || 1 && 0"), r(true));
        assert_eq!(e("1 || 0 && 0"), r(true));
    }

    #[test]
    fn eval_rec() {
        assert_eq!(h::eval_global("{x: 3 - 8}.x"), Ok(Val::Int(-5)));
    }

    #[test]
    fn eval_rec_lookup() {
        let rec = parser::expr_opt(
            r#"{
            b: {
                d: c + a
                c: 1
            }
            a: 1
        }.b.d"#,
        )
        .unwrap();
        let ctx = Ctx::global();
        assert_eq!(eval(&rec, ctx), Ok(Val::Int(2)));
    }

    #[test]
    fn eval_rec_linear_dep() {
        let rec = parser::expr_opt(
            r#"{
            a: b.value
            b: c
            c: d
            d: e
            e: f
            f: {value: 1}
        }.a"#,
        )
        .unwrap();
        let ctx = Ctx::global();
        assert_eq!(eval(&rec, ctx), Ok(Val::Int(1)));
    }
}
