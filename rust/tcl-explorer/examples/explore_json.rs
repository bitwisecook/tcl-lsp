//! Minimal JSON dumper for the differential parity harness.
//!
//! Reads Tcl source (from `--source <text>`, a file path argument, or
//! stdin when neither is given) and prints `serialise_result` as JSON —
//! the same shape `tooling/cli/serialise.py::serialise_result` produces.
//! `tests/test_explorer_rust_parity.py` shells out to this and compares
//! the two implementations key-by-key.
//!
//! This is a throwaway bridge until the real `tcl explore --json` verb
//! lands (EXP-CLI); it deliberately has no dependencies beyond `std`.

use std::io::Read;

fn main() {
    let mut dialect = String::from("tcl8.6");
    let mut source: Option<String> = None;
    let mut path: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dialect" => dialect = args.next().unwrap_or(dialect),
            "--source" => source = args.next(),
            other => path = Some(other.to_owned()),
        }
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
    println!(
        "{}",
        serde_json::to_string(&value).expect("serialise to JSON")
    );
}
