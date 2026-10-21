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
    let cu = crate::compilation_unit::CompilationUnit::build_with_analysis_input(&src, options, None, &input);
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
