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
use super::hints::{arg_shimmer_type, is_numeric_compatible};
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

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`).
fn check_invocation(
    ctx: &mut UseSiteCtx<'_>,
    command: &str,
    args: &[String],
    span: Span,
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
        ctx.out.push(ShimmerWarning {
            span,
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
        });
    }
}

fn check_statement(ctx: &mut UseSiteCtx<'_>, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
    match stmt {
        Statement::Call { command, args, .. } => {
            check_invocation(ctx, command, args, stmt.span(), uses);
        }

        // A command substitution lifted into an assignment value
        // (`set b [lindex $x 0]`) reads its arguments just like a direct
        // call.
        Statement::AssignValue { value, .. } => {
            if let Some((command, args)) =
                crate::value_shapes::parse_command_substitution(value.trim())
            {
                check_invocation(ctx, &command, &args, stmt.span(), uses);
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
}
