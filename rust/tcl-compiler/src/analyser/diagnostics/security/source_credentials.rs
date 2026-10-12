// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Existing credential advice from selected metadata or separate lexical policy.

use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::AuthoredSourceCredentialPosition;

fn selected_credential(original: &OriginalDiagnosticInvocation) -> Option<Diagnostic> {
    let arguments = original.with_schema(|schema| schema.authored_source_credential_arguments())?;
    for argument in arguments {
        let Some(word) = original.word(argument.argument) else {
            continue;
        };
        // Only an original static spelling is hardcoded here. A separate CU
        // value premise cannot turn a dynamic word into a literal in this file.
        let Some(value) = tcl_syntax::word_rules::original_static_word_source_bytes(word) else {
            continue;
        };
        if std::str::from_utf8(&value).is_err() {
            continue;
        }
        let message = match argument.position {
            AuthoredSourceCredentialPosition::Option(option) => format!(
                "Hardcoded credential in {option} argument. Store secrets in \
environment variables or a vault, not in source code."
            ),
            AuthoredSourceCredentialPosition::Header(header) => {
                let Some(name) = original.literal(header) else {
                    continue;
                };
                format!(
                    "Hardcoded credential in {} header value. \
Store secrets in environment variables or a vault, not in source code.",
                    name.to_ascii_lowercase()
                )
            }
        };
        let subject = original.subject(
            RegistrySourceDiagnosticKind::CredentialLiteral,
            Some(argument.argument),
        )?;
        return Some(
            Diagnostic::new(DiagCode::W310, word.span(), message, Severity::Warning)
                .with_subject(subject),
        );
    }
    None
}

/// Command-independent written option spellings are lexical advice. They do
/// not select a command descriptor, alias target or captured operand geometry.
fn lexical_credential(args: &[String], tokens: &[tcl_lexer::Token]) -> Option<Diagnostic> {
    for (index, option) in args.iter().enumerate() {
        let lower = option.to_ascii_lowercase();
        if !tcl_registry::spec::DEFAULT_CREDENTIAL_OPTION_NAMES.contains(&lower.as_str()) {
            continue;
        }
        let (Some(value), Some(token)) = (args.get(index + 1), tokens.get(index + 1)) else {
            continue;
        };
        if super::is_literal_credential_value(value, token) {
            return Some(Diagnostic::new(
                DiagCode::W310,
                token.span,
                format!(
                    "Hardcoded credential in {option} argument. Store secrets in \
environment variables or a vault, not in source code."
                ),
                Severity::Warning,
            ));
        }
    }
    None
}

impl Analyser {
    /// W310 uses selected credential metadata without borrowing a nominal
    /// command table. Default written flags retain their independent policy.
    pub(in crate::analyser) fn emit_w310_hardcoded_credentials(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let selected = original
            .filter(|original| original.matches_analysis(&self.result, &self.source))
            .and_then(selected_credential);
        if let Some(diagnostic) = selected.or_else(|| lexical_credential(args, arg_tokens)) {
            self.result.diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{AnalysisResult, DiagnosticSubject, ResolvedAnalysisInput};
    use std::sync::Arc;

    fn input(context: Arc<tcl_registry::model::ContextRegistry>) -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        input
    }
    fn current_input() -> ResolvedAnalysisInput {
        input(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        )
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser
    }
    fn findings(result: &AnalysisResult) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W310)
            .collect()
    }
    fn credential_input() -> ResolvedAnalysisInput {
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        registry.insert(tcl_registry::CommandSpec {
            name: "credential_current",
            surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
            arity: tcl_registry::Arity::at_least(0),
            options: Box::leak(
                vec![tcl_registry::hover::OptionSpec {
                    name: "-headers",
                    value: tcl_registry::hover::OptionValue::value("value"),
                    ..tcl_registry::hover::OptionSpec::DEFAULT
                }]
                .into_boxed_slice(),
            ),
            credential_options: &["-headers"],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "header_current",
            surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
            subcommands: Box::leak(
                vec![tcl_registry::SubCommand {
                    name: "insert",
                    arity: tcl_registry::Arity::exact(2),
                    credential_arg: Some(2),
                    sensitive_headers: &["authorization"],
                    ..tcl_registry::SubCommand::DEFAULT
                }]
                .into_boxed_slice(),
            ),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        input(Arc::new(baseline.with_command_store(Arc::new(registry))))
    }

    #[test]
    fn original_credential_metadata_keeps_alias_ordinals_and_original_literal_spans() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = credential_input();
        for (source, value, argument, written) in [
            (
                "credential_current -headers {é[$literal]}",
                "{é[$literal]}",
                1,
                Some(1),
            ),
            (
                "interp alias {} take {} credential_current -headers; take {é[$literal]}",
                "{é[$literal]}",
                1,
                Some(0),
            ),
            (
                "interp alias {} take {} credential_current -headers {é[$literal]}; rename take moved; moved",
                "{é[$literal]}",
                1,
                None,
            ),
            (
                "rename credential_current Held; Held -he VALUE",
                "VALUE",
                1,
                Some(1),
            ),
            (
                "header_current insert AUTHORIZATION {é[$literal]}",
                "{é[$literal]}",
                2,
                Some(2),
            ),
            (
                "interp alias {} insert {} header_current insert authorization; insert {é[$literal]}",
                "{é[$literal]}",
                2,
                Some(0),
            ),
            (
                "interp alias {} insert {} header_current insert authorization {é[$literal]}; insert",
                "{é[$literal]}",
                2,
                None,
            ),
        ] {
            let analyser = analyse(source, &input);
            let found = findings(&analyser.result);
            assert_eq!(
                found.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(&source[found[0].span.as_range()], value);
            assert!(found[0].fixes.is_empty());
            let Some(DiagnosticSubject::RegistrySource(subject)) = found[0].subject() else {
                panic!("original credential subject")
            };
            assert_eq!(
                subject.kind(),
                RegistrySourceDiagnosticKind::CredentialLiteral
            );
            assert_eq!(subject.argument(), Some(argument));
            assert_eq!(subject.written_argument(), written);
            assert!(subject.supplemental_literals().is_empty());
        }
    }

    #[test]
    fn original_credential_metadata_refuses_replaced_targets_and_unknown_option_windows() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = credential_input();
        for source in [
            "proc credential_current args {return CUSTOM}; credential_current -headers VALUE",
            "namespace eval custom {proc credential_current args {return CUSTOM}}; custom::credential_current -headers VALUE",
            "interp alias {} take {} credential_current -headers; rename credential_current Held; take VALUE",
            "interp alias {} take {} credential_current -headers; rename credential_current {}; take VALUE",
            "credential_current $option -headers VALUE",
            "credential_current {*}$options -headers VALUE",
            "credential_current -unknown -headers VALUE",
            "credential_current -headers $value",
            "credential_current -headers prefix$value",
            "credential_current -headers [produce]",
            "header_current $selector authorization VALUE",
            "header_current insert $header VALUE",
            "header_current insert other VALUE",
            "header_current insert authorization $value",
            "header_current {*}$args insert authorization VALUE",
        ] {
            assert!(
                findings(&analyse(source, &input).result).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_credential_metadata_keeps_actual_availability_and_source_correspondence() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = credential_input();
        let current = input.context_registry();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        let source = "credential_current -headers VALUE";
        assert_eq!(findings(&analyse(source, &input).result).len(), 1);
        assert!(findings(&analyse(source, &self::input(older)).result).is_empty());
        let mut analyser = analyse(source, &input);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let original = analyser
            .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
            .unwrap();
        for axis in 0..4 {
            let mut changed = analyser.result.clone();
            match axis {
                0 => changed.resolved_input = None,
                1 => changed.resolved_input = Some(current_input()),
                2 => changed.body_lexer_config = None,
                _ => {
                    let mut config = input.lexer_config();
                    config.strict_quoting = !config.strict_quoting;
                    changed.resolved_input = Some(ResolvedAnalysisInput::new(
                        input.unit_profile(),
                        input.unit_profile(),
                        input.context_registry(),
                        config,
                    ));
                }
            }
            assert!(
                !original.matches_analysis(&changed, source),
                "owner axis {axis}"
            );
        }
        assert!(!original.matches_analysis(&analyser.result, "credential_current -headers OTHER"));
        analyser.result.resolved_input = None;
        analyser.result.diagnostics.clear();
        analyser.emit_w310_hardcoded_credentials(
            Some(&original),
            segment.args(),
            segment.arg_tokens(),
        );
        assert!(findings(&analyser.result).is_empty());
    }

    #[test]
    fn lexical_credential_defaults_remain_independent_of_registry_selection() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "mycmd -Password VALUE",
            "proc set args {}; set -secret VALUE",
        ] {
            let analyser = analyse(source, &input);
            let found = findings(&analyser.result);
            assert_eq!(found.len(), 1, "{source}");
            assert!(
                found[0].subject().is_none(),
                "lexical policy supplies no selected command proof"
            );
        }
        assert!(findings(&analyse("mycmd -password $value", &input).result).is_empty());
        assert!(findings(&analyse("mycmd -token [produce]", &input).result).is_empty());
    }
}
