// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Variable canonicalisation / resolution — syntactic ref → canonical
//! [`Place`].
//!
//! The variable analogue of lowering's command canonicalisation: given a
//! variable reference as written (`x`, `a(k)`, `$x`, `${tok}(k)`,
//! `$state($whom)`) plus the per-frame scope context (current namespace, the
//! `global` / `variable` / `upvar` declarations in effect, active traces),
//! produce a canonical [`Place`].
//!
//! Reuses the existing resolution substrate — [`split_array_name`] /
//! [`normalise_qualified_name`] ([`crate::naming`]) and the variable-reference
//! scanner ([`crate::var_refs`]) — rather than re-deriving it.  Anything that
//! cannot be pinned down statically (dynamic variable name, computed array
//! name, `upvar` to a dynamic target) resolves to `UNKNOWN` / `dynamic` so the
//! [`overlap`](crate::place::overlap) relation stays sound (over-approximating).

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::naming::{normalise_qualified_name, split_array_name};
use crate::place::{self, Index, Place, PlaceKind};
use crate::var_refs::{VarReferenceScanner, VarScanOptions};

/// Per-frame scope context for resolving a variable reference.
///
/// Built once per proc/method/namespace scope and reused, the same way the
/// lowerer reuses its command-canonicalisation maps.
#[derive(Debug, Clone)]
pub struct ResolveContext {
    /// Current FQ namespace.
    pub namespace: String,
    /// Names under a `global` declaration.
    pub globals: HashSet<String>,
    /// Names under a `variable` declaration.
    pub ns_vars: HashSet<String>,
    /// Local alias → caller target (`""` = dynamic/unresolvable).
    pub upvar_aliases: HashMap<String, String>,
    /// Names with an active `trace`.
    pub traced: HashSet<String>,
    /// `TclOO` instance vars in scope.
    pub instance_vars: HashSet<String>,
    /// Owning class/object FQ name for `instance_vars`.
    pub instance_owner: String,
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self {
            namespace: "::".to_owned(),
            globals: HashSet::new(),
            ns_vars: HashSet::new(),
            upvar_aliases: HashMap::new(),
            traced: HashSet::new(),
            instance_vars: HashSet::new(),
            instance_owner: String::new(),
        }
    }
}

fn has_subst(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// Resolve the variables read inside a dynamic index / name *text*.
///
/// Returns the bound read-places sorted by `(ns, name)` (deterministic).
fn inner_read_places(text: &str, ctx: &ResolveContext, registry: &CommandRegistry) -> Vec<Place> {
    // Read-roles + cmd-sub recursion so `a([idx])` and `a($ns::i)` are covered.
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: false,
    });
    let names = scanner.scan_word(text, registry);
    let mut places: Vec<Place> = names.iter().map(|n| bind_scalar(n, ctx, None)).collect();
    places.sort_by(|a, b| (&a.ns, &a.name).cmp(&(&b.ns, &b.name)));
    places
}

/// Turn an array index / dict key text into an [`Index`].
fn classify_index(elem: &str, ctx: &ResolveContext, registry: &CommandRegistry) -> Index {
    if has_subst(elem) {
        Index::dynamic(inner_read_places(elem, ctx, registry))
    } else {
        Index::literal(elem)
    }
}

/// Bind a (sigil-stripped, scalar) base name to a canonical [`Place`] using the
/// scope declarations in *ctx*.  `observed` overrides the trace-derived flag
/// when `Some`.
fn bind_scalar(base: &str, ctx: &ResolveContext, observed: Option<bool>) -> Place {
    let obs = observed.unwrap_or_else(|| ctx.traced.contains(base));
    if let Some(target) = ctx.upvar_aliases.get(base) {
        // `upvar_alias` forces `dynamic` when the target is empty.
        return place::upvar_alias(base, target.clone(), false);
    }
    if ctx.instance_vars.contains(base) {
        return place::instance_var(base, &ctx.instance_owner, obs);
    }
    if ctx.globals.contains(base) {
        return place::scalar(base, "::", obs);
    }
    if ctx.ns_vars.contains(base) {
        return place::scalar(base, &ctx.namespace, obs);
    }
    if base.starts_with("::") {
        let fq = normalise_qualified_name(base);
        let (ns, tail) = match fq.rfind("::") {
            Some(i) => (&fq[..i], &fq[i + 2..]),
            None => ("", fq.as_str()),
        };
        let ns = if ns.is_empty() { "::" } else { ns };
        return place::scalar(tail, ns, obs);
    }
    place::scalar(base, place::LOCAL_NS, obs)
}

/// Resolve a variable reference *reference* (as written) to a canonical
/// [`Place`].
///
/// `whole_array` marks a reference the command touches as an entire array
/// (`array get a` / `array set a` / a bare `$a` passed to such), so it becomes
/// `ARRAY_WHOLE` rather than a scalar.
#[must_use]
pub fn resolve_place(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
) -> Place {
    if reference.is_empty() {
        return place::unknown_top();
    }

    // A target whose *name* is value-substituted (`set $X v`, `set ${tok}(k) v`)
    // denotes the variable named by the substitution's value — a place that
    // cannot be pinned down — but it *reads* the name variable(s).  Read
    // references arrive de-sigilled from the scanner, so a leading `$` here
    // always marks a write-target dynamic name.
    if reference.starts_with('$') {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    let (base, elem) = split_array_name(reference);

    if base.is_empty() || has_subst(base) {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    if let Some(elem) = elem {
        let index = classify_index(elem, ctx, registry);
        let bound = bind_scalar(base, ctx, None);
        // Carry namespace/observed/dynamic/owner from the bound scalar.
        return Place {
            kind: PlaceKind::ArrayElem,
            ns: bound.ns,
            name: bound.name,
            index: Some(index),
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
        };
    }

    if whole_array {
        let bound = bind_scalar(base, ctx, None);
        return Place {
            kind: PlaceKind::ArrayWhole,
            ns: bound.ns,
            name: bound.name,
            index: None,
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
        };
    }

    bind_scalar(base, ctx, None)
}

/// Resolve a `dict get/set $d k1 k2 …` access to a `DICT_PATH` [`Place`].
#[must_use]
pub fn resolve_dict_path(
    root_ref: &str,
    keys: &[String],
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let (base, _) = split_array_name(root_ref);
    if base.is_empty() || has_subst(base) {
        return place::unknown(inner_read_places(root_ref, ctx, registry));
    }
    let bound = bind_scalar(base, ctx, None);
    let key_indices: Vec<Index> = keys
        .iter()
        .map(|k| classify_index(k, ctx, registry))
        .collect();
    place::dict_path(bound.name, key_indices, bound.ns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::{LOCAL_NS, places_read_to_form, scalar};

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn ctx() -> ResolveContext {
        ResolveContext::default()
    }

    #[test]
    fn local_scalar_resolves_to_local_ns() {
        let p = resolve_place("x", &ctx(), false, &registry());
        assert_eq!(p, scalar("x", LOCAL_NS, false));
    }

    #[test]
    fn absolute_scalar_splits_namespace_and_tail() {
        assert_eq!(
            resolve_place("::ns::v", &ctx(), false, &registry()),
            scalar("v", "::ns", false)
        );
        assert_eq!(
            resolve_place("::g", &ctx(), false, &registry()),
            scalar("g", "::", false)
        );
    }

    #[test]
    fn global_and_variable_declarations_change_ns() {
        let mut c = ctx();
        c.globals.insert("g".to_owned());
        assert_eq!(
            resolve_place("g", &c, false, &registry()),
            scalar("g", "::", false)
        );
        let mut c = ctx();
        c.namespace = "::ns".to_owned();
        c.ns_vars.insert("v".to_owned());
        assert_eq!(
            resolve_place("v", &c, false, &registry()),
            scalar("v", "::ns", false)
        );
    }

    #[test]
    fn literal_array_element() {
        let p = resolve_place("a(k)", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayElem);
        assert_eq!(p.name, "a");
        assert_eq!(p.ns, LOCAL_NS);
        assert_eq!(p.index, Some(Index::literal("k")));
        assert!(places_read_to_form(&p).is_empty());
    }

    #[test]
    fn dynamic_array_element_records_index_reads() {
        // a($i) — element index reads the scalar `i`.
        let p = resolve_place("a($i)", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayElem);
        assert_eq!(p.name, "a");
        assert_eq!(places_read_to_form(&p), vec![scalar("i", LOCAL_NS, false)]);
    }

    #[test]
    fn dynamic_name_target_is_unknown_but_reads_the_name_var() {
        // set $X v → the *target* is UNKNOWN, but `X` is read.
        let p = resolve_place("$X", &ctx(), false, &registry());
        assert_eq!(p.kind, PlaceKind::Unknown);
        assert_eq!(places_read_to_form(&p), vec![scalar("X", LOCAL_NS, false)]);
    }

    #[test]
    fn resolved_array_elements_feed_the_overlap_precision() {
        // The end-to-end point of stages 1+2: distinct literal elements of the
        // same array resolve to non-overlapping places (the W220 FP fix),
        // while a dynamic index conservatively overlaps any sibling.
        let r = registry();
        let ak = resolve_place("a(k)", &ctx(), false, &r);
        let aj = resolve_place("a(j)", &ctx(), false, &r);
        assert!(!crate::place::overlap(&ak, &aj));
        let dyn_elem = resolve_place("a($i)", &ctx(), false, &r);
        assert!(crate::place::overlap(&dyn_elem, &ak));
    }

    #[test]
    fn upvar_alias_binds_to_caller_target() {
        let mut c = ctx();
        c.upvar_aliases
            .insert("y".to_owned(), "::caller::x".to_owned());
        let p = resolve_place("y", &c, false, &registry());
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
        assert_eq!(p.owner, "::caller::x");
        assert!(!p.dynamic);
        // A dynamic (empty-target) upvar alias is marked dynamic.
        let mut c = ctx();
        c.upvar_aliases.insert("z".to_owned(), String::new());
        let p = resolve_place("z", &c, false, &registry());
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
        assert!(p.dynamic);
    }

    #[test]
    fn whole_array_flag_yields_array_whole() {
        let p = resolve_place("a", &ctx(), true, &registry());
        assert_eq!(p.kind, PlaceKind::ArrayWhole);
        assert_eq!(p.name, "a");
    }

    #[test]
    fn traced_name_is_observed() {
        let mut c = ctx();
        c.traced.insert("x".to_owned());
        let p = resolve_place("x", &c, false, &registry());
        assert!(p.observed);
    }

    #[test]
    fn dict_path_classifies_keys() {
        let keys = vec!["a".to_owned(), "b".to_owned()];
        let p = resolve_dict_path("d", &keys, &ctx(), &registry());
        assert_eq!(p.kind, PlaceKind::DictPath);
        assert_eq!(p.name, "d");
        assert_eq!(p.keys, vec![Index::literal("a"), Index::literal("b")]);
    }
}
