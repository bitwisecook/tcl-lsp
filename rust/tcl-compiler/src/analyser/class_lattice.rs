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

//! Object→class binding lattice + dispatch resolver — **EXPERIMENT**.
//!
//! This module is a *prototype*.  It is **not** wired into any shipping
//! diagnostic; nothing under `analyser/diagnostics/` calls into it.  It
//! exists to measure a hypothesis (see `experiments/mro_eval/RESULTS.md`
//! and `docs/design/tcloo-mro-lattice.md`):
//!
//! > Splitting TclOO tracking into two composable parts —
//! > (1) a precomputed **MRO graph** per class, and
//! > (2) an object→class **binding lattice** over the existing SSA —
//! > beats today's `aggregate_object_types` heuristic on real code, at an
//! > acceptable ⊤-collapse rate and cost.
//!
//! Half (1) already exists and is reused verbatim: [`super::mro`] +
//! [`super::class_hierarchy`] compute the TclOO method-resolution order
//! (two-pass DFS + late placement, faithful to `tclOOCall.c`) and the
//! per-`(class, method)` provider table.  This module supplies half (2)
//! and the resolver that joins them.
//!
//! ## The lattice
//!
//! [`ClassValue`] answers "what class(es) does `$o` hold *here*?":
//!
//! ```text
//!            ⊤  (abstain, tagged with WHY it collapsed)
//!           /|\
//!     Set{A,B,…}      ← control-flow JOIN of several concrete bindings
//!         |
//!     Concrete(A)     ← `set o [A new]`
//!         |
//!         ⊥  (bottom — variable never seen to hold an object)
//! ```
//!
//! The design bias is **sound-by-abstention**: any binding we cannot
//! prove collapses to [`ClassValue::Top`] carrying a [`TopReason`], and
//! the resolver then declines to make a claim.  A wrong confident answer
//! is strictly worse than an honest ⊙ abstention, so we never guess.
//!
//! ## Reuse, not reinvention
//!
//! The primary object-typing signal is read straight out of the existing
//! SSA type lattice (`FunctionUnit::types`, which already infers
//! `TclType::Object { class_name }` per SSA version — that *is* the
//! dataflow we were told to reuse).  This module only adds: the explicit
//! ⊤ taxonomy the scalar type lattice cannot express, the JOIN at merges,
//! and the resolver verdict per call site.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::analyser::class_hierarchy::{ClassHierarchy, build_class_hierarchy};
use crate::analyser::state::VarCommandSite;
use crate::analyser::types::ClassDef;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};

/// Why an object→class binding (or a dispatch) collapsed to ⊤.
///
/// This taxonomy is the point of the experiment: knowing *how often* and
/// *why* the lattice gives up tells us whether the lattice is worth the
/// machinery, and which layer to build first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopReason {
    /// The variable is reassigned along different paths / from a
    /// non-object value — its class is not stable at the call site.
    DynamicAssign,
    /// Bound from the return of a proc / command whose object class we
    /// cannot pin down (`set o [make_thing]`).
    FactoryReturn,
    /// Bound near a runtime `oo::define` that mutates the class after the
    /// fact, so the static class view is stale.
    RuntimeOoDefine,
    /// Bound from introspection (`info object class $x`, `oo::copy $x`,
    /// `self`) — the class is a runtime value, not a literal.
    Introspection,
    /// The receiver carries per-object state (`oo::objdefine $o …`),
    /// whose method table / mixins extend the class we can see.
    PerObjectMixin,
    /// Bound from a `forward` / alias whose target we do not model.
    Forward,
    /// We know the class *name* (`set o [Foo new]`) but `Foo` is defined
    /// in another file we did not index — no `ClassDef` to resolve against.
    CrossFileMiss,
    /// Anything else: a bare parameter, an unknown source, `⊥`-dispatch.
    Unknown,
}

impl TopReason {
    /// Stable snake-case tag for tables / serialisation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DynamicAssign => "dynamic-assign",
            Self::FactoryReturn => "factory-return",
            Self::RuntimeOoDefine => "runtime-oo-define",
            Self::Introspection => "introspection",
            Self::PerObjectMixin => "per-object-mixin",
            Self::Forward => "forward",
            Self::CrossFileMiss => "cross-file-miss",
            Self::Unknown => "unknown",
        }
    }

    /// Every reason, in table order.
    #[must_use]
    pub fn all() -> [TopReason; 8] {
        [
            Self::DynamicAssign,
            Self::FactoryReturn,
            Self::RuntimeOoDefine,
            Self::Introspection,
            Self::PerObjectMixin,
            Self::Forward,
            Self::CrossFileMiss,
            Self::Unknown,
        ]
    }
}

impl std::fmt::Display for TopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Object→class binding lattice value.  See the module docs for the
/// Hasse diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassValue {
    /// Never seen to hold an object.
    Bottom,
    /// Exactly one concrete class (qualified name, leading `::`).
    Concrete(String),
    /// A finite set of concrete classes — the JOIN of several concrete
    /// bindings at a control-flow merge.
    Set(BTreeSet<String>),
    /// Abstain, tagged with the collapse reason.
    Top(TopReason),
}

impl ClassValue {
    /// Lattice JOIN (least upper bound).  ⊤ absorbs; when two ⊤ values
    /// meet the left reason wins (deterministic).  Two distinct concrete
    /// classes widen to a [`ClassValue::Set`] — that is *not* ⊤: a finite
    /// class set is still resolvable.
    #[must_use]
    pub fn join(self, other: ClassValue) -> ClassValue {
        use ClassValue::{Bottom, Concrete, Set, Top};
        match (self, other) {
            (Bottom, x) | (x, Bottom) => x,
            // ⊤ absorbs; the left reason wins (the first alternative of
            // this or-pattern matches `(Top(a), Top(b))`, binding `r = a`).
            (Top(r), _) | (_, Top(r)) => Top(r),
            (Concrete(a), Concrete(b)) => {
                if a == b {
                    Concrete(a)
                } else {
                    Set([a, b].into_iter().collect())
                }
            }
            (Concrete(a), Set(mut s)) | (Set(mut s), Concrete(a)) => {
                s.insert(a);
                Set(s)
            }
            (Set(mut a), Set(b)) => {
                a.extend(b);
                Set(a)
            }
        }
    }

    /// JOIN this value with a single concrete class.
    #[must_use]
    pub fn with_class(self, class: String) -> ClassValue {
        self.join(ClassValue::Concrete(class))
    }

    /// The concrete class set, or `None` for ⊥ / ⊤.
    #[must_use]
    pub fn classes(&self) -> Option<BTreeSet<String>> {
        match self {
            ClassValue::Concrete(c) => Some([c.clone()].into_iter().collect()),
            ClassValue::Set(s) => Some(s.clone()),
            ClassValue::Bottom | ClassValue::Top(_) => None,
        }
    }
}

/// The resolver verdict for one `$obj method` call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchVerdict {
    /// The receiver class(es) are known.
    Resolved {
        /// Candidate receiver classes.
        classes: BTreeSet<String>,
        /// Classes along the MRO chain(s) that actually provide the
        /// method (empty when the method resolves only via an OO builtin
        /// / `unknown` handler / external superclass).
        providers: BTreeSet<String>,
        /// True when every candidate class can service the method
        /// (a provider, an OO builtin, an inherited `unknown` handler, or
        /// an unindexed external superclass).  `false` ⇒ this is where
        /// W308 "unknown method" would fire.
        method_known: bool,
    },
    /// Class binding unknown — abstain, tagged with why.
    Abstain(TopReason),
}

/// Which layers of the model are switched on — drives the ablation study.
#[derive(Debug, Clone, Copy)]
pub struct AblationConfig {
    /// Model the CFG-merge JOIN.  When `false`, a variable keeps only the
    /// *first* concrete class seen (the "single-class from created
    /// instance" baseline) and never widens to a set.
    pub join: bool,
    /// Include class-level mixins when computing the MRO.  When `false`,
    /// the MRO is the pure superclass chain.
    pub mixins_filters: bool,
    /// Purely informational here — the corpus harness realises "cross
    /// file" by passing a class index merged across every file instead of
    /// a per-file index.  Recorded so the emitted table can label the row.
    pub cross_file: bool,
}

impl AblationConfig {
    /// The full model: JOIN + mixins/filters + (caller-supplied) cross-file
    /// index.
    #[must_use]
    pub fn full() -> Self {
        Self {
            join: true,
            mixins_filters: true,
            cross_file: true,
        }
    }
}

/// One recorded call-site verdict.
#[derive(Debug, Clone)]
pub struct SiteReport {
    /// Receiver variable name (no `$`).
    pub var: String,
    /// Method word, when the shape was `$obj method …`.
    pub method: Option<String>,
    /// Byte offset of the dispatch head.
    pub offset: u32,
    /// The lattice value that fed the resolver.
    pub value: ClassValue,
    /// The resolver verdict.
    pub verdict: DispatchVerdict,
}

/// Aggregated instrumentation over a set of call sites.
#[derive(Debug, Clone, Default)]
pub struct DispatchStats {
    /// Every `$obj method` (or `$obj`) site considered.
    pub total_sites: usize,
    /// Sites whose receiver class binding was known (`Resolved`).
    pub resolved: usize,
    /// Sites that collapsed to ⊤ (`Abstain`).
    pub abstain: usize,
    /// Of the resolved sites, how many had the method too.
    pub method_known: usize,
    /// Of the resolved sites, how many named a class but not the method
    /// (the W308 candidate set).
    pub method_unknown: usize,
    /// ⊤ collapses broken down by reason tag.
    pub top_by_reason: BTreeMap<&'static str, usize>,
    /// Orthogonal sub-count: `unknown`-reason abstentions whose receiver
    /// variable is *never assigned* anywhere in the file — i.e. it arrives
    /// as a proc/method parameter, a global, or an `upvar`.  These are the
    /// sites only *interprocedural* object-type flow could resolve; the
    /// intraprocedural lattice can never bind them.
    pub extern_receiver_abstains: usize,
}

impl DispatchStats {
    fn record(&mut self, verdict: &DispatchVerdict) {
        self.total_sites += 1;
        match verdict {
            DispatchVerdict::Resolved { method_known, .. } => {
                self.resolved += 1;
                if *method_known {
                    self.method_known += 1;
                } else {
                    self.method_unknown += 1;
                }
            }
            DispatchVerdict::Abstain(reason) => {
                self.abstain += 1;
                *self.top_by_reason.entry(reason.as_str()).or_insert(0) += 1;
            }
        }
    }

    /// Fold another stats block into this one.
    pub fn merge(&mut self, other: &DispatchStats) {
        self.total_sites += other.total_sites;
        self.resolved += other.resolved;
        self.abstain += other.abstain;
        self.method_known += other.method_known;
        self.method_unknown += other.method_unknown;
        for (k, v) in &other.top_by_reason {
            *self.top_by_reason.entry(k).or_insert(0) += v;
        }
        self.extern_receiver_abstains += other.extern_receiver_abstains;
    }

    /// ⊤-rate: fraction of sites that abstained.  The make-or-break
    /// number.  `0.0` when there were no sites.
    #[must_use]
    pub fn top_rate(&self) -> f64 {
        ratio(self.abstain, self.total_sites)
    }
}

/// `num / den` as an `f64`, `0.0` when `den == 0`.  Counts convert via
/// `u32` so the widening to `f64` is exact (no `usize`→`f64` precision loss).
fn ratio(num: usize, den: usize) -> f64 {
    if den == 0 {
        return 0.0;
    }
    f64::from(u32::try_from(num).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(den).unwrap_or(u32::MAX))
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Classify how a variable was assigned, for ⊤-reason attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AssignKind {
    /// `[Class new|create …]` where `Class` is (`true`) / is not (`false`)
    /// in the supplied class index.
    Constructor { class: String, in_index: bool },
    /// `[info object class …]` / `[oo::copy …]` / `[self …]`.
    Introspection,
    /// `oo::objdefine …` touched the receiver.
    PerObject,
    /// Some other command substitution `[foo …]`.
    Factory,
    /// A plain literal (no `$`, no `[`).
    Literal,
    /// A bare `$other` copy.
    VarCopy,
}

/// Namespace resolution context for a file: the `namespace eval` body
/// ranges (for the enclosing namespace of a call site) and the imported
/// namespace prefixes (from `namespace import`).  This is what makes bare
/// class-name resolution **sound** — a bare `Foo new` resolves in the
/// current namespace, an imported namespace, or the global namespace, and
/// *not* to an arbitrary same-tailed class in an unrelated namespace.
#[derive(Debug, Clone, Default)]
pub struct NsContext {
    /// `(body_start, body_end, namespace_name)` for each `namespace eval`
    /// body.  Innermost (smallest span) wins for a given offset.
    pub namespaces: Vec<(u32, u32, String)>,
    /// Imported namespace prefixes, fully qualified with leading `::`
    /// (e.g. `::SpiceGenTcl` from `namespace import ::SpiceGenTcl::*`).
    pub imports: Vec<String>,
}

impl NsContext {
    /// The innermost `namespace eval` name containing `offset`, or `::`.
    fn enclosing(&self, offset: u32) -> &str {
        self.namespaces
            .iter()
            .filter(|(s, e, _)| *s <= offset && offset <= *e)
            .min_by_key(|(s, e, _)| e.saturating_sub(*s))
            .map_or("::", |(_, _, name)| name.as_str())
    }

    /// Build the context for a file from its [`AnalysisResult`]: the
    /// `namespace eval` body ranges (from the scope tree) and the imported
    /// namespace prefixes (from `namespace import`).
    ///
    /// Imports are collected file-wide — a lexically-scoped approximation;
    /// in practice `namespace import` sits at the top of a namespace body
    /// and applies throughout it.
    #[must_use]
    pub fn from_result(result: &crate::analyser::types::AnalysisResult) -> Self {
        let mut namespaces = Vec::new();
        collect_namespace_ranges(&result.global_scope, &mut namespaces);
        let mut imports: Vec<String> = result
            .namespace_imports
            .iter()
            .map(|imp| import_prefix(&imp.pattern))
            .filter(|p| !p.is_empty())
            .collect();
        imports.sort();
        imports.dedup();
        Self {
            namespaces,
            imports,
        }
    }
}

/// Recursively collect `(start, end, namespace_name)` for every
/// `namespace eval` body in the scope tree.
fn collect_namespace_ranges(
    scope: &crate::analyser::types::Scope,
    out: &mut Vec<(u32, u32, String)>,
) {
    use crate::analyser::types::ScopeKind;
    if scope.kind == ScopeKind::Namespace
        && let Some(span) = scope.body_span
    {
        out.push((span.start(), span.end(), scope.name.clone()));
    }
    for child in &scope.children {
        collect_namespace_ranges(child, out);
    }
}

/// The namespace a `namespace import` pattern draws from: everything before
/// the final `::` component (`::Ns::*` → `::Ns`, `::Ns::Foo` → `::Ns`).
fn import_prefix(pattern: &str) -> String {
    match pattern.rsplit_once("::") {
        Some((head, _)) if !head.is_empty() => head.to_string(),
        _ => String::new(),
    }
}

/// Join a namespace prefix and a (possibly-relative) name into a fully
/// qualified `::`-name — the canonical [`crate::naming`] join.  Notably an
/// *absolute* `name` keeps its own namespace (the previous local join
/// re-prefixed `::other::C` under the current namespace).
fn qualify(prefix: &str, name: &str) -> String {
    crate::naming::qualify(prefix, name)
}

/// Resolve a possibly-bare / namespace-relative class name to a qualified
/// name keyed in `index`.  Returns `(qualified_name, in_index)`.
///
/// **Sound-by-abstention** (fixing the earlier unique-tail heuristic that
/// could mis-resolve across namespaces): a bare `Foo new` at `offset` is
/// tried against, in Tcl's own resolution order,
/// 1. the exact name / `::name` (already qualified or global);
/// 2. `<enclosing namespace>::Foo` — the `namespace eval` the call sits in;
/// 3. `<imported namespace>::Foo` for each `namespace import`ed prefix;
/// 4. the global `::Foo`.
///
/// If none is in the index the name is an honest miss (→ `cross-file-miss`).
/// It is **not** matched to a same-tailed class in an unrelated namespace,
/// so the A3 cross-file run cannot manufacture a confident false resolution
/// from a namespace collision.  `offset = None` skips tier 2 (used when the
/// call site has no known offset, e.g. canonicalising a superclass name).
fn resolve_class_name<S: std::hash::BuildHasher + Clone>(
    name: &str,
    offset: Option<u32>,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
) -> (String, bool) {
    // 1. Exact / global.
    if index.contains_key(name) {
        return (name.to_string(), true);
    }
    let global = qualify("", name);
    if !name.starts_with("::") && index.contains_key(&global) {
        // fall through to try enclosing/imports first only if this is a
        // bare name; a `::`-qualified miss is final below.
    }
    // 2. Enclosing namespace (walking outward to global).
    if let Some(off) = offset {
        let mut nsname = ns.enclosing(off).to_string();
        loop {
            let cand = qualify(&nsname, name);
            if index.contains_key(&cand) {
                return (cand, true);
            }
            if nsname == "::" || nsname.is_empty() {
                break;
            }
            // Strip one namespace component and retry (Tcl resolves in the
            // current namespace then falls back toward global).
            nsname = match nsname.rsplit_once("::") {
                Some((head, _)) if !head.is_empty() => head.to_string(),
                _ => "::".to_string(),
            };
        }
    }
    // 3. Imported namespaces.
    for prefix in &ns.imports {
        let cand = qualify(prefix, name);
        if index.contains_key(&cand) {
            return (cand, true);
        }
    }
    // 4. Global.
    if index.contains_key(&global) {
        return (global, true);
    }
    // Honest miss — best-effort qualified form.
    let canon = if name.starts_with("::") {
        name.to_string()
    } else {
        global
    };
    (canon, false)
}

/// Walk every unit's IR and collect, per variable name, the kinds of RHS
/// it was assigned from.  Var-name granularity (matching the current
/// `aggregate_object_types` heuristic); the SSA type lattice supplies the
/// version-precise object signal separately.
fn collect_assign_kinds<S: std::hash::BuildHasher + Clone>(
    cu: &CompilationUnit,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
) -> HashMap<String, Vec<AssignKind>> {
    use crate::ir::Statement;
    let mut out: HashMap<String, Vec<AssignKind>> = HashMap::new();
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                // `oo::objdefine $var …` — per-object mutation of the receiver.
                if let Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } = stmt
                {
                    let canon = stmt.canonical_command_or_source();
                    if (command == "oo::objdefine" || canon == "::oo::objdefine")
                        && let Some(recv) = args.first()
                        && let Some(v) = strip_dollar(recv)
                    {
                        out.entry(v).or_default().push(AssignKind::PerObject);
                    }
                }
                let Statement::AssignValue {
                    name, value, span, ..
                } = stmt
                else {
                    continue;
                };
                let kind = classify_rhs(value.trim(), span.start(), index, ns);
                out.entry(name.clone()).or_default().push(kind);
            }
        }
    }
    out
}

/// Bare `$name` → `Some(name)`.
fn strip_dollar(text: &str) -> Option<String> {
    let t = text.trim();
    let rest = t.strip_prefix('$')?;
    let inner = rest
        .strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .unwrap_or(rest);
    if !inner.is_empty()
        && inner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
    {
        Some(inner.to_string())
    } else {
        None
    }
}

/// Classify a single RHS value string.  `offset` is the source offset of
/// the assignment, used to resolve a bare class name in its namespace.
fn classify_rhs<S: std::hash::BuildHasher + Clone>(
    value: &str,
    offset: u32,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
) -> AssignKind {
    if let Some((head, args)) = crate::value_shapes::parse_command_substitution(value) {
        // Constructor: `[Class new]` / `[Class create name]`.
        if args.first().is_some_and(|s| s == "new" || s == "create") {
            // A dynamic head — `[$cls new]` / `[[factory] new]` — is a
            // runtime class value, not a literal constructor.  Treat it as
            // introspection (the class is computed, not named).
            if head.contains('$') || head.contains('[') {
                return AssignKind::Introspection;
            }
            let (class, in_index) = resolve_class_name(&head, Some(offset), index, ns);
            return AssignKind::Constructor { class, in_index };
        }
        // Introspection heads whose result is a runtime class handle.
        // `self` is the `TclOO` introspection keyword — registry data
        // (`Traits::TCLOO_INTROSPECTION`), queried rather than spelled
        // (issue #1050). Resolved against the dialect-agnostic default
        // registry: this classifier is reached without a dialect in scope,
        // and `TclOO` introspection is core Tcl from 8.6 onwards.
        if tcl_registry::cache::default_registry().method_dispatch_keyword(&head)
            == Some(tcl_registry::MethodDispatchKind::Introspection)
        {
            return AssignKind::Introspection;
        }
        if head == "oo::copy" || head == "::oo::copy" {
            return AssignKind::Introspection;
        }
        if (head == "info" || head == "::info")
            && matches!(args.first().map(String::as_str), Some("object" | "class"))
        {
            return AssignKind::Introspection;
        }
        return AssignKind::Factory;
    }
    if let Some(_v) = strip_dollar(value) {
        return AssignKind::VarCopy;
    }
    AssignKind::Literal
}

/// Read version-precise object bindings straight out of the SSA type
/// lattice: for every SSA value typed `Object { class_name }`, JOIN its
/// class into the variable's [`ClassValue`].
fn seed_from_type_lattice<S: std::hash::BuildHasher + Clone>(
    fu: &FunctionUnit,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
    cfg: AblationConfig,
    out: &mut HashMap<String, ClassValue>,
) {
    use crate::types::TypeKind;
    for ((sym, _ver), tl) in fu.types.iter() {
        if tl.kind() != TypeKind::Known {
            continue;
        }
        if !matches!(tl.tcl_type(), Some(tcl_registry::TclType::Object)) {
            continue;
        }
        let Some(class_name) = &tl.class_name() else {
            continue;
        };
        // The type lattice's class name is already namespace-resolved by the
        // analyser, so no call-site offset is needed here.
        let (class, _in_index) = resolve_class_name(class_name, None, index, ns);
        let var = fu.ssa.var_name(*sym).to_owned();
        let entry = out.entry(var).or_insert(ClassValue::Bottom);
        if cfg.join {
            *entry = std::mem::replace(entry, ClassValue::Bottom).with_class(class);
        } else if matches!(entry, ClassValue::Bottom) {
            // Single-class baseline: keep only the first concrete class.
            *entry = ClassValue::Concrete(class);
        }
    }
}

/// Build the per-variable [`ClassValue`] map for one compilation unit.
/// Also returns the set of variable names ever assigned in the file
/// (LHS of some `set`/assign), so the caller can tell a locally-bound
/// receiver from one that arrives via parameter / global / `upvar`.
fn build_class_values<S: std::hash::BuildHasher + Clone>(
    cu: &CompilationUnit,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
    cfg: AblationConfig,
) -> (
    HashMap<String, ClassValue>,
    std::collections::HashSet<String>,
) {
    let mut values: HashMap<String, ClassValue> = HashMap::new();

    // 1. SSA type-lattice object bindings (the reused dataflow signal).
    let units = std::iter::once(&cu.top_level)
        .chain(cu.procedures.values())
        .chain(cu.methods.values());
    for fu in units {
        seed_from_type_lattice(fu, index, ns, cfg, &mut values);
    }

    // 2. IR assignment kinds — fold in constructor bindings the type
    //    lattice missed, and attribute ⊤ reasons to everything else.
    let kinds = collect_assign_kinds(cu, index, ns);
    for (var, ks) in &kinds {
        let has_constructor_in_index = ks
            .iter()
            .any(|k| matches!(k, AssignKind::Constructor { in_index: true, .. }));
        let has_constructor_miss = ks.iter().any(|k| {
            matches!(
                k,
                AssignKind::Constructor {
                    in_index: false,
                    ..
                }
            )
        });
        let has_literal = ks.iter().any(|k| matches!(k, AssignKind::Literal));
        let has_introspection = ks.iter().any(|k| matches!(k, AssignKind::Introspection));
        let has_per_object = ks.iter().any(|k| matches!(k, AssignKind::PerObject));
        let has_factory = ks.iter().any(|k| matches!(k, AssignKind::Factory));

        // Fold explicit constructor bindings into the value (the type
        // lattice already covers most, but not `[C create name]` in every
        // shape, and it never records the cross-file miss).
        for k in ks {
            if let AssignKind::Constructor { class, in_index } = k {
                let entry = values.entry(var.clone()).or_insert(ClassValue::Bottom);
                if *in_index {
                    if cfg.join {
                        *entry =
                            std::mem::replace(entry, ClassValue::Bottom).with_class(class.clone());
                    } else if matches!(entry, ClassValue::Bottom) {
                        *entry = ClassValue::Concrete(class.clone());
                    }
                }
            }
        }

        // A variable that also takes a non-object literal *and* an object
        // binding is not stable — dynamic reassignment. This dominates a
        // clean concrete binding (sound-by-abstention).
        let bound = values.get(var).cloned().unwrap_or(ClassValue::Bottom);
        let has_object_binding = bound.classes().is_some();

        let top: Option<TopReason> = if has_per_object {
            Some(TopReason::PerObjectMixin)
        } else if has_object_binding && has_literal {
            Some(TopReason::DynamicAssign)
        } else if has_object_binding {
            None // clean concrete/set binding — leave it
        } else if has_constructor_miss {
            Some(TopReason::CrossFileMiss)
        } else if has_introspection {
            Some(TopReason::Introspection)
        } else if has_factory {
            Some(TopReason::FactoryReturn)
        } else {
            None // literal / var-copy only — not object-typed, leave ⊥
        };
        // Only overwrite to ⊤ when we did not already have a *clean*
        // object binding, or the per-object / dynamic signal dominates.
        if let Some(reason) = top {
            let dominate = matches!(reason, TopReason::PerObjectMixin | TopReason::DynamicAssign);
            if dominate || !has_constructor_in_index {
                values.insert(var.clone(), ClassValue::Top(reason));
            }
        }
    }

    let assigned: std::collections::HashSet<String> = kinds.keys().cloned().collect();
    (values, assigned)
}

/// True when `method` is one of the OO built-in class / object methods
/// that every class responds to.
fn is_oo_builtin(method: &str) -> bool {
    matches!(method, "new" | "create" | "destroy" | "configure" | "cget")
}

/// Resolve one dispatch: JOIN the lattice value with the MRO table.
fn resolve<S: std::hash::BuildHasher + Clone>(
    value: &ClassValue,
    method: Option<&str>,
    hierarchy: &ClassHierarchy,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
) -> DispatchVerdict {
    let classes = match value {
        ClassValue::Top(r) => return DispatchVerdict::Abstain(*r),
        ClassValue::Bottom => return DispatchVerdict::Abstain(TopReason::Unknown),
        ClassValue::Concrete(c) => [c.clone()].into_iter().collect::<BTreeSet<_>>(),
        ClassValue::Set(s) => s.clone(),
    };
    let Some(method) = method else {
        // `$obj` with no method word — class known, nothing to look up.
        return DispatchVerdict::Resolved {
            classes,
            providers: BTreeSet::new(),
            method_known: true,
        };
    };

    let mut providers = BTreeSet::new();
    let mut method_known = true;
    for c in &classes {
        if let Some(p) = hierarchy.method_target(c, method) {
            providers.insert(p.to_string());
            continue;
        }
        // No direct provider — is the method serviceable another way?
        let serviceable = is_oo_builtin(method)
            || hierarchy.method_target(c, "unknown").is_some()
            || index.get(c).is_some_and(|cd| {
                cd.methods.contains_key("unknown")
                    // External (unindexed / base) superclass we can't see.
                    // Canonicalise first: a superclass stored bare
                    // (`Animal`) still resolves to `::Animal` in the index,
                    // and only a *genuinely* unresolvable super counts as
                    // external.
                    || cd.superclasses.iter().any(|s| {
                        let (_q, in_index) = resolve_class_name(s, None, index, ns);
                        !in_index && s != "oo::object" && s != "oo::class"
                    })
            })
            // Class not indexed at all (external) — can't disprove.
            || !index.contains_key(c);
        if !serviceable {
            method_known = false;
        }
    }
    DispatchVerdict::Resolved {
        classes,
        providers,
        method_known,
    }
}

/// Build the MRO/provider hierarchy, honouring the mixins/filters
/// ablation (strip class-level mixins when disabled).
fn hierarchy_for<S: std::hash::BuildHasher + Clone>(
    index: &HashMap<String, ClassDef, S>,
    cfg: AblationConfig,
) -> ClassHierarchy {
    if cfg.mixins_filters {
        build_class_hierarchy(index.clone())
    } else {
        let mut stripped = index.clone();
        for cd in stripped.values_mut() {
            cd.mixins.clear();
            // Both filter slots — the ablation removes *filtering*, and the
            // class object's own slot is as much of it as the instance one.
            cd.filters.clear();
            cd.class_filters.clear();
        }
        build_class_hierarchy(stripped)
    }
}

/// Run the resolver over every `$obj method` site of one compilation unit.
///
/// `index` is the class definition index to resolve against — pass the
/// file's own `all_classes` for the per-file measurement, or a
/// corpus-merged index for the cross-file ablation.  `sites` are the
/// `var_command_sites` the analyser already collected for this file.  `ns`
/// carries the file's `namespace eval` ranges + `namespace import`s so bare
/// class names resolve soundly (pass `&NsContext::default()` to disable —
/// only exact / global names then resolve).
#[must_use]
pub fn analyse_dispatch<S: std::hash::BuildHasher + Clone>(
    cu: &CompilationUnit,
    index: &HashMap<String, ClassDef, S>,
    sites: &[VarCommandSite],
    ns: &NsContext,
    cfg: &AblationConfig,
) -> (Vec<SiteReport>, DispatchStats) {
    let hierarchy = hierarchy_for(index, *cfg);
    let (values, assigned) = build_class_values(cu, index, ns, *cfg);

    let mut reports = Vec::with_capacity(sites.len());
    let mut stats = DispatchStats::default();
    for site in sites {
        let value = values
            .get(&site.var_name)
            .cloned()
            .unwrap_or(ClassValue::Bottom);
        let verdict = resolve(&value, site.method_name.as_deref(), &hierarchy, index, ns);
        stats.record(&verdict);
        // Sub-count: an `unknown` abstention on a receiver never assigned
        // in this file arrives from outside the scope (param / global /
        // upvar) — only interprocedural flow could ever bind it.
        if matches!(verdict, DispatchVerdict::Abstain(TopReason::Unknown))
            && !assigned.contains(&site.var_name)
        {
            stats.extern_receiver_abstains += 1;
        }
        reports.push(SiteReport {
            var: site.var_name.clone(),
            method: site.method_name.clone(),
            offset: site.cmd_span.start(),
            value,
            verdict,
        });
    }
    (reports, stats)
}

/// Public accessor for the per-variable [`ClassValue`] map of one file,
/// keyed by variable name.  Used by the interprocedural-ceiling experiment
/// (`examples/mro_interproc.rs`) to read a caller's argument classes so it
/// can propagate them to a callee's parameters.  Same inputs as
/// [`analyse_dispatch`].
#[must_use]
pub fn class_values<S: std::hash::BuildHasher + Clone>(
    cu: &CompilationUnit,
    index: &HashMap<String, ClassDef, S>,
    ns: &NsContext,
    cfg: &AblationConfig,
) -> HashMap<String, ClassValue> {
    build_class_values(cu, index, ns, *cfg).0
}

/// Model `TclOO` `next` / `nextto`: the *next* class down `class`'s MRO that
/// provides `method`, after `current_provider` (or from `start_from` for
/// `nextto`).  Thin wrapper over the shipping
/// [`ClassHierarchy::next_provider`] (the canonical implementation, now
/// also used by go-to-definition).
#[must_use]
pub fn next_provider(
    hierarchy: &ClassHierarchy,
    class: &str,
    method: &str,
    current_provider: &str,
    start_from: Option<&str>,
) -> Option<String> {
    hierarchy
        .next_provider(class, method, current_provider, start_from)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_bottom_identity() {
        assert_eq!(
            ClassValue::Bottom.join(ClassValue::Concrete("A".into())),
            ClassValue::Concrete("A".into())
        );
        assert_eq!(
            ClassValue::Concrete("A".into()).join(ClassValue::Bottom),
            ClassValue::Concrete("A".into())
        );
    }

    #[test]
    fn join_same_concrete_stays_concrete() {
        let v = ClassValue::Concrete("::A".into()).join(ClassValue::Concrete("::A".into()));
        assert_eq!(v, ClassValue::Concrete("::A".into()));
    }

    #[test]
    fn join_distinct_concretes_widens_to_set() {
        let v = ClassValue::Concrete("::A".into()).join(ClassValue::Concrete("::B".into()));
        match v {
            ClassValue::Set(s) => {
                assert!(s.contains("::A") && s.contains("::B"));
                assert_eq!(s.len(), 2);
            }
            other => panic!("expected set, got {other:?}"),
        }
    }

    #[test]
    fn top_absorbs() {
        let v = ClassValue::Concrete("::A".into()).join(ClassValue::Top(TopReason::DynamicAssign));
        assert_eq!(v, ClassValue::Top(TopReason::DynamicAssign));
        let v = ClassValue::Top(TopReason::FactoryReturn).join(ClassValue::Concrete("::A".into()));
        assert_eq!(v, ClassValue::Top(TopReason::FactoryReturn));
    }

    #[test]
    fn top_left_reason_wins_on_meet() {
        let v = ClassValue::Top(TopReason::Introspection).join(ClassValue::Top(TopReason::Unknown));
        assert_eq!(v, ClassValue::Top(TopReason::Introspection));
    }

    /// A context that imports `::SpiceGenTcl` file-wide (as the OSS
    /// examples do via `namespace import ::SpiceGenTcl::*`).
    fn ns_importing(prefix: &str) -> NsContext {
        NsContext {
            namespaces: Vec::new(),
            imports: vec![prefix.to_string()],
        }
    }
    /// A context whose whole file is one `namespace eval <name> { … }`.
    fn ns_enclosing(name: &str) -> NsContext {
        NsContext {
            namespaces: vec![(0, u32::MAX, name.to_string())],
            imports: Vec::new(),
        }
    }

    #[test]
    fn classify_constructor_in_index() {
        let mut index = HashMap::new();
        index.insert(
            "::Foo".to_string(),
            ClassDef {
                qualified_name: "::Foo".into(),
                name: "Foo".into(),
                ..Default::default()
            },
        );
        let k = classify_rhs("[Foo new]", 0, &index, &NsContext::default());
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::Foo".into(),
                in_index: true
            }
        );
    }

    #[test]
    fn classify_constructor_resolves_via_enclosing_namespace() {
        // Class `::SpiceGenTcl::Circuit`, constructed with the bare name
        // `Circuit` *inside its own namespace* — resolves via the enclosing
        // `namespace eval` (the dominant same-namespace shape).
        let mut index = HashMap::new();
        index.insert(
            "::SpiceGenTcl::Circuit".to_string(),
            ClassDef {
                qualified_name: "::SpiceGenTcl::Circuit".into(),
                name: "Circuit".into(),
                ..Default::default()
            },
        );
        let ns = ns_enclosing("::SpiceGenTcl");
        let k = classify_rhs("[Circuit new $name]", 10, &index, &ns);
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::SpiceGenTcl::Circuit".into(),
                in_index: true
            }
        );
    }

    #[test]
    fn classify_constructor_resolves_via_namespace_import() {
        // Bare `Circuit` at global scope, but the file did
        // `namespace import ::SpiceGenTcl::*` — resolves via the import.
        let mut index = HashMap::new();
        index.insert(
            "::SpiceGenTcl::Circuit".to_string(),
            ClassDef {
                qualified_name: "::SpiceGenTcl::Circuit".into(),
                name: "Circuit".into(),
                ..Default::default()
            },
        );
        let ns = ns_importing("::SpiceGenTcl");
        let k = classify_rhs("[Circuit new $name]", 0, &index, &ns);
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::SpiceGenTcl::Circuit".into(),
                in_index: true
            }
        );
    }

    #[test]
    fn classify_constructor_no_import_stays_unresolved() {
        // The soundness fix (Codex P2): a bare `Circuit` with NO enclosing
        // namespace and NO import must NOT be matched to `::SpiceGenTcl::Circuit`
        // just because the tail is unique — that would be a confident false
        // resolution across namespaces. Honest miss instead.
        let mut index = HashMap::new();
        index.insert(
            "::SpiceGenTcl::Circuit".to_string(),
            ClassDef {
                qualified_name: "::SpiceGenTcl::Circuit".into(),
                name: "Circuit".into(),
                ..Default::default()
            },
        );
        let k = classify_rhs("[Circuit new]", 0, &index, &NsContext::default());
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::Circuit".into(),
                in_index: false
            }
        );
    }

    #[test]
    fn classify_constructor_wrong_namespace_import_stays_unresolved() {
        // Importing an unrelated namespace must not resolve the class.
        let mut index = HashMap::new();
        index.insert(
            "::A::Circuit".to_string(),
            ClassDef {
                qualified_name: "::A::Circuit".into(),
                name: "Circuit".into(),
                ..Default::default()
            },
        );
        let ns = ns_importing("::B");
        let k = classify_rhs("[Circuit new]", 0, &index, &ns);
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::Circuit".into(),
                in_index: false
            }
        );
    }

    #[test]
    fn classify_constructor_cross_file_miss() {
        let index: HashMap<String, ClassDef> = HashMap::new();
        let k = classify_rhs("[Bar create b]", 0, &index, &NsContext::default());
        assert_eq!(
            k,
            AssignKind::Constructor {
                class: "::Bar".into(),
                in_index: false
            }
        );
    }

    #[test]
    fn classify_introspection() {
        let index: HashMap<String, ClassDef> = HashMap::new();
        let ns = NsContext::default();
        assert_eq!(
            classify_rhs("[info object class $x]", 0, &index, &ns),
            AssignKind::Introspection
        );
        assert_eq!(
            classify_rhs("[oo::copy $x]", 0, &index, &ns),
            AssignKind::Introspection
        );
        assert_eq!(
            classify_rhs("[self]", 0, &index, &ns),
            AssignKind::Introspection
        );
    }

    #[test]
    fn classify_factory_and_literal() {
        let index: HashMap<String, ClassDef> = HashMap::new();
        let ns = NsContext::default();
        assert_eq!(
            classify_rhs("[make_widget red]", 0, &index, &ns),
            AssignKind::Factory
        );
        assert_eq!(classify_rhs("hello", 0, &index, &ns), AssignKind::Literal);
        assert_eq!(classify_rhs("$other", 0, &index, &ns), AssignKind::VarCopy);
    }

    fn cls(qname: &str, supers: &[&str], methods: &[&str]) -> ClassDef {
        use crate::analyser::types::MethodDef;
        use tcl_lexer::Span;
        let mut cd = ClassDef {
            name: qname.trim_start_matches("::").to_string(),
            qualified_name: qname.to_string(),
            superclasses: supers.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        };
        for m in methods {
            cd.methods.insert(
                (*m).to_string(),
                MethodDef {
                    name: (*m).to_string(),
                    params: Vec::new(),
                    name_span: Span::new(0, 0),
                    body_span: Span::new(0, 0),
                    kind: "method".into(),
                    is_self_method: false,
                    visibility: "public".into(),
                    doc: String::new(),
                    forward_target: None,
                },
            );
        }
        cd
    }

    #[test]
    fn resolve_concrete_hits_method() {
        let mut index = HashMap::new();
        let a = cls("::A", &[], &["bark"]);
        index.insert("::A".to_string(), a);
        let h = build_class_hierarchy(index.clone());
        let v = ClassValue::Concrete("::A".into());
        match resolve(&v, Some("bark"), &h, &index, &NsContext::default()) {
            DispatchVerdict::Resolved {
                method_known,
                providers,
                ..
            } => {
                assert!(method_known);
                assert!(providers.contains("::A"));
            }
            v @ DispatchVerdict::Abstain(_) => panic!("{v:?}"),
        }
    }

    #[test]
    fn resolve_concrete_missing_method_is_resolved_but_unknown() {
        let mut index = HashMap::new();
        index.insert("::A".to_string(), cls("::A", &[], &["bark"]));
        let h = build_class_hierarchy(index.clone());
        let v = ClassValue::Concrete("::A".into());
        match resolve(&v, Some("fly"), &h, &index, &NsContext::default()) {
            DispatchVerdict::Resolved { method_known, .. } => assert!(!method_known),
            v @ DispatchVerdict::Abstain(_) => panic!("{v:?}"),
        }
    }

    #[test]
    fn resolve_top_abstains_with_reason() {
        let index: HashMap<String, ClassDef> = HashMap::new();
        let h = build_class_hierarchy(index.clone());
        let v = ClassValue::Top(TopReason::FactoryReturn);
        assert_eq!(
            resolve(&v, Some("m"), &h, &index, &NsContext::default()),
            DispatchVerdict::Abstain(TopReason::FactoryReturn)
        );
    }

    #[test]
    fn resolve_set_all_must_resolve() {
        let mut index = HashMap::new();
        index.insert("::A".to_string(), cls("::A", &[], &["m"]));
        index.insert("::B".to_string(), cls("::B", &[], &[])); // no m, no unknown, no external super
        let h = build_class_hierarchy(index.clone());
        let v = ClassValue::Set(["::A".to_string(), "::B".to_string()].into_iter().collect());
        match resolve(&v, Some("m"), &h, &index, &NsContext::default()) {
            DispatchVerdict::Resolved {
                method_known,
                classes,
                ..
            } => {
                assert_eq!(classes.len(), 2);
                assert!(!method_known, "B lacks m, so the set is not fully known");
            }
            v @ DispatchVerdict::Abstain(_) => panic!("{v:?}"),
        }
    }

    #[test]
    fn next_provider_walks_super_chain() {
        // C -> B -> A, all define `m`.  From C's own `m`, `next` should
        // reach B, then A, then exhaust.
        let mut index = HashMap::new();
        index.insert("::A".to_string(), cls("::A", &[], &["m"]));
        index.insert("::B".to_string(), cls("::B", &["::A"], &["m"]));
        index.insert("::C".to_string(), cls("::C", &["::B"], &["m"]));
        let h = build_class_hierarchy(index);
        assert_eq!(
            next_provider(&h, "::C", "m", "::C", None).as_deref(),
            Some("::B")
        );
        assert_eq!(
            next_provider(&h, "::C", "m", "::B", None).as_deref(),
            Some("::A")
        );
        assert_eq!(next_provider(&h, "::C", "m", "::A", None), None);
    }

    #[test]
    fn nextto_restarts_from_named_class() {
        // C -> B -> A, all define `m`.  `nextto ::A` from C jumps
        // straight to A regardless of the current provider.
        let mut index = HashMap::new();
        index.insert("::A".to_string(), cls("::A", &[], &["m"]));
        index.insert("::B".to_string(), cls("::B", &["::A"], &["m"]));
        index.insert("::C".to_string(), cls("::C", &["::B"], &["m"]));
        let h = build_class_hierarchy(index);
        assert_eq!(
            next_provider(&h, "::C", "m", "::C", Some("::A")).as_deref(),
            Some("::A")
        );
    }

    #[test]
    fn next_skips_classes_without_the_method() {
        // C -> B -> A; only C and A define `m`. `next` from C skips B → A.
        let mut index = HashMap::new();
        index.insert("::A".to_string(), cls("::A", &[], &["m"]));
        index.insert("::B".to_string(), cls("::B", &["::A"], &[]));
        index.insert("::C".to_string(), cls("::C", &["::B"], &["m"]));
        let h = build_class_hierarchy(index);
        assert_eq!(
            next_provider(&h, "::C", "m", "::C", None).as_deref(),
            Some("::A")
        );
    }

    #[test]
    fn stats_top_rate() {
        let mut s = DispatchStats::default();
        s.record(&DispatchVerdict::Resolved {
            classes: BTreeSet::new(),
            providers: BTreeSet::new(),
            method_known: true,
        });
        s.record(&DispatchVerdict::Abstain(TopReason::Unknown));
        assert_eq!(s.total_sites, 2);
        assert!((s.top_rate() - 0.5).abs() < 1e-9);
        assert_eq!(s.top_by_reason["unknown"], 1);
    }
}
