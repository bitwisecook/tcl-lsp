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

//! Use-site shimmer detection (S100 / S101).
//!
//! A use-site shimmer occurs when a variable holds a value of type A but
//! is passed to a command argument position that expects type B.  Tcl
//! silently converts the internal representation at runtime — e.g. a
//! `String` to `Int` for `incr`, or a `String` to `List` for `llength` —
//! potentially invalidating cached intreps on shared references.
//!
//! Diagnostics:
//! - **S100**: shimmer outside a loop.
//! - **S101**: shimmer inside a loop (higher severity — converts on every
//!   iteration).
//!
//! This pass covers:
//! - [`Statement::Call`] arguments tagged `shimmers=true` in the registry.
//! - [`Statement::Incr`] — always reads its variable as `Int`.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, TclType};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::Function as CfgFunction;
use crate::command_binding::ModuleCommandMutations;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice};
use crate::value_shapes::is_pure_var_ref;

use super::graph::loop_body_blocks;
use super::hints::{arg_shimmer_type, is_numeric_compatible};
use super::span::def_range_map;
use super::{ShimmerWarning, type_name};

/// Find use-site shimmer warnings for a function.
///
/// Walks every executable block in CFG order, checks each statement's
/// argument words against the registry's `arg_types`, and emits a
/// [`ShimmerWarning`] for each type mismatch where the variable's known
/// type differs from what the command requires.
///
/// `mutations` (module-wide `rename` / `interp alias` / proc-redefinition
/// facts, from [`crate::command_binding::scan_module_command_mutations`])
/// gates whether a literal command spelling may still be trusted to denote
/// its registry builtin: a body that renames `incr`/`lindex`/… away (or
/// shadows it with a user proc) makes every occurrence of that bare name
/// untrustworthy module-wide (the sound over-approximation
/// `ModuleCommandMutations` already uses for the optimiser's builtin-fold
/// gate — see its doc comment), so the corresponding shimmer check is
/// suppressed rather than risk a false claim about a command that may no
/// longer mean what its name suggests. For a top-level [`Statement::Call`],
/// an `interp alias`-created name is additionally *resolved* to its target
/// via [`crate::ir::Statement::canonical_command_or_source`] before the
/// lookup, so a call through the alias is checked exactly like a call to
/// the target itself — `canonical_command` is populated at IR-lowering
/// time from the module-wide alias table. A `[cmd …]` substitution
/// embedded in a `Statement::AssignValue` has no such pre-resolved field
/// (it's parsed ad hoc from source text by
/// [`crate::value_shapes::parse_command_substitution`], not lowered as its
/// own bound `Statement::Call`), so an alias used in that position is
/// looked up — and `mutations.trusts()`-gated — under its literal alias
/// spelling: `set y [li $x 0]` is not checked as a `lindex` call. Exposing
/// the lowering pass's alias table for this ad-hoc lookup too is a known
/// gap, not yet done.
///
/// `inputs.source` is the whole compilation unit's source text — used only
/// to recover a tight per-argument span for a `[cmd …]` substitution
/// embedded in a `Statement::AssignValue` (`set y [lindex $x 0]`), which
/// carries no per-word `CommandTokens` of its own; see
/// [`check_invocation`]'s doc comment and [`nested_call_arg_spans`].
#[must_use]
pub(crate) fn find_use_site_shimmers(inputs: &super::ShimmerInputs<'_>) -> Vec<ShimmerWarning> {
    let cfg = inputs.cfg;
    let ssa = inputs.ssa;
    let types = inputs.types;
    let executable_blocks = inputs.executable_blocks;
    let registry = inputs.registry;
    let values = inputs.values;
    let mutations = inputs.mutations;
    let source = inputs.source;

    let loop_blocks = loop_body_blocks(cfg);
    let def_map = def_range_map(ssa);
    // Loop-invariance facts for the S101→S100 downgrade — only needed when
    // the function has at least one loop block.
    let loop_facts = LoopFacts::compute(cfg, ssa, &loop_blocks, registry, mutations);
    let mut out: Vec<ShimmerWarning> = Vec::new();

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        let in_loop = loop_blocks.contains(cfg.block_name(block_id));
        // Per-block coercion ledger: once a use coerces `(var, ver)` to a
        // target intrep, the runtime representation has already changed, so a
        // later use to the *same* target in the same block is not a second
        // shimmer.
        let mut already_coerced: HashSet<(String, u32, TclType)> = HashSet::new();
        let mut ctx = UseSiteCtx {
            types,
            registry,
            def_map: &def_map,
            values,
            loop_facts: &loop_facts,
            ssa,
            mutations,
            source,
            in_loop,
            already_coerced: &mut already_coerced,
            out: &mut out,
        };
        for ss in &ssa_block.statements {
            check_statement(&mut ctx, &ss.statement, &ss.uses);
        }
    }

    out
}

/// Function-wide read-only context + per-block warning sinks threaded
/// through the use-site shimmer walk.  `in_loop` / `already_coerced` are
/// per-block; `out` accumulates across the whole function.
struct UseSiteCtx<'a> {
    types: &'a HashMap<ValueKey, TypeLattice>,
    registry: &'a CommandRegistry,
    def_map: &'a HashMap<ValueKey, Span>,
    values: &'a HashMap<ValueKey, LatticeValue>,
    loop_facts: &'a LoopFacts,
    ssa: &'a SsaFunction,
    mutations: &'a ModuleCommandMutations,
    source: &'a str,
    in_loop: bool,
    already_coerced: &'a mut HashSet<(String, u32, TclType)>,
    out: &'a mut Vec<ShimmerWarning>,
}

/// Loop-invariance facts used to refine the in-loop shimmer classification.
///
/// A variable used inside a loop is "per-iteration" (S101) only if its
/// intrep can be reset each iteration — i.e. something in the loop body
/// defines it. A loop-*invariant* variable (no def inside any loop block)
/// shimmers once at the first iteration and is cached for the rest, so the
/// right code is S100 (one-time) — *unless* it is coerced to two or more
/// distinct target intreps inside the loop, in which case the converters
/// re-thunk it each pass (genuine S101).
#[derive(Default)]
struct LoopFacts {
    /// Names defined anywhere in a loop block (statement defs + phis).
    def_names: HashSet<String>,
    /// Per-name set of expected intreps requested at any use-site in a
    /// loop block.
    use_targets: HashMap<String, HashSet<TclType>>,
}

impl LoopFacts {
    fn compute(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        loop_blocks: &HashSet<String>,
        registry: &CommandRegistry,
        mutations: &ModuleCommandMutations,
    ) -> Self {
        let mut facts = Self::default();
        if loop_blocks.is_empty() {
            return facts;
        }
        for lbn in loop_blocks {
            let Some(id) = cfg.block_id(lbn) else {
                continue;
            };
            if let Some(sb) = ssa.blocks.get(&id) {
                for st in &sb.statements {
                    facts
                        .def_names
                        .extend(st.defs.keys().map(|&sym| ssa.var_name(sym).to_owned()));
                }
                for phi in &sb.phis {
                    facts.def_names.insert(ssa.var_name(phi.name).to_owned());
                }
            }
            if let Some(cb) = cfg.blocks.get(&id) {
                for stmt in &cb.statements {
                    facts.record_use_targets(stmt, registry, mutations);
                }
            }
        }
        facts
    }

    /// Record the expected intrep of every `$var` argument at a shimmering
    /// position of `stmt` (a direct call or a `[cmd …]` substitution
    /// inside an assignment value).
    fn record_use_targets(
        &mut self,
        stmt: &Statement,
        registry: &CommandRegistry,
        mutations: &ModuleCommandMutations,
    ) {
        let mut record = |command: &str, args: &[String]| {
            if !mutations.trusts(command) {
                return;
            }
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            for (i, word) in args.iter().enumerate() {
                if !word.trim_start().starts_with('$') {
                    continue;
                }
                if let Some(expected) = arg_shimmer_type(registry, command, &arg_refs, i) {
                    let var = normalise_var_name(word.trim()).to_owned();
                    self.use_targets.entry(var).or_default().insert(expected);
                }
            }
        };
        match stmt {
            Statement::Call { args, .. } => record(stmt.canonical_command_or_source(), args),
            Statement::AssignValue { value, .. } => {
                if let Some((command, args)) =
                    crate::value_shapes::parse_command_substitution(value.trim())
                {
                    record(&command, &args);
                }
            }
            _ => {}
        }
    }

    /// Refine `in_loop` for a single use of `var`: a loop-invariant variable
    /// coerced to fewer than two distinct intreps inside the loop converts
    /// once and is cached, so it is S100 (not S101).
    fn effective_in_loop(&self, var: &str, in_loop: bool) -> bool {
        if in_loop && !self.def_names.contains(var) {
            self.use_targets.get(var).is_some_and(|t| t.len() >= 2)
        } else {
            in_loop
        }
    }
}

/// True when `value` is a SCCP CONST string whose text is a hex /
/// octal / binary integer literal (`0xff`, `0o15`, `0b1010`, optionally
/// signed). These spellings classify as `String` by `literal_type` (the
/// canonical stringified intrep differs from the source text) but
/// promote cleanly to `Int` at the first arithmetic op — not a real
/// shimmer.
fn value_is_int_literal_string(value: Option<&LatticeValue>) -> bool {
    let Some(LatticeValue::Const(ConstValue::String(text))) = value else {
        return false;
    };
    let sign_stripped = text.strip_prefix(['+', '-']).unwrap_or(text);
    let bytes = sign_stripped.as_bytes();
    // Need at least a `0x`-style prefix plus one body digit.
    if bytes.len() < 3 || bytes[0] != b'0' {
        return false;
    }
    let body = &sign_stripped[2..];
    match bytes[1] {
        b'x' | b'X' => body.bytes().all(|c| c.is_ascii_hexdigit()),
        b'o' | b'O' => body.bytes().all(|c| (b'0'..=b'7').contains(&c)),
        b'b' | b'B' => body.bytes().all(|c| c == b'0' || c == b'1'),
        _ => false,
    }
}

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`).
///
/// `arg_spans` — the call's own per-word spans, when available, in
/// command-name-then-args order (so arg `i` is `arg_spans[i + 1]`) — narrows
/// each warning's span to just the offending `$var` argument rather than the
/// whole statement, so a shimmer on one argument of a multi-argument call
/// (`dict get $bigMap $key1 $key2`) underlines only that argument. For a
/// top-level `Statement::Call` this is `tokens.argv`; for the
/// [`Statement::AssignValue`] embedded-command-substitution case (whose own
/// `tokens` describe the outer `set`'s words, not the nested call's) it is
/// computed by locating the substitution text directly in `ctx.source` — see
/// [`nested_call_arg_spans`]. `None` (source unavailable, or the
/// substitution text can't be located unambiguously) falls back to `span`,
/// the whole statement.
fn check_invocation(
    ctx: &mut UseSiteCtx<'_>,
    command: &str,
    args: &[String],
    span: Span,
    arg_spans: Option<&[Span]>,
    uses: &HashMap<Symbol, u32>,
) {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for (i, word) in args.iter().enumerate() {
        let Some(expected) = arg_shimmer_type(ctx.registry, command, &arg_refs, i) else {
            continue;
        };
        // Only flag pure variable references — complex words may produce
        // the right type via their own evaluation.
        let stripped = word.trim();
        if !is_pure_var_ref(stripped) {
            continue;
        }
        let var = normalise_var_name(stripped).to_owned();
        let Some(sym) = ctx.ssa.var_symbol(&var) else {
            continue;
        };
        let Some(&ver) = uses.get(&sym) else { continue };
        if ver == 0 {
            continue;
        }
        let lattice = ctx
            .types
            .get(&(sym, ver))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown);
        if lattice.kind != TypeKind::Known {
            continue;
        }
        let Some(current) = lattice.tcl_type else {
            continue;
        };
        if current == expected || is_numeric_compatible(current, expected) {
            continue;
        }
        // A prior use in this block already coerced `(var, ver)` to this
        // intrep — the runtime representation has already changed, so this
        // is not a second shimmer.
        let coercion_key = (var.clone(), ver, expected);
        if !ctx.already_coerced.insert(coercion_key) {
            continue;
        }
        let related: Vec<(Span, String)> = ctx
            .def_map
            .get(&(sym, ver))
            .map(|&sp| vec![(sp, "value defined here".to_owned())])
            .unwrap_or_default();
        // A loop-invariant variable coerced to a single intrep inside the
        // loop converts once and is cached → S100, not the per-iteration
        // S101.
        let code = if ctx.loop_facts.effective_in_loop(&var, ctx.in_loop) {
            DiagCode::S101
        } else {
            DiagCode::S100
        };
        let arg_span = arg_spans
            .and_then(|s| s.get(i + 1))
            .copied()
            .unwrap_or(span);
        ctx.out.push(ShimmerWarning {
            span: arg_span,
            variable: var.clone(),
            from_type: current,
            to_type: expected,
            command: command.to_owned(),
            in_loop: ctx.in_loop,
            code,
            message: format!(
                "{code}: variable '{var}' has {from} intrep \
                 but '{cmd}' expects {to} at arg {i}",
                from = type_name(current),
                cmd = command,
                to = type_name(expected),
            ),
            related,
            fixes: Vec::new(),
        });
    }
}

/// Recover absolute per-argument spans for the arguments of a `[cmd ARG…]`
/// command substitution embedded in a `Statement::AssignValue`'s `value`
/// text (`set y [lindex $x 0]`). Such statements carry no per-word
/// `CommandTokens` for the *nested* call — only the outer `set`'s own words
/// — so this locates each argument directly in `source`: first the
/// substitution text itself (`sub_text`, e.g. `"[lindex $x 0]"`) within the
/// statement's source slice, then each argument word in turn from a cursor
/// that only advances, so repeated words (`[lindex $x $x]`) each resolve to
/// their own occurrence in order rather than all matching the first one.
///
/// Returns spans in the same command-name-then-args convention as
/// [`crate::ir::CommandTokens::argv`] (index 0 is an unused placeholder, arg
/// `i` is index `i + 1`) so [`check_invocation`] can index either source
/// identically. Returns `None` — callers fall back to the whole-statement
/// span — when `source` is empty (offset-0 memoised queries don't carry
/// source text; see `compiler_checks::function_nontaint_checks`'s doc
/// comment) or the substitution text can't be located unambiguously (it
/// must appear in the statement's source slice exactly once).
fn nested_call_arg_spans(
    source: &str,
    stmt_span: Span,
    sub_text: &str,
    args: &[String],
) -> Option<Vec<Span>> {
    if source.is_empty() {
        return None;
    }
    let stmt_start = usize::try_from(stmt_span.start()).ok()?;
    let stmt_end = usize::try_from(stmt_span.end()).ok()?;
    let stmt_text = source.get(stmt_start..stmt_end)?;
    let sub_start = stmt_text.find(sub_text)?;
    if stmt_text[sub_start + 1..].contains(sub_text) {
        return None;
    }

    let mut cursor = 0usize;
    let mut spans = vec![Span::empty(stmt_span.start())];
    for arg in args {
        let found = sub_text[cursor..]
            .match_indices(arg.as_str())
            .find_map(|(i, _)| {
                let abs = cursor + i;
                super::is_standalone_word_at(sub_text, abs, arg.len()).then_some(abs)
            })?;
        cursor = found + arg.len();
        let abs_start = stmt_span.start() + u32::try_from(sub_start + found).ok()?;
        let abs_end = abs_start + u32::try_from(arg.len()).ok()?;
        spans.push(Span::new(abs_start, abs_end));
    }
    Some(spans)
}

fn check_statement(ctx: &mut UseSiteCtx<'_>, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
    match stmt {
        Statement::Call { args, tokens, .. } => {
            // Resolve through an `interp alias` target (`canonical_command_or_source`
            // falls back to the source spelling when there's no alias) and
            // require the resolved name to still be trusted module-wide — a
            // body that `rename`s it away, or redefines it as a user proc,
            // makes every occurrence of the bare name untrustworthy (see this
            // function's doc comment).
            let command = stmt.canonical_command_or_source();
            if ctx.mutations.trusts(command) {
                let arg_spans = tokens.as_ref().map(|t| t.argv.as_slice());
                check_invocation(ctx, command, args, stmt.span(), arg_spans, uses);
            }
        }

        // A command substitution lifted into an assignment value
        // (`set b [lindex $x 0]`) reads its arguments just like a direct
        // call. The statement's own `tokens` (when present) describe the
        // outer `set`'s words, not the nested substitution's, so per-argument
        // spans are instead recovered from the source text directly — see
        // [`nested_call_arg_spans`]. Unlike the `Statement::Call` arm above,
        // `command` here is *not* resolved through `interp alias` — see
        // this module's top doc comment for why (no pre-lowered
        // `canonical_command` exists for an ad-hoc-parsed substitution).
        Statement::AssignValue { value, .. } => {
            let trimmed = value.trim();
            if let Some((command, args)) = crate::value_shapes::parse_command_substitution(trimmed)
                && ctx.mutations.trusts(&command)
            {
                let arg_spans = nested_call_arg_spans(ctx.source, stmt.span(), trimmed, &args);
                check_invocation(
                    ctx,
                    &command,
                    &args,
                    stmt.span(),
                    arg_spans.as_deref(),
                    uses,
                );
            }
        }

        // `incr` reads both its target variable and (when present) its
        // increment argument as Int.  The two checks are independent — an
        // Int target with a String `$amount` still shimmers on the amount —
        // so neither must short-circuit the other. `incr` is lowered to this
        // dedicated statement purely lexically (no alias/rename resolution
        // happens at that point — see `lowering/hooks/incr.rs`), so the
        // *only* available mitigation for a renamed-away `incr` is to
        // suppress the check entirely when the module untrusts the name;
        // there is no alternate target name to redirect the check to.
        Statement::Incr { name, amount, .. } if ctx.mutations.trusts("incr") => {
            check_incr_var(ctx, normalise_var_name(name), stmt.span(), uses);
            if let Some(amt) = amount.as_deref().map(str::trim)
                && amt.starts_with('$')
            {
                check_incr_var(ctx, normalise_var_name(amt), stmt.span(), uses);
            }
        }

        _ => {}
    }
}

/// Check one `incr` operand (the target variable or the increment
/// argument) for an intrep shimmer to `Int`.
///
/// `var` is the normalised variable name. Fires when its known intrep is a
/// non-int, non-numeric type that is not a clean hex/octal/binary integer
/// literal string (that spelling promotes cleanly to int).
fn check_incr_var(ctx: &mut UseSiteCtx<'_>, var: &str, span: Span, uses: &HashMap<Symbol, u32>) {
    let Some(sym) = ctx.ssa.var_symbol(var) else {
        return;
    };
    let Some(&ver) = uses.get(&sym) else { return };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type else {
        return;
    };
    if current == TclType::Int || is_numeric_compatible(current, TclType::Int) {
        return;
    }
    if value_is_int_literal_string(ctx.values.get(&(sym, ver))) {
        return;
    }
    let related: Vec<(Span, String)> = ctx
        .def_map
        .get(&(sym, ver))
        .map(|&sp| vec![(sp, "value defined here".to_owned())])
        .unwrap_or_default();
    let code = if ctx.in_loop {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    ctx.out.push(ShimmerWarning {
        span,
        variable: var.to_owned(),
        from_type: current,
        to_type: TclType::Int,
        command: "incr".to_owned(),
        in_loop: ctx.in_loop,
        code,
        message: format!(
            "{code}: variable '{var}' has {from} intrep but 'incr' expects int",
            from = type_name(current),
        ),
        related,
        fixes: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::shimmer::ShimmerInputs;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// A String variable passed to `incr` triggers S100.
    #[test]
    fn shimmer_detected_for_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings
            .iter()
            .find(|w| w.command == "incr" && w.from_type == TclType::String);
        assert!(
            w.is_some(),
            "expected incr/String shimmer, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().to_type, TclType::Int);
    }

    /// A hex literal string (`0x80`) classifies as
    /// String but promotes cleanly to Int at `incr` — no shimmer.
    #[test]
    fn no_shimmer_for_hex_literal_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set n 0x80\nincr n", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "0x80 promotes cleanly to int — no incr shimmer expected: {incr_shimmers:?}",
        );
    }

    /// An Int variable passed to `incr` has no shimmer.
    #[test]
    fn no_shimmer_for_int_used_with_incr() {
        let cu = CompilationUnit::build_for("set x 5\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "unexpected incr shimmer for Int: {incr_shimmers:?}"
        );
    }

    /// An Int variable passed to `lindex` should trigger a shimmer
    /// (Int → List at arg 0).
    #[test]
    fn shimmer_detected_for_int_used_with_lindex() {
        let cu = CompilationUnit::build_for("set x 5\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex shimmer for Int var, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::Int);
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// The warning span is the offending `$var` argument itself, not the
    /// whole call — precision matters more the more other arguments a call
    /// has: `dict get $badMap key1 key2` should underline just `$badMap`,
    /// not the whole three-argument statement.
    #[test]
    fn shimmer_span_is_the_argument_not_the_whole_statement() {
        let src = "set badMap 5\ndict get $badMap key1 key2";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings
            .iter()
            .find(|w| w.command == "dict" && w.variable == "badMap")
            .unwrap_or_else(|| panic!("expected a dict-get shimmer, got: {warnings:?}"));
        let stmt_start = u32::try_from(src.find("dict get").unwrap()).unwrap();
        let stmt_end = u32::try_from(src.len()).unwrap();
        let arg_start = u32::try_from(src.find("$badMap").unwrap()).unwrap();
        let arg_end = arg_start + u32::try_from("$badMap".len()).unwrap();
        assert_eq!(
            (w.span.start(), w.span.end()),
            (arg_start, arg_end),
            "expected the span to cover exactly '$badMap' ({arg_start}..{arg_end}), \
             not the whole statement ({stmt_start}..{stmt_end}): got {:?}",
            w.span
        );
    }

    /// A `foreach` element variable is typed String (list elements
    /// stringify); using it as a list intrep inside the loop body via a
    /// `[lindex $x 0]` command substitution re-thunks each iteration —
    /// per-iteration shimmer (S101). The substitution lives in an
    /// `AssignValue` value, so this exercises the `AssignValue` arm.
    #[test]
    fn shimmer_detected_for_foreach_var_used_as_list_in_cmd_sub() {
        let src = "proc f {l} {\n    foreach x $l {\n        set b [lindex $x 0]\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex S101 for foreach var, got: {warnings:?}"
        );
        let w = w.unwrap();
        assert_eq!(w.code, DiagCode::S101);
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
        // Tight highlighting: `set b [lindex $x 0]` is a `Statement::
        // AssignValue` (the `[lindex …]` is a command substitution nested in
        // the value, not a top-level `Statement::Call`), which carries no
        // per-word tokens of its own — the span must still cover just the
        // `$x` argument, recovered from source via `nested_call_arg_spans`,
        // not the whole `set b [lindex $x 0]` statement.
        let arg_start = u32::try_from(src.find("$x").unwrap()).unwrap();
        let arg_end = arg_start + u32::try_from("$x".len()).unwrap();
        assert_eq!(
            (w.span.start(), w.span.end()),
            (arg_start, arg_end),
            "expected the span to cover exactly '$x' ({arg_start}..{arg_end}), \
             not the whole statement: got {:?}",
            w.span
        );
    }

    /// A loop-invariant variable (defined outside the loop) coerced to a
    /// single intrep inside the loop converts once and is cached — that is a
    /// one-time S100, not the per-iteration S101.
    #[test]
    fn loop_invariant_single_target_downgrades_to_s100() {
        let src = "proc f {} {\n  set data {1 2 3}\n  for {set i 0} {$i < 3} {incr i} {\n    set x [lindex $data $i]\n  }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex" && w.variable == "data");
        assert!(
            w.is_some(),
            "expected lindex shimmer for data: {warnings:?}"
        );
        assert_eq!(
            w.unwrap().code,
            DiagCode::S100,
            "loop-invariant single-target use is one-time (S100): {warnings:?}"
        );
    }

    /// `incr n $step` where `$step` holds a String shimmers on the increment
    /// argument (Tcl coerces it to int).
    #[test]
    fn incr_amount_string_var_shimmers() {
        let cu = CompilationUnit::build_for(
            "set n 0\nset step \"hello\"\nincr n $step",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings
            .iter()
            .find(|w| w.command == "incr" && w.variable == "step");
        assert!(
            w.is_some(),
            "expected incr-amount shimmer for step: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::String);
        assert_eq!(w.unwrap().to_type, TclType::Int);
    }

    /// `foreach x $s { ... }` where `$s` is String-typed (e.g. also used as
    /// a plain string elsewhere) shimmers on `foreach`'s own list argument —
    /// iterating requires a List intrep. Exercises the
    /// `Traits::LOOP_LIST_HEADER`-driven structural rule in `hints.rs`.
    #[test]
    fn shimmer_detected_for_foreach_list_argument_itself() {
        let src =
            "proc f {} {\n  set s \"a b c\"\n  string length $s\n  foreach x $s { puts $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        let w = warnings
            .iter()
            .find(|w| w.command == "foreach" && w.variable == "s");
        assert!(
            w.is_some(),
            "expected foreach shimmer for String var 's', got: {warnings:?}"
        );
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// Variables with Unknown type do not produce false-positive shimmers.
    #[test]
    fn no_shimmer_for_unknown_type() {
        // `set x $other` — type of x is Unknown (other has no known type).
        let cu = CompilationUnit::build_for("set x $other\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry: &registry(),
            values: &fu.sccp.values,
            mutations: &ModuleCommandMutations::default(),
            source: &cu.source,
        });
        // x has Unknown type; should not produce a shimmer.
        let lindex_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "lindex").collect();
        assert!(
            lindex_shimmers.is_empty(),
            "unexpected shimmer for Unknown type: {lindex_shimmers:?}"
        );
    }
}
