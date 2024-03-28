mod expr;
mod parse;
mod runner;

use crate::parse::parse;
use crate::runner::Runner;
use std::fs::File;
use std::env;
use std::io::Read;
use std::process::exit;
use crate::expr::Expr;

enum Tag {
    FromArg(usize),
    FromFile(String)
}

fn location(tag: &Tag) -> String {
    match tag {
        Tag::FromArg(1) => "1st argument".to_owned(),
        Tag::FromArg(2) => "2nd argument".to_owned(),
        Tag::FromArg(3) => "3rd argument".to_owned(),
        Tag::FromArg(n) => format!("{}th argument", n),
        Tag::FromFile(file) => format!("file {}", file)
    }
}

struct Program {
    tag: Tag,
    code: Vec<u8>
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Vec<Program> {
    let mut index = 1;
    let mut vec = Vec::new();
    while let Some(arg) = args.next() {
        if arg.is_empty() { continue };
        if arg == "-e" {
            let code = args.next().unwrap_or_else(|| {
                eprintln!("expect program after -e");
                exit(1)
            });
            vec.push(Program {
                tag: Tag::FromArg(index),
                code: code.into_bytes()
            })
        } else {
            let mut code = Vec::new();
            let mut file = File::open(&arg).unwrap_or_else(|_| {
                eprintln!("cannot open file: {}", arg);
                exit(1)
            });
            file.read_to_end(&mut code).unwrap_or_else(|_| {
                eprintln!("cannot read file: {}", arg);
                exit(1)
            });
            vec.push(Program {
                tag: Tag::FromFile(arg),
                code
            })
        }
        index += 1
    }
    vec
}

fn main() {
    let programs = parse_args(env::args().into_iter().skip(1));
    if programs.is_empty() {
        eprintln!("please designate one or more program");
        exit(1);
    }
    let expr = programs.into_iter().map(|program| {
        match parse(&program.code) {
            None => Err(format!("parse error on {}", location(&program.tag))),
            Some(e) => Ok(e)
        }
    }).collect::<Result<Vec<_>, _>>().unwrap_or_else(|err| {
        eprintln!("{}", err);
        exit(1)
    }).into_iter().fold(Expr::I, |f, g| Expr::S * (Expr::K * f) * g);
    Runner::run(&expr);
}
