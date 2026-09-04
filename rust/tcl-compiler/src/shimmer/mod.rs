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

//! Intrep shimmer analysis — detect places where a variable's
//! Tcl-value intrep (list/dict/int/…) is converted at a use site.
//!
//! Decomposed into
//! independently-testable sub-modules:
//!
//! | Sub-module    | Responsibility                                  |
//! |---------------|-------------------------------------------------|
//! | [`graph`]     | Loop detection, CFG reachability                |
//! | [`hints`]     | Registry arg-type hints, numeric compatibility  |
//! | [`span`]      | SSA definition → source span mapping            |
//! | [`use_site`]  | S100/S101 use-site shimmer detection            |
//! | [`phi`]       | S101 phi-node shimmer detection                 |
//! | [`expr`]      | S100 expression-level shimmer detection         |
//! | [`thunking`]  | S102 loop-oscillation detection                 |
//! | [`sharing`]   | S103 shared-value copy-on-write detection       |
//! | [`byte_array`]| S110 byte-array-corruption detection            |

pub mod byte_array;
pub mod commit;
pub mod expr;
pub mod graph;
pub mod hints;
pub mod phi;
pub mod sharing;
pub mod span;
pub mod thunking;
pub mod use_site;

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::{BytePayloadSpec, CommandRegistry, TclType};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::TypeLattice;

/// A use-site where a variable's intrep is converted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShimmerWarning {
    /// Source span of the use.
    pub span: Span,
    /// Variable name.
    pub variable: String,
    /// Source intrep (the type the variable held).
    pub from_type: TclType,
    /// Target intrep (the type the command expected).
    pub to_type: TclType,
    /// Command that triggered the conversion.
    pub command: String,
    /// Whether the use is inside a loop body.
    pub in_loop: bool,
    /// Diagnostic code (`"S100"` / `"S101"`).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Related spans + labels for diagnostic context.
    pub related: Vec<(Span, String)>,
}

/// A variable that oscillates between two types across loop iterations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThunkingWarning {
    /// Source span.
    pub span: Span,
    /// Variable name.
    pub variable: String,
    /// First observed type.
    pub type_a: TclType,
    /// Second observed type.
    pub type_b: TclType,
    /// Diagnostic code (`"S102"`).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Related spans.
    pub related: Vec<(Span, String)>,
}

/// A mutation of a provably shared value — C Tcl duplicates the whole value
/// before writing (`Tcl_IsShared` → `Tcl_DuplicateObj`), so the mutation
/// pays an O(n) copy. See [`sharing`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SharingWarning {
    /// Source span of the mutating command.
    pub span: Span,
    /// The mutated variable.
    pub variable: String,
    /// The other live holder of the same value.
    pub shared_with: String,
    /// The mutating command (`lappend` / `lset` / `dict`).
    pub command: String,
    /// Diagnostic code (`"S103"`).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Related spans: the copy that created the share, and the partner's
    /// still-live read.
    pub related: Vec<(Span, String)>,
}

/// Human-readable lowercase name for a Tcl intrep type.
///
/// ```
/// use tcl_compiler::shimmer::type_name;
/// use tcl_registry::TclType;
/// assert_eq!(type_name(TclType::Int), "int");
/// ```
#[must_use]
pub fn type_name(t: TclType) -> String {
    format!("{t:?}").to_ascii_lowercase()
}

/// Resolve the `from` side of a warning against the committed-intrep state:
/// the single committed rep when every path agrees, a *path-dependent* label
/// naming the possibilities when different paths committed different reps,
/// the steady-state may-rep of a loop-carried value, or the lattice fallback.
/// Returns the struct-level `from_type` plus the message label (they differ
/// only in the path-dependent case, where no single `TclType` is truthful).
///
/// A candidate equal to `expected` never names the from side — a loop header
/// re-joins its own conversion via the back edge (`dict for`'s list argument
/// is already a dict on iterations 2+), and that same-type path is exactly
/// the one *not* paying; the reported conversion is from the other state.
#[must_use]
pub(crate) fn committed_from_label(
    state: &commit::CommitState,
    fallback: TclType,
    expected: TclType,
) -> (TclType, String) {
    if let Some(t) = state.single_committed()
        && t != expected
    {
        return (t, type_name(t));
    }
    let may: Vec<TclType> = state
        .may_types()
        .iter()
        .copied()
        .filter(|&t| t != expected)
        .collect();
    if may.len() >= 2 {
        let names: Vec<String> = may.iter().map(|&t| type_name(t)).collect();
        return (fallback, format!("path-dependent ({})", names.join(" or ")));
    }
    if let [t] = may[..] {
        return (t, type_name(t));
    }
    (fallback, type_name(fallback))
}

/// Find intrep-shimmer warnings for a single function.
///
/// Runs three sub-passes in order:
/// 1. **Use-site** ([`use_site`]): a command argument expects a different
///    type than the variable currently holds (S100 outside loops, S101
///    inside loops).
/// 2. **Phi-node** ([`phi`]): control-flow merges two differently-typed
///    versions of a variable (S101).
/// 3. **Expression** ([`expr`]): arithmetic/comparison operators used with
///    the wrong operand type (S100).
#[must_use]
pub(crate) fn find_shimmer_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, crate::analyses::LatticeValue>,
    executable_edges: &HashSet<(BlockId, BlockId)>,
) -> Vec<ShimmerWarning> {
    // The committed-intrep dataflow (first-use commit) is shared by the
    // use-site and expr detectors: it tells each use whether the value has
    // already committed a different intrep on every path (a genuine second
    // conversion) or is still pure (a free first conversion).
    let commit_ctx = commit::CommitCtx {
        registry,
        ssa,
        types,
        values,
    };
    let facts = ShimmerFacts {
        commit: commit::compute_commit_facts(cfg, &commit_ctx, executable_blocks, executable_edges),
        loop_blocks: graph::loop_body_blocks(cfg),
    };
    let mut out = Vec::new();
    out.extend(use_site::find_use_site_shimmers(
        cfg,
        ssa,
        types,
        executable_blocks,
        registry,
        values,
        &facts,
    ));
    out.extend(phi::find_phi_shimmers(
        cfg,
        ssa,
        types,
        executable_blocks,
        &facts.loop_blocks,
    ));
    out.extend(expr::find_expr_shimmers(
        cfg,
        ssa,
        types,
        executable_blocks,
        values,
        registry,
        &facts,
    ));
    out
}

/// Per-function facts the three shimmer sub-passes share, each computed once
/// per CFG rather than once per pass.
///
/// The committed-intrep dataflow is consumed by the use-site and expr
/// detectors; the cycle-block set by all three (it decides the S100 / S101
/// in-loop split).  Recomputing the latter per pass is what made a
/// 2000-branch flat proc take ~69 s before the graph walk became a single
/// Tarjan pass (issue #1240).
pub(crate) struct ShimmerFacts {
    /// Committed-intrep facts — see [`commit::compute_commit_facts`].
    pub commit: commit::CommitFacts,
    /// Every block that lies on a cycle — see [`graph::loop_body_blocks`].
    pub loop_blocks: HashSet<String>,
}

/// Find every shimmer warning across a whole compilation unit.
///
/// Public `*_for_cu` entry point (mirroring
/// [`crate::gvn::find_redundancies_for_cu`]) so downstream tooling — the
/// compiler explorer, the MCP server — can run the analysis without
/// re-deriving per-function inputs. Walks each function's SSA / type /
/// SCCP results in `cu.functions()` order.
#[must_use]
pub fn find_shimmer_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_shimmer_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            registry,
            &fu.sccp.values,
            &fu.sccp.executable_edges,
        ));
    }
    out
}

/// The "first used as" intrep pushback for every function unit: variables
/// whose def is **pure** (a literal / interpolation / string-command result)
/// and whose every executable typed read committed the **same** intrep map to
/// that intrep, keyed by function name then variable name (all versions must
/// agree).  Consumed by hover to surface e.g. "pure string — first used as:
/// list" at the creation site.
#[must_use]
pub fn first_use_commitments_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<String, HashMap<String, TclType>> {
    let mut out: HashMap<String, HashMap<String, TclType>> = HashMap::new();
    for fu in cu.analysable_functions() {
        let ctx = commit::CommitCtx {
            registry,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let facts = commit::compute_commit_facts(
            &fu.cfg,
            &ctx,
            &fu.sccp.executable_blocks,
            &fu.sccp.executable_edges,
        );
        // Collapse (sym, ver) → name: every version with a pushback must agree
        // on the same intrep, else the name is dropped (conflicting uses).
        let mut per_name: HashMap<String, Option<TclType>> = HashMap::new();
        for ((sym, _ver), t) in facts.single_commitments(&ctx) {
            per_name
                .entry(fu.ssa.var_name(sym).to_owned())
                .and_modify(|slot| {
                    if *slot != Some(t) {
                        *slot = None;
                    }
                })
                .or_insert(Some(t));
        }
        let agreed: HashMap<String, TclType> = per_name
            .into_iter()
            .filter_map(|(name, t)| t.map(|t| (name, t)))
            .collect();
        if !agreed.is_empty() {
            out.insert(fu.name.clone(), agreed);
        }
    }
    out
}

/// Find every thunking warning across a whole compilation unit. See
/// [`find_shimmer_warnings_for_cu`].
#[must_use]
pub fn find_thunking_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Vec<ThunkingWarning> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_thunking_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            registry,
            cu.method_instance_vars(&fu.name),
        ));
    }
    out
}

/// Find thunking warnings for a single function.
///
/// Identifies variables that oscillate between two intrep types across
/// loop iterations, causing a type conversion on every pass (S102).
#[must_use]
pub(crate) fn find_thunking_warnings<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    extra_scope_aliases: Option<&HashSet<String, S>>,
) -> Vec<ThunkingWarning> {
    thunking::find_thunking_warnings(
        cfg,
        ssa,
        types,
        executable_blocks,
        registry,
        extra_scope_aliases,
    )
}

/// Find shared-value copy-on-write warnings (S103) for a single function.
///
/// Fires on the tight `set b $a` … `lappend/lset/dict-mutator` pattern
/// where both holders are provably alive — C Tcl duplicates the shared
/// value before every such mutation. See [`sharing`] for the pattern and
/// its deliberate exclusions.
#[must_use]
pub(crate) fn find_sharing_warnings<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    def_use: &crate::def_use::DefUseResult,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    extra_scope_aliases: Option<&HashSet<String, S>>,
) -> Vec<SharingWarning> {
    sharing::find_sharing_warnings(
        cfg,
        ssa,
        def_use,
        executable_blocks,
        registry,
        extra_scope_aliases,
    )
}

/// Find every shared-value copy-on-write warning (S103) across a whole
/// compilation unit. See [`find_shimmer_warnings_for_cu`].
#[must_use]
pub fn find_sharing_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Vec<SharingWarning> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_sharing_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.def_use,
            &fu.sccp.executable_blocks,
            registry,
            cu.method_instance_vars(&fu.name),
        ));
    }
    out
}

/// Find byte-array-corruption warnings (S110) for a single function.
///
/// A forward byte-provenance dataflow flags binary data (a `*::payload`
/// getter, `binary format` / `binary decode` / `encoding convertto`) that is
/// coerced to a character string and then written back through a byte sink
/// (`*::payload replace`), or case-folded / re-encoded directly. See
/// [`byte_array`]. `payload_layouts` is the dialect-gated `*::payload` byte
/// command set (empty under non-iRules dialects).
#[must_use]
pub(crate) fn find_byte_array_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    payload_layouts: &HashMap<&'static str, BytePayloadSpec>,
) -> Vec<ShimmerWarning> {
    byte_array::find_byte_array_warnings(cfg, ssa, executable_blocks, registry, payload_layouts)
}

/// Find every byte-array-corruption warning (S110) across a whole compilation
/// unit. The `*::payload` byte-command set is taken from the registry (already
/// scoped to the loaded dialect). See [`find_shimmer_warnings_for_cu`].
#[must_use]
pub fn find_byte_array_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Vec<ShimmerWarning> {
    let payload_layouts = registry.byte_array_payload_layouts();
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_byte_array_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.executable_blocks,
            registry,
            &payload_layouts,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::Function;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn type_name_is_lowercase() {
        assert_eq!(type_name(TclType::Int), "int");
        assert_eq!(type_name(TclType::String), "string");
        assert_eq!(type_name(TclType::List), "list");
    }

    /// API smoke-test: both entry points accept an empty function.
    #[test]
    fn find_shimmer_warnings_empty_function() {
        let f = Function::new("::top", "entry");
        let ssa = crate::ssa::build_ssa(&f, &registry());
        let sccp = crate::sccp::sccp(
            &f,
            &ssa,
            None,
            crate::tcl_expr_eval::FoldPolicy::default(),
            crate::sccp::TraceInputs {
                registry: &registry(),
                traced_variables: &std::collections::BTreeSet::new(),
                has_dynamic_variable_trace: false,
            },
        );
        let types: HashMap<ValueKey, TypeLattice> = HashMap::new();
        assert!(
            find_shimmer_warnings(
                &f,
                &ssa,
                &types,
                &sccp.executable_blocks,
                &registry(),
                &sccp.values,
                &sccp.executable_edges,
            )
            .is_empty()
        );
        assert!(
            find_thunking_warnings(
                &f,
                &ssa,
                &types,
                &sccp.executable_blocks,
                &registry(),
                None::<&HashSet<String>>
            )
            .is_empty()
        );
    }
}
