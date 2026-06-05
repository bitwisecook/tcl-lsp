//! PARSER-CONVERGENCE PROBE (dev tool). Dumps the `tcl-lexer` token stream for
//! the tricky cases, to design the runtime's lowering from the shared lexer onto
//! the eval `Command`/`WordPart` model (see `rust-runtime-port.md`, "Parser
//! convergence"). Run: `cargo run --example probe`.
use tcl_lexer::{Lexer, SourceMap, TokenType};
fn dump(s: &str) {
    let sm = SourceMap::new(s);
    print!("{s:?}  =>  ");
    match Lexer::new(s).tokenise_all() {
        Ok(toks) => {
            for t in toks {
                if t.kind == TokenType::Eof {
                    continue;
                }
                print!("{}({:?}) ", t.kind.name(), sm.text(t.span));
            }
        }
        Err(e) => print!("ERR {e:?}"),
    }
    println!();
}
fn main() {
    for s in [
        "set x [foo $y]",
        "a$b[c]",
        "x${name}y",
        "$arr($i)",
        "a\\tb",
        "{a\nb}",
        "{a\\\nb}",
        "{*}$args",
        "{*} x",
        "# comment",
        "set x #y",
        "a; b",
        "\"q $v\"",
    ] {
        dump(s);
    }
}
