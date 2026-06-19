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
    let mut args: Vec<String> = std::env::args().collect();
    // `--init` bootstraps the standard library (TCL_LIBRARY → source init.tcl)
    // before evaluating, like a real `tclsh`.
    let init = args.get(1).map(String::as_str) == Some("--init");
    if init {
        args.remove(1);
    }
    // A file path is *sourced* (like `tclsh script.tcl`: `info script` is set and
    // `info frame` reports `type source`); stdin is evaluated as a script.
    let path = args.get(1).cloned();
    let src = if let Some(path) = &path {
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
    if init && interp.init_library() == Code::Error {
        eprintln!(
            "init error: {}",
            String::from_utf8_lossy(&interp.result_bytes())
        );
        std::process::exit(1);
    }
    let code = match &path {
        Some(p) => interp.eval_sourced(&src, p.as_bytes()),
        None => interp.eval_str(&src),
    };
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
