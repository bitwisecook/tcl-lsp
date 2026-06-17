//! Rust-only byte-identity gate (golden corpus).
//!
//! Compiles each fixture under `tests/fixtures/codegen/matching/` through the
//! Rust pipeline and compares the (label-stripped) disassembly to a checked-in
//! `*.golden`. Unlike `differential_codegen.rs` — which oracles against the
//! Python emitter and silently skips when `python3` is absent — this needs no
//! Python, so the byte-identity safety net for the bytecode emitter survives
//! Python's retirement. It also guards the bytecode emitter against accidental
//! churn while the greenfield WASM backend is built alongside it.
//!
//! Regenerate the goldens after an *intentional* codegen change:
//!
//! ```text
//! BLESS_GOLDEN=1 cargo test -p tcl-compiler --test codegen_golden
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

/// Compile `source` through the Rust pipeline and render its disassembly.
fn rust_disasm(source: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(source, &registry);
    let cfg = build_cfg(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    format_module_asm(&asm)
}

/// Strip label-comment lines (`  # label:`) so the compare ignores internal
/// label names (only the bytecode stream / literals / jump targets matter).
fn strip_label_comments(disasm: &str) -> String {
    disasm
        .lines()
        .filter(|l| !l.trim_start().starts_with("# "))
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn matching_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codegen/matching")
}

#[test]
fn rust_codegen_matches_golden_corpus() {
    let bless = std::env::var_os("BLESS_GOLDEN").is_some();
    let dir = matching_dir();
    let mut fixtures: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read matching corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tcl"))
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(), "no fixtures in {}", dir.display());

    let mut problems: Vec<String> = Vec::new();
    for tcl in &fixtures {
        let name = tcl.file_name().unwrap().to_string_lossy().into_owned();
        let source = fs::read_to_string(tcl).expect("read fixture");
        let disasm = strip_label_comments(&rust_disasm(&source));
        let golden = tcl.with_extension("golden");
        if bless {
            fs::write(&golden, format!("{disasm}\n")).expect("write golden");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(expected) if disasm.trim_end() == expected.trim_end() => {}
            Ok(_) => problems.push(format!(
                "{name}: disassembly changed (BLESS_GOLDEN=1 to re-bless if intended)"
            )),
            Err(_) => problems.push(format!("{name}: missing golden (BLESS_GOLDEN=1 to create)")),
        }
    }

    assert!(
        problems.is_empty(),
        "Rust bytecode codegen diverged from the golden corpus:\n  {}",
        problems.join("\n  ")
    );
}
