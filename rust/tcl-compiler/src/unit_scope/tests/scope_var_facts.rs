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

//! Literal-value facts for a scope's variables: unknown writes, computed names and scope aliases.

use super::*;

#[test]
fn scope_var_facts_separate_literal_from_unknown_writes() {
    let reg = registry();
    let ir = retained_scope_fixture(
        "proc p {} {\n set a one\n set b [gets stdin]\n set c two\n set c three\n}\n",
        &reg,
    );
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    let facts = collect_scope_var_facts(
        cfg,
        &surface(&reg),
        ir.source_metadata_input
            .as_ref()
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
            .as_deref(),
    );
    assert!(!facts.dynamic_name_write);
    assert_eq!(
        facts.vars["a"].values.iter().cloned().collect::<Vec<_>>(),
        vec!["one".to_owned()],
    );
    assert!(facts.vars["b"].unknown);
    assert_eq!(
        facts.vars["c"].values.iter().cloned().collect::<Vec<_>>(),
        vec!["three".to_owned(), "two".to_owned()],
    );
}

#[test]
fn a_write_through_a_computed_variable_name_poisons_the_whole_scope() {
    let reg = registry();
    let ir = retained_scope_fixture("proc p {n} {\n set a one\n set $n two\n}\n", &reg);
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    assert!(
        collect_scope_var_facts(
            cfg,
            &surface(&reg),
            ir.source_metadata_input
                .as_ref()
                .map(crate::analyser::ResolvedAnalysisInput::context_registry)
                .as_deref()
        )
        .dynamic_name_write
    );
}

#[test]
fn a_scope_alias_declaration_makes_the_aliased_name_unknown() {
    let reg = registry();
    let ir = retained_scope_fixture("proc p {} {\n global cmd\n set other one\n}\n", &reg);
    let cfg_module = build_cfg(&ir, false);
    let cfg = fixture_id_if_present(&cfg_module.procedures, "::p")
        .and_then(|id| cfg_module.procedures.get(&id))
        .expect("p lowered");
    let facts = collect_scope_var_facts(
        cfg,
        &surface(&reg),
        ir.source_metadata_input
            .as_ref()
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
            .as_deref(),
    );
    assert!(
        facts.vars["cmd"].unknown,
        "`global cmd` binds an outer variable any other body may write",
    );
}

#[test]
fn scope_variable_facts_keep_actual_availability_and_refuse_missing_or_foreign_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original normal-write metadata only; no physical variable or frame claim.
    let context = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let registry = context.commands();
    let module = supplied_unit_fixture("dict set target key value", &context);
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let current = collect_scope_var_facts(&cfg.top_level, &surface(registry), Some(&context));
    assert!(!current.dynamic_name_write && !current.cross_frame_write);
    assert!(current.vars["target"].unknown);
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let foreign = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    for withheld in [Some(&older), Some(foreign.as_ref()), None] {
        let facts = collect_scope_var_facts(&cfg.top_level, &surface(registry), withheld);
        assert!(facts.dynamic_name_write && facts.cross_frame_write);
    }
}
