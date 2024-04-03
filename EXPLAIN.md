# RustでのLazy K実装

Lazy Kは3つのコンビネータS,K,Iと関数適用のみで構成される言語で遅延評価をもとに計算する。

```
S x y z = (x z)(y z)
K x y = x
I x = x
```

Rustで単純に構文木を定義するとこうなる。

```rust
#[derive(Clone)]
enum Expr {
    S, K, I,
    Apply(Rc<Expr>, Rc<Expr>)
}
```

Rustの制約上、再帰的にデータが現われるところはなんらかのポインタで包む必要がある。今回は簡単にcloneしたいのでRcにした。

## 遅延評価

遅延評価の実装のためにG-machineを使った。
G-machineは構文木を左に辿りながら簡約基を探すので、最外簡約ができる。

また、遅延評価は簡約ステップの削減のために一度評価した部分式は書き換える必要があるが、
今のデータでは書き換えができない。
RefCellにしてもいいが、以下の理由からRefCellは使用しなかった。

* borrow_mut()やborrow().deref()などが頻発し、書き辛い。
* 簡約して構文木を組み直すときにRefCell::new()を呼び出すため、アロケーション回数が多くなり遅い。

今回の実装では計算用のデータとして以下を使用した。

```rust
type Addr = usize;
#[derive(Clone)]
enum Graph {
    S, K, I,
    Apply(Addr, Addr)
}
type Memory = Vec<Graph>;
```

`Addr`は`Memory`でのindexを表わしている。
遅延評価で部分式を書き換えるときは`Memory`内の該当の場所を書き換える。

### G-machine

G-machineではまず構文木を左に辿りながらスタックに経路を積み上げていく。

```rust
type Stack = Vec<Graph>;
fn spine(mut top: Addr, memory: &Memory, stack: &mut Stack) {
    loop {
        stack.push(top);
        match &memory[top] {
            Graph::Apply(l, _) => top = *l,
            _ => break,
        }
    }
}
```

スタックに積んだあとは、スタックのトップにコンビネータがあり、トップ以下の数が引数の数と同じになっているので
トップのコンビネータに応じて処理を分岐する。

```rust
fn get_rhs(memory: &Memory, addr: Addr) -> Addr {
    match &memory[addr] {
        Graph::Apply(_, rhs) => *rhs,
        _ => unreachable!(),
    }
}
fn reduce(top: Addr, memory: &mut Memory) -> bool {
    let mut stack = Vec::new();
    spine(top, memory, &mut stack);
    let Some(f) = stack.pop() else { unreachable!() };
    match memory[f] {
        S => {
            let Some(r1) = stack.pop() else { return false; };
            let Some(r2) = stack.pop() else { return false; };
            let Some(r3) = stack.pop() else { return false; };
            let x = get_rhs(memory, r1);
            let y = get_rhs(memory, r2);
            let z = get_rhs(memory, r3);
            let lhs = memory.len();
            memory.push(Apply(x, z));
            let rhs = memory.len();
            memory.push(Apply(y, z));
            memory[r3] = Apply(lhs, rhs);
            true
        }
        K => {
            let Some(r1) = stack.pop() else { return false; };
            let Some(r2) = stack.pop() else { return false; };
            let x = get_rhs(memory, r1);
            memory[r2] = memory[x].clone();
            true
        }
        I => {
            let Some(r) = stack.pop() else { return false };
            let x = get_rhs(memory, r);
            memory[r] = memory[x].clone();
            true
        }
    }
}
```

`reduce`が`false`を返すまで繰り返せばWHNFまで簡約できるが、いくつか無駄がある。

* 毎回0からスタックを積み上げる必要がない。簡約した後に`spine`を呼べばいい。
* Sの簡約時に`memory.push`で新たなメモリを必要とする。Kの簡約で不要になったスペースをうまく使いたい。
* K, Iの簡約で`clone`すると必要なメモリ量が増える。

これを改善すると以下になる。

```rust
enum Graph {
    // 以下を追加
    Link(Addr), Free
}
fn follow_link(memory: &Memory, mut expr: Addr) -> Addr {
    while let Graph::Link(arg1) = memory[expr] {
        expr = arg1;
    }
    expr
}
fn push(memory: &mut Memory, graph: Graph) -> ExprId {
    for i in 0..memory.len() {
        if let Graph::Free = memory[i] {
            memory[i] = graph;
            return i as ExprId;
        }
    }
    let id = memory.len();
    memory.push(graph);
    id
}
fn reduce(top: Addr, memory: &mut Memory) {
    use Graph::*;
    let mut stack = Vec::new();
    spine(top, memory, &mut stack);
    loop {
        let Some(mut f) = stack.pop() else { break };
        f = follow_link(memory, f);
        match memory[f] {
            S => {
                let Some(r1) = stack.pop() else { break };
                let Some(r2) = stack.pop() else { break };
                let Some(r3) = stack.pop() else { break };
                let x = get_rhs(memory, r1);
                let y = get_rhs(memory, r2);
                let z = get_rhs(memory, r3);
                let lhs = push(memory, Apply(x, z));
                let rhs = push(memory, Apply(y, z));
                memory[r3] = Apply(lhs, rhs);
                spine(r3, memory, &mut stack);
            }
            K => {
                let Some(r1) = stack.pop() else { break };
                let Some(r2) = stack.pop() else { break };
                let x = get_rhs(memory, r1);
                memory[r2] = Link(x);
                spine(r2, memory, &mut stack);
            }
            I => {
                let Some(r) = stack.pop() else { break };
                let x = get_rhs(r, memory);
                memory[r] = Link(x);
                spine(r, memory, &mut stack);
            }
        }
    }
}
```

これにより、`reduce`を一回呼ぶだけでWHNFまで計算でき、
以下で定義する`garbage_collect`と交互に呼べばメモリ使用量を抑え、
アロケーションで遅くなるのを防げる。

```rust
fn garbage_collect(memory: &mut Memory, top: Addr) {
    let mut need = vec![false; memory.len()];
    let mut queue = VecDeque::new();
    queue.push_front(top);
    while let Some(id) = queue.pop_back() {
        if need[id] {
            continue;
        }
        need[id] = true;
        match &memory[id] {
            Graph::Apply(l, r) => {
                queue.push_front(*l);
                queue.push_front(*r);
            }
            Graph::Link(x) => queue.push_front(*x),
            _ => continue,
        }
    }

    // S,K,Iがmemoryの先頭に置かれている想定
    for i in 3..memory.len() {
        if !need[i] {
            memory[i] = Graph::Free;
        }
    }
}
```

## I/O

Lazy Kのプログラムは文字列を文字列に変換する関数として動作する。
そして文字列はASCIIコードのリストとして扱われる。
ここで数値はchurch encodingされ、リストはscott encodingされる。

### 出力
まず、出力部分だけ考える。
scott encodingのリスト`xs`は`xs K`で先頭要素がでるので、
先頭要素を数値にしてASCIIコードとして出力すれば先頭1文字がでる。

また、`xs (K I)`でtailがでるので、上を繰り返せば出力部分は解決。

`church encoding → 数値`は内部的に使う特殊な項を追加して解決した。

```rust
enum Graph {
    // 追加
    Inc, Num(u16),
}
fn reduce(top: Addr, memory: &mut Memory) {
    loop {
        match memory[f] {
            // 以下の2パターンを追加
            Inc => {
                let Some(r) = stack.pop() else { break };
                let mut n = get_rhs(memory, r);
                reduce(n, memory);
                n = follow_link(memory, n);
                match &memory[n] {
                    Num(n) => {
                        memory[r] = Num(n + 1);
                        stack.push(r);
                    }
                    _ => panic!("cannot increment"),
                };
            }
            Num(_) => break,
        }
    }
}
fn print_list(memory: &mut Memory, mut top: Addr) {
    let mut writer = stdout().lock();
    loop {
        let head = push(memory, Graph::Apply(top, K));

        let inc = push(memory, Graph::Inc);
        let zero = push(memory, Graph::Num(0));
        let i = push(memory, Graph::Apply(head, inc));
        let g = push(memory, Graph::Apply(i, zero));
        reduce(memory, g);
        match &memory[g] {
            Graph::Num(ch) => {
                // Lazy Kは256以上の数値で停止する
                if *ch >= 256 {
                    break;
                };
                let _ = writer.write(&[(ch & 0xFF) as u8]);
                let ki = push(Graph::Apply(K, I));
                top = push(memory, Graph::Apply(top, ki));
            }
            _ => {
                panic!("cannot reduce to numeric value")
            }
        };
        if memory.len() * size_of::<Graph>() > 256 * 1024 * 1024 {
            garbage_collect(memory, top);
        }
    }
    let _ = writer.flush();
}
```

### 入力

Lazy Kは入力も遅延することに気をつける(そうでないとインタラクティブな処理ができない)。
つまり、入力が必要になったタイミングで入力待ちになるようにする。

これは入力文字列をあらわす項を導入して解決した。

```rust
enum Graph { 
    // 追加
    Stdin
}
fn reduce(top: Addr, memory: &mut Memory) {
    loop {
        match memory[f] {
            // 追加
            Stdin => {
                let _ = stdout().flush();
                let mut buf = [0u8; 1];
                let n = match stdin().read_exact(&mut buf) {
                    Ok(_) => buf[0] as u16,
                    _ => 256,
                };
                let church = push_church(memory, n);
                let stdin = push(memory, Stdin);
                let cons = push_cons(memory, church, stdin);
                memory[f] = Link(cons);
                spine(f, memory, &mut stack);
            }
        }
    }
}
```

## 実行

入出力を扱えるようになったため、Lazy Kプログラムを実行できる。

```rust
const S: Addr = 0;
const K: Addr = 1;
const I: Addr = 2;
fn push_expr(memory: &mut Memory, expr: &Expr) -> ExprId {
    match expr {
        Expr::I => I,
        Expr::K => K,
        Expr::S => S,
        Expr::Apply(l, r) => {
            let l = push_expr(memory, l.as_ref());
            let r = push_expr(memory, r.as_ref());
            push(memory, Graph::Apply(l, r))
        }
    }
}
fn run(expr: &Expr) {
    let mut memory = vec![Graph::S, Graph::K, Graph::I];
    let i = push_expr(&mut memory, expr);
    let stdin = push(&mut memory, Graph::Stdin);
    let top = push(&mut memory, Graph::Apply(i, stdin));
    print_list(&mut memory, top)
}
```
