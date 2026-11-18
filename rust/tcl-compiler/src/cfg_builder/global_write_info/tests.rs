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

use super::*;
use tcl_registry::{ArgRole, CommandRegistry, CommandSpec, StateTransitionDescriptor};
fn summary_id(module: &Module, label: &str) -> crate::ir::ProcedureId {
    let mut matching: Vec<_> = module
        .original_source_procedure_bodies()
        .filter(|(_, procedure)| {
            procedure.qualified_name == label
                || procedure.name == label
                || tcl_syntax::naming::key_tail(&procedure.name) == label
        })
        .map(|(id, _)| *id)
        .collect();
    if matching.is_empty() {
        let registry = module.resolved_registry();
        let owners = super::super::OriginalProcedureSummaryOwners::for_module(module, registry);
        for script in std::iter::once(&module.top_level).chain(
            module
                .original_source_procedure_bodies()
                .map(|(_, procedure)| &procedure.body),
        ) {
            for statement in &script.statements {
                let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
                    continue;
                };
                if super::super::summary_candidates::original_head(tokens).as_deref() == Some(label)
                {
                    for (id, _) in owners
                        .calls(tokens, registry)
                        .expect("original fixture call is closed")
                    {
                        if !matching.contains(&id) {
                            matching.push(id);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        matching.len(),
        1,
        "authored fixture summary {label:?} must identify one body"
    );
    matching[0]
}

fn module(src: &str) -> Module {
    module_with_registry(src, &CommandRegistry::build_default())
}

fn module_with_registry(src: &str, registry: &CommandRegistry) -> Module {
    module_in(src, registry, "tcl8.6")
}

/// The lenient `tcl` environment: a Logical source with no native naming.
fn logical_module(src: &str) -> Module {
    module_in(src, &CommandRegistry::build_default(), "tcl")
}

fn module_in(src: &str, registry: &CommandRegistry, environment: &str) -> Module {
    let context = tcl_registry::model::ingress::static_context_for(environment)
        .with_command_store(registry.snapshot().shared_registry());
    let profile = tcl_registry::model::ingress::static_context_for(environment)
        .commands()
        .profile()
        .unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        config,
    );
    let lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_dialect(Some(profile))
        .with_resolved_analysis_input(input);
    crate::lowering::lower_to_ir_with(lowerer, src)
}

/// The globals a summary names, each spelling unrooted: the naming owner
/// records an outer-scope write under its rooted and its plain spelling
/// (`variable_observation_names::record_variable_write_name`).
fn unrooted_names(row: &GlobalWriteInfo) -> BTreeSet<&str> {
    row.names
        .iter()
        .map(|name| tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name))
        .collect()
}

#[test]
fn a_global_link_spelled_with_a_colon_run_writes_the_same_global() {
    // `global ::g` and `global ::::g` link the same global: tclsh 8.4.20,
    // 8.6.18, 9.0.4 and 9.1.0 print 1 then 2 for `left; puts $::g; right;
    // puts $::g`. Each procedure's summary names g, so no read of ::g folds
    // across a call to either (rewritten from an internal footprint check by
    // ruling of 17 November).
    let original =
        module("proc left {} {global ::g; set g 1}\nproc right {} {global ::::g; set g 2}");
    let rows = detect_global_write_procs(&original);
    for name in ["::left", "::right"] {
        let row = &rows[&summary_id(&original, name)];
        assert_eq!(
            unrooted_names(row),
            BTreeSet::from(["g"]),
            "{name}: {row:?}"
        );
        assert!(!row.opaque_global_frame, "{name}");
    }
    let unknown = module("proc p {} {global g; set g 1; unknown_call}");
    let rows = detect_global_write_procs(&unknown);
    let row = &rows[&summary_id(&unknown, "::p")];
    assert!(row.names.contains("g"));
    assert!(
        row.opaque_global_frame,
        "a known write cannot close an unknown call remainder"
    );
}

#[test]
fn original_global_source_write_keeps_empty_scalar_and_array_root() {
    // experiments/name-handling/results/source-empty-write-projection-20261014/oracles/observations.json
    for body in ["set {} SCALAR", "set {(key)} ARRAY"] {
        let module = module(&format!("proc p {{}} {{uplevel #0 {{{body}}}}}"));
        assert!(module.source_entry.native_entry.is_none());
        let summaries = detect_global_write_procs(&module);
        assert!(summaries[&summary_id(&module, "::p")].names.contains(""));
    }
    let dynamic = module("proc p {name} {uplevel #0 {set $name VALUE}}");
    assert!(detect_global_write_procs(&dynamic)[&summary_id(&dynamic, "::p")].opaque_global_frame);
}

#[test]
fn original_immediate_body_inventory_keeps_protected_children_and_refusals() {
    let module = logical_module("catch {set ::hits VALUE}");
    let registry = module.resolved_registry();
    let parent = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.first().unwrap())
        .unwrap();
    let children =
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module, registry, parent,
        )
        .expect("complete original protected body");
    assert_eq!(children.len(), 1);
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(
        crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
            registry,
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, &module)
                .unwrap(),
            &children[0]
        )
        .unwrap()
        .0
        .literal_names,
        vec!["::hits"]
    );
    let mut stale = parent.clone();
    stale.argv_texts[1] = "set ::other VALUE".to_owned();
    assert!(
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module, registry, &stale,
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.source_metadata_input = None;
    assert!(
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &changed, registry, parent,
        )
        .is_none()
    );
    let dynamic = self::module("catch $script");
    let parent = dynamic
        .top_level
        .retained_source_tokens_for_statement(dynamic.top_level.statements.first().unwrap())
        .unwrap();
    assert!(
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &dynamic,
            dynamic.resolved_registry(),
            parent,
        )
        .is_none()
    );
}

#[test]
fn logical_source_child_effects_keep_readonly_ownership_and_refuse_stale_modules() {
    let module = logical_module("catch {set ::hits VALUE}");
    let registry = module.resolved_registry();
    let input = module.source_metadata_input.as_ref().unwrap();
    let parent = module
        .top_level
        .retained_source_tokens_for_statement(&module.top_level.statements[0])
        .unwrap();
    let children =
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module, registry, parent,
        )
        .unwrap();
    let child = &children[0];
    let binding = child.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    assert!(
        binding
            .original_invocation_metadata_for_module(child, &module, registry)
            .is_none()
    );
    let words = binding
        .original_logical_source_words(child, input, registry)
        .unwrap();
    assert!(matches!(
        words.source(),
        crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(_)
    ));
    assert!(!words.operands_preserve_source_lookup());
    assert_eq!(
        words
            .with_source_schema(
                input.borrowed_context_registry(),
                tcl_registry::CommandRegistry::variable_write_projection_for_selected_source
            )
            .unwrap()
            .literal_names,
        vec!["::hits"]
    );
    let mut stale = child.clone();
    stale.argv_texts[1] = "::other".to_owned();
    assert!(
        crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            &module, registry, &stale
        )
        .is_none()
    );
    let mut missing_carrier = child.clone();
    missing_carrier.source_binding =
        Some(crate::command_binding::SourceInvocationBinding::default());
    assert!(
        crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            &module,
            registry,
            &missing_carrier
        )
        .is_none()
    );
    let mut missing_words = child.clone();
    missing_words.source_binding = None;
    assert!(
        crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            &module,
            registry,
            &missing_words
        )
        .is_none()
    );
    for facet in 0..5 {
        let mut stale = module.clone();
        match facet {
            0 => stale.source_metadata_input = None,
            1 => stale.source = tcl_lexer::SourceImage::document("catch {set ::other VALUE}"),
            2 => stale.retained_source_bindings = None,
            3 => stale.lexer_config.strict_quoting = !stale.lexer_config.strict_quoting,
            4 => stale.source_entry.unknown_entry = true,
            _ => unreachable!(),
        }
        assert!(crate::registry_invocation::source_structure::original_logical_registry_words_for_module(&stale, registry, child).is_none(), "changed facet {facet}");
    }
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut stronger = module.clone();
    stronger.source_entry.native_entry = Some(std::sync::Arc::new(entry));
    assert!(
        crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            &stronger, registry, child
        )
        .is_none()
    );
}

#[test]
fn a_catch_bodys_nested_substitution_writes_its_globals() {
    // `catch {set ::hits [incr ::other]}` writes both globals: tclsh 8.6.18,
    // 9.0.4 and 9.1.0 print `1 1` for `p; puts "$::hits $::other"` (8.4's
    // `incr` refuses an unset variable, so the module runs as 8.6). The
    // summary names both, so neither folds across a call (rewritten from an
    // internal carrier check by ruling of 17 November).
    let m = module("proc p {} {catch {set ::hits [incr ::other]}}");
    let rows = detect_global_write_procs(&m);
    let row = &rows[&summary_id(&m, "::p")];
    assert_eq!(
        unrooted_names(row),
        BTreeSet::from(["hits", "other"]),
        "{row:?}"
    );
}

#[test]
fn original_substitution_protected_bodies_share_global_write_coverage() {
    // experiments/name-handling/results/immediate-protected-body-writes-20261014/oracles/observations.json
    for body in [
        "catch {incr ::hits}",
        "catch {catch {incr ::hits}}",
        "try {incr ::hits} finally {incr ::done}",
    ] {
        let module = module(&format!("proc p {{}} {{set result [{body}]}}"));
        let info = detect_global_write_procs(&module);
        assert!(
            info[&summary_id(&module, "::p")].names.contains("hits"),
            "{body}"
        );
    }
}

#[test]
fn original_global_alias_writes_keep_empty_local_and_remote_names() {
    // experiments/name-handling/results/global-empty-alias-cell-20261014/oracles/observations.json
    for (body, target) in [
        ("upvar #0 {} local; set local VALUE", ""),
        ("upvar #0 target {}; set {} VALUE", "target"),
        ("upvar #0 {} {}; set {} VALUE", ""),
    ] {
        let module = module(&format!("proc p {{}} {{{body}}}"));
        let summaries = detect_global_write_procs(&module);
        assert!(
            summaries[&summary_id(&module, "::p")]
                .names
                .contains(target),
            "{body}"
        );
    }
    let module = module("proc p {name} {upvar #0 {} $name; set ordinary VALUE}");
    let summaries = detect_global_write_procs(&module);
    assert!(summaries[&summary_id(&module, "::p")].names.contains(""));
}

#[test]
fn standalone_frame_write_advice_keeps_unprofiled_catalogue_separate_from_missing_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Explicit catalogue advice proves no actual source input, frame or write.
    let registry = CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let module = crate::lowering::lower_to_ir("set ::standalone VALUE", &registry);
    let bindings = ModuleCommandBindings::analyse(&module, &registry);
    let procedures = HashMap::new();
    let compatible = own_frame_global_writes_standalone(
        &module.top_level,
        &registry,
        &bindings,
        "::",
        &procedures,
        &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
        module.lexer_config,
    );
    assert!(compatible.names.contains("::standalone"));
    assert!(compatible.names.contains("standalone"));
    let missing = own_frame_global_writes_with_metadata_context(
        &module.top_level,
        &registry,
        &bindings,
        "::",
        &procedures,
        &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
        (None, module.lexer_config),
    );
    assert!(missing.opaque_global_frame);
    assert!(missing.names.is_empty());
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let refused = own_frame_global_writes_with_metadata_context(
        &module.top_level,
        &registry,
        &bindings,
        "::",
        &procedures,
        &super::super::OriginalProcedureSummaryOwners::for_module(&module, &registry),
        (Some(foreign.as_ref().into()), module.lexer_config),
    );
    assert!(refused.opaque_global_frame);
    assert!(refused.names.is_empty());
}

#[test]
fn supplied_global_write_metadata_keeps_alias_arguments_and_literal_names() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source summaries establish no Native frame/link/current cell.
    let registry = CommandRegistry::build_default();
    for (source, expected) in [
        (
            "interp alias {} put {} set {$scalar(open}; proc p {} {global {$scalar(open}; put VALUE}",
            "$scalar(open",
        ),
        (
            "rename set moved; interp alias {} put {} moved {café}; proc p {} {global café; set result [put VALUE]}",
            "café",
        ),
        (
            "interp alias {} bind {} upvar #0 {$outer}; proc p {} {bind {$local}; set {$local} VALUE}",
            "$outer",
        ),
        (
            "proc p {} {uplevel #0 {set {scalar(open} VALUE}}",
            "scalar(open",
        ),
        (
            "interp alias {} at-root {} uplevel #0; proc p {} {at-root {set {café(open} VALUE}}",
            "café(open",
        ),
    ] {
        let module = module_with_registry(source, &registry);
        let information = detect_global_write_procs_with_registry(&module, &registry);
        assert!(
            information[&summary_id(&module, "::p")]
                .names
                .contains(expected),
            "{source}: {information:?}"
        );
    }
    for source in [
        "proc set args {}; interp alias {} put {} set hidden; proc p {} {put VALUE}",
        "interp alias {} put {} set hidden; rename set moved; proc set args {}; proc p {} {put VALUE}",
    ] {
        let module = module_with_registry(source, &registry);
        assert!(
            !detect_global_write_procs_with_registry(&module, &registry)
                [&summary_id(&module, "::p")]
                .names
                .contains("hidden")
        );
    }
}

#[test]
fn supplied_global_write_summary_refuses_missing_foreign_and_stale_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let module = module_with_registry("proc p {} {uplevel #0 {set g VALUE}}", &registry);
    assert!(
        detect_global_write_procs_with_registry(&module, &registry)[&summary_id(&module, "::p")]
            .names
            .contains("g")
    );
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    assert!(
        detect_global_write_procs_with_registry(&missing, &registry)[&summary_id(&missing, "::p")]
            .opaque_global_frame
    );
    let mut foreign = module.clone();
    let input = module.source_metadata_input.as_ref().unwrap();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        input.lexer_config(),
    ));
    assert!(
        detect_global_write_procs_with_registry(&foreign, &registry)[&summary_id(&foreign, "::p")]
            .opaque_global_frame
    );
    let mut stale = module;
    stale.lexer_config.expand_syntax = !stale.lexer_config.expand_syntax;
    assert!(
        detect_global_write_procs_with_registry(&stale, &registry)[&summary_id(&stale, "::p")]
            .opaque_global_frame
    );
}

#[test]
fn supplied_script_write_projection_uses_current_roles_instead_of_call_defs() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "gated-write",
        surface: registry.get("dict").unwrap().surface,
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    });
    let module = module_with_registry("gated-write {$scalar(open} VALUE", &registry);
    let aliases = ModuleCommandBindings::analyse(&module, &registry);
    for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
        let context = tcl_registry::model::ingress::static_context_for(environment)
            .with_command_store(registry.snapshot().shared_registry());
        let projection = script_value_write_projection_with_metadata_context(
            &module.top_level,
            &registry,
            &aliases,
            "::",
            Some((&context).into()),
            module.lexer_config,
        );
        assert_eq!(
            projection
                .literal_names
                .contains(&"$scalar(open".to_owned()),
            available,
            "{environment}: {projection:?}"
        );
        if !available {
            assert!(projection.opaque_variable_frame);
        }
    }
    assert!(
        script_value_write_projection_with_metadata_context(
            &module.top_level,
            &registry,
            &aliases,
            "::",
            None,
            module.lexer_config
        )
        .opaque_variable_frame
    );
}

#[test]
fn supplied_global_write_call_closure_includes_source_expression_targets() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let module = module(
        "proc mutate {} {global g; set g VALUE; return 0}; proc p {} {return [expr {[mutate]}]}",
    );
    assert!(
        detect_global_write_procs(&module)[&summary_id(&module, "::p")]
            .names
            .contains("g")
    );
}

#[test]
fn empty_module_has_no_writes() {
    let m = module("");
    assert!(detect_global_write_procs(&m).is_empty());
}

/// Equal leaf names retain independent body summaries.
#[test]
fn same_tailed_bodies_retain_independent_write_summaries() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = module(
        "namespace eval ::b {}; namespace eval ::a {}\n\
             proc ::b::run {} { global bbb; set bbb 1 }\n\
             proc ::a::run {} { global aaa; set aaa 1 }\n",
    );
    let info = detect_global_write_procs(&m);
    let a = summary_id(&m, "::a::run");
    let b = summary_id(&m, "::b::run");
    assert_ne!(a, b);
    assert_eq!(info[&a].names, BTreeSet::from(["aaa".to_owned()]));
    assert_eq!(info[&b].names, BTreeSet::from(["bbb".to_owned()]));
    let union: BTreeSet<_> = info[&a].names.union(&info[&b].names).cloned().collect();
    assert_eq!(union, BTreeSet::from(["aaa".to_owned(), "bbb".to_owned()]));
}

#[test]
fn proc_with_no_global_write_is_empty() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = module("proc ::p {} { set x 1 }");
    let info = detect_global_write_procs(&m);
    assert!(info.get(&summary_id(&m, "::p")).unwrap().is_empty());
}

#[test]
fn direct_global_write_recorded() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The `mutate` repro: global x; set x 99.
    let m = module("proc ::mutate {} { global x; set x 99 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::mutate"))
            .unwrap()
            .names
            .contains("x")
    );
    // The fixture selector accepts either display for this single body.
    assert!(
        info.get(&summary_id(&m, "mutate"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn variable_namespace_write_recorded() {
    let m = module("namespace eval ::ns {}\nproc ::ns::p {} { variable v; set v 1 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::ns::p"))
            .unwrap()
            .names
            .contains("v")
    );
}

#[test]
fn upvar_hash_zero_counts_as_global() {
    let m = module("proc ::p {} { upvar #0 g local; set local 1 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("g")
    );
    // Not the local alias name — recording "local" would be a no-op at
    // the caller (no such caller-frame variable), missing the real "g".
    assert!(
        !info
            .get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("local")
    );
}

#[test]
fn namespace_upvar_rename_resolves_to_outer_name() {
    let m = module(
        "namespace eval ::ns {}\nproc ::ns::p {} { namespace upvar ::ns nsvar localvar\nset localvar 1 }",
    );
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::ns::p"))
            .unwrap()
            .names
            .contains("nsvar")
    );
    assert!(
        !info
            .get(&summary_id(&m, "::ns::p"))
            .unwrap()
            .names
            .contains("localvar")
    );
}

#[test]
fn branch_alternative_outer_aliases_are_unioned() {
    let m = module("proc ::p {c} { if {$c} { upvar #0 a x } else { upvar #0 b x }; set x 1 }");
    let info = detect_global_write_procs(&m);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["a".to_owned(), "b".to_owned()])
    );
}

#[test]
fn dynamic_outer_alias_target_widens_the_global_frame() {
    let m = module("proc ::p {name} { upvar #0 $name x; set x 1 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info[&summary_id(&m, "::p")].opaque_global_frame,
        "got {:?}",
        info[&summary_id(&m, "::p")]
    );
}

#[test]
fn dynamic_namespace_alias_target_widens_the_global_frame() {
    let m = module("proc ::p {ns} { namespace upvar $ns x local; set local 1 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info[&summary_id(&m, "::p")].opaque_global_frame,
        "got {:?}",
        info[&summary_id(&m, "::p")]
    );
}

#[test]
fn caller_frame_upvar_is_not_a_global_write() {
    // `upvar 1` aliases the *caller* frame, not global/namespace —
    // must not be treated as an outer-scope write.
    let m = module("proc ::p {} { upvar 1 caller_x local; set local 1 }");
    let info = detect_global_write_procs(&m);
    assert!(info.get(&summary_id(&m, "::p")).unwrap().is_empty());
}

#[test]
fn read_only_global_is_not_a_write() {
    let m = module("proc ::p {} { global x; puts $x }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p")).unwrap().is_empty(),
        "read-only declaration produced effects: {:?}",
        info.get(&summary_id(&m, "::p")).unwrap()
    );
}

#[test]
fn renamed_global_descriptor_establishes_alias_without_counting_declaration_as_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "rename global declare_global\n\
             proc ::p {} { declare_global x; set x 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn renamed_trace_descriptor_is_not_mistaken_for_a_value_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "rename trace observe\n\
             proc ::p {} { variable x; observe add variable x write callback }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert!(info[&summary_id(&m, "::p")].is_empty());
}

#[test]
fn custom_replacement_under_builtin_spelling_uses_its_own_descriptor() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "global",
        arg_roles: &[(0, ArgRole::VarWrite)],
        state_transitions: tcl_registry::knowledge::Knowledge::Partial(
            StateTransitionDescriptor::EMPTY,
        ),
        world_effects: tcl_registry::knowledge::Knowledge::Partial(
            tcl_registry::WorldEffectDescriptor::VARIABLE_WRITE,
        ),
        native_compilation: Some(tcl_registry::native_compilation::NativeCompilationSpec {
            grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
            operation: tcl_registry::SemanticOperationId::Invoke,
            body: tcl_registry::native_compilation::NativeBodyCompilation::Direct,
        }),
        successful_handler: Some(
            tcl_registry::native_compilation::SuccessfulHandlerSpec::VariableOperands,
        ),
        ..CommandSpec::DEFAULT
    });
    let m = module_with_registry("proc ::p {} { variable x; global x value }", &registry);
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn variable_declaration_without_value_is_not_a_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry("proc ::p {} { variable x }", &registry);
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert!(info[&summary_id(&m, "::p")].is_empty());
}

#[test]
fn variable_value_form_records_the_namespace_value_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry("proc ::p {} { variable x 1 }", &registry);
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn variable_mixed_pairs_only_record_value_bearing_cells() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry("proc ::p {} { variable x 1 y }", &registry);
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_baked_level_is_part_of_exact_transition_resolution() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} upvar #0\n\
             proc ::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_defined_after_the_procedure_is_live_when_the_body_runs() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {} { g x local; set local 1 }\n\
             interp alias {} g {} upvar #0",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_established_inside_the_procedure_applies_to_later_calls() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {} { interp alias {} g {} upvar #0; g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_established_in_evaluated_substitution_applies_to_later_calls() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {} {\
                 set ignored [interp alias {} g {} upvar #0]; \
                 g x local; \
                 set local 1\
             }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn late_namespaced_alias_resolves_inside_namespace_eval_block() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "namespace eval ::n {}\n\
             proc ::n::p {} { eval { g x local; set local 1 } }\n\
             interp alias {} ::n::g {} upvar #0",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::n::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_established_by_another_callable_procedure_is_in_the_may_state() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { install; g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_created_in_namespaced_proc_is_global_in_source_interpreter() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "namespace eval ::n {}\n\
             proc ::n::install {} { interp alias {} g {} upvar #0 }\n\
             proc ::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn dynamic_command_binding_transition_widens_the_global_frame_summary() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {from to} { rename $from $to; unknown_target }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn canonical_alias_target_participates_in_transitive_write_closure() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::mutate {} { global x; set x 99 }\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::outer")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn late_alias_summary_is_published_under_its_source_spelling() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "call_mutate")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert_eq!(
        info[&summary_id(&m, "::outer")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_global_target_survives_same_tail_namespace_collision() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::mutate {} { global x; set x 99 }\n\
             namespace eval ::z {}\n\
             proc ::z::mutate {} {}\n\
             interp alias {} call_mutate {} mutate\n\
             proc ::outer {} { call_mutate }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "call_mutate")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert_eq!(
        info[&summary_id(&m, "::outer")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn rename_preserves_every_module_live_alias_state_with_its_baked_arguments() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} upvar #0\n\
             rename g h\n\
             proc ::p {} { h x local; set local 1 }\n\
             proc ::old_name {} { g y local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
    // Summaries deliberately use the module-wide may-state: `g` was a live
    // binding at one executable point, so a stored body calling it remains
    // conservative even though this particular definition follows the
    // rename. Lifetime-sensitive procedure reachability is not encoded in
    // the summary key.
    assert_eq!(
        info[&summary_id(&m, "::old_name")].names,
        BTreeSet::from(["y".to_owned()])
    );
}

#[test]
fn alias_chain_composes_every_baked_argument_in_order() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} bind_global {} upvar #0\n\
             interp alias {} bind_x {} bind_global x\n\
             interp alias {} bind_local {} bind_x\n\
             proc ::p {} { bind_local local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn embedded_global_alias_transition_maps_later_local_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {} { set _ [upvar #0 g local]; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["g".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn embedded_alias_prefix_preserves_global_alias_arguments() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} bind {} upvar #0 g\n\
             proc ::p {} { set _ [bind local]; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["g".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn embedded_namespace_alias_transition_maps_later_local_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::p {} { set _ [namespace upvar ::target x local]; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn alias_target_prefix_resolves_in_the_target_interpreters_global_namespace() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} upvar #0\n\
             namespace eval ::n {}\n\
             proc ::n::upvar args {}\n\
             proc ::n::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::n::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn conditional_alias_override_joins_both_live_bindings() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "set replace [llength $argv]\n\
             interp alias {} g {} upvar #0\n\
             if {$replace} { interp alias {} g {} puts }\n\
             proc ::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn conditional_namespace_alias_keeps_global_fallback_live() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} upvar #0\n\
             namespace eval ::n {}\n\
             proc ::n::install {replace} {\n\
                 if {$replace} { interp alias {} ::n::g {} puts }\n\
             }\n\
             proc ::n::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::n::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn non_mutating_alias_alternative_preserves_the_incoming_binding() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "set replace [llength $argv]\n\
             interp alias {} g {} upvar #0\n\
             interp alias {} mutate {} list\n\
             if {$replace} { interp alias {} mutate {} rename }\n\
             mutate g {}\n\
             proc ::p {} { g x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_baked_namespace_upvar_prefix_is_part_of_exact_transition_resolution() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} bind_ns {} namespace upvar ::target\n\
             proc ::p {} { bind_ns x local; set local 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_baked_variable_name_still_records_value_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} initialise {} variable x\n\
             proc ::p {} { initialise 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_baked_set_target_is_a_global_value_write() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} put {} set x\n\
             proc ::p {} { global x; put 1 }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn alias_baked_custom_varwrite_target_uses_its_registry_descriptor() {
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "write-cell",
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    });
    let m = module_with_registry(
        "interp alias {} write_x {} write-cell x\n\
             proc ::p {} { global x; write_x value }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn write_on_conditional_branch_still_counted() {
    // Sound over-approximation: flow-insensitive, so a write inside
    // an `if` still counts even though it may not always execute.
    let m = module("proc ::p {} { global x\nif {$c} { set x 1 } }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn declaration_after_write_still_counted() {
    // Flow-insensitive union: order within the body doesn't matter.
    let m = module("proc ::p {} { set x 1\nglobal x }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn logical_nested_write_projection_requires_original_children_not_literal_brackets() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    for (source, expected) in [
        ("proc p {} {set y [incr x]}", true),
        ("proc p {} {set y {[incr x]}}", false),
    ] {
        let module = module_in(source, &registry, "tcl");
        let bindings = ModuleCommandBindings::analyse(&module, &registry);
        let semantics = GlobalWriteSemantics::from_module(&module, &registry, &bindings).unwrap();
        let procedure = &module.procedures[&summary_id(&module, "::p")];
        let statement = procedure.body.statements.first().unwrap();
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let (writes, _) = original_nested_footprint(tokens, &semantics);
        assert_eq!(
            writes.literal_names.contains(&"x".to_owned()),
            expected,
            "{source}"
        );
        assert!(!writes.opaque_variable_frame, "{source}");
        let mut changed = tokens.clone();
        changed.argv_texts[2] = "[incr foreign]".to_owned();
        let (rejected, _) = original_nested_footprint(&changed, &semantics);
        assert!(rejected.opaque_variable_frame, "{source}");
        assert!(rejected.literal_names.is_empty(), "{source}");
    }
}

#[test]
fn embedded_incr_in_command_substitution_recorded() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = module("proc ::p {} { global x\nset y [incr x] }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn expanded_global_write_target_widens_the_global_frame() {
    let m = module("proc ::p {names} { global {*}$names; set x 1 }");
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn embedded_proc_call_in_assign_value_propagates_global_write() {
    let m = module(
        "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { set result \"value=[mutate]\" }",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::outer")].names.contains("x"));
}

/// #1797. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 set `::g` to `1`
/// through `caller`: the list-built script defines `::n::writer`, and its
/// recovered body's summary joins the call precisely.
#[test]
fn a_recovered_procedure_body_has_a_precise_global_summary() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment(release).analyser_profile(),
        );
        let m = module_with_registry(
            "namespace eval ::n [list proc writer {} {global g; set g 1}]\n\
                 proc ::caller {} {::n::writer}",
            &registry,
        );
        let info = detect_global_write_procs_with_registry(&m, &registry);
        let caller = &info[&summary_id(&m, "::caller")];
        assert!(!caller.opaque_global_frame, "{release}");
        assert!(caller.names.contains("g"), "{release}");
    }
}

/// #1797. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: after
/// `rename p old` and a redefinition of `p`, `caller` sets `::g` through
/// the new generation and `caller_old` sets nothing through the old one.
#[test]
fn each_call_joins_only_its_own_global_write_generation() {
    let m = module(
        "proc p {} {}\n\
             rename p old\n\
             proc p {} {global g; set g 1}\n\
             proc caller {} {p}\n\
             proc caller_old {} {old}",
    );
    let info = detect_global_write_procs(&m);
    let caller = &info[&summary_id(&m, "::caller")];
    assert!(!caller.opaque_global_frame);
    assert!(caller.names.contains("g"));
    let caller_old = &info[&summary_id(&m, "::caller_old")];
    assert!(!caller_old.opaque_global_frame);
    assert!(caller_old.names.is_empty());
}

#[test]
fn embedded_proc_call_in_return_propagates_global_write() {
    let m = module(
        "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::outer {} { return [mutate] }",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::outer")].names.contains("x"));
}

#[test]
fn embedded_proc_calls_in_assign_expr_and_expr_eval_propagate_global_write() {
    let m = module(
        "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::assign_expr {} { set result [expr {[mutate] + 1}] }\n\
             proc ::expr_eval {} { expr {[mutate] + 1} }",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::assign_expr")].names.contains("x"));
    assert!(info[&summary_id(&m, "::expr_eval")].names.contains("x"));
}

#[test]
fn embedded_proc_calls_in_if_and_while_conditions_propagate_global_write() {
    let m = module(
        "proc ::mutate {} { global x; set x 99; return 0 }\n\
             proc ::conditional {} { if {[mutate]} { return } }\n\
             proc ::loop {} { while {[mutate]} { break } }",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::conditional")].names.contains("x"));
    assert!(info[&summary_id(&m, "::loop")].names.contains("x"));
}

#[test]
fn dynamic_eval_body_widens_later_command_binding_resolution() {
    // The body can install `late` before the following call executes:
    // `outer {interp alias {} late {} mutate}`. Its text is dynamic in
    // the stored procedure, so the registry's dynamic-body fact and the
    // lowered Barrier must conservatively widen the command table.
    let m = module(
        "proc ::mutate {} { global x; set x 99 }\n\
             proc ::outer {body} { eval $body; late }\n\
             outer {interp alias {} late {} mutate}",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::outer")].opaque_global_frame);
}

#[test]
fn transitive_closure_through_direct_call() {
    // `outer` calls `mutate`, which writes global x — `outer`'s
    // summary must include `x` too (transitive).
    let m = module("proc ::mutate {} { global x; set x 99 }\nproc ::outer {} { mutate }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::outer"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn recursive_proc_terminates_and_includes_write() {
    let m =
        module("proc ::p {n} { if {$n == 0} { return }\n global x\n incr x\n p [expr {$n-1}] }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn mutually_recursive_procs_share_the_write() {
    let m = module("proc ::a {} { global x; set x 1; b }\nproc ::b {} { a }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::a"))
            .unwrap()
            .names
            .contains("x")
    );
    assert!(
        info.get(&summary_id(&m, "::b"))
            .unwrap()
            .names
            .contains("x")
    );
}

#[test]
fn uplevel_hash_zero_literal_body_records_global_writes() {
    // `uplevel #0 {set x 99}` needs no `global`
    // declaration at all (tclsh 9.0.3/9.0.4: the caller-visible `x`
    // really is 99 afterwards).
    let m = module("proc ::setter {} { uplevel #0 { set x 99 } }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::setter"))
            .unwrap()
            .names
            .contains("x")
    );
    assert!(
        !info
            .get(&summary_id(&m, "::setter"))
            .unwrap()
            .opaque_global_frame
    );
}

/// A global-frame script that links a name to a global writes that
/// global: tclsh 8.4 to 9.1 print `1 1` for
/// `proc p {} {uplevel #0 {upvar 0 g local; set local 1}}; set g 0; p;
/// puts -nonewline "$g "; proc q {} {uplevel #0 {upvar #0 h local2; set
/// local2 1}}; set h 0; q; puts $h`.
#[test]
fn a_global_frame_script_writes_the_global_its_own_alias_names() {
    for (body, global) in [
        ("upvar 0 g local; set local 1", "g"),
        ("::upvar 0 g local; ::set local 1", "g"),
        ("upvar #0 h local2; set local2 1", "h"),
    ] {
        let m = module(&format!("proc ::p {{}} {{ uplevel #0 {{ {body} }} }}"));
        let info = detect_global_write_procs(&m);
        assert!(
            info[&summary_id(&m, "::p")].names.contains(global),
            "{body}: {info:?}"
        );
    }
}

#[test]
fn uplevel_hash_zero_literal_body_resolves_late_alias_prefix_write() {
    // The proc body is lowered before the later alias exists, so `put`
    // carries no lower-time def.  Tcl 9.0.4 runs the alias prefix as
    // `set x 99` in frame #0; the closed module binding projection must
    // therefore attribute the global `x` write to `setter`.
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::setter {} { uplevel #0 { put 99 } }\n\
             interp alias {} put {} set x",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::setter")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
}

#[test]
fn alias_baked_global_literal_body_resolves_late_inner_alias() {
    // `g` is itself installed after the procedure body was lowered, so
    // this takes the plain-call literal-body path rather than UpFrame.
    // Its inner `put` alias is later still and must remain visible.
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::setter {} { g { put 99 } }\n\
             interp alias {} g {} uplevel #0\n\
             interp alias {} put {} set x",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::setter")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
}

#[test]
fn nested_relative_zero_inside_global_upframe_stays_global() {
    // The inner `uplevel 0` inherits the outer `#0` frame rather than
    // returning to the defining procedure's frame.
    let m = module("proc ::setter {} { uplevel #0 { uplevel 0 { ::set g 1 } } }");
    let info = detect_global_write_procs(&m);
    assert!(
        info[&summary_id(&m, "::setter")].names.contains("g"),
        "got {info:?}"
    );
    assert!(
        !info[&summary_id(&m, "::setter")].opaque_global_frame,
        "got {info:?}"
    );
}

#[test]
fn late_alias_to_relative_zero_inside_global_frame_stays_global() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} uplevel 0",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::setter")].names,
        BTreeSet::from(["g".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
}

#[test]
fn late_alias_to_eval_inside_global_frame_stays_global() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::setter {} {uplevel #0 {::inner {::set g 1}}}\n\
             interp alias {} ::inner {} eval",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::setter")].names,
        BTreeSet::from(["g".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::setter")].opaque_global_frame);
}

#[test]
fn transparent_eval_inside_global_upframe_uses_global_command_resolution() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "proc ::helper {} { global global_hit; set global_hit 1 }\n\
             namespace eval ::n {\n\
                 proc helper {} { global namespaced_hit; set namespaced_hit 1 }\n\
                 proc setter {} { uplevel #0 { eval { helper } } }\n\
             }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert!(
        info[&summary_id(&m, "::n::setter")]
            .names
            .contains("global_hit")
    );
    assert!(
        !info[&summary_id(&m, "::n::setter")]
            .names
            .contains("namespaced_hit"),
        "a transparent block must not restore its lowering-time namespace: {info:?}"
    );
    assert!(!info[&summary_id(&m, "::n::setter")].opaque_global_frame);
}

#[test]
fn transparent_eval_inside_global_upframe_resolves_a_late_global_alias() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "namespace eval ::n {\n\
                 proc put args {}\n\
                 proc setter {} { uplevel #0 { eval { put 2 } } }\n\
             }\n\
             interp alias {} put {} set x",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::n::setter")].names,
        BTreeSet::from(["x".to_owned()]),
        "a transparent Block inherits the selected global command namespace"
    );
    assert!(!info[&summary_id(&m, "::n::setter")].opaque_global_frame);
}

#[test]
fn uplevel_hash_zero_constructed_list_records_global_writes() {
    // `uplevel #0 [list set k 77]` — tclsh 9.0.4: the global `k` is 77.
    let m = module("proc ::s3 {} { uplevel #0 [list set k 77] }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::s3"))
            .unwrap()
            .names
            .contains("k")
    );
    assert!(
        !info
            .get(&summary_id(&m, "::s3"))
            .unwrap()
            .opaque_global_frame
    );
}

#[test]
fn constructed_global_body_with_dynamic_operand_is_opaque() {
    let m = module("proc ::p {name} { uplevel #0 [list set $name 1] }");
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
    assert!(info[&summary_id(&m, "::p")].names.is_empty());
}

#[test]
fn rebound_list_constructor_widens_global_frame_effects() {
    let m = module(
        "proc list args { return {set hidden 1} }\n\
             proc ::p {} { uplevel #0 [list set claimed 1] }",
    );
    let info = detect_global_write_procs(&m);
    assert!(info[&summary_id(&m, "::p")].opaque_global_frame);
    assert!(info[&summary_id(&m, "::p")].names.is_empty());
}

#[test]
fn alias_baked_global_uplevel_frame_is_resolved_from_invocation_facts() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} uplevel #0\n\
             proc ::p {} { g {set x 1} }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
    assert!(!info[&summary_id(&m, "::p")].opaque_global_frame);
}

#[test]
fn constructed_global_uplevel_resolves_command_in_global_namespace() {
    let registry = CommandRegistry::build_default();
    let m = module_with_registry(
        "interp alias {} g {} set\n\
             interp alias {} ::n::g {} list\n\
             proc ::n::p {} { uplevel #0 [list g x 1] }",
        &registry,
    );
    let info = detect_global_write_procs_with_registry(&m, &registry);
    assert_eq!(
        info[&summary_id(&m, "::n::p")].names,
        BTreeSet::from(["x".to_owned()])
    );
}

#[test]
fn uplevel_hash_zero_dynamic_script_is_opaque() {
    // `uplevel #0 $body` — any global may be written or read.
    let m = module("proc ::s2 {body} { uplevel #0 $body }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::s2"))
            .unwrap()
            .opaque_global_frame
    );
    assert!(!info.get(&summary_id(&m, "::s2")).unwrap().is_empty());
}

#[test]
fn uplevel_hash_zero_opaqueness_is_transitive() {
    let m = module("proc ::s2 {body} { uplevel #0 $body }\nproc ::outer {body} { s2 $body }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::outer"))
            .unwrap()
            .opaque_global_frame
    );
}

#[test]
fn uplevel_hash_zero_non_writing_script_stays_empty() {
    // TN — a global-frame script that writes nothing (`uplevel #0
    // [list puts hi]`) contributes neither names nor opaqueness.
    let m = module("proc ::shout {} { uplevel #0 [list puts hi] }");
    let info = detect_global_write_procs(&m);
    assert!(info.get(&summary_id(&m, "::shout")).unwrap().is_empty());
}

#[test]
fn uplevel_caller_frame_is_not_a_global_write() {
    // An absolute value-write command proves the body only writes the
    // caller's frame, which is `upvar_info`'s business. A relative `set`
    // could be shadowed in the caller namespace by an arbitrary global-
    // writing procedure and must therefore widen this summary.
    let m = module("proc ::p {} { uplevel 1 { ::set n 1 } }");
    let info = detect_global_write_procs(&m);
    let row = info.get(&summary_id(&m, "::p")).unwrap();
    assert!(row.is_empty(), "{row:?}");
}

#[test]
fn uplevel_hash_zero_dynamic_write_target_in_literal_body_is_opaque() {
    // `uplevel #0 {set $n 1}` — the body is readable but the written
    // name is not.
    let m = module("proc ::p {n} { uplevel #0 \"set $n 1\" }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .opaque_global_frame
    );
}

#[test]
fn unrelated_global_write_is_not_confused_with_other_procs() {
    let m = module("proc ::mutate_y {} { global y; set y 1 }\nproc ::p {} { global x; set x 1 }");
    let info = detect_global_write_procs(&m);
    assert!(
        info.get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("x")
    );
    assert!(
        !info
            .get(&summary_id(&m, "::p"))
            .unwrap()
            .names
            .contains("y")
    );
}
#[test]
fn global_body_summaries_use_issued_identity_and_preserve_current_header_truth() {
    let original = module("proc mutate {} {set ::x VALUE}");
    let id = summary_id(&original, "::mutate");
    let expected = detect_global_write_procs(&original);
    assert!(expected[&id].names.contains("x"));
    let mut labelled = original.clone();
    if let Some(procedure) = labelled.procedures.get_mut(&id) {
        procedure.qualified_name = "report-only-label".to_owned();
    } else {
        labelled.body_units.get_mut(&id).unwrap().qualified_name = "report-only-label".to_owned();
    }
    assert_eq!(detect_global_write_procs(&labelled), expected);
    let mut wrong_kind = original.clone();
    if let Some(procedure) = wrong_kind.procedures.get_mut(&id) {
        procedure.body_kind = crate::ir::ProcedureBodyKind::Event;
    } else {
        wrong_kind.body_units.get_mut(&id).unwrap().body_kind = crate::ir::ProcedureBodyKind::Event;
    }
    let changed = detect_global_write_procs(&wrong_kind);
    assert!(changed.get(&id).is_none_or(|info| info.opaque_global_frame));
}
