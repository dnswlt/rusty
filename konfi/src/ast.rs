#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    UnPlus,  // +
    UnMinus, // -
    Not,     // !
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Plus,       // +
    Minus,      // -
    Times,      // *
    Div,        // /
    LogicalAnd, // &&
    LogicalOr,  // ||
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    Nil,
    Int(i64),
    Double(f64),
    Str(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Var {
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Var(Var),
    UnExpr(UnOp, Box<Expr>),
    BinExpr(Box<Expr>, BinOp, Box<Expr>),
    Rec(Rec),
    Call(Call),
    Fun(Fun),
}

#[derive(Debug, PartialEq)]
pub struct Fun {
    params: Vec<Var>,
    body: Box<Expr>,
}

#[derive(Debug, PartialEq)]
pub struct Call {
    fun: Box<Expr>,
    args: Vec<Box<Expr>>,
}

#[derive(Debug, PartialEq)]
pub struct Rec {
    pub let_vars: Vec<LetBinding>,
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub name: Var,
    pub value: Box<Expr>,
}

#[derive(Debug, PartialEq)]
pub struct LetBinding {
    var: Var,
    value: Expr,
}
