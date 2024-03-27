use crate::expr::Expr::*;
use crate::expr::*;
use pom::parser::*;

fn is_space(byte: &u8) -> bool {
    let ch = char::from(*byte);
    ch.is_whitespace()
}

pub fn parse_unlambda(str: &[u8]) -> Result<Expr, &'static str> {
    let (expr, str) = parse_unlambda_inner(str)?;
    if str.iter().all(is_space) {
        Ok(expr)
    } else {
        Err("unexpected end")
    }
}

fn parse_unlambda_inner(str: &[u8]) -> Result<(Expr, &[u8]), &'static str> {
    match str.first() {
        None => Err("end of string"),
        Some(b's') => Ok((S, &str[1..])),
        Some(b'k') => Ok((K, &str[1..])),
        Some(b'i') => Ok((I, &str[1..])),
        Some(b'`') => parse_unlambda_inner(&str[1..])
            .and_then(|(lhs, str)| parse_unlambda_inner(&str).map(|(rhs, str)| (lhs * rhs, str))),
        Some(c) if is_space(c) => parse_unlambda_inner(&str[1..]),
        _ => Err("unknown character"),
    }
}

pub fn parse_iota(str: &[u8]) -> Result<Expr, &'static str> {
    let (expr, str) = parse_iota_inner(str)?;
    if str.iter().all(is_space) {
        Ok(expr)
    } else {
        Err("unexpected end")
    }
}

fn iota() -> Expr {
    use Expr::*;
    S * (S * I * (K * S)) * (K * K)
}

fn parse_iota_inner(str: &[u8]) -> Result<(Expr, &[u8]), &'static str> {
    match str.first() {
        None => Err("end of string"),
        Some(b'i') => Ok((iota(), &str[1..])),
        Some(b'*') => parse_iota_inner(&str[1..])
            .and_then(|(lhs, str)| parse_iota_inner(&str).map(|(rhs, str)| (lhs * rhs, str))),
        Some(c) if is_space(c) => parse_iota_inner(&str[1..]),
        _ => Err("unknown character"),
    }
}

fn parse_jot(bytes: &[u8]) -> Result<Expr, &'static str> {
    match bytes.last() {
        None => Ok(I),
        Some(b'0') => {
            let w = parse_jot(&bytes[..bytes.len() - 1])?;
            Ok(w * S * K)
        }
        Some(b'1') => {
            let w = parse_jot(&bytes[..bytes.len() - 1])?;
            Ok(S * (K * w))
        }
        Some(byte) if is_space(byte) => parse_jot(&bytes[..bytes.len() - 1]),
        _ => Err("unknown character"),
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn combinator<'a>() -> Parser<'a, u8, Expr> {
    (one_of(b"sS").map(|_| S) | one_of(b"kK").map(|_| K) | one_of(b"iI").map(|_| I)) - space()
}

fn term<'a>() -> Parser<'a, u8, Expr> {
    combinator() | { sym(b'(') * space() * call(expr) - space() - sym(b')') - space() }
}

fn expr<'a>() -> Parser<'a, u8, Expr> {
    (term() + term().repeat(0..)).map(|(t, ts)| ts.into_iter().fold(t, |l, r| l * r))
}

pub fn parse_combinator(str: &[u8]) -> Result<Expr, &'static str> {
    (space() * expr() - end())
        .parse(str)
        .map_err(|_| "parse error")
}

pub fn parse(str: &[u8]) -> Option<Expr> {
    let functions: [&dyn Fn(&[u8]) -> Result<Expr, &'static str>; 4] =
        [&parse_combinator, &parse_unlambda, &parse_iota, &parse_jot];
    functions.iter().filter_map(|f| f(str).ok()).next()
}
