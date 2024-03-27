mod expr;
mod parse;
mod runner;

use std::fs::File;
use std::io;
use std::io::Read;
use std::process::exit;
use clap::Parser;
use crate::parse::parse_lazy_k;
use crate::runner::Program;

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
    match parse_lazy_k(&program) {
        Ok(expr) => {
            Program::run(&expr);
            Ok(())
        },
        Err(e) => {
            eprintln!("parse error: {}", e);
            exit(1)
        }
    }
}
