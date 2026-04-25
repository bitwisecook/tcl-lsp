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
//! Module layout:
//!
//! - [`types`] — public record types ([`SignatureScanResult`] +
//!   per-collection records) plus [`ParamDef`].
//! - [`params`] — `parse_param_list` for the proc parameter-list arg.
//! - `ctx` (private) — internal scan state ([`ScanCtx`],
//!   `FactoryCandidate`, `ProcBodyInfo`, `FACTORY_SKIP_HEADS`).
//! - `handlers` (private) — per-command handlers
//!   (`handle_proc`, `handle_namespace`, `handle_package`, …).
//! - `walker` (private) — top-level `scan` plus the body-recursion
//!   helpers (`maybe_recurse_body`, `handle_if` / `handle_catch` /
//!   `handle_try`) and the factory-body sub-walker
//!   (`scan_factory_candidates` / `scan_factory_structural`).
//! - `factory` (private) — second-pass factory-wrapper resolver
//!   (`is_factory_body`, `lookup_factory`, `resolve_factory_defs`).
//!
//! The public entry point is [`extract_signatures`].
//!
//! [`ParamDef`]: types::ParamDef

mod ctx;
mod factory;
mod handlers;
pub mod params;
pub mod types;
mod walker;

use tcl_registry::CommandRegistry;

use ctx::ScanCtx;
pub use types::SignatureScanResult;

/// Extract a lightweight [`SignatureScanResult`] for a Tcl source.
///
/// The public entry point for the signature scanner. Mirrors
/// `extract_signatures` in `core/analysis/signature_scan.py` —
/// runs the main walker, then resolves factory-wrapper synthetic
/// procs.
///
/// The `_registry` parameter is plumbed through but not currently
/// consulted; it matches every other tcl-compiler analysis entry
/// point's signature so future heuristics that need command-spec
/// metadata can read it without an API change.
#[must_use]
pub fn extract_signatures(source: &str, _registry: &CommandRegistry) -> SignatureScanResult {
    let mut ctx = ScanCtx::default();
    walker::scan(source, None, "", false, &mut ctx);
    factory::resolve_factory_defs(&mut ctx);
    ctx.result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> SignatureScanResult {
        let registry = CommandRegistry::build_default();
        extract_signatures(src, &registry)
    }

    #[test]
    fn proc_only() {
        let r = run("proc foo {} {}");
        assert!(r.procs.contains_key("::foo"));
        assert!(r.classes.is_empty());
        assert_eq!(r.command_invocations.len(), 1);
    }

    #[test]
    fn class_only() {
        let r = run("oo::class create MyCls { method foo {} {} }");
        assert!(r.classes.contains_key("::MyCls"));
        assert!(r.procs.is_empty());
    }

    #[test]
    fn namespace_eval_with_inner_proc() {
        let r = run("namespace eval ns { proc inner {} {} }");
        assert!(r.procs.contains_key("::ns::inner"));
    }

    #[test]
    fn package_require_and_source() {
        let r = run("package require Tcl 8.6\nsource /a/b.tcl");
        assert_eq!(r.package_requires.len(), 1);
        assert_eq!(r.package_requires[0].name, "Tcl");
        assert_eq!(r.source_targets.len(), 1);
        assert!(r.source_targets[0].is_literal);
    }

    #[test]
    fn interp_alias_local_recorded() {
        let r = run("interp alias {} myalias {} puts hello");
        assert!(r.command_aliases.contains_key("::myalias"));
    }

    #[test]
    fn tcllib_factory_wrapper_emits_synthetic_proc() {
        let src = "
            proc DEFC {name args body} {
                proc $name $args $body
            }
            proc INIT {} {
                DEFC ChildA {x} {return 1}
                DEFC ChildB {y} {return 2}
            }
        ";
        let r = run(src);
        // The wrapper itself + the INIT proc are real.
        assert!(r.procs.contains_key("::DEFC"));
        assert!(r.procs.contains_key("::INIT"));
        // Synthetic factory-emitted procs should be present too.
        assert!(r.procs.contains_key("::ChildA"));
        assert!(r.procs.contains_key("::ChildB"));
        // Synthetic procs have empty params (per-call arg map is
        // not statically known).
        assert!(r.procs["::ChildA"].params.is_empty());
    }
}
