use crate::expr::Expr;
use std::collections::VecDeque;
use std::io;
use std::io::{stdin, stdout, Read, StdinLock, Write};
use std::mem::size_of;

type ExprId = usize;

#[derive(Clone, Debug)]
pub enum Graph {
    Apply(ExprId, ExprId),
    S,
    K,
    I,
    Link(ExprId),
    Inc,
    Num(u16),
    Stdin,
    Free,
}

pub struct Runner {
    e: Vec<Graph>,
    start: ExprId,
    fresh_id: ExprId,
    input: StdinLock<'static>,
}

type Stack = Vec<ExprId>;

const S: ExprId = 0;
const K: ExprId = 1;
const I: ExprId = 2;
const INC: ExprId = 3;
const ZERO: ExprId = 4;

const PROGRAM_AREA_END: usize = 5;

impl Runner {
    fn new() -> Self {
        /*
        let mut hash = HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default());
        hash.extend([(0,Graph::S),(1,Graph::K),(2, Graph::I),(3, Graph::Inc),(4, Graph::Num(0))]);
         */
        Self {
            e: vec![
                Graph::S,
                Graph::K,
                Graph::I,
                Graph::Inc,
                Graph::Num(0),
                Graph::Stdin,
            ],
            start: 6,
            fresh_id: 6,
            input: stdin().lock(),
        }
    }

    #[allow(dead_code)]
    pub fn compile(expr: &Expr) -> Self {
        let mut p = Self::new();
        p.start = p.push_expr(expr);
        p
    }

    fn push_expr(&mut self, expr: &Expr) -> ExprId {
        match expr {
            Expr::I => I,
            Expr::K => K,
            Expr::S => S,
            Expr::A(l, r) => {
                let l = self.push_expr(l.as_ref());
                let r = self.push_expr(r.as_ref());
                self.push(Graph::Apply(l, r))
            }
        }
    }

    fn push_church(&mut self, num: u16) -> ExprId {
        if num == 0 {
            self.push(Graph::Apply(K, I))
        } else {
            let ks = self.push(Graph::Apply(K, S));
            let s_ks = self.push(Graph::Apply(S, ks));
            let s_ks_k = self.push(Graph::Apply(s_ks, K));
            let succ = self.push(Graph::Apply(S, s_ks_k));
            let n = self.push_church(num - 1);
            self.push(Graph::Apply(succ, n))
        }
    }

    pub fn push_cons(&mut self, car: ExprId, cdr: ExprId) -> ExprId {
        let k_car = self.push(Graph::Apply(K, car));
        let k_cdr = self.push(Graph::Apply(K, cdr));
        let s_i = self.push(Graph::Apply(S, I));
        let s_i_car = self.push(Graph::Apply(s_i, k_car));
        let s_car = self.push(Graph::Apply(S, s_i_car));
        self.push(Graph::Apply(s_car, k_cdr))
    }

    fn spine(&self, mut id: ExprId, stack: &mut Stack) {
        loop {
            id = self.follow_link(id);
            stack.push(id);
            match &self.e[id] {
                Graph::Apply(l, _) => id = *l,
                _ => break,
            };
        }
    }

    fn get_rhs(&self, expr_id: ExprId) -> ExprId {
        match &self.e[expr_id] {
            Graph::Apply(_, rhs) => *rhs,
            _ => unreachable!(),
        }
    }

    /*
    fn gen_id(&mut self) -> ExprId {
        let id = self.fresh_id;
        self.fresh_id += 1;
        id
    }
     */

    fn push(&mut self, graph: Graph) -> ExprId {
        for i in self.fresh_id..self.e.len() {
            if let Graph::Free = self.e[i] {
                self.e[i] = graph;
                self.fresh_id = (i + 1) as ExprId;
                return i as ExprId;
            }
        }
        let id = self.e.len();
        self.e.push(graph);
        self.fresh_id = id + 1 as ExprId;
        id
    }

    fn follow_link(&self, mut expr: ExprId) -> ExprId {
        while let Graph::Link(arg1) = self.e[expr] {
            expr = arg1;
        }
        expr
    }

    fn reduce(&mut self, start: ExprId) {
        use Graph::*;
        let mut stack = Vec::new();
        self.spine(start, &mut stack);
        loop {
            //println!("stack depth: {}", stack.len());
            let Some(mut f) = stack.pop() else { break };
            f = self.follow_link(f);
            match self.e[f] {
                S => {
                    let Some(r1) = stack.pop() else { break };
                    let Some(r2) = stack.pop() else { break };
                    let Some(r3) = stack.pop() else { break };
                    let x = self.get_rhs(r1);
                    let y = self.get_rhs(r2);
                    let z = self.get_rhs(r3);
                    let lhs = self.push(Apply(x, z));
                    let rhs = self.push(Apply(y, z));
                    self.e[r3] = Apply(lhs, rhs);
                    self.spine(r3, &mut stack);
                }
                K => {
                    let Some(r1) = stack.pop() else { break };
                    let Some(r2) = stack.pop() else { break };
                    let x = self.get_rhs(r1);
                    self.e[r2] = Link(x);
                    self.spine(r2, &mut stack);
                }
                I => {
                    let Some(r) = stack.pop() else { break };
                    let x = self.get_rhs(r);
                    self.e[r] = Link(x);
                    self.spine(r, &mut stack);
                }
                Inc => {
                    let Some(r) = stack.pop() else { break };
                    let mut n = self.get_rhs(r);
                    self.reduce(n);
                    n = self.follow_link(n);
                    match &self.e[n] {
                        Num(n) => {
                            self.e[r] = Num(n + 1);
                            stack.push(r);
                        }
                        _ => panic!("cannot increment"),
                    };
                }
                Num(_) => break,
                Stdin => {
                    let _ = stdout().flush();
                    let mut buf = [0u8; 1];
                    let n = match self.input.read_exact(&mut buf) {
                        Ok(_) => buf[0] as u16,
                        _ => 256,
                    };
                    let church = self.push_church(n);
                    let stdin = self.push(Stdin);
                    let cons = self.push_cons(church, stdin);
                    self.e[f] = Link(cons);
                    self.spine(f, &mut stack);
                }
                ref t => panic!("unreachable: {:?}", &t),
            };
        }
    }

    fn garbage_collect(&mut self) {
        let mut need = vec![false; self.e.len()];
        let mut queue = VecDeque::new();
        queue.push_front(self.start);
        while let Some(id) = queue.pop_back() {
            if need[id] {
                continue;
            }
            need[id] = true;
            match &self.e[id] {
                Graph::Apply(l, r) => {
                    queue.push_front(*l);
                    queue.push_front(*r);
                }
                Graph::Link(x) => queue.push_front(*x),
                _ => continue,
            }
        }

        for i in PROGRAM_AREA_END..self.e.len() {
            if !need[i] {
                self.e[i] = Graph::Free;
            }
        }
        self.fresh_id = PROGRAM_AREA_END;
    }

    fn print_expr(&self, id: ExprId, string: &mut String) -> io::Result<()> {
        match self.e[id] {
            Graph::Apply(x, y) => {
                string.push('`');
                self.print_expr(x, string)?;
                self.print_expr(y, string)?;
            }
            Graph::S => string.push('s'),
            Graph::K => string.push('k'),
            Graph::I => string.push('i'),
            Graph::Link(x) => self.print_expr(x, string)?,
            Graph::Inc => string.push_str("<increment>"),
            Graph::Num(n) => {
                string.push_str("<number:");
                string.push_str(&n.to_string());
                string.push('>');
            }
            Graph::Stdin => string.push_str("<stdin>"),
            Graph::Free => string.push_str("<runtime bug>"),
        }
        Ok(())
    }

    pub fn print_list(&mut self) {
        let mut writer = stdout().lock();
        loop {
            let car = self.push(Graph::Apply(self.start, K));

            let i = self.push(Graph::Apply(car, INC));
            let g = self.push(Graph::Apply(i, ZERO));
            self.reduce(g);
            match &self.e[g] {
                Graph::Num(ch) => {
                    if *ch >= 256 {
                        break;
                    };
                    let _ = writer.write(&[(ch & 0xFF) as u8]);
                    let ki = self.push(Graph::Apply(K, I));
                    self.start = self.push(Graph::Apply(self.start, ki));
                }
                _ => {
                    let mut string = String::new();
                    let _ = self.print_expr(g, &mut string);
                    panic!("cannot reduce to numeric value: {}", string)
                }
            };
            if self.e.len() * size_of::<Graph>() > 256 * 1024 * 1024 {
                self.garbage_collect();
            }
        }
        let _ = writer.flush();
    }

    pub fn run(expr: &Expr) {
        let mut this = Self::new();
        let i = this.push_expr(expr);
        let stdin = this.push(Graph::Stdin);
        this.start = this.push(Graph::Apply(i, stdin));
        this.print_list()
    }
}
