from pathlib import Path
p=Path('/tmp/name-cfg-index-read-consumers-20261014/after/rust/tcl-compiler/src/cfg_builder/mod.rs');s=p.read_text();marker='    fn summary_keys_decline_one_changed_member_and_foreign_module_source() {';assert s.count(marker)==1
s=s.replace(marker,'''    fn read_beside_writes_uses_original_roles_and_preserves_refusals() {
        for (source, expected) in [
            ("puts [set x 2] [info exists x]", vec!["x".to_owned()]),
            ("puts [set x 2]", Vec::new()),
        ] {
            let module = lower_module(source);
            let registry = module.resolved_registry();
            let prepared = prepare_cfg_context_bundle(&module, registry);
            let builder = CfgBuilder::new_with_classes(
                false,
                prepared.context.0.clone(),
                prepared.context.1.clone(),
                prepared.context.2.clone(),
                prepared.context.3.clone(),
                registry,
                prepared.command_classes.clone(),
            ).with_procedure_summary_owners(prepared.context.4.clone());
            let statement = module.top_level.statements.last().unwrap();
            let tokens = module.top_level.retained_source_tokens_for_statement(statement).unwrap();
            let written = vec!["x".to_owned(), "unrelated".to_owned()];
            let reads = builder.read_beside_writes(&written, Some(tokens), |scanner| {
                scanner.scan_lexical_word("", registry)
            });
            assert_eq!(reads, expected, "{source}");
            assert_eq!(builder.read_beside_writes(&written, None, |scanner| {
                scanner.scan_lexical_word("", registry)
            }), written);
            let mut changed = tokens.clone();
            changed.source_binding = None;
            assert_eq!(builder.read_beside_writes(&written, Some(&changed), |scanner| {
                scanner.scan_lexical_word("", registry)
            }), written);
        }
    }

    #[test]
    fn summary_keys_decline_one_changed_member_and_foreign_module_source() {''')
p.write_text(s)
