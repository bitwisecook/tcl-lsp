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

//! Public driver for the var-escape analysis.
//!
//! The orchestrator threads the
//! intraprocedural pass ([`analyse_script`] /
//! [`analyse_cfg_function`]), the interprocedural fixpoint
//! ([`solve_interprocedural_escape`]), and slot resolution
//! ([`populate_local_slots`]) — into per-proc [`ProcEscapeSummary`]s keyed
//! by qualified name.
//!
//! Two source modes:
//!
//! * [`analyse_var_escape`] — the **IR-only tree walk**. This is the path
//!   the inliner consumes (`pure_leaf` is computed here); it needs only the
//!   lowered [`Module`]. [`analyse_var_escape_with_registry`] is the same
//!   walk with a dialect/profile registry threaded through.
//! * [`analyse_var_escape_cu`] — the **flow-sensitive CFG + SSA** path,
//!   driven from a [`CompilationUnit`]. Used by codegen for the per-SSA
//!   frame analysis; it leaves `pure_leaf` at its default (the inlining
//!   predicate is only meaningful on the IR path).

use std::collections::HashMap;

use crate::compilation_unit::CompilationUnit;
use crate::ir::Module;
use tcl_core_types::BodyFrameId;

use super::cfg_propagation::{CfgEscapeResult, analyse_cfg_function_with_registry};
use super::interprocedural::solve_interprocedural_escape_with_registry;
use super::slot_resolution::populate_local_slots;
use super::types::{EscapeTag, ProcEscapeSummary};
use super::walker::{analyse_script, analyse_script_with_registry};

/// Qualified name the top-level script is keyed under. Callers that only
/// care about proc bodies filter it out.
pub const TOP_LEVEL_QNAME: &str = "::top";

/// Convert a flow-sensitive [`CfgEscapeResult`] to a per-name
/// [`ProcEscapeSummary`]. Per-SSA-version tags are collapsed by
/// "FRAME wins" (already done in `name_tags`); the per-version detail is
/// preserved in [`ProcEscapeSummary::ssa_tags`]. `pure_leaf` is left at
/// its default — the inlining predicate is computed only on the IR-walk
/// path.
#[must_use]
pub fn cfg_result_to_summary(result: &CfgEscapeResult) -> ProcEscapeSummary {
    let frame_needed =
        result.dynamic_barrier() || result.name_tags.values().any(|t| *t == EscapeTag::Frame);
    ProcEscapeSummary {
        tags: result.name_tags.clone(),
        flags: result.flags,
        frame_needed,
        upvar_source_names: result.upvar_source_names.clone(),
        direct_callees: result.direct_callees.clone(),
        ssa_tags: result.ssa_tags.clone(),
        local_slots: std::collections::BTreeMap::new(),
        barriers: result.barriers.clone(),
        tag_reasons: result.tag_reasons.clone(),
        pure_leaf: false,
    }
}

/// Per-proc escape summaries from the lowered IR module (tree-walk path),
/// keyed by qualified name with the top level under [`TOP_LEVEL_QNAME`].
///
/// When `interprocedural` is `true` (the production default) callee-induced
/// escapes and the transitive `pure_leaf` fixpoint are folded in; pass
/// `false` to inspect the raw per-proc result (tests). Compile-time local
/// slot indices are always folded in last.
#[must_use]
pub fn analyse_var_escape(
    module: &Module,
    interprocedural: bool,
) -> HashMap<BodyFrameId, ProcEscapeSummary> {
    if let Some(snapshot) = &module.registry_snapshot {
        return analyse_var_escape_with_registry(module, interprocedural, snapshot.registry());
    }
    let mut result: HashMap<BodyFrameId, ProcEscapeSummary> = HashMap::new();
    result.insert(
        BodyFrameId::Top(module.procedure_ids.module_id()),
        analyse_script(&module.top_level, std::iter::empty::<String>()),
    );
    for (id, proc) in module.procedures.iter().chain(module.body_units.iter()) {
        result.insert(
            BodyFrameId::Body(*id),
            analyse_script(&proc.body, proc.params.iter().cloned()),
        );
    }
    for (id, method) in &module.methods {
        result.insert(
            BodyFrameId::Body(*id),
            analyse_script(&method.body, method.params.iter().cloned()),
        );
    }
    if interprocedural {
        for summary in result.values_mut() {
            summary.pure_leaf = false;
        }
    }
    populate_local_slots(&result, Some(module))
}

/// Registry-aware form of [`analyse_var_escape`]. This is the production
/// entry point when lowering has already selected a dialect/profile registry.
#[must_use]
pub fn analyse_var_escape_with_registry(
    module: &Module,
    interprocedural: bool,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<BodyFrameId, ProcEscapeSummary> {
    let mut result: HashMap<BodyFrameId, ProcEscapeSummary> = HashMap::new();
    result.insert(
        BodyFrameId::Top(module.procedure_ids.module_id()),
        analyse_script_with_registry(&module.top_level, std::iter::empty::<String>(), registry),
    );
    for (id, proc) in module.procedures.iter().chain(module.body_units.iter()) {
        result.insert(
            BodyFrameId::Body(*id),
            analyse_script_with_registry(&proc.body, proc.params.iter().cloned(), registry),
        );
    }
    for (id, method) in &module.methods {
        result.insert(
            BodyFrameId::Body(*id),
            analyse_script_with_registry(&method.body, method.params.iter().cloned(), registry),
        );
    }
    if interprocedural {
        result = solve_interprocedural_escape_with_registry(
            &result,
            &super::interprocedural::EscapeCallGraph::for_module(module, registry),
            registry,
        );
    }
    populate_local_slots(&result, Some(module))
}

/// Per-proc escape summaries from a [`CompilationUnit`] (the flow-sensitive
/// CFG/SSA path). Used by codegen frame analysis.
///
/// Uses the immutable command store retained by the unit. Missing ownership
/// keeps the independent CFG analyses conservative.
#[must_use]
pub fn analyse_var_escape_cu(
    cu: &CompilationUnit,
    interprocedural: bool,
) -> HashMap<BodyFrameId, ProcEscapeSummary> {
    if let Some(snapshot) = &cu.ir_module.registry_snapshot {
        return analyse_var_escape_cu_with_registry(cu, interprocedural, snapshot.registry());
    }
    let mut result = HashMap::new();
    result.insert(
        BodyFrameId::Top(cu.ir_module.procedure_ids.module_id()),
        cfg_result_to_summary(&super::cfg_propagation::analyse_cfg_function(
            &cu.top_level.cfg,
            &cu.top_level.ssa,
            std::iter::empty::<String>(),
        )),
    );
    for (id, function) in cu
        .procedures
        .iter()
        .chain(cu.methods.iter())
        .chain(cu.body_units.iter())
    {
        result.insert(
            BodyFrameId::Body(*id),
            cfg_result_to_summary(&super::cfg_propagation::analyse_cfg_function(
                &function.cfg,
                &function.ssa,
                cu.ir_module
                    .procedures
                    .get(id)
                    .or_else(|| cu.ir_module.body_units.get(id))
                    .map(|body| body.params.clone())
                    .or_else(|| cu.ir_module.methods.get(id).map(|body| body.params.clone()))
                    .unwrap_or_default(),
            )),
        );
    }
    populate_local_slots(&result, Some(&cu.ir_module))
}

/// Registry-aware form of [`analyse_var_escape_cu`]. This is the production
/// entry point when lowering has already selected a dialect/profile registry.
///
/// The walk consults the registry for command traits (`FRAMELESS_RUNTIME`,
/// `EXPANSION_ESCAPE_SAFE`, `FIRST_ARG_VARNAME`), semantic operation ids, and
/// `profile()` for the lexer config it re-lowers nested bodies under — so the
/// registry a unit was lowered with and `tcl8.6` need not agree, and an
/// escape analysis that under-reports escaping variables licenses
/// optimisation a correct one would refuse (#2167).
#[must_use]
pub fn analyse_var_escape_cu_with_registry(
    cu: &CompilationUnit,
    interprocedural: bool,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<BodyFrameId, ProcEscapeSummary> {
    let mut result: HashMap<BodyFrameId, ProcEscapeSummary> = HashMap::new();
    let top = analyse_cfg_function_with_registry(
        &cu.top_level.cfg,
        &cu.top_level.ssa,
        std::iter::empty::<String>(),
        registry,
    );
    result.insert(
        BodyFrameId::Top(cu.ir_module.procedure_ids.module_id()),
        cfg_result_to_summary(&top),
    );
    for (qname, fu) in cu
        .procedures
        .iter()
        .chain(cu.methods.iter())
        .chain(cu.body_units.iter())
    {
        let params = cu
            .ir_module
            .procedures
            .get(qname)
            .or_else(|| cu.ir_module.body_units.get(qname))
            .map(|p| p.params.clone())
            .or_else(|| {
                cu.ir_module
                    .methods
                    .get(qname)
                    .map(|method| method.params.clone())
            })
            .unwrap_or_default();
        let proc = analyse_cfg_function_with_registry(&fu.cfg, &fu.ssa, params, registry);
        result.insert(BodyFrameId::Body(*qname), cfg_result_to_summary(&proc));
    }
    if interprocedural {
        result = solve_interprocedural_escape_with_registry(
            &result,
            &super::interprocedural::EscapeCallGraph::for_module(&cu.ir_module, registry),
            registry,
        );
    }
    populate_local_slots(&result, Some(&cu.ir_module))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn summaries(src: &str) -> (Module, HashMap<BodyFrameId, ProcEscapeSummary>) {
        let module = lower_to_ir(src, &CommandRegistry::build_default());
        let values = analyse_var_escape(&module, true);
        (module, values)
    }

    fn fixture_key(module: &Module, display: &str) -> BodyFrameId {
        let mut matching = module
            .procedures
            .iter()
            .filter(|(_, body)| body.qualified_name == display);
        let id = *matching.next().expect("authored fixture body").0;
        assert!(matching.next().is_none(), "ambiguous authored fixture body");
        BodyFrameId::Body(id)
    }

    #[test]
    fn implicit_escape_driver_uses_the_retained_actual_registry() {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir("info exists literal", &registry);
        let implicit = analyse_var_escape(&module, false);
        let explicit = analyse_var_escape_with_registry(&module, false, &registry);
        assert_eq!(implicit, explicit);
        let mut detached = module;
        detached.registry_snapshot = None;
        for statement in &mut detached.top_level.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.source_binding = None;
            }
        }
        detached.top_level.command_binding_sites = Default::default();
        assert!(
            analyse_var_escape(&detached, false)
                .values()
                .all(|summary| { summary.dynamic_barrier() && !summary.pure_leaf })
        );
    }

    #[test]
    fn pure_leaf_proc_is_flagged() {
        // A leaf proc that only does pure value computation is pure-leaf.
        let (module, s) = summaries("proc ::add {a b} { return [expr {$a + $b}] }");
        let add = &s[&fixture_key(&module, "::add")];
        assert!(add.pure_leaf, "pure leaf proc should be flagged");
        assert!(add.safe_to_inline());
        assert!(add.safe_to_dce());
        assert!(add.safe_for_frame_elision());
    }

    #[test]
    fn upvar_proc_is_not_pure_leaf() {
        // `upvar` makes the proc observe a caller frame — not pure-leaf.
        let (module, s) = summaries("proc ::setit {name val} { upvar 1 $name v\n set v $val }");
        assert!(
            !s[&fixture_key(&module, "::setit")].pure_leaf,
            "upvar proc must not be pure-leaf"
        );
    }

    #[test]
    fn caller_of_impure_proc_is_downgraded() {
        // `::wrap` is locally pure but calls `::setit` (upvar) — the
        // transitive fixpoint downgrades `::wrap`.
        let (module, s) = summaries(
            "proc ::setit {name val} { upvar 1 $name v\n set v $val }\n\
             proc ::wrap {} { setit x 1 }",
        );
        assert!(!s[&fixture_key(&module, "::setit")].pure_leaf);
        assert!(
            !s[&fixture_key(&module, "::wrap")].pure_leaf,
            "caller of an impure proc must be downgraded, got {:?}",
            s[&fixture_key(&module, "::wrap")].pure_leaf,
        );
    }

    #[test]
    fn caller_of_frameless_builtin_stays_pure_leaf() {
        // Calling only frameless runtime builtins (`puts`) keeps pure-leaf.
        let (module, s) = summaries("proc ::log {} { puts hi }");
        assert!(
            s[&fixture_key(&module, "::log")].pure_leaf,
            "wrapper of a frameless builtin is pure-leaf"
        );
    }

    #[test]
    fn interprocedural_flag_off_keeps_pure_leaf() {
        // The `interprocedural=false` path still runs the intraprocedural
        // pass, so a genuinely pure leaf is flagged without the fixpoint.
        let module = lower_to_ir(
            "proc ::add {a b} { return [expr {$a + $b}] }",
            &CommandRegistry::build_default(),
        );
        let raw = analyse_var_escape(&module, false);
        assert!(
            raw[&fixture_key(&module, "::add")].pure_leaf,
            "pure leaf flagged with IPA off"
        );
        // And the top level is present under the canonical key.
        assert!(raw.contains_key(&BodyFrameId::Top(module.procedure_ids.module_id())));
    }
}
