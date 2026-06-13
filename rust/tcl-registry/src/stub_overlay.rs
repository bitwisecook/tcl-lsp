//! Per-document `# tcl-lsp: stub` command overlay.
//!
//! Registry-side mirror of the user-declared stub-command
//! surface that participates in analyser / compiler queries
//! (param-trait inference, role lookup, scope-alias detection,
//! barrier detection).  Stubs are parsed from comment
//! directives in the source by the analyser
//! (`tcl_compiler::analyser::utils::scan_source_for_stubs`)
//! and converted into [`StubSig`] entries via
//! [`StubOverlay::insert`].  The analyser then consults the
//! overlay alongside the global [`crate::CommandRegistry`]
//! when scanning a proc body for parameter traits.
//!
//! Mirrors the design called out in the
//! `SYNC-MAY19-stub-overlay-a` chunk-log row — the Python side
//! threads stub state through a `_stub_signatures_var`
//! `ContextVar` wrapped by a `stub_signature_scope()` context
//! manager.  Rust doesn't need the `ContextVar` dance because
//! the analyser is single-threaded and already owns the
//! per-document state; instead an `Option<StubOverlay>` lives
//! on the analyser and is rebuilt per `analyse()` call.
//!
//! ## Why keep it separate from `CommandRegistry`
//!
//! Stubs are per-document declarations: they should not
//! pollute the registry that's shared across all documents in
//! a workspace.  Mutating the global registry per analysis
//! call would also defeat the caching / interning the
//! registry relies on.  An overlay sidesteps both concerns by
//! letting callers consult the registry first, then the
//! overlay, with the overlay reset between documents.
//!
//! ## Fingerprinting
//!
//! [`StubOverlay::fingerprint`] produces a stable 64-bit hash
//! of the overlay's contents.  Downstream caches (the
//! compilation-unit cache, the interprocedural-summary cache)
//! include the fingerprint in their cache key so a per-
//! document stub change invalidates the cached entries that
//! depended on the previous stub set.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::arg_role::ArgRole;

/// One argument declared on a `# tcl-lsp: stub NAME {...}`
/// parameter list — name, role (typed), and an optional flag
/// (`?...?` markers on the source token).
///
/// Mirrors `core/analysis/semantic_model.py::StubArgDef` but
/// keyed by the typed [`ArgRole`] enum rather than a string —
/// the analyser converts the source string (`"body"` /
/// `"var"` / etc.) to [`ArgRole`] at overlay-construction
/// time so all subsequent queries are typed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StubArg {
    /// Argument name as written in the directive.
    pub name: String,
    /// Argument role.  When the source directive doesn't
    /// declare a `:role` annotation, callers should default
    /// to [`ArgRole::Value`].
    pub role: ArgRole,
    /// `true` when the source token was wrapped in `?…?`.
    pub optional: bool,
}

bitflags::bitflags! {
    /// Trailing flag set on a `# tcl-lsp: stub` directive.
    /// Packed into a single byte because the flags are
    /// orthogonal — having six independent `bool` fields on
    /// [`StubSig`] would trip `clippy::struct_excessive_bools`
    /// and lose the convenience of `contains` / `intersects`
    /// queries.  Mirrors the bit layout of
    /// `tcl_compiler::analyser::types::StubFlags` so the
    /// analyser-side conversion is a 1-for-1 bit copy.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StubSigFlags: u8 {
        /// `-barrier` — command creates a dynamic barrier
        /// (like `eval`).
        const BARRIER     = 1 << 0;
        /// `-loop` — command has a loop body.
        const LOOP        = 1 << 1;
        /// `-pure` — command is side-effect-free.
        const PURE        = 1 << 2;
        /// `-mutator` — command mutates state.
        const MUTATOR     = 1 << 3;
        /// `-unsafe` — command is unsafe.
        const UNSAFE      = 1 << 4;
        /// `-scope_alias` — command creates a scope alias
        /// (like `upvar`).
        const SCOPE_ALIAS = 1 << 5;
    }
}

/// One stub command's full signature — the overlay-side
/// shape of a `# tcl-lsp: stub` directive.  Mirrors
/// `core/analysis/semantic_model.py::StubCommandDef` but
/// drops the source-span field (the overlay only cares about
/// the semantic shape; the span lives on the original
/// `StubCommandDef` that the analyser keeps for diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StubSig {
    /// Stub command name (e.g. `my_eval`).
    pub name: String,
    /// Parameters in declaration order.
    pub args: Vec<StubArg>,
    /// Flag set — `-barrier` / `-loop` / `-pure` / `-mutator`
    /// / `-unsafe` / `-scope_alias`.
    pub flags: StubSigFlags,
}

/// Per-document overlay mapping stub-command names to their
/// [`StubSig`].
///
/// Uses a [`BTreeMap`] (rather than `HashMap`) so
/// [`Self::fingerprint`] can hash entries in a deterministic
/// order without an explicit sort step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StubOverlay {
    sigs: BTreeMap<String, StubSig>,
}

impl StubOverlay {
    /// Construct an empty overlay.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert / overwrite a stub signature.  Later inserts
    /// for the same name replace earlier ones (matches the
    /// "last directive wins" semantics that the Python
    /// `_stub_signatures_var` overlay implements implicitly
    /// through dict-update behaviour).
    pub fn insert(&mut self, sig: StubSig) {
        self.sigs.insert(sig.name.clone(), sig);
    }

    /// Look up a stub signature by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&StubSig> {
        self.sigs.get(name)
    }

    /// `true` when `name` is declared in this overlay.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.sigs.contains_key(name)
    }

    /// Iterate every stub signature in deterministic order
    /// (sorted by name).  Useful for diagnostic emitters and
    /// overlay-aware completion providers.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &StubSig)> {
        self.sigs.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of stub signatures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sigs.len()
    }

    /// `true` when no stub signatures have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sigs.is_empty()
    }

    /// Return the argument indices for `name` whose declared
    /// role matches `role`.  Indices are 0-based against the
    /// argument list (not the command word).  Returns an
    /// empty `Vec` when `name` is not declared in this
    /// overlay or when the stub has no args with the given
    /// role.  Mirrors
    /// [`crate::CommandRegistry::arg_indices_for_role`]'s
    /// shape so callers can swap between them transparently.
    ///
    /// `_args` is accepted for shape parity with the registry
    /// method (and to leave a hook for future
    /// `arg_role_resolver`-style dynamic resolvers on stubs);
    /// the current implementation doesn't need the values.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, _args: &[&str], role: ArgRole) -> Vec<usize> {
        let Some(sig) = self.sigs.get(name) else {
            return Vec::new();
        };
        sig.args
            .iter()
            .enumerate()
            .filter(|(_, a)| a.role == role)
            .map(|(i, _)| i)
            .collect()
    }

    /// Stable 64-bit fingerprint of the overlay's contents.
    /// Two overlays with the same set of [`StubSig`] entries
    /// always produce the same fingerprint; any change to
    /// the names, arg shape, or flag set changes the
    /// fingerprint.
    ///
    /// Downstream consumers (compilation-unit cache,
    /// interprocedural-summary cache) include this in their
    /// cache key so stub edits invalidate the cached entries
    /// that depended on the previous stub set.  Mirrors the
    /// `compute_stub_fingerprint()` helper called out in the
    /// `SYNC-MAY19-stub-overlay` chunk-log row.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        // `BTreeMap` iterates in sorted-by-key order, so the
        // hash is order-stable without an explicit sort.
        for (name, sig) in &self.sigs {
            name.hash(&mut hasher);
            sig.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Map a stub-comment role string (`"body"` / `"expr"` /
    /// `"var"` / `"var_read"` / `"name"` / `"pattern"` /
    /// `"channel"` / `"value"`) to the corresponding typed
    /// [`ArgRole`].  Returns [`ArgRole::Value`] for unknown
    /// strings (matches the analyser's "value is the default"
    /// behaviour from `stub_comments.py`).
    ///
    /// Centralised here so overlay builders in other crates
    /// (the analyser converts `StubArgDef` records to
    /// [`StubArg`] entries) share one canonical mapping.
    #[must_use]
    pub fn parse_role(role: &str) -> ArgRole {
        match role {
            "body" => ArgRole::Body,
            "expr" => ArgRole::Expr,
            "var" => ArgRole::VarWrite,
            "var_read" => ArgRole::VarRead,
            "name" => ArgRole::Name,
            "pattern" => ArgRole::Pattern,
            "channel" => ArgRole::Channel,
            _ => ArgRole::Value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(name: &str, args: &[(&str, ArgRole, bool)]) -> StubSig {
        StubSig {
            name: name.to_string(),
            args: args
                .iter()
                .map(|(n, r, opt)| StubArg {
                    name: (*n).to_string(),
                    role: *r,
                    optional: *opt,
                })
                .collect(),
            flags: StubSigFlags::empty(),
        }
    }

    #[test]
    fn empty_overlay_returns_none_for_lookups() {
        let o = StubOverlay::new();
        assert!(o.is_empty());
        assert_eq!(o.len(), 0);
        assert!(o.get("anything").is_none());
        assert!(!o.contains("anything"));
        assert!(o
            .arg_indices_for_role("anything", &[], ArgRole::Body)
            .is_empty());
    }

    #[test]
    fn insert_and_get_round_trip() {
        let mut o = StubOverlay::new();
        let s = sig("my_eval", &[("script", ArgRole::Body, false)]);
        o.insert(s.clone());
        assert!(o.contains("my_eval"));
        assert_eq!(o.get("my_eval"), Some(&s));
        assert_eq!(o.len(), 1);
    }

    #[test]
    fn arg_indices_for_role_matches_overlay_args() {
        let mut o = StubOverlay::new();
        o.insert(sig(
            "with_var",
            &[
                ("varName", ArgRole::VarWrite, false),
                ("value", ArgRole::Value, false),
                ("body", ArgRole::Body, false),
            ],
        ));
        assert_eq!(
            o.arg_indices_for_role("with_var", &[], ArgRole::VarWrite),
            vec![0],
        );
        assert_eq!(
            o.arg_indices_for_role("with_var", &[], ArgRole::Body),
            vec![2],
        );
        assert_eq!(
            o.arg_indices_for_role("with_var", &[], ArgRole::Expr),
            Vec::<usize>::new(),
        );
    }

    #[test]
    fn arg_indices_for_role_returns_multiple_indices_per_role() {
        let mut o = StubOverlay::new();
        o.insert(sig(
            "twin_bodies",
            &[
                ("a", ArgRole::Body, false),
                ("b", ArgRole::Body, false),
                ("c", ArgRole::Value, false),
            ],
        ));
        assert_eq!(
            o.arg_indices_for_role("twin_bodies", &[], ArgRole::Body),
            vec![0, 1],
        );
    }

    #[test]
    fn insert_overwrites_existing_signature() {
        let mut o = StubOverlay::new();
        o.insert(sig("redef", &[("a", ArgRole::Body, false)]));
        o.insert(sig("redef", &[("a", ArgRole::Expr, false)]));
        // Last insert wins.
        let s = o.get("redef").expect("redef present");
        assert_eq!(s.args[0].role, ArgRole::Expr);
        assert_eq!(o.len(), 1);
    }

    #[test]
    fn iter_returns_sorted_entries() {
        let mut o = StubOverlay::new();
        o.insert(sig("zeta", &[]));
        o.insert(sig("alpha", &[]));
        o.insert(sig("mu", &[]));
        let names: Vec<&str> = o.iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn fingerprint_is_stable_for_same_contents() {
        let mut a = StubOverlay::new();
        let mut b = StubOverlay::new();
        a.insert(sig("foo", &[("x", ArgRole::Body, false)]));
        b.insert(sig("foo", &[("x", ArgRole::Body, false)]));
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn fingerprint_is_insert_order_independent() {
        let mut a = StubOverlay::new();
        let mut b = StubOverlay::new();
        a.insert(sig("foo", &[]));
        a.insert(sig("bar", &[]));
        b.insert(sig("bar", &[]));
        b.insert(sig("foo", &[]));
        assert_eq!(
            a.fingerprint(),
            b.fingerprint(),
            "fingerprint must hash entries in name-sorted order",
        );
    }

    #[test]
    fn fingerprint_changes_when_role_changes() {
        let mut a = StubOverlay::new();
        let mut b = StubOverlay::new();
        a.insert(sig("foo", &[("x", ArgRole::Body, false)]));
        b.insert(sig("foo", &[("x", ArgRole::Expr, false)]));
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_flag_changes() {
        let base = sig("foo", &[("x", ArgRole::Body, false)]);
        let mut barrier_set = base.clone();
        barrier_set.flags |= StubSigFlags::BARRIER;
        let mut a = StubOverlay::new();
        a.insert(base);
        let mut b = StubOverlay::new();
        b.insert(barrier_set);
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn parse_role_canonicalises_known_strings() {
        assert_eq!(StubOverlay::parse_role("body"), ArgRole::Body);
        assert_eq!(StubOverlay::parse_role("expr"), ArgRole::Expr);
        assert_eq!(StubOverlay::parse_role("var"), ArgRole::VarWrite);
        assert_eq!(StubOverlay::parse_role("var_read"), ArgRole::VarRead);
        assert_eq!(StubOverlay::parse_role("name"), ArgRole::Name);
        assert_eq!(StubOverlay::parse_role("pattern"), ArgRole::Pattern);
        assert_eq!(StubOverlay::parse_role("channel"), ArgRole::Channel);
        assert_eq!(StubOverlay::parse_role("value"), ArgRole::Value);
    }

    #[test]
    fn parse_role_defaults_unknown_to_value() {
        assert_eq!(StubOverlay::parse_role(""), ArgRole::Value);
        assert_eq!(StubOverlay::parse_role("totally_made_up"), ArgRole::Value);
    }
}
