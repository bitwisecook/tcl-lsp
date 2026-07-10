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

//! Thunking (loop-oscillation) detection (S102).
//!
//! A "thunk" occurs when a variable inside a loop alternates between two
//! intrep types on successive iterations, forcing a type conversion on
//! every pass through the loop.
//!
//! Detection strategy:
//!
//! 1. Find **loop headers** — blocks that are:
//!    - inside a cycle (`loop_body_blocks`), **and**
//!    - have at least one predecessor from *outside* the loop (entry edge), **and**
//!    - have at least one predecessor from *inside* the loop (back edge).
//!
//! 2. For each phi node at a loop header whose `TypeLattice` is
//!    `Shimmered`, the variable merges two different types at every
//!    iteration.  That is the thunking pattern.
//!
//! Diagnostic code:
//! - **S102**: variable oscillates between two intrep types across loop
//!   iterations.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::TclType;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::ssa::{Phi, SsaFunction, Symbol, ValueKey, Version};
use crate::types::{TypeKind, TypeLattice};

use crate::codegen::values::split_array_ref;

use super::graph::{build_successors, loop_body_blocks};
use super::span::{def_range_map, phi_span};
use super::{ThunkingWarning, type_name};

/// Invert a successor map into a predecessor map.
pub(super) fn build_predecessors(
    succs: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut preds: HashMap<String, Vec<String>> = HashMap::new();
    for (p, targets) in succs {
        for t in targets {
            preds.entry(t.clone()).or_default().push(p.clone());
        }
    }
    preds
}

/// Blocks of the natural loop with `header`: `{header}` plus every block that
/// reaches a back-edge source without passing through the header.  This is the
/// per-loop block set `build_loop_forest` provides for the S102 pass, so a
/// sibling loop's type effect on the same name never pollutes this loop's
/// oscillation check.
pub(super) fn natural_loop_blocks(
    header: &str,
    preds: &HashMap<String, Vec<String>>,
    loop_blocks: &HashSet<String>,
) -> HashSet<String> {
    let mut body: HashSet<String> = HashSet::new();
    body.insert(header.to_owned());
    let mut stack: Vec<String> = preds
        .get(header)
        .map(|ps| {
            ps.iter()
                .filter(|p| loop_blocks.contains(*p))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    for s in &stack {
        body.insert(s.clone());
    }
    while let Some(n) = stack.pop() {
        if let Some(ps) = preds.get(&n) {
            for p in ps {
                if body.insert(p.clone()) {
                    stack.push(p.clone());
                }
            }
        }
    }
    body
}

/// SSA versions of each name defined by the empty literal (`set x {}` / `""`)
/// over the whole function — the typeless-empty reset, excluded from the
/// oscillation type sets.
pub(super) fn empty_value_versions(ssa: &SsaFunction) -> HashMap<Symbol, HashSet<Version>> {
    let mut out: HashMap<Symbol, HashSet<Version>> = HashMap::new();
    for sb in ssa.blocks.values() {
        for s in &sb.statements {
            let is_empty = matches!(
                &s.statement,
                Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. }
                    if value.is_empty()
            );
            if is_empty {
                for (&sym, &ver) in &s.defs {
                    out.entry(sym).or_default().insert(ver);
                }
            }
        }
    }
    out
}

/// Foreach-header blocks whose body is a single `break` — the destructure
/// idiom (`foreach {a b c} $l break`), a one-time multi-assign whose bindings
/// must not count as per-iteration loop-body types.  Detected by a
/// block-shape heuristic.
pub(super) fn destructure_foreach_blocks(cfg: &CfgFunction) -> HashSet<String> {
    let mut out = HashSet::new();
    for block in cfg.blocks.values() {
        if block.statements.len() == 1
            && let Statement::Call { command, .. } = &block.statements[0]
            && command == "break"
        {
            out.insert(block.name.clone());
        }
    }
    out
}

/// Reverse name → [`BlockId`] lookup for an [`SsaFunction`], built from its
/// own interned name table. Lets a name-keyed loop-block set index the
/// `BlockId`-keyed `ssa.blocks` map without a `&cfg::Function` on hand.
fn ssa_name_to_id(ssa: &SsaFunction) -> HashMap<&str, BlockId> {
    ssa.block_names()
        .iter()
        .enumerate()
        .map(|(i, name)| {
            (
                name.as_str(),
                BlockId(u32::try_from(i).expect("SSA block count fits in u32")),
            )
        })
        .collect()
}

/// KNOWN, non-empty intreps each name is *defined as* inside the per-loop
/// blocks of `header` (destructure foreach blocks excluded).
pub(super) fn per_loop_body_types(
    header: &str,
    loop_block_set: &HashSet<String>,
    destructure: &HashSet<String>,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    empty_by_name: &HashMap<Symbol, HashSet<Version>>,
) -> HashMap<Symbol, HashSet<TclType>> {
    let _ = header;
    let name_to_id = ssa_name_to_id(ssa);
    let mut out: HashMap<Symbol, HashSet<TclType>> = HashMap::new();
    for lbn in loop_block_set {
        if destructure.contains(lbn) {
            continue;
        }
        let Some(lssa) = name_to_id
            .get(lbn.as_str())
            .and_then(|id| ssa.blocks.get(id))
        else {
            continue;
        };
        for s in &lssa.statements {
            for (&sym, &ver) in &s.defs {
                if empty_by_name
                    .get(&sym)
                    .is_some_and(|set| set.contains(&ver))
                {
                    continue;
                }
                if let Some(t) = types.get(&(sym, ver))
                    && t.kind == TypeKind::Known
                    && let Some(tt) = t.tcl_type
                {
                    out.entry(sym).or_default().insert(tt);
                }
            }
        }
    }
    out
}

/// Symbols ever written through array-element syntax (`arr(key)`) anywhere
/// in the function.
///
/// [`crate::naming::normalise_var_name`] strips the `(key)` suffix before
/// SSA interning, so every element of an array collapses onto ONE `Symbol`
/// / version chain — `arr(a)` and `arr(b)` are independent runtime slots
/// that never actually shimmer against each other, but once conflated onto
/// one symbol, two elements individually holding stable-but-different
/// types look exactly like a genuine same-slot oscillation. Excluding any
/// symbol proven to be an array base avoids reporting a thunk across
/// unrelated elements; reuses the same array-reference recognition
/// [`crate::codegen::values::split_array_ref`] already provides for
/// codegen, rather than re-deriving the `(...)` pattern here.
///
/// `pub(super)` so the sibling use-site (S100/S101) and phi-merge (S101) passes
/// share the *same* exclusion: every element of an array collapses onto one
/// symbol for all three passes, so any one of them can conflate two
/// stable-but-different elements into a spurious shimmer (FP-SH-13).
pub(super) fn array_element_symbols(cfg: &CfgFunction, ssa: &SsaFunction) -> HashSet<Symbol> {
    let mut out = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            let raw = match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => name.as_str(),
                _ => continue,
            };
            let Some((base, _key)) = split_array_ref(raw) else {
                continue;
            };
            if let Some(sym) = ssa.var_symbol(base) {
                out.insert(sym);
            }
        }
    }
    out
}

/// True when some statement inside `loop_block_set` both reads and writes a
/// version of `sym` — a loop-carried self-reference (`set x [expr {$x+1}]`),
/// as opposed to a variable that is simply reset from independent literals
/// each pass (`set x "value"` / `set x [list 1 2]`, neither of which reads
/// `$x`). Only a self-referential variable can genuinely pay a per-iteration
/// re-conversion cost from a prior pass's value; a reset-only variable's
/// differing per-branch types are fresh objects with no shared state to
/// reinterpret, so they must not count as evidence of thunking.
fn loop_self_referential(ssa: &SsaFunction, loop_block_set: &HashSet<String>, sym: Symbol) -> bool {
    let name_to_id = ssa_name_to_id(ssa);
    loop_block_set.iter().any(|lbn| {
        name_to_id
            .get(lbn.as_str())
            .and_then(|id| ssa.blocks.get(id))
            .is_some_and(|block| {
                block
                    .statements
                    .iter()
                    .any(|s| s.defs.contains_key(&sym) && s.uses.contains_key(&sym))
            })
    })
}

/// Best-effort in-loop span for a `(Symbol, TclType)` pair: the earliest def
/// of `sym` typed exactly `t` among `loop_block_set`'s own statements
/// (destructure-foreach blocks excluded, matching [`per_loop_body_types`]'s
/// exclusion). Lets the S102 warning anchor on the actual loop-body
/// statement that produced the surprising type, rather than on whichever
/// incoming def happens to sort earliest in the source — which for an
/// ordinary `while`/`for`/`foreach` is always the pre-loop initialiser, not
/// the oscillating code the developer needs to look at. `None` when no
/// in-loop def produced that type (the oscillation was detected purely from
/// the header phi's own direct incoming edges).
fn per_loop_type_span(
    ctx: &ThunkCtx<'_>,
    loop_block_set: &HashSet<String>,
    sym: Symbol,
    target: TclType,
) -> Option<Span> {
    let name_to_id = ssa_name_to_id(ctx.ssa);
    let mut best: Option<Span> = None;
    for lbn in loop_block_set {
        if ctx.destructure.contains(lbn) {
            continue;
        }
        let Some(lssa) = name_to_id
            .get(lbn.as_str())
            .and_then(|id| ctx.ssa.blocks.get(id))
        else {
            continue;
        };
        for s in &lssa.statements {
            let Some(&ver) = s.defs.get(&sym) else {
                continue;
            };
            if ctx
                .empty_by_name
                .get(&sym)
                .is_some_and(|set| set.contains(&ver))
            {
                continue;
            }
            let matches_target = ctx
                .types
                .get(&(sym, ver))
                .is_some_and(|t| t.kind == TypeKind::Known && t.tcl_type == Some(target));
            if !matches_target {
                continue;
            }
            let Some(&sp) = ctx.def_map.get(&(sym, ver)) else {
                continue;
            };
            best = Some(match best {
                Some(b) if (b.start(), b.end()) <= (sp.start(), sp.end()) => b,
                _ => sp,
            });
        }
    }
    best
}

/// Identify the loop-header blocks of `cfg` by name.
///
/// A loop header is on a cycle and has both an entry edge (predecessor
/// outside the loop) and a back edge (predecessor inside the loop, other
/// than self-loops).  The successor graph is name-keyed, so headers are
/// tracked by name too.
fn loop_header_names(
    cfg: &CfgFunction,
    loop_blocks: &HashSet<String>,
    succs: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    cfg.block_names()
        .iter()
        .filter(|bn| {
            if !loop_blocks.contains(bn.as_str()) {
                return false;
            }
            let has_entry = succs.iter().any(|(pred, targets)| {
                targets.iter().any(|t| t == *bn) && !loop_blocks.contains(pred)
            });
            let has_back = succs.iter().any(|(pred, targets)| {
                targets.iter().any(|t| t == *bn) && loop_blocks.contains(pred) && pred != *bn
            });
            has_entry && has_back
        })
        .cloned()
        .collect()
}

/// Shared, read-only context for [`classify_thunking_phi`].
struct ThunkCtx<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    types: &'a HashMap<ValueKey, TypeLattice>,
    empty_by_name: &'a HashMap<Symbol, HashSet<Version>>,
    loop_blocks: &'a HashSet<String>,
    def_map: &'a HashMap<ValueKey, Span>,
    destructure: &'a HashSet<String>,
    /// Symbols proven to be an array base ([`array_element_symbols`]) —
    /// excluded from S102 entirely, since their conflated version chain
    /// mixes independent elements under one phi.
    array_syms: &'a HashSet<Symbol>,
}

/// Decide whether a loop-header phi genuinely thunks (oscillates between
/// two intreps each iteration), returning the warning when it does.
///
/// `per_loop` is the body-type map for *this* loop only, so sibling loops
/// do not pollute the oscillation check. `this_loop` is the same loop's raw
/// block-name set, needed (alongside `per_loop`) to anchor the warning's
/// span on an in-loop statement.
fn classify_thunking_phi(
    ctx: &ThunkCtx<'_>,
    phi: &Phi,
    this_loop: &HashSet<String>,
    per_loop: &HashMap<Symbol, HashSet<TclType>>,
) -> Option<ThunkingWarning> {
    if ctx.array_syms.contains(&phi.name) {
        return None;
    }
    let lattice = ctx.types.get(&(phi.name, phi.version))?;
    if lattice.kind != TypeKind::Shimmered {
        return None;
    }
    let type_a = lattice.from_type?;
    let type_b = lattice.tcl_type?;

    // A loop-header phi is SHIMMERED whenever the entry type differs
    // from the body-exit type — but that includes the *one-time*
    // promotion of an empty accumulator (`set r {}; foreach …
    // {lappend r …}`): STRING once, then LIST forever.  Genuine
    // thunking requires the body to re-introduce the entry type (so
    // the phi re-shimmers each pass) or to produce ≥2 conflicting
    // types itself.  Classify the phi's incomings (entry vs body) and
    // require oscillation.
    //
    // A self-referential variable (some loop statement both reads and
    // writes it — [`loop_self_referential`]) can carry oscillation through
    // an intermediate merge: `if {$n % 2} {set x [expr {$x+1}]} else {set x
    // [string range $x 0 end]}` reaches the header via a bottom-of-loop join
    // that is itself SHIMMERED, not KNOWN. For that case only, unpack the
    // join's own two constituent types as body evidence directly — a
    // non-self-referential variable (`set x "value"` / `set x [list 1 2]`,
    // neither reading `$x`) stays excluded: it creates a fresh, unrelated
    // value each pass, so two different per-branch types cost nothing extra
    // to "oscillate" between.
    let empty_vers = ctx.empty_by_name.get(&phi.name);
    let self_referential = loop_self_referential(ctx.ssa, this_loop, phi.name);
    let mut entry_types: HashSet<TclType> = HashSet::new();
    let mut body_types: HashSet<TclType> = HashSet::new();
    let mut has_body_incoming = false;
    for (pred, &inc_ver) in &phi.incoming {
        if inc_ver == 0 {
            continue;
        }
        let Some(inc_type) = ctx.types.get(&(phi.name, inc_ver)) else {
            continue;
        };
        let is_empty = empty_vers.is_some_and(|s| s.contains(&inc_ver));
        if ctx.loop_blocks.contains(ctx.cfg.block_name(*pred)) {
            if inc_type.kind == TypeKind::Known && !is_empty {
                has_body_incoming = true;
                if let Some(t) = inc_type.tcl_type {
                    body_types.insert(t);
                }
            } else if self_referential && inc_type.kind == TypeKind::Shimmered {
                has_body_incoming = true;
                body_types.extend(inc_type.from_type);
                body_types.extend(inc_type.tcl_type);
            }
        } else if inc_type.kind == TypeKind::Known
            && !is_empty
            && let Some(t) = inc_type.tcl_type
        {
            entry_types.insert(t);
        }
    }
    if !has_body_incoming {
        return None;
    }
    let mut all_body_types = body_types;
    if let Some(pl) = per_loop.get(&phi.name) {
        all_body_types.extend(pl.iter().copied());
    }
    let oscillates =
        entry_types.intersection(&all_body_types).next().is_some() || all_body_types.len() >= 2;
    if !oscillates {
        return None;
    }

    // Anchor on the loop-body statement that actually produced one of the
    // two oscillating types, rather than on whichever incoming def sorts
    // earliest in the source — for an ordinary `while`/`for`/`foreach` that
    // is always the pre-loop initialiser, not the code the developer needs
    // to look at. Falls back to the old whole-phi heuristic only when
    // neither type traces to an in-loop def (e.g. the oscillation came
    // purely from the header's own two direct incoming edges).
    let span = [type_b, type_a]
        .into_iter()
        .find_map(|t| per_loop_type_span(ctx, this_loop, phi.name, t))
        .unwrap_or_else(|| phi_span(phi, ctx.ssa, ctx.def_map));
    let related: Vec<_> = phi
        .incoming
        .iter()
        .filter_map(|(pred_block, &ver)| {
            ctx.def_map.get(&(phi.name, ver)).copied().map(|sp| {
                (
                    sp,
                    format!("version from '{}'", ctx.cfg.block_name(*pred_block)),
                )
            })
        })
        .collect();

    let var = ctx.ssa.var_name(phi.name);
    Some(ThunkingWarning {
        span,
        variable: var.to_owned(),
        type_a,
        type_b,
        code: DiagCode::S102,
        message: format!(
            "S102: '{var}' oscillates between {a} and {b} across \
             loop iterations (thunking)",
            a = type_name(type_a),
            b = type_name(type_b),
        ),
        related,
    })
}

/// Find thunking warnings for a function.
///
/// Returns one [`ThunkingWarning`] per loop-header phi node whose type
/// lattice is `Shimmered` and whose incomings genuinely oscillate.
#[must_use]
pub(crate) fn find_thunking_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
) -> Vec<ThunkingWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let succs = build_successors(cfg);
    let def_map = def_range_map(ssa);
    let mut out = Vec::new();

    let loop_headers = loop_header_names(cfg, &loop_blocks, &succs);
    let preds = build_predecessors(&succs);
    let empty_by_name = empty_value_versions(ssa);
    let destructure = destructure_foreach_blocks(cfg);
    let array_syms = array_element_symbols(cfg, ssa);

    let ctx = ThunkCtx {
        cfg,
        ssa,
        types,
        empty_by_name: &empty_by_name,
        loop_blocks: &loop_blocks,
        def_map: &def_map,
        destructure: &destructure,
        array_syms: &array_syms,
    };

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let block_name = cfg.block_name(block_id);
        if !loop_headers.contains(block_name) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };

        // Per-loop body types for *this* loop only (sibling loops must not
        // pollute the oscillation check).
        let this_loop = natural_loop_blocks(block_name, &preds, &loop_blocks);
        let per_loop = per_loop_body_types(
            block_name,
            &this_loop,
            &destructure,
            ssa,
            types,
            &empty_by_name,
        );

        for phi in &ssa_block.phis {
            if let Some(warning) = classify_thunking_phi(&ctx, phi, &this_loop, &per_loop) {
                out.push(warning);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// A loop variable that is always Int — no thunking.
    #[test]
    fn no_thunking_for_uniform_int_loop_variable() {
        let cu = CompilationUnit::build_for(
            "for {set i 0} {$i < 10} {incr i} { set x 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "unexpected thunking warnings: {w:?}");
    }

    /// Sibling (non-nested) loops that each set the same var to a different
    /// but per-loop-monomorphic type must not be reported as oscillating.
    #[test]
    fn no_s102_for_sibling_loops_with_different_types() {
        let cu = CompilationUnit::build_for(
            "proc f {items} { foreach a $items { set x \"value\" }\n \
             foreach b $items { set x [list 1 2] } }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "sibling loops must not thunk: {w:?}");
    }

    /// The empty-accumulator promotion (`set r {}; foreach … {lappend r …}`)
    /// is a one-time STRING→LIST stabilisation, not per-iteration oscillation.
    #[test]
    fn no_s102_for_empty_accumulator_promotion() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set r {}\n foreach x {1 2 3} { lappend r $x }\n return [llength $r] }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "accumulator promotion must not thunk: {w:?}");
    }

    /// Empty source: no thunking warnings and no panic.
    #[test]
    fn thunking_empty_source() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty());
    }

    /// A while loop with no variables — no thunking.
    #[test]
    fn no_thunking_for_no_loop_variables() {
        let cu = CompilationUnit::build_for("while {1} { puts \"hello\" }", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "unexpected thunking: {w:?}");
    }

    /// A loop body that produces ≥2 distinct intrep types in one iteration
    /// (`set x "s"; set x [list 1]` — STRING then LIST) genuinely re-thunks
    /// every pass and fires S102.  (A *one-time* promotion — `set x 0; while …
    /// { set x "hello" }`, where x stabilises at STRING after the first pass —
    /// is suppressed; see the sibling/accumulator tests.)
    #[test]
    fn thunking_detected_for_two_type_loop_body() {
        // Non-constant condition so SCCP keeps both edges executable.
        let cu = CompilationUnit::build_for(
            "proc f {n} { set x 0\n while {$n} { set x \"s\"\n set x [list 1] }\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_thunking = w
            .iter()
            .any(|tw| tw.variable == "x" && tw.code == DiagCode::S102);
        assert!(
            has_thunking,
            "expected S102 thunking for 'x' with a two-type loop body, got: {w:?}"
        );
    }

    /// The warning's primary span must anchor inside the loop body (on the
    /// statement that actually produces the surprising type), not on the
    /// pre-loop initialiser — the "earliest incoming def" heuristic always
    /// picked the initialiser (the textually-first def), which is almost
    /// never where the developer needs to look.
    #[test]
    fn span_anchors_inside_loop_not_pre_loop_initialiser() {
        let src = "proc f {} { set x 0\n while {1} { set x [expr {$x + 1}]\n \
                    set x [string range $x 0 end] } }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert_eq!(w.len(), 1, "expected exactly one S102 warning: {w:?}");
        let while_pos = u32::try_from(src.find("while").unwrap()).unwrap();
        assert!(
            w[0].span.start() > while_pos,
            "S102 span should anchor inside the loop body (after offset {while_pos}), got {:?}",
            w[0].span
        );
    }

    /// A variable that reads its own prior value in one loop-body branch and
    /// is reset to an unrelated type in the other (`if {...} {set x [string
    /// range $x 0 end]} else {set x [list 1 2]}`) genuinely oscillates: the
    /// direct loop-internal predecessor into the header phi is itself a
    /// SHIMMERED merge (not a single KNOWN type, since it is the join of the
    /// two branches), which the self-reference check must still recognise as
    /// body evidence.
    #[test]
    fn thunking_detected_for_self_referential_branchy_oscillation() {
        let cu = CompilationUnit::build_for(
            "proc f {n} { set x \"seed\"\n while {$n} { \
             if {$n % 2} { set x [string range $x 0 end] } else { set x [list 1 2] }\n \
             incr n -1 }\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.iter().any(|tw| tw.variable == "x"),
            "expected S102 for the self-referential branchy oscillation, got: {w:?}"
        );
    }

    /// Control for the above: when NEITHER branch reads `$x` (both assign
    /// unrelated fresh literals), the variable is not self-referential — it
    /// creates a new, unrelated value each pass with nothing to reinterpret,
    /// so it must NOT be reported as thunking even though the header phi is
    /// still SHIMMERED and the direct predecessor is still a merge.
    #[test]
    fn no_s102_for_non_self_referential_branchy_reset() {
        let cu = CompilationUnit::build_for(
            "proc f {n} { set x \"seed\"\n while {$n} { \
             if {$n % 2} { set x \"value\" } else { set x [list 1 2] }\n \
             incr n -1 }\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.is_empty(),
            "non-self-referential branchy reset must not thunk: {w:?}"
        );
    }

    /// Array-element writes collapse onto one SSA symbol per array (the
    /// `(key)` suffix is stripped before interning) — two elements that are
    /// each individually type-stable but differ from each other must not be
    /// reported as one variable oscillating: they are independent runtime
    /// slots, not the same value shimmering back and forth.
    #[test]
    fn no_s102_for_array_element_conflation() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set arr(x) 0\n while {1} { \
             set arr(x) [expr {$arr(x) + 1}]\n set arr(x) [string range $arr(x) 0 end] } }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.is_empty(),
            "array-element writes must not be conflated into an S102 report: {w:?}"
        );
    }

    /// A `trace add variable … write …` callback can rewrite a variable's
    /// value (and therefore its intrep) immediately after any assignment —
    /// so a `set`-only view of a traced variable's literal types must not
    /// report S102: [`crate::type_infer::propagate_types`] forces every def
    /// of a traced name to OVERDEFINED via the shared
    /// [`crate::var_observability::ModuleVariableTraces`] fact, the same one
    /// [`crate::sccp::sccp`] already consumes for its own lattice.
    #[test]
    fn no_s102_for_traced_variable() {
        let cu = CompilationUnit::build_for(
            "proc f {} { trace add variable x write {apply {{n1 n2 op} {}}}\n set x 0\n \
             while {1} { set x [expr {$x + 1}]\n set x [string range $x 0 end] } }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "traced variable must not thunk: {w:?}");
    }

    /// A self-referential loop whose body-exit is `SHIMMERED(Numeric, String)`
    /// merging with an `Int` loop entry: pre-fix `type_join` degraded
    /// `Known(Int) ⊔ SHIMMERED(Numeric, String)` to OVERDEFINED (exact-equality
    /// match), silently masking the genuine thunk. The numeric-refinement join
    /// keeps the header phi SHIMMERED, so S102 now fires.
    #[test]
    fn thunking_detected_for_numeric_shimmer_masked_by_int_entry() {
        let cu = CompilationUnit::build_for(
            "proc f {n} { set x 0\n \
             while {$n > 0} { if {$n % 2} { set x [expr {$x + $n}] } else { set x \"s$x\" }\n \
             incr n -1 }\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.iter().any(|tw| tw.variable == "x"),
            "numeric/string oscillation seeded by an Int entry must fire S102: {w:?}"
        );
    }

    /// TP control for the above: the identical loop shape, untraced, must
    /// still fire — proves the trace check isn't blanket-silencing S102.
    #[test]
    fn thunking_still_fires_for_untraced_control() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set x 0\n \
             while {1} { set x [expr {$x + 1}]\n set x [string range $x 0 end] } }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.iter().any(|tw| tw.variable == "x"),
            "untraced control must still fire S102: {w:?}"
        );
    }
}
