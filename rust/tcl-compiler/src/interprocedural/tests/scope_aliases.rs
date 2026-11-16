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

//! `global`, `variable` and `upvar` aliases: which writes through them count as writes of global state.

use super::*;

fn source_point_trace(source: &str, body: &str) -> Vec<String> {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = CompilationUnit::build_for_profile(source, &registry, false, profile);
    let statements = fixture_id_if_present(&unit.ir_module.procedures, body)
        .and_then(|id| unit.ir_module.procedures.get(&id))
        .map(|row| &row.body)
        .or_else(|| {
            fixture_id_if_present(&unit.ir_module.methods, body)
                .and_then(|id| unit.ir_module.methods.get(&id))
                .map(|row| &row.body)
        });
    statements.into_iter().flat_map(|body| body.statements.iter().map(|statement| {
            let Some(tokens) = body.retained_source_tokens_for_statement(statement) else {
                return format!("no source carrier {:?}", statement.span());
            };
            let Some(binding) = &tokens.source_binding else {
                return format!("{:?}: no source binding", tokens.argv_texts);
            };
            let normal = crate::registry_invocation::normal_transfer_invocation(
                &registry, None, tokens,
            );
            format!(
                "{:?}: unknown={} absent={} handler={:?} frame={:?} ns={} known={} globals={:?} ns_vars={:?} mutations={:?}",
                tokens.argv_texts, binding.unknown, binding.may_be_absent,
                binding.proved_handler_target().map(|target| (&target.command, target.registry_backed)),
                binding.variable_context.frame_kind, binding.variable_context.namespace,
                binding.variable_context.namespace_known, binding.variable_context.globals,
                binding.variable_context.ns_vars,
                normal.map(|normal| normal.mutation_places(&binding.variable_context, &registry)),
            )
        })).collect()
}

#[test]
fn positioned_alias_traits_use_selected_frame_and_transition_grammar() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        for (level, caller_write) in [("0x1", true), ("#0", false), ("0", false)] {
            let source = format!("proc writer {{n}} {{upvar {level} $n local; set local NEW}}");
            let unit = CompilationUnit::build_for_dialect(&source, registry, false, dialect);
            let identities =
                crate::realm::document_realm_bindings_for_module(&unit.ir_module, registry)
                    .expect("fixture's retained Module source owner");
            let analysis = build_interprocedural_analysis(
                &unit.ir_module,
                registry,
                Some(profile),
                ObjectTypeMap::none(),
                &identities,
                None,
            );
            let traits = &analysis
                .procedures
                .get(&fixture_id(&analysis.procedures, "::writer"))
                .expect("authored fixture body")
                .param_traits["n"];
            assert!(
                traits.contains(&ProcArgTrait::VarRead),
                "{dialect} {level}: {traits:?}; source body: {:#?}",
                unit.ir_module
                    .procedures
                    .get(&fixture_id(&unit.ir_module.procedures, "::writer"))
                    .expect("authored fixture body")
                    .body
            );
            assert_eq!(
                traits.contains(&ProcArgTrait::VarWrite),
                caller_write,
                "only the caller-frame alias writes this caller parameter: {dialect} {level}: {traits:?}"
            );
        }
    }
}

// writes_global recognises scope aliases

#[test]
fn global_alias_write_counts_as_writes_global() {
    // A bare `set g` after `global g` mutates a caller-visible
    // variable through the alias — writes_global, even though the
    // written name is bare, not `::`-qualified.
    for src in [
        "proc ::f {} { global g\nset g 5 }",
        "proc ::f {} { variable v\nset v 1 }",
        "proc ::f {} { upvar #0 ::x g\nset g 1 }",
        "proc ::f {} { global g\nincr g }",
        "proc ::f {} { global g\nappend g x }",
    ] {
        let ia = build(src);
        let s = fixture_proc(&ia, "::f").expect("::f summary");
        assert!(
            s.writes_global,
            "expected writes_global for: {src}; points={:?}",
            source_point_trace(src, "::f"),
        );
    }
}

/// An alias pair whose level selects the current frame stays a global
/// alias: its other variable is in practice a qualified, computed or
/// namespace-declared name (`upvar 0 $token state`), which reaches a
/// namespace cell from this frame. Its level is read as a `FrameLevel`,
/// so every spelling of the global frame and of this one counts.
#[test]
fn standalone_empty_alias_declaration_and_mutation_have_distinct_write_effects() {
    let registry = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let known = HashSet::new();
    let params = HashSet::new();
    let module = crate::ir::Module::default();
    let ctx = ScanCtx {
        known: &known,
        registry: &registry,
        dialect: None,
        params: &params,
        object_types: ObjectTypeMap::none().candidates,
        identities: crate::realm::CommandBindingRealm::none(),
        declared: None,
        positioned_substitutions: false,
        source_input: None,
        source_procedures: &module.procedures,
        source_module: &module,
        current_tokens: None,
        parameter_body: None,
    };
    let mut facts = LocalFacts::default();
    scan_call_statement(
        "global",
        &[String::new()],
        &[String::new()],
        None,
        ctx,
        &mut facts,
    );
    assert!(facts.global_aliases.contains(""));
    assert!(
        !facts.writes_global,
        "declaring the literal empty alias is not a value write"
    );
    scan_call_statement(
        "append",
        &[String::new(), "VALUE".into()],
        &[String::new()],
        None,
        ctx,
        &mut facts,
    );
    assert!(
        facts.writes_global,
        "mutation reaches the retained literal empty alias"
    );
}

#[test]
fn literal_empty_global_alias_mutations_remain_visible() {
    let analysis = build("proc ::f {} {global {}; append {} VALUE}");
    assert!(
        fixture_proc(&analysis, "::f")
            .expect("authentic standalone declaration")
            .writes_global
    );
}

#[test]
fn nominal_global_alias_coverage_keeps_empty_data_separate_from_unknown_names() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let coverage =
        global_alias_names("global", &[String::new(), "array(key)".into()], &registry).unwrap();
    assert!(coverage.names.contains("") && coverage.names.contains("array"));
    assert!(!coverage.unknown);
    let coverage = global_alias_names("global", &["$computed".into()], &registry).unwrap();
    assert!(coverage.names.is_empty() && coverage.unknown);
    let coverage = global_alias_names(
        "upvar",
        &["#0".into(), "source".into(), String::new()],
        &registry,
    )
    .unwrap();
    assert!(coverage.names.contains("") && !coverage.unknown);
    assert!(
        global_alias_names(
            "upvar",
            &["1".into(), "source".into(), "local".into()],
            &registry
        )
        .is_none()
    );
}

#[test]
fn an_alias_at_the_current_or_global_frame_is_a_global_alias() {
    for src in [
        "proc ::f {} { upvar 0 ::x g\nset g 1 }",
        "proc ::f {t} { upvar +0 $t g\nset g 1 }",
        "proc ::f {} { upvar #00 x g\nset g 1 }",
    ] {
        let ia = build(src);
        assert!(
            ia.procedures
                .get(&fixture_id(&ia.procedures, "::f"))
                .expect("authored fixture body")
                .writes_global,
            "{src}"
        );
    }
}

#[test]
fn non_aliased_local_write_is_not_writes_global() {
    // A plain local `set g` (no global/variable/upvar #0 decl) is
    // not a global write; `upvar 1` aliases a caller frame, not
    // global scope, so it is not flagged here either.
    for src in [
        "proc ::f {} { set g 5 }",
        "proc ::f {} { upvar 1 x g\nset g 1 }",
    ] {
        let ia = build(src);
        let s = fixture_proc(&ia, "::f").expect("::f summary");
        assert!(!s.writes_global, "unexpected writes_global for: {src}");
    }
}
