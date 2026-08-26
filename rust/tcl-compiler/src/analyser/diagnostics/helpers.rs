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
/// classified once for the whole diagnostic code (issue #1195): where the
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
/// (`utils::concat_script_window`, `concat_barrier_words`), so what counts
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

/// Collect `(var, guard_block)` pairs for every
/// `[info exists X]` / `[array exists X]` branch condition in `fu`.
/// A read of `var` in any block dominated by `guard_block` is guarded
/// (X provably exists).  A positive query guards the true target; a
/// `![info exists X]` query guards the false target.
pub(super) fn collect_existence_guards(
    fu: &crate::compilation_unit::FunctionUnit,
) -> Vec<(String, BlockId)> {
    use crate::cfg::Terminator;
    let mut guards = Vec::new();
    for block in fu.cfg.blocks.values() {
        if let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &block.terminator
            && let Some(query) = crate::expr_ast::existence_query_var(condition)
        {
            // Either spelling proves the name is bound in the guarded region:
            // `array exists X` implies `info exists X`.
            let target = if query.negated {
                *false_target
            } else {
                *true_target
            };
            guards.push((query.var, target));
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

/// Names whose whole binding is removed by an `unset` call.  Conservative:
/// only a **literal** bare name kills
/// (a dynamic `unset $name` targets the variable *named by* `$name`, not
/// `name` itself — yet the IR records `name` in the call's defs, so a
/// `$`-stripping harvest would wrongly mark it killed).  Per-element
/// `unset x(k)` drops one array element, not the binding, so it is
/// skipped too.
fn whole_unset_names(args: &[String]) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        let is_dashdash = args[i] == "--";
        i += 1;
        if is_dashdash {
            break;
        }
    }
    for raw in &args[i..] {
        // Literal bare names only — skip dynamic (`$`/`${…}`/`[…]`) and
        // element-subscripted (`x(k)`) targets.
        if raw.contains('$') || raw.contains('[') || raw.contains('(') {
            continue;
        }
        let base = crate::naming::normalise_var_name(raw);
        if !base.is_empty() {
            out.insert(base.to_string());
        }
    }
    out
}

/// Read-only context threaded unchanged through [`phi_can_undef`]'s recursion:
/// the phi indices, the `unset`-killed set, the executable block / edge sets,
/// the dominating existence guards, the registry-owned startup binding, and
/// the SSA function itself.
pub(super) struct PhiUndefCtx<'a> {
    pub phi_def: &'a PhiDefMap,
    pub phi_block: &'a PhiBlockMap,
    pub killed: &'a FxHashSet<(String, crate::ssa::Version)>,
    pub considered: &'a HashSet<BlockId>,
    pub executable_edges: &'a HashSet<(BlockId, BlockId)>,
    pub exists_guards: &'a [(String, BlockId)],
    /// Startup bindings exist only in the document's initial global frame.
    pub initial_global: bool,
    /// Locals that registry metadata says alias the interpreter's global
    /// namespace in this function (`global name`).
    pub global_aliases: &'a HashSet<String>,
    pub dialect: tcl_registry::prelude::DialectSet,
    pub ssa: &'a crate::ssa::SsaFunction,
}

/// Return the registry spelling for a potential startup variable, removing
/// only Tcl's global marker.  A named namespace (`::pkg::name`) deliberately
/// remains qualified and cannot accidentally inherit a global startup fact.
pub(super) fn startup_var_name(name: &str) -> &str {
    let normalised = crate::naming::normalise_var_name(name);
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
    if initial_global || crate::naming::normalise_var_name(name).starts_with("::") {
        return true;
    }
    let startup_name = startup_var_name(name);
    global_aliases.iter().any(|alias| {
        let alias_normalised = crate::naming::normalise_var_name(alias);
        alias_normalised
            .strip_prefix("::")
            .unwrap_or(alias_normalised)
            == startup_name
    })
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
pub(super) fn phi_can_undef(
    name: &str,
    version: crate::ssa::Version,
    ctx: &PhiUndefCtx<'_>,
    seen: &mut FxHashSet<(String, crate::ssa::Version)>,
) -> bool {
    let PhiUndefCtx {
        phi_def,
        phi_block,
        killed,
        considered,
        executable_edges,
        exists_guards,
        initial_global,
        global_aliases,
        dialect,
        ssa,
    } = ctx;
    let startup_name = startup_var_name(name);
    let global_binding = has_global_startup_binding(name, *initial_global, global_aliases);
    let rematerialises_after_unset =
        global_binding && tcl_registry::special_vars::is_lazily_readable(startup_name, *dialect);
    let key = (name.to_string(), version);
    if killed.contains(&key) {
        // A Tcl read trace is not an eager startup fact: `unset` removes the
        // current value, but a later read materialises it again.  Registry
        // data confines this to `tcl_precision` on Tcl 8.x; eager bindings
        // such as argv remain genuine W210 reads after `unset`.
        return !rematerialises_after_unset;
    }
    if version == 0 {
        // A version-zero incoming normally is the undef origin.  The default
        // Tcl host, however, binds a registry-declared subset before user
        // code.  This must be decided while tracing phi incomings too: a
        // conditional write otherwise makes a merge with the startup version
        // look undefined.  `unset` still wins below for real killed versions,
        // and procedure-local frames never set `initial_global`.
        return !(global_binding
            && tcl_registry::special_vars::is_readable_at_startup(startup_name, *dialect));
    }
    if seen.contains(&key) {
        // Cycle (loop-header phi): the DFS seed already accounted for the
        // entry path's contribution; treat the back-edge as not-undef to
        // avoid every loop-header phi self-triggering.
        return false;
    }
    let Some(phi) = phi_def.get(&key) else {
        // Concrete (non-phi) definition reached this version — safe.
        return false;
    };
    // The block this phi lives in — the destination of each incoming edge.
    let this_block = phi_block.get(&key).copied();
    seen.insert(key.clone());
    let mut result = false;
    for (&pred, &incoming_ver) in &phi.incoming {
        if !considered.contains(&pred) {
            continue;
        }
        // A phi has one operand per predecessor *edge*; an operand arriving on
        // a non-executable edge (SCCP proved the edge dead — e.g. the
        // `cond → exit` edge of `while 1`, which a `break` makes the loop's
        // only real exit) can never actually be read, so its version-0 origin
        // must not count as a possible undef.  This filter is only applied
        // when SCCP edge info is available (a non-empty set).
        if let Some(blk) = this_block
            && !executable_edges.is_empty()
            && !executable_edges.contains(&(pred, blk))
        {
            continue;
        }
        // A dominating existence guard proves the variable is defined at
        // the predecessor; that incoming cannot be undef regardless of
        // its SSA version.
        if exists_guards
            .iter()
            .any(|(gv, gblk)| gv == name && block_dominated_by(ssa, pred, *gblk))
        {
            continue;
        }
        if phi_can_undef(name, incoming_ver, ctx, seen) {
            result = true;
            break;
        }
    }
    seen.remove(&key);
    result
}

/// `(name, version) → Phi` index used by [`phi_can_undef`].
pub(super) type PhiDefMap = FxHashMap<(String, crate::ssa::Version), crate::ssa::Phi>;

/// `(name, version) → defining block` index, so [`phi_can_undef`] can test
/// each incoming `(pred, phi_block)` edge against the SCCP-executable edge set.
pub(super) type PhiBlockMap = FxHashMap<(String, crate::ssa::Version), BlockId>;

/// Build the `(name, version) → Phi` index, the `(name, version) → block`
/// index, and the set of `unset`-killed versions for [`phi_can_undef`],
/// restricted to `considered` (executable) blocks.
pub(super) fn build_phi_undef_index(
    ssa: &crate::ssa::SsaFunction,
    considered: &HashSet<BlockId>,
) -> (
    PhiDefMap,
    PhiBlockMap,
    FxHashSet<(String, crate::ssa::Version)>,
) {
    use crate::ir::Statement;
    let mut phi_def: PhiDefMap = FxHashMap::default();
    let mut phi_block: PhiBlockMap = FxHashMap::default();
    let mut killed: FxHashSet<(String, crate::ssa::Version)> = FxHashSet::default();
    for &bn in considered {
        let Some(sblock) = ssa.blocks.get(&bn) else {
            continue;
        };
        for phi in &sblock.phis {
            let phi_name = ssa.var_name(phi.name).to_owned();
            phi_def.insert((phi_name.clone(), phi.version), phi.clone());
            phi_block.insert((phi_name, phi.version), bn);
        }
        for s in &sblock.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &s.statement
            else {
                continue;
            };
            let is_unset = canonical_command.as_deref() == Some("::unset") || command == "unset";
            if !is_unset {
                continue;
            }
            let whole = whole_unset_names(args);
            for (&def_sym, def_ver) in &s.defs {
                let def_name = ssa.var_name(def_sym);
                if whole.contains(def_name) {
                    killed.insert((def_name.to_owned(), *def_ver));
                }
            }
        }
    }
    (phi_def, phi_block, killed)
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
    /// Names written by a command substitution buried inside an `expr`
    /// argument (`set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during
    /// expr evaluation).  The `[…]` is opaque to SSA def tracking, so a later
    /// `$tmp` read in the same expression looks read-before-set.  Name-level,
    /// suppress-only.
    cmd_sub_writes: FxHashSet<String>,
    /// Names written by a `Traits::SCRIPT_CONCATENATES_ARGS` call whose
    /// script the lowering left as an opaque barrier — `eval set l2 hello`
    /// really does set `l2` in the caller's own frame, but its words reach
    /// the IR as barrier arguments with no def attached, so a later
    /// `puts $l2` looked read-before-set (issue #1051).  Name-level,
    /// suppress-only.
    script_concat_writes: FxHashSet<String>,
    /// `(name, version)` pairs killed by an `unset` — undef at their reads,
    /// so a direct read of one is read-before-set just like a version-0
    /// origin.
    pub(super) killed: FxHashSet<(String, crate::ssa::Version)>,
    /// Phi versions that can be undefined on some executable path
    /// (a one-branch `set y 1` merge, or a try-handler merge). A statement
    /// read of one is read-before-set; the def-use pass can't express this
    /// because the read targets the *phi* version, not a version-0 origin.
    pub(super) can_undef: FxHashSet<(String, crate::ssa::Version)>,
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
    pub(super) loop_entry_only_undef: FxHashMap<(String, crate::ssa::Version), FxHashSet<String>>,
}

impl UndefSuppression {
    /// True when a read of `name` is suppressed by an alias declaration or a
    /// `dict with` / `dict update` unpack.  Blanket variant: an unknown-shape
    /// dict suppresses every non-concrete name (the conservative
    /// "might-have-the-key" stance, used where no truth source can confirm
    /// the dict is empty — e.g. a `return` after a `dict with` on a param).
    pub(super) fn suppresses(&self, name: &str) -> bool {
        self.suppresses_strict(name)
            || (self.has_dict_with
                && self.dict_with_any_unknown
                && !self.explicitly_defined.contains(name))
    }

    /// True when reading `key` at `block` is a safe *after-loop* read of a
    /// variable the loop body defines on every iteration (see
    /// [`Self::loop_entry_only_undef`]): the version is loop-entry-only-undef
    /// and `block` is outside the loop body. A read inside the loop body still
    /// fires (first-iteration undef is real).
    pub(super) fn after_loop_defined(
        &self,
        key: &(String, crate::ssa::Version),
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
        if self.alias_tails.contains(name)
            || self.dict_vars.contains(name)
            || self.cmd_sub_writes.contains(name)
            || self.script_concat_writes.contains(name)
        {
            return true;
        }
        self.has_dict_with
            && !self.explicitly_defined.contains(name)
            && self.dict_with_known_keys.contains(name)
    }
}

/// Build the [`UndefSuppression`] context over `considered` blocks.
/// Names written by a command substitution buried inside an `expr` argument.
/// `set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during expr
/// evaluation; the `set x [expr {E}]` form lowers to `AssignExpr`, so the
/// condition-out-var extractor over its expr recovers those writes.
/// Name-level, suppress-only.
fn collect_expr_cmd_sub_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
) -> FxHashSet<String> {
    use crate::ir::Statement;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut out = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            if let Statement::AssignExpr { expr, .. } = stmt {
                out.extend(crate::ir_helpers::condition_command_out_vars(
                    expr, registry,
                ));
            }
        }
        // A branch condition (`if {![catch {set x 1}]} …`) evaluates its command
        // substitutions before either arm, so any variables they write — the
        // catch result var *and* the catch body's assignments — are (maybe) set
        // in the taken arm and must not look read-before-set.
        if let Some(crate::cfg::Terminator::Branch { condition, .. }) = &block.terminator {
            out.extend(crate::ir_helpers::condition_command_out_vars(
                condition, registry,
            ));
        }
    }
    out
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
) -> FxHashSet<String> {
    use crate::ir::Statement;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut out = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            let Statement::Barrier {
                command,
                args,
                tokens: Some(tokens),
                ..
            } = stmt
            else {
                continue;
            };
            let Some(spec) = registry.get(command) else {
                continue;
            };
            if !spec
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
                || spec.body_kind != tcl_registry::BodyKind::Plain
            {
                continue;
            }
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            let Some(&first) = registry
                .arg_indices_for_role(command, &arg_strs, tcl_registry::ArgRole::Body)
                .first()
            else {
                continue;
            };
            // `argv_texts` / `argv_kinds` include the command word at index 0;
            // `args` does not, so the body index shifts by one.
            //
            // A dynamic tail declines the join, not the braced first word:
            // `eval {set l2 5} $extra` still runs `set l2 5` as written
            // (concatenation only appends after it), so its writes are
            // recovered from that word alone — mirroring the analyser's
            // `dispatch_concatenated_script` fallback.
            let script = if let Some(script) = concat_barrier_words(tokens, first + 1) {
                script
            } else {
                let (Some(text), Some(&kind)) = (
                    tokens.argv_texts.get(first + 1),
                    tokens.argv_kinds.get(first + 1),
                ) else {
                    continue;
                };
                if kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                text.clone()
            };
            let mut writes = Vec::new();
            crate::ir_helpers::script_text_out_vars(&script, registry, &mut writes);
            out.extend(writes);
        }
    }
    out
}

/// The `Tcl_ConcatObj` join of a barrier call's words from `first` onwards,
/// or `None` when any of them is not statically-known script text (the real
/// script is then unknowable — see [`word_is_static_script_text`], the same
/// predicate `crate::analyser::utils::concat_script_window` applies to the
/// analyser's own token slices). This join is consumed for write-name
/// harvesting only, never for spans, so a plain text join suffices here.
fn concat_barrier_words(tokens: &crate::ir::CommandTokens, first: usize) -> Option<String> {
    let texts = tokens.argv_texts.get(first..)?;
    let kinds = tokens.argv_kinds.get(first..)?;
    if texts.len() < 2 || texts.len() != kinds.len() {
        return None;
    }
    let mut joined = String::new();
    for (text, &kind) in texts.iter().zip(kinds) {
        if !word_is_static_script_text(text, kind) {
            return None;
        }
        let trimmed = text.trim_matches(|c: char| c.is_ascii_whitespace());
        if trimmed.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(trimmed);
    }
    Some(joined)
}

/// `dict with` / `dict update` key-aware suppression: record the dict-var
/// names and, when the dict value is a same-block literal (or an
/// interprocedurally-propagated SCCP const), its keys.  A value that resolves
/// to neither marks the dict shape unknown.
fn harvest_dict_with_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    s: &mut UndefSuppression,
) {
    use crate::ir::Statement;
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            let is_dict = command == "dict" || stmt.canonical_command_or_source() == "::dict";
            if !is_dict {
                continue;
            }
            if args.first().map(String::as_str) != Some("with")
                && args.first().map(String::as_str) != Some("update")
            {
                continue;
            }
            s.has_dict_with = true;
            let Some(dict_var) = args.get(1) else {
                s.dict_with_any_unknown = true;
                continue;
            };
            let dvar = crate::naming::normalise_var_name(dict_var).to_string();
            if dvar.is_empty() {
                s.dict_with_any_unknown = true;
                continue;
            }
            s.dict_vars.insert(dvar.clone());
            // Resolve the dict's value to harvest its keys.  Prefer the SCCP
            // CONST of the SPECIFIC version read by this dict-with (so
            // interprocedurally-propagated literals — a caller passing `{}` —
            // are honoured), falling back to a same-block literal `set`.  A
            // known value (even empty) harvests its keys; only a value that
            // resolves to neither marks the dict shape unknown.
            let mut literal: Option<String> = None;
            if let Some(dsym) = fu.ssa.var_symbol(&dvar)
                && let Some(sb) = fu.ssa.blocks.get(&bn)
                && let Some(ver) = sb
                    .statements
                    .get(idx)
                    .and_then(|s| s.uses.get(&dsym).copied())
                && let Some(crate::analyses::LatticeValue::Const(
                    crate::analyses::ConstValue::String(v),
                )) = fu.sccp.values.get(&(dsym, ver))
            {
                literal = Some(v.clone());
            }
            if literal.is_none() {
                for prev in (0..idx).rev() {
                    match &block.statements[prev] {
                        Statement::AssignConst { name, value, .. }
                            if crate::naming::normalise_var_name(name) == dvar =>
                        {
                            literal = Some(value.clone());
                            break;
                        }
                        // A barrier between us and the literal invalidates it.
                        Statement::Barrier { .. } => break,
                        _ => {}
                    }
                }
            }
            match literal {
                Some(v) => {
                    let elems = crate::tcl_expr_eval::split_tcl_list(&v);
                    if args.first().map(String::as_str) == Some("update") {
                        // `dict update d k1 v1 k2 v2 … BODY` binds each value-var
                        // vN to the value of key kN *inside the body* — but only
                        // when kN is present in the dict (tclsh: an absent key
                        // leaves vN unset). So a read of vN is suppressed exactly
                        // when kN is a known-present key. args[2..len-1] are the
                        // key/value pairs; the final arg is the BODY.
                        let present: HashSet<&str> =
                            elems.iter().step_by(2).map(String::as_str).collect();
                        let end = args.len().saturating_sub(1);
                        let mut i = 2;
                        while i + 1 < end {
                            if present.contains(args[i].as_str()) {
                                let valvar =
                                    crate::naming::normalise_var_name(&args[i + 1]).to_string();
                                if !valvar.is_empty() {
                                    s.dict_with_known_keys.insert(valvar);
                                }
                            }
                            i += 2;
                        }
                    } else {
                        // `dict with`: the body binds each present key as a local.
                        for (i, key) in elems.into_iter().enumerate() {
                            if i % 2 == 0 {
                                s.dict_with_known_keys.insert(key);
                            }
                        }
                    }
                }
                None => s.dict_with_any_unknown = true,
            }
        }
    }
}

pub(super) fn build_undef_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    dialect: tcl_registry::prelude::DialectSet,
) -> UndefSuppression {
    let (phi_def, phi_block, killed) = build_phi_undef_index(&fu.ssa, considered);
    // Phi versions that can reach an undef origin on some executable path —
    // a statement read of one is read-before-set. The per-use existence
    // guard + suppression set still apply in the emitter loop.
    let exists_guards = collect_existence_guards(fu);
    let undef_ctx = PhiUndefCtx {
        phi_def: &phi_def,
        phi_block: &phi_block,
        killed: &killed,
        considered,
        executable_edges: &fu.sccp.executable_edges,
        exists_guards: &exists_guards,
        initial_global,
        global_aliases,
        dialect,
        ssa: &fu.ssa,
    };
    let mut can_undef: FxHashSet<(String, crate::ssa::Version)> = FxHashSet::default();
    for key in phi_def.keys() {
        let mut seen = FxHashSet::default();
        if phi_can_undef(&key.0, key.1, &undef_ctx, &mut seen) {
            can_undef.insert(key.clone());
        }
    }
    let loop_entry_only_undef = build_loop_entry_only_undef(fu, &can_undef, &undef_ctx);
    let mut s = UndefSuppression {
        cmd_sub_writes: collect_expr_cmd_sub_writes(fu, considered),
        script_concat_writes: collect_script_concat_writes(fu, considered),
        killed,
        can_undef,
        loop_entry_only_undef,
        ..Default::default()
    };
    harvest_dict_with_suppression(fu, considered, &mut s);

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

    s.alias_tails = collect_qualified_variable_alias_tails(fu, considered);
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
    can_undef: &FxHashSet<(String, crate::ssa::Version)>,
    ctx: &PhiUndefCtx<'_>,
) -> FxHashMap<(String, crate::ssa::Version), FxHashSet<String>> {
    let mut out: FxHashMap<(String, crate::ssa::Version), FxHashSet<String>> = FxHashMap::default();
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
            if foreach_header_provably_empty(fu, header_id) {
                continue;
            }
            let body_blocks: FxHashSet<String> = natural.blocks.iter().cloned().collect();
            let Some(ssa_header) = fu.ssa.blocks.get(&header_id) else {
                continue;
            };
            for phi in &ssa_header.phis {
                let name = fu.ssa.var_name(phi.name).to_owned();
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
                    if out.contains_key(&(name.clone(), ver_in)) {
                        return true;
                    }
                    let mut seen = FxHashSet::default();
                    !phi_can_undef(&name, ver_in, ctx, &mut seen)
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
                        .all(|a| crate::tcl_expr_eval::split_tcl_list(a).is_empty())
        )
    })
}

/// Local-alias tail names declared by a *qualified* `variable`
/// (`variable ns::tail` / `variable ${name}::tail`): the bare tail read
/// resolves to the namespace var, not an unset local.
fn collect_qualified_variable_alias_tails(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
) -> FxHashSet<String> {
    use crate::ir::Statement;
    let mut tails = FxHashSet::default();
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            let (Statement::Barrier { command, args, .. } | Statement::Call { command, args, .. }) =
                stmt
            else {
                continue;
            };
            if command != "variable" && stmt.canonical_command_or_source() != "::variable" {
                continue;
            }
            // `variable` alternates (name, value?) pairs — names at even args.
            let mut i = 0;
            while i < args.len() {
                let text = &args[i];
                if text.contains("::") {
                    let tail = text.rsplit("::").next().unwrap_or(text);
                    let (base, _) = crate::naming::split_array_name(tail);
                    if !base.is_empty()
                        && !base.contains('$')
                        && !base.contains('[')
                        && !base.contains('{')
                    {
                        tails.insert(crate::naming::normalise_var_name(base).to_string());
                    }
                }
                i += 2;
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
                    let normalised = crate::naming::normalise_var_name(name);
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

/// Compute the set of global variable names that any procedure
/// in `cu` writes.
///
/// A global write happens when a proc either:
///
/// 1. assigns to a fully-qualified name (``::var``), or
/// 2. declares ``global var`` and then assigns to ``var`` in the
///    same proc body.
///
/// The result is the union of (1) and the intersection of
/// global aliases × locally-written names (case (2)).  Used at
/// top-level to suppress W210 for globals a helper proc may
/// populate before the top-level read.
///
/// There is no ``CommandRegistry::is_destroys_variable`` yet, so
/// commands like ``unset`` aren't filtered out of the "writes" set.
/// That makes the suppression slightly more permissive (more
/// vars marked "written-by-procs" → more W210 suppressions).
/// Safe-on-correctness — the alternative is false positives
/// on real RBS sites.  When the registry gains
/// ``destroys_variable``, add the filter here.
pub(super) fn globals_written_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        let mut global_aliases: FxHashSet<String> = FxHashSet::default();
        let mut written: FxHashSet<String> = FxHashSet::default();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let names: Vec<&String> = match stmt {
                    Statement::Call { command, defs, .. } => {
                        if command == "global" {
                            for d in defs {
                                global_aliases.insert(d.clone());
                            }
                            continue;
                        }
                        // `unset` destroys a variable, it never assigns one, so
                        // it must not count as a write that could make a later
                        // top-level read safe. tclsh: a proc whose only touch of
                        // `::x` is `unset ::x` leaves a top-level `$x` genuinely
                        // read-before-set ("can't read \"x\": no such variable").
                        // (`variable`/`upvar` only *declare*/alias; `unset`
                        // removes.) A proc that also `set`s the global still
                        // contributes via that assignment statement.
                        if matches!(command.as_str(), "variable" | "upvar" | "unset") {
                            continue;
                        }
                        defs.iter().collect()
                    }
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    _ => continue,
                };
                for name in names {
                    if let Some(bare) = name.strip_prefix("::") {
                        let bare = bare.trim_start_matches(':');
                        if !bare.is_empty() {
                            result.insert(bare.to_string());
                        }
                    } else {
                        written.insert(name.clone());
                    }
                }
            }
        }
        for n in global_aliases.intersection(&written) {
            result.insert(n.clone());
        }
    }
    result
}

/// Compute the set of global variable names that any procedure in `cu`
/// **reads**.
///
/// The read-side mirror of [`globals_written_by_procs`]: a top-level
/// assignment (`set cfg …`, which runs in the global namespace) is not a
/// dead store (W220) or unused variable (W211) when a helper proc consumes
/// it — exactly as a top-level *read* is not read-before-set (W210) when a
/// helper proc populates it.  A proc reads a global when it either:
///
/// 1. reads a fully-qualified name (`$::cfg`, `$::ns::cfg` — bare tail
///    `cfg`), which always resolves outside the proc's own frame, or
/// 2. declares `global cfg` and then reads `cfg` in the same body.
///
/// Name-level and deliberately permissive (the global namespace is shared
/// between the top-level script and every proc, so a same-named read may be
/// the consumer): the alternative is a false unused / dead-store hint on the
/// extremely common "config global set at the top, read inside procs" shape.
/// `variable` / `upvar` aliases are *not* counted — they bind the current
/// namespace / a caller frame, not the global scope, so they cannot consume
/// a global-scope top-level `set`.
pub(super) fn globals_read_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        // Names the proc declares as global-scope aliases (`global cfg`).
        let mut global_aliases: FxHashSet<String> = FxHashSet::default();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { command, defs, .. } = stmt
                    && command == "global"
                {
                    for d in defs {
                        global_aliases.insert(d.clone());
                    }
                }
            }
        }
        // Names the proc reads.  The def-use chains already fold in the
        // `return $x` terminator and branch-condition reads that the raw
        // per-statement `uses` miss, so a chain carrying any use marks its
        // name as read.
        let mut read: FxHashSet<String> = FxHashSet::default();
        for (key, chain) in &fu.def_use.chains {
            if !chain.uses.is_empty() {
                read.insert(key.0.clone());
            }
        }
        // (1) A fully-qualified read always targets the global / a named
        // namespace scope. Strip only the *leading* `::` — mirroring
        // `globals_written_by_procs` above — so a truly-global read (`$::cfg`,
        // one segment once the leading `::` is gone) collapses to the bare
        // name a top-level `set cfg` produces, while a *namespaced* read
        // (`$::n::cfg`) keeps its `n::` qualification intact. Collapsing to
        // just the last segment (the tail) would conflate `::n::cfg` with an
        // unrelated bare `::cfg` of the same tail — a different storage cell —
        // and could mask a genuinely-unused top-level global that happens to
        // share a name with a namespaced variable read elsewhere.
        for name in &read {
            if let Some(bare) = name.strip_prefix("::") {
                let bare = bare.trim_start_matches(':');
                if !bare.is_empty() {
                    result.insert(bare.to_string());
                }
            }
        }
        // (2) A `global cfg` alias the body reads is a read of global `cfg`.
        for n in global_aliases.intersection(&read) {
            result.insert(n.clone());
        }
    }
    result
}
