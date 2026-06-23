//! Minimal JSON dumper for the differential parity harness.
//!
//! Reads Tcl source (from `--source <text>`, a file path argument, or
//! stdin when neither is given) and prints `serialise_result` as JSON.
//! The differential parity harness shells out to this and compares
//! the two implementations key-by-key.
//!
//! It deliberately has no dependencies beyond `std`.

use std::io::Read;

fn main() {
    let mut dialect = String::from("tcl8.6");
    let mut source: Option<String> = None;
    let mut path: Option<String> = None;
    // When set, dump `build_view(view, serialise_result)` instead of the
    // serialised result — used by the TUI-tree parity harness.
    let mut view_tree: Option<String> = None;
    // When set, read an already-serialised result as JSON from stdin and dump
    // `build_view(view, that)` — isolates the tree builder from any
    // serialise divergence (both sides build from identical data).
    let mut view_tree_stdin: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dialect" => dialect = args.next().unwrap_or(dialect),
            "--source" => source = args.next(),
            "--view-tree" => view_tree = args.next(),
            "--view-tree-stdin" => view_tree_stdin = args.next(),
            other => path = Some(other.to_owned()),
        }
    }

    if let Some(view) = view_tree_stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok();
        let data: serde_json::Value = serde_json::from_str(&buf).expect("valid JSON on stdin");
        println!(
            "{}",
            serde_json::to_string(&tcl_explorer::build_view(&view, &data)).expect("serialise")
        );
        return;
    }

    let source = source.unwrap_or_else(|| {
        if let Some(p) = path {
            std::fs::read_to_string(&p).unwrap_or_else(|e| {
                eprintln!("error reading {p}: {e}");
                std::process::exit(2);
            })
        } else {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok();
            buf
        }
    });

    let result = tcl_explorer::run_pipeline(&source, &dialect);
    let value = tcl_explorer::serialise_result(&result);

    let out = if let Some(view) = view_tree {
        serde_json::to_string(&tcl_explorer::build_view(&view, &value))
    } else {
        serde_json::to_string(&value)
    };
    println!("{}", out.expect("serialise to JSON"));
}
