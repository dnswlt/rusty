use clap::Parser;
use konfi::{parser, eval, json};
use std::{fs, io};

#[derive(Parser, Debug)]
#[command(name = "konfi")]
#[command(author = "Dennis Walter <dennis.walter@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "Konfi config language processor", long_about = None)]
struct Args {
    input_file: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let input = fs::read_to_string(&args.input_file)?;
    match parser::parse_module(&input) {
        Ok(module) => {
            let val = eval::eval(&module.expr, eval::Ctx::global()).expect("Cannot eval module");
            let j = json::to_json(&val).expect("Cannot serialize to JSON");
            println!("{}", serde_json::to_string_pretty(&j).expect("Cannot pretty-print JSON"));
        }
        Err(e) => {
            println!("Cannot parse {}:\n{}", args.input_file, e.message);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, ""));
        }
    }
    Ok(())
}
