use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Expr {
    I,
    K,
    S,
    A(Rc<Expr>, Rc<Expr>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Expr::I => write!(f, "i"),
            Expr::K => write!(f, "k"),
            Expr::S => write!(f, "s"),
            Expr::A(x, y) => write!(f, "`{}{}", x.as_ref(), y.as_ref()),
        }
    }
}
