use crate::expr::Expr::*;
use crate::expr::*;
use pom::parser::*;

fn is_space(byte: &u8) -> bool {
    let ch = char::from(*byte);
    ch.is_whitespace()
}

fn iota() -> Expr {
    use Expr::*;
    S * (S * I * (K * S)) * (K * K)
}

fn parse_jot(bytes: &[u8]) -> Result<Expr, &'static str> {
    let mut expr = I;
    for byte in bytes {
        match byte {
            b'0' => expr = expr * S * K,
            b'1' => expr = S * (K * expr),
            _ => return Err("unknown character"),
        }
    }
    Ok(expr)
}

fn expr_dash<'a>() -> Parser<'a, u8, Expr> {
    one_of(b"sS").map(|_| S)
        | one_of(b"kK").map(|_| K)
        | one_of(b"i").map(|_| I)
        | (sym(b'`') * call(expr) + call(expr)).map(|(f, g)| f * g)
        | (sym(b'*') * call(iota_expr) + call(iota_expr)).map(|(f, g)| f * g)
        | { sym(b'(') * call(cc_expr) - sym(b')') }
}

fn iota_expr<'a>() -> Parser<'a, u8, Expr> {
    sym(b'i').map(|_| iota()) | expr_dash()
}

fn expr<'a>() -> Parser<'a, u8, Expr> {
    sym(b'i').map(|_| I) | expr_dash()
}

fn cc_expr<'a>() -> Parser<'a, u8, Expr> {
    (expr() + expr().repeat(0..)).map(|(t, ts)| ts.into_iter().fold(t, |l, r| l * r))
}

pub fn parse_expr(str: &[u8]) -> Result<Expr, &'static str> {
    (cc_expr() - end()).parse(str).map_err(|_| "parse error")
}

pub fn parse(str: &str) -> Option<Expr> {
    let code = str
        .lines()
        .map(|line| line.as_bytes().iter().take_while(|c| **c != b'#'))
        .flatten()
        .filter(|c| !is_space(c))
        .map(|b| *b)
        .collect::<Vec<u8>>();

    if code.is_empty() || code[0] == b'0' || code[0] == b'1' {
        parse_jot(&code).ok()
    } else {
        parse_expr(&code).ok()
    }
}
