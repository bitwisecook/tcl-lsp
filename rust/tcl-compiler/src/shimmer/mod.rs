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
//! | [`byte_array`]| S110 byte-array-corruption detection            |

pub mod byte_array;
pub mod expr;
pub mod graph;
pub mod hints;
pub mod phi;
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
    /// Suggested fixes, when a mechanical, semantics-preserving rewrite is
    /// available (e.g. `expr`'s numeric-var-in-string-comparison shimmer:
    /// `eq`/`ne`/`lt`/`le`/`gt`/`ge` rewritten to `==`/`!=`/`</<=/>/>=` when
    /// both operands are provably numeric — see
    /// [`expr::find_operator_fix`](crate::shimmer::expr::find_operator_fix)).
    /// Empty when no such fix exists; [`crate::compiler_checks`]'s
    /// `from_shimmer` copies this into the `Diagnostic`-level `fixes` field
    /// consumers already read.
    pub fixes: Vec<crate::irules_checks::CodeFix>,
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
///
/// `source` is the whole compilation unit's source text, forwarded to
/// [`expr::find_expr_shimmers`] to build its eq/ne/lt/le/gt/ge quick fix.
#[must_use]
pub(crate) fn find_shimmer_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, crate::analyses::LatticeValue>,
    source: &str,
) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();
    out.extend(use_site::find_use_site_shimmers(
        cfg,
        ssa,
        types,
        executable_blocks,
        registry,
        values,
    ));
    out.extend(phi::find_phi_shimmers(cfg, ssa, types, executable_blocks));
    out.extend(expr::find_expr_shimmers(
        cfg,
        ssa,
        types,
        executable_blocks,
        source,
    ));
    out
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
            &cu.source,
        ));
    }
    out
}

/// Find every thunking warning across a whole compilation unit. See
/// [`find_shimmer_warnings_for_cu`].
#[must_use]
pub fn find_thunking_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
) -> Vec<ThunkingWarning> {
    let mut out = Vec::new();
    for fu in cu.analysable_functions() {
        out.extend(find_thunking_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
        ));
    }
    out
}

/// Find thunking warnings for a single function.
///
/// Identifies variables that oscillate between two intrep types across
/// loop iterations, causing a type conversion on every pass (S102).
#[must_use]
pub(crate) fn find_thunking_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
) -> Vec<ThunkingWarning> {
    thunking::find_thunking_warnings(cfg, ssa, types, executable_blocks)
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
            None,
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
                "",
            )
            .is_empty()
        );
        assert!(find_thunking_warnings(&f, &ssa, &types, &sccp.executable_blocks).is_empty());
    }
}
