//! Internal scan-state types used by the `signature_scan` walker.
//!
//! These are crate-private — they accumulate intermediate results
//! during the walk and are consumed by the second-pass factory
//! resolver (see [`super::factory::resolve_factory_defs`]) before
//! the public [`SignatureScanResult`] is returned.
//!
//! - [`ScanCtx`] is threaded as `&mut` through the walker and every
//!   handler; it owns the public `result` accumulator plus the two
//!   first-pass vectors `candidates` and `proc_bodies`.
//! - [`FactoryCandidate`] records each four-token call (`HEAD NAME
//!   ARGS BODY`) the walker spotted, deferring binding to a real
//!   factory until pass two.
//! - [`ProcBodyInfo`] records each proc body's text + params +
//!   home namespace so the factory detector can spot the canonical
//!   `proc $name $args $body` shape.
//! - [`FACTORY_SKIP_HEADS`] is the negative list of built-in heads
//!   that incidentally take the same four-token shape but are
//!   definitely not factory wrappers.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

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
    /// Command heads that match the factory-wrapper token shape but
    /// are not factories — sourced from the registry's
    /// `NOT_PROC_FACTORY` trait plus [`FACTORY_SKIP_NONCOMMAND_HEADS`],
    /// built once by `extract_signatures`.  Empty in `Default`
    /// (used only by focused unit tests).
    pub(super) skip_heads: std::collections::HashSet<String>,
}

/// Factory-skip heads that are **not** registered commands and so
/// cannot carry the registry's `NOT_PROC_FACTORY` trait: the `TclOO`
/// definition keywords `method` / `classmethod` and the
/// (unregistered) itcl class-definition heads.  `extract_signatures`
/// unions these with the registry-stamped heads to rebuild the full
/// former `_FACTORY_SKIP_HEADS` set.
pub(super) const FACTORY_SKIP_NONCOMMAND_HEADS: &[&str] =
    &["method", "classmethod", "itcl::class", "::itcl::class"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noncommand_skip_heads_are_the_unregistered_four() {
        assert_eq!(FACTORY_SKIP_NONCOMMAND_HEADS.len(), 4);
        for head in ["method", "classmethod", "itcl::class", "::itcl::class"] {
            assert!(FACTORY_SKIP_NONCOMMAND_HEADS.contains(&head));
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
