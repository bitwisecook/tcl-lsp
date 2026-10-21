use tcl_runtime_api::CompileService as _;
#[test]
fn zz_sol49_probe() {
    let src = std::env::var("PROBE_SRC").unwrap_or_else(|_| "set z x\nputs [string length \"{}$z\"]\n".into());
    let dialect = std::env::var("PROBE_DIALECT").unwrap_or_else(|_| "tcl8.6".into());
    let profile = tcl_dialect::DialectProfile::find(&dialect).unwrap();
    let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
    let asm = service.compile_for_profile(&src, profile);
    match asm {
        Ok(asm) => {
            for (i, ins) in asm.top_level.instructions.iter().enumerate() {
                eprintln!("{i:3} {:?} {:?} {}", ins.op, ins.operands, ins.comment);
            }
            eprintln!("LITERALS {:?}", asm.top_level.literals);
        }
        Err(e) => eprintln!("ERR {e:?}"),
    }
}

#[test]
fn zz_sol49_analyse() {
    let src = std::env::var("PROBE_SRC").unwrap_or_else(|_| "when HTTP_REQUEST { set ::counter 0 }".into());
    let dialect = std::env::var("PROBE_DIALECT").unwrap_or_else(|_| "f5-irules".into());
    let mut a = crate::analyser::Analyser::new();
    let res = a.analyse(&src, &dialect);
    for d in &res.diagnostics {
        eprintln!("DIAG {} {}", d.code.as_str(), d.message);
    }
}

#[test]
fn zz_sol49_cu() {
    let src = std::env::var("PROBE_SRC").unwrap_or_else(|_| "when HTTP_REQUEST { set ::counter 0 }".into());
    let dialect = std::env::var("PROBE_DIALECT").unwrap_or_else(|_| "f5-irules".into());
    let mut a = crate::analyser::Analyser::new();
    let res = a.analyse(&src, &dialect);
    let input = res.resolved_input.clone().unwrap();
    let generation = input.context_registry();
    let registry = generation.commands().clone();
    let options = crate::compilation_unit::UnitBuildOptions {
        registry: &registry,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: None,
    };
    let cu = if std::env::var_os("PROBE_PROFILE_BUILD").is_some() {
        let registry = tcl_registry::model::ingress::static_context_for(&dialect).commands();
        let profile = tcl_registry::model::ingress::resolve_environment(&dialect).analyser_profile();
        crate::compilation_unit::CompilationUnit::build_for_profile(&src, registry, false, profile)
    } else {
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(&src, options, None, &input)
    };
    let dump = |label: &str, fu: &crate::compilation_unit::FunctionUnit| {
        eprintln!("FUNC {label} points={} ", fu.ssa.point_contexts.is_some());
        let mut ids: Vec<_> = fu.cfg.blocks.keys().collect();
        ids.sort();
        for id in ids {
            for st in &fu.cfg.blocks[id].statements {
                let t = format!("{st:?}");
                eprintln!("   ST {}", &t[..t.len().min(std::env::var("PROBE_W").ok().and_then(|w| w.parse().ok()).unwrap_or(300))]);
            }
        }
    };
    if std::env::var_os("PROBE_PROFILE_BUILD").is_some() {
        let registry = tcl_registry::model::ingress::static_context_for(&dialect).commands();
        let profile = tcl_registry::model::ingress::resolve_environment(&dialect).analyser_profile();
        for d in crate::compiler_checks::run_all_checks(&cu, registry, Some(profile)) {
            eprintln!("CHECK {:?} {}", d.code, d.message);
        }
        eprintln!("SCCP top {:?}", format!("{:?}", cu.top_level.sccp.values).chars().take(1500).collect::<String>());
    }
    dump("top", &cu.top_level);
    for (id, fu) in &cu.procedures {
        if let Some(points) = fu.ssa.point_contexts.as_ref() {
            for (&bid, block) in &fu.cfg.blocks {
                for (index, statement) in block.statements.iter().enumerate() {
                    let before = points.before_statement(bid, index);
                    let after = points.after_statement(bid, index);
                    let places = crate::place_bridge::statement_mutation_places_with_continuation(statement, before, after, &registry);
                    eprintln!("MUT {:?} ns={:?} dyn={} pol={} -> {:?}", statement.tokens().map(|t| t.argv_texts.clone()), before.namespace_identity, before.dynamic_bindings, before.execution_name_policy.is_some(), places.iter().map(|p| (p.ns.clone(), p.name.clone(), p.dynamic, p.is_global())).collect::<Vec<_>>());
                }
            }
        }
        let event = crate::source_graph::event_body_for_function(&cu.ir_module, id, fu, &registry);
        eprintln!("PROC {id:?} name={:?} event={:?}", cu.ir_module.procedures.get(id).map(|p| p.name.clone()), event.map(|e| e.event()));
        if let Some(p) = cu.ir_module.procedures.get(id) {
            for st in &p.body.statements { let t = format!("{st:?}"); eprintln!("   IR {}", &t[..t.len().min(400)]); }
        }
        dump("proc", fu);
    }
    for (id, fu) in &cu.body_units {
        eprintln!("BODYUNIT {id:?}");
        dump("body", fu);
    }
}

#[test]
fn zz_sol49_irules_checks() {
    let src = std::env::var("PROBE_SRC").unwrap_or_else(|_| "when HTTP_REQUEST { set ::counter 0 }".into());
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let context = environment.default_context_registry();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        std::sync::Arc::clone(&context),
        config,
    );
    let cu = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        &src,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let q = Some(environment.unit_profile().surface_query());
    for x in crate::irules_checks::find_collect_flow_warnings(&cu, context.commands(), q) { eprintln!("IRW collect {}", x.message); }
    for x in crate::irules_checks::find_unnormalised_getter_warnings(&cu, context.commands(), q) { eprintln!("IRW getter {}", x.message); }
}

#[test]
fn zz_sol49_world() {
    let src = std::env::var("PROBE_SRC").unwrap_or_else(|_| {
        "snit::type Factory {\n    typemethod make {} {\n        return [Factory create x]\n    }\n}\nFactory make\n".into()
    });
    let dialect = std::env::var("PROBE_DIALECT").unwrap_or_else(|_| "tcl8.6".into());
    let mut a = crate::analyser::Analyser::new();
    let res = a.analyse(&src, &dialect).clone();
    eprintln!("CLASSES {:?}", res.all_classes.keys().collect::<Vec<_>>());
    eprintln!("ORIGINAL_CLASS_DECLS {}", res.original_class_declarations().count());
    for record in res.original_class_declarations() {
        let members = &record.metadata().original_members;
        for declaration in members.declarations() {
            eprintln!(
                "   MEMBER side={:?} name={:?} delegate={} purpose={:?}",
                declaration.side(),
                String::from_utf8_lossy(declaration.original_name_input().bytes()),
                declaration.native_class_delegate(),
                declaration.name_purpose(),
            );
        }
        eprintln!(
            "   CLASSOBJ methods={:?} instance={:?}",
            members.methods(crate::analyser::types::MemberSide::ClassObject).map(|m| m.len()),
            members.methods(crate::analyser::types::MemberSide::Instance).map(|m| m.len()),
        );
    }
    eprintln!(
        "ORIGINAL_PROC_DECLS {} lexical_advice={} procs={:?}",
        res.original_procedure_declarations().count(),
        res.allows_lexical_declaration_advice(),
        res.all_procs.keys().collect::<Vec<_>>()
    );
    for inv in &res.command_invocations {
        eprintln!(
            "INV {:?} range={:?} lookup={} input={}",
            inv.name,
            inv.range,
            inv.original_lookup.is_some(),
            inv.original_name_input.is_some()
        );
    }
    let Some(realm) = res.retained_command_realm() else {
        eprintln!("NO REALM");
        return;
    };
    let image = tcl_lexer::SourceImage::document(&src);
    for inv in &res.command_invocations {
        let head = res
            .body_lexer_config
            .and_then(|config| realm.original_written_head_at_span_in_source(&image, inv.range, config));
        eprintln!(
            "INVHEAD {:?} exec_site={} head={:?} resolved={:?} refd={}",
            inv.name,
            inv.lookup.is_execution_site(),
            head.map(|(w, _)| src.get(w.span().start() as usize..w.span().end() as usize).unwrap_or("?").to_owned()),
            inv.resolved_qualified_name,
            inv.resolved_command_reference.is_some(),
        );
    }
    for seg in crate::segmenter::segment_commands(&src) {
        let offset = seg.argv[0].span.start();
        let b = realm.invocation_at_source(seg.name(), offset);
        eprintln!(
            "SEG {:?}@{offset} reach={:?} unknown={} absent={} targets={:?} declared={:?} catalogue={:?}",
            seg.name(),
            b.runtime_reachability(),
            b.unknown,
            b.may_be_absent,
            b.targets.iter().map(|t| (t.command.clone(), t.kind, t.implementation_allocation.is_some(), t.identity.is_some())).collect::<Vec<_>>(),
            b.declared_command.as_ref().map(|d| format!("{d:?}").chars().take(200).collect::<String>()),
            b.catalogue_command.as_ref().map(|d| format!("{d:?}").chars().take(200).collect::<String>()),
        );
        let proc_call = crate::registry_invocation::source_structure::source_class_proc_call_at(&src, &res, offset);
        eprintln!(
            "   class_proc_call={:?}",
            proc_call.as_ref().map(|c| (
                String::from_utf8_lossy(c.member_name()).into_owned(),
                c.member_span(),
                c.class_declaration().source_class(&res).is_some()
            ))
        );
        let ctor = crate::registry_invocation::source_structure::source_constructor_call_at(&src, &res, offset);
        eprintln!(
            "   ctor_call={} source_class={}",
            ctor.is_some(),
            ctor.as_ref().is_some_and(|c| c.class_declaration().source_class(&res).is_some()),
        );
        eprintln!(
            "   class_factory={:?} named_obj={} class_entries={:?}",
            b.proved_class_definition_factory().map(|(f, _)| f.to_owned()),
            b.named_object_instance_at_dispatch().is_some(),
            b.class_definition_method_entries().map(|(_, e)| e.len()),
        );
    }
}
