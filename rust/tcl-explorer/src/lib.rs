//! Compiler-explorer pipeline over the Tcl compiler crates.
//!
//! This is the Rust home of `tooling/explorer/pipeline.py` — the single
//! source-in / result-out entry point that the CLI (`tcl explore`), the
//! TUI, and the Rust → WASM web GUI all share. It is a *thin aggregator*:
//! the heavy lifting (lexing, lowering, CFG/SSA, the analyses) lives in
//! `tcl-compiler`; this crate only assembles those artefacts into the
//! shape the explorer front-ends render, and (in later phases) serialises
//! them to the `wasm-explorer-view.md` JSON contract.
//!
//! No `pyo3` dependency, no filesystem, no I/O — `run_pipeline` is pure
//! compute over a source string so the same code compiles to
//! `wasm32-unknown-unknown` for the in-browser GUI.

#![forbid(unsafe_code)]

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_registry::registry_for_dialect;

/// Per-function compilation artefacts surfaced by the explorer.
///
/// Mirrors `FunctionSnapshot` in `pipeline.py`. The underlying
/// [`FunctionUnit`] already carries the CFG, SSA, def-use, SCCP, type, and
/// taint results, so the snapshot is just a named, ordered view onto the
/// [`CompilationUnit`]'s function table (top-level first, then procedures
/// in qualified-name order — matching `build_snapshots`).
#[derive(Debug, Clone, Copy)]
pub struct FunctionSnapshot<'a> {
    /// Qualified function name (`::top` for the top-level script).
    pub name: &'a str,
    /// The full per-function unit (CFG, SSA, analyses).
    pub unit: &'a FunctionUnit,
}

impl FunctionSnapshot<'_> {
    /// Number of basic blocks in this function's CFG.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.unit.cfg.blocks.len()
    }
}

/// Complete result of running the explorer pipeline.
///
/// Owns the [`CompilationUnit`]; the serialiser and renderers borrow from
/// it. Mirrors `CompilerExplorerResult` in `pipeline.py`, fleshed out
/// view-by-view in later phases (EXP-1..N).
#[derive(Debug)]
pub struct ExplorerResult {
    /// The source that was compiled.
    pub source: String,
    /// The dialect the pipeline was configured for.
    pub dialect: String,
    /// The shared compilation unit (IR module, CFG module, per-function units).
    pub unit: CompilationUnit,
}

impl ExplorerResult {
    /// Per-function snapshots: top-level first, then procedures in
    /// qualified-name order (matching `build_snapshots`).
    #[must_use]
    pub fn snapshots(&self) -> Vec<FunctionSnapshot<'_>> {
        let mut out = vec![FunctionSnapshot {
            name: "::top",
            unit: &self.unit.top_level,
        }];
        let mut qnames: Vec<&String> = self.unit.procedures.keys().collect();
        qnames.sort();
        for qname in qnames {
            out.push(FunctionSnapshot {
                name: qname,
                unit: &self.unit.procedures[qname],
            });
        }
        out
    }

    /// Total basic-block count across every function (matching
    /// `compute_stats`' `blocks`).
    #[must_use]
    pub fn total_blocks(&self) -> usize {
        self.snapshots()
            .iter()
            .map(FunctionSnapshot::block_count)
            .sum()
    }
}

/// Run the full explorer pipeline on `source` for `dialect`.
///
/// Builds the [`CompilationUnit`] (the analogue of Python's
/// `ensure_compilation_unit`) including interprocedural analysis, exactly
/// as `run_pipeline` does. The per-pass graceful-degradation Python needs
/// (`try/except` around optional passes) collapses here because the Rust
/// passes are infallible — they are run as part of building the unit.
#[must_use]
pub fn run_pipeline(source: &str, dialect: &str) -> ExplorerResult {
    let registry = registry_for_dialect(dialect);
    let unit = CompilationUnit::build_for(source, registry, false)
        .with_interprocedural(registry, Some(dialect));

    ExplorerResult {
        source: source.to_owned(),
        dialect: dialect.to_owned(),
        unit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_pipeline_top_level_only() {
        let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
        let snapshots = result.snapshots();
        // Only the top-level function exists for a script with no procs.
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].name, "::top");
        assert!(result.total_blocks() >= 1);
    }

    #[test]
    fn run_pipeline_collects_procedures_in_order() {
        let src = "proc beta {} { return 2 }\nproc alpha {} { return 1 }\nalpha";
        let result = run_pipeline(src, "tcl8.6");
        let names: Vec<&str> = result.snapshots().iter().map(|s| s.name).collect();
        // Top-level first, then procedures sorted by qualified name.
        assert_eq!(names[0], "::top");
        let procs = &names[1..];
        assert!(procs.contains(&"::alpha"));
        assert!(procs.contains(&"::beta"));
        let mut sorted = procs.to_vec();
        sorted.sort_unstable();
        assert_eq!(procs, sorted.as_slice());
    }

    #[test]
    fn unparseable_dialect_does_not_panic() {
        let result = run_pipeline("set x 1", "not-a-real-dialect");
        assert_eq!(result.dialect, "not-a-real-dialect");
        assert!(!result.snapshots().is_empty());
    }
}
