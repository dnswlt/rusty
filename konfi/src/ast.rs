#[derive(Debug)]
pub enum UnOp {
    UnPlus,  // +
    UnMinus, // -
    Not,     // !
}

#[derive(Debug)]
pub enum BinOp {
    Plus,       // +
    Minus,      // -
    LogicalAnd, // &&
    LogicalOr,  // ||
}

#[derive(Debug)]
pub enum Literal {
    Nil,
    Int(i64),
    Double(f64),
    Str(String),
}

#[derive(Debug)]
pub enum Expr {
    Literal(Literal),
    UnExpr(UnOp, Box<Expr>),
    BinExpr(Box<Expr>, BinOp, Box<Expr>),
    Rec(Record),
}

#[derive(Debug)]
pub struct Record {
    let_vars: Vec<LetBinding>,
    fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
    name: String,
    value: Expr,
}

#[derive(Debug)]
pub struct LetBinding {
    var: String,
    value: Expr,
}
