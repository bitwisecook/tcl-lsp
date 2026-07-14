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
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice};
use crate::value_shapes::is_pure_var_ref;

use super::graph::loop_body_blocks;
use super::hints::{
    ShimmerExpectation, arg_shimmer_expectation, arg_shimmer_type, is_numeric_compatible,
};
use super::span::def_range_map;
use super::{ShimmerWarning, type_name};

/// Find use-site shimmer warnings for a function.
///
/// Walks every executable block in CFG order, checks each statement's
/// argument words against the registry's `arg_types`, and emits a
/// [`ShimmerWarning`] for each type mismatch where the variable's known
/// type differs from what the command requires.
#[must_use]
pub(crate) fn find_use_site_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Vec<ShimmerWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let def_map = def_range_map(ssa);
    // Loop-invariance facts for the S101→S100 downgrade — only needed when
    // the function has at least one loop block.
    let loop_facts = LoopFacts::compute(cfg, ssa, &loop_blocks, registry);
    // Array-base symbols are excluded (FP-SH-13): `normalise_var_name` strips
    // the `(key)` suffix, so `arr(a)` and `arr(b)` share one symbol / version
    // chain. Two individually-stable but different elements then look, at a
    // use site, exactly like one variable holding the wrong intrep — the same
    // conflation the S102 pass already guards. Reuse its exclusion so S100 /
    // S101 don't false-positive on independent array elements.
    let array_syms = super::thunking::array_element_symbols(cfg, ssa);
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
            array_syms: &array_syms,
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
    /// Array-base symbols excluded from shimmer reporting (FP-SH-13) — a
    /// conflated `arr(a)`/`arr(b)` symbol can hold either element's intrep.
    array_syms: &'a HashSet<Symbol>,
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
                    facts.record_use_targets(stmt, registry);
                }
            }
        }
        facts
    }

    /// Record the expected intrep of every `$var` argument at a shimmering
    /// position of `stmt` (a direct call or a `[cmd …]` substitution
    /// inside an assignment value).
    fn record_use_targets(&mut self, stmt: &Statement, registry: &CommandRegistry) {
        let mut record = |command: &str, args: &[String]| {
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
            Statement::Call { command, args, .. } => record(command, args),
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

/// Expected intrep for every list/dict argument of a synthetic loop-header
/// call: the CFG builder lowers `foreach` / `lmap` / `dict for` / `dict map`
/// to a `Statement::Call` whose `command` is that keyword (or, for the dict
/// forms, the two-word compound `"dict for"` / `"dict map"`) and whose
/// `args` are *only* the list/dict arguments — one per iterator group, in
/// order (see `cfg_builder::cfg_lower::lower_foreach`; identified by
/// `Statement::Call::foreach_groups` being `Some`, not by the command name).
///
/// Every iterator group's argument expects the *same* intrep, so this reads
/// the registry once and the caller applies the result uniformly — unlike a
/// real call's `arg_types`, which is keyed per source-position index and
/// so can't reach a multi-group loop's later arguments at all.
///
/// `dict for` / `dict map` are two-word compound names (AGENTS.md's
/// compound-command pattern — a base command with a subcommand argument,
/// like `namespace upvar`): split at the space and dispatch through the
/// `dict` subcommand table via [`arg_shimmer_type`]'s existing subcommand
/// path, requesting sub-index 1 (both subcommands declare their dict
/// argument there).
fn foreach_header_expected_type(registry: &CommandRegistry, command: &str) -> Option<TclType> {
    if let Some((base, sub)) = command.split_once(' ') {
        // `arg_shimmer_type`'s subcommand path computes `sub_idx =
        // arg_index - 1`; requesting `arg_index = 2` reads sub-index 1.
        arg_shimmer_type(registry, base, &[sub], 2)
    } else {
        arg_shimmer_type(registry, command, &[], 0)
    }
}

/// Grouped, per-call arguments for [`check_invocation`] — keeps that
/// function's own parameter list short (the `ctx` + `uses` thread-through
/// state are the only params that vary independently of "which invocation").
#[derive(Clone, Copy)]
struct InvocationSite<'a> {
    /// Source spelling, for the warning's `command` field and message.
    command: &'a str,
    /// Registry lookup key — the *canonical* command name when the call is
    /// an `interp alias` target (`interp alias {} myindex {} ::lindex;
    /// myindex $x 0` resolves `lookup_command = "::lindex"`), so an aliased
    /// call to a shimmering builtin is still recognised, matching
    /// [`Statement::Call::canonical_command`]'s documented "diagnostics
    /// read better with the spelling the user wrote" rationale.
    lookup_command: &'a str,
    /// Argument words.
    args: &'a [String],
    /// Per-argument absolute spans, index-aligned with `args`, when
    /// available (see `fallback_span`).
    arg_spans: &'a [Span],
    /// True for the synthetic loop-header shape [`foreach_header_expected_type`]
    /// covers (`foreach` / `lmap` / `dict for` / `dict map`) — every
    /// argument shares one expected type instead of a per-index one.
    is_foreach_header: bool,
    /// Span used for any argument index `arg_spans` doesn't cover — a
    /// synthetic call built without per-word token spans (some test
    /// fixtures) or a substitution word count `arg_spans` didn't fully
    /// resolve.
    fallback_span: Span,
}

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`).
///
/// Two narrower residual gaps remain, both strictly better than today's "no
/// alias detection at all":
/// - Argument *indices* are unadjusted, so an alias that prepends fixed
///   arguments (`interp alias {} foo {} ::bar prefix`) can index-shift the
///   wrong argument.
/// - A read-modify-write shimmering argument that is a **bare variable
///   name**, not a `$`-prefixed read (`incr`/`append`/`lappend`'s target) is
///   never seen here even when aliased: `is_pure_var_ref` below only matches
///   `$`-style reads, and `incr`'s own canonical name bypasses this function
///   entirely via the dedicated [`Statement::Incr`] node (see
///   [`check_incr_var`]) — a form `lower_command` only builds for the literal
///   command name, not an alias target.
fn check_invocation(
    ctx: &mut UseSiteCtx<'_>,
    site: &InvocationSite<'_>,
    uses: &HashMap<Symbol, u32>,
) {
    let InvocationSite {
        command,
        lookup_command,
        args,
        arg_spans,
        is_foreach_header,
        fallback_span,
    } = *site;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for (i, word) in args.iter().enumerate() {
        let expectation = if is_foreach_header {
            foreach_header_expected_type(ctx.registry, lookup_command).map(|expected| {
                ShimmerExpectation {
                    expected,
                    transparent_from: &[],
                }
            })
        } else {
            arg_shimmer_expectation(ctx.registry, lookup_command, &arg_refs, i)
        };
        let Some(expectation) = expectation else {
            continue;
        };
        let expected = expectation.expected;
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
        // Skip an array base (FP-SH-13): its conflated version chain mixes
        // independent elements, so a "wrong intrep" here may just be a
        // different element's type.
        if ctx.array_syms.contains(&sym) {
            continue;
        }
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
        // The registry marks intreps this operation reads directly without
        // installing `expected` (e.g. `string length`'s pure-byte-array fast
        // path): those operands keep their rep — no shimmer.
        if expectation.is_transparent_from(current) {
            continue;
        }
        // A numeric lattice type does not prove a numeric *intrep*: a
        // literal-defined `set x 42` is a pure string at runtime
        // (tclsh-verified) with nothing for a String-installing operation to
        // destroy, and when the numeric rep IS installed its string
        // regenerates in O(digits). Only container intreps (List/Dict) lose
        // real structure to a String expectation, so numeric currents stay
        // silent there.
        if expected == TclType::String
            && matches!(
                current,
                TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
            )
        {
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
        let span = arg_spans.get(i).copied().unwrap_or(fallback_span);
        ctx.out.push(ShimmerWarning {
            span,
            variable: var.clone(),
            from_type: current,
            to_type: expected,
            command: command.to_owned(),
            in_loop: ctx.in_loop,
            code,
            message: format!(
                "variable '{var}' has {from} intrep \
                 but '{cmd}' expects {to} (argument {n})",
                from = type_name(current),
                cmd = command,
                to = type_name(expected),
                n = i + 1,
            ),
            related,
        });
    }
}

fn check_statement(ctx: &mut UseSiteCtx<'_>, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
    match stmt {
        Statement::Call {
            command,
            args,
            tokens,
            foreach_groups,
            ..
        } => {
            let lookup = stmt.canonical_command_or_source();
            // `tokens.argv[0]` is the command word; `argv[1..]` are the
            // per-argument spans, index-aligned with `args` (both are the
            // literal source words — alias resolution never reorders or
            // reparents them, see `check_invocation`'s doc comment).
            let arg_spans: Vec<Span> = tokens
                .as_ref()
                .map(|t| t.argv.iter().skip(1).copied().collect())
                .unwrap_or_default();
            check_invocation(
                ctx,
                &InvocationSite {
                    command,
                    lookup_command: lookup,
                    args,
                    arg_spans: &arg_spans,
                    is_foreach_header: foreach_groups.is_some(),
                    fallback_span: stmt.span(),
                },
                uses,
            );
        }

        // A command substitution lifted into an assignment value
        // (`set b [lindex $x 0]`) reads its arguments just like a direct
        // call. The substitution's own command word is re-parsed from raw
        // text (no lowering pass resolves it), so no `interp alias`
        // resolution is available here — the lookup key is the source
        // spelling itself.
        Statement::AssignValue { value, tokens, .. } => {
            // The value word's own absolute span (`argv`'s last entry —
            // `set name value` is always two args) anchors the re-lexed
            // substitution's relative spans; falls back to the
            // whole-statement span when tokens aren't available (some test
            // fixtures build `AssignValue` without them).
            let value_base = tokens
                .as_ref()
                .and_then(|t| t.argv.last())
                .map(|sp| sp.start());
            if let (Some(base), Some((command, args_with_spans))) = (
                value_base,
                crate::value_shapes::parse_command_substitution_with_spans(value),
            ) {
                let args: Vec<String> = args_with_spans.iter().map(|(a, _)| a.clone()).collect();
                let arg_spans: Vec<Span> = args_with_spans
                    .iter()
                    .map(|(_, rel)| Span::new(base + rel.start(), base + rel.end()))
                    .collect();
                check_invocation(
                    ctx,
                    &InvocationSite {
                        command: &command,
                        lookup_command: &command,
                        args: &args,
                        arg_spans: &arg_spans,
                        is_foreach_header: false,
                        fallback_span: stmt.span(),
                    },
                    uses,
                );
            } else if let Some((command, args)) =
                crate::value_shapes::parse_command_substitution(value.trim())
            {
                // Fallback for shapes the span-aware lexer-based parser
                // rejects (no `tokens`, or an embedded `;`/newline second
                // command) but the older bracket-counting parser still
                // accepts — whole-statement span, same as before this fix.
                check_invocation(
                    ctx,
                    &InvocationSite {
                        command: &command,
                        lookup_command: &command,
                        args: &args,
                        arg_spans: &[],
                        is_foreach_header: false,
                        fallback_span: stmt.span(),
                    },
                    uses,
                );
            }
        }

        Statement::Incr { name, amount, .. } => {
            // `incr` reads both its target variable and (when present) its
            // increment argument as Int.  The two checks are independent —
            // an Int target with a String `$amount` still shimmers on the
            // amount — so neither must short-circuit the other.
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
    // Skip an array base (FP-SH-13) — see `check_invocation`.
    if ctx.array_syms.contains(&sym) {
        return;
    }
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
            "variable '{var}' has {from} intrep but 'incr' expects int",
            from = type_name(current),
        ),
        related,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// A String variable passed to `incr` triggers S100.
    #[test]
    fn shimmer_detected_for_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
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
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
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
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
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
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex shimmer for Int var, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::Int);
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// The warning's span is tight around the offending `$x` argument, not
    /// the whole `lindex $x 0` invocation — the developer's eye should land
    /// on the variable, not have to scan the whole call.
    #[test]
    fn shimmer_span_is_tight_around_call_argument() {
        let src = "set x 5\nlindex $x 0";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex")
            .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
        let text = &src[w.span.start() as usize..w.span.end() as usize];
        assert_eq!(
            text, "$x",
            "span should cover only the '$x' argument, got {text:?} from {w:?}"
        );
    }

    /// Same tightness property through the `[cmd …]`-in-`AssignValue` path
    /// (`set b [lindex $x 0]`) — the span still lands on `$x`, not the
    /// whole `set b […]` statement.
    #[test]
    fn shimmer_span_is_tight_around_assign_value_substitution_argument() {
        let src = "set x 5\nset b [lindex $x 0]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "lindex")
            .unwrap_or_else(|| panic!("expected lindex shimmer, got: {warnings:?}"));
        let text = &src[w.span.start() as usize..w.span.end() as usize];
        assert_eq!(
            text, "$x",
            "span should cover only the '$x' argument, got {text:?} from {w:?}"
        );
    }

    /// Two shimmering arguments of the *same* call each get their *own*
    /// tight span — proves the per-index span lookup, not just "arg 0's
    /// span reused everywhere". `linsert list index element`: `list_var`
    /// (String, arg 0) expects List; `index_var` (String, arg 1) expects
    /// Int.
    #[test]
    fn shimmer_span_is_tight_per_argument_not_reused_across_args() {
        let src = "set list_var hello\n\
                    set index_var world\n\
                    linsert $list_var $index_var x";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let list_w = warnings
            .iter()
            .find(|w| w.command == "linsert" && w.variable == "list_var")
            .unwrap_or_else(|| panic!("expected linsert shimmer for list_var, got: {warnings:?}"));
        let list_text = &src[list_w.span.start() as usize..list_w.span.end() as usize];
        assert_eq!(list_text, "$list_var", "got {list_w:?}");

        let index_w = warnings
            .iter()
            .find(|w| w.command == "linsert" && w.variable == "index_var")
            .unwrap_or_else(|| panic!("expected linsert shimmer for index_var, got: {warnings:?}"));
        let index_text = &src[index_w.span.start() as usize..index_w.span.end() as usize];
        assert_eq!(index_text, "$index_var", "got {index_w:?}");
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
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex S101 for foreach var, got: {warnings:?}"
        );
        let w = w.unwrap();
        assert_eq!(w.code, DiagCode::S101);
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TP: `foreach`'s own list argument shimmers when the variable holds a
    /// non-list intrep — the CFG builder lowers the header to a synthetic
    /// `Statement::Call` (`command="foreach", args=[list_arg]`) that never
    /// reaches the per-index `arg_types` path, so this is the
    /// `foreach_header_expected_type` path specifically.
    #[test]
    fn shimmer_detected_for_foreach_header_list_argument() {
        let src = "proc f {} {\n    set l hello\n    foreach x $l { puts $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "foreach" && w.variable == "l")
            .unwrap_or_else(|| panic!("expected foreach header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TN control: a genuine list variable in `foreach` must not shimmer.
    #[test]
    fn no_shimmer_for_foreach_header_with_real_list() {
        let src = "proc f {} {\n    set l [list 1 2 3]\n    foreach x $l { puts $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        assert!(
            warnings.iter().all(|w| w.command != "foreach"),
            "unexpected foreach header shimmer for a real list: {warnings:?}"
        );
    }

    /// TP: `lmap`'s list argument shares the same synthetic header shape.
    #[test]
    fn shimmer_detected_for_lmap_header_list_argument() {
        let src = "proc f {} {\n    set l hello\n    lmap x $l { set x $x }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "lmap" && w.variable == "l")
            .unwrap_or_else(|| panic!("expected lmap header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
    }

    /// TP: `dict for`'s dict argument shimmers to Dict — the compound
    /// two-word command name (`"dict for"`) routes through the `dict`
    /// subcommand table via `foreach_header_expected_type`'s split.
    #[test]
    fn shimmer_detected_for_dict_for_header_argument() {
        let src = "proc f {} {\n    set d hello\n    dict for {k v} $d { puts $k }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "dict for" && w.variable == "d")
            .unwrap_or_else(|| panic!("expected dict-for header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::Dict);
    }

    /// TP: `dict map` shares `dict for`'s compound-name dispatch.
    #[test]
    fn shimmer_detected_for_dict_map_header_argument() {
        let src = "proc f {} {\n    set d hello\n    dict map {k v} $d { set v $v }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "dict map" && w.variable == "d")
            .unwrap_or_else(|| panic!("expected dict-map header shimmer, got: {warnings:?}"));
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::Dict);
    }

    /// A loop-invariant variable (defined outside the loop) coerced to a
    /// single intrep inside the loop converts once and is cached — that is a
    /// one-time S100, not the per-iteration S101.
    #[test]
    fn loop_invariant_single_target_downgrades_to_s100() {
        let src = "proc f {} {\n  set data {1 2 3}\n  for {set i 0} {$i < 3} {incr i} {\n    set x [lindex $data $i]\n  }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
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
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
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

    /// Array elements collapse onto one SSA symbol (the `(key)` suffix is
    /// stripped before interning), so two individually-stable but different
    /// elements must not use-site shimmer against each other (FP-SH-13):
    /// `set arr(n) 5; set arr(label) "text"; incr arr(n)` must not report
    /// `arr` as "string used with incr" — `arr(n)` is always int.
    #[test]
    fn no_use_site_shimmer_for_array_element_conflation() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set arr(n) 5\n set arr(label) \"text\"\n incr arr(n) }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        assert!(
            warnings.is_empty(),
            "array-element conflation must not use-site shimmer: {warnings:?}"
        );
    }

    /// TP control: the identical shape on a plain scalar still fires — the
    /// array guard must not blanket-silence use-site shimmer.
    /// `string length`/`index`/`range` install the string intrep on their
    /// subject (tclsh-verified: a list becomes `string`) — genuine shimmer —
    /// but a pure byte array short-circuits and keeps its rep
    /// (`transparent_from`), so it must stay silent.
    #[test]
    fn string_subject_shimmers_list_but_not_bytearray() {
        // TP: a List-typed subject is coerced to the string intrep.
        let cu =
            CompilationUnit::build_for("set l [list 1 2 3]\nstring length $l", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "l" && w.to_type == TclType::String),
            "list subject of `string length` must shimmer: {warnings:?}"
        );

        // FP guard: a ByteArray-typed subject passes through untouched.
        let cu2 = CompilationUnit::build_for(
            "set ba [binary format c* {200 201}]\nstring length $ba\nstring range $ba 0 1",
            &registry(),
            false,
        );
        let fu2 = cu2.function("::top").unwrap();
        let warnings2 = find_use_site_shimmers(
            &fu2.cfg,
            &fu2.ssa,
            &fu2.types,
            &fu2.sccp.executable_blocks,
            &registry(),
            &fu2.sccp.values,
        );
        assert!(
            !warnings2.iter().any(|w| w.variable == "ba"),
            "byte-array subject is transparent — no shimmer: {warnings2:?}"
        );
    }

    #[test]
    fn use_site_shimmer_still_fires_for_scalar_control() {
        let cu =
            CompilationUnit::build_for("proc f {} { set n hello\n incr n }", &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.command == "incr" && w.variable == "n"),
            "plain scalar must still use-site shimmer: {warnings:?}"
        );
    }

    /// Variables with Unknown type do not produce false-positive shimmers.
    #[test]
    fn no_shimmer_for_unknown_type() {
        // `set x $other` — type of x is Unknown (other has no known type).
        let cu = CompilationUnit::build_for("set x $other\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        // x has Unknown type; should not produce a shimmer.
        let lindex_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "lindex").collect();
        assert!(
            lindex_shimmers.is_empty(),
            "unexpected shimmer for Unknown type: {lindex_shimmers:?}"
        );
    }

    /// TP: a call through an `interp alias` to a shimmering builtin is still
    /// detected — the registry lookup keys off the resolved
    /// `canonical_command`, not the alias's source spelling.
    #[test]
    fn shimmer_detected_through_interp_alias_to_lindex() {
        let cu = CompilationUnit::build_for(
            "interp alias {} myindex {} ::lindex\nset x hello\nmyindex $x 0",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings.iter().find(|w| w.variable == "x");
        assert!(
            w.is_some(),
            "expected shimmer through interp alias, got: {warnings:?}"
        );
        let w = w.unwrap();
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
        // The message keeps the alias spelling the user wrote, not the
        // resolved canonical target.
        assert_eq!(w.command, "myindex");
    }

    /// TN control: an alias to a *non*-shimmering command (`puts`) must not
    /// spuriously fire just because the alias itself resolved.
    #[test]
    fn no_shimmer_through_interp_alias_to_non_shimmering_command() {
        let cu = CompilationUnit::build_for(
            "interp alias {} say {} ::puts\nset x hello\nsay $x",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        assert!(
            warnings.iter().all(|w| w.variable != "x"),
            "unexpected shimmer through non-shimmering alias: {warnings:?}"
        );
    }
}
