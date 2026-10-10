// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Existing frame/interpreter advice over original selected source operands.

use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::{Traits, arg_role::ArgRole};

/// Original possible script/command positions. The selected metadata and option
/// horizon describe source advice; they establish no target interpreter or frame.
fn interpreter_script_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<(&'static str, Vec<usize>, bool)> {
    let selected = schema.authored_source_descriptors();
    let member = selected.subcommand?;
    if !selected
        .command
        .taint_interp_eval_subcommands
        .contains(&member.name)
    {
        return None;
    }
    let count = schema.words.arguments().exact_argv_len()?;
    let (roles, complete) = schema.authored_source_argument_roles();
    // An unknown payload can leave the generic role resolver incomplete.
    // The selected option grammar below independently retains its fixed
    // prefix, widths and original possible command boundary.
    let bodies = if complete {
        roles
            .into_iter()
            .filter_map(|(index, role)| {
                (role == ArgRole::Body)
                    .then_some(schema.semantics.argument_offset + usize::from(index))
            })
            .filter(|index| *index < count)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if let Some(first) = bodies.first().copied() {
        let concatenates = schema
            .semantics
            .traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS);
        return Some((
            member.name,
            if concatenates {
                (first..count).collect()
            } else {
                bodies
            },
            concatenates,
        ));
    }
    let scan = schema.authored_source_diagnostic_options()?;
    if scan
        .options
        .iter()
        .any(|option| !option.available || option.values.is_none())
    {
        return None;
    }
    use tcl_registry::resolved_invocation::AuthoredSourceOptionBoundary as Boundary;
    let command = match scan.boundary {
        // A computed boundary retains the possibility of a command operand;
        // it does not prove that an earlier option loop accepts this value.
        Boundary::Positional(index) | Boundary::Dynamic(index) => index,
        Boundary::Terminator(index) => index.checked_add(1)?,
        Boundary::Unknown(_)
        | Boundary::Ambiguous { .. }
        | Boundary::End
        | Boundary::Indeterminate => return None,
    };
    (command < count).then_some((member.name, vec![command], false))
}

impl Analyser {
    pub(in crate::analyser) fn emit_w301_uplevel_injection(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let logical = self.result.allows_retained_logical_declaration_advice();
        let Some(ranges) = original
            .with_schema(|schema| {
                schema
                    .semantics
                    .traits
                    .contains(
                        Traits::SCRIPT_CONCATENATES_ARGS
                            | Traits::TAINT_SINK
                            | Traits::EVALUATES_IN_SHIFTED_FRAME,
                    )
                    .then(|| {
                        if logical {
                            schema.authored_logical_source_frame_script_arguments()
                        } else {
                            schema.authored_source_frame_script_arguments()
                        }
                    })
                    .flatten()
            })
            .flatten()
        else {
            return;
        };
        for mut arguments in ranges {
            let multiple = arguments.len() > 1;
            let Some((argument, word)) = arguments.find_map(|argument| {
                let word = original.word(argument)?;
                super::super::usage::original_word_has_substitution(word)
                    .then_some((argument, word))
            }) else {
                continue;
            };
            if !multiple
                && (self.original_canonical_list_substitution(word)
                    || matches!(word.tokens(), [token] if token.kind == tcl_lexer::TokenType::Var))
            {
                continue;
            }
            let Some(subject) =
                original.subject(RegistrySourceDiagnosticKind::ScriptReparse, Some(argument))
            else {
                continue;
            };
            let command = original.command();
            let message = if multiple {
                format!(
                    "{command} with multiple arguments concatenates them into a script (like eval). Use a single braced body or {{*}}$cmdList to avoid injection."
                )
            } else {
                format!(
                    "{command} with an unbraced script argument may cause double substitution. Use braces: {command} 1 {{...}}"
                )
            };
            self.result.diagnostics.push(
                Diagnostic::new(DiagCode::W301, word.span(), message, Severity::Warning)
                    .with_subject(subject),
            );
            break;
        }
    }

    pub(in crate::analyser) fn emit_w312_interp_eval_injection(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let Some((member, arguments, concatenates)) =
            original.with_schema(interpreter_script_arguments).flatten()
        else {
            return;
        };
        let multiple = concatenates && arguments.len() > 1;
        let Some((argument, word)) = arguments.into_iter().find_map(|argument| {
            let word = original.word(argument)?;
            super::super::usage::original_word_has_substitution(word).then_some((argument, word))
        }) else {
            return;
        };
        if !multiple && self.original_canonical_list_substitution(word) {
            return;
        }
        let Some(subject) =
            original.subject(RegistrySourceDiagnosticKind::ScriptReparse, Some(argument))
        else {
            return;
        };
        let command = original.command();
        let message = if multiple {
            format!(
                "{command} {member} with multiple arguments concatenates them into a script (like eval). Use a single braced body to avoid injection."
            )
        } else {
            format!(
                "{command} {member} with an unbraced script argument may cause code injection. Use braces: {command} {member} $child {{...}}"
            )
        };
        self.result.diagnostics.push(
            Diagnostic::new(DiagCode::W312, word.span(), message, Severity::Warning)
                .with_subject(subject),
        );
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
    fn findings(result: &AnalysisResult, code: DiagCode) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .collect()
    }

    #[test]
    fn original_frame_reparse_keeps_possible_layouts_alias_prefixes_and_whole_words() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "uplevel 1 \"pré $value\"",
            "uplevel $first $second",
            "rename uplevel Held; Held 1 pre$value",
            "interp alias {} run {} uplevel 1; run \"pré $value\"",
        ] {
            let analyser = analyse(source, &input);
            let diagnostics = findings(&analyser.result, DiagCode::W301);
            assert_eq!(
                diagnostics.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("source frame subject")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::ScriptReparse);
            assert_eq!(
                subject.argument(),
                Some(if source.contains("$first") { 0 } else { 1 })
            );
            if source.starts_with("interp alias") {
                assert_eq!(subject.written_argument(), Some(0));
                assert_eq!(&source[diagnostics[0].span.as_range()], "\"pré $value\"");
            }
        }
        for source in [
            "uplevel 1 $body",
            "uplevel 1 {set x $value}",
            "interp alias {} make {} list set x; uplevel 1 [make $value]",
            "proc uplevel args {return CUSTOM}; uplevel 1 pre$value",
            "namespace eval custom {proc uplevel args {return CUSTOM}}; ::custom::uplevel 1 pre$value",
            "interp alias {} run {} uplevel 1; rename uplevel Held; run pre$value",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W301).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_interpreter_reparse_uses_selected_script_words_without_path_or_value_drift() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for (source, argument, written, text) in [
            ("interp ev $child \"pré $script\"", 2, 2, "\"pré $script\""),
            (
                "interp alias {} run {} interp eval {}; run $script",
                2,
                0,
                "$script",
            ),
            (
                "interp alias {} run {} interp invokehidden {} -namespace ::N; run $command ARG",
                4,
                0,
                "$command",
            ),
            (
                "interp invokehidden $child -namespace $namespace $command ARG",
                4,
                4,
                "$command",
            ),
            ("interp invokehidden $child -- $command", 3, 3, "$command"),
            (
                "rename interp Held; Held eval $child $script",
                2,
                2,
                "$script",
            ),
        ] {
            let analyser = analyse(source, &input);
            let diagnostics = findings(&analyser.result, DiagCode::W312);
            assert_eq!(
                diagnostics.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(&source[diagnostics[0].span.as_range()], text);
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("source interpreter subject")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::ScriptReparse);
            assert_eq!(subject.argument(), Some(argument));
            assert_eq!(subject.written_argument(), Some(written));
        }
        for source in [
            "interp eval $child literal tail",
            "interp eval $child {set x $value}",
            "interp invokehidden $child -namespace $namespace literal $arg",
            "interp invokehidden $child -namespace",
            "interp invokehidden $child -unknown $command",
            "interp eval $child [list set x $value]",
            "interp alias {} run {} interp eval {}; proc interp args {return CUSTOM}; run $script",
            "interp alias {} run {} interp eval {}; rename interp Held; run $script",
            "namespace eval custom {proc interp args {return CUSTOM}}; ::custom::interp eval $child $script",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W312).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_crossing_advice_keeps_selected_option_availability_and_missing_owner_refusals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = current_input();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.context_registry().commands())),
        );
        assert!(Arc::ptr_eq(
            current.context_registry().commands(),
            older.commands()
        ));
        let source = "interp invokehidden $child -namespace ::N $command";
        assert_eq!(
            findings(&analyse(source, &current).result, DiagCode::W312).len(),
            1
        );
        assert!(findings(&analyse(source, &input(older)).result, DiagCode::W312).is_empty());
        let source = "uplevel 1 \"pré $value\"; interp eval $child $script";
        let analyser = analyse(source, &current);
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            current.lexer_config(),
        );
        let originals = segments
            .iter()
            .map(|segment| {
                analyser
                    .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(findings(&analyser.result, DiagCode::W301).len(), 1);
        assert_eq!(findings(&analyser.result, DiagCode::W312).len(), 1);
        for axis in 0..4 {
            let mut changed = analyse(source, &current);
            changed.result.diagnostics.clear();
            match axis {
                0 => changed.result.resolved_input = None,
                1 => {
                    let mut stale = current.clone();
                    stale.config.strict_quoting = !stale.config.strict_quoting;
                    changed.result.resolved_input = Some(stale);
                }
                2 => {
                    changed.source =
                        "uplevel 1 \"pré $other\"; interp eval $child $other".to_owned()
                }
                _ => {
                    changed.result.resolved_input = Some(input(
                        tcl_registry::model::ingress::resolve_environment("tcl9.0")
                            .default_context_registry(),
                    ))
                }
            }
            for original in &originals {
                changed.emit_w301_uplevel_injection(Some(original));
                changed.emit_w312_interp_eval_injection(Some(original));
            }
            assert!(
                findings(&changed.result, DiagCode::W301).is_empty(),
                "owner axis {axis}"
            );
            assert!(
                findings(&changed.result, DiagCode::W312).is_empty(),
                "owner axis {axis}"
            );
        }
    }
}
