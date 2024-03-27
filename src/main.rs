mod expr;
mod parse;
mod runner;

use crate::parse::parse;
use crate::runner::Program;
use clap::Parser;
use std::fs::File;
use std::io;
use std::io::Read;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    #[arg(index = 1)]
    program_file: String,
    #[arg(short)]
    e: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let program = if args.e {
        Vec::from(args.program_file.as_str())
    } else {
        let mut vec = Vec::new();
        let mut file = File::open(&args.program_file)?;
        file.read_to_end(&mut vec)?;
        vec
    };
    match parse(&program) {
        Some(expr) => {
            Program::run(&expr);
            Ok(())
        }
        None => {
            eprintln!("parse error");
            exit(1)
        }
    }
}
