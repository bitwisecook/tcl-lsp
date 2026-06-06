//! Minimal script runner (dev tool) — evaluate a Tcl script through the runtime
//! interpreter and print its result.
//!
//! Usage: `cargo run --example run_script -- path/to/script.tcl`
//!        `echo 'puts [expr {2+2}]' | cargo run --example run_script`
//!
//! This is the end-to-end "execute a simple script" path: parse → eval loop →
//! builtins (`set`/`expr`/`if`/`while`/`for`/`foreach`/`proc`/`puts`/…). `puts`
//! writes to stdout directly; the script's final result is printed last.

use std::io::Read;

use tcl_runtime::interp::{Code, Interp};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let src = if let Some(path) = args.get(1) {
        std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("run_script: cannot read {path}: {e}");
            std::process::exit(2);
        })
    } else {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).expect("read stdin");
        buf
    };

    let mut interp = Interp::new();
    let code = interp.eval_str(&src);
    let result = interp.result_bytes();
    if code == Code::Error {
        eprintln!("error: {}", String::from_utf8_lossy(&result));
        std::process::exit(1);
    }
    // Print the script's final result (if any), like an interactive evaluation.
    if !result.is_empty() {
        println!("{}", String::from_utf8_lossy(&result));
    }
}
