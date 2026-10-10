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

//! Shared helpers for the analyser diagnostics families.
//!
//! Free functions and small types used by more than one diagnostic family
//! (or by both the per-function dispatcher in the module root and a family):
//! source-slice extraction, dotted-quad scanning shared between the
//! subnet-mask and invalid-IP checks, the substitution / braced-word
//! predicates shared by the usage and security checks, the
//! defined-variable / existence-guard / globals-written collectors consumed
//! by the read-before-set machinery, and the [`UndefSuppression`] context
//! plus its phi-undef index that the dataflow read-before-set emitters
//! consult.

use std::collections::HashSet;
use tcl_dialect::model::SurfaceQuery;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::cfg::BlockId;

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// The source text covered by `span`, or `None` when the span is out
/// of bounds / not on char boundaries.
pub(super) fn source_slice(source: &str, span: tcl_lexer::Span) -> Option<String> {
    let start = span.start() as usize;
    let end = span.end() as usize;
    if start <= end && end <= source.len() {
        source.get(start..end).map(str::to_owned)
    } else {
        None
    }
}

/// A `\w` byte: ASCII alphanumeric or underscore (word boundary basis).
pub(super) fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// One dotted-quad match found in a value: the four octet substrings and
/// the byte offset where it begins (for context checks like a preceding
/// `/`).
pub(super) struct DottedQuad<'a> {
    pub(super) octets: [&'a str; 4],
    pub(super) start: usize,
    /// Byte offset just past the final octet (the regex `m.end()`).
    pub(super) end: usize,
}

/// Find every `\b\d{1,N}.\d{1,N}.\d{1,N}.\d{1,N}\b` dotted quad in
/// `text` (non-overlapping, left-to-right), replacing the regex scan.
/// `max_digits` caps each octet's digit count (`3` for the subnet-mask
/// check, `4` for the invalid-IP one).  Each octet starts at a word
/// boundary, so a longer digit run (a 4th/5th digit) simply fails to
/// align with the following `.` and is skipped — matching the regex.
pub(super) fn find_dotted_quads(text: &str, max_digits: usize) -> Vec<DottedQuad<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before && let Some((octets, end)) = match_dotted_quad(text, i, max_digits) {
            out.push(DottedQuad {
                octets,
                start: i,
                end,
            });
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Match a dotted quad starting at byte `start` (a word boundary), each
/// octet `1..=max_digits` digits separated by `.`, requiring a trailing
/// word boundary.  Returns the octet substrings and the end offset.
fn match_dotted_quad(text: &str, start: usize, max_digits: usize) -> Option<([&str; 4], usize)> {
    let bytes = text.as_bytes();
    let mut pos = start;
    let mut octets: [&str; 4] = [""; 4];
    for (k, slot) in octets.iter_mut().enumerate() {
        let run_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let len = pos - run_start;
        if len == 0 || len > max_digits {
            return None;
        }
        *slot = &text[run_start..pos];
        if k < 3 {
            if bytes.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1; // consume the dot
        }
    }
    // Trailing `\b`: end of string or a non-word byte.
    if pos < bytes.len() && is_word_byte(bytes[pos]) {
        return None;
    }
    Some((octets, pos))
}

/// True when `tok` is a brace-quoted word (`{…}`, a `Str` token).
pub(super) fn is_braced_word(tok: &tcl_lexer::Token) -> bool {
    tok.kind == tcl_lexer::TokenType::Str
}

/// True when `text` carries a substitution (`$` / `[`) or `tok` is a
/// `Var` / `Cmd` token.
pub(in crate::analyser) fn has_substitution(text: &str, tok: &tcl_lexer::Token) -> bool {
    has_substitution_of_kind(text, tok.kind)
}

/// [`has_substitution`] for a consumer that holds the word's token *kind*
/// without the token itself — the IR's `CommandTokens` records `argv_kinds`
/// alongside `argv_texts`, with no `Token` to hand.  The one predicate both
/// spellings share, so a change to what counts as a substitution reaches
/// every static-word check at once.
pub(in crate::analyser) fn has_substitution_of_kind(
    text: &str,
    kind: tcl_lexer::TokenType,
) -> bool {
    text.contains('$')
        || text.contains('[')
        || matches!(kind, tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd)
}

/// The safety class of a "wrap this word in braces" quick-fix (W100's
/// unbraced expression, W105's unbraced code block).
///
/// Bracing is the recommended form because it stops Tcl substituting the
/// word *before* the command sees it.  That is exactly why it cannot be
/// classified once for the whole diagnostic code: where the
/// written word carries no substitution, brace-quoting reaches the command
/// with byte-identical text and nothing observable changes; where it does,
/// the fix deliberately removes a round of substitution and a program that
/// depended on it changes behaviour.  C Tcl 9.0.3:
///
/// ```tcl
/// set a {$x}; set x 3; set b 2
/// puts [expr $a + $b]      ;# 5  — `$a` substitutes to `$x`, then expr
/// puts [expr {$a + $b}]    ;# error: `$a` is the string `$x`
/// ```
///
/// Equivalence therefore requires the written word to be substitution-free
/// on *every* mechanism the outer parse applies:
///
/// * no `$` / `[` and no whole-word `Var` / `Cmd` token — the caller passes
///   this as *`has_substitution`*, since W100 and W105 each already compute
///   it (W100 over a joined argument run, W105 over one body word);
/// * no backslash — the outer parse decodes `\n`, `\t`, `\x41`, and a
///   line continuation, so the braced text is not the text that reached
///   the command before;
/// * no `"` — a quoted word is stripped of its quotes by the outer parse,
///   so brace-quoting it either keeps them as literal characters or (where
///   the emitter strips them) depends on the word being exactly one quoted
///   run, which a `"a" eq "b"` argument list is not.
pub(in crate::analyser) fn brace_wrap_fix_safety(
    text: &str,
    has_substitution: bool,
) -> crate::analyser::types::FixSafety {
    use crate::analyser::types::FixSafety;
    if has_substitution || text.contains('\\') || text.contains('"') {
        FixSafety::BehaviourHardening
    } else {
        FixSafety::SemanticsEquivalent
    }
}

/// Whether one word of a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
/// tail contributes *statically-known script text* to the `Tcl_ConcatObj`
/// join — the one predicate every eval-family static-tail check shares
/// (`utils::concat_script_window` and source-owned diagnostic tails), so what counts
/// as static moves everywhere at once.
///
/// A braced (`Str`) word always does: the braces blocked every outer
/// substitution, so its contents reach the join byte-for-byte — a `$` or
/// `[` inside is script *text* that the eval-family command itself
/// resolves when the joined script runs, not a value consumed before it
/// (tclsh8.6.14 / tclsh9.0.4: `set value 5; eval {set l2} {$value};
/// puts $l2` → `5`).
///
/// Any other word is static only when nothing runs before the join: no
/// `$`/`[` substitution ([`has_substitution_of_kind`], which also rejects
/// whole-word `Var`/`Cmd` tokens), no backslash (the outer parse decodes
/// it, so the joined text is not the written text), and no quote byte
/// (quoting is consumed by the outer parse, never carried into the join).
/// A `{*}` expansion prefix restructures the words entirely and is never
/// static.
pub(in crate::analyser) fn word_is_static_script_text(
    text: &str,
    kind: tcl_lexer::TokenType,
) -> bool {
    match kind {
        tcl_lexer::TokenType::Str => true,
        tcl_lexer::TokenType::Expand => false,
        _ => !has_substitution_of_kind(text, kind) && !text.contains(['\\', '"']),
    }
}

/// An identifier-continuation byte: ASCII alphanumeric, `_`, or `:` (the
/// namespace-separator byte).
pub(super) fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// Exact cell guarded by a condition and its dominated successor region.
pub(super) type ExistenceGuard = (crate::var_resolve::VariableCellKey, BlockId);

/// Collect `(cell, guard_block)` pairs for every
/// `[info exists X]` / `[array exists X]` branch condition in `fu`.
/// A read of `var` in any block dominated by `guard_block` is guarded
/// (X provably exists).  A positive query guards the true target; a
/// `![info exists X]` query guards the false target.
pub(super) fn collect_existence_guards(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<ExistenceGuard> {
    use crate::cfg::Terminator;
    let mut guards = Vec::new();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return guards;
    };
    if config.normalized() != fu.source_lexer_config().normalized() {
        return guards;
    }
    for (&condition_block, block) in &fu.cfg.blocks {
        if let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            condition_base,
            ..
        }) = &block.terminator
        {
            let selected = if fu.ssa.point_contexts.is_some() {
                condition_base.and_then(|base| {
                    let tokens = crate::ssa::SsaSourceView::at_statement(
                        &fu.ssa,
                        condition_block,
                        usize::MAX,
                    )
                    .source_tokens()?;
                    let (query, context) =
                        crate::existence_query::in_expr_for_diagnostics_at_with_metadata_context(
                            condition,
                            base,
                            tokens,
                            registry,
                            config,
                            Some(metadata),
                        )?;
                    let place = crate::var_resolve::resolve_literal_place(
                        &query.var, &context, false, registry,
                    );
                    Some((query, crate::var_resolve::canonical_place_key(&place)?))
                })
            } else {
                crate::existence_query::in_expr_with_metadata_context(
                    condition,
                    registry,
                    config,
                    Some(metadata),
                )
                .and_then(|query| {
                    let symbol = fu
                        .ssa
                        .var_symbol_at_terminator(condition_block, &query.var)?;
                    Some((query, fu.ssa.cell_key(symbol).clone()))
                })
            };
            if let Some((query, cell)) = selected {
                let target = if query.negated {
                    *false_target
                } else {
                    *true_target
                };
                guards.push((cell, target));
            }
        }
    }
    guards
}

/// True when `block` is dominated by `dom` (walking the SSA immediate
/// dominator chain; a block dominates itself).
pub(super) fn block_dominated_by(
    ssa: &crate::ssa::SsaFunction,
    block: BlockId,
    dom: BlockId,
) -> bool {
    let mut cur = block;
    loop {
        if cur == dom {
            return true;
        }
        match ssa.idom.get(&cur) {
            Some(Some(parent)) => cur = *parent,
            _ => return false,
        }
    }
}

/// Read-only context threaded unchanged through [`phi_can_undef`]'s recursion:
/// the phi indices, the `unset`-killed set, the executable block / edge sets,
/// the dominating existence guards, the registry-owned startup binding, and
/// the SSA function itself.
pub(super) struct PhiUndefCtx<'a> {
    /// The registry whose special-variable faces answer the startup facts,
    /// pack rows included.
    pub registry: &'a tcl_registry::CommandRegistry,
    pub phi_def: &'a PhiDefMap,
    pub phi_block: &'a PhiBlockMap,
    pub killed: &'a FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub considered: &'a HashSet<BlockId>,
    pub executable_edges: &'a HashSet<(BlockId, BlockId)>,
    pub exists_guards: &'a [ExistenceGuard],
    pub may_defs: &'a MayDefMap,
    /// Startup bindings exist only in the document's initial global frame.
    pub initial_global: bool,
    /// Locals that registry metadata says alias the interpreter's global
    /// namespace in this function (`global name`).
    pub global_aliases: &'a HashSet<String>,
    pub dialect: Option<SurfaceQuery<'a>>,
    pub ssa: &'a crate::ssa::SsaFunction,
    /// The definitions a route's outcome preserved, each with the version
    /// it read ([`crate::sccp::SccpResult::preserved`]).
    pub preserved: &'a std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
}

/// The version whose binding a read of `version` reads: through the
/// definitions a route preserved ([`through_preserved`]) and the fresh
/// versions a call to code the module cannot see gave the names live after
/// it ([`crate::ssa::SsaFunction::binding_version`]), followed until neither
/// moves it. Each step reads an earlier version, so the walk ends.
fn binding_origin(
    ssa: &crate::ssa::SsaFunction,
    preserved: &std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
    symbol: crate::ssa::Symbol,
    mut version: crate::ssa::Version,
) -> crate::ssa::Version {
    loop {
        let origin = ssa.binding_version(symbol, through_preserved(preserved, symbol, version));
        if origin == version {
            return version;
        }
        version = origin;
    }
}

/// The version a read of `version` reads through the definitions a route
/// preserved: a preserved definition is its prior version's value and
/// existence, followed until a version no outcome preserved.
fn through_preserved(
    preserved: &std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
    symbol: crate::ssa::Symbol,
    mut version: crate::ssa::Version,
) -> crate::ssa::Version {
    // A preserved definition reads an earlier version, so the chain ends;
    // the bound keeps a malformed map from looping.
    for _ in 0..=preserved.len() {
        match preserved.get(&(symbol, version)) {
            Some(&prior) if prior != version => version = prior,
            _ => break,
        }
    }
    version
}

/// Return the registry spelling for a resolved potential startup variable, removing
/// only Tcl's global marker.  A named namespace (`::pkg::name`) deliberately
/// remains qualified and cannot accidentally inherit a global startup fact.
pub(super) fn startup_var_name(name: &str) -> &str {
    // naming.diagnostics.original-resolved-variable-name-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
    let normalised = crate::naming::split_array_name_braced(name, true).0;
    normalised.strip_prefix("::").unwrap_or(normalised)
}

/// Whether `name` resolves to the interpreter's global binding in this
/// analysis frame.  The `global`-alias set comes from a registry trait, so a
/// new dialect spelling can opt in without a command-name branch here.
pub(super) fn has_global_startup_binding(
    name: &str,
    initial_global: bool,
    global_aliases: &HashSet<String>,
) -> bool {
    if initial_global
        || crate::naming::split_array_name_braced(name, true)
            .0
            .starts_with("::")
    {
        return true;
    }
    let startup_name = startup_var_name(name);
    global_aliases.iter().any(|alias| {
        let alias_normalised = crate::naming::split_array_name_braced(alias, true).0;
        alias_normalised
            .strip_prefix("::")
            .unwrap_or(alias_normalised)
            == startup_name
    })
}

/// Registry startup spelling only when the exact cell belongs to the native
/// root namespace. Authored cells retain their separate declaration-only scope
/// contract; a same-spelled named namespace never acquires root startup facts.
pub(super) fn startup_cell_binding<'a>(
    cell: &'a crate::var_resolve::VariableCellKey,
    initial_global: bool,
    global_aliases: &HashSet<String>,
) -> (&'a str, bool) {
    use crate::var_resolve::VariableCellKey;
    match cell {
        VariableCellKey::Authored(name) => (
            startup_var_name(name),
            has_global_startup_binding(name, initial_global, global_aliases),
        ),
        VariableCellKey::Namespace { identity, simple } => {
            let Ok(display) = simple.try_utf8() else {
                return ("", false);
            };
            (
                display,
                match identity {
                    crate::command_binding::SourceNamespaceKey::Authored(namespace) => {
                        namespace == "::"
                    }
                    _ => identity
                        .exact_native_path()
                        .is_some_and(tcl_core_types::ByteNamespacePath::is_root),
                },
            )
        }
        VariableCellKey::Lifetime { cell, .. } | VariableCellKey::Element { cell, .. } => {
            startup_cell_binding(cell, initial_global, global_aliases)
        }
        _ => ("", false),
    }
}

/// Startup facts for one variable name, as [`phi_can_undef`] needs them.
///
/// They depend only on `name` (and the registry-owned dialect data), so they
/// are computed once per name instead of once per phi operand examined.
#[derive(Clone, Copy)]
struct StartupFacts {
    /// The default Tcl host binds this name before user code runs.
    readable_at_startup: bool,
    /// A read after `unset` materialises the value again — registry data
    /// confines this to `tcl_precision` on Tcl 8.x.
    rematerialises_after_unset: bool,
}

impl StartupFacts {
    fn for_name(name: &crate::var_resolve::VariableCellKey, ctx: &PhiUndefCtx<'_>) -> Self {
        let (startup_name, global_binding) =
            startup_cell_binding(name, ctx.initial_global, ctx.global_aliases);
        Self {
            readable_at_startup: global_binding
                && ctx
                    .registry
                    .is_readable_at_startup(startup_name, ctx.dialect),
            rematerialises_after_unset: global_binding
                && ctx.registry.is_lazily_readable(startup_name, ctx.dialect),
        }
    }
}

/// A phi version, as the undef trace keys them: the variable's interned SSA
/// [`Symbol`](crate::ssa::Symbol) rather than its name, so the index and its
/// worklist hash and copy a pair of `u32`s.
type VersionKey = (crate::ssa::Symbol, crate::ssa::Version);

/// Answers for [`phi_can_undef`], built once per [`PhiUndefCtx`] and shared by
/// every query run against it.
///
/// The trace used to be a DFS per query whose `seen` set was a *path* (grey)
/// set with no result reuse, so it enumerated every simple path through the
/// phi graph: N sibling conditional writes to one variable followed by a read
/// cost ~2^N visits, and real Quartus sources reach N ≈ 105 (issue #2021).
/// [`PhiUndefIndex::build`] answers every version at once instead, in one pass
/// over the phi operands plus a worklist.
#[derive(Default)]
pub(super) struct PhiUndefMemo {
    index: Option<PhiUndefIndex>,
}

impl PhiUndefMemo {
    /// The index, built on first use. `ctx` must be the same context on every
    /// call — a memo is per-context state, not a cache across contexts.
    fn index(&mut self, ctx: &PhiUndefCtx<'_>) -> &PhiUndefIndex {
        self.index.get_or_insert_with(|| PhiUndefIndex::build(ctx))
    }
}

/// Every version that can be undefined, plus the per-name facts the leaf
/// answers need.
struct PhiUndefIndex {
    /// Phi versions reaching an undef origin on some executable path.
    undef: FxHashSet<VersionKey>,
    /// `unset`-killed versions and the answer each gives.
    killed: FxHashMap<VersionKey, bool>,
    /// Startup facts per variable, for the version-0 answer.
    facts: FxHashMap<crate::ssa::Symbol, StartupFacts>,
}

impl PhiUndefIndex {
    /// Mark every phi version that can be undefined.
    ///
    /// A phi is undef when any of its reachable, non-existence-guarded
    /// incomings is undef, an incoming being undef when it is the version-0
    /// origin (and the host does not bind the name at startup), an
    /// `unset`-killed version (that a read does not materialise again), or
    /// itself an undef phi. Nothing else in that rule depends on how a version
    /// was reached, so it is plain reachability over the phi-operand graph:
    /// this walks the *reverse* graph from the undef origins, which answers
    /// every version in one pass — the same answers a per-query forward DFS
    /// gives, without re-deriving them once per path.
    ///
    /// Cycles need no special case here, and that keeps the old walk's
    /// "a back-edge is not undef" rule exactly. That walk's cut only stopped
    /// it re-entering a version already open on the *current* path, never
    /// stopping it reaching one by another route, and a query started with an
    /// empty path — so it still explored every version reachable from the
    /// query, and answered `true` for exactly the queries that reach an undef
    /// origin. A loop-header phi is undef only when an origin genuinely
    /// reaches it: going round the loop offers nothing the entry edge did not,
    /// so a phi whose only route to an origin is through itself stays
    /// unmarked, just as the cut answered "not undef" on the back-edge.
    fn build(ctx: &PhiUndefCtx<'_>) -> Self {
        let mut facts: FxHashMap<crate::ssa::Symbol, StartupFacts> = FxHashMap::default();
        let mut killed: FxHashMap<VersionKey, bool> = FxHashMap::default();
        for (name, version) in ctx.killed {
            let Some(symbol) = ctx.ssa.cell_symbol(name) else {
                continue;
            };
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(name, ctx));
            // A Tcl read trace is not an eager startup fact: `unset` removes
            // the current value, but a later read materialises it again.
            // Eager bindings such as argv remain genuine W210 reads after
            // `unset`.
            killed.insert((symbol, *version), !name_facts.rematerialises_after_unset);
        }

        let mut walk = UndefWalk::default();
        for (key, phi) in ctx.phi_def {
            let (name, version) = (&key.0, key.1);
            let symbol = phi.name;
            let node = (symbol, version);
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(name, ctx));
            if killed.contains_key(&node) {
                // Killed wins over the phi: the version is decided by the kill,
                // and its incomings never come into it.
                continue;
            }
            // The block this phi lives in — the destination of each incoming
            // edge.
            let this_block = ctx.phi_block.get(key).copied();
            for (&pred, &incoming) in &phi.incoming {
                if !ctx.considered.contains(&pred) {
                    continue;
                }
                // A phi has one operand per predecessor *edge*; an operand
                // arriving on a non-executable edge (SCCP proved the edge dead
                // — e.g. the `cond → exit` edge of `while 1`, which a `break`
                // makes the loop's only real exit) can never actually be read,
                // so its version-0 origin must not count as a possible undef.
                // This filter is only applied when SCCP edge info is available
                // (a non-empty set).
                if let Some(block) = this_block
                    && !ctx.executable_edges.is_empty()
                    && !ctx.executable_edges.contains(&(pred, block))
                {
                    continue;
                }
                let operand = (symbol, incoming);
                // A later deletion supersedes the condition's existence fact.
                if !killed.contains_key(&operand)
                    && ctx
                        .exists_guards
                        .iter()
                        .any(|(gv, gblk)| gv == name && block_dominated_by(ctx.ssa, pred, *gblk))
                {
                    continue;
                }
                // A definition a route preserved, and the fresh version a call
                // to code the module cannot see leaves, read the binding of an
                // earlier version.
                let incoming = binding_origin(ctx.ssa, ctx.preserved, symbol, incoming);
                walk.take(
                    node,
                    (symbol, incoming),
                    &killed,
                    name_facts.readable_at_startup,
                );
            }
        }
        // A name an opaque `switch`'s arm may write is a phi with one
        // operand: the version the statement read, which it holds when no arm
        // runs.
        for (key, &(block, prior)) in ctx.may_defs {
            let Some(symbol) = ctx.ssa.var_symbol(&key.0) else {
                continue;
            };
            let node = (symbol, key.1);
            if killed.contains_key(&node)
                || ctx.exists_guards.iter().any(|(cell, guard)| {
                    cell == &key.0 && block_dominated_by(ctx.ssa, block, *guard)
                })
            {
                continue;
            }
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(&key.0, ctx));
            let prior = through_preserved(ctx.preserved, symbol, prior);
            walk.take(
                node,
                (symbol, prior),
                &killed,
                name_facts.readable_at_startup,
            );
        }
        let undef = walk.finish();
        Self {
            undef,
            killed,
            facts,
        }
    }
}

/// The reachability walk behind [`PhiUndefIndex::build`]: which versions can
/// reach an undef origin, over the operand edges of the phis and of the
/// opaque `switch` may-definitions.
#[derive(Default)]
struct UndefWalk {
    undef: FxHashSet<VersionKey>,
    worklist: Vec<VersionKey>,
    /// Reverse operand edges: an undef version makes every version that takes
    /// it as an operand undef too.
    users: FxHashMap<VersionKey, Vec<crate::ssa::Version>>,
}

impl UndefWalk {
    /// `node` takes `operand`: undef when the operand is an undef origin or
    /// already known undef, and recorded as a user of it otherwise.
    fn take(
        &mut self,
        node: VersionKey,
        operand: VersionKey,
        killed: &FxHashMap<VersionKey, bool>,
        readable_at_startup: bool,
    ) {
        let origin = if let Some(&answer) = killed.get(&operand) {
            answer
        } else if operand.1 == 0 {
            // A version-zero incoming normally is the undef origin. The
            // default Tcl host, however, binds a registry-declared subset
            // before user code, and a conditional write would otherwise make
            // a merge with the startup version look undefined.
            // Procedure-local frames never set `initial_global`.
            !readable_at_startup
        } else if self.undef.contains(&operand) {
            true
        } else {
            // Another phi (or a concrete definition, which is never undef
            // and so never enters `undef`).
            self.users.entry(operand).or_default().push(node.1);
            return;
        };
        if origin && self.undef.insert(node) {
            self.worklist.push(node);
        }
    }

    /// Follow every undef version to the versions that take it.
    fn finish(mut self) -> FxHashSet<VersionKey> {
        while let Some(node) = self.worklist.pop() {
            let Some(users) = self.users.get(&node) else {
                continue;
            };
            for &user in users {
                let up = (node.0, user);
                if self.undef.insert(up) {
                    self.worklist.push(up);
                }
            }
        }
        self.undef
    }
}

/// Phi-from-undef trace.  A use's SSA version > 0 normally proves a prior
/// definition reached it, but a phi result whose reachable incomings
/// include an undefined (version-0) or `unset`-killed origin only reaches
/// on a subset of paths — the others read an unset variable.  Returns
/// true when `(name, version)` can be undefined on some reachable path.
///
/// Version 0 is
/// the undef origin; an `unset`-killed version is undef; a non-phi
/// (concrete) definition is never undef; a phi is undef if any of its
/// reachable, non-existence-guarded incomings is undef.  Cycles
/// (loop-header phis) conservatively resolve to *not* undef on the cycle.
///
/// `memo` holds the [`PhiUndefIndex`] that answers this, built on the first
/// query and reused by every later one; it must only be shared between
/// queries whose [`PhiUndefCtx`] is the same.
pub(super) fn phi_can_undef(
    name: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    ctx: &PhiUndefCtx<'_>,
    memo: &mut PhiUndefMemo,
) -> bool {
    let Some(symbol) = ctx.ssa.cell_symbol(name) else {
        // No SSA symbol means the function never defines the name, so it has
        // neither a phi nor a kill: only the startup answer can apply.
        return version == 0 && !StartupFacts::for_name(name, ctx).readable_at_startup;
    };
    let version = binding_origin(ctx.ssa, ctx.preserved, symbol, version);
    let index = memo.index(ctx);
    if let Some(&answer) = index.killed.get(&(symbol, version)) {
        return answer;
    }
    if version == 0 {
        return !index
            .facts
            .get(&symbol)
            .copied()
            .unwrap_or_else(|| StartupFacts::for_name(name, ctx))
            .readable_at_startup;
    }
    index.undef.contains(&(symbol, version))
}

/// `(name, version) → Phi` index used by [`phi_can_undef`].
pub(super) type PhiDefMap =
    FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), crate::ssa::Phi>;

/// `(name, version) → defining block` index, so [`phi_can_undef`] can test
/// each incoming `(pred, phi_block)` edge against the SCCP-executable edge set.
pub(super) type PhiBlockMap =
    FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), BlockId>;

/// The versions an opaque `switch` may define — a name one of its arms
/// writes — each with the version the statement read and the block it sits
/// in. Such a version holds its prior one when no arm runs, so it is
/// undefined exactly when that one can be: a phi with one operand.
pub(super) type MayDefMap = FxHashMap<
    (crate::var_resolve::VariableCellKey, crate::ssa::Version),
    (BlockId, crate::ssa::Version),
>;

/// The step each definition of a call to a procedure of the module takes
/// from the callee's transfer summary, by `(name, version)`, with the block
/// the call sits in and the version it found
/// ([`crate::value_transfer::summary_steps`]).
pub(super) type CallStepMap = FxHashMap<
    (crate::var_resolve::VariableCellKey, crate::ssa::Version),
    (
        crate::value_transfer::ExistenceStep,
        BlockId,
        crate::ssa::Version,
    ),
>;

/// The [`CallStepMap`] of `fu`'s calls in `considered` blocks: empty where
/// no module's procedures are in hand.
pub(super) fn call_steps(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
    registry: &tcl_registry::CommandRegistry,
) -> CallStepMap {
    let mut steps = CallStepMap::default();
    let Some(module) = module else {
        return steps;
    };
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return steps;
    };
    if !metadata.permits_logical_source_names() {
        return steps;
    }
    let config = fu.source_lexer_config();
    for &bn in considered {
        let Some(block) = fu.ssa.blocks.get(&bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            for (place, step) in crate::value_transfer::summary_steps(
                module,
                &fu.name,
                &statement.statement,
                &config,
            ) {
                let Some(symbol) = fu.ssa.var_symbol(&place) else {
                    continue;
                };
                if let Some(&version) = statement.defs.get(&symbol) {
                    let prior = crate::sccp::prior_version(block, index, symbol);
                    steps.insert(
                        (fu.ssa.cell_key(symbol).clone(), version),
                        (step, bn, prior),
                    );
                }
            }
        }
    }
    steps
}

/// The indices [`phi_can_undef`] answers from: phi operands, the block each
/// phi sits in, the `unset`-killed versions, and the may-definitions of the
/// opaque `switch` statements.
pub(super) struct UndefIndexMaps {
    pub phi_def: PhiDefMap,
    pub phi_block: PhiBlockMap,
    pub killed: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub may_defs: MayDefMap,
}

/// Build the `(name, version) → Phi` index, the `(name, version) → block`
/// index, and the set of `unset`-killed versions for [`phi_can_undef`],
/// restricted to `considered` (executable) blocks.
pub(super) fn build_phi_undef_index(
    ssa: &crate::ssa::SsaFunction,
    considered: &HashSet<BlockId>,
    registry: &tcl_registry::CommandRegistry,
    steps: &CallStepMap,
) -> UndefIndexMaps {
    use tcl_registry::value_transfer::{BindingKind, Existence};
    let mut phi_def: PhiDefMap = FxHashMap::default();
    let mut phi_block: PhiBlockMap = FxHashMap::default();
    let mut killed = FxHashSet::default();
    let mut may_defs = MayDefMap::default();
    for &bn in considered {
        let Some(sblock) = ssa.blocks.get(&bn) else {
            continue;
        };
        for phi in &sblock.phis {
            let phi_name = ssa.cell_key(phi.name).clone();
            phi_def.insert((phi_name.clone(), phi.version), phi.clone());
            phi_block.insert((phi_name, phi.version), bn);
        }
        for statement in &sblock.statements {
            if crate::ssa::has_arm_may_defs(&statement.statement) {
                let refreshed = crate::ssa::refreshed_bases(&statement.statement, registry);
                for symbol in &statement.may_defs {
                    if refreshed.iter().any(|base| base == ssa.var_name(*symbol)) {
                        continue;
                    }
                    if let (Some(&version), Some(&prior)) =
                        (statement.defs.get(symbol), statement.uses.get(symbol))
                    {
                        may_defs.insert((ssa.cell_key(*symbol).clone(), version), (bn, prior));
                    }
                }
            }
            for (&symbol, &version) in &statement.defs {
                if statement.destruction_defs.contains(&symbol) {
                    killed.insert((ssa.cell_key(symbol).clone(), version));
                }
            }
        }
    }
    // A call to a procedure of the module is the assignment its summary
    // states for each place it names: a step that leaves the place unset
    // whatever it held kills it, one that sets it is a definition, and any
    // other — a may-bind, a preserve, or a may-unset, which a summary also
    // states out of its own caution — leaves the place unset where it was
    // before the call, so it reads the version before the call.
    for (key, &(step, block, prior)) in steps {
        let after_bound = step.apply(Existence::Bound(BindingKind::Either));
        let after_unbound = step.apply(Existence::Unbound);
        if after_bound == Existence::Unbound && after_unbound == Existence::Unbound {
            killed.insert(key.clone());
        } else if matches!(after_unbound, Existence::Unbound | Existence::MayBound) {
            may_defs.insert(key.clone(), (block, prior));
        }
    }
    for (block, markers) in &ssa.value_clobbers {
        if !considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let origin = ssa.binding_version(symbol, fresh);
                let name = ssa.cell_key(symbol).clone();
                if killed.contains(&(name.clone(), origin)) {
                    killed.insert((name, fresh));
                }
            }
        }
    }
    UndefIndexMaps {
        phi_def,
        phi_block,
        killed,
        may_defs,
    }
}

/// Name-level suppression context for the `return`-value phi-from-undef W210
/// pass, harvested from `dict with` / `dict update` and qualified `variable`
/// declarations.
#[derive(Default)]
pub(super) struct UndefSuppression {
    /// A `dict with` / `dict update` is present (enables the key-aware gate).
    has_dict_with: bool,
    /// At least one dict-with target's value shape is statically unknown.
    dict_with_any_unknown: bool,
    /// Keys provably unpacked by some known-literal dict-with target.
    dict_with_known_keys: HashSet<String>,
    /// The dict-with target variable names themselves.
    dict_vars: HashSet<String>,
    /// Names with a concrete (version > 0) statement/phi definition.
    explicitly_defined: HashSet<String>,
    /// Local-alias tails declared by a qualified `variable ns::tail`.
    alias_tails: FxHashSet<String>,
    /// Where a command substitution buried inside an `expr` argument writes
    /// a name (`set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during
    /// expr evaluation): per name, each `(block, statement)` that writes it.
    /// The `[…]` is opaque to SSA def tracking, so a `$tmp` read in the same
    /// expression or after it looks read-before-set; a read before it is
    /// still one. Suppress-only.
    cmd_sub_writes: FxHashMap<String, Vec<(BlockId, usize)>>,
    /// Where code the module cannot see runs: each `(block, statement)` of a
    /// marker for it ([`crate::ssa::is_unseen_call_marker`]). A name nothing in
    /// the function assigns may be one that code set, so a read of it that
    /// follows a marker is no read before it is set. Suppress-only.
    unseen_call_sites: Vec<(BlockId, usize)>,
    /// Names written by a `Traits::SCRIPT_CONCATENATES_ARGS` call whose
    /// script the lowering left as an opaque barrier — `eval set l2 hello`
    /// really does set `l2` in the caller's own frame, but its words reach
    /// the IR as barrier arguments with no def attached, so a later
    /// `puts $l2` would look read-before-set.  Name-level, suppress-only.
    script_concat_writes: FxHashSet<String>,
    /// `(name, version)` pairs killed by an `unset` — undef at their reads,
    /// so a direct read of one is read-before-set just like a version-0
    /// origin.
    pub(super) killed: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub(super) call_steps: CallStepMap,
    /// Phi versions that can be undefined on some executable path
    /// (a one-branch `set y 1` merge, or a try-handler merge). A statement
    /// read of one is read-before-set; the def-use pass can't express this
    /// because the read targets the *phi* version, not a version-0 origin.
    pub(super) can_undef: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub(super) preserved_undef:
        FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    /// Loop-header phi versions whose *only* undef source is the loop's entry
    /// (zero-trip) edge — the loop body assigns the variable on every back
    /// edge, so the value is defined whenever the loop ran ≥1 time. Maps each
    /// such `(name, version)` to the loop's body-block name set.
    ///
    /// A read of the version reached *after* the loop (a block outside the
    /// set) is not read-before-set: matching C Tcl — which errors only when
    /// the iterator list / condition is actually empty at runtime, not merely
    /// when it *could* be — we assume a loop that may run does run. A read
    /// *inside* the loop body (a block in the set) still fires, because a
    /// first-iteration read before the body's assignment is a genuine error.
    /// A *provably* empty loop (`foreach x {}`, or a constant-false
    /// `while`/`for` SCCP already prunes) is excluded, so it keeps firing.
    pub(super) loop_entry_only_undef:
        FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), FxHashSet<String>>,
}

impl UndefSuppression {
    /// True when a read of `name` is suppressed by an alias declaration or a
    /// `dict with` / `dict update` unpack.  Blanket variant: an unknown-shape
    /// dict suppresses every non-concrete name (the conservative
    /// "might-have-the-key" stance, used where no truth source can confirm
    /// the dict is empty — e.g. a `return` after a `dict with` on a param).
    pub(super) fn suppresses(&self, name: &str) -> bool {
        self.suppresses_strict(name) || self.dict_with_blanket(name)
    }

    /// [`Self::suppresses`] for a read of the version `key`. A version in
    /// [`Self::preserved_undef`] was written by no substitution, so the
    /// condition-write suppression does not speak for it; every other one
    /// still does.
    pub(super) fn suppresses_read(
        &self,
        name: &str,
        key: &(crate::var_resolve::VariableCellKey, crate::ssa::Version),
    ) -> bool {
        if self.preserved_undef.contains(key) {
            return self.suppresses_unsubstituted(name) || self.dict_with_blanket(name);
        }
        self.suppresses(name)
    }

    /// The unknown-shape `dict with` blanket of [`Self::suppresses`].
    fn dict_with_blanket(&self, name: &str) -> bool {
        self.has_dict_with && self.dict_with_any_unknown && !self.explicitly_defined.contains(name)
    }

    /// True when reading `key` at `block` is a safe *after-loop* read of a
    /// variable the loop body defines on every iteration (see
    /// [`Self::loop_entry_only_undef`]): the version is loop-entry-only-undef
    /// and `block` is outside the loop body. A read inside the loop body still
    /// fires (first-iteration undef is real).
    pub(super) fn after_loop_defined(
        &self,
        key: &(crate::var_resolve::VariableCellKey, crate::ssa::Version),
        block: &str,
    ) -> bool {
        self.loop_entry_only_undef
            .get(key)
            .is_some_and(|body| !body.contains(block))
    }

    /// Like [`Self::suppresses`] but **without** the unknown-shape blanket —
    /// only alias tails, dict vars, and *provably-unpacked* keys suppress.
    /// Used on statement reads inside a `dict with` body, where an
    /// unknown-shape dict (e.g. an interprocedurally-empty literal
    /// SCCP cannot yet resolve) must still fire so a genuine missing-key read
    /// is not hidden.
    pub(super) fn suppresses_strict(&self, name: &str) -> bool {
        self.suppresses_unsubstituted(name)
    }

    /// Whether an `expr` argument's command substitution writes `name` at
    /// or before the read at `index` of `block` — earlier in the block, or
    /// in a block that dominates it; `index` -1 is the block's terminator.
    pub(super) fn written_by_substitution_before(
        &self,
        name: &str,
        ssa: &crate::ssa::SsaFunction,
        block: BlockId,
        index: i32,
    ) -> bool {
        self.cmd_sub_writes.get(name).is_some_and(|sites| {
            sites.iter().any(|&(site, at)| {
                if site == block {
                    usize::try_from(index).map_or(true, |index| at <= index)
                } else {
                    crate::loops::dominates(ssa, site, block)
                }
            })
        })
    }

    /// Whether code the module cannot see runs before the read at `index` of
    /// `block` — earlier in the block, or in a block that dominates it; `index`
    /// -1 is the block's terminator. A read in the statement a marker stands
    /// ahead of is after it, as the marker for a substitution's command stands
    /// ahead of its host.
    pub(super) fn unseen_call_before(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: BlockId,
        index: i32,
    ) -> bool {
        self.unseen_call_sites.iter().any(|&(site, at)| {
            if site == block {
                usize::try_from(index).map_or(true, |index| at < index)
            } else {
                crate::loops::dominates(ssa, site, block)
            }
        })
    }

    /// The name-level suppressions, none of them a substitution's write.
    fn suppresses_unsubstituted(&self, name: &str) -> bool {
        if self.alias_tails.contains(name)
            || self.dict_vars.contains(name)
            || self.script_concat_writes.contains(name)
        {
            return true;
        }
        self.has_dict_with
            && !self.explicitly_defined.contains(name)
            && self.dict_with_known_keys.contains(name)
    }
}

/// Each `(block, statement)` in the blocks `considered` where code the module
/// cannot see runs that may set any name the function reads: at the top level
/// any such code, which can set a global by name; in a procedure a sourced
/// file, which runs in the procedure's own frame. A callee the module cannot
/// see sets a procedure's local through `upvar 1` under a name it is handed,
/// which the per-name abstention answers
/// ([`crate::interprocedural::collect_opaque_callee_name_args`]), so its marker
/// is no site here.
fn collect_unseen_call_sites(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    initial_global: bool,
    semantics: UndefSuppressionSemantics<'_>,
) -> Vec<(BlockId, usize)> {
    let registry = semantics.context.commands();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return Vec::new();
    };
    let sources = |statement: &crate::ir::Statement| {
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            Some(metadata),
            statement,
        )
        .is_some_and(|call| {
            call.facts
                .traits
                .contains(tcl_registry::Traits::SOURCES_FILE)
        })
    };
    let mut out = Vec::new();
    for &block_id in considered {
        let Some(block) = fu.cfg.blocks.get(&block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            if crate::ssa::is_unseen_call_marker(statement)
                && (initial_global
                    || block
                        .statements
                        .iter()
                        .filter(|other| {
                            other.span() == statement.span() && other.synthetic_marker().is_none()
                        })
                        .any(sources))
            {
                out.push((block_id, index));
            }
        }
    }
    out
}

fn collect_expr_cmd_sub_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    semantics: UndefSuppressionSemantics<'_>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let registry = semantics.context.commands();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return out;
    };
    let Some(input) = metadata.source_analysis_input() else {
        return out;
    };
    if semantics.analysis.resolved_input.as_ref() != Some(input)
        || input.lexer_config().normalized() != semantics.lexer_config.normalized()
    {
        return out;
    }
    for &block_id in considered {
        let Some(block) = fu.ssa.blocks.get(&block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let crate::ir::Statement::AssignExpr {
                expr_base: Some(base),
                ..
            } = &statement.statement
            else {
                continue;
            };
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index).source_tokens()
            else {
                continue;
            };
            collect_original_expression_writes(tokens, *base, semantics, metadata, &mut out);
        }
        if let Some(crate::cfg::Terminator::Branch {
            condition_base: Some(base),
            ..
        }) = fu
            .cfg
            .blocks
            .get(&block_id)
            .and_then(|block| block.terminator.as_ref())
        {
            if let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, usize::MAX)
                    .source_tokens()
            {
                collect_original_expression_writes(tokens, *base, semantics, metadata, &mut out);
            }
        }
    }
    out
}

/// Select the authentic Expr operand at its own original source site. An
/// outer assignment's post-substitution lookup cannot replace a child lookup.
fn collect_original_expression_writes(
    tokens: &crate::ir::CommandTokens,
    expression_base: u32,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    let Some(input) = metadata.source_analysis_input() else {
        return;
    };
    let positions = tokens
        .source_binding
        .iter()
        .filter_map(|binding| binding.invocation_site().map(|site| site.offset))
        .chain(tokens.nested_bindings.iter().map(|(offset, _)| *offset));
    let mut visited = FxHashSet::default();
    for offset in positions.filter(|offset| visited.insert(*offset)) {
        let Some(words) = crate::registry_invocation::source_structure::source_registry_words_at(
            semantics.source,
            semantics.analysis,
            offset,
        ) else {
            continue;
        };
        let Some(bodies) = words.source_expression_script_bodies_at(input, expression_base) else {
            continue;
        };
        for body in bodies {
            collect_original_expression_body_writes(&body, semantics, metadata, out);
        }
    }
}

fn collect_original_expression_body_writes(
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    let Some(realm) = semantics.analysis.retained_command_realm() else {
        return;
    };
    let Some(text) = semantics.source.get(body.content_span().as_range()) else {
        return;
    };
    if !body.matches_context(semantics.context)
        || !body.matches_source(
            &tcl_lexer::SourceImage::document(semantics.source),
            semantics.lexer_config,
        )
    {
        return;
    }
    let source = tcl_lexer::SourceMap::new(semantics.source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        body.content_span().start(),
        semantics.lexer_config.nested(),
    );
    for segment in segments {
        let tokens =
            crate::ir::CommandTokens::from_segmented(&source, semantics.lexer_config, &segment);
        let Some(head) = tokens.argv.first() else {
            continue;
        };
        let binding = realm.invocation_at_source("", head.start());
        let Some((_, original)) = binding.original_recorded_command() else {
            continue;
        };
        // The lifted geometry is genuine original syntax. Its own source
        // issuer below validates the complete vector and lookup horizon.
        for nested in crate::word_subst::checked_lifted_calls(&original, semantics.lexer_config)
            .unwrap_or_default()
        {
            if let Some(nested) = nested.tokens {
                if let Some(head) = nested.argv.first() {
                    collect_original_command_writes(head.start(), semantics, metadata, out);
                }
            }
        }
        collect_original_command_writes(head.start(), semantics, metadata, out);
    }
}

fn collect_original_command_writes(
    offset: u32,
    semantics: UndefSuppressionSemantics<'_>,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    out: &mut FxHashSet<String>,
) {
    let Some(realm) = semantics.analysis.retained_command_realm() else {
        return;
    };
    let binding = realm.invocation_at_source("", offset);
    let Some((_, tokens)) = binding.original_recorded_command() else {
        return;
    };
    let Some(footprint) = binding.original_materialized_footprint(
        &tokens,
        semantics.source,
        semantics.context.commands(),
        Some(metadata),
    ) else {
        return;
    };
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_tokens(
            semantics.source,
            semantics.analysis,
            &tokens,
        )
    else {
        return;
    };
    for name in footprint.invocation_writes(&words).names {
        let root = tcl_syntax::naming::split_array_name_braced(&name, true).0;
        if !root.is_empty() {
            out.insert(root.to_owned());
        }
    }
}

/// Names written by a [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`]
/// command whose trailing words are all static literals.
///
/// The lowering only inlines the single-word `eval {script}` shape; a
/// multi-word `eval set l2 hello` stays a `Statement::Barrier`, so SSA sees
/// no definition of `l2` and a later `puts $l2` reads a version-0 origin.
/// Reconstructing the `Tcl_ConcatObj` join here recovers the write without
/// claiming anything the barrier does not already guarantee — name-level and
/// suppress-only, exactly like [`collect_expr_cmd_sub_writes`].
///
/// Gated on [`tcl_registry::BodyKind::Plain`], which is the registry's own
/// record of *whose frame the body runs in*.  `eval` is `Plain` — its script
/// runs in the caller's own frame, so its writes are this function's writes.
/// `uplevel`, `namespace eval`, `namespace inscope`, and `interp eval` are
/// all `Structural`: their scripts write somewhere else entirely, and
/// tclsh8.6.14/9.0.4 confirm the difference —
/// `proc p {} {uplevel 1 set x 5; puts $x}` errors `can't read "x"` because
/// the `set` landed in the *caller's* frame. Suppressing on those would hide
/// a real read-before-set.
fn collect_script_concat_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    semantics: UndefSuppressionSemantics<'_>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.ssa.blocks.get(&bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            if !matches!(statement.statement, crate::ir::Statement::Barrier { .. }) {
                continue;
            }
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, index).source_tokens()
            else {
                continue;
            };
            let Some(words) =
                crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                    semantics.source,
                    semantics.analysis,
                    tokens,
                )
            else {
                continue;
            };
            let same_frame_concat = words.with_source_schema(semantics.context, |schema| {
                schema
                    .semantics
                    .traits
                    .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
                    && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                    && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            }) == Some(true);
            if !same_frame_concat {
                continue;
            }
            let Some(first) = words.roles().and_then(|roles| {
                roles.iter().find_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::Body).then_some(index)
                })
            }) else {
                continue;
            };
            if words.with_source_schema(semantics.context, |schema| {
                schema.authored_source_script_timing_at(first)
            }) != Some(Some(tcl_registry::ScriptTiming::SameInvocation))
            {
                continue;
            }
            let Some(metadata) = fu.invocation_metadata_context(semantics.context.commands())
            else {
                continue;
            };
            let Some(input) = metadata.source_analysis_input() else {
                continue;
            };
            if semantics.analysis.resolved_input.as_ref() != Some(input)
                || input.lexer_config().normalized() != semantics.lexer_config.normalized()
            {
                continue;
            }
            let Some(binding) = tokens.source_binding.as_ref() else {
                continue;
            };
            let Some(footprint) = binding.original_materialized_footprint(
                tokens,
                semantics.source,
                semantics.context.commands(),
                Some(metadata),
            ) else {
                continue;
            };
            let tail = &words.arguments()[first..];
            let values: Option<Vec<_>> = tail
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .collect();
            let script = if let Some(values) = values {
                match words.dialect().and_then(|dialect| dialect.native_family) {
                    Some(tcl_dialect::model::Family::Jim) => {
                        tcl_syntax::list::concat_bytes_jim(values)
                    }
                    Some(
                        tcl_dialect::model::Family::Tcl
                        | tcl_dialect::model::Family::F5Tcl
                        | tcl_dialect::model::Family::F5Irules,
                    ) => tcl_syntax::list::concat_bytes(values),
                    None => continue,
                }
            } else {
                // Only the genuine braced first value supplies unchanged prefix
                // text when later values are unknown; it is lexical suppression,
                // not a completed write or an entered body.
                let Some(word) = words
                    .operands()
                    .get(first)
                    .and_then(Option::as_ref)
                    .and_then(
                        crate::registry_invocation::source_structure::OriginalOperandSource::word,
                    )
                    .filter(|word| word.group().kind == tcl_lexer::WordKind::Braced)
                else {
                    continue;
                };
                let Some(value) = tail
                    .first()
                    .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                else {
                    continue;
                };
                if word.group().expand {
                    continue;
                }
                value.to_vec()
            };
            let Ok(script) = std::str::from_utf8(&script) else {
                continue;
            };
            out.extend(footprint.script_writes(script).names);
        }
    }
    out
}

/// `dict with` / `dict update` key-aware suppression: record the dict-var
/// names and, when the dict value is a same-block literal (or an
/// interprocedurally-propagated SCCP const), the variables the registry's
/// plan binds from it on entry — each key `dict with` finds, each `dict
/// update` variable whose key the dictionary holds.  A value that resolves
/// to neither marks the dict shape unknown.
fn harvest_dict_with_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    s: &mut UndefSuppression,
    rules: tcl_syntax::word_rules::WordValueRules,
    registry: &tcl_registry::CommandRegistry,
) {
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let Some(tokens) = stmt.tokens() else {
                continue;
            };
            let Some(advice) =
                crate::registry_invocation::declaration_dictionary_scope_advice(registry, tokens)
            else {
                continue;
            };
            let args = &advice.arguments;
            s.has_dict_with = true;
            let Some(dict_var) = args
                .get(advice.plan.dictionary_argument)
                .and_then(Option::as_deref)
            else {
                s.dict_with_any_unknown = true;
                continue;
            };
            let dvar = crate::naming::split_array_name_braced(dict_var, true)
                .0
                .to_string();
            if dvar.is_empty() {
                s.dict_with_any_unknown = true;
                continue;
            }
            s.dict_vars.insert(dvar.clone());
            let literal = dictionary_suppression_literal(fu, bn, idx, &dvar, tokens, registry);
            match literal {
                Some(v) => {
                    let elems = crate::tcl_expr_eval::split_tcl_list(&v, rules);
                    match &advice.plan.bindings {
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::Pairs(pairs) => {
                            let present: HashSet<&str> =
                                elems.iter().step_by(2).map(String::as_str).collect();
                            for &(key, variable) in pairs {
                                if args
                                    .get(key)
                                    .and_then(Option::as_deref)
                                    .is_some_and(|key| present.contains(key))
                                    && let Some(variable) =
                                        args.get(variable).and_then(Option::as_deref)
                                {
                                    let variable =
                                        crate::naming::split_array_name_braced(variable, true)
                                            .0
                                            .to_string();
                                    if !variable.is_empty() {
                                        s.dict_with_known_keys.insert(variable);
                                    }
                                }
                            }
                        }
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::AllKeys(path)
                            if path.is_empty() =>
                        {
                            for (i, key) in elems.into_iter().enumerate() {
                                if i % 2 == 0 {
                                    s.dict_with_known_keys.insert(key);
                                }
                            }
                        }
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::AllKeys(_) => {
                            s.dict_with_any_unknown = true;
                        }
                    }
                }
                None => s.dict_with_any_unknown = true,
            }
        }
    }
}

/// Resolve only the dictionary value read by this original invocation.
fn dictionary_suppression_literal(
    fu: &crate::compilation_unit::FunctionUnit,
    bn: BlockId,
    idx: usize,
    dvar: &str,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    use crate::ir::Statement;

    // Resolve the dict's value to harvest its keys.  Prefer the SCCP
    // CONST of the SPECIFIC version read by this dict-with (so
    // interprocedurally-propagated literals — a caller passing `{}` —
    // are honoured), falling back to a same-block literal `set`.  A
    // known value (even empty) harvests its keys; only a value that
    // resolves to neither marks the dict shape unknown.
    let read = fu.ssa.var_symbol_at(bn, idx, dvar).and_then(|symbol| {
        fu.ssa
            .blocks
            .get(&bn)?
            .statements
            .get(idx)?
            .uses
            .get(&symbol)
            .copied()
            .map(|version| (symbol, version))
    });
    let mut literal = read.and_then(|(symbol, version)| {
        match fu.diagnostic_value_facts().values().get(&(symbol, version)) {
            Some(crate::analyses::LatticeValue::Const(crate::analyses::ConstValue::String(
                value,
            ))) => Some(value.clone()),
            _ => None,
        }
    });
    if literal.is_none()
        && let Some((symbol, version)) = read
    {
        // A literal fallback must define this exact reaching SSA value.
        // Equal source spellings cannot donate another alias or lifetime.
        literal = fu.ssa.blocks.get(&bn).and_then(|data| {
            data.statements[..idx].iter().rev().find_map(|statement| {
                if statement.defs.get(&symbol) != Some(&version) {
                    return None;
                }
                match &statement.statement {
                    Statement::AssignConst { value, .. } => Some(value.clone()),
                    _ => None,
                }
            })
        });
    }
    if literal.is_none()
        && let Some(binding) = tokens.source_binding.as_ref()
        && let Some(site) = binding.invocation_site()
        && let Some(report) = binding.declaration_flow_report(registry)
    {
        literal = report
            .conditional_handler_literal(site, dvar)
            .map(str::to_owned);
    }
    // A represented constant cannot survive a missing original
    // dictionary read, even if its SSA slot still has the old value.
    if let Some((symbol, _)) = read
        && crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, idx)
            .source_tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.invocation_variable_reads.as_ref())
            .is_some_and(|reads| {
                reads.native_reads.iter().any(|access| {
                    crate::var_resolve::canonical_binding_value_key(&access.place).as_ref()
                        == Some(fu.ssa.cell_key(symbol))
                        && access
                            .variable_context
                            .closed_contents_presence(&access.place)
                            != Some(crate::var_resolve::ContentsPresence::Defined)
                })
            })
    {
        literal = None;
    }
    literal
}

/// Release-, dialect-, and registry-shaped inputs used while reconstructing
/// read-before-set suppression from detached expression and list text.
#[derive(Clone, Copy)]
pub(super) struct UndefSuppressionSemantics<'a> {
    pub dialect: Option<SurfaceQuery<'a>>,
    pub rules: tcl_syntax::word_rules::WordValueRules,
    pub lexer_config: tcl_lexer::LexerConfig,
    pub source: &'a str,
    pub analysis: &'a crate::analyser::AnalysisResult,
    pub context: &'a tcl_registry::model::ContextRegistry,
    /// The module's procedures, whose transfer summaries say what a call to
    /// one does to the places it names.
    pub module: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
}

pub(super) fn build_undef_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    semantics: UndefSuppressionSemantics<'_>,
) -> UndefSuppression {
    let UndefSuppressionSemantics {
        dialect,
        rules,
        lexer_config,
        module,
        ..
    } = semantics;
    let registry = semantics.context.commands();
    let commands = registry;
    let call_steps = call_steps(fu, considered, module, commands);
    let UndefIndexMaps {
        phi_def,
        phi_block,
        killed,
        may_defs,
    } = build_phi_undef_index(&fu.ssa, considered, registry, &call_steps);
    let exists_guards = collect_existence_guards(fu, registry, lexer_config);
    // Phi versions that can reach an undef origin on some executable path —
    // a statement read of one is read-before-set. The per-use existence
    // guard + suppression set still apply in the emitter loop.
    let undef_ctx = PhiUndefCtx {
        registry: commands,
        phi_def: &phi_def,
        phi_block: &phi_block,
        killed: &killed,
        may_defs: &may_defs,
        considered,
        executable_edges: fu.diagnostic_value_facts().executable_edges(),
        exists_guards: &exists_guards,
        initial_global,
        global_aliases,
        dialect,
        ssa: &fu.ssa,
        preserved: &fu.sccp.preserved,
    };
    let mut can_undef: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)> =
        FxHashSet::default();
    // One memo for the whole sweep and the loop-entry fixpoint below: both run
    // against `undef_ctx` unchanged, so an answer found for one key stands for
    // every other query (issue #2021 — without it the sweep re-walks every
    // path through the phi graph).
    let mut memo = PhiUndefMemo::default();
    for key in phi_def.keys().chain(may_defs.keys()) {
        if phi_can_undef(&key.0, key.1, &undef_ctx, &mut memo) {
            can_undef.insert(key.clone());
        }
    }
    // A definition its statement left untouched — a `regexp` that did not
    // match, a `scan` whose input ran out, any declared `Preserve` — holds
    // its prior version, so it is undefined exactly when that version can
    // be: a read of it is then a read before set.
    let mut preserved_undef: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)> =
        FxHashSet::default();
    for &(symbol, version) in fu.sccp.preserved.keys() {
        let name = fu.ssa.cell_key(symbol);
        if phi_can_undef(name, version, &undef_ctx, &mut memo) {
            can_undef.insert((name.to_owned(), version));
            preserved_undef.insert((name.to_owned(), version));
        }
    }
    for (block, markers) in &fu.ssa.value_clobbers {
        if !considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let name = fu.ssa.cell_key(symbol);
                if phi_can_undef(name, fresh, &undef_ctx, &mut memo) {
                    can_undef.insert((name.clone(), fresh));
                }
            }
        }
    }
    let loop_entry_only_undef =
        build_loop_entry_only_undef(fu, &can_undef, &undef_ctx, rules, &mut memo);
    let mut s = UndefSuppression {
        cmd_sub_writes: collect_expr_cmd_sub_writes(fu, considered, semantics),
        script_concat_writes: collect_script_concat_writes(fu, considered, semantics),
        unseen_call_sites: collect_unseen_call_sites(fu, considered, initial_global, semantics),
        killed,
        call_steps,
        can_undef,
        preserved_undef,
        loop_entry_only_undef,
        ..Default::default()
    };
    harvest_dict_with_suppression(fu, considered, &mut s, rules, registry);

    // Names with a concrete (version > 0) statement or phi definition — a
    // dict-with scope never suppresses these (they are genuinely set).
    if s.has_dict_with {
        for &bn in considered {
            let Some(sb) = fu.ssa.blocks.get(&bn) else {
                continue;
            };
            for st in &sb.statements {
                for (&n, v) in &st.defs {
                    if *v > 0 {
                        s.explicitly_defined.insert(fu.ssa.var_name(n).to_owned());
                    }
                }
            }
            for phi in &sb.phis {
                if phi.version > 0 {
                    s.explicitly_defined
                        .insert(fu.ssa.var_name(phi.name).to_owned());
                }
            }
        }
    }

    s.alias_tails = collect_qualified_variable_alias_tails(fu, considered, semantics.context);
    s
}

/// Build the [`UndefSuppression::loop_entry_only_undef`] map: loop-header phi
/// versions whose sole undef origin is the loop's zero-trip entry edge.
///
/// For each natural loop (built over the SCCP-executable subgraph, so a
/// provably-dead loop body never forms a loop) whose header carries a phi in
/// `can_undef`, the phi qualifies when *every* back-edge (in-loop
/// predecessor) operand is itself defined — i.e. the loop body assigns the
/// variable on each iteration and the only way the phi is undef is by skipping
/// the loop entirely. A provably-empty `foreach` (all iterator lists are
/// empty literals) is excluded: its body never runs, so tclsh always errors,
/// and the read must keep firing.
fn build_loop_entry_only_undef(
    fu: &crate::compilation_unit::FunctionUnit,
    can_undef: &FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    ctx: &PhiUndefCtx<'_>,
    rules: tcl_syntax::word_rules::WordValueRules,
    memo: &mut PhiUndefMemo,
) -> FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), FxHashSet<String>> {
    let mut out: FxHashMap<
        (crate::var_resolve::VariableCellKey, crate::ssa::Version),
        FxHashSet<String>,
    > = FxHashMap::default();
    if can_undef.is_empty() {
        return out;
    }
    let forest = crate::loops::build_loop_forest(&fu.cfg, &fu.ssa, ctx.considered);
    // Fixpoint over the loop forest so *nested* accumulators converge: an inner
    // loop whose body defines the variable is itself loop-entry-only-undef, so
    // for the enclosing loop its exit operand counts as defined (we assume both
    // loops run). Innermost loops are marked first; a pass that marks nothing
    // new terminates. Bounded by the forest size (each pass marks ≥1 phi or
    // stops), so at most `loops.len()` passes.
    loop {
        let mut changed = false;
        for natural in &forest.loops {
            let Some(header_id) = fu.cfg.block_id(&natural.header) else {
                continue;
            };
            // A provably-empty `foreach` runs zero times: its body-assigned
            // variables are never set, so tclsh always errors — keep firing.
            if foreach_header_provably_empty(fu, header_id, rules) {
                continue;
            }
            let body_blocks: FxHashSet<String> = natural.blocks.iter().cloned().collect();
            let Some(ssa_header) = fu.ssa.blocks.get(&header_id) else {
                continue;
            };
            for phi in &ssa_header.phis {
                let name = fu.ssa.cell_key(phi.name).clone();
                let key = (name.clone(), phi.version);
                if out.contains_key(&key) || !can_undef.contains(&key) {
                    continue;
                }
                // The phi is "entry-only undef" when no *back-edge* operand can
                // be undef: a body predecessor that still merges an unset value
                // is a conditional-def-in-body case (or a break/continue before
                // the set) that tclsh can still leave unset — those keep firing.
                // An operand already proven loop-entry-only-undef (a nested
                // loop's result) counts as defined here.
                let entry_only = phi.incoming.iter().all(|(&pred, &ver_in)| {
                    // Only in-loop (back-edge) predecessors gate the verdict;
                    // the pre-header entry edge carries the zero-trip undef we
                    // assume away. Non-executable predecessors never read.
                    if !ctx.considered.contains(&pred) {
                        return true;
                    }
                    if !body_blocks.contains(fu.cfg.block_name(pred)) {
                        return true;
                    }
                    let binding = fu.ssa.binding_version(phi.name, ver_in);
                    if out.contains_key(&(name.clone(), binding)) {
                        return true;
                    }
                    !phi_can_undef(&name, ver_in, ctx, memo)
                });
                if entry_only {
                    out.insert(key, body_blocks.clone());
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // A call to code the module cannot see changes value facts without
    // introducing a new binding.
    // Carry the established after-loop binding proof through their fresh value
    // versions, just as `phi_can_undef` follows that same binding lineage.
    for (block, markers) in &fu.ssa.value_clobbers {
        if !ctx.considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let binding = fu.ssa.binding_version(symbol, fresh);
                let name = fu.ssa.cell_key(symbol).clone();
                if let Some(body) = out.get(&(name.clone(), binding)).cloned() {
                    out.insert((name, fresh), body);
                }
            }
        }
    }
    out
}

/// True when `header_id` is the header of a `foreach` / `lmap` / `dict for`
/// whose iterator lists are *all* statically-empty literals (so the loop body
/// provably never runs). The synthetic iterator-binding node placed at the
/// loop header records each iterator list's text in `args`; an argument splits
/// to zero elements only for an empty literal (a `$`/`[` substitution splits
/// to a single opaque element, a non-empty literal to ≥1 element).
fn foreach_header_provably_empty(
    fu: &crate::compilation_unit::FunctionUnit,
    header_id: BlockId,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    use crate::ir::Statement;
    let Some(block) = fu.cfg.blocks.get(&header_id) else {
        return false;
    };
    block.statements.iter().any(|stmt| {
        matches!(
            stmt,
            Statement::Call { foreach_groups: Some(_), args, .. }
                if !args.is_empty()
                    && args
                        .iter()
                        .all(|a| crate::tcl_expr_eval::split_tcl_list(a, rules).is_empty())
        )
    })
}

/// Local-alias tail names in the retained source declaration for a
/// qualified namespace-variable operand. The actual context admits the
/// command metadata; this name-level suppression establishes no executed
/// namespace link, cell identity, frame or stored contents.
fn collect_qualified_variable_alias_tails(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    context: &tcl_registry::model::ContextRegistry,
) -> FxHashSet<String> {
    let mut tails = FxHashSet::default();
    for statement in considered
        .iter()
        .filter_map(|id| fu.cfg.blocks.get(id))
        .flat_map(|block| &block.statements)
    {
        let Some(invocation) = crate::registry_invocation::resolved_statement_invocation_in_context(
            context, statement,
        ) else {
            continue;
        };
        let Some(transitions) = invocation.facts.state_transitions.declared() else {
            continue;
        };
        for fact in transitions.facts() {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            let tcl_registry::VariableAliasTarget::CurrentNamespace { variable } = &alias.target
            else {
                continue;
            };
            if variable.literal().is_some_and(|name| name.contains("::"))
                && let Some(local) = alias.local.literal()
            {
                tails.insert(local.to_owned());
            }
        }
    }
    tails
}

/// Collect every variable name defined anywhere in `cfg`.
///
/// Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
pub(super) fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::split_array_name_braced(name, true).0;
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Exact cross-procedure cells used only for diagnostic suppression.
#[derive(Default)]
pub(super) struct DiagnosticCellFacts {
    pub known_defined: HashSet<crate::var_resolve::VariableCellKey>,
    pub externally_read: HashSet<crate::var_resolve::VariableCellKey>,
}

/// Selected successful definitions at the original statement boundary.
/// Alias registration and destruction are not value-producing writes.
pub(super) fn original_definition_places(
    fu: &crate::compilation_unit::FunctionUnit,
    block: BlockId,
    index: usize,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<crate::place::Place> {
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return Vec::new();
    };
    let Some(tokens) =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
    else {
        return Vec::new();
    };
    let Some(binding) = &tokens.source_binding else {
        return Vec::new();
    };
    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    ) else {
        return Vec::new();
    };
    if normal
        .variable_traits()
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
        || normal.variable_alias_transitions().next().is_some()
    {
        return Vec::new();
    }
    normal
        .definition_places(&binding.variable_context, registry)
        .into_iter()
        .filter(|place| {
            if place.dynamic {
                return false;
            }
            if !place.observed {
                return true;
            }
            // A destination before variable callbacks is only a possible
            // definition. Retain an observed store's exact normal physical
            // successor; an error-only callback supplies no such world.
            binding.normal_variable_continuation().is_some_and(|after| {
                after.captured_contents_presence(place)
                    == crate::var_resolve::ContentsPresence::Defined
                    && place.cell.as_ref().is_some_and(|cell| {
                        cell.generation != crate::place::CellGeneration::Unknown
                            && after
                                .generations
                                .get(&crate::var_resolve::cell_key(place))
                                .copied()
                                .unwrap_or(crate::place::CellGeneration::Incoming)
                                == cell.generation
                    })
            })
        })
        .collect()
}

/// Consensus physical cell of the same original read, independently of SSA values.
pub(super) fn original_read_cell(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::var_resolve::VariableCellKey> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
        return None;
    }
    let mut selected = None;
    for context in access.context_alternatives() {
        let place = access.place_in_context(context, registry);
        let cell = crate::var_resolve::canonical_place_key(&place)?;
        if selected.as_ref().is_some_and(|previous| previous != &cell) {
            return None;
        }
        selected = Some(cell);
    }
    selected
}

/// Wrong-kind scalar read of an existing array, not an undefined contents read.
/// Every original physical alternative must retain the same live root kind.
pub(super) fn original_read_is_live_array_scalar(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    access.context_residual() == crate::command_binding::SourceVariableReadResidual::Closed
        && !access.context_alternatives().is_empty()
        && access.context_alternatives().iter().all(|context| {
            let place = access.place_in_context(context, registry);
            place.index.is_none()
                && context.root_contents_kind(&place)
                    == Some(crate::var_resolve::RootContentsKind::Array)
        })
}

/// Original occurrence-only wrong-kind advice, independently of SSA versions.
pub(super) fn original_array_scalar_read_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let mut found = false;
    for (&block, body) in &fu.ssa.blocks {
        for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let original = fu.abs_span(access.source.span);
                if original.start() <= span.start() && span.end() <= original.end() {
                    found = true;
                    if !original_read_is_live_array_scalar(access, registry) {
                        return false;
                    }
                }
            }
        }
    }
    found
}

/// Exact attempted cell/version at one original diagnostic read occurrence.
/// Undefined versions remain diagnosable without a completed value receipt.
/// Conflicting source sites or physical cells cannot borrow a display label.
pub(super) fn original_read_occurrence_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
) -> Option<crate::def_use::SsaValueKey> {
    let mut selected = None;
    for (&block, body) in &fu.ssa.blocks {
        for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let original = fu.abs_span(access.source.span);
                if original.start() > span.start() || span.end() > original.end() {
                    continue;
                }
                let (symbol, version) =
                    view.read_occurrence_reference(&access.source, &access.original_spelling)?;
                let value = (fu.ssa.cell_key(symbol).clone(), version);
                if selected.as_ref().is_some_and(|previous| previous != &value) {
                    return None;
                }
                selected = Some(value);
            }
        }
    }
    selected
}

/// Original physical cells read at this operation for one represented SSA slot.
pub(super) fn original_read_cells_at(
    fu: &crate::ssa::SsaFunction,
    point: (BlockId, usize),
    value_cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<crate::var_resolve::VariableCellKey> {
    crate::ssa::SsaSourceView::at_statement(fu, point.0, point.1)
        .source_tokens()
        .into_iter()
        .flat_map(|tokens| &tokens.variable_accesses)
        .filter(|access| {
            fu.read_reference_at(point.0, point.1, &access.source, &access.original_spelling)
                .is_some_and(|read| fu.cell_key(read.symbol) == value_cell)
        })
        .filter_map(|access| original_read_cell(access, registry))
        .collect()
}

/// Original root-special-variable key of the exact cell defined here.
pub(super) fn special_variable_definition(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    let block = fu.cfg.block_id(&definition.block)?;
    let index = usize::try_from(definition.statement_index).ok()?;
    original_definition_places(fu, block, index, registry)
        .iter()
        .find_map(|place| {
            (crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(cell))
                .then(|| {
                    crate::var_resolve::root_namespace_variable_simple_name(place)
                        .map(str::to_owned)
                })
                .flatten()
        })
}

/// A selected root definition at an original diagnostic source occurrence.
/// A declaration-only name without an actual definition supplies no effect.
pub(super) fn special_variable_definition_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    let metadata = fu.invocation_metadata_context(registry)?;
    fu.cfg.blocks.iter().find_map(|(&block, data)| {
        data.statements.iter().enumerate().find_map(|(index, _)| {
            let tokens =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()?;
            let binding = tokens.source_binding.as_ref()?;
            let normal =
                crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                    registry,
                    Some(metadata),
                    tokens,
                )?;
            if normal
                .variable_traits()
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
                || normal.variable_alias_transitions().next().is_some()
            {
                return None;
            }
            normal
                .written_definition_places(&binding.variable_context, registry)
                .into_iter()
                .find_map(|(argument, place)| {
                    let original =
                        fu.abs_span(tokens.words().get(argument.checked_add(1)?)?.source().span);
                    if original.start() <= span.start() && span.end() <= original.end() {
                        crate::var_resolve::root_namespace_variable_simple_name(&place)
                            .map(str::to_owned)
                    } else {
                        None
                    }
                })
        })
    })
}

/// Namespace writes retain their selected cell and physical lifetime.
/// Written command names and displayed paths cannot donate a definition.
pub(super) fn globals_written_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    cu.procedures
        .values()
        .flat_map(|fu| {
            fu.ssa.blocks.iter().flat_map(move |(&block, data)| {
                (0..data.statements.len()).flat_map(move |index| {
                    let entered = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                        .source_tokens()
                        .and_then(|tokens| tokens.source_binding.as_ref())
                        .is_some_and(|binding| {
                            binding.runtime_reachability()
                                == crate::command_binding::SourceRuntimeReachability::Reached
                        });
                    entered
                        .then(|| original_definition_places(fu, block, index, registry))
                        .into_iter()
                        .flatten()
                        .filter(crate::place::Place::is_global)
                        .filter_map(|place| crate::var_resolve::canonical_place_key(&place))
                })
            })
        })
        .collect()
}

/// Whether the exact original definition is visible to a retained external read.
pub(super) fn definition_has_cell_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    value_cell: &crate::var_resolve::VariableCellKey,
    facts: &HashSet<crate::var_resolve::VariableCellKey>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(block) = fu.cfg.block_id(&definition.block) else {
        return false;
    };
    let Ok(index) = usize::try_from(definition.statement_index) else {
        return false;
    };
    original_definition_places(fu, block, index, registry)
        .iter()
        .any(|place| {
            crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(value_cell)
                && crate::var_resolve::canonical_place_key(place)
                    .is_some_and(|cell| facts.contains(&cell))
        })
}

/// Every captured read of this represented value must select an advised physical cell.
pub(super) fn read_has_cell_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    point: (BlockId, usize),
    value_cell: &crate::var_resolve::VariableCellKey,
    facts: &HashSet<crate::var_resolve::VariableCellKey>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(tokens) =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, point.0, point.1).source_tokens()
    else {
        return false;
    };
    let mut seen = false;
    for access in &tokens.variable_accesses {
        let place = access.place_in_context(&access.variable_context, registry);
        if crate::var_resolve::canonical_binding_value_key(&place).as_ref() != Some(value_cell) {
            continue;
        }
        seen = true;
        if !original_read_cell(access, registry).is_some_and(|cell| facts.contains(&cell)) {
            return false;
        }
    }
    seen
}

/// Original namespace reads retain exact table, cell and lifetime identity.
/// The consumer compares these keys before projecting diagnostic labels.
pub(super) fn globals_read_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    let mut cells = HashSet::new();
    for fu in cu.procedures.values() {
        for (&block, data) in &fu.ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                cells.extend(tokens.variable_accesses.iter().filter_map(|access| {
                    let cell = original_read_cell(access, registry)?;
                    cell.namespace_membership_for_advice()
                        .is_some()
                        .then_some(cell)
                }));
                for place in tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_variable_reads.as_ref())
                    .into_iter()
                    .flat_map(|reads| &reads.native_reads)
                    .map(|read| &read.place)
                {
                    if place.is_global()
                        && let Some(key) = crate::var_resolve::canonical_place_key(place)
                    {
                        cells.insert(key);
                    }
                }
            }
        }
    }
    cells
}

#[cfg(test)]
mod exact_cell_diagnostic_tests {
    use super::*;
    use crate::command_binding::SourceNamespaceKey;
    use crate::var_resolve::VariableCellKey;

    #[test]
    fn qualified_alias_tail_advice_uses_retained_context_over_catalogue_profile() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // This checks authored metadata availability and source-name suppression,
        // without claiming native namespace-link execution or cell identity.
        let catalogue = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let no_tcl = tcl_registry::model::ingress::static_context_for("f5-bigip")
            .with_command_store(std::sync::Arc::clone(catalogue.commands()));
        assert!(std::sync::Arc::ptr_eq(
            catalogue.commands(),
            no_tcl.commands()
        ));
        assert!(
            catalogue
                .context()
                .resolve_spec(catalogue.commands(), "variable")
                .is_some()
        );
        assert!(
            no_tcl
                .context()
                .resolve_spec(no_tcl.commands(), "variable")
                .is_none()
        );
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "variable ns::tail; variable unqualified",
            catalogue.commands(),
            false,
            "tcl8.6",
        );
        let function = &unit.top_level;
        let considered = function.cfg.blocks.keys().copied().collect();
        assert_eq!(
            collect_qualified_variable_alias_tails(function, &considered, catalogue),
            FxHashSet::from_iter(["tail".to_owned()]),
            "positive original qualified alias-tail metadata",
        );
        assert!(collect_qualified_variable_alias_tails(function, &considered, &no_tcl).is_empty());
        let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc variable args {}; variable ns::tail",
            catalogue.commands(),
            false,
            "tcl8.6",
        );
        let considered = replaced.top_level.cfg.blocks.keys().copied().collect();
        assert!(
            collect_qualified_variable_alias_tails(&replaced.top_level, &considered, catalogue)
                .is_empty(),
            "an authored replacement cannot borrow namespace-variable metadata",
        );
    }

    fn occurrence_span(source: &str, spelling: &str) -> tcl_lexer::Span {
        let start = source.rfind(spelling).expect("original occurrence");
        tcl_lexer::Span::new(
            u32::try_from(start).expect("bounded source offset"),
            u32::try_from(start + spelling.len()).expect("bounded source end"),
        )
    }

    #[test]
    fn original_array_scalar_advice_does_not_revive_retired_roots_or_elements() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        for (source, spelling, wrong_kind) in [
            ("proc f {} {set a(k) 1; set copy $a}", "$a", true),
            ("proc f {} {set a(k) 1; unset a; set copy $a}", "$a", false),
            (
                "proc f {} {set a(k) 1; unset a(k); set copy $a(k)}",
                "$a(k)",
                false,
            ),
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
                source, registry, false, "tcl8.6",
            );
            let function = cu.procedures.values().next().expect("original procedure");
            assert_eq!(
                original_array_scalar_read_at_span(
                    function,
                    occurrence_span(source, spelling),
                    registry,
                ),
                wrong_kind,
                "{source}"
            );
        }
    }

    #[test]
    fn special_variable_advice_matches_its_original_output_word() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let source = "proc f {} {scan {a b} {%s %s} local ::auto_path}";
        let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = cu.procedures.values().next().expect("original procedure");
        assert_eq!(
            special_variable_definition_at_span(
                function,
                occurrence_span(source, "::auto_path"),
                registry,
            ),
            Some("auto_path".into())
        );
        assert_eq!(
            special_variable_definition_at_span(
                function,
                occurrence_span(source, "local"),
                registry,
            ),
            None
        );
    }

    fn footprint_inputs(
        source: &str,
    ) -> (
        crate::compilation_unit::CompilationUnit,
        crate::analyser::AnalysisResult,
        crate::analyser::ResolvedAnalysisInput,
    ) {
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        (unit, analysis, input)
    }

    fn footprint_semantics<'a>(
        source: &'a str,
        analysis: &'a crate::analyser::AnalysisResult,
        input: &'a crate::analyser::ResolvedAnalysisInput,
    ) -> super::UndefSuppressionSemantics<'a> {
        super::UndefSuppressionSemantics {
            dialect: Some(
                input
                    .borrowed_context_registry()
                    .context()
                    .authoring_query(),
            ),
            rules: tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
            lexer_config: input.lexer_config(),
            source,
            analysis,
            context: input.borrowed_context_registry(),
        }
    }

    #[test]
    fn materialized_suppression_keeps_original_aliases_and_literal_names() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        // The real diagnostic suppression consumer receives possible names,
        // never a Native write, successful evaluation or child argv receipt.
        for (source, expected) in [
            (
                "proc f {} {eval set {café(open} VALUE; puts ${café(open}}",
                Some("café(open"),
            ),
            (
                "interp alias {} emit {} eval set {$literal}; proc f {} {emit VALUE; puts ${$literal}}",
                Some("$literal"),
            ),
            (
                "rename set moved; interp alias {} write {} moved café; proc f {} {eval write VALUE; puts $café}",
                Some("café"),
            ),
            (
                "proc set args {}; proc f {} {eval set hidden VALUE; puts $hidden}",
                None,
            ),
        ] {
            let (unit, analysis, input) = footprint_inputs(source);
            let function = unit.function("::f").unwrap();
            let considered = function.ssa.blocks.keys().copied().collect();
            let names = super::collect_script_concat_writes(
                function,
                &considered,
                footprint_semantics(source, &analysis, &input),
            );
            if let Some(expected) = expected {
                assert!(names.contains(expected), "{source}: {names:?}");
            } else {
                assert!(!names.contains("hidden"), "known replacement: {names:?}");
            }
        }
    }

    #[test]
    fn expression_suppression_uses_each_original_child_lookup_horizon() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        for (source, expected) in [
            (
                "proc f {} {set result [expr {[catch {set {café(open} VALUE} status] || $status}]}",
                Some("café(open"),
            ),
            (
                "interp alias {} assign {} set {$literal}; proc f {} {set result [expr {[assign VALUE]}]}",
                Some("$literal"),
            ),
            (
                "proc f {} {set result [expr {[set x VALUE] + [rename set replaced]}]}",
                Some("x"),
            ),
            (
                "proc f {} {if {[catch {set x VALUE} first]} {} elseif {[set y VALUE]} {}}",
                Some("x"),
            ),
            (
                "proc f {} {if {[catch {set x VALUE} first]} {} elseif {[set y VALUE]} {}}",
                Some("y"),
            ),
            (
                "proc catch args {}; proc f {} {set result [expr {[catch {set hidden VALUE} status]}]}",
                None,
            ),
        ] {
            let (unit, analysis, input) = footprint_inputs(source);
            let function = unit.function("::f").unwrap();
            let considered = function.ssa.blocks.keys().copied().collect();
            let names = super::collect_expr_cmd_sub_writes(
                function,
                &considered,
                footprint_semantics(source, &analysis, &input),
            );
            if let Some(expected) = expected {
                assert!(names.contains(expected), "{source}: {names:?}");
            } else {
                assert!(
                    !names.contains("hidden") && !names.contains("status"),
                    "known child replacement: {names:?}"
                );
            }
        }
    }

    #[test]
    fn materialized_suppression_refuses_missing_foreign_and_stale_source_input() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        let source = "proc f {} {eval set output VALUE; puts $output}";
        let (unit, analysis, input) = footprint_inputs(source);
        let function = unit.function("::f").unwrap();
        let considered = function.ssa.blocks.keys().copied().collect();
        let semantics = footprint_semantics(source, &analysis, &input);
        assert!(
            super::collect_script_concat_writes(function, &considered, semantics)
                .contains("output")
        );
        let mut missing = function.clone();
        missing.source_metadata_input = None;
        assert!(super::collect_script_concat_writes(&missing, &considered, semantics).is_empty());
        let mut foreign = function.clone();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(super::collect_script_concat_writes(&foreign, &considered, semantics).is_empty());
        let stale = super::UndefSuppressionSemantics {
            source: "proc f {} {eval set output OTHER; puts $output}",
            ..semantics
        };
        assert!(super::collect_script_concat_writes(function, &considered, stale).is_empty());
    }

    #[test]
    fn original_definition_advice_refuses_missing_and_foreign_function_metadata() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // These are selected source/API definition places, not measured native stores.
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let source = "proc f {} {scan {a b} {%s %s} local ::auto_path}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let original = unit.procedures.values().next().unwrap();
        let span = occurrence_span(source, "::auto_path");
        let definitions = |function: &crate::compilation_unit::FunctionUnit| {
            function
                .cfg
                .blocks
                .iter()
                .flat_map(|(&block, body)| {
                    (0..body.statements.len()).flat_map(move |index| {
                        original_definition_places(function, block, index, registry)
                    })
                })
                .collect::<Vec<_>>()
        };
        assert!(definitions(original).iter().any(|place| {
            crate::var_resolve::root_namespace_variable_simple_name(place) == Some("auto_path")
        }));
        assert_eq!(
            special_variable_definition_at_span(original, span, registry),
            Some("auto_path".to_owned())
        );
        let mut missing = original.clone();
        missing.source_metadata_input = None;
        assert!(definitions(&missing).is_empty());
        assert_eq!(
            special_variable_definition_at_span(&missing, span, registry),
            None
        );
        let mut foreign = original.clone();
        let input = original.source_metadata_input().unwrap();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(definitions(&foreign).is_empty());
        assert_eq!(
            special_variable_definition_at_span(&foreign, span, registry),
            None
        );
    }

    #[test]
    fn original_existence_guards_use_retained_availability_and_refuse_missing_foreign() {
        // naming.compiler.retained-existence-metadata
        // docs/design/analysis/name-resolution-proofs/retained-existence-metadata.md
        // Conditional branch advice only; no native query result or current contents proof.
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let info = registry.get("info").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            surface: registry.get("dict").unwrap().surface,
            ..info
        });
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let profile = tcl_registry::model::ingress::static_context_for("tcl")
            .commands()
            .profile()
            .unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            "proc f {} {if {[info exists {scalar(open}]} {puts ${scalar(open}}}",
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            context,
        );
        let original = unit.procedures.values().next().unwrap();
        assert!(
            original
                .source_metadata_input()
                .unwrap()
                .has_logical_source_name_context()
        );
        assert_eq!(
            collect_existence_guards(original, &registry, config).len(),
            1
        );
        let input = original.source_metadata_input().unwrap();
        let mut unavailable = original.clone();
        unavailable.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(
                tcl_registry::model::ingress::static_context_for("tcl8.4")
                    .with_command_store(registry.snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(collect_existence_guards(&unavailable, &registry, config).is_empty());
        let mut missing = original.clone();
        missing.source_metadata_input = None;
        assert!(collect_existence_guards(&missing, &registry, config).is_empty());
        let mut foreign = original.clone();
        foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(collect_existence_guards(&foreign, &registry, config).is_empty());
    }

    #[test]
    fn original_read_killed_match_keeps_alias_cell_and_version() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let source = "proc f {} {set original 1; upvar 0 original linked; unset linked; catch {puts $original}; set unrelated 1; puts $unrelated}";
        let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = cu.procedures.values().next().expect("original procedure");
        let considered = function.ssa.blocks.keys().copied().collect();
        let (_, _, killed) = build_phi_undef_index(&function.ssa, &considered, registry);
        let original =
            original_read_occurrence_at_span(function, occurrence_span(source, "$original"))
                .expect("original read value");
        let unrelated =
            original_read_occurrence_at_span(function, occurrence_span(source, "$unrelated"))
                .expect("unrelated read value");
        assert!(killed.contains(&original), "{original:?}: {killed:?}");
        assert!(!killed.contains(&unrelated));
        assert!(!killed.contains(&(original.0.clone(), original.1 + 1)));
        let access = function
            .ssa
            .blocks
            .iter()
            .flat_map(|(&block, body)| (0..body.statements.len()).map(move |index| (block, index)))
            .find_map(|(block, index)| {
                let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                let access = view
                    .source_tokens()?
                    .variable_accesses
                    .iter()
                    .find(|access| access.original_spelling == "$original")?;
                Some((
                    view.read_reference(&access.source, &access.original_spelling)?,
                    view.read_occurrence_reference(&access.source, &access.original_spelling)?,
                ))
            })
            .expect("exact original attempted read");
        assert!(
            access.0.version.is_none(),
            "unset produced no readable value"
        );
        assert_eq!(function.ssa.cell_key(access.1.0), &original.0);
        assert_eq!(access.1.1, original.1);
    }

    #[test]
    fn original_unhandled_missing_read_has_no_unentered_suffix_value_or_contents_receipt() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {} {set original 1; upvar 0 original linked; unset linked; puts $original; set unrelated 1; puts $unrelated}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.function("::f").unwrap();
        let span = occurrence_span(source, "$unrelated");
        assert!(original_read_occurrence_at_span(function, span).is_none());
        let mut original_word_retained = false;
        for (&block, body) in &function.ssa.blocks {
            for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for word in tokens.words() {
                    if function.abs_span(word.source().span) != span {
                        continue;
                    }
                    original_word_retained = true;
                    assert!(view.read_reference(word.source(), "$unrelated").is_none());
                    assert!(
                        view.read_completion_at(word.source(), "$unrelated", registry)
                            .is_none()
                    );
                    assert!(
                        view.read_contents_presence_at(word.source(), "$unrelated", registry)
                            .is_none()
                    );
                }
            }
        }
        assert!(
            original_word_retained,
            "original unentered diagnostic source remains available"
        );
    }

    #[test]
    fn destruction_versions_do_not_kill_a_prior_read_or_supply_a_stored_value() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {} {set original 1; puts $original; upvar 0 original linked; unset linked; puts $original}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.function("::f").unwrap();
        let first_start = source.find("$original").unwrap();
        let first = original_read_occurrence_at_span(
            function,
            tcl_lexer::Span::new(
                u32::try_from(first_start).unwrap(),
                u32::try_from(first_start + "$original".len()).unwrap(),
            ),
        )
        .expect("original earlier read");
        let start = source.rfind("$original").unwrap();
        let later = original_read_occurrence_at_span(
            function,
            tcl_lexer::Span::new(
                u32::try_from(start).unwrap(),
                u32::try_from(start + "$original".len()).unwrap(),
            ),
        )
        .expect("original attempted read after destruction");
        let considered = function.ssa.blocks.keys().copied().collect();
        let (_, _, killed) = build_phi_undef_index(&function.ssa, &considered, registry);
        assert_eq!(first.0, later.0, "the alias selects the same actual cell");
        assert_ne!(first.1, later.1, "destruction has its own contents version");
        assert!(
            !killed.contains(&first),
            "an earlier readable value was not destroyed then"
        );
        assert!(killed.contains(&later));
        let destruction = function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find(|statement| !statement.destruction_defs.is_empty())
            .expect("a distinct destruction transition");
        assert!(
            destruction
                .destruction_defs
                .iter()
                .all(|symbol| destruction.defs.contains_key(symbol))
        );
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                for &symbol in &statement.destruction_defs {
                    assert!(!view.normal_store_contents_preserved(symbol, registry));
                }
            }
        }
    }

    #[test]
    fn original_read_occurrence_does_not_borrow_an_unknown_alias_cell() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {which} {set a 1; set b 2; upvar 0 $which linked; puts $linked}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.function("::f").unwrap();
        assert!(
            original_read_occurrence_at_span(function, occurrence_span(source, "$linked"))
                .is_none(),
            "a written alias label cannot identify its dynamic target cell",
        );
    }

    #[test]
    fn killed_values_supersede_exists_guards_for_scalar_element_and_return() {
        for source in [
            "proc f {x} {if {[info exists x]} {unset x; puts $x}}",
            "proc f {flag x} {if {$flag && [info exists x]} {unset x}; puts $x}",
            "proc f {flag} {set a(k) 1; if {$flag && [info exists a(k)]} {unset a(k)}; puts $a(k)}",
            "proc f {x} {if {[info exists x]} {unset x; return $x}}",
            "proc f {flag x} {if {$flag && [info exists x]} {unset x}; return $x}",
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(
                result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == crate::analyser::types::DiagCode::W210
                }),
                "{source}: {:?}",
                result.diagnostics
            );
        }
        for source in [
            "proc f {x} {if {[info exists x]} {puts $x}}",
            "proc f {x} {if {[info exists x]} {return $x}}",
            "proc f {} {set a(k) 1; if {[info exists a(k)]} {puts $a(k)}}",
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(
                result.diagnostics.iter().all(|diagnostic| {
                    diagnostic.code != crate::analyser::types::DiagCode::W210
                }),
                "{source}: {:?}",
                result.diagnostics
            );
        }
    }

    fn namespace_cell(path: tcl_core_types::ByteNamespacePath) -> VariableCellKey {
        VariableCellKey::Namespace {
            identity: SourceNamespaceKey::Native(
                tcl_runtime_api::native_compilation::NativeNamespaceContext {
                    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                        owner: 1,
                        interpreter: 1,
                    },
                    token: 1,
                    path,
                },
            ),
            simple: "argv".into(),
        }
    }

    #[test]
    fn startup_facts_require_exact_root_namespace_geometry() {
        let aliases = HashSet::from(["argv".to_owned()]);
        let root = namespace_cell(tcl_core_types::ByteNamespacePath::default());
        assert_eq!(startup_cell_binding(&root, false, &aliases), ("argv", true));
        let named = namespace_cell(tcl_core_types::ByteNamespacePath::default().with_child("::"));
        assert_eq!(
            startup_cell_binding(&named, true, &aliases),
            ("argv", false)
        );
        let local = VariableCellKey::Activation {
            identity: "actual-call".into(),
            simple: "argv".into(),
        };
        assert_eq!(startup_cell_binding(&local, true, &aliases), ("", false));
    }

    #[test]
    fn procedure_namespace_effects_require_original_selected_cells() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let original = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set ::counter 0\nproc init {} {global counter; set counter 1}\nproc read {} {global counter; return $counter}\ninit; read\nputs $counter",
            registry,
            false,
            "tcl8.6",
        );
        let written = globals_written_by_procs(&original, registry);
        let read = globals_read_by_procs(&original, registry);
        assert!(!written.is_empty(), "selected entered namespace write");
        assert!(!read.is_empty(), "selected entered namespace read");
        assert!(written.intersection(&read).next().is_some());
        let deferred = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc init {} {global counter; set counter 1}\nproc read {} {global counter; return $counter}",
            registry,
            false,
            "tcl8.6",
        );
        assert!(
            globals_written_by_procs(&deferred, registry).is_empty(),
            "source declaration metadata cannot establish normal namespace writes"
        );
        let observed = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set ::counter 0; proc changed args {error OBSERVED}; trace add variable ::counter write changed\nproc init {} {global counter; set counter 1}\ninit",
            registry,
            false,
            "tcl8.6",
        );
        assert!(
            globals_written_by_procs(&observed, registry).is_empty(),
            "an erroring write observer prevents the successful-definition receipt"
        );
        let replaced = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "rename global native_global\nproc global args {}\nproc init {} {global counter; set counter 1}\nproc read {} {set counter LOCAL; return $counter}\ninit; read; puts $counter",
            registry,
            false,
            "tcl8.6",
        );
        assert!(globals_written_by_procs(&replaced, registry).is_empty());
        assert!(globals_read_by_procs(&replaced, registry).is_empty());
    }

    #[test]
    fn namespace_read_advice_does_not_suppress_a_same_named_local() {
        let source = "proc reader {} {return $::counter}\nproc local {} {set counter FIRST; set counter SECOND}\n";
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == crate::analyser::types::DiagCode::W211
                    && diagnostic.message.contains("counter")
            }),
            "{:?}",
            result.diagnostics
        );
    }

    #[test]
    fn safe_initialiser_does_not_exempt_its_substituted_operand() {
        for (source, missing) in [
            ("proc f {} {lappend x item; puts $x}", false),
            ("proc f {} {lappend x $x; puts $x}", true),
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert_eq!(
                result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == crate::analyser::types::DiagCode::W210
                }),
                missing,
                "{source}: {:?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn root_special_writes_do_not_suppress_same_named_locals() {
        for (source, unused) in [
            ("proc f {} {set auto_path VALUE}", true),
            ("proc f {} {set ::auto_path VALUE}", false),
            ("proc f {} {global auto_path; set auto_path VALUE}", false),
        ] {
            let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert_eq!(
                result.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == crate::analyser::types::DiagCode::W211
                }),
                unused,
                "{source}: {:?}",
                result.diagnostics
            );
        }
    }

    #[test]
    fn dict_literal_fallback_requires_the_original_reaching_value() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for (source, expected) in [
            (
                "proc f {} {set d {present 1}; dict update d present local {puts $local}}",
                true,
            ),
            (
                "proc f {} {set d {present 1}; unset d; dict update d present local {puts $local}}",
                false,
            ),
            (
                "proc f {} {set d {present 1}; unknown_writer; dict update d present local {puts $local}}",
                false,
            ),
            (
                "proc f {} {set d {present 1}; unset d; set d {present 2}; dict update d present local {puts $local}}",
                true,
            ),
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
            let function = cu.procedures.values().next().expect("original procedure");
            let considered = function.ssa.blocks.keys().copied().collect();
            let mut suppression = UndefSuppression::default();
            harvest_dict_with_suppression(
                function,
                &considered,
                &mut suppression,
                tcl_syntax::word_rules::WordValueRules::default(),
                &registry,
            );
            assert_eq!(
                suppression.dict_with_known_keys.contains("local"),
                expected,
                "{source}"
            );
        }
    }
}

#[cfg(test)]
mod original_resolved_variable_advice_tests {
    use super::*;

    #[test]
    fn authored_startup_advice_preserves_literal_dollars_and_exact_global_aliases() {
        // naming.diagnostics.original-resolved-variable-name-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
        // Authored reporting advice only. These names grant no Native cell,
        // startup value, trace, completed getter or entered frame.
        let empty = HashSet::new();
        for (name, expected) in [
            ("argv", "argv"),
            ("::argv", "argv"),
            ("$argv", "$argv"),
            ("::$argv", "$argv"),
            ("${argv}", "${argv}"),
            ("argv(open", "argv(open"),
            ("$argv(k)", "$argv"),
            ("é(k)", "é"),
        ] {
            let cell = crate::var_resolve::VariableCellKey::Authored(name.to_owned());
            assert_eq!(startup_cell_binding(&cell, true, &empty), (expected, true));
        }
        assert!(!has_global_startup_binding("$::argv", false, &empty));
        assert!(!has_global_startup_binding(
            "$argv",
            false,
            &HashSet::from(["argv".to_owned()])
        ));
        assert!(has_global_startup_binding(
            "$argv",
            false,
            &HashSet::from(["::$argv".to_owned()])
        ));
        assert!(!has_global_startup_binding(
            "argv",
            false,
            &HashSet::from(["::$argv".to_owned()])
        ));
    }

    #[test]
    fn literal_variable_diagnostics_keep_defined_names_and_startup_advice_distinct() {
        // naming.diagnostics.original-resolved-variable-name-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
        // Genuine retained Logical analysis, not original Native read or store
        // admission. Check emitted diagnostics and exact read geometry.
        for source in [
            "puts ${$argv}",
            "set {$counter} VALUE; puts ${counter}; puts ${$counter}",
        ] {
            let mut analyser = crate::analyser::Analyser::new();
            let analysis = analyser.analyse(source, "tcl");
            assert!(analysis.allows_retained_logical_declaration_advice());
            let reads: Vec<_> = analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect();
            let expected = if source.starts_with("puts") {
                "${$argv}"
            } else {
                "${counter}"
            };
            assert_eq!(reads, vec![expected], "{source:?}");
        }
        let analysis = crate::analyser::Analyser::new().analyse("puts $argv", "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        assert!(
            !analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
        );
    }
    #[test]
    fn literal_scalar_parentheses_do_not_borrow_array_root_parameters() {
        // naming.diagnostics.original-resolved-variable-name-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
        // Retained Logical read-before-set advice only: these authored names
        // and spans do not establish a Native array kind or current element.
        for (source, expected) in [
            ("proc f {a} {puts ${a(b}}", vec!["${a(b}"]),
            ("proc f {é} {puts ${é(b}}", vec!["${é(b}"]),
            ("proc f {{$a}} {puts ${$a(b}}", vec!["${$a(b}"]),
            ("proc f {a} {puts ${a(k)}}", vec![]),
            ("proc f {é} {puts ${é(k)}}", vec![]),
            ("proc f {{$a}} {puts ${$a(k)}}", vec![]),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            assert!(analysis.allows_retained_logical_declaration_advice());
            let reads: Vec<_> = analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect();
            assert_eq!(reads, expected, "{source:?}");
        }
    }
}
