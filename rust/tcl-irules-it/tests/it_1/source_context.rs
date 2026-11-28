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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords;
    use tcl_irules::*;
    use tcl_lexer::{LexerConfig, SourceImage};
    use tcl_registry::{CommandRegistry, Traits};

    #[test]
    fn original_irules_source_context_keeps_event_receipts_and_handler_barriers() {
        crate::support::install_surfaces();
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "proc helper {} {pool from_helper}\nproc dormant {} {pool dormant}\nwhen HTTP_REQUEST {call helper; pool direct; snatpool source_snat}";
        let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        assert_eq!(inventory.events().len(), 1);
        assert!(inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"from_helper")
        }));
        assert!(!inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"dormant")
        }));
        assert!(
            inventory
                .commands()
                .iter()
                .all(|command| !command.obligations().is_empty())
        );
        assert_eq!(inventory.events()[0].event(), "HTTP_REQUEST");
        for source in [
            "proc when args {}; when HTTP_REQUEST {pool bad}",
            "proc pool args {}; when HTTP_REQUEST {pool bad}",
            "proc ::pool args {}; when HTTP_REQUEST {::pool bad}",
        ] {
            let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
            assert!(
                !inventory
                    .commands()
                    .iter()
                    .any(|command| command.words().command() == "pool"),
                "{source}: a genuine source replacement blocks catalogue advice"
            );
        }
    }
    #[test]
    fn original_irules_unavailable_rename_retains_only_conditional_source_advice() {
        crate::support::install_surfaces();
        // BIG-IP's separate literal-head loader observation rejects rename:
        // docs/design/analysis/name-resolution-proofs/bigip-literal-head-loader-boundary.md
        // No selected TMM move descriptor means no known source tombstone.
        // The remaining source path grants no accepted rule or event execution.
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "rename pool moved; when HTTP_REQUEST {pool conditional}";
        let inventory = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let pool = inventory
            .commands()
            .iter()
            .find(|command| command.words().command() == "pool")
            .expect("conditional source schema remains separate from loader admission");
        let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
            metadata,
        ) = pool.words().source()
        else {
            panic!("hosted advice must retain its authentic source issuer");
        };
        assert!(metadata.authored_barriers().has_unknown_transitions());
        assert!(
            pool.obligations()
                .contains(&IrulesSourceObligation::ConditionalApplicability)
        );
    }

    fn reference_only_script(
        _arguments: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }

    #[test]
    fn original_irules_source_paths_keep_reference_only_bodies_as_syntax() {
        crate::support::install_surfaces();
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // Same whole source and Registry role; only selected timing changes.
        // Neither branch supplies event execution or native body entry.
        let profile = tcl_dialect::DialectProfile::irules();
        let source = "when HTTP_REQUEST {source-body {pool nested}; pool direct}";
        for reference_only in [true, false] {
            let mut registry = CommandRegistry::build_default().project_for_profile(profile);
            registry.insert(tcl_registry::CommandSpec {
                name: "source-body",
                arity: tcl_registry::Arity::exact(1),
                arg_roles: &[(0, tcl_registry::ArgRole::Body)],
                script_timing_resolver: reference_only.then_some(reference_only_script),
                ..tcl_registry::CommandSpec::DEFAULT
            });
            let inventory = OriginalIrulesSourceContext::capture(source, &registry).unwrap();
            let is_pool = |words: &OriginalRegistryWords, name: &[u8]| {
                words.command() == "pool"
                    && words
                        .arguments()
                        .first()
                        .and_then(|word| word.literal_bytes())
                        == Some(name)
            };
            assert!(
                inventory
                    .source_vectors()
                    .iter()
                    .any(|(_, words)| is_pool(words, b"nested")),
                "readonly original syntax remains independently visible"
            );
            assert!(
                inventory
                    .commands()
                    .iter()
                    .any(|command| is_pool(command.words(), b"direct"))
            );
            assert_eq!(
                inventory
                    .commands()
                    .iter()
                    .any(|command| is_pool(command.words(), b"nested")),
                !reference_only,
                "{reference_only}: only potential evaluation supplies an event-rooted path"
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn configured_event_roots_preserve_literal_body_values_and_event_availability() {
        crate::support::install_surfaces();
        let source = concat!(
            "when HTTP_REQUEST {pool \"a\\\"b\"; pool {茶}; pool $dynamic}\n",
            "when BOGUS_EVENT {pool unknown}\n",
            "when XML_BEGIN_DOCUMENT {pool retired}\n",
        );
        let environment = tcl_registry::model::ingress::resolve_environment("f5-irules");
        let profile = tcl_dialect::DialectProfile::irules();
        for (version, includes_retired_event) in [("9.4.0", true), ("16.1.0", false)] {
            let keyed = tcl_registry::model::KeyedVersions {
                bigip: Some(tcl_dialect::model::Version::parse(version).unwrap()),
                ..Default::default()
            };
            let context = environment.plain_context_registry(&keyed);
            let registry = context.commands();
            let config = LexerConfig {
                braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
                ..LexerConfig::for_profile(Some(profile))
            };
            let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
            let unit = Arc::new(CompilationUnit::build_with_context_registry(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                context.clone(),
            ));
            let mut analyser = Analyser::new().with_resolved_input(input);
            analyser.set_cu_override(unit.clone());
            let analysis = analyser.analyse(source, profile.name);
            assert!(
                unit.ir_module
                    .irules_event_bodies
                    .values()
                    .any(|event| event.event() == "HTTP_REQUEST"),
                "{version}: the actual lowering producer retains the HTTP declaration"
            );
            assert!(
                unit.ir_module.procedures.iter().any(|(id, procedure)| {
                    tcl_compiler::source_graph::event_body_for_procedure(
                        &unit.ir_module,
                        id,
                        procedure,
                        registry,
                    )
                    .is_some_and(|event| event.event() == "HTTP_REQUEST")
                }),
                "{version}: the original HTTP declaration still owns its current lowered procedure"
            );
            let retained = OriginalIrulesSourceContext::from_source_analysis(source, &analysis)
                .expect("public original-source ingress retains the configured declarations");
            let (_, declaration) = retained
                .source_vectors()
                .iter()
                .find(|(span, _)| span.start() == 0)
                .expect(
                    "the original configured HTTP declaration retains its complete source argv",
                );
            assert_eq!(
                declaration.with_source_schema(&context, |schema| schema
                    .semantics
                    .traits
                    .contains(Traits::IS_EVENT_HANDLER)),
                Some(true),
                "{version}: the actual original declaration retains its selected source schema"
            );
            assert_eq!(
                declaration.roles(),
                Some([(1, tcl_registry::ArgRole::Body)].as_slice()),
                "{version}: the selected source schema retains the actual original body ordinal"
            );
            let bodies = declaration.source_script_bodies_for(
                &context,
                tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            );
            assert_eq!(
                bodies.len(),
                1,
                "{version}: the original declaration retains exactly one potential body source"
            );
            assert_eq!(
                source.get(bodies[0].content_span().as_range()),
                Some(r#"pool "a\"b"; pool {茶}; pool $dynamic"#),
                "{version}: the body owner retains the exact written original content"
            );
            let inventory = OriginalIrulesSourceContext::from_analysis(
                source,
                &analysis,
                &unit.ir_module,
                &context,
            )
            .unwrap();
            let http_event = inventory
                .events()
                .iter()
                .find(|event| event.event() == "HTTP_REQUEST")
                .expect("the original configured HTTP_REQUEST declaration retains its body owner");
            let image = SourceImage::document(source);
            assert!(http_event.matches_source(&image, config));
            let event_word = http_event
                .event_word(&image, config, registry)
                .expect("the original Hosted event operand retains its own source naming purpose");
            assert_eq!(
                source.get(event_word.span().as_range()),
                Some("HTTP_REQUEST")
            );
            assert_eq!(event_word.config(), config);
            assert!(
                http_event
                    .event_word(
                        &SourceImage::document(&format!("#{source}")),
                        config,
                        registry
                    )
                    .is_none()
            );
            let changed = LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            };
            assert!(http_event.event_word(&image, changed, registry).is_none());
            let http_commands = inventory
                .commands()
                .iter()
                .filter(|command| command.event_source() == http_event)
                .collect::<Vec<_>>();
            assert_eq!(
                http_commands.len(),
                3,
                "{version}: every original HTTP body argv remains source advice"
            );
            for (command, expected) in
                http_commands
                    .iter()
                    .zip([Some("a\"b".as_bytes()), Some("茶".as_bytes()), None])
            {
                let operand = command.words().original_argument_word(0).unwrap();
                assert_eq!(
                    tcl_syntax::word_rules::original_static_word_unicode_value(operand).as_deref(),
                    expected,
                    "{version}: complete original operand {}",
                    command.span().start()
                );
                assert_eq!(
                    command.words().with_source_schema(&context, |schema| {
                        tcl_irules::original_source_object_operands(schema, None).len()
                    }),
                    Some(1),
                    "{version}: original object slot {}",
                    command.span().start()
                );
            }
            let references = tcl_irules::extract_original_irules_source_object_references(
                source, &inventory, None,
            );
            let names = references
                .iter()
                .map(|reference| reference.name.as_str())
                .collect::<Vec<_>>();
            assert!(
                names.contains(&"a\"b") && names.contains(&"茶"),
                "{version}: {names:?}"
            );
            assert!(
                !names.contains(&"unknown") && !names.contains(&"$dynamic"),
                "{version}: {names:?}"
            );
            assert_eq!(
                names.contains(&"retired"),
                includes_retired_event,
                "{version}: {names:?}"
            );
            assert!(
                !inventory
                    .events()
                    .iter()
                    .any(|event| event.event() == "BOGUS_EVENT")
            );
            assert_eq!(
                tcl_irules::extract_original_irules_source_object_references(
                    &format!("#changed\n{source}"),
                    &inventory,
                    None
                ),
                [] as [tcl_irules::IrulesObjectReference; 0]
            );
        }
    }

    #[test]
    fn original_irules_called_body_uses_complete_source_geometry_and_not_proc_reports() {
        crate::support::install_surfaces();
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let profile = tcl_dialect::DialectProfile::irules();
        let context = Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let config = LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let source = "proc helper {} {pool first; pool final}\nproc dormant {} {pool dormant}\nwhen HTTP_REQUEST {call helper}";
        let unit = Arc::new(CompilationUnit::build_with_context_registry(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            context.clone(),
        ));
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(unit.clone());
        let mut analysis = analyser.analyse(source, profile.name);
        analysis.all_procs.clear();
        let inventory = OriginalIrulesSourceContext::from_analysis(
            source,
            &analysis,
            &unit.ir_module,
            &context,
        )
        .unwrap();
        for operand in [b"first".as_slice(), b"final".as_slice()] {
            let command = inventory
                .commands()
                .iter()
                .find(|command| {
                    command
                        .words()
                        .arguments()
                        .first()
                        .and_then(|word| word.literal_bytes())
                        == Some(operand)
                })
                .expect("complete genuine called source body");
            assert!(
                command
                    .obligations()
                    .contains(&IrulesSourceObligation::ProcedureBindingUnavailable)
            );
            assert_eq!(command.event_source().event(), "HTTP_REQUEST");
        }
        assert!(!inventory.commands().iter().any(|command| {
            command
                .words()
                .arguments()
                .first()
                .and_then(|word| word.literal_bytes())
                == Some(b"dormant")
        }));
    }
}
