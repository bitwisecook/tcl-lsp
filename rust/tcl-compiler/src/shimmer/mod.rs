//! Intrep shimmer analysis — detect places where a variable's
//! Tcl-value intrep (list/dict/int/…) is converted at a use site.
//!
//! Ported from `core/compiler/shimmer.py` (C27d). Decomposed into
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

pub mod expr;
pub mod graph;
pub mod hints;
pub mod phi;
pub mod span;
pub mod thunking;
pub mod use_site;

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, TclType};

use crate::cfg::Function as CfgFunction;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::TypeLattice;

// Re-export the graph helpers that are part of the historical public API.

// ---------------------------------------------------------------------------
// Shared diagnostic types
// ---------------------------------------------------------------------------

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
    pub code: String,
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
    pub code: String,
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

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

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
    executable_blocks: &HashSet<String>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, crate::analyses::LatticeValue>,
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
    out.extend(expr::find_expr_shimmers(cfg, ssa, types, executable_blocks));
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
    executable_blocks: &HashSet<String>,
) -> Vec<ThunkingWarning> {
    thunking::find_thunking_warnings(cfg, ssa, types, executable_blocks)
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
        let sccp = crate::sccp::sccp(&f, &ssa, None, None);
        let types: HashMap<ValueKey, TypeLattice> = HashMap::new();
        assert!(
            find_shimmer_warnings(
                &f,
                &ssa,
                &types,
                &sccp.executable_blocks,
                &registry(),
                &sccp.values
            )
            .is_empty()
        );
        assert!(find_thunking_warnings(&f, &ssa, &types, &sccp.executable_blocks).is_empty());
    }
}
