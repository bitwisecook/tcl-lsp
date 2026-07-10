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
    /// `eq`/`ne`/`lt`/`le`/`gt`/`ge` → `==`/`!=`/`<`/`<=`/`>`/`>=`). Empty for
    /// most shimmer findings — the general case is a performance advisory
    /// with no single canonical rewrite the LSP could safely automate.
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

/// The per-function inputs every shimmer sub-pass draws from — bundled so
/// [`find_shimmer_warnings`] takes one argument instead of eight
/// (`clippy::too_many_arguments`); each sub-pass still takes only the
/// individual fields it actually needs.
pub(crate) struct ShimmerInputs<'a> {
    pub cfg: &'a CfgFunction,
    pub ssa: &'a SsaFunction,
    pub types: &'a HashMap<ValueKey, TypeLattice>,
    pub executable_blocks: &'a HashSet<BlockId>,
    pub registry: &'a CommandRegistry,
    pub values: &'a HashMap<ValueKey, crate::analyses::LatticeValue>,
    /// Module-wide `rename`/`interp alias`/proc-redefinition facts — see
    /// [`use_site::find_use_site_shimmers`]'s doc comment.
    pub mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// Whole compilation-unit source text, used only to build the expr
    /// pass's eq/ne/lt/le/gt/ge quick fix — see
    /// [`expr::find_operator_fix`].
    pub source: &'a str,
}

/// Whether `text[start..start + len]` is bounded by non-identifier
/// characters (or a text boundary) on both sides — i.e. it appears as a
/// standalone word, not as a substring of a larger identifier.
///
/// Shared by [`expr::find_operator_fix`] and
/// [`use_site::nested_call_arg_spans`], both of which locate a
/// mechanical-fix target by scanning already-parsed source text rather
/// than re-parsing it (see `find_operator_fix`'s doc comment for why this
/// textual-scan approximation is safe here).
fn is_standalone_word_at(text: &str, start: usize, len: usize) -> bool {
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let before_ok = text[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_word_char(c));
    let after_ok = text[start + len..]
        .chars()
        .next()
        .is_none_or(|c| !is_word_char(c));
    before_ok && after_ok
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
pub(crate) fn find_shimmer_warnings(inputs: &ShimmerInputs<'_>) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();
    out.extend(use_site::find_use_site_shimmers(inputs));
    out.extend(phi::find_phi_shimmers(
        inputs.cfg,
        inputs.ssa,
        inputs.types,
        inputs.executable_blocks,
    ));
    out.extend(expr::find_expr_shimmers(
        inputs.cfg,
        inputs.ssa,
        inputs.types,
        inputs.executable_blocks,
        inputs.source,
    ));
    out
}

/// Find every shimmer warning across a whole compilation unit.
///
/// Public `*_for_cu` entry point (mirroring
/// [`crate::gvn::find_redundancies_for_cu`]) so downstream tooling — the
/// compiler explorer, the MCP server — can run the analysis without
/// re-deriving per-function inputs. Walks every statically-analysable body
/// in `cu.analysable_body_function_units()` order — top-level, every
/// procedure, **and** every `TclOO`/snit method body and synthetic body unit
/// (`apply` lambda, `namespace eval` block). `cu.functions()`/
/// `analysable_functions()` deliberately skip methods and body units (kept
/// for the per-proc passes' established behaviour — see their doc
/// comments); shimmer has no such constraint, and skipping them silently
/// dropped every shimmer/thunking/byte-array finding inside a method or
/// lambda body even though the CFG / SSA / type pipeline already analyses
/// them soundly (`cu.methods` / `cu.body_units` are built through the
/// identical pipeline as `cu.procedures`).
///
/// `mutations` (module-wide `rename`/`interp alias`/proc-redefinition facts)
/// is computed once here via
/// [`crate::command_binding::scan_module_command_mutations`] and shared by
/// every function's use-site pass — see [`use_site::find_use_site_shimmers`]'s
/// doc comment for how it gates command-name trust.
#[must_use]
pub fn find_shimmer_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> Vec<ShimmerWarning> {
    let mutations = crate::command_binding::scan_module_command_mutations(&cu.ir_module, registry);
    let mut out = Vec::new();
    for fu in cu.analysable_body_function_units() {
        out.extend(find_shimmer_warnings(&ShimmerInputs {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            types: &fu.types,
            executable_blocks: &fu.sccp.executable_blocks,
            registry,
            values: &fu.sccp.values,
            mutations: &mutations,
            source: &cu.source,
        }));
    }
    out
}

/// Find every thunking warning across a whole compilation unit. See
/// [`find_shimmer_warnings_for_cu`] (including method/body-unit coverage).
#[must_use]
pub fn find_thunking_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
) -> Vec<ThunkingWarning> {
    let mut out = Vec::new();
    for fu in cu.analysable_body_function_units() {
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
    for fu in cu.analysable_body_function_units() {
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

    /// A per-iteration shimmer inside a `TclOO` method body must fire exactly
    /// like the identical code in an ordinary `proc` — `find_shimmer_warnings_for_cu`
    /// must walk `cu.methods`, not just `cu.procedures`/`cu.top_level`
    /// (`cu.analysable_functions()` deliberately excludes methods; this CU-level
    /// entry point must use the coverage-complete `all_body_function_units()`
    /// instead — see its doc comment).
    #[test]
    fn shimmer_fires_inside_tcloo_method_body() {
        let src = "oo::class create Foo {\n  method run {items} {\n    foreach x $items {\n      set y [lindex $x 0]\n      incr x\n    }\n  }\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        assert!(
            !cu.methods.is_empty(),
            "expected a TclOO method unit to be built"
        );
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::S101),
            "expected an S101 shimmer inside the method body, got: {warnings:?}"
        );
    }

    /// The same coverage gap for S102 (thunking) — a genuine loop-body
    /// oscillation inside a method must still fire.
    #[test]
    fn thunking_fires_inside_tcloo_method_body() {
        let src = "oo::class create Foo {\n  method run {n} {\n    set x 0\n    while {$n} {\n      set x \"s\"\n      set x [list 1]\n    }\n    return $x\n  }\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_thunking_warnings_for_cu(&cu);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::S102),
            "expected an S102 thunking warning inside the method body, got: {warnings:?}"
        );
    }

    /// (TP) A call through an `interp alias`-created name is checked exactly
    /// like a call to its target: `li` aliases `lindex`, so `li $x 0` on an
    /// Int-typed `x` shimmers Int→List, identically to calling `lindex`
    /// directly.
    #[test]
    fn shimmer_fires_through_interp_alias() {
        let src = "interp alias {} li {} lindex\nproc f {} {\n  set x 5\n  li $x 0\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        let w = warnings.iter().find(|w| w.variable == "x");
        assert!(
            w.is_some(),
            "expected a shimmer through the 'li' alias to lindex, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// (TN control) The identical code with no alias in play still fires —
    /// confirms `shimmer_fires_through_interp_alias` isn't vacuously true.
    #[test]
    fn shimmer_fires_for_direct_lindex_call_control() {
        let src = "proc f {} {\n  set x 5\n  lindex $x 0\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.variable == "x" && w.command == "lindex"),
            "expected the direct-call control to still fire: {warnings:?}"
        );
    }

    /// (FP guard) `rename incr {}` deletes the builtin `incr` module-wide —
    /// any later `incr`-shaped call cannot be trusted to mean Tcl's builtin
    /// (the interpreter would in fact raise "invalid command name" at
    /// runtime), so `find_shimmer_warnings_for_cu` must not claim an
    /// incr-shimmer for it.
    #[test]
    fn no_shimmer_for_incr_after_module_wide_rename() {
        let src = "rename incr {}\nproc f {} {\n  set x \"hello\"\n  incr x\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            !warnings.iter().any(|w| w.command == "incr"),
            "incr must not be trusted after a module-wide rename: {warnings:?}"
        );
    }

    /// (TN control) Without the `rename`, the identical `incr` call still
    /// fires — confirms the guard above isn't vacuous.
    #[test]
    fn shimmer_fires_for_incr_without_rename_control() {
        let src = "proc f {} {\n  set x \"hello\"\n  incr x\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            warnings.iter().any(|w| w.command == "incr"),
            "expected the no-rename control to still fire: {warnings:?}"
        );
    }

    /// (FP guard) A `rename lindex myLindex` moves the builtin away from the
    /// bare `lindex` spelling module-wide; a later literal `lindex` call
    /// (now denoting whatever the module rebound it to, or nothing at all)
    /// must not be trusted as the original builtin.
    #[test]
    fn no_shimmer_for_lindex_after_module_wide_rename() {
        let src = "rename lindex myLindex\nproc f {} {\n  set x 5\n  lindex $x 0\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            !warnings.iter().any(|w| w.command == "lindex"),
            "lindex must not be trusted after a module-wide rename: {warnings:?}"
        );
    }

    /// (FP guard) `my variable count` links `count` to the object's
    /// instance-variable storage — it does not itself assign `count` any
    /// value, so its true intrep depends on whatever *other* method last set
    /// it to (not knowable locally). Before the registry fix (`oo_my.rs`'s
    /// `variable` subcommand: `creates_scope_alias` + an `arg_role_resolver`
    /// so `count` is even recognised as a def in the first place), the
    /// generic `Statement::Call` fallback typed `count` from `my`'s own
    /// nominal `return_type: Some(TclType::String)` — spuriously claiming
    /// `count` is a fresh `String` and firing an S100 shimmer on `incr
    /// count` that has nothing to do with the variable's real, externally-set
    /// value.
    #[test]
    fn no_shimmer_for_incr_on_tcloo_instance_variable() {
        let src = "oo::class create Foo {\n  method bump {} {\n    my variable count\n    incr count\n  }\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            !warnings.iter().any(|w| w.variable == "count"),
            "'my variable'-linked instance var must not spuriously shimmer: {warnings:?}"
        );
    }

    /// (TN control) A genuinely *local* variable in the same method body
    /// still shimmers normally — `my variable` linkage must not blanket-
    /// suppress shimmer detection for the whole method
    /// (`shimmer_fires_inside_tcloo_method_body` already covers this more
    /// broadly; this variant specifically confirms the suppression is
    /// scoped to the linked name, not the enclosing method).
    #[test]
    fn shimmer_still_fires_for_local_var_alongside_instance_variable() {
        let src = "oo::class create Foo {\n  method bump {} {\n    my variable count\n    set local \"hello\"\n    incr local\n  }\n}\n";
        let cu = crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false);
        let warnings = find_shimmer_warnings_for_cu(&cu, &registry());
        assert!(
            warnings.iter().any(|w| w.variable == "local"),
            "local var shimmer must still fire alongside 'my variable': {warnings:?}"
        );
    }

    /// API smoke-test: both entry points accept an empty function.
    #[test]
    fn find_shimmer_warnings_empty_function() {
        let f = Function::new("::top", "entry");
        let ssa = crate::ssa::build_ssa(&f, &registry());
        let sccp = crate::sccp::sccp(&f, &ssa, None, None, None);
        let types: HashMap<ValueKey, TypeLattice> = HashMap::new();
        assert!(
            find_shimmer_warnings(&ShimmerInputs {
                cfg: &f,
                ssa: &ssa,
                types: &types,
                executable_blocks: &sccp.executable_blocks,
                registry: &registry(),
                values: &sccp.values,
                mutations: &crate::command_binding::ModuleCommandMutations::default(),
                source: "",
            })
            .is_empty()
        );
        assert!(find_thunking_warnings(&f, &ssa, &types, &sccp.executable_blocks).is_empty());
    }
}
