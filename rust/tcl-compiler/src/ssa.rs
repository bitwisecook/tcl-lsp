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

// These algorithms always use the default RandomState hasher; making
// them generic over BuildHasher adds complexity for no real benefit.
//! Static Single-Assignment (SSA) construction over CFG blocks.
//!
//! SSA is a variable-naming discipline where every variable is assigned
//! exactly once. When control flow merges (e.g. after an `if`), a
//! synthetic *phi node* is inserted to select the correct version of a
//! variable depending on which predecessor block was executed.
//!
//! This module provides:
//!
//! 1. SSA data structures: [`Phi`], [`SsaStatement`], [`SsaBlock`],
//!    [`SsaFunction`].
//! 2. **Dominator** computation: [`compute_dominators`] and
//!    `compute_idom` for immediate dominators.
//! 3. **Dominance frontier**: [`compute_dominance_frontier`].
//! 4. **Phi placement**: [`compute_phi_vars`] using the iterated
//!    dominance frontier algorithm.
//! 5. **Variable definition extraction**: [`defs_of`] extracts variable
//!    names defined by an IR statement.
//!
//! The full SSA rename pass ([`build_ssa`]) and variable-use scanner
//! ([`uses_of`]) are now implemented, completing the SSA construction
//! pipeline. The rename pass walks the dominator tree, assigns SSA
//! versions to variable definitions and uses, and fills in phi-node
//! incoming edges.

use std::collections::{BTreeSet, HashMap, HashSet};

use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use tcl_registry::CommandRegistry;

use crate::cfg::{self, BlockId};
use crate::ir::{CommandTokens, Statement, WordExpr, WordPart};
use crate::naming::normalise_var_name;
use crate::var_refs::{VarReferenceScanner, VarScanOptions, vars_in_expr};

/// SSA version number — each definition of a variable gets a unique version.
pub type Version = u32;

/// Interned identifier for an SSA variable name.
///
/// Variable names (`"x"`, `"::ns::count"`, `"arr"`, …) are interned per
/// [`SsaFunction`] into a dense `u32` index so the hot per-statement SSA and
/// dataflow maps (`defs` / `uses`, `entry_versions` / `exit_versions`, the
/// taint / SCCP / type lattices keyed by [`ValueKey`]) key on a cheap copyable
/// id instead of hashing and cloning the name string. The `u32` reflects
/// first-seen order during SSA construction, so [`Symbol`]'s `Ord` is that
/// first-seen order — a deterministic ordering; no analysis relies on
/// variable names being in lexicographic order.
///
/// Resolve a symbol back to its display name with [`SsaFunction::var_name`],
/// and a name to its symbol (when interned) with [`SsaFunction::var_symbol`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub u32);

/// Key identifying a specific SSA value: `(variable symbol, version)`.
pub type ValueKey = (Symbol, Version);

// SSA data structures

/// A phi node merging variable versions at a control-flow join.
///
/// `incoming` maps each predecessor block to the variable version that
/// flows in from that edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Phi {
    /// Variable, as an interned [`Symbol`]. Resolve the display name with
    /// [`SsaFunction::var_name`].
    pub name: Symbol,
    /// SSA version assigned by this phi.
    pub version: Version,
    /// Predecessor block → incoming version.
    pub incoming: HashMap<BlockId, Version>,
}

/// An IR statement annotated with SSA version numbers.
///
/// `uses` maps each variable read by the statement to the
/// SSA version in scope. `defs` maps each variable written
/// to its newly assigned version. Both key on the interned
/// variable [`Symbol`]; resolve a symbol's display name with
/// [`SsaFunction::var_name`].
#[derive(Debug, Clone, PartialEq)]
pub struct SsaStatement {
    /// The underlying IR statement.
    pub statement: Statement,
    /// Variables read: symbol → SSA version.
    pub uses: HashMap<Symbol, Version>,
    /// Variables written: symbol → SSA version.
    pub defs: HashMap<Symbol, Version>,
    /// The subset of [`Self::defs`] that are *synthetic* array-element
    /// writes, not writes the statement performs itself: the base refresh
    /// alongside an element write (`set arr(k) v` also defs `arr`), and the
    /// element fan of a dynamic-key / whole-array write (`set arr($i) v`
    /// defs every known `arr(*)`). Type inference **joins** across a
    /// may-def (old type ⊔ written value); write-sensitive passes (shimmer
    /// oscillation, dead-store) must not count one as a real write.
    pub may_defs: HashSet<Symbol>,
    /// The subset of [`Self::uses`] that are [`UseClass::Quoted`] — carried
    /// only by a brace-quoted word this statement does not substitute. The
    /// use is real for liveness (the text may be evaluated later) but is not
    /// a read *here*, so read-before-set must ignore it. See [`UseClass`].
    pub quoted_uses: HashSet<Symbol>,
}

/// How a statement consumes a variable reference.
///
/// Tcl substitutes `$name` in a bare or `"`-quoted word, and never in a
/// brace-quoted one: `puts {$y}` prints the two characters `$y` and reads
/// nothing (tclsh 9.0.4 / 8.6.16 agree, and `puts {$y}` succeeds with `y`
/// undefined). A braced word's contents may still be *evaluated* — by
/// `expr`, by `if`, by an `after` callback, by an unknown definer — but when
/// and in which frame is the callee's business.
///
/// The two consumers of the use set need opposite conservatism about that
/// word, which is why the use is classified rather than present-or-absent:
///
/// - liveness / dead-store (W211, W220, store elimination) must assume the
///   word **may** be evaluated, so the use must exist;
/// - read-before-set (W210) must assume it **may not** be, or may be
///   evaluated in a frame that binds the name, so it must not claim the read.
///
/// Filtering at either end breaks the other: dropping the use resurrects
/// `W211 set but never used` on `set a(k) 1; puts {$a(k)}`, and recording the
/// name as a self-initialising def deletes the feeding store outright
/// (issues #1142, #1237).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseClass {
    /// Substituted at this call site, or evaluated by the callee in this same
    /// frame (an [`ArgRole::Expr`](tcl_registry::ArgRole::Expr) /
    /// [`ArgRole::Body`](tcl_registry::ArgRole::Body) word). A genuine read,
    /// here and now.
    Substituted,
    /// Carried only inside a brace-quoted word this call site does not
    /// substitute and that nothing evaluates in this frame — see
    /// [`braced_word_class`] for the three kinds a braced word falls into and
    /// which of them land here. Kept as a use so liveness stays conservative;
    /// ignored by read-before-set.
    Quoted,
}

/// A CFG basic block in SSA form.
///
/// `entry_versions` / `exit_versions` record which SSA version
/// of each variable is live at the start and end of the block,
/// keyed by the interned variable [`Symbol`].
#[derive(Debug, Clone, PartialEq)]
pub struct SsaBlock {
    /// Block name.
    pub name: String,
    /// Phi nodes at the start of this block.
    pub phis: Vec<Phi>,
    /// SSA-annotated statements.
    pub statements: Vec<SsaStatement>,
    /// Variable versions at block entry: symbol → version.
    pub entry_versions: HashMap<Symbol, Version>,
    /// Variable versions at block exit: symbol → version.
    pub exit_versions: HashMap<Symbol, Version>,
}

/// Complete SSA representation of one Tcl procedure or top-level script.
///
/// Includes the dominator tree and dominance frontier so that
/// downstream passes (SCCP, liveness) do not need to recompute them.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaFunction {
    /// Procedure name.
    pub name: String,
    /// Entry block id.
    pub entry: BlockId,
    /// SSA blocks keyed by block id.
    pub blocks: HashMap<BlockId, SsaBlock>,
    /// Immediate dominator: block → parent (None for entry).
    pub idom: HashMap<BlockId, Option<BlockId>>,
    /// Dominance frontier: block → frontier blocks.
    pub dominance_frontier: HashMap<BlockId, Vec<BlockId>>,
    /// Dominator tree: block → children.
    pub dominator_tree: HashMap<BlockId, Vec<BlockId>>,
    /// Block-name interner copied from the source CFG, so an
    /// [`SsaFunction`]-only consumer can still resolve a [`BlockId`] to its
    /// display name. Names indexed by [`BlockId`]`.0`, in creation order.
    block_names: Vec<String>,
    /// Variable-name interner: names indexed by [`Symbol`]`.0`, in first-seen
    /// order during SSA construction. Resolves a [`Symbol`] back to the
    /// display name needed for diagnostics and serialised output.
    var_names: Vec<String>,
    /// Reverse interner index: variable name → its [`Symbol`].
    var_to_symbol: FxHashMap<String, Symbol>,
}

/// Per-[`SsaFunction`] variable-name interner.
///
/// Assigns each distinct variable name a dense [`Symbol`] in first-seen order
/// and resolves a symbol back to its name. The SSA builder threads one of
/// these while renaming and hands its tables to the finished [`SsaFunction`].
#[derive(Debug, Default, Clone)]
struct VarInterner {
    names: Vec<String>,
    to_symbol: FxHashMap<String, Symbol>,
}

impl VarInterner {
    /// Intern `name`, returning its [`Symbol`]. Assigns the next dense id the
    /// first time a name is seen and returns the existing id on re-interning.
    fn intern(&mut self, name: &str) -> Symbol {
        if let Some(&sym) = self.to_symbol.get(name) {
            return sym;
        }
        let sym = Symbol(u32::try_from(self.names.len()).expect("SSA var count fits in u32"));
        self.names.push(name.to_owned());
        self.to_symbol.insert(name.to_owned(), sym);
        sym
    }
}

impl SsaFunction {
    /// A trivial SSA shell — no blocks, no dominance information — used when
    /// the complexity guard skips the expensive SSA build for an oversized
    /// body. Downstream dataflow passes run over zero blocks (a cheap no-op),
    /// and the compilation-unit builder flags the function so per-proc
    /// diagnostic passes skip it entirely.
    #[must_use]
    pub fn trivial(name: impl Into<String>, entry: BlockId, block_names: Vec<String>) -> Self {
        Self {
            name: name.into(),
            entry,
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
            block_names,
            var_names: Vec::new(),
            var_to_symbol: FxHashMap::default(),
        }
    }

    /// Intern variable `name`, returning its [`Symbol`].
    ///
    /// Assigns the next dense id (first-seen order) the first time a name is
    /// seen and returns the existing id on re-interning, so a symbol is stable
    /// for the life of the function. Used by tests and consumers that build a
    /// fresh SSA function by hand.
    pub fn intern_var(&mut self, name: &str) -> Symbol {
        if let Some(&sym) = self.var_to_symbol.get(name) {
            return sym;
        }
        let sym = Symbol(u32::try_from(self.var_names.len()).expect("SSA var count fits in u32"));
        self.var_names.push(name.to_owned());
        self.var_to_symbol.insert(name.to_owned(), sym);
        sym
    }

    /// The display name of variable [`Symbol`] `sym`.
    ///
    /// # Panics
    /// Panics if `sym` was not produced by this function's variable interner.
    #[must_use]
    pub fn var_name(&self, sym: Symbol) -> &str {
        &self.var_names[sym.0 as usize]
    }

    /// Whether the def of `name` recorded at (`block_name`, `stmt_idx`) is a
    /// **synthetic** array-element may-def ([`SsaStatement::may_defs`]) — the
    /// base refresh of an element write, or the element fan of a dynamic-key
    /// / whole-array write — rather than a write the statement performs
    /// itself. Unused-variable / dead-store reporting skips these: the user
    /// wrote one assignment, not one per fanned symbol.
    #[must_use]
    pub fn is_synthetic_def(&self, block_name: &str, stmt_idx: i32, name: &str) -> bool {
        let Some(sym) = self.var_symbol(name) else {
            return false;
        };
        let Ok(idx) = usize::try_from(stmt_idx) else {
            return false;
        };
        self.blocks
            .values()
            .find(|b| b.name == block_name)
            .and_then(|b| b.statements.get(idx))
            .is_some_and(|st| st.may_defs.contains(&sym))
    }

    /// The [`Symbol`] a variable name resolves to, if it was interned during
    /// SSA construction. Returns `None` for a name that is not an SSA variable
    /// of this function — a lookup of such a name in any [`ValueKey`]-keyed map
    /// is a miss, which is the correct "no fact" answer.
    #[must_use]
    pub fn var_symbol(&self, name: &str) -> Option<Symbol> {
        self.var_to_symbol.get(name).copied()
    }

    /// The interned variable names, indexed by [`Symbol`]`.0` (first-seen order).
    #[must_use]
    pub fn var_names(&self) -> &[String] {
        &self.var_names
    }

    /// The display name of block `id`.
    ///
    /// # Panics
    /// Panics if `id` is outside this function's interner range.
    #[must_use]
    pub fn block_name(&self, id: BlockId) -> &str {
        &self.block_names[id.0 as usize]
    }

    /// The interned block names, indexed by [`BlockId`]`.0`.
    #[must_use]
    pub fn block_names(&self) -> &[String] {
        &self.block_names
    }

    /// The [`BlockId`] a block name resolves to, if interned. Linear scan over
    /// the (per-function, small) name table.
    #[must_use]
    pub fn block_id(&self, name: &str) -> Option<BlockId> {
        self.block_names
            .iter()
            .position(|n| n == name)
            .map(|i| BlockId(u32::try_from(i).expect("block count fits in u32")))
    }

    /// Build the O(1)-query dominance index for this function
    /// ([`DominatorIntervals`]).  One O(V) walk; build it once per function
    /// and reuse it for every query.
    #[must_use]
    pub fn dominator_intervals(&self) -> DominatorIntervals {
        DominatorIntervals::build(self)
    }
}

/// Dominator-tree DFS interval numbering: answers `dominates(a, b)` in O(1).
///
/// The straightforward answer walks `b`'s immediate-dominator chain looking
/// for `a`, which is O(depth) with a hash lookup per hop — and on a flat
/// N-branch dispatch chain the idom chain *is* the whole function, so a
/// per-block-pair loop over it is O(V²) (issue #1250).  A pre-order DFS of
/// the dominator tree instead assigns every block a half-open `[enter, exit)`
/// interval that contains exactly its dominator-tree subtree, and `a`
/// dominates `b` iff `b`'s interval nests inside `a`'s.
///
/// Blocks outside the dominator forest (unreachable, hence absent from
/// `idom`) have no interval and dominate nothing but themselves — the same
/// answer the chain walk gives, since it runs out of parents.
#[derive(Debug, Clone, Default)]
pub struct DominatorIntervals {
    /// `(enter, exit)` per block, indexed by [`BlockId`]`.0`.  `None` for a
    /// block the DFS never reached.
    intervals: Vec<Option<(u32, u32)>>,
}

impl DominatorIntervals {
    /// Number `ssa`'s dominator forest.
    ///
    /// Children are derived from `idom` rather than read off
    /// [`SsaFunction::dominator_tree`], so the index is correct for any
    /// function whose `idom` is populated (including hand-built test
    /// fixtures that never ran the tree builder).
    ///
    /// The DFS starts at the entry block and then picks up any other root
    /// (a block whose `idom` is `None`), so a forest with more than one root
    /// is numbered in full rather than silently losing a component.
    #[must_use]
    fn build(ssa: &SsaFunction) -> Self {
        let slots = ssa
            .idom
            .iter()
            .flat_map(|(id, parent)| [Some(*id), *parent])
            .flatten()
            .map(|b| b.0 as usize + 1)
            .chain(std::iter::once(ssa.block_names.len()))
            .max()
            .unwrap_or(0);
        let mut intervals = vec![None; slots];
        if ssa.idom.is_empty() {
            return Self { intervals };
        }
        let mut children: Vec<Vec<BlockId>> = vec![Vec::new(); slots];
        let mut roots: Vec<BlockId> = Vec::new();
        for (id, parent) in &ssa.idom {
            match parent {
                Some(p) => children[p.0 as usize].push(*id),
                None if *id != ssa.entry => roots.push(*id),
                None => {}
            }
        }
        for kids in &mut children {
            kids.sort_unstable();
        }
        roots.sort_unstable();
        if ssa.idom.contains_key(&ssa.entry) {
            roots.insert(0, ssa.entry);
        }

        let mut counter: u32 = 0;
        // Explicit stack of `(block, next child index)` — an iterative
        // pre-order DFS, so a deep dominator chain cannot blow the stack.
        let mut stack: Vec<(BlockId, usize)> = Vec::new();
        for root in roots {
            if intervals[root.0 as usize].is_some() {
                continue;
            }
            intervals[root.0 as usize] = Some((counter, counter));
            counter += 1;
            stack.push((root, 0));
            while let Some((node, child_idx)) = stack.pop() {
                let kids = &children[node.0 as usize];
                if child_idx < kids.len() {
                    stack.push((node, child_idx + 1));
                    let child = kids[child_idx];
                    if intervals[child.0 as usize].is_none() {
                        intervals[child.0 as usize] = Some((counter, counter));
                        counter += 1;
                        stack.push((child, 0));
                    }
                } else if let Some((_, exit)) = intervals[node.0 as usize].as_mut() {
                    *exit = counter;
                }
            }
        }
        Self { intervals }
    }

    /// True when `ancestor` dominates `node` (a block dominates itself).
    #[must_use]
    pub fn dominates(&self, ancestor: BlockId, node: BlockId) -> bool {
        if ancestor == node {
            return true;
        }
        let (Some(Some((a_enter, a_exit))), Some(Some((n_enter, n_exit)))) = (
            self.intervals.get(ancestor.0 as usize),
            self.intervals.get(node.0 as usize),
        ) else {
            return false;
        };
        *a_enter <= *n_enter && *n_exit <= *a_exit
    }
}

/// CFG block ceiling above which deep analysis (SSA / dataflow) is skipped.
///
/// A pathologically large body — almost always machine-generated, e.g. a
/// tens-of-thousands-of-block nested-if dispatch tree — would cost seconds of
/// SSA + SCCP / type / taint / liveness dataflow for near-zero useful
/// findings, and an unbounded analysis also lets interprocedural summaries
/// grow over-optimistic on adversarial input.
pub const COMPLEXITY_GUARD_BLOCKS: usize = 20_000;

/// Body-size (bytes) ceiling for the deep-analysis complexity guard. A flat
/// generated command list is block-light — so [`COMPLEXITY_GUARD_BLOCKS`]
/// never fires — yet byte-huge, and the O(blocks·vars) SSA walk plus
/// SCCP / taint / liveness still costs seconds. The ceiling is 256 KiB.
pub const DEEP_ANALYSIS_BODY_BYTES: usize = 262_144;

/// True when `func` is large enough that deep analysis (SSA / dataflow) is
/// skipped. This is the block-count half of the guard; the body-byte half is
/// applied by the compilation-unit builder, which has the body span.
#[must_use]
pub fn is_complexity_guarded(func: &cfg::Function) -> bool {
    func.blocks.len() > COMPLEXITY_GUARD_BLOCKS
}

// Variable definition extraction

/// Extract variable names defined by an IR statement.
///
/// Handles assignments (`set`, `incr`), call defs, `trace add variable`
/// (via registry roles when `registry` is supplied), and `dict for`/
/// `dict map` barriers.
///
/// Pass `Some(&CommandRegistry)` when available so barrier defs route
/// through the registry's `ArgRole::VarWrite` query; pass
/// `None` for the legacy string-match path used by the unit-test
/// helpers.
#[must_use]
pub fn defs_of(stmt: &Statement) -> Vec<String> {
    defs_of_with_registry(stmt, None)
}

/// Whether a barrier's command is an ensemble loop subcommand whose first
/// argument is the iteration-variable list (`::tcl::dict::for` / `::map`,
/// `::tcl::array::for`). Resolved from the registry's `loop_list_header`
/// flag on the subcommand: the barrier name's last segment is the
/// subcommand, the one before it the base command (`… ::tcl::dict::for` →
/// `dict for`). Callers without a registry (test helpers) fall back to the
/// legacy suffix heuristic, mirroring the trace fallback below.
fn barrier_is_loop_list_header(command: &str, registry: Option<&CommandRegistry>) -> bool {
    let Some(registry) = registry else {
        return command.ends_with("::for") || command.ends_with("::map");
    };
    let segments = crate::naming::qualifier_segments(command.as_bytes());
    let [.., base, sub] = segments.as_slice() else {
        return false;
    };
    let (Ok(base), Ok(sub)) = (std::str::from_utf8(base), std::str::from_utf8(sub)) else {
        return false;
    };
    registry
        .get(base)
        .and_then(|spec| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.loop_list_header)
}

/// Whether an IR assignment target `name` (as written) is a *dynamic* write
/// target — its variable name is computed at runtime from a substitution
/// (`set $p …`, `set ${tok} …`, `set a$b(k) …`).  Such a target is opaque
/// (no static def) and *reads* the substituted name-bearing variable(s).
///
/// A leading `$`
/// marks a substituted write-target name, and a substitution in the array
/// *base* (`a$b(k)`) is dynamic too.  A bare array element (`arr($i)`) keeps
/// its static base `arr` (only the index is dynamic), so it is **not** a
/// dynamic target here.
///
/// `braced_literal` is the statement's own `name_braced` flag: a brace-quoted
/// word (`set {$n} 1`) substitutes nothing, so its `$` is part of a perfectly
/// static name and the target is **not** dynamic (issue #1078).  Reading the
/// name text alone called it dynamic, withheld the def, and recorded a read of
/// `n` — a `W210 Variable 'n' is read before it is set` on code that never
/// mentions `n`.
#[must_use]
pub fn is_dynamic_write_target(name: &str, braced_literal: bool) -> bool {
    if braced_literal {
        return false;
    }
    if name.starts_with('$') {
        return true;
    }
    let base = match name.find('(') {
        Some(i) => &name[..i],
        None => name,
    };
    base.is_empty() || base.contains('$') || base.contains('[')
}

/// Whether a loop-variable-list word is **static** — its source performs no
/// substitution, so the word's text *is* the Tcl list of names the loop binds.
///
/// Dynamism is a property of the source word's *kind*, not of the bytes in its
/// value — the same rule the neighbouring `VarWrite` path applies via
/// [`CommandTokens::arg_is_braced_literal`].  A brace-quoted word substitutes
/// nothing, so `foreach {{$x}} {*}$spec {puts ${$x}}` binds the perfectly legal
/// variable named `$x`; testing the reconstructed word text for `$` / `[`
/// dropped that name from the barrier's def set and reported a phantom
/// `W210 Variable '$x' is read before it is set` on code `tclsh` runs happily
/// (PR #1481 review of issue #1380).  A word that really does substitute
/// (`foreach $names …`, `foreach [names] …`, `foreach {*}$spec …`) names
/// nothing statically and contributes no def.
///
/// `text` is only consulted when the statement carries no structured word for
/// this argument — a synthetic or lossy token snapshot, where the kind is
/// unknown and the conservative text test is the best available answer.
fn loop_var_list_word_is_static(word: Option<&WordExpr>, text: &str) -> bool {
    match word {
        Some(WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }) => true,
        Some(
            WordExpr::Variable { .. }
            | WordExpr::CommandSubstitution { .. }
            | WordExpr::Expand { .. },
        ) => false,
        // A compound word is static exactly when every part is text: a quoted
        // (`"a b"`) or backslash-escaped (`a\ b`) var-list substitutes nothing,
        // while any `$` / `[` part makes the whole word dynamic.
        Some(WordExpr::Template { parts, .. }) => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        Some(WordExpr::Opaque { .. }) | None => !text.contains('$') && !text.contains('['),
    }
}

/// The defs a `Statement::Barrier` contributes under the registry — its
/// loop-variable lists plus its `ArgRole::VarWrite` targets.
///
/// Skips *scope-alias* commands (`global`, `variable`, `upvar`) whose variable
/// bindings are tracked separately by the `var_scoping` pass; without the skip
/// we'd produce partial defs for the vararg forms (`global x y z` would mark
/// only `x`).  The discriminator is `CREATES_SCOPE_ALIAS`, *not*
/// `CREATES_DYNAMIC_BARRIER`: `trace` is a dynamic barrier but not a scope
/// alias, so `trace add variable x` must still surface its `VarWrite` def.
fn registry_barrier_defs(
    reg: &CommandRegistry,
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
) -> Vec<String> {
    use tcl_registry::{ArgRole, Traits};

    let spec = reg.get(command);
    if spec.is_some_and(|spec| spec.traits.contains(Traits::CREATES_SCOPE_ALIAS)) {
        return Vec::new();
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // Loop variables (`ArgRole::LoopVarList` — `foreach` / `lmap` var-lists,
    // `dict for` / `array for` key-value pairs).  A structured loop that stays
    // a barrier — `{*}` expansion among its words, a dynamic body — still
    // binds every variable its *literal* var-list words name, and its body is
    // scanned for reads in this same frame, so without these defs a body read
    // of a loop variable reported W210 "read before it is set" (issue #1380).
    // A dynamic var-list word (`foreach {*}$pairs …`) names nothing statically
    // and contributes no def — see [`loop_var_list_word_is_static`] for how
    // that verdict is reached.
    //
    // A **conditionally-bound** layout is excluded: `dict update dictVar key
    // varName … body` binds `varName` only when the key is present at runtime,
    // so it is not a definite def and the key-aware read-before-set harvester
    // owns it instead (`RepeatedArgLayout::conditional_binding`, issue #1278).
    let loop_vars_conditional = |repeated: &[tcl_registry::RepeatedArgLayout]| {
        repeated
            .iter()
            .any(|layout| layout.conditional_binding && layout.role == ArgRole::LoopVarList)
    };
    let conditional_loop_vars = spec.is_some_and(|spec| {
        args.first()
            .and_then(|first| spec.resolve_subcommand(first))
            .map_or_else(
                || loop_vars_conditional(spec.repeated_args),
                |sub| loop_vars_conditional(sub.repeated_args),
            )
    });
    let mut defs: Vec<String> = if conditional_loop_vars {
        Vec::new()
    } else {
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::LoopVarList)
            .into_iter()
            .filter_map(|idx| args.get(idx).map(|word| (idx, word)))
            .filter(|(idx, word)| {
                loop_var_list_word_is_static(tokens.and_then(|t| t.words().get(idx + 1)), word)
            })
            .filter_map(|(_, word)| tcl_syntax::list::split_list(word).ok())
            .flatten()
            .map(std::borrow::Cow::into_owned)
            .filter(|name| !name.is_empty())
            .collect()
    };
    defs.extend(
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::VarWrite)
            .into_iter()
            .filter_map(|idx| {
                // A brace-quoted name word is a literal name, `$` and all
                // (issue #1078).
                let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
                args.get(idx).map(|s| {
                    let name = crate::naming::element_var_name_braced(s, braced);
                    if reg.option_variable_scope(
                        command,
                        &arg_strs,
                        idx,
                        reg.own_availability_mask(),
                    ) == Some(tcl_registry::VariableScope::Global)
                        && !name.starts_with("::")
                    {
                        format!("::{name}")
                    } else {
                        name.to_owned()
                    }
                })
            })
            .filter(|n| !n.is_empty()),
    );
    defs
}

/// Registry-aware `defs_of`.
///
/// Barrier defs route through
/// `ArgRole::VarWrite` instead of a hardcoded string-match.  The
/// registry-aware path also covers `::trace` and any future trace
/// alias spellings without code edits, plus skips
/// `creates_dynamic_barrier` specs (`global` / `variable` /
/// `upvar` are handled by `var_scoping`, not by SSA's per-arg
/// `VarWrite` walk).
#[must_use]
pub fn defs_of_with_registry(stmt: &Statement, registry: Option<&CommandRegistry>) -> Vec<String> {
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            // A write-target whose *name* is value-substituted (`set $p …`,
            // `set ${tok}(k) …`) denotes the variable named by the
            // substitution's value — a place that cannot be pinned down, so it
            // is **not** a static def of the name-bearing variable.  `uses_of`
            // records the name read separately.  A brace-quoted target
            // (`set {$n} 1`) substitutes nothing — it is a static def of the
            // variable literally named `$n` (issue #1078).
            if is_dynamic_write_target(name, *name_braced) {
                return Vec::new();
            }
            // A constant-keyed array element defs its own variable
            // (`arr(k)`); a dynamic key stays on the base (the rename walk
            // fans the def over the array's known elements).
            vec![crate::naming::element_var_name_braced(name, *name_braced).to_owned()]
        }
        Statement::Call { defs, .. } if !defs.is_empty() => defs.clone(),
        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            // Loop-header barriers (`::tcl::dict::for`/`::map`, `::tcl::array::for`
            // — any ensemble subcommand the registry marks `loop_list_header`):
            // args[0] is the iteration-variable list, so extract the names.
            // Resolved via the registry rather than a name-suffix match — a
            // user proc named `my::for` must NOT have its first argument
            // misread as loop variables.
            if !args.is_empty() && barrier_is_loop_list_header(command, registry) {
                // A loop var-list is a Tcl list, not whitespace-separated
                // source text.  Decode grouping and backslash substitutions so
                // `{one name}`, `"two name"`, and `three\ name` each define
                // one variable.  A malformed var-list makes the command fail
                // before its body runs, so it contributes no reaching defs.
                return tcl_syntax::list::split_list(&args[0])
                    .map(|names| {
                        names
                            .into_iter()
                            .map(std::borrow::Cow::into_owned)
                            .collect()
                    })
                    .unwrap_or_default();
            }
            if let Some(reg) = registry {
                let defs = registry_barrier_defs(reg, command, args, tokens.as_ref());
                if !defs.is_empty() {
                    return defs;
                }
            }
            // Legacy string-match fallback for callers without a
            // registry (test helpers).
            if command == "trace" && args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
                return vec![crate::naming::element_var_name(&args[2]).to_owned()];
            }
            Vec::new()
        }
        // An opaque (glob/regexp/fall-through) `switch` definitely-defines a
        // variable only when *every reaching* path assigns it: there must be a
        // `default` arm (covering the no-match path) and the variable must be
        // *must-defined* in the default body and in every arm that has a body
        // (fall-through arms with no body delegate to a later body, already
        // covered). An arm that *cannot complete normally* (always
        // `return`s/`error`s/`tailcall`s/…) never reaches the code after the
        // switch, so it is excluded from the intersection (FP-RBS-14). This
        // reproduces the phi the expanded arm blocks would build, conservatively
        // — a variable only *conditionally* assigned inside an arm is not
        // claimed, so we never hide a genuine read-before-set. The expanded
        // (exact, non-fall-through) switch never reaches here; its arm defs come
        // from the real per-block statements. Shares the "flow facts"
        // definite-assignment helpers with the CFG builder (`cfg_builder`), so
        // both layers agree on "cannot complete normally".
        Statement::Switch {
            default_body: Some(_),
            ..
        } => crate::cfg_builder::switch_must_defines(stmt)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

// Dominator algorithms

/// Compute the dominator sets for all blocks in a CFG function.
///
/// Uses the iterative dataflow algorithm. Returns a map from block
/// name to the set of blocks that dominate it.
///
/// The fixpoint visits blocks in **reverse postorder** so that each
/// block is processed after the predecessors it depends on, which is
/// what makes the iteration converge in a small constant number of
/// passes for a reducible CFG instead of one pass per block.  Driving
/// the fixpoint off the reachable-block *set* (arbitrary hash order)
/// instead made convergence O(blocks) passes — pathological on a proc
/// body that lowers to a long chain of branches (e.g. a 700-way
/// `if {$x == N} {...}` dispatch), where it turned an O(N²) job into
/// O(N³) and stalled the analyser.
#[must_use]
pub fn compute_dominators(func: &cfg::Function) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut dom: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        if !reachable.contains(id) || *id == func.entry {
            dom.insert(*id, HashSet::from([*id]));
        } else {
            dom.insert(*id, reachable.clone());
        }
    }

    // Reverse postorder over blocks reachable from the entry — this is
    // exactly the reachable set, ordered so predecessors precede the
    // blocks that depend on them.
    let rpo = func.reverse_postorder();
    let preds = func.predecessors();
    let mut changed = true;
    while changed {
        changed = false;
        for id in &rpo {
            if *id == func.entry {
                continue;
            }
            let bn_preds: Vec<BlockId> = preds
                .get(id)
                .map(|p| {
                    p.iter()
                        .copied()
                        .filter(|p| reachable.contains(p))
                        .collect()
                })
                .unwrap_or_default();

            let new_dom = if bn_preds.is_empty() {
                HashSet::from([*id])
            } else {
                let mut inter = dom[&bn_preds[0]].clone();
                for p in &bn_preds[1..] {
                    inter = inter.intersection(&dom[p]).copied().collect();
                }
                inter.insert(*id);
                inter
            };

            if new_dom != dom[id] {
                dom.insert(*id, new_dom);
                changed = true;
            }
        }
    }
    dom
}

/// Compute immediate dominators directly via the Cooper-Harvey-Kennedy
/// "A Simple, Fast Dominance Algorithm".
///
/// Returns the same map shape as [`compute_idom`] (entry and
/// unreachable blocks map to `None`, every other reachable block to
/// `Some(parent)`), but **without** materialising the full dominator
/// *sets*: it works on reverse-postorder block indices and a single
/// `idom` pointer per block, so it is O(N·D) time and O(N) memory
/// rather than the O(N²) memory / O(N³) worst-case time of the
/// set-based [`compute_dominators`] + [`compute_idom`] pair.  This is
/// what keeps `build_ssa` bounded on pathologically large functions
/// (a single multi-thousand-branch generated proc would otherwise
/// exhaust memory building the dominator sets).
#[must_use]
pub(crate) fn compute_idom_fast(func: &cfg::Function) -> HashMap<BlockId, Option<BlockId>> {
    const UNDEF: usize = usize::MAX;

    // Shared iterative RPO — see `cfg::Function::reverse_postorder`.
    let rpo = func.reverse_postorder();
    let mut out: HashMap<BlockId, Option<BlockId>> = HashMap::new();
    for id in func.blocks.keys() {
        out.insert(*id, None);
    }
    if rpo.is_empty() {
        return out;
    }
    // Map block id → reverse-postorder index (entry == 0).
    let mut rpo_index: FxHashMap<BlockId, usize> =
        FxHashMap::with_capacity_and_hasher(rpo.len(), FxBuildHasher);
    for (i, n) in rpo.iter().enumerate() {
        rpo_index.insert(*n, i);
    }
    let preds = func.predecessors();

    let mut idom: Vec<usize> = vec![UNDEF; rpo.len()];
    idom[0] = 0; // entry is its own dominator (sentinel for the walk).

    // Walk up the idom tree from both nodes until they meet, using
    // RPO indices (a dominator always has a strictly smaller index).
    let intersect = |mut a: usize, mut b: usize, idom: &[usize]| -> usize {
        while a != b {
            while a > b {
                a = idom[a];
            }
            while b > a {
                b = idom[b];
            }
        }
        a
    };

    let mut changed = true;
    while changed {
        changed = false;
        // Skip the entry (index 0); process the rest in RPO order so
        // each block sees already-processed predecessors.
        for i in 1..rpo.len() {
            let mut new_idom = UNDEF;
            if let Some(ps) = preds.get(&rpo[i]) {
                for p in ps {
                    let Some(&pi) = rpo_index.get(p) else {
                        continue; // unreachable predecessor
                    };
                    if idom[pi] == UNDEF {
                        continue; // not processed yet this pass
                    }
                    new_idom = if new_idom == UNDEF {
                        pi
                    } else {
                        intersect(pi, new_idom, &idom)
                    };
                }
            }
            if new_idom != UNDEF && idom[i] != new_idom {
                idom[i] = new_idom;
                changed = true;
            }
        }
    }

    for (i, id) in rpo.iter().enumerate() {
        if i == 0 || idom[i] == UNDEF {
            out.insert(*id, None);
        } else {
            out.insert(*id, Some(rpo[idom[i]]));
        }
    }
    out
}

/// Compute immediate dominators from dominator sets.
///
/// The immediate dominator of a block is the closest strict dominator
/// (the one with the largest dominator set).
///
/// Retained as the reference set-based implementation that
/// [`compute_idom_fast`] (the production path) is cross-validated
/// against; see the `compute_idom_fast_matches_reference` test. Only
/// the fast path is used in production, so this is compiled for tests.
#[cfg(test)]
#[must_use]
pub(crate) fn compute_idom(
    func: &cfg::Function,
    dom: &HashMap<BlockId, HashSet<BlockId>>,
) -> HashMap<BlockId, Option<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut idom: HashMap<BlockId, Option<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        idom.insert(*id, None);
    }

    for id in &reachable {
        if *id == func.entry {
            continue;
        }
        let strict: HashSet<BlockId> = dom[id].iter().copied().filter(|d| d != id).collect();
        if strict.is_empty() {
            continue;
        }
        // The idom is the strict dominator with the largest dom set.
        let best = *strict.iter().max_by_key(|d| dom[*d].len()).unwrap();
        idom.insert(*id, Some(best));
    }
    idom
}

/// Compute the dominance frontier for each block.
///
/// A block `b` is in the dominance frontier of block `a` if `a`
/// dominates a predecessor of `b` but does not strictly dominate `b`.
#[must_use]
pub(crate) fn compute_dominance_frontier(
    func: &cfg::Function,
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let preds = func.predecessors();
    let mut df: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        df.insert(*id, HashSet::new());
    }

    for id in &reachable {
        let bn_preds: Vec<BlockId> = preds
            .get(id)
            .map(|p| {
                p.iter()
                    .copied()
                    .filter(|p| reachable.contains(p))
                    .collect()
            })
            .unwrap_or_default();

        if bn_preds.len() < 2 {
            continue;
        }

        for p in &bn_preds {
            let mut runner = Some(*p);
            while let Some(r) = runner {
                if idom.get(id).and_then(|i| i.as_ref()) == Some(&r) {
                    break;
                }
                df.entry(r).or_default().insert(*id);
                runner = idom.get(&r).copied().flatten();
            }
        }
    }
    df
}

/// Build the dominator tree from immediate dominators.
///
/// Returns a map from each block to its children in the dominator tree.
/// Children are sorted by [`BlockId`] (block-creation order) for
/// deterministic traversal.
#[must_use]
pub(crate) fn build_dom_tree(
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for id in idom.keys() {
        tree.entry(*id).or_default();
    }
    for (id, parent) in idom {
        if let Some(p) = parent {
            tree.entry(*p).or_default().push(*id);
        }
    }
    for children in tree.values_mut() {
        children.sort_unstable();
    }
    tree
}

/// Compute which variables need phi nodes in each block.
///
/// Uses the iterated dominance frontier algorithm: for each variable,
/// starting from blocks where it is defined, propagate phi nodes to
/// the dominance frontier until convergence.
#[must_use]
pub(crate) fn compute_phi_vars(
    func: &cfg::Function,
    df: &HashMap<BlockId, HashSet<BlockId>>,
    registry: &CommandRegistry,
    elems: &ArrayElems,
) -> HashMap<BlockId, HashSet<String>> {
    let reachable = func.reachable_blocks();
    let (nonlocal_names, all_defsites) =
        nonlocal_names_and_defsites(func, &reachable, registry, elems);

    // Semi-pruned SSA (Briggs et al. 1998): place phis only for *non-local*
    // (upward-exposed-use) names. A phi for a purely-local name has no reader,
    // so dropping it removes only dead phis (~40% of minimal-SSA phis) without
    // changing any use/value/liveness/diagnostic result.
    let mut phi: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for id in func.blocks.keys() {
        phi.insert(*id, HashSet::new());
    }

    for (var, sites) in &all_defsites {
        if !nonlocal_names.contains(var) {
            continue;
        }
        let mut work: Vec<BlockId> = sites.iter().copied().collect();
        work.sort_unstable();
        let mut has_phi: FxHashSet<BlockId> = FxHashSet::default();

        while let Some(nb) = work.pop() {
            for fb in df.get(&nb).into_iter().flatten() {
                if has_phi.insert(*fb) {
                    phi.entry(*fb).or_default().insert(var.clone());
                    if !sites.contains(fb) {
                        work.push(*fb);
                    }
                }
            }
        }
    }
    phi
}

/// Semi-pruned SSA support: the *non-local* names (upward-exposed uses) and
/// every variable's def-site blocks, in one pass.
///
/// A variable is *non-local* if some block reads it before (re)defining it in
/// that block — the read could observe a value flowing in from a predecessor,
/// so a phi at a merge is meaningful. A name only ever read after its in-block
/// definition (or never read) needs no phi. `defsites` is unfiltered (all
/// defined names); the caller restricts phi placement to the non-local names.
/// The constant-keyed elements of each array base referenced anywhere in a
/// function: `"arr"` -> `{"arr(a)", "arr(b)"}`. Feeds [`expand_defs`]'s
/// fan-out — a dynamic-key or whole-array write may hit any of these.
pub(crate) type ArrayElems = FxHashMap<String, BTreeSet<String>>;

/// Collect every constant-keyed array element defined or read in `func`.
fn collect_array_elems(func: &cfg::Function, registry: &CommandRegistry) -> ArrayElems {
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: true,
    });
    let mut elems: ArrayElems = ArrayElems::default();
    let note = |name: &str, elems: &mut ArrayElems| {
        if let Some(open) = name.find('(') {
            elems
                .entry(name[..open].to_owned())
                .or_default()
                .insert(name.to_owned());
        }
    };
    for block in func.blocks.values() {
        for stmt in &block.statements {
            for d in defs_of_with_registry(stmt, Some(registry)) {
                note(&d, &mut elems);
            }
            for u in uses_of(stmt, &mut scanner, registry) {
                note(&u, &mut elems);
            }
        }
        match &block.terminator {
            Some(cfg::Terminator::Branch { condition, .. }) => {
                for u in condition.vars_element_qualified() {
                    note(&u, &mut elems);
                }
            }
            Some(cfg::Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value {
                    for u in scanner.scan_word(v, registry) {
                        note(&u, &mut elems);
                    }
                }
                if let Some(e) = expr {
                    for u in e.vars_element_qualified() {
                        note(&u, &mut elems);
                    }
                }
            }
            _ => {}
        }
    }
    elems
}

/// Expand a statement's direct def names with the array-element fan-out:
///
/// - an element def (`arr(k)`) also defs its base `arr`, so whole-array
///   reads (`array get arr`) see a fresh version;
/// - a base def where constant-keyed elements exist (a dynamic-key write
///   `set arr($i) v`, or a whole-array writer like `array set`) **fans**
///   over every known element — the write may have hit any of them.
///
/// The rename walk records a *use* of each fanned-only name's prior
/// version, so type inference joins the old element type with the written
/// value instead of trusting either.
fn expand_defs(direct: &[String], elems: &ArrayElems) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let push = |n: String, out: &mut Vec<String>| {
        if !out.contains(&n) {
            out.push(n);
        }
    };
    for d in direct {
        if let Some(open) = d.find('(') {
            push(d.clone(), &mut out);
            push(d[..open].to_owned(), &mut out);
        } else {
            push(d.clone(), &mut out);
            if let Some(els) = elems.get(d) {
                for e in els {
                    push(e.clone(), &mut out);
                }
            }
        }
    }
    out
}

/// The use-side counterpart of [`expand_defs`]: reading a base whose
/// constant-keyed elements are known (`$a($i)`, `array get a`, `parray a`)
/// reads *every* element, and a dynamic-key / whole-array write reads each
/// fanned element's prior version (the may-def join input). Both make the
/// element chains live and upward-exposed so phi placement and liveness see
/// them (adversarial findings F1/F2/F5 on PR #944).
fn expand_uses(direct_uses: &[String], direct_defs: &[String], elems: &ArrayElems) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let push = |n: &str, out: &mut Vec<String>| {
        if !out.iter().any(|e| e == n) {
            out.push(n.to_owned());
        }
    };
    for u in direct_uses {
        if let Some(els) = elems.get(u) {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    for d in direct_defs {
        if !d.contains('(')
            && let Some(els) = elems.get(d)
        {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    out
}

fn nonlocal_names_and_defsites(
    func: &cfg::Function,
    reachable: &HashSet<BlockId>,
    registry: &CommandRegistry,
    elems: &ArrayElems,
) -> (FxHashSet<String>, FxHashMap<String, FxHashSet<BlockId>>) {
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: true,
    });
    let mut nonlocal_names: FxHashSet<String> = FxHashSet::default();
    let mut defsites: FxHashMap<String, FxHashSet<BlockId>> = FxHashMap::default();

    for bn in reachable {
        let Some(block) = func.blocks.get(bn) else {
            continue;
        };
        let mut defined_here: FxHashSet<String> = FxHashSet::default();
        for stmt in &block.statements {
            let direct_uses = uses_of(stmt, &mut scanner, registry);
            let direct_defs = defs_of_with_registry(stmt, Some(registry));
            for u in
                direct_uses
                    .iter()
                    .cloned()
                    .chain(expand_uses(&direct_uses, &direct_defs, elems))
            {
                if !defined_here.contains(&u) {
                    nonlocal_names.insert(u);
                }
            }
            for var in expand_defs(&direct_defs, elems) {
                defsites.entry(var.clone()).or_default().insert(*bn);
                defined_here.insert(var);
            }
        }
        // Terminator reads are element-qualified like statement reads (a
        // condition's `$a(k)` must place the element's phi), and a base
        // read fans over known elements.
        let mut term_uses: Vec<String> = Vec::new();
        match &block.terminator {
            Some(cfg::Terminator::Branch { condition, .. }) => {
                term_uses.extend(condition.vars_element_qualified());
            }
            Some(cfg::Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value {
                    term_uses.extend(scanner.scan_word(v, registry));
                }
                if let Some(e) = expr {
                    term_uses.extend(e.vars_element_qualified());
                }
            }
            _ => {}
        }
        for u in term_uses
            .iter()
            .cloned()
            .chain(expand_uses(&term_uses, &[], elems))
        {
            if !defined_here.contains(&u) {
                nonlocal_names.insert(u);
            }
        }
    }
    (nonlocal_names, defsites)
}

// Variable-use extraction
//
// These functions determine which variables an IR statement reads.

/// Return `true` when argument at `arg_index` is a braced literal
/// (single-token STR word).
///
/// When token info is unavailable, returns `false` so unknown
/// arguments are still scanned as ordinary inputs. We only exclude
/// bodies when we can positively identify them as single-token
/// braced literals.
fn is_braced_arg(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    // tokens.argv includes the command name at index 0; args are 1-based.
    let tok_index = arg_index + 1;
    if tok_index >= tokens.single_token_word.len() {
        return false;
    }
    if !tokens.single_token_word[tok_index] {
        return false;
    }
    // A single-token word from a VAR or CMD token is not braced.
    if let Some(text) = tokens.argv_texts.get(tok_index) {
        !text.starts_with("${") && !text.starts_with('[')
    } else {
        true
    }
}

/// Return BODY arg indices that should be excluded from local statement uses.
///
/// We only exclude handler-style bodies that are lowered/analysed separately.
/// Dynamic evaluation commands like `eval` still need their args treated as
/// ordinary dataflow inputs (for taint and read-before-set tracking).
pub(crate) fn structural_body_indices(
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> HashSet<usize> {
    use tcl_registry::{ArgRole, BodyKind};

    // A foreign-dialect builtin disabled in the active dialect — known in some
    // dialect but absent from the active registry's `by_name` (the analyser
    // loads only the active dialect, so an iRules `when` / `log` / `session`
    // misses under plain Tcl) — is an unknown would-be user command here. Tcl
    // never substitutes inside its braced arguments, so every braced arg is
    // opaque *data*, not analysable script/expr; scanning it would read its
    // `$vars` and emit spurious findings (W210). Skip them. A command
    // unknown in *every* dialect (`get` miss *and* not known-in-any) — a real
    // user proc / TclOO body / recovery artefact — is NOT skipped: its braced
    // body still recurses.
    if registry.get(command).is_none() && registry.known_in_any_dialect(command) {
        return (0..args.len())
            .filter(|&idx| is_braced_arg(tokens, idx))
            .collect();
    }

    // The registry-declared `body_kind` on each spec /
    // subcommand tells us whether body args run in the caller's
    // frame (`Plain`) or in a separate definition / dispatch context
    // (`Structural`).  Only `Structural` body args belong in this
    // skip set — `if`, `while`, `for`, `foreach`, `catch`, `try`,
    // … bodies share the caller's frame and SSA must scan them as
    // part of the enclosing block's data flow.
    // An `ArgRole::LambdaLiteral` word is a whole anonymous procedure —
    // `{params body ?ns?}` — and C Tcl runs it in a **fresh frame**, so no
    // name written inside it is a read (or write) in the frame the call is
    // written in.  tclsh 9.0.4 / 8.6.14, identical:
    //
    //   set x 7; apply {{} {puts $x}}   ;# can't read "x": no such variable
    //
    // The lambda's own body is analysed on its own, in that frame, with its
    // parameter list bound (`lowering::lower_apply` registers it as a body
    // unit) — so scanning the literal here both mis-frames the read and
    // duplicates it, drawing a false `W210` on the lambda's own parameters
    // (issue #1070).  Unconditional, not gated on `body_kind`: the role is
    // itself the "fresh frame" statement.  A *dynamic* lambda (`apply
    // $lambda`) is not braced, so `is_braced_arg` keeps it a genuine read.
    let lambda_words: HashSet<usize> = {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        registry
            .arg_indices_for_role(command, &arg_strs, ArgRole::LambdaLiteral)
            .into_iter()
            .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
            .collect()
    };

    if let Some(spec) = registry.get(command) {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Subcommand body_kind (if the call dispatches to a sub).
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind == BodyKind::Structural {
            let candidates = registry.arg_indices_for_role(command, &arg_strs, ArgRole::Body);
            return candidates
                .into_iter()
                .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
                .chain(lambda_words)
                .collect();
        }
    }

    lambda_words
}

/// Extract variable names used (read) by an IR statement.
///
/// Uses a [`VarReferenceScanner`] to find variable references in word texts
/// and expression trees.
///
/// Returns sorted variable names, excluding variables that are defined
/// by this statement (unless they exhibit read-before-write semantics).
///
/// Names only, dropping the [`UseClass`] classification — use
/// [`uses_of_classified`] when the caller must distinguish a substituted read
/// from one carried by an unevaluated brace-quoted word.
pub fn uses_of(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<String> {
    uses_of_classified(stmt, scanner, registry)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The classified reads a statement scan accumulates: names substituted (or
/// evaluated in this frame) by the statement, and names mentioned only inside
/// a brace-quoted word it passes through verbatim.
#[derive(Default)]
struct ClassifiedUses {
    substituted: BTreeSet<String>,
    quoted: BTreeSet<String>,
}

impl ClassifiedUses {
    /// Absorb another scan's two halves, keeping each name in its own bucket.
    /// A name that is substituted *somewhere* stays substituted — the final
    /// `quoted.retain` in [`uses_of_classified`] resolves the overlap once,
    /// at the top, so intermediate merges never have to.
    fn merge(&mut self, other: ClassifiedUses) {
        self.substituted.extend(other.substituted);
        self.quoted.extend(other.quoted);
    }

    /// Absorb a classified name list (as [`uses_of_classified`] returns it).
    fn merge_classified(&mut self, uses: impl IntoIterator<Item = (String, UseClass)>) {
        for (name, class) in uses {
            match class {
                UseClass::Substituted => self.substituted.insert(name),
                UseClass::Quoted => self.quoted.insert(name),
            };
        }
    }

    /// Drop every name a collapsed body defines itself, from both halves.
    fn remove_defs(&mut self, defs: &HashSet<String>) {
        self.substituted.retain(|v| !defs.contains(v));
        self.quoted.retain(|v| !defs.contains(v));
    }
}

/// [`uses_of`] with each name's [`UseClass`].
///
/// A name reached by both a substituted word and a brace-quoted one is
/// [`UseClass::Substituted`] — the definite read wins.
pub fn uses_of_classified(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<(String, UseClass)> {
    let mut found = ClassifiedUses::default();
    let mut reads_own_def: BTreeSet<String> = BTreeSet::new();

    match stmt {
        Statement::ExprEval { expr, .. } => {
            found.substituted.extend(expr_vars_for(scanner, expr));
        }

        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => {
            uses_in_assignment(
                stmt,
                scanner,
                registry,
                &mut found.substituted,
                &mut reads_own_def,
            );
        }

        Statement::Call { .. } => {
            uses_in_call(stmt, scanner, registry, &mut found, &mut reads_own_def);
        }

        // A braced return value is literal: `proc f {} { return {$y} }`
        // returns the two characters `$y` and reads nothing.
        // tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]`
        // prints `$y` with `y` undefined.
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            if let Some(v) = value {
                let sink = if *braced {
                    &mut found.quoted
                } else {
                    &mut found.substituted
                };
                sink.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                found.substituted.extend(expr_vars_for(scanner, e));
            }
        }

        Statement::Barrier { .. } => {
            uses_in_barrier(stmt, scanner, registry, &mut found, &mut reads_own_def);
        }

        // A non-lowered (glob/regexp/fall-through) `switch` is kept opaque as a
        // single `Statement::Switch` in the block. Recover the subject + arm /
        // default body reads so a variable read only as the subject or only
        // inside an arm body isn't reported unused.
        Statement::Switch {
            subject,
            arms,
            default_body,
            patterns_braced,
            ..
        } => {
            found.merge(switch_reads(
                subject,
                arms,
                default_body.as_ref(),
                *patterns_braced,
                scanner,
                registry,
            ));
            // The subject is read *before* any arm assigns, so it stays a live
            // read even when an arm also defines it (`defs_of` may now report
            // the subject var as switch-defined). Without this the read-before-
            // def of the subject would be filtered out below.
            for v in scanner.scan_word(subject, registry) {
                reads_own_def.insert(v);
            }
        }

        // Other structured IR statements (If, For, While, …) are flattened by
        // the CFG builder before SSA construction, so they never reach here.
        _ => {}
    }

    // Exclude variables defined by this statement, unless they're
    // read-before-write.  Route through the registry so
    // `trace add variable` defs come from the registry's VarWrite
    // role rather than a string match.
    let defs: HashSet<String> = defs_of_with_registry(stmt, Some(registry))
        .into_iter()
        .collect();
    // A name reached both ways is a definite read — the quoted mention adds
    // nothing the substituted one does not already assert.
    let ClassifiedUses {
        substituted,
        mut quoted,
    } = found;
    quoted.retain(|v| !substituted.contains(v));
    substituted
        .into_iter()
        .map(|v| (v, UseClass::Substituted))
        .chain(quoted.into_iter().map(|v| (v, UseClass::Quoted)))
        .filter(|(v, _)| !v.is_empty() && (!defs.contains(v) || reads_own_def.contains(v)))
        .collect()
}

/// Variable reads of a [`Statement::Barrier`]: its head + non-body argument
/// words, plus the scope-alias name a `dict with` / `dict update` unpacks.
/// Extracted from [`uses_of_classified`].
fn uses_in_barrier(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let Statement::Barrier {
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        registry,
        found,
    );
    // Scope-alias subcommands (`dict with` / `dict update` — any
    // resolved subcommand with `creates_scope_alias`): the aliased
    // variable name is a plain string, not a $-substitution, so
    // scan_word misses it.
    //
    // The variable arg carries both VarRead and VarWrite roles.
    // When barrier defs route through the registry, the same name
    // appears in `defs` from the VarWrite query.  The closing
    // filter in `uses_of_classified`
    // (`!defs.contains(v) || reads_own_def.contains(v)`) would
    // then drop the var unless we mark it as reads-own-def here.
    // Without this, a proc whose only reference to a parameter is
    // `dict with $param {}` would produce a false unused-parameter
    // diagnostic.
    let creates_scope_alias = registry
        .get(command)
        .zip(args.first())
        .and_then(|(spec, sub)| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias);
    if !creates_scope_alias {
        return;
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in registry.arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::VarWrite) {
        let Some(word) = args.get(idx) else { continue };
        let alias_var = normalise_var_name(word);
        if !alias_var.is_empty() {
            let owned = alias_var.to_owned();
            found.substituted.insert(owned.clone());
            reads_own_def.insert(owned);
        }
    }
}

/// Variable reads of an assignment-style statement (`set` / `set =expr` /
/// `set =value` / `incr`). A *dynamic* target name (`set $p …`) reads its
/// name-bearing variable(s) (the genuine read the dead-store / unused checks
/// need — `defs_of` withholds the static def); a static target that the RHS
/// also reads is recorded as read-before-write. Extracted from [`uses_of`].
/// The value word an aliased / renamed `set` (`interp alias {} myset {} set` /
/// `rename set myset`) stores, or `None` when the `Call` is not a
/// value-passthrough store in the `set VAR VALUE` shape.
///
/// Keyed off the *canonical* command's `Set` lowering hook — the registry's
/// own "this is a value-passthrough store" fact — so the read scan matches the
/// un-aliased `set`, never the source spelling. The two-arg / single-def guard
/// restricts it to the setter shape (no `interp alias` prepended args shifting
/// the value word out of `args[1]`, and not the one-arg getter, which has no
/// def).
fn canonical_set_value<'a>(
    command: &str,
    canonical_command: Option<&str>,
    args: &'a [String],
    defs: &[String],
    registry: &CommandRegistry,
) -> Option<&'a str> {
    if defs.len() != 1 || args.len() != 2 {
        return None;
    }
    let canon = canonical_command.unwrap_or(command);
    let is_set = registry.get(canon).and_then(|s| s.lowering_hook)
        == Some(tcl_registry::hooks::LoweringHookId::Set);
    is_set.then(|| args[1].as_str())
}

/// Reads of a `set`-style value word, matching what the un-aliased `set`
/// lowering captures: an `[expr {…}]` value is parsed as an expression (so a
/// braced `$x`, which plain word scanning would miss, is seen); any other value
/// word is scanned for `$`-substitutions, recursing into `[...]` command
/// substitutions.
fn set_value_reads(
    value: &str,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> BTreeSet<String> {
    let trimmed = value.trim();
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
        && let Some((expr_arg, _)) =
            crate::lowering_hooks::extract_single_expr_arg(inner, &HashSet::new())
    {
        let expr = crate::parse_expr(&expr_arg, None);
        return if scanner.element_qualified() {
            expr.vars_element_qualified().into_iter().collect()
        } else {
            vars_in_expr(&expr)
        };
    }
    scanner.scan_word(value, registry)
}

/// Variable reads of a [`Statement::Call`]: its head + non-body argument
/// words, the explicit `VarRead`-role names (`reads`), a read-modify-write
/// target (`reads_own_defs`), and — for an aliased / renamed `set` — its value
/// word. Mirrors [`uses_in_assignment`]'s shape; extracted from [`uses_of`].
///
/// An aliased / renamed `set` (`interp alias {} myset {} set` / `rename set
/// myset`) stays a runtime `Call` (codegen must not inline it — the binding may
/// change by call time), but for def/use it reads its value word exactly as the
/// un-aliased `set VAR VALUE` would: an `[expr {…}]` value is parsed as an
/// expression so a braced `$x` is seen, and the target is a read-before-write
/// whenever the value references it. Without this a loop-carried self-store
/// (`myset x [expr {$x+1}]`) would look write-only, so no header phi would form
/// and S102 could never fire on the aliased store.
fn uses_in_call(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
    found: &mut ClassifiedUses,
    reads_own_def: &mut BTreeSet<String>,
) {
    let Statement::Call {
        command,
        canonical_command,
        args,
        defs,
        reads,
        reads_own_defs,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    scan_command_words(
        command,
        canonical_command.as_deref(),
        args,
        tokens.as_ref(),
        scanner,
        registry,
        found,
    );
    let vars_found = &mut found.substituted;
    if let Some(value) =
        canonical_set_value(command, canonical_command.as_deref(), args, defs, registry)
    {
        for v in set_value_reads(value, scanner, registry) {
            if defs.iter().any(|d| d.as_str() == v) {
                reads_own_def.insert(v.clone());
            }
            vars_found.insert(v);
        }
    }
    for name in reads {
        if !name.is_empty() {
            vars_found.insert(name.clone());
        }
    }
    if *reads_own_defs {
        for name in defs {
            vars_found.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
    // A destroying command (`unset a(k)`) consumes the target's *existence*:
    // the killed store is not dead — deleting it would make the unset error
    // on every call. Record the prior version as a read (DESTROYS_VARIABLE,
    // matching the pre-per-element behavior the base-level use gave).
    let destroys = registry
        .get(canonical_command.as_deref().unwrap_or(command))
        .is_some_and(|spec| {
            spec.traits
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
        });
    if destroys {
        for name in defs {
            vars_found.insert(name.clone());
            reads_own_def.insert(name.clone());
        }
    }
}

fn uses_in_assignment(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
    vars_found: &mut BTreeSet<String>,
    reads_own_def: &mut BTreeSet<String>,
) {
    // A brace-quoted target's `$` is part of a literal name, not a
    // substitution: it is neither a dynamic target nor a read of the
    // `$`-less lookalike (issue #1078).
    let note_reads_own = |name: &str,
                          braced: bool,
                          scanner: &VarReferenceScanner,
                          vars_found: &BTreeSet<String>,
                          rod: &mut BTreeSet<String>| {
        let norm = scanner.canonical_name_braced(name, braced);
        if !norm.is_empty() && vars_found.contains(norm) {
            rod.insert(norm.to_owned());
        }
    };
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            }
        }
        Statement::AssignExpr {
            name,
            name_braced,
            expr,
            ..
        } => {
            vars_found.extend(expr_vars_for(scanner, expr));
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::AssignValue {
            name,
            name_braced,
            value,
            ..
        } => {
            vars_found.extend(scanner.scan_word(value, registry));
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                note_reads_own(name, *name_braced, scanner, vars_found, reads_own_def);
            }
        }
        Statement::Incr {
            name,
            name_braced,
            amount,
            ..
        } => {
            if is_dynamic_write_target(name, *name_braced) {
                vars_found.extend(scanner.scan_word(name, registry));
            } else {
                let norm = scanner.canonical_name_braced(name, *name_braced);
                if !norm.is_empty() {
                    vars_found.insert(norm.to_owned());
                    reads_own_def.insert(norm.to_owned());
                }
            }
            if let Some(amt) = amount {
                vars_found.extend(scanner.scan_word(amt, registry));
            }
        }
        _ => {}
    }
}

/// The expression-AST variable reads, named per the scanner's qualification
/// mode — element-qualified for the SSA build, base names otherwise.
fn expr_vars_for(
    scanner: &VarReferenceScanner,
    expr: &crate::expr_ast::ExprNode,
) -> BTreeSet<String> {
    if scanner.element_qualified() {
        expr.vars_element_qualified().into_iter().collect()
    } else {
        vars_in_expr(expr)
    }
}

/// Scan a `Call` / `Barrier` head word plus its non-body argument words into
/// `vars_found`. `Body`-role args (loop / `if` / `catch` scripts) are skipped —
/// they are lowered into their own CFG blocks. Extracted from [`uses_of`].
fn scan_command_words(
    command: &str,
    canonical_command: Option<&str>,
    args: &[String],
    tokens: Option<&CommandTokens>,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
    out: &mut ClassifiedUses,
) {
    out.substituted.extend(scanner.scan_word(command, registry));
    // Registry lookups use the canonical name when the lowering resolved one
    // (a bare `test` under `namespace import ::tcltest::*` canonicalises to
    // `tcltest::test`; the raw spelling alone is not a registry key).
    let lookup = canonical_command.unwrap_or(command);
    let body_indices = structural_body_indices(lookup, args, tokens, registry);
    // A **brace-quoted word in a variable-name position** is a literal name,
    // not a template: `unset {$n}` destroys the variable called `$n` and reads
    // nothing (tclsh 9.0.4 / 8.6.14 — `set {$n} v; unset {$n}` leaves `n`
    // untouched).  Scanning its de-braced content recorded a phantom read of
    // `n`, which surfaced as `W213 Variable 'n' may not exist` (issue #1078).
    // Only name roles are exempt: a braced `Expr` word (`expr {$a + $b}`)
    // really does substitute, so its reads must still be seen.
    let name_role_braced: std::collections::HashSet<usize> =
        tokens.map_or_else(std::collections::HashSet::new, |t| {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            let mut idx =
                registry.arg_indices_for_role(lookup, &arg_strs, tcl_registry::ArgRole::VarWrite);
            idx.extend(registry.arg_indices_for_role(
                lookup,
                &arg_strs,
                tcl_registry::ArgRole::VarRead,
            ));
            idx.into_iter()
                .filter(|&i| t.arg_is_braced_literal(i))
                .collect()
        });
    // Positions whose brace-quoted word the callee still evaluates in *this*
    // frame — `expr {$a + $b}`, `if {$c} …`. Registry-driven: the set is
    // every `ArgRole` that answers `braced_word_evaluated_in_frame`, never a
    // command name.
    let in_frame_braced: std::collections::HashSet<usize> = {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        tcl_registry::ArgRole::ALL
            .iter()
            .filter(|role| role.braced_word_evaluated_in_frame())
            .flat_map(|&role| registry.arg_indices_for_role(lookup, &arg_strs, role))
            .collect()
    };
    // Whether the registry describes this command at all — the difference
    // between a braced word that is known **data** and one that is merely
    // *unclassified*. See [`braced_word_class`].
    let described = registry_describes(registry, lookup);
    for (idx, arg) in args.iter().enumerate() {
        if body_indices.contains(&idx) || name_role_braced.contains(&idx) {
            continue;
        }
        let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
        for name in scanner.scan_word(arg, registry) {
            let class = braced_word_class(&BracedWordSite {
                braced,
                evaluated_in_frame: in_frame_braced.contains(&idx),
                described,
                word: arg,
                name: &name,
            });
            match class {
                UseClass::Quoted => out.quoted.insert(name),
                UseClass::Substituted => out.substituted.insert(name),
            };
        }
    }
}

/// Whether the registry describes `name` — accepting a space-joined
/// *ensemble subcommand* spelling (`dict for`, `dict map`) as well as a plain
/// command name.
///
/// The CFG's synthetic loop header names itself that way
/// (`cfg_builder::cfg_lower::lower_foreach`), and `registry.get` keys only on
/// whole command names, so a plain `get` would report a `dict for` header as
/// *unclassified* and read its braced value word as a substitution.  Mirrors
/// the base/sub split `shimmer::use_site::foreach_header_expected_type`
/// already does for the same statement (issue #1260).
fn registry_describes(registry: &CommandRegistry, name: &str) -> bool {
    if registry.get(name).is_some() {
        return true;
    }
    name.split_once(' ').is_some_and(|(base, sub)| {
        registry
            .get(base)
            .is_some_and(|spec| spec.resolve_subcommand(sub).is_some())
    })
}

/// One `(argument word, name found in it)` pair, with the facts
/// [`braced_word_class`] decides on.
struct BracedWordSite<'a> {
    /// The word is a brace-quoted literal — Tcl substitutes nothing in it.
    braced: bool,
    /// The registry gives this position a role whose word the callee
    /// re-evaluates in the *calling* frame (`Body` / `Expr`).
    evaluated_in_frame: bool,
    /// The registry has a `CommandSpec` for this command at all.
    described: bool,
    /// The word's text (braces already stripped).
    word: &'a str,
    /// The name the scan found inside it.
    name: &'a str,
}

/// How this statement consumes `site.name`.
///
/// Tcl substitutes nothing inside `{…}`, so a braced word is never read *at*
/// the call site. What the word then **is** falls into three kinds, and the
/// answer differs per kind:
///
/// - **script, this frame** — `expr {$a + $b}`, `if {$c} …`: the callee
///   re-evaluates the text where the caller's variables are in scope, so the
///   name really is read here. [`UseClass::Substituted`].
/// - **data** — a braced word of a command the registry *describes*, at a
///   role that never evaluates it: `puts {$y}`, `string match {$pat*} …`,
///   `lsort -command {cmp $x}`. The registry is the authority that nothing
///   here evaluates the word in this frame, so there is no read.
///   [`UseClass::Quoted`] (issue #1237).
/// - **unclassified** — a braced word of a command the registry does *not*
///   describe: a user proc, an unknown definer. It may be a script, and if it
///   is it may run in this frame — a wrapper that hands it to an
///   `uplevel`-ing worker does exactly that, and tclsh then errors on an
///   unset name — so the read stands. **Unless** the word sets the name
///   itself first: then the read is of that script's own local whichever
///   frame it runs in, which is the shape an un-hooked definer body takes
///   (issue #1142).
fn braced_word_class(site: &BracedWordSite<'_>) -> UseClass {
    if !site.braced || site.evaluated_in_frame {
        return UseClass::Substituted;
    }
    if site.described || word_sets_name(site.word, site.name) {
        return UseClass::Quoted;
    }
    UseClass::Substituted
}

/// True when `word`, read as a script, contains a top-level `set NAME …` for
/// `name` — so a `$name` elsewhere in the same word reads that script's own
/// local rather than a variable of the enclosing frame.
///
/// The `Call` twin of the analyser's `barrier_body_locally_sets`, which
/// recovers the same fact for an opaque `Statement::Barrier` body. Segmenting
/// is skipped unless the word plausibly holds a `set` at all.
fn word_sets_name(word: &str, name: &str) -> bool {
    if !word.contains("set") {
        return false;
    }
    crate::segmenter::segment_commands(word)
        .into_iter()
        .filter(|seg| seg.texts.first().map(String::as_str) == Some("set"))
        .filter_map(|seg| seg.texts.get(1).map(|w| normalise_var_name(w).to_owned()))
        .any(|target| target == name)
}

/// Reads of a non-lowered (`-glob`/`-regexp`, or `-exact` with a fall-through
/// arm) `switch` kept opaque in a CFG block: the subject word, every arm
/// pattern, and the *free* reads of each arm/default body.
///
/// The arm bodies are the *only* scripts in the pipeline that reach SSA
/// un-lowered, so this walk is the one place a `UseClass` would otherwise be
/// invented rather than derived. It is threaded through instead (issue
/// #1266): a brace-quoted data word inside an arm keeps the same
/// [`UseClass::Quoted`] it would carry had the arm been lowered, so
/// read-before-set skips it while liveness still honours it.
fn switch_reads(
    subject: &str,
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&crate::ir::Script>,
    patterns_braced: bool,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses {
        substituted: scanner.scan_word(subject, registry),
        quoted: BTreeSet::new(),
    };
    for arm in arms {
        // A pattern from the canonical single braced `{pat body …}` block is a
        // literal list element — tclsh 9.0.4: `proc f {z} {switch -glob $z
        // {$a* {puts hit} default {puts miss}}}; f {$a}` prints `hit` with `a`
        // undefined. Supplied as separate words (`switch $s $pat {body}`) the
        // pattern does substitute. Same classification the lowered `-exact`
        // path already applies to its patterns.
        let pattern_sink = if patterns_braced {
            &mut reads.quoted
        } else {
            &mut reads.substituted
        };
        pattern_sink.extend(scanner.scan_word(&arm.pattern, registry));
        if let Some(body) = &arm.body {
            reads.merge(free_reads_in_script(body, scanner, registry));
        }
    }
    if let Some(db) = default_body {
        reads.merge(free_reads_in_script(db, scanner, registry));
    }
    reads
}

/// Reads a collapsed body consumes from the *outer* scope: its reads minus its
/// own defs (so arm-local temporaries — `set tmp 1; puts $tmp` — aren't seen as
/// outer reads). The def set is completed with the `for`-init/next and
/// if/while/for condition command-sub defs that `defs_from_ir_script` omits.
fn free_reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> ClassifiedUses {
    let mut defs: HashSet<String> = crate::ir_helpers::defs_from_ir_script(script)
        .into_iter()
        .collect();
    defs.extend(collapsed_extra_defs(script, registry, 0));
    let mut reads = reads_in_script(script, scanner, registry);
    reads.remove_defs(&defs);
    reads
}

/// Recursively collect classified variable reads from an un-lowered IR script.
fn reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses::default();
    for stmt in &script.statements {
        reads.merge(reads_in_stmt(stmt, scanner, registry));
    }
    reads
}

/// Classified variable reads of a single statement, recursing into nested
/// bodies. Leaf reads come from [`uses_of_classified`] (which resolves a
/// nested `Statement::Switch` via [`switch_reads`]); structured statements are
/// walked here because they are not lowered inside an opaque switch arm.
///
/// Every context this walk adds by hand — an `if`/`while`/`for` condition, a
/// loop's value word, a nested body — is one the enclosing frame really does
/// evaluate, so it is [`UseClass::Substituted`]; the single exception is a
/// **braced** loop value word, which is literal list text (issue #1260).
fn reads_in_stmt(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses::default();
    reads.merge_classified(uses_of_classified(stmt, scanner, registry));
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                reads.substituted.extend(vars_in_expr(&clause.condition));
                reads.merge(reads_in_script(&clause.body, scanner, registry));
            }
            if let Some(eb) = else_body {
                reads.merge(reads_in_script(eb, scanner, registry));
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            reads.substituted.extend(vars_in_expr(condition));
            reads.merge(reads_in_script(body, scanner, registry));
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            reads.merge(reads_in_script(init, scanner, registry));
            reads.substituted.extend(vars_in_expr(condition));
            reads.merge(reads_in_script(next, scanner, registry));
            reads.merge(reads_in_script(body, scanner, registry));
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                // A braced value word is literal list text — `foreach n {a $b
                // c}` iterates the three characters `$b`, it does not read
                // `b` (issue #1260). The name is still *recorded*, as
                // `Quoted`: dropping it here would take its liveness use with
                // it and resurrect a false W220 on a store whose only mention
                // is that word (the #1237 guard rail). Classifying is what
                // separates the two.
                let sink = if it.list_braced {
                    &mut reads.quoted
                } else {
                    &mut reads.substituted
                };
                sink.extend(scanner.scan_word(&it.list_arg, registry));
            }
            reads.merge(reads_in_script(body, scanner, registry));
        }
        Statement::Catch { body, .. } => {
            reads.merge(reads_in_script(body, scanner, registry));
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            reads.merge(reads_in_script(body, scanner, registry));
            for handler in handlers {
                reads.merge(reads_in_script(&handler.body, scanner, registry));
            }
            if let Some(fb) = finally_body {
                reads.merge(reads_in_script(fb, scanner, registry));
            }
        }
        _ => {}
    }
    reads
}

/// Depth cap for [`collapsed_extra_defs`]'s recursion over nested
/// `if`/`while`/`for`/`foreach`/`catch`/`try`/`switch` bodies — issue #996.
/// Transitively bounded today via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// feeding SSA construction was built by `crate::lowering`, which already
/// caps its own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_COLLAPSED_EXTRA_DEFS_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// `for`-init/next clause defs and if/while/for condition command-sub defs
/// (`[regexp … -> v]`) that [`crate::ir_helpers::defs_from_ir_script`] does not
/// recurse — recovered for the collapsed-body read subtraction only.
fn collapsed_extra_defs(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    depth: u32,
) -> BTreeSet<String> {
    use crate::ir_helpers::{defs_from_expr, defs_from_ir_script};
    let mut extra = BTreeSet::new();
    if MAX_COLLAPSED_EXTRA_DEFS_DEPTH.exceeded(depth) {
        return extra;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    extra.extend(defs_from_expr(&clause.condition, registry));
                    extra.extend(collapsed_extra_defs(&clause.body, registry, depth + 1));
                }
                if let Some(eb) = else_body {
                    extra.extend(collapsed_extra_defs(eb, registry, depth + 1));
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                extra.extend(defs_from_ir_script(init));
                extra.extend(defs_from_ir_script(next));
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(init, registry, depth + 1));
                extra.extend(collapsed_extra_defs(next, registry, depth + 1));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                for handler in handlers {
                    extra.extend(collapsed_extra_defs(&handler.body, registry, depth + 1));
                }
                if let Some(fb) = finally_body {
                    extra.extend(collapsed_extra_defs(fb, registry, depth + 1));
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                    }
                }
                if let Some(db) = default_body {
                    extra.extend(collapsed_extra_defs(db, registry, depth + 1));
                }
            }
            _ => {}
        }
    }
    extra
}

/// Frame for the iterative rename walk (avoids deep recursion).
struct RenameFrame {
    block: BlockId,
    child_index: usize,
    pushed_vars: Vec<String>,
    phase: RenamePhase,
}

/// Phase within a single rename frame.
enum RenamePhase {
    /// Process phi nodes and statements for this block.
    Enter,
    /// Iterate over dominator-tree children.
    ProcessChildren,
}

/// Name-keyed per-block state produced by the rename walk and consumed when
/// assembling the final SSA blocks. Bundled so the walk and the assembly step
/// share one value instead of threading five parallel maps.
#[derive(Default)]
struct RenameOutputs {
    phi_versions: HashMap<BlockId, HashMap<String, Version>>,
    phi_incoming: HashMap<BlockId, HashMap<String, HashMap<BlockId, Version>>>,
    entry_versions: HashMap<BlockId, HashMap<String, Version>>,
    exit_versions: HashMap<BlockId, HashMap<String, Version>>,
    stmt_infos: HashMap<BlockId, Vec<SsaStatement>>,
}

/// Add variable definitions introduced through a registry-typed instance's
/// class option table (for example a widget's linked input variable).
///
/// The ordinary registry def scan sees direct command heads. Runtime object
/// commands (`.entry configure …`, `$widget configure …`) need the same scan
/// after resolving their receiver class. The method opt-in and the option's
/// external-input bit are both registry data; this layer names neither a
/// method nor an option.
fn enrich_instance_option_defs(
    func: &cfg::Function,
    registry: &CommandRegistry,
) -> Option<cfg::Function> {
    let mut enriched = func.clone();
    let defs = enrich_instance_option_defs_with_initial(
        &mut enriched,
        registry,
        &crate::taint::InstanceClassState::new(),
    );
    (!defs.is_empty()).then_some(enriched)
}

/// Mirror a top-level `::name` definition under Tcl's equivalent bare spelling.
///
/// A widget's deferred `-textvariable` must name a global so it outlives the
/// constructor frame.  The registry/lowerer therefore records its static
/// `VarWrite` as `::name`. At module scope, however, Tcl resolves `$name` to
/// that same global variable.  SSA otherwise interns those spellings as two
/// unrelated variables and loses the input source at the overwhelmingly
/// common top-level `$name` read.  Only the one-segment global spelling has
/// this equivalence: `::ns::name` is not a bare `name` in `::top`.
///
/// This is a scope rule, not a Tk rule. Any registry option declaring a
/// global `VarWrite` gets the same top-level alias, while procedure bodies
/// retain Tcl's strict local/global distinction.
fn mirror_top_level_global_defs(func: &mut cfg::Function) -> bool {
    let mut changed = false;
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call { defs, .. } = statement else {
                continue;
            };
            let aliases: Vec<String> = defs
                .iter()
                .filter_map(|name| {
                    let bare = name.strip_prefix("::")?;
                    (!bare.is_empty() && !bare.contains("::")).then_some(bare.to_owned())
                })
                .filter(|alias| !defs.contains(alias))
                .collect();
            changed |= !aliases.is_empty();
            defs.extend(aliases);
        }
    }
    changed
}

/// In-place form used by module CFG construction when a procedure begins with
/// proven interpreter-global receiver facts.
pub(crate) fn enrich_instance_option_defs_with_initial(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
    initial: &crate::taint::InstanceClassState,
) -> HashSet<String> {
    let classes = crate::taint::local_instance_classes_with_initial(func, registry, initial);
    let mut instance_defs = HashSet::new();
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                span,
                command,
                args,
                defs,
                ..
            } = statement
            else {
                continue;
            };
            if args.is_empty() {
                continue;
            }
            let Some(class) =
                crate::taint::unique_instance_class(command, Some(&classes), Some(span.start()))
            else {
                continue;
            };
            let invocation_args: Vec<&str> = args.iter().map(String::as_str).collect();
            let Some(invocation) = registry.resolve_instance_invocation(
                class,
                command,
                &invocation_args,
                registry.own_availability_mask(),
            ) else {
                continue;
            };
            if !invocation
                .semantics
                .traits
                .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
            {
                continue;
            }
            let option_args: Vec<&str> = args[1..].iter().map(String::as_str).collect();
            for (option_index, candidate) in args[1..].iter().enumerate() {
                if candidate.is_empty() || candidate.contains(['$', '[']) {
                    continue;
                }
                if tcl_registry::taint::taints_var_write(
                    registry,
                    class,
                    &option_args,
                    registry.own_availability_mask(),
                    candidate,
                ) {
                    let name = crate::naming::element_var_name(candidate);
                    let name = if registry.option_variable_scope(
                        class,
                        &option_args,
                        option_index,
                        registry.own_availability_mask(),
                    ) == Some(tcl_registry::VariableScope::Global)
                        && !name.starts_with("::")
                    {
                        format!("::{name}")
                    } else {
                        name.to_owned()
                    };
                    if !name.is_empty() {
                        instance_defs.insert(name.clone());
                        if !defs.contains(&name) {
                            defs.push(name);
                        }
                    }
                }
            }
        }
    }
    instance_defs
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
///
/// This function is inherently long because the rename walk couples
/// version counters, stacks, phi versions, incoming edges, and
/// per-statement use/def maps — splitting it would just scatter the
/// state across many parameters.
// Long renumbering pass with sequential block-walk phases.
/// Intern names that appear *only* in a terminator (a `return $x` value /
/// expr, or a branch condition). The rename walk interns statement and phi
/// names but not terminator reads, so a parameter read solely in the return
/// (`proc p {x} { return $x }`) would otherwise be absent from the interner —
/// leaving `var_symbol` unable to resolve it, which breaks any consumer that
/// resolves such a name (e.g. the interprocedural O103 seed for an
/// argument-sensitive passthrough). Interning is map-membership only; no SSA
/// statement / version map changes, so every analysis result is unaffected.
fn intern_terminator_reads(
    func: &cfg::Function,
    interner: &mut VarInterner,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) {
    for block in func.blocks.values() {
        match &block.terminator {
            Some(cfg::Terminator::Branch { condition, .. }) => {
                for u in condition.vars_element_qualified() {
                    interner.intern(&u);
                }
            }
            Some(cfg::Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value {
                    for u in scanner.scan_word(v, registry) {
                        interner.intern(&u);
                    }
                }
                if let Some(e) = expr {
                    for u in e.vars_element_qualified() {
                        interner.intern(&u);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Assemble the final [`SsaBlock`] map from the rename walk's outputs: build
/// each block's phi list and intern its name-keyed entry / exit version maps
/// onto the persisted [`Symbol`]-keyed form. Extracted from [`build_ssa`].
fn assemble_ssa_blocks(
    func: &cfg::Function,
    phi_vars: &HashMap<BlockId, HashSet<String>>,
    interner: &mut VarInterner,
    out: &mut RenameOutputs,
) -> HashMap<BlockId, SsaBlock> {
    let mut ssa_blocks: HashMap<BlockId, SsaBlock> = HashMap::new();
    for (bn, block) in &func.blocks {
        let mut phis: Vec<Phi> = Vec::new();
        let mut phi_var_list: Vec<String> = phi_vars
            .get(bn)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        phi_var_list.sort();

        for var in &phi_var_list {
            phis.push(Phi {
                name: interner.intern(var),
                version: out
                    .phi_versions
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .copied()
                    .unwrap_or(0),
                incoming: out
                    .phi_incoming
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .cloned()
                    .unwrap_or_default(),
            });
        }

        // Intern the entry / exit version maps (built name-keyed during the
        // walk) onto the persisted [`Symbol`]-keyed block.
        let intern_versions = |interner: &mut VarInterner, m: HashMap<String, Version>| {
            m.into_iter()
                .map(|(name, ver)| (interner.intern(&name), ver))
                .collect::<HashMap<Symbol, Version>>()
        };
        let entry_v = intern_versions(interner, out.entry_versions.remove(bn).unwrap_or_default());
        let exit_v = intern_versions(interner, out.exit_versions.remove(bn).unwrap_or_default());

        ssa_blocks.insert(
            *bn,
            SsaBlock {
                name: block.name.clone(),
                phis,
                statements: out.stmt_infos.remove(bn).unwrap_or_default(),
                entry_versions: entry_v,
                exit_versions: exit_v,
            },
        );
    }
    ssa_blocks
}

/// Mutable state threaded through the dominator-tree rename walk: the live
/// version stacks / counters keyed by variable name, the use-scanner and
/// name interner, and the accumulated per-block [`RenameOutputs`].
struct RenameWalk {
    version_counter: HashMap<String, Version>,
    stacks: HashMap<String, Vec<Version>>,
    scanner: VarReferenceScanner,
    interner: VarInterner,
    out: RenameOutputs,
}

impl RenameWalk {
    fn new(func: &cfg::Function) -> Self {
        let mut out = RenameOutputs::default();
        for id in func.blocks.keys() {
            out.phi_versions.insert(*id, HashMap::new());
            out.phi_incoming.insert(*id, HashMap::new());
            out.entry_versions.insert(*id, HashMap::new());
            out.exit_versions.insert(*id, HashMap::new());
            out.stmt_infos.insert(*id, Vec::new());
        }
        Self {
            version_counter: HashMap::new(),
            stacks: HashMap::new(),
            scanner: VarReferenceScanner::new(VarScanOptions {
                include_var_read_roles: true,
                recurse_cmd_substitutions: true,
                include_reads_before_write: false,
                element_qualified: true,
            }),
            interner: VarInterner::default(),
            out,
        }
    }

    /// Top (current) version of `var`, or `0` when none is live.
    fn top(&self, var: &str) -> Version {
        self.stacks
            .get(var)
            .and_then(|s| s.last().copied())
            .unwrap_or(0)
    }

    /// Allocate a fresh version for `var` and push it onto its live stack.
    fn push_new(&mut self, var: &str) -> Version {
        let vn = self.version_counter.get(var).copied().unwrap_or(0) + 1;
        self.version_counter.insert(var.to_owned(), vn);
        self.stacks.entry(var.to_owned()).or_default().push(vn);
        vn
    }

    /// Snapshot the currently-visible (var, version) pairs (those with a live
    /// version > 0), including this block's phi targets.
    fn visible_versions(&self, bn: BlockId) -> HashMap<String, Version> {
        let mut visible_vars: BTreeSet<String> = self.stacks.keys().cloned().collect();
        visible_vars.extend(self.out.phi_versions[&bn].keys().cloned());
        visible_vars
            .iter()
            .filter_map(|v| {
                let t = self.top(v);
                (t > 0).then(|| (v.clone(), t))
            })
            .collect()
    }

    /// Rename one statement's uses and defs into SSA form: look up each read's
    /// current version (carrying its [`UseClass`] through to `quoted_uses`),
    /// then push a fresh version for each def. Extracted from
    /// [`Self::enter_block`].
    fn rename_statement(
        &mut self,
        stmt: &Statement,
        frame: &mut RenameFrame,
        registry: &CommandRegistry,
        elems: &ArrayElems,
    ) -> SsaStatement {
        let classified = uses_of_classified(stmt, &mut self.scanner, registry);
        let class_of: HashMap<&str, UseClass> = classified
            .iter()
            .map(|(name, class)| (name.as_str(), *class))
            .collect();
        let uses_list: Vec<String> = classified.iter().map(|(name, _)| name.clone()).collect();
        let mut uses_map: HashMap<Symbol, Version> = HashMap::new();
        let mut quoted_uses: HashSet<Symbol> = HashSet::new();
        // A base read (`$a($i)`, `array get a`) reads every known
        // constant-keyed element — record their versions so the element chains
        // are live. A fanned element inherits its base's class: reached only
        // through a quoted base mention it is no more definite than that
        // mention.
        let fanned = expand_uses(&uses_list, &[], elems);
        for var in uses_list.iter().chain(fanned.iter()) {
            let v = self.top(var);
            let sym = self.interner.intern(var);
            if uses_map.insert(sym, v).is_some() {
                continue;
            }
            let class = class_of.get(var.as_str()).copied().or_else(|| {
                let base = &var[..var.find('(')?];
                class_of.get(base).copied()
            });
            if class == Some(UseClass::Quoted) {
                quoted_uses.insert(sym);
            }
        }

        let direct_defs = defs_of_with_registry(stmt, Some(registry));
        let expanded = expand_defs(&direct_defs, elems);
        // A fanned element def is a *may*-write — the dynamic-key /
        // whole-array write may have hit it: record a use of its prior version
        // so type inference joins old and new rather than trusting the written
        // value alone. (The base refresh of an element write is also a may-def,
        // but reads nothing — an extra base use would make dead-store analysis
        // see every element write as an observation of the whole array.)
        for var in expanded
            .iter()
            .filter(|v| !direct_defs.contains(v) && v.contains('('))
        {
            let ver = self.top(var);
            uses_map.entry(self.interner.intern(var)).or_insert(ver);
        }
        let mut defs_map: HashMap<Symbol, Version> = HashMap::new();
        let mut may_defs: HashSet<Symbol> = HashSet::new();
        for var in expanded {
            let ver = self.push_new(&var);
            frame.pushed_vars.push(var.clone());
            let sym = self.interner.intern(&var);
            defs_map.insert(sym, ver);
            if !direct_defs.contains(&var) {
                may_defs.insert(sym);
            }
        }

        SsaStatement {
            statement: stmt.clone(),
            uses: uses_map,
            defs: defs_map,
            may_defs,
            quoted_uses,
        }
    }

    /// Process one block on first visit: assign phi versions, record entry /
    /// exit versions, rename statement uses/defs, and seed successors' phi
    /// incoming edges. Extracted from [`build_ssa`]'s rename walk.
    fn enter_block(
        &mut self,
        frame: &mut RenameFrame,
        func: &cfg::Function,
        phi_vars: &HashMap<BlockId, HashSet<String>>,
        registry: &CommandRegistry,
        elems: &ArrayElems,
    ) {
        let bn = frame.block;

        // Process phi nodes — push new versions.
        let mut phi_var_list: Vec<String> = phi_vars
            .get(&bn)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        phi_var_list.sort();

        for var in &phi_var_list {
            let ver = self.push_new(var);
            frame.pushed_vars.push(var.clone());
            self.out
                .phi_versions
                .get_mut(&bn)
                .unwrap()
                .insert(var.clone(), ver);
            self.out
                .phi_incoming
                .get_mut(&bn)
                .unwrap()
                .entry(var.clone())
                .or_default();
        }

        // Record entry versions.
        let ev = self.visible_versions(bn);
        *self.out.entry_versions.get_mut(&bn).unwrap() = ev;

        // Process statements.
        if let Some(block) = func.blocks.get(&bn) {
            let stmts: Vec<Statement> = block.statements.clone();
            for stmt in &stmts {
                let info = self.rename_statement(stmt, frame, registry, elems);
                self.out.stmt_infos.get_mut(&bn).unwrap().push(info);
            }
        }

        // Record exit versions.
        let xv = self.visible_versions(bn);
        *self.out.exit_versions.get_mut(&bn).unwrap() = xv;

        // Fill in phi incoming edges for successors — the terminator's
        // successors plus any `try` exception-edge handler targets, so
        // a handler block's phis see this block's versions.
        for succ in func.block_successors(bn) {
            if !func.blocks.contains_key(&succ) {
                continue;
            }
            let mut succ_phis: Vec<String> = phi_vars
                .get(&succ)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            succ_phis.sort();
            for var in &succ_phis {
                let v = self.top(var);
                self.out
                    .phi_incoming
                    .get_mut(&succ)
                    .unwrap()
                    .entry(var.clone())
                    .or_default()
                    .insert(bn, v);
            }
        }
    }
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
#[must_use]
pub fn build_ssa(func: &cfg::Function, registry: &CommandRegistry) -> SsaFunction {
    // Complexity guard: skip the O(blocks·vars) phi placement + rename walk
    // for a pathologically large (usually generated) body. Returns a trivial
    // SSA; the compilation-unit builder likewise produces a trivial analysis
    // and flags the function so per-proc diagnostic passes skip it.
    if is_complexity_guarded(func) {
        return SsaFunction::trivial(func.name.clone(), func.entry, func.block_names().to_vec());
    }

    let mut enriched = enrich_instance_option_defs(func, registry);
    if func.name == "::top" {
        let target = enriched.get_or_insert_with(|| func.clone());
        let _ = mirror_top_level_global_defs(target);
    }
    let func = enriched.as_ref().unwrap_or(func);

    // 1. Compute dominance information.  Use the Cooper-Harvey-
    //    Kennedy immediate-dominator algorithm directly — it is
    //    O(N) memory, where the set-based `compute_dominators` is
    //    O(N²) and exhausts memory on a single huge generated proc
    //    (tens of thousands of CFG blocks).
    let idom = compute_idom_fast(func);
    let df = compute_dominance_frontier(func, &idom);
    let tree = build_dom_tree(&idom);
    let elems = collect_array_elems(func, registry);
    let phi_vars = compute_phi_vars(func, &df, registry, &elems);

    // 2. Set up rename state: the transient version stacks / counters, the
    // use-scanner, name interner, and per-block outputs (keyed by variable
    // name / version, interned to `Symbol` when blocks are assembled).
    let mut walk = RenameWalk::new(func);

    // 3. Rename walk — iterative using an explicit stack to avoid
    //    deep recursion on large dominator trees.
    let mut stack: Vec<RenameFrame> = Vec::new();

    if func.blocks.contains_key(&func.entry) {
        stack.push(RenameFrame {
            block: func.entry,
            child_index: 0,
            pushed_vars: Vec::new(),
            phase: RenamePhase::Enter,
        });
    }

    while let Some(frame) = stack.last_mut() {
        match frame.phase {
            RenamePhase::Enter => {
                walk.enter_block(frame, func, &phi_vars, registry, &elems);
                frame.phase = RenamePhase::ProcessChildren;
            }

            RenamePhase::ProcessChildren => {
                let bn = frame.block;
                let children = tree.get(&bn).cloned().unwrap_or_default();
                let idx = frame.child_index;

                if idx < children.len() {
                    frame.child_index += 1;
                    let child = children[idx];
                    stack.push(RenameFrame {
                        block: child,
                        child_index: 0,
                        pushed_vars: Vec::new(),
                        phase: RenamePhase::Enter,
                    });
                } else {
                    // Pop versions pushed in this block.
                    let pushed = frame.pushed_vars.clone();
                    for var in pushed.iter().rev() {
                        if let Some(s) = walk.stacks.get_mut(var) {
                            s.pop();
                            if s.is_empty() {
                                walk.stacks.remove(var);
                            }
                        }
                    }
                    stack.pop();
                }
            }
        }
    }

    // 4. Assemble SSA blocks.
    let ssa_blocks = assemble_ssa_blocks(func, &phi_vars, &mut walk.interner, &mut walk.out);

    intern_terminator_reads(func, &mut walk.interner, &mut walk.scanner, registry);

    SsaFunction {
        name: func.name.clone(),
        entry: func.entry,
        blocks: ssa_blocks,
        idom,
        dominance_frontier: df
            .into_iter()
            .map(|(k, v)| {
                let mut sorted: Vec<BlockId> = v.into_iter().collect();
                sorted.sort_unstable();
                (k, sorted)
            })
            .collect(),
        dominator_tree: tree,
        block_names: func.block_names().to_vec(),
        var_names: walk.interner.names,
        var_to_symbol: walk.interner.to_symbol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function, Terminator};
    use crate::expr_ast::ExprNode;
    use tcl_lexer::Span;

    /// Intern `name` into `func` and insert a fresh block for it, returning
    /// the [`BlockId`]. The shared test idiom for building a CFG by hand.
    fn block(func: &mut Function, name: &str) -> BlockId {
        let id = func.intern_block(name);
        func.blocks.insert(id, Block::new(name));
        id
    }

    fn make_goto(target: BlockId) -> Terminator {
        Terminator::Goto { target, span: None }
    }

    fn make_branch(cond: &str, t: BlockId, f: BlockId) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Raw { text: cond.into() },
            true_target: t,
            false_target: f,
            span: None,
            condition_base: None,
        }
    }

    fn make_return() -> Terminator {
        Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        }
    }

    /// `(func, [entry, then, else, end])` for a diamond CFG:
    /// entry → branch → then/else → end → return.
    fn diamond_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let then = block(&mut func, "then");
        let els = block(&mut func, "else");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
        func
    }

    /// Build a loop CFG: entry → header → branch → body → header / end
    fn loop_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let header = block(&mut func, "header");
        let body = block(&mut func, "body");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
        func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return());
        func
    }

    /// Resolve a block name to its id in `func` (test convenience).
    fn id_of(func: &Function, name: &str) -> BlockId {
        func.block_id(name).expect("block name interned")
    }

    // Data structure tests

    #[test]
    fn phi_construction() {
        let phi = Phi {
            name: Symbol(0),
            version: 3,
            incoming: HashMap::from([(BlockId(1), 1), (BlockId(2), 2)]),
        };
        assert_eq!(phi.name, Symbol(0));
        assert_eq!(phi.version, 3);
        assert_eq!(phi.incoming.len(), 2);
    }

    #[test]
    fn ssa_statement_construction() {
        let stmt = SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            uses: HashMap::new(),
            defs: HashMap::from([(Symbol(0), 1)]),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        assert_eq!(stmt.defs[&Symbol(0)], 1);
        assert!(stmt.uses.is_empty());
    }

    #[test]
    fn ssa_block_construction() {
        let block = SsaBlock {
            name: "entry".into(),
            phis: vec![],
            statements: vec![],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::from([(Symbol(0), 1)]),
        };
        assert_eq!(block.name, "entry");
        assert!(block.phis.is_empty());
    }

    #[test]
    fn ssa_function_construction() {
        let func = SsaFunction {
            name: "::test".into(),
            entry: BlockId(0),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
            block_names: vec!["entry".into()],
            var_names: Vec::new(),
            var_to_symbol: rustc_hash::FxHashMap::default(),
        };
        assert_eq!(func.name, "::test");
    }

    #[test]
    fn var_interner_assigns_first_seen_order_symbols() {
        let mut func = SsaFunction::trivial("::test", BlockId(0), vec!["entry".into()]);
        let x = func.intern_var("x");
        let y = func.intern_var("y");
        assert_eq!(x, Symbol(0));
        assert_eq!(y, Symbol(1));
        // Re-interning a known name returns the existing symbol.
        assert_eq!(func.intern_var("x"), x);
        assert_eq!(func.var_symbol("y"), Some(y));
        assert_eq!(func.var_symbol("missing"), None);
        assert_eq!(func.var_name(x), "x");
        assert_eq!(func.var_names(), ["x".to_string(), "y".to_string()]);
    }

    // defs_of tests

    #[test]
    fn defs_of_assign_const() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_dynamic_target_name_has_no_static_def() {
        // `set $p 1` / `set ${p} 1` write the variable *named by* `$p`, not
        // `p` itself — an opaque place, so there is no static def.
        for name in ["$p", "${p}", "a$b", "[gen]"] {
            let stmt = Statement::AssignValue {
                span: Span::new(0, 10),
                name: name.into(),
                name_braced: false,
                value: "1".into(),
                value_needs_backsubst: false,
                tokens: None,
            };
            assert!(
                defs_of(&stmt).is_empty(),
                "dynamic target {name:?} must not be a static def"
            );
        }
        // A constant-keyed array element defs its own per-element variable
        // (the rename walk adds the base def alongside); a dynamic key
        // stays on the base.
        let arr = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr(idx)".into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&arr), vec!["arr(idx)"]);
        let dyn_key = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr($i)".into(),
            name_braced: false,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&dyn_key), vec!["arr"]);
        // Braces suppress substitution: the key is the literal text `$i`.
        let braced = Statement::AssignValue {
            span: Span::new(0, 10),
            name: "arr($i)".into(),
            name_braced: true,
            value: "1".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        assert_eq!(defs_of(&braced), vec!["arr($i)"]);
    }

    #[test]
    fn is_dynamic_write_target_classifies_names() {
        assert!(is_dynamic_write_target("$p", false));
        assert!(is_dynamic_write_target("${p}", false));
        assert!(is_dynamic_write_target("a$b", false));
        assert!(is_dynamic_write_target("[gen]", false));
        assert!(!is_dynamic_write_target("p", false));
        assert!(!is_dynamic_write_target("arr(idx)", false));
        assert!(!is_dynamic_write_target("ns::var", false));
    }

    #[test]
    fn brace_quoted_write_target_is_not_dynamic() {
        // Issue #1078 — `set {$n} 1` names the literal variable `$n`; the
        // braces suppressed every substitution, so nothing about the target
        // is computed.  tclsh 9.0.4 / 8.6.14 (identical):
        //   set {$n} v; info exists {$n} → 1 ; info exists n → 0
        assert!(!is_dynamic_write_target("$n", true));
        assert!(!is_dynamic_write_target("${n}", true));
        assert!(!is_dynamic_write_target("[gen]", true));
        assert!(!is_dynamic_write_target("$a(k)", true));
        // TN control: the same spellings *unbraced* are still dynamic.
        assert!(is_dynamic_write_target("$n", false));
        assert!(is_dynamic_write_target("[gen]", false));
    }

    #[test]
    fn defs_of_incr() {
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        assert_eq!(defs_of(&stmt), vec!["i"]);
    }

    #[test]
    fn defs_of_call_with_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 20),
            command: "lappend".into(),
            canonical_command: None,
            args: vec!["list".into(), "item".into()],
            defs: vec!["list".into()],
            reads: vec![],
            reads_own_defs: true,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(defs_of(&stmt), vec!["list"]);
    }

    #[test]
    fn defs_of_call_no_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 10),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert!(defs_of(&stmt).is_empty());
    }

    #[test]
    fn defs_of_return() {
        let stmt = Statement::Return {
            span: Span::new(0, 10),
            value: Some("1".into()),
            expr: None,
            braced: false,
        };
        assert!(defs_of(&stmt).is_empty());
    }

    #[test]
    fn defs_of_barrier_trace() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "variable".into(), "$x".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_barrier_dict_for() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "dict for".into(),
            command: "dict::for".into(),
            canonical_command: None,
            args: vec!["k v".into(), "$d".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["k", "v"]);
    }

    /// Loop-header barriers resolve via the registry's `loop_list_header`
    /// flag — the ensemble-rewritten spellings work, and a user proc that
    /// merely ENDS in `::for` does not have its first argument misread as
    /// loop variables (the old suffix match did exactly that).
    #[test]
    fn defs_of_barrier_loop_header_via_registry() {
        let reg = CommandRegistry::build_default();
        let barrier = |command: &str| Statement::Barrier {
            span: Span::new(0, 30),
            reason: "loop".into(),
            command: command.into(),
            canonical_command: None,
            args: vec!["k v".into(), "$d".into()],
            tokens: None,
        };
        for cmd in ["::tcl::dict::for", "dict::for", "::tcl::dict::map"] {
            assert_eq!(
                defs_of_with_registry(&barrier(cmd), Some(&reg)),
                vec!["k", "v"],
                "{cmd} is a registry loop-list-header subcommand"
            );
        }
        assert_eq!(
            defs_of_with_registry(&barrier("::tcl::array::for"), Some(&reg)),
            vec!["k", "v"],
            "array for shares the loop-list-header shape"
        );
        assert!(
            !defs_of_with_registry(&barrier("my::for"), Some(&reg))
                .iter()
                .any(|d| d == "k" || d == "v"),
            "a user proc ending in ::for must not be misread as a loop header"
        );
    }

    #[test]
    fn defs_of_barrier_loop_header_uses_tcl_list_grammar() {
        let reg = CommandRegistry::build_default();
        let barrier = |var_list: &str| Statement::Barrier {
            span: Span::new(0, 30),
            reason: "loop".into(),
            command: "::tcl::dict::for".into(),
            canonical_command: None,
            args: vec![var_list.into(), "$d".into()],
            tokens: None,
        };

        assert_eq!(
            defs_of_with_registry(
                &barrier(r#"{one name} "two name" three\ name plain"#),
                Some(&reg),
            ),
            vec!["one name", "two name", "three name", "plain"],
        );
        assert!(
            defs_of_with_registry(&barrier("{unterminated"), Some(&reg)).is_empty(),
            "a malformed var-list fails before defining loop variables"
        );
    }

    /// `tcltest::test` body indices come from the spec's arg-role resolver
    /// (option-keyed `-setup`/`-body`/`-cleanup` values plus the legacy
    /// positional body) via the generic `ArgRole::Body` walk — the old
    /// `command == "test"` special case is gone.
    #[test]
    fn structural_body_indices_tcltest_via_registry() {
        use tcl_registry::ArgRole;
        let reg = CommandRegistry::build_default();
        // Option form: -setup and -body values are Body roles; -result's is
        // not. Both the qualified and the exported bare spelling resolve.
        let option_form = [
            "n",
            "d",
            "-setup",
            "{set x 1}",
            "-body",
            "{incr x}",
            "-result",
            "2",
        ];
        let got = reg.arg_indices_for_role("tcltest::test", &option_form, ArgRole::Body);
        assert_eq!(got, vec![3, 5], "option-form bodies via the option model");
        // The bare `test` spelling is NOT a registry key: it resolves only
        // through the lowering's namespace-import canonicalisation (which
        // stamps `canonical_command` — threaded into the SSA lookups). A
        // user proc that merely happens to be called `test` must not pick
        // up tcltest body semantics, which the old string match caused.
        assert!(
            reg.arg_indices_for_role("test", &option_form, ArgRole::Body)
                .is_empty()
        );
        // Legacy positional form: body is the penultimate argument.
        let legacy = ["n", "d", "{incr x}", "1"];
        let got = reg.arg_indices_for_role("tcltest::test", &legacy, ArgRole::Body);
        assert_eq!(got, vec![2], "legacy positional body");
        // Option form with NO body options marks nothing (and must not fall
        // back to the positional branch: `-result` is not a body).
        let no_body = ["n", "d", "-result", "2"];
        let got = reg.arg_indices_for_role("tcltest::test", &no_body, ArgRole::Body);
        assert!(got.is_empty(), "no body options → no body roles: {got:?}");
        // The generic structural walk consumes the same roles (braced-token
        // filtering applies on real token streams; `None` tokens filter all,
        // so only the empty expectation is assertable here).
        let no_body_args: Vec<String> = no_body.iter().map(|s| (*s).to_string()).collect();
        assert!(structural_body_indices("tcltest::test", &no_body_args, None, &reg).is_empty());
    }

    /// `trace add variable` defs route through the registry's
    /// `ArgRole::VarWrite` query rather than a string match.
    #[test]
    fn defs_of_barrier_trace_via_registry() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "variable".into(), "$x".into()],
            tokens: None,
        };
        assert_eq!(defs_of_with_registry(&stmt, Some(&reg)), vec!["x"]);
    }

    /// `trace add execution` does NOT define a variable — the
    /// command name being traced is not a `VarWrite` target.
    #[test]
    fn defs_of_barrier_trace_add_execution_no_def() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            canonical_command: None,
            args: vec!["add".into(), "execution".into(), "foo".into()],
            tokens: None,
        };
        assert!(defs_of_with_registry(&stmt, Some(&reg)).is_empty());
    }

    /// `global x y z` produces NO defs from the registry path
    /// (`var_scoping` handles the per-arg list).  Without the
    /// `CREATES_DYNAMIC_BARRIER` skip, the role-driven walk would
    /// only mark `x` (partial defs) and miss `y` / `z`.
    #[test]
    fn defs_of_barrier_global_vararg_no_partial_defs() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "global".into(),
            command: "global".into(),
            canonical_command: None,
            args: vec!["x".into(), "y".into(), "z".into()],
            tokens: None,
        };
        assert!(defs_of_with_registry(&stmt, Some(&reg)).is_empty());
    }

    /// `variable a b c` — same vararg-list shape as
    /// `global`, same skip.
    #[test]
    fn defs_of_barrier_variable_vararg_no_partial_defs() {
        let reg = CommandRegistry::build_default();
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "variable".into(),
            command: "variable".into(),
            canonical_command: None,
            args: vec!["a".into(), "b".into(), "c".into()],
            tokens: None,
        };
        assert!(defs_of_with_registry(&stmt, Some(&reg)).is_empty());
    }

    // Dominator tests

    #[test]
    fn dominators_linear() {
        // entry → b1 → b2 → return
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let b1 = block(&mut func, "b1");
        let b2 = block(&mut func, "b2");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        func.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
        func.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

        let dom = compute_dominators(&func);
        assert_eq!(dom[&entry], HashSet::from([entry]));
        assert_eq!(dom[&b1], HashSet::from([entry, b1]));
        assert_eq!(dom[&b2], HashSet::from([entry, b1, b2]));
    }

    #[test]
    fn dominators_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);

        // entry dominates everything
        for id in func.blocks.keys() {
            assert!(dom[id].contains(&entry));
        }
        // then and else are not dominated by each other
        assert!(!dom[&then].contains(&els));
        assert!(!dom[&els].contains(&then));
        // end is dominated by entry but not by then or else
        assert!(dom[&end].contains(&entry));
        assert!(!dom[&end].contains(&then));
        assert!(!dom[&end].contains(&els));
    }

    #[test]
    fn idom_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom[&entry], None);
        assert_eq!(idom[&then], Some(entry));
        assert_eq!(idom[&els], Some(entry));
        assert_eq!(idom[&end], Some(entry));
    }

    #[test]
    fn idom_loop() {
        let func = loop_cfg();
        let entry = func.entry;
        let (header, body, end) = (
            id_of(&func, "header"),
            id_of(&func, "body"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom[&entry], None);
        assert_eq!(idom[&header], Some(entry));
        assert_eq!(idom[&body], Some(header));
        assert_eq!(idom[&end], Some(header));
    }

    #[test]
    fn compute_idom_fast_matches_reference() {
        // The production CHK path (`compute_idom_fast`) must produce
        // exactly the same immediate dominators as the set-based
        // reference (`compute_idom` over `compute_dominators`) across
        // linear / diamond / loop shapes.
        let mut linear = Function::new("::test", "entry");
        let entry = linear.entry;
        let b1 = block(&mut linear, "b1");
        let b2 = block(&mut linear, "b2");
        linear.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        linear.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
        linear.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

        for func in [linear, diamond_cfg(), loop_cfg()] {
            let reference = compute_idom(&func, &compute_dominators(&func));
            let fast = compute_idom_fast(&func);
            assert_eq!(fast, reference, "CHK idom diverged for {:?}", func.entry);
        }
    }

    #[test]
    fn compute_idom_fast_handles_long_chain_without_blowup() {
        // A long chain of `if`-style diamonds (the shape a big
        // generated dispatch proc lowers to) must compute quickly via
        // CHK — this is the regression for the analyser stalling /
        // OOMing on machine-generated files.
        let mut func = Function::new("::big", "b0");
        let n = 4000;
        for i in 0..n {
            let cur = func.intern_block(format!("b{i}"));
            func.blocks
                .entry(cur)
                .or_insert_with(|| Block::new(format!("b{i}")));
            let then = block(&mut func, &format!("t{i}"));
            let next = block(&mut func, &format!("b{}", i + 1));
            func.blocks.get_mut(&then).unwrap().terminator = Some(make_return());
            func.blocks.get_mut(&cur).unwrap().terminator = Some(make_branch("c", then, next));
        }
        let bn = id_of(&func, &format!("b{n}"));
        func.blocks.get_mut(&bn).unwrap().terminator = Some(make_return());
        let idom = compute_idom_fast(&func);
        // Each chain block's idom is the previous chain block.
        assert_eq!(idom[&id_of(&func, "b1")], Some(id_of(&func, "b0")));
        assert_eq!(idom[&bn], Some(id_of(&func, &format!("b{}", n - 1))));
        assert_eq!(idom[&id_of(&func, "b0")], None);
    }

    #[test]
    fn dominance_frontier_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // then and else have "end" in their dominance frontier
        assert!(df[&then].contains(&end));
        assert!(df[&els].contains(&end));
        // entry has no dominance frontier
        assert!(df[&entry].is_empty());
    }

    #[test]
    fn dominance_frontier_loop() {
        let func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // body has "header" in its dominance frontier (back edge)
        assert!(df[&body].contains(&header));
        // entry strictly dominates header, so header is NOT in entry's DF
        assert!(
            df[&entry].is_empty(),
            "entry's DF should be empty; got {:?}",
            df[&entry]
        );
    }

    #[test]
    fn dom_tree_diamond() {
        let func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let tree = build_dom_tree(&idom);

        // entry's children include then, else, end (all directly dominated)
        let entry_children = &tree[&entry];
        assert!(entry_children.contains(&els));
        assert!(entry_children.contains(&end));
        assert!(entry_children.contains(&then));
    }

    // Phi placement tests

    #[test]
    fn phi_vars_diamond_with_defs() {
        // x defined in both then and else → phi needed at end
        let mut func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );
        func.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 20),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&els)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(30, 40),
                name: "x".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        assert!(phi[&end].contains("x"), "x should need a phi at 'end'");
        assert!(
            !phi[&entry].contains("x"),
            "x should not need a phi at entry"
        );
    }

    #[test]
    fn phi_vars_single_def_no_phi() {
        // x defined only in entry → no phi needed anywhere
        let mut func = diamond_cfg();
        let entry = func.entry;
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        for vars in phi.values() {
            assert!(!vars.contains("x"), "x should not need a phi anywhere");
        }
    }

    #[test]
    fn phi_vars_loop_def() {
        // i defined in entry and body → phi at header
        let mut func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(30, 40),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        assert!(
            phi[&header].contains("i"),
            "i should need a phi at 'header'"
        );
    }

    #[test]
    fn phi_vars_no_defs_no_phis() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(
            &func,
            &df,
            &CommandRegistry::build_default(),
            &ArrayElems::default(),
        );

        for vars in phi.values() {
            assert!(vars.is_empty(), "no defs → no phis");
        }
    }

    // uses_of tests

    fn default_registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn uses_of_assign_const() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.is_empty(), "constant assignment reads nothing");
    }

    #[test]
    fn uses_of_assign_value_with_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: false,
        });
        let stmt = Statement::AssignValue {
            span: Span::new(0, 15),
            name: "y".into(),
            name_braced: false,
            value: "$x".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"x".to_string()), "should read $x");
        assert!(
            !uses.contains(&"y".to_string()),
            "should not read $y (it's defined)"
        );
    }

    #[test]
    fn uses_of_incr() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        // incr reads and writes — reads_own_def
        assert!(uses.contains(&"i".to_string()), "incr reads the variable");
    }

    #[test]
    fn uses_of_return_with_value() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Return {
            span: Span::new(0, 15),
            value: Some("$result".into()),
            expr: None,
            braced: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"result".to_string()));
    }

    #[test]
    fn uses_of_expr_eval() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::ExprEval {
            span: Span::new(0, 20),
            expr: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Var {
                    text: "$b".into(),
                    name: "b".into(),
                    start: 5,
                    end: 7,
                }),
            },
            expr_base: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"a".to_string()));
        assert!(uses.contains(&"b".to_string()));
    }

    // UseClass classification (issues #1142 / #1237)

    /// A `Call` whose argument words are `args`, with per-word token kinds
    /// derived from the word text: `{…}` lexes to `Str` (brace-quoted, the one
    /// form Tcl leaves wholly unsubstituted), everything else to `Esc`.
    fn call_with_words(command: &str, args: &[&str]) -> Statement {
        let words: Vec<String> = std::iter::once(command.to_owned())
            .chain(args.iter().map(|a| (*a).to_owned()))
            .collect();
        let kinds: Vec<tcl_lexer::TokenType> = words
            .iter()
            .map(|w| {
                if w.starts_with('{') && w.ends_with('}') {
                    tcl_lexer::TokenType::Str
                } else {
                    tcl_lexer::TokenType::Esc
                }
            })
            .collect();
        let contents: Vec<String> = args
            .iter()
            .map(|a| {
                a.strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .unwrap_or(a)
                    .to_owned()
            })
            .collect();
        Statement::Call {
            span: Span::new(0, 1),
            command: command.to_owned(),
            canonical_command: None,
            args: contents,
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(crate::ir::CommandTokens::from_lossy_parts(
                vec![Span::new(0, 1); words.len()],
                words.clone(),
                kinds,
                vec![true; words.len()],
                Vec::new(),
                None,
            )),
            foreach_groups: None,
        }
    }

    fn classify(stmt: &Statement, reg: &CommandRegistry, name: &str) -> Option<UseClass> {
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        uses_of_classified(stmt, &mut scanner, reg)
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, class)| class)
    }

    /// A braced word at a role the callee does **not** evaluate in this frame
    /// is a `Quoted` use: still recorded (liveness must assume it may be
    /// evaluated later) but not a read here.
    /// tclsh-proof: tclsh8.6.14 — `puts {$y}` prints `$y` with `y` undefined.
    #[test]
    fn uses_of_classified_braced_data_word_is_quoted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["{$y}"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// The same name in an unbraced word is a definite read.
    #[test]
    fn uses_of_classified_unbraced_word_is_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["$y"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A braced `Expr`-role word really does substitute — `expr` evaluates it
    /// against the caller's variables — so it stays `Substituted`.
    #[test]
    fn uses_of_classified_braced_expr_word_is_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("expr", &["{$a + $b}"]);
        assert_eq!(classify(&stmt, &reg, "a"), Some(UseClass::Substituted));
        assert_eq!(classify(&stmt, &reg, "b"), Some(UseClass::Substituted));
    }

    /// A command the registry does not describe carries no role information,
    /// so its braced word is *unclassified*: it may be a script that runs in
    /// this frame. A name the word sets itself is that script's own local —
    /// the un-hooked definer shape (#1142) — so it is `Quoted`.
    #[test]
    fn uses_of_classified_unknown_definer_body_local_is_quoted() {
        let reg = default_registry();
        let stmt = call_with_words(
            "mydefiner",
            &["::foo::bar", "{optlist}", "{set y 1; return $y}"],
        );
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// TN control — a name the unclassified word does **not** set stays a
    /// read: a wrapper handing the word to an `uplevel`-ing worker really does
    /// evaluate it in this frame, and tclsh errors on the unset name.
    #[test]
    fn uses_of_classified_unknown_command_free_read_stays_substituted() {
        let reg = default_registry();
        let stmt = call_with_words("wrapper", &["myf", "{puts $myf}"]);
        assert_eq!(classify(&stmt, &reg, "myf"), Some(UseClass::Substituted));
    }

    /// A name reached both ways in one statement is a definite read.
    #[test]
    fn uses_of_classified_substituted_wins_over_quoted() {
        let reg = default_registry();
        let stmt = call_with_words("puts", &["{$y}", "$y"]);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// A braced `return` value is literal.
    /// tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]` prints
    /// `$y` with `y` undefined.
    #[test]
    fn uses_of_classified_braced_return_value_is_quoted() {
        let reg = default_registry();
        let stmt = Statement::Return {
            span: Span::new(0, 15),
            value: Some("$y".into()),
            expr: None,
            braced: true,
        };
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// Build a one-arm opaque `switch` around `body`, with `pattern` as the
    /// arm's pattern and the canonical braced arm block.
    fn opaque_switch(subject: &str, pattern: &str, body: crate::ir::Script) -> Statement {
        Statement::Switch {
            span: Span::new(0, 40),
            subject: subject.into(),
            subject_span: Span::new(0, 1),
            arms: vec![crate::ir::SwitchArm {
                pattern: pattern.into(),
                pattern_span: Span::new(0, 1),
                body: Some(body),
                body_span: Some(Span::new(0, 1)),
                fallthrough: false,
            }],
            default_body: None,
            default_span: None,
            mode: crate::ir::SwitchMode::Glob,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        }
    }

    /// Issue #1266 — an arm body is the one script that reaches SSA
    /// un-lowered, and its reads must arrive classified exactly as the same
    /// word would be outside the arm. A braced data word is `Quoted`.
    /// tclsh-proof: tclsh 9.0.4 — `proc f {z} { switch -glob $z { a* { puts
    /// {$b} } } }; f abc` prints `$b` with `b` undefined.
    #[test]
    fn uses_of_classified_opaque_switch_arm_braced_data_word_is_quoted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(call_with_words("puts", &["{$y}"]));
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
        // The subject is a genuine read of this frame.
        assert_eq!(classify(&stmt, &reg, "z"), Some(UseClass::Substituted));
    }

    /// TP control — the substituting spelling of the same arm word stays a
    /// definite read, so the classification is threaded, not dropped.
    #[test]
    fn uses_of_classified_opaque_switch_arm_unbraced_word_is_substituted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(call_with_words("puts", &["$y"]));
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// Issue #1266 — a `Statement::Foreach` inside an opaque arm is walked by
    /// `reads_in_stmt` rather than lowered, so `ForeachIterator::list_braced`
    /// (#1260) is the extra fact that walk needs: a braced value word is
    /// literal list text, recorded `Quoted` so liveness still honours it.
    #[test]
    fn uses_of_classified_opaque_switch_arm_braced_foreach_list_is_quoted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(Statement::Foreach {
            span: Span::new(0, 20),
            iterators: vec![crate::ir::ForeachIterator {
                vars: vec!["n".into()],
                list_arg: "a $y c".into(),
                list_braced: true,
            }],
            body: crate::ir::Script::new(),
            body_span: Span::new(0, 1),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        });
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    }

    /// TP control for the loop value word — the substituting spelling is a
    /// definite read even inside an opaque arm.
    #[test]
    fn uses_of_classified_opaque_switch_arm_substituted_foreach_list_is_substituted() {
        let reg = default_registry();
        let mut body = crate::ir::Script::new();
        body.statements.push(Statement::Foreach {
            span: Span::new(0, 20),
            iterators: vec![crate::ir::ForeachIterator {
                vars: vec!["n".into()],
                list_arg: "a $y c".into(),
                list_braced: false,
            }],
            body: crate::ir::Script::new(),
            body_span: Span::new(0, 1),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        });
        let stmt = opaque_switch("$z", "a*", body);
        assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
    }

    /// Issue #1266 — a pattern from the canonical braced arm block is a
    /// literal list element; supplied as separate words it substitutes.
    #[test]
    fn uses_of_classified_switch_pattern_class_follows_patterns_braced() {
        let reg = default_registry();
        let braced = opaque_switch("$z", "$p*", crate::ir::Script::new());
        assert_eq!(classify(&braced, &reg, "p"), Some(UseClass::Quoted));
        let mut worded = opaque_switch("$z", "$p*", crate::ir::Script::new());
        if let Statement::Switch {
            patterns_braced, ..
        } = &mut worded
        {
            *patterns_braced = false;
        }
        assert_eq!(classify(&worded, &reg, "p"), Some(UseClass::Substituted));
    }

    // build_ssa tests

    #[test]
    fn build_ssa_linear() {
        let reg = default_registry();
        // entry: set x 1; set y $x; return
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().statements = vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::AssignValue {
                span: Span::new(8, 16),
                name: "y".into(),
                name_braced: false,
                value: "$x".into(),
                value_needs_backsubst: false,
                tokens: None,
            },
        ];
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.name, "::test");
        assert_eq!(ssa.entry, entry);

        let sx = ssa.var_symbol("x").expect("x interned");
        let sy = ssa.var_symbol("y").expect("y interned");
        let entry_blk = &ssa.blocks[&entry];
        // First statement (set x 1): defs x=1
        assert_eq!(entry_blk.statements[0].defs.get(&sx), Some(&1));
        // Second statement (set y $x): uses x=1, defs y=1
        assert_eq!(entry_blk.statements[1].uses.get(&sx), Some(&1));
        assert_eq!(entry_blk.statements[1].defs.get(&sy), Some(&1));
    }

    #[test]
    fn build_ssa_diamond_phi() {
        let reg = default_registry();
        // entry → branch on $x → then: set x 1 → end
        //                       → else: set x 2 → end
        let mut func = diamond_cfg();
        let entry = func.entry;
        let (then, els, end) = (
            id_of(&func, "then"),
            id_of(&func, "else"),
            id_of(&func, "end"),
        );

        // Define x in entry first so it's used in the condition.
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });

        func.blocks
            .get_mut(&then)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 18),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&els)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(20, 28),
                name: "x".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            });
        // Read `x` after the join so it is upward-exposed at `end` — under
        // semi-pruned SSA a phi is placed only for a name with a downstream
        // reader (a dead phi for an unread merge is correctly dropped).
        func.blocks
            .get_mut(&end)
            .unwrap()
            .statements
            .push(Statement::Call {
                span: Span::new(30, 38),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["$x".into()],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            });

        let ssa = build_ssa(&func, &reg);

        // x is defined in both then and else and read at end, so the end block
        // should have a phi for x.
        let sx = ssa.var_symbol("x").expect("x interned");
        let end_block = &ssa.blocks[&end];
        assert!(
            end_block.phis.iter().any(|phi| phi.name == sx),
            "end block should have a phi for x"
        );

        // The phi should have incoming edges from then and else.
        if let Some(phi) = end_block.phis.iter().find(|p| p.name == sx) {
            assert!(
                phi.incoming.contains_key(&then),
                "phi should have incoming from then"
            );
            assert!(
                phi.incoming.contains_key(&els),
                "phi should have incoming from else"
            );
        }
    }

    #[test]
    fn build_ssa_loop() {
        let reg = default_registry();
        // entry: set i 0 → header: branch $i<10 → body: incr i → header
        //                                        → end: return
        let mut func = loop_cfg();
        let entry = func.entry;
        let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));

        func.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 8),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            });
        func.blocks
            .get_mut(&body)
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(20, 28),
                name: "i".into(),
                name_braced: false,
                amount: None,
                safe_on_uninit: false,
            });

        let ssa = build_ssa(&func, &reg);

        // header should have a phi for i (from entry and body).
        let si = ssa.var_symbol("i").expect("i interned");
        let header_blk = &ssa.blocks[&header];
        assert!(
            header_blk.phis.iter().any(|p| p.name == si),
            "header should have a phi for i"
        );
    }

    #[test]
    fn canonical_set_value_recognises_aliased_setter_only() {
        let reg = default_registry();
        let s = |xs: &[&str]| xs.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        // Aliased `set` (2-arg setter) → the value word.
        assert_eq!(
            canonical_set_value("myset", Some("set"), &s(&["x", "0"]), &s(&["x"]), &reg),
            Some("0")
        );
        // One-arg getter (no def) → None.
        assert_eq!(
            canonical_set_value("myset", Some("set"), &s(&["x"]), &[], &reg),
            None
        );
        // A non-set canonical (an aliased `puts`) → None.
        assert_eq!(
            canonical_set_value("myputs", Some("puts"), &s(&["x", "0"]), &s(&["x"]), &reg),
            None
        );
        // No canonical + a bare non-set command → None.
        assert_eq!(
            canonical_set_value("frobnicate", None, &s(&["x", "0"]), &s(&["x"]), &reg),
            None
        );
    }

    #[test]
    fn set_value_reads_parses_braced_expr() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        // A braced `[expr {…}]` value: plain word scanning misses `$x` inside
        // the braces, so it must be parsed as an expression.
        assert!(
            set_value_reads("[expr {$x + 1}]", &mut scanner, &reg).contains("x"),
            "braced expr value must expose $x as a read"
        );
        // A command-substitution value with a bare `$x` is caught by ordinary
        // recursion.
        assert!(set_value_reads("[string range $x 0 end]", &mut scanner, &reg).contains("x"));
        // A pure var-ref value.
        assert!(set_value_reads("$y", &mut scanner, &reg).contains("y"));
        // A bare literal reads nothing.
        assert!(set_value_reads("0", &mut scanner, &reg).is_empty());
    }

    #[test]
    fn build_ssa_empty_function() {
        let reg = default_registry();
        let mut func = Function::new("::empty", "entry");
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.blocks.len(), 1);
        assert!(ssa.blocks[&entry].phis.is_empty());
        assert!(ssa.blocks[&entry].statements.is_empty());
    }

    #[test]
    fn complexity_guard_skips_oversized_ssa() {
        let reg = default_registry();
        // A small function is below the ceiling and analysed normally.
        let small = Function::new("::small", "entry");
        assert!(!is_complexity_guarded(&small));

        // A function above the block ceiling is guarded: `build_ssa` returns a
        // trivial SSA without running the O(blocks·vars) dominator + phi walk
        // that would cost seconds on a pathological generated body.
        let mut big = Function::new("::big", "b0");
        let b0 = big.entry;
        for i in 0..=COMPLEXITY_GUARD_BLOCKS {
            let id = big.intern_block(format!("b{i}"));
            big.blocks.insert(id, Block::new(format!("b{i}")));
        }
        assert!(big.blocks.len() > COMPLEXITY_GUARD_BLOCKS);
        assert!(is_complexity_guarded(&big));

        let ssa = build_ssa(&big, &reg);
        assert!(ssa.blocks.is_empty(), "guarded SSA must be trivial");
        assert_eq!(ssa.name, "::big");
        assert_eq!(ssa.entry, b0);
    }

    /// Regression coverage for issue #996: `collapsed_extra_defs` recurses
    /// once per nested `If`/`While`/`For`/`Foreach`/`Catch`/`Try`/`Switch`
    /// body, with no depth cap of its own before this fix. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible crash.
    /// 2000 levels is comfortably past this new cap; the assertion is that
    /// the call returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_collapsed_extra_defs() {
        use crate::ir::{IfClause, Script};

        const DEPTH: usize = 2000;
        let leaf = Statement::AssignConst {
            span: Span::new(0, 0),
            name: "leaf".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Raw { text: "1".into() },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        let reg = CommandRegistry::build_default();
        let extra = collapsed_extra_defs(&script, &reg, 0);
        // Nothing but the leaf `AssignConst` here is a def source visible to
        // this helper (it only recovers `for`/condition-command-sub defs) —
        // the assertion is that this returns at all without overflowing the
        // stack, not what it returns.
        let _ = extra;
    }
}
