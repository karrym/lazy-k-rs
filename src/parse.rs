use std::rc::Rc;
use pom::parser::*;
use crate::expr::Expr::*;
use crate::expr::*;

pub fn parse_unlambda(str: &str) -> Result<(Expr, &str), String> {
    match str.chars().next() {
        None => Err("end of string".to_owned()),
        Some('s') => Ok((S, &str[1..])),
        Some('k') => Ok((K, &str[1..])),
        Some('i') => Ok((I, &str[1..])),
        Some('`') => parse_unlambda(&str[1..])
            .and_then(|(lhs, str)| parse_unlambda(&str).map(|(rhs, str)| (A(Rc::new(lhs), Rc::new(rhs)), str))),
        Some(c) if c.is_whitespace() => parse_unlambda(&str[1..]),
        _ => Err("unknown character".to_owned())
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn combinator<'a>() -> Parser<'a, u8, Expr> {
    (one_of(b"sS").map(|_| S) | one_of(b"kK").map(|_| K) | one_of(b"iI").map(|_| I)) - space()
}

fn term<'a>() -> Parser<'a, u8, Expr> {
    combinator() | {
        sym(b'(') * space() * call(expr) - space() - sym(b')') - space()
    }
}

fn expr<'a>() -> Parser<'a, u8, Expr> {
    (term() + term().repeat(0..)).map(|(t, ts)| ts.into_iter().fold(t, |l, r| Expr::A(Rc::new(l), Rc::new(r))))
}

pub fn parse_lazy_k(str: &[u8]) -> pom::Result<Expr> {
    (space() * expr() - end()).parse(str)
}

