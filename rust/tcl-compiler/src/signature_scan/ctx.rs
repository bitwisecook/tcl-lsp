//! Internal scan-state types used by the `signature_scan` walker.
//!
//! These are crate-private — they accumulate intermediate results
//! during the walk and are consumed by the second-pass factory
//! resolver before the public [`SignatureScanResult`] is returned.
//!
//! Fields carry `#[allow(dead_code)]` until the C40b/c/d sub-strips
//! that consume them land; once those wire-ins are in place, the
//! attributes can be removed.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

#![allow(dead_code)]

use tcl_lexer::Token;

use super::types::SignatureScanResult;

/// A factory-wrapper call captured during the first scan pass.
///
/// `signature_scan` recognises tcllib-style factory wrappers (e.g.
/// `DEFC name args body`) by their canonical four-token shape and
/// defers binding to a real factory until after the full source has
/// been scanned. `FactoryCandidate` records the call-site data the
/// second pass needs to attribute the synthetic proc to the right
/// namespace.
#[derive(Debug, Clone)]
pub(super) struct FactoryCandidate {
    /// The command head as written at the call site (e.g. `"DEFC"`,
    /// `"::foo::DEF"`).
    pub(super) head: String,
    /// The proc-name argument as written.
    pub(super) name: String,
    /// Token of the name argument (used for the synthetic proc's
    /// `name_range`).
    pub(super) name_tok: Token,
    /// Token of the body argument (used for the synthetic proc's
    /// `body_range`).
    pub(super) body_tok: Token,
    /// Effective namespace at the call site, no leading `::`.
    pub(super) ns_prefix: String,
}

/// First-pass record of a proc body, used to identify factory
/// wrappers by their `proc $p1 $p2 $p3` body shape.
///
/// The wrapper's home namespace is the namespace any factory-emitted
/// procs end up in at runtime — `proc $name …` executed inside a
/// proc creates the command in the caller's current namespace, which
/// for a factory-wrapper INIT path is unambiguously the wrapper's
/// own home.
#[derive(Debug, Clone)]
pub(super) struct ProcBodyInfo {
    /// Fully-qualified proc name with leading `::`.
    pub(super) qname: String,
    /// Parameter names (in declaration order) — the factory body
    /// detector matches these against the `proc $a $b $c` body
    /// shape.
    pub(super) params: Vec<String>,
    /// Verbatim proc body text.
    pub(super) body_text: String,
    /// Namespace any synthetic procs created by this wrapper live
    /// in, no leading `::`.
    pub(super) ns_prefix: String,
}

/// Mutable scan context threaded through the walker.
#[derive(Debug, Default)]
pub(super) struct ScanCtx {
    /// Public result accumulator.
    pub(super) result: SignatureScanResult,
    /// Factory-call candidates collected during pass 1.
    pub(super) candidates: Vec<FactoryCandidate>,
    /// Proc-body records collected during pass 1, used to identify
    /// real factory wrappers in pass 2.
    pub(super) proc_bodies: Vec<ProcBodyInfo>,
}

/// Heads that incidentally match the `HEAD NAME BRACED BRACED`
/// four-token shape but are definitely not factory wrappers.
///
/// Mirrors `_FACTORY_SKIP_HEADS` in `core/analysis/signature_scan.py`.
pub(super) const FACTORY_SKIP_HEADS: &[&str] = &[
    "proc",
    "namespace",
    "if",
    "switch",
    "while",
    "for",
    "foreach",
    "try",
    "catch",
    "eval",
    "apply",
    "expr",
    "uplevel",
    "upvar",
    "variable",
    "set",
    "lappend",
    "dict",
    "array",
    "string",
    "list",
    "lindex",
    "package",
    "source",
    "interp",
    "oo::class",
    "oo::define",
    "oo::objdefine",
    "method",
    "classmethod",
    "itcl::class",
    "::itcl::class",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_heads_matches_python_count() {
        // The Python list at signature_scan.py:546 has 32 entries.
        assert_eq!(FACTORY_SKIP_HEADS.len(), 32);
    }

    #[test]
    fn skip_heads_includes_canonical_builtins() {
        for head in ["proc", "namespace", "if", "for", "package", "source"] {
            assert!(
                FACTORY_SKIP_HEADS.contains(&head),
                "expected {head:?} in skip list"
            );
        }
    }

    #[test]
    fn default_ctx_is_empty() {
        let ctx = ScanCtx::default();
        assert!(ctx.candidates.is_empty());
        assert!(ctx.proc_bodies.is_empty());
        assert!(ctx.result.procs.is_empty());
        assert!(ctx.result.command_invocations.is_empty());
    }
}
