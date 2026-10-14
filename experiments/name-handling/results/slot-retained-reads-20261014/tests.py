from pathlib import Path
p=Path('/tmp/name-slot-retained-reads-20261014/after/rust/tcl-compiler/src/slot_allocation.rs');s=p.read_text(); marker='    /// The backward fixpoint sweeps exit-first, so a function whose blocks'
assert s.count(marker)==1
s=s.replace(marker,'''    fn supplied_function(
        source: &str,
    ) -> (std::sync::Arc<tcl_registry::model::ContextRegistry>, crate::compilation_unit::CompilationUnit) {
        let context = tcl_registry::model::ingress::resolve_environment("tcl8.6")
            .default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile, profile, std::sync::Arc::clone(&context), config,
        );
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        (context, unit)
    }

    fn return_site(cfg: &cfg::Function) -> (BlockId, &cfg::Block) {
        cfg.blocks.iter().find_map(|(&id, block)| {
            matches!(block.terminator, Some(Terminator::Return { .. })).then_some((id, block))
        }).expect("represented return")
    }

    #[test]
    fn retained_terminator_read_selects_the_original_ssa_cell() {
        let (context, unit) = supplied_function(
            "proc p {} {set value DATA; set unrelated OTHER; return $value}",
        );
        let body = unit.procedures.values().find(|body| body.name == "::p").unwrap();
        let (id, block) = return_site(&body.cfg);
        let reads = terminator_read_names(&body.cfg, id, block, &body.ssa, context.commands());
        assert!(reads.contains("value"), "{reads:?}");
        assert!(!reads.contains("unrelated"), "{reads:?}");
    }

    #[test]
    fn foreign_and_unavailable_terminator_owners_keep_known_cells_live() {
        let (context, unit) = supplied_function(
            "proc p {} {set value DATA; set protected OTHER; return $value}",
        );
        let body = unit.procedures.values().find(|body| body.name == "::p").unwrap();
        let mut foreign = context.commands().clone();
        let mut added = foreign.get("set").unwrap().clone();
        added.name = "foreign_probe";
        foreign.insert(added);
        let (id, block) = return_site(&body.cfg);
        let foreign_reads = terminator_read_names(&body.cfg, id, block, &body.ssa, &foreign);
        assert!(foreign_reads.contains("value"), "{foreign_reads:?}");
        assert!(foreign_reads.contains("protected"), "{foreign_reads:?}");
        let mut cfg = body.cfg.clone();
        cfg.metadata_context = crate::cfg_builder::CfgMetadataContext::Unavailable;
        let mut ssa = body.ssa.clone();
        ssa.point_contexts = None;
        let (id, block) = return_site(&cfg);
        let unavailable = terminator_read_names(&cfg, id, block, &ssa, context.commands());
        assert!(unavailable.contains("value"), "{unavailable:?}");
        assert!(unavailable.contains("protected"), "{unavailable:?}");
    }

    #[test]
    fn retained_array_root_read_keeps_its_known_members_live() {
        let (context, unit) = supplied_function(
            "proc p {} {set a(k) FIRST; set a(j) SECOND; return [array get a]}",
        );
        let body = unit.procedures.values().find(|body| body.name == "::p").unwrap();
        let (id, block) = return_site(&body.cfg);
        let reads = terminator_read_names(&body.cfg, id, block, &body.ssa, context.commands());
        assert!(reads.contains("a(k)"), "{reads:?}");
        assert!(reads.contains("a(j)"), "{reads:?}");
    }

    #[test]
    fn standalone_nested_command_keeps_protective_reads() {
        let (context, unit) = supplied_function(
            "proc p {} {set protected DATA; return [unknown_read]}",
        );
        let body = unit.procedures.values().find(|body| body.name == "::p").unwrap();
        let mut cfg = body.cfg.clone();
        cfg.metadata_context = crate::cfg_builder::CfgMetadataContext::Standalone;
        let mut ssa = body.ssa.clone();
        ssa.point_contexts = None;
        let (id, block) = return_site(&cfg);
        let reads = terminator_read_names(&cfg, id, block, &ssa, context.commands());
        assert!(reads.contains("protected"), "{reads:?}");
    }

'''+marker)
p.write_text(s)
