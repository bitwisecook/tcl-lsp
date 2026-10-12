// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original body namespaces, including constructed paths whose displays collide.

use super::*;
use crate::command_binding::{ExecutedScriptSource, SourceNamespaceKey, SourceOriginId};
use std::sync::Arc;

fn native_module(source: &str) -> (tcl_vm::Vm, Module) {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let profile = registry.profile().unwrap();
    let (runtime, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut lowerer = Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    lowerer.set_source_analysis_options(SourceAnalysisOptions {
        native_entry: Some(&entry),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        ..Default::default()
    });
    (runtime, lower_to_ir_with(lowerer, source))
}

#[test]
fn original_colon_procedure_body_keeps_definition_namespace() {
    // Native six-engine scope controls: C86+ :f is a simple command in N;
    // the reporting spelling ::N:::f cannot be split to recover its holder.
    let (_owner, module) =
        native_module("namespace eval N {proc :f {} {return [namespace current]}}");
    let original = module
        .procedure_implementation_bodies
        .iter()
        .find(|body| body.source.try_text() == Ok("return [namespace current]"))
        .expect("original installed declaration");
    assert_eq!(original.namespace, "::N");
    let procedure = module
        .procedures
        .values()
        .find(|procedure| procedure.body.executed_source.as_deref() == Some(&original.source))
        .expect("original procedure body lowered");
    assert_eq!(
        procedure.body.namespace_context.as_deref(),
        Some(&original.namespace_key)
    );
    assert_eq!(
        procedure
            .body
            .namespace_context
            .as_ref()
            .unwrap()
            .display()
            .as_deref(),
        Some("::N")
    );
}

#[test]
fn original_colliding_namespace_displays_keep_both_body_contexts() {
    let (_owner, module) = native_module(
        "namespace eval a: {namespace eval b {proc p {} {return FIRST}}}; \
         namespace eval a {namespace eval :b {proc p {} {return SECOND}}}",
    );
    let originals = ["return FIRST", "return SECOND"].map(|text| {
        module
            .procedure_implementation_bodies
            .iter()
            .find(|body| body.source.try_text() == Ok(text))
            .expect("original declaration survives name collision")
    });
    assert_eq!(
        originals[0].namespace_key.display(),
        originals[1].namespace_key.display()
    );
    assert_ne!(originals[0].namespace_key, originals[1].namespace_key);
    for original in originals {
        let procedure = module
            .procedures
            .values()
            .chain(module.body_units.values())
            .find(|procedure| {
                procedure.body.executed_source.as_deref() == Some(&original.source)
                    && procedure.body.namespace_context.as_deref() == Some(&original.namespace_key)
            })
            .expect("coverage retains the exact original body namespace");
        assert_eq!(
            procedure.body.namespace_context.as_deref(),
            Some(&original.namespace_key)
        );
    }
}

#[test]
fn original_body_refusal_preserves_key_and_unknown_never_invents_one() {
    let registry = CommandRegistry::build_default();
    let mut lowerer = Lowerer::new(&registry);
    let source = ExecutedScriptSource::contiguous(
        Arc::new(SourceOriginId::authored(&Arc::from("set x 1"))),
        "set x 1",
        0,
    )
    .unwrap();
    let namespace = SourceNamespaceKey::authored("::retained");
    let refused = lowerer.lower_original_body(source.clone(), Some(&namespace));
    assert_eq!(refused.namespace_context.as_deref(), Some(&namespace));
    assert_eq!(refused.executed_source.as_deref(), Some(&source));
    assert!(matches!(
        refused.statements.as_slice(),
        [Statement::Barrier { .. }]
    ));
    let unknown = lowerer.lower_original_body(source.clone(), None);
    assert!(unknown.namespace_context.is_none());
    assert_eq!(unknown.executed_source.as_deref(), Some(&source));
    assert!(matches!(
        unknown.statements.as_slice(),
        [Statement::Barrier { .. }]
    ));
}

#[test]
fn original_event_body_keeps_conditional_frame_without_claiming_entry() {
    let profile = tcl_dialect::DialectProfile::irules();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let source = "when HTTP_REQUEST {set debug 1; puts $debug}";
    let module = lower_to_ir_with(
        Lowerer::with_config(
            registry,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .with_dialect(Some(profile)),
        source,
    );
    let body = &module.procedures["::when::HTTP_REQUEST"].body;
    assert_eq!(body.statements.len(), 2);
    assert_eq!(
        body.namespace_context.as_deref(),
        Some(&SourceNamespaceKey::authored("::"))
    );
    assert_eq!(
        body.executed_source.as_ref().unwrap().try_text(),
        Ok("set debug 1; puts $debug")
    );
    assert!(module.source_entry.native_entry.is_none());
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    assert_eq!(
        bindings.executed_script_namespace_context(body.executed_source.as_deref().unwrap()),
        Some(SourceNamespaceKey::authored("::"))
    );
    assert!(!bindings.has_actual_procedure_entry(body.executed_source.as_deref().unwrap()));
}

#[test]
fn original_replaced_and_retired_declarations_keep_analysis_only_coverage() {
    let source = "proc p {} {return FIRST}; proc p {} {return SECOND}; \
        rename p {}; missing_command";
    let (_owner, mut module) = native_module(source);
    let originals = module.procedure_implementation_bodies.to_vec();
    assert_eq!(originals.len(), 2);
    assert_ne!(originals[0].allocation, originals[1].allocation);
    for original in &originals {
        assert!(
            module
                .procedures
                .values()
                .chain(module.body_units.values())
                .any(|unit| {
                    unit.body.executed_source.as_deref() == Some(&original.source)
                        && unit.body.namespace_context.as_deref() == Some(&original.namespace_key)
                }),
            "original body {}",
            original.source.try_text().unwrap()
        );
    }
    assert!(!module.original_declaration_body_units.is_empty());
    assert!(module.installed_procedure_body_units.is_empty());
    for (label, allocation) in &module.original_declaration_body_units {
        let unit = &module.body_units[label];
        assert!(originals.iter().any(|original| {
            &original.allocation == allocation
                && unit.body.executed_source.as_deref() == Some(&original.source)
                && unit.body.namespace_context.as_deref() == Some(&original.namespace_key)
        }));
        assert!(
            module
                .executable_script_roots()
                .iter()
                .all(|(body, _)| *body != &unit.body)
        );
    }
    let top_level = module.top_level.clone();
    let inventory = module.procedure_implementation_bodies.clone();
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    crate::specialise_factories::specialise_factories_with_cap(&mut module, owner.commands(), 0);
    assert!(module.original_declaration_body_units.is_empty());
    assert_eq!(module.top_level, top_level);
    assert_eq!(module.procedure_implementation_bodies, inventory);
}

#[test]
fn shared_original_body_keeps_distinct_declaration_sites() {
    let (_owner, module) = native_module(
        "set body {return SHARED}; proc p {} $body; proc p {} $body; \
         rename p {}; missing_command",
    );
    let originals = module.procedure_implementation_bodies.to_vec();
    assert_eq!(originals.len(), 2);
    assert_ne!(originals[0].allocation.site, originals[1].allocation.site);
    assert!(!module.original_declaration_body_units.is_empty());
    for original in &originals {
        let unit = module
            .original_declaration_body_units
            .iter()
            .find_map(|(label, allocation)| {
                (allocation == &original.allocation).then(|| &module.body_units[label])
            })
            .or_else(|| {
                module.procedures.values().find(|unit| {
                    unit.span.start() == original.allocation.site.offset
                        && unit.body.executed_source.as_deref() == Some(&original.source)
                        && unit.body.namespace_context.as_deref() == Some(&original.namespace_key)
                })
            })
            .expect("each original declaration site retains its own analysis unit");
        assert_eq!(unit.body.executed_source.as_deref(), Some(&original.source));
        assert_eq!(
            unit.body.namespace_context.as_deref(),
            Some(&original.namespace_key)
        );
        if module
            .original_declaration_body_units
            .values()
            .any(|allocation| allocation == &original.allocation)
        {
            assert!(
                module
                    .executable_script_roots()
                    .iter()
                    .all(|(body, _)| *body != &unit.body)
            );
        }
    }
}
