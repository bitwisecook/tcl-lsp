//! Signature-only scan for background-indexed Tcl files.
//!
//! Port of `core/analysis/signature_scan.py` (chunk **C40**). Walks
//! the segmented command stream of a Tcl source and extracts a
//! lightweight [`SignatureScanResult`] subset — proc / class
//! definitions, package requires, source targets, command aliases,
//! namespace imports, auto-path entries, and a flat command-invocation
//! list — without running the full analyser pipeline.
//!
//! Used by cross-file LSP features (workspace symbols, `package
//! require` resolution, command-usage counts) on every non-OPEN
//! document; the full analyser still runs on `didOpen` /
//! `didChange` so OPEN files are unaffected.
//!
//! The walker stays at the segmenter level (not the IR level) — the
//! whole point of this module is to be fast for background-indexed
//! files, so it deliberately avoids the lowering pass.
//!
//! Submodules are filled in by the C40 sub-strips (`C40a*` types,
//! `C40b*` per-command handlers, `C40c*` walker, `C40d*` factory
//! resolution + entry point, `C40e*` `PyO3` binding + Python shim +
//! differential harness).

mod ctx;
mod handlers;
pub mod params;
pub mod types;
mod walker;
