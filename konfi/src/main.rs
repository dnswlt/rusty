use konfi::ast::*;

fn main() {
    let b = Box::new;
    println!(
        "Hello, world!, {:?}",
        Expr::BinExpr(
            b(Expr::Literal(Literal::Int(9))),
            BinOp::Plus,
            b(Expr::Literal(Literal::Str(String::from("10")))),
        )
    );
}
