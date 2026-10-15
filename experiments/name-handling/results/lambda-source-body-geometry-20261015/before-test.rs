    #[test]
    fn complete_body_coverage_refuses_unrepresented_lambda_containers() {
        for source in ["apply {{} {puts hidden}}", "apply $lambda"] {
            let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
            let context = tcl_registry::model::context_for_profile(profile);
            let config = LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let syntax = super::super::source_structure::OriginalSourceRegistryContext::capture(
                source, input,
            );
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words = syntax.words(&command).unwrap();
            assert!(
                words
                    .roles()
                    .unwrap()
                    .iter()
                    .any(|(_, role)| *role == ArgRole::LambdaLiteral)
            );
            assert!(source_script_bodies_for_mutation_coverage(&words, &context).is_none());
            assert!(words.source_script_bodies(&context).is_empty());
        }
    }

