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
use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn provided_unit(
    source: &str,
    providers: &[crate::provider_fixtures::Provider],
) -> (CommandRegistry, CompilationUnit) {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let entry = crate::provider_fixtures::entry(&registry, providers);
    let context = std::sync::Arc::new(
        crate::environment_ingress::context_for_profile(registry.profile().unwrap())
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let unit = CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            dialect: registry.profile(),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        context,
    );
    (registry, unit)
}

#[test]
fn retained_source_script_assignments_share_the_original_constructor_seed() {
    let source = "oo::class create C {method m {} {}}\nproc build {} {set object [C new]; set inert {[C new]}}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    let (id, procedure) = unit
        .ir_module
        .original_source_procedure_bodies()
        .find(|(_, procedure)| procedure.name == "build")
        .unwrap();
    let owners = OwnerIndex::build(&unit, FactMode::Full);
    let mut sink = FactSink {
        facts: ObjectHandleFacts::default(),
        owners: &owners,
        metadata_context: crate::registry_invocation::InvocationMetadataContext::for_module(
            registry,
            &unit.ir_module,
        ),
        source_analysis: Some(&analysis),
        mode: FactMode::Full,
        saw_constructor_arg: false,
        stats: LatticeStats::default(),
    };
    harvest_script_factories(
        &procedure.body,
        Some(BodyFrameId::Body(*id)),
        registry,
        &mut sink,
    );
    assert!(
        sink.facts
            .any_scope
            .get("object")
            .is_some_and(|classes| classes.contains("::C"))
    );
    assert!(!sink.facts.any_scope.contains_key("inert"));
    assert_eq!(
        sink.facts.proven_reads,
        [] as [crate::object_types::ObjectHandleRead; 0]
    );
    assert_eq!(
        sink.facts.physical_type_reads,
        [] as [crate::object_types::PhysicalObjectTypeReadEvidence; 0]
    );
    assert!(sink.facts.normal_procedure_results.is_empty());
}

#[test]
fn logical_object_class_unions_require_current_analysis_and_module_inputs() {
    let source = "oo::class create C {method m {} {}}\nset object [C new]\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    let classes = logical_source_object_handle_classes(&unit, registry, &analysis).unwrap();
    assert!(
        classes
            .get("object")
            .is_some_and(|classes| classes.contains("::C"))
    );
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert!(logical_source_object_handle_classes(&missing, registry, &analysis).is_none());
    let mut changed = unit.clone();
    changed.source.push(' ');
    assert!(logical_source_object_handle_classes(&changed, registry, &analysis).is_none());
    let foreign = crate::analyser::Analyser::new().analyse("oo::class create D {}", "tcl");
    assert!(logical_source_object_handle_classes(&unit, registry, &foreign).is_none());
    let other = tcl_registry::model::ingress::static_context_for("jim");
    assert!(logical_source_object_handle_classes(&unit, other.commands(), &analysis).is_none());
}

#[test]
fn logical_constructor_candidates_require_the_actual_source_input_and_vector() {
    let source = "oo::class create C {method m {} {}}\nset object [C new]\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let context =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, input)
            .unwrap();
    let offset = source.find("C new").unwrap();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[offset..offset + "C new".len()],
        u32::try_from(offset).unwrap(),
        input.lexer_config(),
    );
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &commands[0],
    );
    assert_eq!(
        original_logical_constructor_class(&analysis, context, &tokens).as_deref(),
        Some("::C")
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(original_logical_constructor_class(&missing, context, &tokens).is_none());
    let mut changed = tokens.clone();
    let WordExpr::Literal { text, .. } = &mut changed.word_exprs[1] else {
        panic!("actual literal selector")
    };
    *text = "unrepresentedSelector".to_owned();
    assert!(original_logical_constructor_class(&analysis, context, &changed).is_none());
    let foreign = crate::analyser::Analyser::new()
        .analyse("oo::class create D {}\nset object [D new]", "tcl");
    assert!(original_logical_constructor_class(&foreign, context, &tokens).is_none());
}

#[test]
fn advisory_argument_edges_keep_original_substitution_shapes() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = r#"list $object "${two words}" {$object} {[C new]} prefix$object $object(key) ${object(key)}"#;
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &commands[0],
    );
    let ctor_params = HashMap::new();
    let out = HashMap::from([
        ("object".to_owned(), HashSet::from(["::C".to_owned()])),
        ("two words".to_owned(), HashSet::from(["::C".to_owned()])),
    ]);
    let module = crate::compilation_unit::CompilationUnit::build_for("", registry, false);
    let ctx = ScanContext {
        ctor_params: &ctor_params,
        out: &out,
        by_scope: None,
        owners: OwnerIndex::shared_empty(),
        registry,
        metadata_context: None,
        source_analysis: None,
        unit: BodyFrameId::Top(module.ir_module.procedure_ids.module_id()),
        tokens: Some(&tokens),
    };
    assert_eq!(deref_arg_var(&tokens.words()[1], ctx), Some("object"));
    assert_eq!(deref_arg_var(&tokens.words()[2], ctx), Some("two words"));
    assert!(arg_classes(&tokens.words()[1], ctx).is_some());
    assert!(arg_classes(&tokens.words()[2], ctx).is_some());
    for word in &tokens.words()[3..] {
        assert!(deref_arg_var(word, ctx).is_none(), "{word:?}");
        assert!(arg_classes(word, ctx).is_none(), "{word:?}");
    }
    assert!(sole_substitution_tokens(&tokens.words()[4], &tokens, registry).is_none());
}

#[test]
fn source_positioned_receiver_proofs_do_not_reuse_an_older_object_value() {
    let registry = release_registry();
    let source =
        "oo::class create Pin {}\nset x [Pin new]\n$x retained\nset x 0\nset y $x\n$y stale\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    let retained = u32::try_from(source.find("$x retained").unwrap()).unwrap();
    let stale = u32::try_from(source.find("$y stale").unwrap()).unwrap();
    assert_eq!(
        facts.classes_in_scope(retained, "x"),
        Some(&HashSet::from(["::Pin".to_owned()]))
    );
    assert!(facts.classes_in_scope(stale, "y").is_none(), "{facts:?}");
    let proof = facts
        .physical_type_reads
        .iter()
        .find(|proof| proof.source().span.start() == retained && proof.variable() == "x")
        .expect("the retained read has a closed physical address receipt");
    assert_eq!(proof.candidate_class(), "::Pin");
    assert!(proof.place().cell.is_some());
    assert_eq!(
        facts.physical_type_read_at(proof.frame(), proof.source(), proof.evaluation()),
        Some(proof)
    );
    let mut advisory = facts.clone();
    advisory
        .any_scope
        .insert("y".to_owned(), HashSet::from(["::Pin".to_owned()]));
    advisory.by_scope.insert(
        (
            ObjectFactOwner::Frame(BodyFrameId::Top(cu.ir_module.procedure_ids.module_id())),
            "y".to_owned(),
        ),
        HashSet::from(["::Pin".to_owned()]),
    );
    assert_eq!(advisory.physical_type_reads, facts.physical_type_reads);
    assert!(advisory.classes_in_scope(stale, "y").is_none());
}

#[test]
fn opaque_constructor_cannot_prove_the_original_concrete_class() {
    let registry = CommandRegistry::build_default();
    let source = "oo::class create Other {}\noo::class create C {constructor {} {oo::objdefine [self] class Other}}\nset object [C new]\n$object method\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    let offset = u32::try_from(source.find("$object method").unwrap()).unwrap();
    assert!(
        facts
            .classes_in_scope(offset, "object")
            .is_none_or(|classes| !classes.contains("::C")),
        "{facts:?}"
    );
    assert!(
        facts.physical_type_reads.iter().all(|proof| {
            proof.source().span.start() != offset || proof.candidate_class() != "::C"
        }),
        "constructor reclassification cannot donate a ::C type-read receipt"
    );
}

#[test]
fn same_cell_reclassification_does_not_prove_the_old_object_class() {
    let registry = CommandRegistry::build_default();
    let source = "oo::class create C {method valid {} {return C}}\noo::class create B {method valid {} {return B}}\nset object [C new]\noo::objdefine $object class B\n$object valid\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
    assert!(
        facts
            .classes_in_scope(offset, "object")
            .is_none_or(|classes| !classes.contains("::C")),
        "unchanged variable contents do not preserve the object's class: {facts:?}"
    );
}

#[test]
fn materialised_receiver_reads_do_not_enter_document_offset_queries() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "oo::class create C {method valid {} {return ok}}\nset object [C new]\nset body {$object valid}\neval $body\n";
    let unit = CompilationUnit::build_for(source, registry, false);
    let facts = object_handle_facts(&unit, registry);
    assert!(
        facts
            .proven_reads
            .iter()
            .all(|read| read.variable != "object"),
        "{facts:?}"
    );
    let original = source.replacen("set body", "$object valid\nset body", 1);
    let unit = CompilationUnit::build_for(&original, registry, false);
    let facts = object_handle_facts(&unit, registry);
    let offset = u32::try_from(original.find("$object valid").unwrap()).unwrap();
    assert!(
        facts.instance_in_scope(offset, "object").is_some(),
        "{facts:?}"
    );
}

#[test]
fn receiver_read_does_not_preserve_class_across_argument_mutation() {
    let registry = CommandRegistry::build_default();
    let source = "oo::class create C {method valid {arg} {return C}}\noo::class create B {method valid {arg} {return B}}\nproc mutate {object} {oo::objdefine $object class B; return ARG}\nset object [C new]\n$object valid [mutate $object]\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
    assert!(
        facts.classes_in_scope(offset, "object").is_none(),
        "a receiver read before argv is not current dispatch proof: {facts:?}"
    );
}

#[test]
fn physical_type_receipts_do_not_close_object_method_dispatch() {
    let registry = CommandRegistry::build_default();
    let source = "oo::class create C {method valid {} {return ORIGINAL}}\nset object [C new]\noo::objdefine $object method valid {} {return REPLACED}\n$object valid\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    assert!(
        facts
            .physical_type_reads
            .iter()
            .all(PhysicalObjectTypeReadEvidence::object_dispatch_unknown)
    );
}

#[test]
fn replaced_class_command_does_not_seed_factory_provenance() {
    let registry = CommandRegistry::build_default();
    let source = "oo::class create Pin {}\nrename Pin {}\nproc Pin {args} {return plain}\nset receiver [Pin new]\n";
    let cu = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    assert!(!facts.any_scope.contains_key("receiver"), "{facts:?}");
}

#[test]
fn bareword_widget_path_is_a_handle() {
    // `ttk::treeview .t` is syntactically identical to the tcllib
    // naming-factory shape (`struct::graph g`) `harvest_unit` already
    // reads generically via `creates_instance_at`/`object_class` — so a
    // Tk widget's bareword path becomes a tracked handle with zero new
    // code in this pass, once the registry declares those two fields.
    let src = "package require Tk\nttk::treeview .t\n.t instate {selected} {}\n";
    let (registry, cu) = provided_unit(src, &[crate::provider_fixtures::Provider::Tk]);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get(".t").map(|s| s.contains("ttk::treeview")),
        Some(true),
        "`.t` should be a tracked ttk::treeview handle; got {map:?}"
    );
}

#[test]
fn var_captured_widget_path_is_a_handle() {
    let src = "package require Tk\nset lb [listbox .l]\n$lb curselection\n";
    let (registry, cu) = provided_unit(src, &[crate::provider_fixtures::Provider::Tk]);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("lb").map(|s| s.contains("listbox")),
        Some(true),
        "`lb` should be a tracked listbox handle; got {map:?}"
    );
}

#[test]
fn unprovided_widget_and_container_catalogues_do_not_donate_handles() {
    let registry = CommandRegistry::build_default();
    for source in [
        "package require Tk\nset receiver [listbox .l]",
        "package require struct::tree\nset receiver [struct::tree]",
    ] {
        let unit = CompilationUnit::build_for(source, &registry, false);
        assert!(!object_handle_classes(&unit, &registry).contains_key("receiver"));
    }
}

#[test]
fn scalar_handle_from_constructor() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "oo::class create Chart {method Xaxis {args} {}}\nset chart [Chart new]\n$chart Xaxis -name x\n";
    let cu = CompilationUnit::build_for(src, registry, false);
    let map = object_handle_classes(&cu, registry);
    assert_eq!(
        map.get("chart").map(|s| s.contains("::Chart")),
        Some(true),
        "chart should retain its actual declared constructor candidate; got {map:?}"
    );
    let unprovided = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "set chart [ticklecharts::chart new]\n$chart Xaxis -name x",
        &unprovided,
        false,
    );
    assert!(!object_handle_classes(&unit, &unprovided).contains_key("chart"));
}

#[test]
fn registry_naming_factory_handles() {
    // A registry object-factory names its instance either positionally
    // (`struct::graph g`) or as a substitution result (`set g [struct::graph]`)
    // — both must be tracked as `struct::graph` handles for `$g walk …`
    // method-callback resolution.
    for (src, key) in [
        ("struct::graph myG\nmyG walk root -command cb\n", "myG"),
        ("set g [struct::graph]\n$g walk root -command cb\n", "g"),
        ("struct::tree myT\nmyT walkproc root cb\n", "myT"),
    ] {
        let source = format!("package require struct::graph\npackage require struct::tree\n{src}");
        let (registry, cu) = provided_unit(
            &source,
            &[
                crate::provider_fixtures::Provider::Graph,
                crate::provider_fixtures::Provider::Tree,
            ],
        );
        let map = object_handle_classes(&cu, &registry);
        let class = if src.contains("tree") {
            "struct::tree"
        } else {
            "struct::graph"
        };
        assert_eq!(
            map.get(key).map(|s| s.contains(class)),
            Some(true),
            "`{src}` should track {key} as a {class} handle; got {map:?}"
        );
    }
}

#[test]
fn normal_callable_return_types_do_not_require_a_tcloo_class() {
    use crate::provider_fixtures::Provider;
    for (prelude, providers, expected) in [
        (
            "",
            vec![Provider::Tree],
            Some(tcl_registry::TclType::Object),
        ),
        ("", vec![], None),
        (
            "rename ::struct::tree ::savedTree; proc ::struct::tree args {return ordinary}\n",
            vec![Provider::Tree],
            Some(tcl_registry::TclType::String),
        ),
    ] {
        let source = format!(
            "package require struct::tree\n{prelude}proc make {{}} {{return [struct::tree]}}\nmake\n"
        );
        let (registry, unit) = provided_unit(&source, &providers);
        let facts = object_handle_facts(&unit, &registry);
        let result = unit
            .ir_module
            .top_level
            .statements
            .last()
            .and_then(|statement| {
                unit.ir_module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
            .and_then(|target| facts.normal_procedure_result(target))
            .and_then(crate::types::TypeLattice::tcl_type);
        assert_eq!(
            result,
            expected,
            "{source}; units={:?}",
            unit.procedures
                .iter()
                .map(|(name, unit)| (name, &unit.return_type))
                .collect::<Vec<_>>()
        );
        assert!(
            !facts.returns_object.contains_key(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0
            )
        );
        assert_eq!(
            facts.proven_reads,
            [] as [crate::object_types::ObjectHandleRead; 0]
        );
    }
}

#[test]
fn normal_return_type_receipts_do_not_follow_a_reused_proc_name() {
    let source = "proc make {} {return 1}\nmake\nproc make {} {return ordinary}\nmake\n";
    let (registry, unit) = provided_unit(source, &[]);
    let facts = object_handle_facts(&unit, &registry);
    let calls = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .filter_map(|statement| {
            let tokens = unit
                .ir_module
                .top_level
                .retained_source_tokens_for_statement(statement)?;
            let target = tokens.source_binding.as_ref()?.proved_execution_target()?;
            (target.kind == crate::command_binding::BindingKind::Proc).then(|| {
                facts
                    .normal_procedure_result(target)
                    .and_then(crate::types::TypeLattice::tcl_type)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        [
            Some(tcl_registry::TclType::Int),
            Some(tcl_registry::TclType::String)
        ]
    );
}

#[test]
fn normal_naming_factory_candidates_require_the_actual_loaded_handler() {
    use crate::provider_fixtures::Provider;
    let create = "package require struct::graph\nstruct::graph g\n";
    let (registry, loaded) = provided_unit(create, &[Provider::Graph]);
    let candidates = object_handle_classes(&loaded, &registry);
    assert!(
        candidates
            .get("g")
            .is_some_and(|classes| classes.contains("struct::graph"))
    );
    // Nominal candidates do not fabricate physical variable/object reads.
    assert_eq!(
        object_handle_facts(&loaded, &registry).proven_reads,
        [] as [crate::object_types::ObjectHandleRead; 0]
    );

    let (registry, unprovided) = provided_unit(create, &[]);
    assert!(!object_handle_classes(&unprovided, &registry).contains_key("g"));

    let source = "package require struct::graph\nrename ::struct::graph ::savedGraph\nproc ::struct::graph args {return DECOY}\nstruct::graph g\n";
    let (registry, replaced) = provided_unit(source, &[Provider::Graph]);
    assert!(!object_handle_classes(&replaced, &registry).contains_key("g"));
}

#[test]
fn retained_procedure_factory_candidates_do_not_require_an_executable_unit() {
    use crate::provider_fixtures::Provider;
    for (prelude, providers, expected) in [
        ("", vec![Provider::Graph], true),
        ("", vec![], false),
        (
            "rename ::struct::graph ::savedGraph; proc ::struct::graph args {return DECOY}\n",
            vec![Provider::Graph],
            false,
        ),
    ] {
        let source = format!(
            "package require struct::graph\n{prelude}proc build {{}} {{struct::graph g; g node insert root}}\nbuild\n"
        );
        let (registry, unit) = provided_unit(&source, &providers);
        assert_eq!(
            object_handle_classes(&unit, &registry).contains_key("g"),
            expected,
            "{source}"
        );
        assert_eq!(
            object_handle_facts(&unit, &registry).proven_reads,
            [] as [crate::object_types::ObjectHandleRead; 0]
        );
    }
}

#[test]
fn registry_factory_operator_form_binds_nothing() {
    // `struct::graph = $serial` puts a deserialise *operator* (`=`) in the
    // `?name?` slot — it names no object command, so neither
    // `object_handle_classes` nor the analyser's `instance_classes` may bind
    // it (a bogus `=` handle would suppress real W123/W307 and mis-resolve a
    // command literally named `=`).
    let registry = CommandRegistry::build_default();
    for op in ["=", ":=", "as", "deserialize"] {
        let src = format!("struct::graph {op} $serial\n");
        let cu = CompilationUnit::build_for(&src, &registry, false);
        assert!(
            !object_handle_classes(&cu, &registry).contains_key(op),
            "`struct::graph {op}` must not track `{op}` as an object handle"
        );
        let r = crate::analyser::Analyser::new().analyse(&src, "tcl9.0");
        assert!(
            !r.instance_classes.contains_key(op),
            "`struct::graph {op}` must not bind `{op}` in instance_classes"
        );
    }
}

#[test]
fn non_constructor_assignment_is_not_a_handle() {
    let registry = CommandRegistry::build_default();
    let src = "set x [expr {1 + 2}]\nset y hello\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert!(
        map.is_empty(),
        "no object handles expected for non-constructor assignments; got {map:?}"
    );
}

#[test]
fn interproc_param_from_object_arg_is_a_handle() {
    // `set p [Pin new]; connect $p` — the object flows into `connect`'s
    // parameter `dev`, so `$dev method …` in the body resolves (the
    // param-receiver case the mro_eval experiment measured as 60% of ⊤).
    let registry = release_registry();
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc connect {dev} { $dev cfg }\n\
                   set p [Pin new]\n\
                   connect $p\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("dev").map(|s| s.contains("::Pin")),
        Some(true),
        "connect's param `dev` should be a ::Pin handle; got {map:?}"
    );
}

#[test]
fn interproc_param_flows_through_call_chain() {
    // `a $p` → `b $x` → `$y cfg`: the class flows two hops through the
    // fixpoint, so the innermost param `y` is a ::Pin handle.
    let registry = release_registry();
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   proc a {x} { b $x }\n\
                   proc b {y} { $y cfg }\n\
                   set p [Pin new]\n\
                   a $p\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("y").map(|s| s.contains("::Pin")),
        Some(true),
        "the class should flow a→b so `y` is a ::Pin handle; got {map:?}"
    );
}

#[test]
fn aliasing_copies_handle_class() {
    // `set a [Pin new]; set b $a` — the plain var-ref assignment aliases
    // `a`'s class onto `b`, so `$b method …` resolves (VTA assignment edge).
    let registry = release_registry();
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   set a [Pin new]\n\
                   set b $a\n\
                   $b cfg\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("b").map(|s| s.contains("::Pin")),
        Some(true),
        "`b` should alias `a`'s ::Pin class; got {map:?}"
    );
}

#[test]
fn child_constructor_arguments_do_not_require_the_parent_assignment_handler() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (parent, possible) in [
        ("outer [Wrap new $p]", true),
        ("if {0} {outer [Wrap new $p]}", false),
        ("outer [error STOP; Wrap new $p]", false),
    ] {
        let source = format!(
            "oo::class create Pin {{method cfg {{}} {{}}}}; oo::class create Wrap {{constructor {{inner}} {{$inner cfg}}}}; proc outer args {{return CUSTOM}}; set p [Pin new]; {parent}"
        );
        let cu = CompilationUnit::build_for_profile(&source, &registry, false, profile);
        let map = object_handle_classes(&cu, &registry);
        assert_eq!(
            map.get("inner")
                .is_some_and(|classes| classes.contains("::Pin")),
            possible,
            "{source}: {map:?}",
        );
    }
}

#[test]
fn constructor_param_typed_from_object_arg() {
    // Case B — an object passed *into* a constructor: `Wrap new $p` binds
    // the constructor's parameter `inner` to ::Pin, so `$inner method …`
    // inside the constructor body resolves (VTA constructor-param edge).
    let registry = release_registry();
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Wrap {\n\
                     variable held\n\
                     constructor {inner} { $inner cfg; set held $inner }\n\
                   }\n\
                   set p [Pin new]\n\
                   set w [Wrap new $p]\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("inner").map(|s| s.contains("::Pin")),
        Some(true),
        "constructor param `inner` should be a ::Pin handle; got {map:?}"
    );
    // …and the aliasing edge carries it onto the instance var it is stored
    // in, so a *different* method dispatching `$held m` resolves too.
    assert_eq!(
        map.get("held").map(|s| s.contains("::Pin")),
        Some(true),
        "instance var `held` should alias the ::Pin ctor param; got {map:?}"
    );
}

#[test]
fn instance_var_from_constructor_param_bridges_methods() {
    // The full Case-B shape: an object flows in through the constructor,
    // is stored in an instance variable, and is dispatched on from an
    // unrelated method.  ctor-param + aliasing edges together resolve it.
    let registry = release_registry();
    let src = "oo::class create Motor { method spin {args} {} }\n\
                   oo::class create Car {\n\
                     variable engine\n\
                     constructor {e} { set engine $e }\n\
                     method go {} { $engine spin }\n\
                   }\n\
                   set m [Motor new]\n\
                   set c [Car new $m]\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_handle_classes(&cu, &registry);
    assert_eq!(
        map.get("engine").map(|s| s.contains("::Motor")),
        Some(true),
        "instance var `engine` should be a ::Motor handle via ctor param; got {map:?}"
    );
}

/// A widget factory returns the path it creates: Tk 9.0.3 gives `.tree
/// Treeview` for `set t [ttk::treeview .tree]; list $t [winfo class $t]`.
/// The variable holding the result is a handle of the same class as the
/// path the bare form publishes.
#[test]
fn a_factory_result_held_in_a_variable_is_a_handle_of_the_published_class() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let class_of = |source: &str| {
        let cu = CompilationUnit::build_for(source, registry, false);
        let mut classes: Vec<_> = object_handle_classes(&cu, registry).into_iter().collect();
        classes.sort_by(|left, right| left.0.cmp(&right.0));
        classes
    };
    let treeview = HashSet::from(["ttk::treeview".to_owned()]);
    assert_eq!(
        class_of("package require Tk\nttk::treeview .tree\n.tree current\n"),
        [(".tree".to_owned(), treeview.clone())]
    );
    assert_eq!(
        class_of("package require Tk\nset t [ttk::treeview .tree]\n$t current\n"),
        [("t".to_owned(), treeview)]
    );
}

#[test]
fn snit_named_constructor_has_nominal_name_without_a_physical_class() {
    let source = "package require snit\nsnit::type foo { method smeth {} {} }\nset o [foo create x]\n$o smeth\n";
    let (registry, cu) = provided_unit(source, &[crate::provider_fixtures::Provider::Snit]);
    let facts = object_handle_facts(&cu, &registry);
    let read = u32::try_from(source.find("$o smeth").unwrap()).unwrap();
    assert!(facts.classes_in_scope(read, "o").is_none());
    assert!(facts.proven_reads.iter().all(|proof| proof.variable != "o"));
    let create = u32::try_from(source.find("foo create").unwrap()).unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let entry =
        crate::provider_fixtures::entry(&registry, &[crate::provider_fixtures::Provider::Snit]);
    let selected = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        entry.options(),
    );
    assert!(
        selected
            .invocation_at_source("::foo", create)
            .nominal_definition_name_result(&registry)
            .is_some()
    );
    let unprovided =
        crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
    assert!(
        unprovided
            .invocation_at_source("::foo", create)
            .nominal_definition_name_result(&registry)
            .is_none()
    );
}

#[test]
fn collection_of_objects_is_tracked() {
    let registry = CommandRegistry::build_default();
    let src = "oo::class create Pin {}\n\
                   dict set pins a [Pin new]\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_collection_classes(&cu);
    assert_eq!(
        map.get("pins").map(|s| s.contains("::Pin")),
        Some(true),
        "pins should be a collection of ::Pin; got {map:?}"
    );
}

#[test]
fn collection_class_bridges_across_methods() {
    // The interprocedural case: one method fills the `pins`
    // collection, a *different* method dispatches on an element.  The
    // cross-scope union makes `pins` a collection-of-Pin at both sites.
    let registry = CommandRegistry::build_default();
    let src = "oo::class create Pin { method cfg {args} {} }\n\
                   oo::class create Dev {\n\
                     variable pins\n\
                     method add {k} { ::dict set pins $k [::Pin new] }\n\
                     method use {k} { [::dict get $pins $k] cfg -node 1 }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_collection_classes(&cu);
    assert_eq!(
        map.get("pins").map(|s| s.contains("::Pin")),
        Some(true),
        "pins should be a collection of ::Pin, harvested from method add; got {map:?}"
    );
}

#[test]
fn spicegentcl_configurable_device_shape_resolves() {
    // The exact SpiceGenTcl shape: namespaced `oo::configurable` classes,
    // the collection built and dispatched in the *same* big method's switch
    // arms with fully-qualified constructors.  Locks in that an
    // `oo::configurable` class body is lowered, so its `[::ns::Pin new]`
    // writes type the `Pins` dict.
    let registry = CommandRegistry::build_default();
    let src = "namespace eval ::SpiceGenTcl {\n\
                     oo::configurable create Pin { property node }\n\
                     oo::configurable create Device {\n\
                       variable Pins\n\
                       method actOnPin {action pin node} {\n\
                         ::switch -- $action {\n\
                           add { ::dict append Pins $pin [::SpiceGenTcl::Pin new $pin $node] }\n\
                           node { [::dict get $Pins $pin] configure -node $node }\n\
                         }\n\
                       }\n\
                     }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let map = object_collection_classes(&cu);
    assert_eq!(
        map.get("Pins").map(|s| s.contains("::SpiceGenTcl::Pin")),
        Some(true),
        "Pins should be a collection of ::SpiceGenTcl::Pin; got {map:?}"
    );
}

/// Build a unit and its full fact set in one step (the tests below want
/// both).
/// The shipped catalogue projected for Tcl 8.6, the release these
/// fixtures analyse.
fn release_registry() -> CommandRegistry {
    CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap())
}

fn facts_for(src: &str) -> (CommandRegistry, ObjectHandleFacts) {
    let registry = release_registry();
    let cu = CompilationUnit::build_for(src, &registry, false);
    let facts = object_handle_facts(&cu, &registry);
    (registry, facts)
}

/// The classes `by_scope` binds for `(owner, var)`, sorted for a stable
/// assertion message.
fn scoped(facts: &ObjectHandleFacts, owner: &str, var: &str) -> Vec<String> {
    let owners = facts
        .owner_spans
        .iter()
        .filter_map(|span| {
            if span.unit == owner {
                Some(ObjectFactOwner::Frame(span.frame))
            } else if span.class.as_deref() == Some(owner) {
                span.class_owner.clone()
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();
    let mut values = owners
        .into_iter()
        .flat_map(|owner| {
            facts
                .by_scope
                .get(&(owner, var.to_owned()))
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

#[test]
fn colliding_method_reports_have_distinct_scoped_object_candidates() {
    let source = "namespace eval A {}\noo::class create A::B {method c {} {set handle [Pin new]}}\noo::class create A {method B::c {} {set handle 0}}\n";
    let registry = release_registry();
    let unit = CompilationUnit::build_for(source, &registry, false);
    let facts = object_handle_facts(&unit, &registry);
    let scopes = facts
        .owner_spans
        .iter()
        .filter(|scope| scope.unit == "::A::B::c")
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2);
    assert_ne!(scopes[0].frame, scopes[1].frame);
    assert_ne!(scopes[0].class_owner, scopes[1].class_owner);
}

#[test]
fn any_scope_is_verbatim_object_handle_classes() {
    // The widened carrier must not perturb the map every existing consumer
    // reads: `any_scope` is the old return value, byte-for-byte.
    for src in [
        "set chart [ticklecharts::chart new]\n$chart Xaxis -name x\n",
        "oo::class create Pin { method cfg {args} {} }\n\
             proc connect {dev} { $dev cfg }\n\
             set p [Pin new]\nconnect $p\n",
        "set x [expr {1 + 2}]\n",
        "",
    ] {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &registry, false);
        assert_eq!(
            object_handle_facts(&cu, &registry).any_scope,
            object_handle_classes(&cu, &registry),
            "any_scope must equal object_handle_classes for `{src}`"
        );
    }
}

#[test]
fn empty_seed_fast_path_is_behaviour_preserving() {
    // The fast path skips the propagation walk when no edge can fire.  Pin
    // it against the unconditional walk on every shape that binds
    // from an *empty seed set* — a bare `out.is_empty()` gate (the obvious
    // one) silently drops all four of the middle cases here.
    for src in [
        // Nothing at all: the fast path fires.
        "proc noop {} { return 1 }\nset a 1\nputs $a\n",
        "proc a {x} { b $x }\nproc b {y} { puts $y }\na 1\n",
        // Proc-return edge from a factory whose object never lands in a
        // `set` target the harvest can see.
        "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { return [Pin new] }\n\
             set c [make]\n$c cfg\n",
        // `arg_classes`' direct-constructor branch: a registry factory
        // written straight into a proc-parameter, a constructor
        // parameter, and a bare naming factory.
        "proc take {dev} { $dev curselection }\ntake [listbox .l]\n",
        "oo::class create Wrap { constructor {inner} { $inner curselection } }\n\
             Wrap new [listbox .l]\n",
        "proc take {dev} { $dev walk root }\ntake [struct::graph]\n",
        // Nested inside an assigned value.
        "oo::class create Wrap { constructor {inner} {} }\n\
             set w [Wrap new [listbox .l]]\n",
    ] {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &registry, false);
        assert_eq!(
            object_handle_classes(&cu, &registry),
            object_handle_classes_full_walk(&cu, &registry),
            "the empty-seed fast path changed the map for `{src}`"
        );
    }
}

#[test]
fn owner_attribution_alias_edge_binds_in_the_assigning_unit() {
    // `set b $a` inside `mk` binds `b` in `::mk`, not globally — the
    // same-name collision across procs `any_scope` cannot avoid.
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc mk {} { set a [Pin new]\n set b $a\n $b cfg }\n\
             proc other {} { set b 7\n puts $b }\n",
    );
    assert_eq!(scoped(&facts, "::mk", "b"), vec!["::Pin".to_owned()]);
    assert!(
        scoped(&facts, "::other", "b").is_empty(),
        "`b` in ::other is an unrelated integer; by_scope must not widen \
             onto it (any_scope does: {:?})",
        facts.any_scope.get("b")
    );
}

#[test]
fn generic_factory_return_candidates_flow_without_a_return_type_receipt() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (prefix, expected) in [
        ("oo::class create Pin {}", true),
        (
            "oo::class create Pin {}; rename Pin {}; proc Pin args {return plain}",
            false,
        ),
    ] {
        let source = format!(
            "{prefix}; proc make {{}} {{return [Pin new]}}; proc take {{dev}} {{return ok}}; set receiver [make]; take $receiver"
        );
        let mut unit = CompilationUnit::build_for_profile(
            &source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        unit.procedures.remove(
            unit.ir_module
                .procedures
                .unique_display("::make")
                .unwrap()
                .0,
        );
        unit.procedures.remove(
            unit.ir_module
                .procedures
                .unique_display("::take")
                .unwrap()
                .0,
        );
        let facts = object_handle_facts(&unit, registry);
        assert_eq!(
            facts
                .any_scope
                .get("dev")
                .is_some_and(|classes| classes.contains("::Pin")),
            expected,
            "the retained factory implementation selects the advisory flow: {facts:?}"
        );
        assert!(
            !facts.returns_object.contains_key(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0
            )
        );
    }
}

#[test]
fn owner_attribution_return_edge_binds_in_the_assigning_unit() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { set p [Pin new]\n return $p }\n\
             proc use {} { set q [make]\n $q cfg }\n",
        &registry,
        false,
    );
    let facts = object_handle_facts(&unit, &registry);
    assert_eq!(scoped(&facts, "::use", "q"), vec!["::Pin".to_owned()]);
    assert_eq!(
        facts
            .returns_object
            .get(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0
            )
            .map(String::as_str),
        Some("::Pin"),
        "the factory-proc return fact must be exported; got {:?}",
        facts.returns_object
    );
}

#[test]
fn owner_attribution_param_edge_binds_in_the_callee() {
    // `connect $p` binds `dev` in `::connect` — the callee, where the name
    // lives — not in the caller that supplied the argument.
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc connect {dev} { $dev cfg }\n\
             set p [Pin new]\nconnect $p\n",
    );
    assert_eq!(scoped(&facts, "::connect", "dev"), vec!["::Pin".to_owned()]);
    assert!(
        scoped(&facts, "::top", "dev").is_empty(),
        "the param edge must not bind `dev` at the call site's scope"
    );
    assert_eq!(scoped(&facts, "::top", "p"), vec!["::Pin".to_owned()]);
}

#[test]
fn owner_attribution_ctor_param_edge_binds_in_the_constructor() {
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             oo::class create Wrap {\n\
               variable held\n\
               constructor {inner} { $inner cfg; set held $inner }\n\
             }\n\
             set p [Pin new]\nset w [Wrap new $p]\n",
    );
    assert_eq!(
        scoped(&facts, "::Wrap::<constructor>", "inner"),
        vec!["::Pin".to_owned()],
        "the ctor param is owned by the constructor unit; by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn owner_attribution_instance_var_is_owned_by_the_class() {
    // The cross-method bridge: `engine` is written in `Car`'s constructor
    // and read in `Car`'s `go`.  Keying it by the *class* (not by either method)
    // is what makes the cross-method dispatch resolvable at all.
    let (_r, facts) = facts_for(
        "oo::class create Motor { method spin {args} {} }\n\
             oo::class create Car {\n\
               variable engine\n\
               constructor {e} { set engine $e }\n\
               method go {} { $engine spin }\n\
             }\n\
             set m [Motor new]\nset c [Car new $m]\n",
    );
    assert_eq!(
        scoped(&facts, "::Car", "engine"),
        vec!["::Motor".to_owned()],
        "instance var `engine` is owned by ::Car; by_scope={:?}",
        facts.by_scope
    );
    assert!(
        scoped(&facts, "::Car::<constructor>", "engine").is_empty(),
        "an instance variable must not also be keyed by the writing method"
    );
    // …and `classes_in_scope` finds it from a byte inside the *other*
    // method, via the class fallback.
    let offset = facts
        .owner_spans
        .iter()
        .find(|o| o.unit == "::Car::go")
        .expect("::Car::go has an owner span")
        .span
        .start();
    assert_eq!(
        facts
            .classes_in_scope(offset, "engine")
            .map(|s| s.contains("::Motor")),
        Some(true),
        "classes_in_scope inside ::Car::go must reach the class-owned name"
    );
}

#[test]
fn collection_and_global_cell_facts_are_exported() {
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             dict set pins a [Pin new]\n\
             set ::board [listbox .l]\n",
    );
    assert_eq!(
        facts.collections.get("pins").map(|s| s.contains("::Pin")),
        Some(true),
        "collections must carry object_collection_classes verbatim; got {:?}",
        facts.collections
    );
    assert_eq!(
        facts
            .global_object_cells
            .get("::board")
            .map(|s| s.contains("listbox")),
        Some(true),
        "a `::`-qualified handle must be exported as a global cell; got {:?}",
        facts.global_object_cells
    );
    assert!(
        !facts.global_object_cells.contains_key("pins"),
        "an unqualified local is not a global cell"
    );
}

#[test]
fn registry_factory_operator_form_binds_nothing_by_scope() {
    // The `by_scope` twin of `registry_factory_operator_form_binds_nothing`:
    // the deserialise operator words name no object command, so the
    // abstention must survive in the scope-keyed map too — a bogus `=`
    // binding there would be read by the rename/refusal consumers.
    let registry = CommandRegistry::build_default();
    for op in ["=", ":=", "as", "deserialize"] {
        let src = format!("struct::graph {op} $serial\n");
        let cu = CompilationUnit::build_for(&src, &registry, false);
        let facts = object_handle_facts(&cu, &registry);
        assert!(
            !facts.any_scope.contains_key(op),
            "`struct::graph {op}` must not track `{op}` in any_scope"
        );
        assert!(
            facts.by_scope.keys().all(|(_owner, name)| name != op),
            "`struct::graph {op}` must not track `{op}` in by_scope; got {:?}",
            facts.by_scope
        );
    }
}

#[test]
fn by_scope_does_not_import_another_units_binding_through_an_alias() {
    // FP guard.  `x` is a `::Pin` in `::a` and a plain integer in `::b`.
    // `any_scope` unions the two — that is its documented, deliberate
    // imprecision — but the alias `set y $x` inside `::b` must resolve `x`
    // **in `::b`**, where there is no object.  A `(::b, y) → ::Pin` entry
    // would be a false *singleton* in the map the rename edits and the
    // "provably a different class" refusal gate read as authoritative: it
    // would rewrite an unrelated integer.
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc a {} { set x [Pin new] }\n\
             proc b {} { set x 0\n set y $x }\n",
    );
    assert_eq!(
        scoped(&facts, "::a", "x"),
        vec!["::Pin".to_owned()],
        "the seed itself must still be scope-keyed to ::a"
    );
    assert!(
        scoped(&facts, "::b", "y").is_empty(),
        "`y` in ::b aliases ::b's own integer `x`, not ::a's Pin; \
             by_scope must not import the other unit's binding (any_scope \
             legitimately unions it: {:?})",
        facts.any_scope.get("y")
    );
    assert!(
        scoped(&facts, "::b", "x").is_empty(),
        "::b's `x` is an integer; by_scope must not bind it"
    );
}

#[test]
fn by_scope_alias_within_one_unit_still_propagates() {
    // TP twin of the guard above: the same alias edge, both ends in the
    // same unit, must still bind.
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc c {} { set p [Pin new]\n set q $p\n $q cfg }\n",
    );
    assert_eq!(
        scoped(&facts, "::c", "q"),
        vec!["::Pin".to_owned()],
        "an alias whose source is bound in the *same* unit must propagate; \
             by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn by_scope_instance_var_alias_reads_the_class_owner() {
    // The cross-method bridge under scoped propagation: `engine` is
    // written in the constructor and read in another method.  The source lookup for
    // `set m $engine` inside `::Car::go` must fall back to the *class*
    // owner, or the bridge the design is built on would break.
    let (_r, facts) = facts_for(
        "oo::class create Motor { method spin {args} {} }\n\
             oo::class create Car {\n\
               variable engine\n\
               constructor {e} { set engine $e }\n\
               method go {} { set m $engine\n $m spin }\n\
             }\n\
             set mo [Motor new]\n\
             set c [Car new $mo]\n",
    );
    assert_eq!(
        scoped(&facts, "::Car", "engine"),
        vec!["::Motor".to_owned()],
        "the class-owned instance variable must still bind"
    );
    assert_eq!(
        scoped(&facts, "::Car::go", "m"),
        vec!["::Motor".to_owned()],
        "reading a class-owned instance variable from a method must resolve \
             through the class owner; by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn by_scope_cross_unit_call_edges_still_bind() {
    // The two legitimately cross-scope edges must survive the scoped
    // source resolution: a proc-return edge (the source is the callee's
    // return type, which is unit-independent) and a proc-parameter edge
    // (the argument is resolved in the *caller*, bound in the callee).
    let (_r, facts) = facts_for(
        "oo::class create Pin { method cfg {args} {} }\n\
             proc make {} { set p [Pin new]\n return $p }\n\
             proc connect {dev} { $dev cfg }\n\
             proc drive {} { set q [make]\n connect $q }\n",
    );
    assert_eq!(
        scoped(&facts, "::drive", "q"),
        vec!["::Pin".to_owned()],
        "the proc-return edge binds in the assigning unit; by_scope={:?}",
        facts.by_scope
    );
    assert_eq!(
        scoped(&facts, "::connect", "dev"),
        vec!["::Pin".to_owned()],
        "the proc-parameter edge reads the argument in ::drive and binds \
             the parameter in ::connect; by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn method_return_capture_types_the_handle() {
    // `set b [$a make]` — the receiver is already typed and
    // `::A::make`'s own return type names an object class, so the captured
    // handle is a ::B in both facts.
    let (_r, facts) = facts_for(
        "oo::class create A { method make {} { ::return [::B new] } }\n\
             oo::class create B { method greet {} { ::return \"hi\" } }\n\
             set a [A new]\n\
             set b [$a make]\n\
             $b greet\n",
    );
    assert_eq!(
        facts.any_scope.get("b").map(|s| s.contains("::B")),
        Some(true),
        "`b` should be a ::B handle via the method-return edge; got {:?}",
        facts.any_scope
    );
    assert_eq!(
        scoped(&facts, "::top", "b"),
        vec!["::B".to_owned()],
        "the method-return edge binds in the assigning unit; by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn method_return_edge_abstains_on_computed_member_and_literal_return() {
    // TN twins of the edge: a computed member word proves nothing, and a
    // method returning a plain literal types no handle.
    let (_r, facts) = facts_for(
        "oo::class create A {\n\
               method make {} { return [B new] }\n\
               method name {} { return plain }\n\
             }\n\
             oo::class create B {}\n\
             set a [A new]\n\
             set m make\n\
             set b [$a $m]\n\
             set c [$a name]\n",
    );
    assert!(
        !facts.any_scope.contains_key("b"),
        "a computed member word must not bind; got {:?}",
        facts.any_scope
    );
    assert!(
        !facts.any_scope.contains_key("c"),
        "a literal-returning method must not bind; got {:?}",
        facts.any_scope
    );
}

#[test]
fn method_return_edge_does_not_import_another_units_receiver() {
    // The by_scope guard for the new edge: `a` is a ::A only inside
    // `::mk`; a same-named `a` in `::other` is untyped there, so the
    // capture in `::other` must not bind in the scope-keyed map (the
    // union may — its documented imprecision).
    let (_r, facts) = facts_for(
        "oo::class create A { method make {} { ::return [::B new] } }\n\
             oo::class create B {}\n\
             proc mk {} { set a [A new]\n set b [$a make] }\n\
             proc other {} { set a 1\n set b [$a make] }\n",
    );
    assert_eq!(
        scoped(&facts, "::mk", "b"),
        vec!["::B".to_owned()],
        "the in-unit capture must bind; by_scope={:?}",
        facts.by_scope
    );
    assert!(
        scoped(&facts, "::other", "b").is_empty(),
        "::other's `a` is an integer; the scoped fact must not import \
             ::mk's receiver; by_scope={:?}",
        facts.by_scope
    );
}

#[test]
fn owner_spans_are_sorted_and_resolve_the_innermost_scope() {
    let (_r, facts) = facts_for(
        "set g 1\n\
             proc alpha {} { set a 1 }\n\
             oo::class create K { method m {} { set b 2 } }\n\
             proc beta {} { set c 3 }\n",
    );
    assert!(
        facts
            .owner_spans
            .windows(2)
            .all(|w| w[0].span.start() <= w[1].span.start()),
        "owner_spans must be start-sorted for binary search; got {:?}",
        facts.owner_spans
    );
    // Top-level bytes resolve to `::top`, so a top-level handle has an
    // owner to key against rather than falling off the index.
    assert_eq!(
        facts.owner_at(0).map(|o| o.unit.as_str()),
        Some("::top"),
        "byte 0 is top-level code; got {:?}",
        facts.owner_at(0)
    );
    for unit in ["::alpha", "::beta", "::K::m"] {
        let span = facts
            .owner_spans
            .iter()
            .find(|o| o.unit == unit)
            .unwrap_or_else(|| panic!("{unit} must have an owner span"))
            .span;
        assert_eq!(
            facts.owner_at(span.start()).map(|o| o.unit.as_str()),
            Some(unit),
            "owner_at must resolve a byte inside {unit} to it, not to the \
                 enclosing top level; owner_spans={:?}",
            facts.owner_spans
        );
    }
    assert_eq!(
        facts
            .owner_spans
            .iter()
            .find(|o| o.unit == "::K::m")
            .and_then(|o| o.class.as_deref()),
        Some("::K"),
        "a method's owner span must name its class for the fallback key"
    );
}
#[test]
fn factory_return_metadata_keeps_supplied_availability_and_refuses_missing_or_foreign_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional factory class candidates; no successful return or allocated receiver.
    let baseline = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut descriptor = registry.get("return").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(descriptor);
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let registry = context.commands();
    let mut unit = CompilationUnit::build_with_context_registry(
        "oo::class create Pin {}; proc make {} {return [Pin new]}",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            dialect: registry.profile(),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        std::sync::Arc::clone(&context),
    );
    unit.procedures.remove(
        unit.ir_module
            .procedures
            .unique_display("::make")
            .unwrap()
            .0,
    );
    assert!(returning_procs(&unit).is_empty());
    let current = returning_proc_candidates(&unit, registry, None);
    assert_eq!(
        current
            .get(
                unit.ir_module
                    .procedures
                    .unique_display("::make")
                    .unwrap()
                    .0
            )
            .map(String::as_str),
        Some("::Pin")
    );
    let original = unit
        .ir_module
        .procedures
        .get(
            unit.ir_module
                .procedures
                .unique_display("::make")
                .unwrap()
                .0,
        )
        .unwrap()
        .body
        .clone();
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let foreign = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
    );
    for withheld in [
        Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        )),
        Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign,
            input.lexer_config(),
        )),
        None,
    ] {
        unit.ir_module.source_metadata_input = withheld;
        assert!(returning_proc_candidates(&unit, registry, None).is_empty());
        assert_eq!(
            unit.ir_module
                .procedures
                .get(
                    unit.ir_module
                        .procedures
                        .unique_display("::make")
                        .unwrap()
                        .0
                )
                .unwrap()
                .body,
            original
        );
    }
}

#[test]
fn normal_result_query_uses_the_shared_roster_and_withdraws_missing_source_ownership() {
    // naming.source.original-collection-body-owner
    // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
    // Conditional result types under existing allocation guards, not runtime execution.
    let source = "proc make {} {return 1}\nmake\n";
    let (registry, unit) = provided_unit(source, &[]);
    assert!(
        unit.retained_source_class_names(&registry)
            .unwrap()
            .is_empty()
    );
    let target = unit
        .ir_module
        .top_level
        .statements
        .last()
        .and_then(|statement| {
            unit.ir_module
                .top_level
                .retained_source_tokens_for_statement(statement)
        })
        .and_then(|tokens| tokens.source_binding.as_ref())
        .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
        .expect("original selected procedure allocation");
    let mut baseline = ObjectHandleFacts::default();
    retain_normal_procedure_result_types(&unit, &registry, &mut baseline);
    assert_eq!(
        baseline
            .normal_procedure_result(target)
            .and_then(crate::types::TypeLattice::tcl_type),
        Some(tcl_registry::TclType::Int)
    );
    for axis in 0..5 {
        let mut changed = unit.clone();
        match axis {
            0 => changed.source_class_facts = None,
            1 => changed.ir_module.retained_source_bindings = None,
            2 => changed.ir_module.source_metadata_input = None,
            3 => changed.ir_module.lexer_config.strict_quoting ^= true,
            _ => changed.source = "proc other {} {return 1}\nother\n".to_owned(),
        }
        let mut stale = baseline.clone();
        retain_normal_procedure_result_types(&changed, &registry, &mut stale);
        assert!(stale.normal_procedure_results.is_empty(), "axis {axis}");
        assert!(stale.normal_procedure_result(target).is_none());
    }
    let mut changed_store = registry;
    changed_store.insert(tcl_registry::CommandSpec {
        name: "unrelated_selected_command",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let mut stale = baseline;
    retain_normal_procedure_result_types(&unit, &changed_store, &mut stale);
    assert!(stale.normal_procedure_results.is_empty());
    assert!(stale.normal_procedure_result(target).is_none());
}
