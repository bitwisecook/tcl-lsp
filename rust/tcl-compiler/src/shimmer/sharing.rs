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

//! Shared-value copy-on-write detection (S103).
//!
//! C Tcl never mutates a value another holder can still see: every
//! variable-mutating list/dict command duplicates a shared value before
//! writing — `Tcl_IsShared(...) → Tcl_DuplicateObj(...)` in
//! `Tcl_LappendObjCmd` (`tclVar.c`, the `else if (Tcl_IsShared(varValuePtr))`
//! copy), `TclLsetFlat` (`tclListObj.c`, `subListPtr = Tcl_IsShared(listPtr)
//! ? Tcl_DuplicateObj(listPtr) : listPtr`), and each `dict`
//! variable-mutating subcommand (`tclDictObj.c`, the `if
//! (Tcl_IsShared(dictPtr)) { dictPtr = Tcl_DuplicateObj(dictPtr); }` blocks;
//! the core mutation APIs `Tcl_ListObjAppendElement` / `Tcl_ListObjReplace`
//! / `Tcl_DictObjPut` all `Tcl_Panic` on a shared object, so callers MUST
//! copy first). So `lappend`/`lset`/`dict set` on a value with refcount ≥ 2
//! is an O(n) whole-value copy on every call — the classic hidden cost when
//! a list is also stored somewhere else.
//!
//! Verified against tclsh 8.6.14 (`tcl::unsupported::representation`,
//! pointer identity is the evidence — the printed refcount also counts
//! transient stack references):
//!
//! ```text
//! set a [lrepeat 1000 x]; set b $a
//!   b before:            object pointer at 0x...eff0   (same as a — shared)
//!   lappend b y
//!   b after:             object pointer at 0x...fa10   (fresh duplicate)
//!   a after:             object pointer at 0x...eff0   (original, untouched)
//! set d $c; lset d 0 z        → d gets a fresh pointer
//! set f $e; dict set f k3 v3  → f gets a fresh pointer
//! set h $g; lappend g z       → g gets a fresh pointer (mutating the SOURCE
//!                               while the copy lives duplicates too)
//! set n $m; set m "gone"; lappend n y
//!                             → n keeps its pointer (sole holder — no copy)
//! ```
//!
//! # The tight pattern (deliberate under-approximation)
//!
//! A value is *provably* shared at a mutation site only when this pass can
//! see both holders. It fires exclusively on:
//!
//! 1. a same-block pure-copy assignment `set b $a` (the value word is
//!    exactly one variable reference — tclsh-verified `set b "$a"` shares
//!    the same object, so the quoted form counts too), followed by
//! 2. a mutation of `b` — or of `a`, the symmetric case — through
//!    `lappend` / `lset` / `dict set|unset|append|lappend|incr` whose
//!    consumed SSA version is exactly the version the copy created (any
//!    intervening redefinition breaks the pair), while
//! 3. the *other* member of the pair is still read afterwards (an operand
//!    or terminator use strictly after the mutation in the same block, or
//!    in a block reachable from it) — both holders alive, so the duplicate
//!    is guaranteed and the sharing is evidently wanted.
//!
//! Everything looser stays silent by design:
//!
//! - **Parameters mutated directly** (`proc p {l} { lappend l x }`): the
//!   caller's copy is the normal case — C Tcl procs receive shared argument
//!   objects by design, and idiomatic Tcl accepts the first-write copy.
//!   (The pattern above cannot match one anyway: the mutation must consume
//!   the same-block copy's own def.) A copy *from* a parameter (`set b
//!   $lst; lappend b x` with `$lst` still read) does fire — the in-proc
//!   alias makes the duplicate unconditional, tclsh-verified above.
//! - **Explicit copies** (`set b [lrange $a 0 end]`) are not pure-var-ref
//!   assignments — no pair, silent.
//! - **A dead source** (`a` never read after the mutation): the duplicate
//!   still happens at runtime — the variable slot keeps its reference until
//!   rewritten — but the fix is trivial and flagging it would punish
//!   harmless code, so the pass demands a live partner.
//! - **Aliased / traced / qualified names** (`global`, `variable`,
//!   `upvar`, in-function `trace`, `::`-qualified, `TclOO` instance
//!   variables) and **array elements** (one conflated SSA symbol per
//!   array): another route can reach the slot or the symbol conflates
//!   distinct runtime slots — abstain, matching the sibling passes'
//!   FP-SH-02/13/15 guards.
//! - **Cross-block copies, `eval`-style barriers between copy and
//!   mutation, phi-carried liveness**: abstain.
//!
//! Diagnostic: **S103**, severity Hint — "mutation of a potentially shared
//! value copies it".

use std::collections::{HashMap, HashSet};

use tcl_core_types::DiagCode;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::def_use::{DefUseResult, UseKind, UseSite};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, Version};
use crate::value_shapes::is_pure_var_ref;

use super::SharingWarning;
use super::graph::build_successors;

/// A same-block pure-copy assignment `set dst $src`: the two SSA values
/// that share one runtime object from this statement on.
struct CopyPair {
    src: (Symbol, Version),
    dst: (Symbol, Version),
    /// Span of the `set dst $src` statement, for the related-span note.
    span: Span,
}

/// Resolve the registry-declared shared-container mutation target.
///
/// The effective command, subcommand, form, target position, and arity floor
/// are all registry data. Unknown or dynamic shapes abstain.
///
/// The context-less (`None`) primitive call is deliberate under invariant
/// I4: this is a **widening** query — a mutation answer widens the S101
/// shimmer warning set, so over-approximating across environments is the
/// conservative direction, never a specialisation on an unproved binding.
fn mutation_target<'a>(stmt: &'a Statement, registry: &CommandRegistry) -> Option<&'a str> {
    let Statement::Call { args, .. } = stmt else {
        return None;
    };
    let values: Vec<&str> = args.iter().map(String::as_str).collect();
    let invocation = tcl_registry::model::resolve_invocation_in_context(
        registry,
        None,
        stmt.canonical_command_or_source(),
        &values,
    )?;
    let offset = invocation.semantics.argument_offset;
    let effective_count = args.len().checked_sub(offset)?;
    let target = invocation
        .semantics
        .representation_effect
        .mutation_target_index(effective_count, offset)?;
    args.get(target).map(String::as_str)
}

/// Function-wide inputs and memos threaded through the walk.
struct SharingCtx<'a> {
    registry: &'a CommandRegistry,
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    def_use: &'a DefUseResult,
    executable_blocks: &'a HashSet<BlockId>,
    /// Name-keyed CFG successor map ([`build_successors`]).
    succs: HashMap<String, Vec<String>>,
    /// Forward-reachability memo, per mutation block (BFS over successor
    /// names; a block reaches itself only via a cycle).
    reachable_memo: HashMap<String, HashSet<String>>,
    /// Names another route can reach: `global`/`variable`/`upvar` aliases
    /// and in-function `trace` targets (the per-function view the type/SCCP
    /// passes also consume), plus the caller's implicit instance-variable
    /// set.
    excluded_names: HashSet<String>,
    /// Array bases conflate every element onto one SSA symbol (FP-SH-13) —
    /// a pair or mutation involving one may mix distinct runtime slots.
    array_syms: HashSet<Symbol>,
}

impl SharingCtx<'_> {
    /// Whether the variable is outside the pass's certainty envelope —
    /// aliased/traced/instance names, `::`-qualified names, array bases.
    fn excluded(&self, name: &str, sym: Symbol) -> bool {
        name.starts_with("::")
            || self.excluded_names.contains(name)
            || self.array_syms.contains(&sym)
    }
}

/// Find shared-value copy-on-write warnings (S103) for one function.
///
/// Walks every executable block in CFG order; see the module docs for the
/// pattern and the deliberate exclusions. `extra_scope_aliases` is the
/// caller-supplied implicit alias set (a `TclOO` method's class-declared
/// instance variables), mirroring the S102 pass's parameter of the same
/// name.
#[must_use]
pub(crate) fn find_sharing_warnings<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    def_use: &DefUseResult,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    extra_scope_aliases: Option<&HashSet<String, S>>,
) -> Vec<SharingWarning> {
    let mut excluded_names =
        crate::var_observability::analyse_var_observability(cfg, registry).escaping_var_names();
    if let Some(extra) = extra_scope_aliases {
        excluded_names.extend(extra.iter().cloned());
    }
    let mut ctx = SharingCtx {
        registry,
        cfg,
        ssa,
        def_use,
        executable_blocks,
        succs: build_successors(cfg),
        reachable_memo: HashMap::new(),
        excluded_names,
        array_syms: super::thunking::array_element_symbols(cfg, ssa),
    };

    let mut out = Vec::new();
    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        let block_name = cfg.block_name(block_id).to_owned();

        // Live copy pairs seen so far in this block. A `Barrier`/`UpFrame`
        // can rewrite arbitrary variables without an SSA version bump, so
        // it clears the ledger (abstain across it).
        let mut pairs: Vec<CopyPair> = Vec::new();
        for (idx, ss) in ssa_block.statements.iter().enumerate() {
            match &ss.statement {
                Statement::Barrier { .. } | Statement::UpFrame { .. } => pairs.clear(),
                Statement::AssignValue {
                    name, value, span, ..
                } => {
                    if let Some(pair) = copy_pair(&ctx, ss, name, value, *span) {
                        pairs.push(pair);
                    }
                }
                stmt => {
                    if let Some(w) = mutation_warning(&mut ctx, &block_name, idx, ss, stmt, &pairs)
                    {
                        out.push(w);
                    }
                }
            }
        }
    }
    out
}

/// Recognise a pure-copy assignment `set dst $src` and build its
/// [`CopyPair`]. `None` for anything else — a computed value, an excluded
/// name, a self-copy, or an assignment whose SSA versions are unavailable.
fn copy_pair(
    ctx: &SharingCtx<'_>,
    ss: &crate::ssa::SsaStatement,
    name: &str,
    value: &str,
    span: Span,
) -> Option<CopyPair> {
    if !is_pure_var_ref(value) {
        return None;
    }
    let dst_name = normalise_var_name(name);
    let src_name = normalise_var_name(value);
    let dst_sym = ctx.ssa.var_symbol(dst_name)?;
    let src_sym = ctx.ssa.var_symbol(src_name)?;
    if dst_sym == src_sym || ctx.excluded(dst_name, dst_sym) || ctx.excluded(src_name, src_sym) {
        return None;
    }
    let src_ver = *ss.uses.get(&src_sym)?;
    let dst_ver = *ss.defs.get(&dst_sym)?;
    // A version-0 source is a parameter/live-in — still a genuine share
    // (tclsh-verified in the module docs), so it is kept.
    Some(CopyPair {
        src: (src_sym, src_ver),
        dst: (dst_sym, dst_ver),
        span,
    })
}

/// Check one candidate mutation statement against the block's live copy
/// pairs and build the S103 warning when the tight pattern completes:
/// `stmt` mutates one member of a pair at exactly the shared version and
/// the other member is still read afterwards.
fn mutation_warning(
    ctx: &mut SharingCtx<'_>,
    block_name: &str,
    stmt_idx: usize,
    ss: &crate::ssa::SsaStatement,
    stmt: &Statement,
    pairs: &[CopyPair],
) -> Option<SharingWarning> {
    let target_name = normalise_var_name(mutation_target(stmt, ctx.registry)?);
    let target_sym = ctx.ssa.var_symbol(target_name)?;
    if ctx.excluded(target_name, target_sym) {
        return None;
    }
    // The mutation must consume a version and write a new one —
    // `{*}`-expanded or dynamic-name spellings record no def and are
    // skipped.
    let used_ver = *ss.uses.get(&target_sym)?;
    ss.defs.get(&target_sym)?;
    let mutated = (target_sym, used_ver);
    // First matching pair wins (deterministic: pairs are in statement
    // order); one warning per mutation statement.
    let (partner_key, copy_span) = pairs.iter().find_map(|p| {
        if p.dst == mutated {
            Some((p.src, p.span))
        } else if p.src == mutated {
            Some((p.dst, p.span))
        } else {
            None
        }
    })?;
    let live_read = partner_read_after(ctx, block_name, stmt_idx, partner_key)?;
    let command = stmt.canonical_command_or_source().to_owned();
    let variable = target_name.to_owned();
    let shared_with = ctx.ssa.var_name(partner_key.0).to_owned();
    let mut related = vec![(
        copy_span,
        format!("'{variable}' and '{shared_with}' share one value from here"),
    )];
    if let Some(sp) = live_read.span {
        related.push((sp, format!("'{shared_with}' is still read here")));
    }
    Some(SharingWarning {
        span: stmt.span(),
        message: format!(
            "mutation of a potentially shared value copies it — '{command}' \
             duplicates '{variable}' (still shared with '{shared_with}') \
             before writing",
        ),
        variable,
        shared_with,
        command,
        code: DiagCode::S103,
        related,
    })
}

/// Evidence that the partner is still read after the mutation.
struct LiveRead {
    /// Best-effort span of one qualifying read, when resolvable.
    span: Option<Span>,
}

/// Is the partner SSA value read strictly *after* the mutation? Returns the
/// [`LiveRead`] evidence when it is, `None` when the partner is dead past
/// the mutation.
///
/// "After" means: an operand use later in the same block, a terminator use
/// of the same block (the terminator executes after every statement), or
/// any use in a different executable block forward-reachable from the
/// mutation's block. Phi-incoming uses do not count — value flow through a
/// merge is not the direct read the tight pattern demands. Same-block uses
/// at earlier indices are first-iteration reads even under a loop (the
/// re-entry value arrives through a phi as a different version), so they
/// never count.
fn partner_read_after(
    ctx: &mut SharingCtx<'_>,
    block_name: &str,
    stmt_idx: usize,
    partner: (Symbol, Version),
) -> Option<LiveRead> {
    let partner_name = ctx.ssa.var_name(partner.0).to_owned();
    let uses: &[UseSite] = ctx.def_use.uses_of(&partner_name, partner.1);
    let mut verdict: Option<LiveRead> = None;
    for u in uses {
        let qualifies = match u.kind {
            UseKind::PhiIncoming => false,
            UseKind::Terminator => {
                u.block == block_name
                    || reachable(ctx, block_name, &u.block) && executable(ctx, &u.block)
            }
            UseKind::Operand => {
                if u.block == block_name {
                    usize::try_from(u.statement_index).is_ok_and(|i| i > stmt_idx)
                } else {
                    reachable(ctx, block_name, &u.block) && executable(ctx, &u.block)
                }
            }
        };
        if !qualifies {
            continue;
        }
        let span = use_span(ctx, u);
        // Prefer a use with a resolvable span for the related note.
        if span.is_some() {
            return Some(LiveRead { span });
        }
        verdict = Some(LiveRead { span: None });
    }
    verdict
}

/// Whether `block` (by name) is forward-reachable from `from` via CFG
/// successor edges, memoised per source block. The source block itself is
/// only reachable through a cycle.
fn reachable(ctx: &mut SharingCtx<'_>, from: &str, block: &str) -> bool {
    if !ctx.reachable_memo.contains_key(from) {
        let mut seen: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = ctx.succs.get(from).cloned().unwrap_or_default();
        while let Some(bn) = frontier.pop() {
            if !seen.insert(bn.clone()) {
                continue;
            }
            if let Some(next) = ctx.succs.get(&bn) {
                frontier.extend(next.iter().cloned());
            }
        }
        ctx.reachable_memo.insert(from.to_owned(), seen);
    }
    ctx.reachable_memo[from].contains(block)
}

/// Whether the named block survived SCCP (a use in a proven-dead block is
/// no evidence of liveness).
fn executable(ctx: &SharingCtx<'_>, block: &str) -> bool {
    ctx.cfg
        .block_id(block)
        .is_some_and(|id| ctx.executable_blocks.contains(&id))
}

/// Best-effort source span for a qualifying use site.
fn use_span(ctx: &SharingCtx<'_>, u: &UseSite) -> Option<Span> {
    let id = ctx.cfg.block_id(&u.block)?;
    match u.kind {
        UseKind::Operand => {
            let idx = usize::try_from(u.statement_index).ok()?;
            ctx.ssa
                .blocks
                .get(&id)
                .and_then(|b| b.statements.get(idx))
                .map(|ss| ss.statement.span())
        }
        UseKind::Terminator => ctx
            .cfg
            .blocks
            .get(&id)
            .and_then(|b| b.terminator.as_ref())
            .and_then(crate::cfg::Terminator::span),
        UseKind::PhiIncoming => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::side_effects::{SideEffect, SideEffectTarget};
    use tcl_registry::{ArgRole, Arity, CommandSpec, RepresentationEffect, Traits};

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn warnings_for_fn_with_registry(
        src: &str,
        name: &str,
        registry: &CommandRegistry,
    ) -> Vec<SharingWarning> {
        let cu = CompilationUnit::build_for(src, registry, false);
        let fu = cu.function(name).unwrap();
        find_sharing_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.def_use,
            &fu.sccp.executable_blocks,
            registry,
            None::<&HashSet<String>>,
        )
    }

    fn warnings_for_fn(src: &str, name: &str) -> Vec<SharingWarning> {
        warnings_for_fn_with_registry(src, name, &registry())
    }

    fn warnings_for(src: &str) -> Vec<SharingWarning> {
        warnings_for_fn(src, "::top")
    }

    /// TP: the tight pattern — `set b $a`, `lappend b`, `a` read afterwards.
    /// tclsh 8.6.14 (pointer identity is the evidence):
    ///
    /// ```text
    /// set a [lrepeat 1000 x]; set b $a
    ///   b before:  object pointer at 0x...eff0   (same object as a)
    ///   lappend b y
    ///   b after:   object pointer at 0x...fa10   (fresh duplicate)
    ///   a after:   object pointer at 0x...eff0   (original, untouched)
    /// ```
    #[test]
    fn s103_fires_for_copy_then_lappend_with_source_live() {
        let w = warnings_for("set a [lrepeat 1000 x]\nset b $a\nlappend b y\nputs [llength $a]");
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
        assert_eq!(w[0].variable, "b");
        assert_eq!(w[0].shared_with, "a");
        assert_eq!(w[0].command, "lappend");
        assert_eq!(w[0].code, DiagCode::S103);
    }

    #[test]
    fn s103_consumes_representation_effect_without_command_name_knowledge() {
        let mut registry = registry();
        registry.insert(CommandSpec {
            name: "bagmutate",
            traits: Traits::READS_BEFORE_WRITE | Traits::FIRST_ARG_VARNAME,
            arity: Arity::at_least(2),
            arg_roles: &[(0, ArgRole::VarWrite)],
            assigns_variable_at: Some(0),
            representation_effect: Some(RepresentationEffect::copy_on_write_container(0, 2)),
            side_effects: &[SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            }],
            ..CommandSpec::DEFAULT
        });
        let warnings = warnings_for_fn_with_registry(
            "set a [list 1 2 3]\nset b $a\nbagmutate b y\nputs [llength $a]",
            "::top",
            &registry,
        );

        assert_eq!(
            warnings.len(),
            1,
            "registry-declared mutation: {warnings:?}"
        );
        assert_eq!(warnings[0].command, "bagmutate");
    }

    /// TP: `lset` on the copy (tclsh-verified: `set d $c; lset d 0 z` gives
    /// `d` a fresh object pointer while `c` keeps the original).
    #[test]
    fn s103_fires_for_lset_on_shared_copy() {
        let w = warnings_for("set c [list 1 2 3]\nset d $c\nlset d 0 z\nputs [llength $c]");
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
        assert_eq!(w[0].command, "lset");
    }

    /// TP: `dict set` on the copy (tclsh-verified: `set f $e; dict set f k3
    /// v3` gives `f` a fresh object pointer while `e` keeps the original).
    #[test]
    fn s103_fires_for_dict_set_on_shared_copy() {
        let w = warnings_for(
            "set e [dict create k1 v1]\nset f $e\ndict set f k3 v3\nputs [dict size $e]",
        );
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
        assert_eq!(w[0].command, "dict");
        assert_eq!(w[0].variable, "f");
    }

    /// TP, symmetric direction: mutating the SOURCE while the copy lives
    /// (tclsh-verified: `set h $g; lappend g z` gives `g` a fresh pointer
    /// while `h` keeps the original) — the task design's case (b).
    #[test]
    fn s103_fires_for_mutating_the_source_while_copy_lives() {
        let w = warnings_for("set g [list 1 2 3]\nset h $g\nlappend g z\nputs [llength $h]");
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
        assert_eq!(w[0].variable, "g");
        assert_eq!(w[0].shared_with, "h");
    }

    /// TP: the quoted pure-copy spelling `set b "$a"` shares the same
    /// object (tclsh-verified: identical object pointer, and `lappend b`
    /// then duplicates), so it forms a pair too.
    #[test]
    fn s103_fires_for_quoted_pure_copy() {
        let w = warnings_for("set a [list 1 2 3]\nset b \"$a\"\nlappend b y\nputs [llength $a]");
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
    }

    /// TP: the source may be a proc parameter — the in-proc `set b $lst`
    /// alias shares with the parameter slot, so the first `lappend b`
    /// duplicates (tclsh-verified inside a proc: `b` gets a fresh pointer).
    /// Only *direct* parameter mutation is the excluded idiom.
    #[test]
    fn s103_fires_for_copy_from_parameter_still_read() {
        let w = warnings_for_fn(
            "proc p {lst} {\n  set b $lst\n  lappend b x\n  return [llength $lst]\n}",
            "::p",
        );
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
        assert_eq!(w[0].variable, "b");
        assert_eq!(w[0].shared_with, "lst");
    }

    /// TP: the live partner read may sit in a later (reachable) block.
    #[test]
    fn s103_fires_when_partner_read_in_later_block() {
        let w = warnings_for_fn(
            "proc p {c} {\n  set a [list 1 2 3]\n  set b $a\n  lappend b x\n  if {$c} { puts [llength $a] }\n  return $b\n}",
            "::p",
        );
        assert_eq!(w.len(), 1, "expected exactly one S103: {w:?}");
    }

    /// FP guard: mutating a parameter directly is the accepted idiom — the
    /// caller's copy is the normal case; silent.
    #[test]
    fn s103_silent_for_direct_parameter_mutation() {
        let w = warnings_for_fn("proc p {l} {\n  lappend l x\n  return $l\n}", "::p");
        assert!(w.is_empty(), "direct param mutation must be silent: {w:?}");
    }

    /// FP guard: an explicit copy (`lrange $a 0 end`) is not a pure-var-ref
    /// assignment — no pair, silent.
    #[test]
    fn s103_silent_for_explicit_copy() {
        let w = warnings_for(
            "set a [list 1 2 3]\nset b [lrange $a 0 end]\nlappend b y\nputs [llength $a]",
        );
        assert!(w.is_empty(), "explicit copy must be silent: {w:?}");
    }

    /// FP guard: the source is dead past the mutation — the sharing is not
    /// wanted, silent (deliberate under-approximation; see module docs).
    #[test]
    fn s103_silent_when_source_dead_after_mutation() {
        let w = warnings_for("set a [list 1 2 3]\nset b $a\nlappend b y\nputs [llength $b]");
        assert!(w.is_empty(), "dead source must be silent: {w:?}");
    }

    /// FP guard: the source was redefined before the mutation — `b` is the
    /// sole holder at the write (tclsh-verified: `set n $m; set m "gone";
    /// lappend n y` keeps `n`'s object pointer — no duplicate), silent.
    #[test]
    fn s103_silent_when_source_redefined_before_mutation() {
        let w = warnings_for(
            "set m [list 1 2 3]\nset n $m\nset m gone\nlappend n y\nputs [llength $n]\nputs $m",
        );
        assert!(w.is_empty(), "redefined source must be silent: {w:?}");
    }

    /// FP guard: array elements conflate onto one SSA symbol per array —
    /// a pair involving one may mix distinct runtime slots (FP-SH-13);
    /// silent.
    #[test]
    fn s103_silent_for_array_elements() {
        let w = warnings_for(
            "set arr(src) [list 1 2 3]\nset b $arr(src)\nlappend b y\nputs [llength $arr(src)]",
        );
        assert!(w.is_empty(), "array-element source must be silent: {w:?}");
        let w = warnings_for(
            "set a [list 1 2 3]\nset arr(dst) $a\nlappend arr(dst) y\nputs [llength $a]",
        );
        assert!(w.is_empty(), "array-element target must be silent: {w:?}");
    }

    /// FP guard: a second mutation of the same variable consumes the
    /// *duplicate's* version, not the copy's — only the first mutation
    /// pays the copy, so only one S103 fires.
    #[test]
    fn s103_fires_once_for_repeated_mutation() {
        let w = warnings_for(
            "set a [list 1 2 3]\nset b $a\nlappend b x\nlappend b y\nputs [llength $a]",
        );
        assert_eq!(w.len(), 1, "only the first mutation copies: {w:?}");
    }

    /// FP guard: `upvar`-aliased names abstain (another route reaches the
    /// slot), mirroring the sibling passes' alias guards.
    #[test]
    fn s103_silent_for_upvar_aliased_name() {
        let w = warnings_for_fn(
            "proc p {} {\n  upvar 1 outer b\n  set a [list 1 2 3]\n  set b $a\n  lappend b y\n  puts [llength $a]\n}",
            "::p",
        );
        assert!(w.is_empty(), "upvar-aliased target must be silent: {w:?}");
    }

    /// FP guard: an `eval` barrier between the copy and the mutation can
    /// rewrite either variable without an SSA version bump — abstain.
    #[test]
    fn s103_silent_across_barrier() {
        let w = warnings_for(
            "set a [list 1 2 3]\nset b $a\neval $::code\nlappend b y\nputs [llength $a]",
        );
        assert!(
            w.is_empty(),
            "a barrier between copy and mutation must abstain: {w:?}"
        );
    }

    /// FP guard: non-mutating arities — `lappend b` (read/create only) and
    /// `lset b v` (whole-value replacement, no surgery on the old value) —
    /// stay silent.
    #[test]
    fn s103_silent_for_non_mutating_arities() {
        let w = warnings_for("set a [list 1 2 3]\nset b $a\nlappend b\nputs [llength $a]");
        assert!(
            w.is_empty(),
            "`lappend b` with no values must be silent: {w:?}"
        );
        let w = warnings_for("set a [list 1 2 3]\nset b $a\nlset b {}\nputs [llength $a]");
        assert!(
            w.is_empty(),
            "index-less `lset` is a plain assignment: {w:?}"
        );
    }

    /// The warning's related spans anchor the copy statement and the
    /// still-live read, and the primary span covers the mutation.
    #[test]
    fn s103_spans_anchor_mutation_copy_and_read() {
        let src = "set a [list 1 2 3]\nset b $a\nlappend b y\nputs [llength $a]";
        let w = warnings_for(src);
        assert_eq!(w.len(), 1, "{w:?}");
        let text = |sp: Span| &src[sp.start() as usize..sp.end() as usize];
        assert_eq!(text(w[0].span), "lappend b y");
        assert_eq!(w[0].related.len(), 2, "{:?}", w[0].related);
        assert_eq!(text(w[0].related[0].0), "set b $a");
        assert_eq!(text(w[0].related[1].0), "puts [llength $a]");
    }
}
