//! Ensembles (T1.5) — the canonical `ens sub …` → `target …` redirect.
//!
//! An ensemble command maps its first argument (a subcommand) to a target
//! command prefix and forwards the rest — the generalisation of the
//! `dict for` → `::tcl::dict::for` rewrite (the A3 contract: "ensembles map
//! `ens sub` → target (default `::ens::sub` or `-map`, unambiguous-prefix unless
//! `-prefix 0`)"). Modelled on C Tcl 9's `tclEnsemble.c`.
//!
//! This module is the **pure** part: the config an ensemble carries and the
//! subcommand-resolution + error-wording rules. `namespace ensemble create`
//! builds an [`EnsembleConfig`] (`cmd_namespace.rs`); the dispatch trampoline
//! that re-dispatches to the target lives on the interp (`interp.rs`), the same
//! split as `interp alias` (build in `cmd_alias.rs`, dispatch in `interp.rs`).

use crate::namespace::NsId;

/// An ensemble `-map`: each entry is `(subcommand, target command prefix words)`.
pub type EnsembleMap = Vec<(Vec<u8>, Vec<Vec<u8>>)>;

/// An ensemble command's configuration (the payload of
/// [`Command::Ensemble`](crate::interp::Command)).
#[derive(Clone, Debug)]
pub struct EnsembleConfig {
    /// The namespace subcommands dispatch into (default targets are
    /// `<ns>::<sub>`); the ns `namespace ensemble create` ran in.
    pub ns: NsId,
    /// `-map`: subcommand → target command prefix (words). When present without
    /// an explicit `-subcommands`, its keys are the valid subcommand set.
    pub map: Option<EnsembleMap>,
    /// `-subcommands`: the explicit valid subcommand set. When `None` (and no
    /// `-map`), the set is the namespace's exported commands.
    pub subcommands: Option<Vec<Vec<u8>>>,
    /// `-prefixes`: allow unambiguous-prefix subcommand matching (default true).
    pub prefixes: bool,
}

/// Resolve `sub` against the (sorted) subcommand set: an exact match wins;
/// otherwise, if `prefixes`, a *unique* prefix matches. Returns the index into
/// `subs`, or `None` for unknown / ambiguous.
#[must_use]
pub fn resolve_subcommand(subs: &[Vec<u8>], sub: &[u8], prefixes: bool) -> Option<usize> {
    if let Some(i) = subs.iter().position(|s| s.as_slice() == sub) {
        return Some(i); // exact match (beats any longer prefix-sharing name)
    }
    if !prefixes {
        return None;
    }
    let mut found = None;
    for (i, s) in subs.iter().enumerate() {
        if s.starts_with(sub) {
            if found.is_some() {
                return None; // ambiguous prefix
            }
            found = Some(i);
        }
    }
    found
}

/// The `must be a, b, or c` clause (Tcl's `, or`-before-last join; a single item
/// is bare).
#[must_use]
pub fn must_be(subs: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    let last = subs.len().saturating_sub(1);
    for (i, s) in subs.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b", ");
            if i == last {
                out.extend_from_slice(b"or ");
            }
        }
        out.extend_from_slice(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&[u8]]) -> Vec<Vec<u8>> {
        items.iter().map(|s| s.to_vec()).collect()
    }

    #[test]
    fn exact_and_prefix_resolution() {
        let subs = v(&[b"bar", b"baz"]);
        assert_eq!(resolve_subcommand(&subs, b"bar", true), Some(0));
        assert_eq!(resolve_subcommand(&subs, b"baz", true), Some(1));
        // `ba` / `b` are ambiguous between bar and baz.
        assert_eq!(resolve_subcommand(&subs, b"ba", true), None);
        assert_eq!(resolve_subcommand(&subs, b"b", true), None);
        // an exact match beats prefix ambiguity.
        let subs2 = v(&[b"bar", b"barx"]);
        assert_eq!(resolve_subcommand(&subs2, b"bar", true), Some(0));
        // prefixes off → only exact.
        assert_eq!(resolve_subcommand(&subs, b"ba", false), None);
        assert_eq!(resolve_subcommand(&subs, b"bar", false), Some(0));
    }

    #[test]
    fn must_be_join() {
        assert_eq!(must_be(&v(&[b"go"])), b"go");
        assert_eq!(must_be(&v(&[b"bar", b"baz"])), b"bar, or baz");
        assert_eq!(must_be(&v(&[b"a", b"b", b"c"])), b"a, b, or c");
    }
}
