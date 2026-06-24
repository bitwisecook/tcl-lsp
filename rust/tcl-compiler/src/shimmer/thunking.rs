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

use tcl_registry::TclType;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey, Version};
use crate::types::{TypeKind, TypeLattice};

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

/// Find thunking warnings for a function.
///
/// Returns one [`ThunkingWarning`] per phi node at a loop header whose
/// type lattice is `Shimmered`.
#[must_use]
#[allow(clippy::too_many_lines)]
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

    // A loop header is a block that is on a cycle and has both an
    // entry edge (predecessor outside the loop) and a back edge
    // (predecessor inside the loop, other than self-loops).  The
    // successor graph is name-keyed, so headers are tracked by name too.
    let loop_headers: HashSet<String> = cfg
        .block_names()
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
        .collect();

    let preds = build_predecessors(&succs);
    let empty_by_name = empty_value_versions(ssa);
    let destructure = destructure_foreach_blocks(cfg);

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
            let key = (phi.name, phi.version);
            let Some(lattice) = types.get(&key) else {
                continue;
            };
            if lattice.kind != TypeKind::Shimmered {
                continue;
            }
            let Some(type_a) = lattice.from_type else {
                continue;
            };
            let Some(type_b) = lattice.tcl_type else {
                continue;
            };

            // A loop-header phi is SHIMMERED whenever the entry type differs
            // from the body-exit type — but that includes the *one-time*
            // promotion of an empty accumulator (`set r {}; foreach …
            // {lappend r …}`): STRING once, then LIST forever.  Genuine
            // thunking requires the body to re-introduce the entry type (so
            // the phi re-shimmers each pass) or to produce ≥2 conflicting
            // types itself.  Classify the phi's incomings (entry vs body) and
            // require oscillation.
            let empty_vers = empty_by_name.get(&phi.name);
            let mut entry_types: HashSet<TclType> = HashSet::new();
            let mut body_types: HashSet<TclType> = HashSet::new();
            let mut has_body_incoming = false;
            for (pred, &inc_ver) in &phi.incoming {
                if inc_ver == 0 {
                    continue;
                }
                let Some(inc_type) = types.get(&(phi.name, inc_ver)) else {
                    continue;
                };
                let is_empty = empty_vers.is_some_and(|s| s.contains(&inc_ver));
                if loop_blocks.contains(cfg.block_name(*pred)) {
                    if inc_type.kind == TypeKind::Known && !is_empty {
                        has_body_incoming = true;
                        if let Some(t) = inc_type.tcl_type {
                            body_types.insert(t);
                        }
                    }
                } else if inc_type.kind == TypeKind::Known
                    && !is_empty
                    && let Some(t) = inc_type.tcl_type
                {
                    entry_types.insert(t);
                }
            }
            if !has_body_incoming {
                continue;
            }
            let mut all_body_types = body_types;
            if let Some(pl) = per_loop.get(&phi.name) {
                all_body_types.extend(pl.iter().copied());
            }
            let oscillates = entry_types.intersection(&all_body_types).next().is_some()
                || all_body_types.len() >= 2;
            if !oscillates {
                continue;
            }

            let span = phi_span(phi, ssa, &def_map);
            let related: Vec<_> = phi
                .incoming
                .iter()
                .filter_map(|(pred_block, &ver)| {
                    def_map.get(&(phi.name, ver)).copied().map(|sp| {
                        (
                            sp,
                            format!("version from '{}'", cfg.block_name(*pred_block)),
                        )
                    })
                })
                .collect();

            let var = ssa.var_name(phi.name);
            out.push(ThunkingWarning {
                span,
                variable: var.to_owned(),
                type_a,
                type_b,
                code: "S102".to_owned(),
                message: format!(
                    "S102: '{var}' oscillates between {a} and {b} across \
                     loop iterations (thunking)",
                    a = type_name(type_a),
                    b = type_name(type_b),
                ),
                related,
            });
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
        let has_thunking = w.iter().any(|tw| tw.variable == "x" && tw.code == "S102");
        assert!(
            has_thunking,
            "expected S102 thunking for 'x' with a two-type loop body, got: {w:?}"
        );
    }
}
