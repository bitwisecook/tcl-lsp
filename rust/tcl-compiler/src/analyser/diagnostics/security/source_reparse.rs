// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source reparse advice over original selected outer and nested calls.

use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::Traits;

impl Analyser {
    /// The single child retains its original lookup horizon and selected
    /// return descriptor. This source idiom supplies no measured result value.
    pub(super) fn original_canonical_list_substitution(
        &self,
        word: &tcl_lexer::NativeWord,
    ) -> bool {
        self.original_substitution_call(word).is_some_and(|child| {
            child.with_schema(|schema| schema.return_type() == Some(tcl_registry::TclType::List))
                == Some(true)
        })
    }

    /// The selected descriptor owns concatenation; the original word arena
    /// owns lexical substitutions, including quoted braces and captured words.
    pub(in crate::analyser) fn emit_w101_eval_string_concat(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source)
            || original.with_schema(|schema| {
                schema
                    .semantics
                    .traits
                    .contains(Traits::SCRIPT_CONCATENATES_ARGS | Traits::TAINT_SINK)
                    && !schema
                        .semantics
                        .traits
                        .contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
                    && !schema
                        .authored_source_descriptors()
                        .subcommand
                        .is_some_and(|selected| {
                            schema
                                .authored_source_descriptors()
                                .command
                                .taint_interp_eval_subcommands
                                .contains(&selected.name)
                        })
            }) != Some(true)
        {
            return;
        }
        let count = original.words().arguments().len();
        if count == 1
            && original
                .word(0)
                .is_some_and(|word| self.original_canonical_list_substitution(word))
        {
            return;
        }
        let Some((argument, word)) = (0..count).find_map(|argument| {
            let word = original.word(argument)?;
            super::super::usage::original_word_has_substitution(word).then_some((argument, word))
        }) else {
            return;
        };
        let Some(subject) =
            original.subject(RegistrySourceDiagnosticKind::ScriptReparse, Some(argument))
        else {
            return;
        };
        let command = original.command();
        let fixes = self.original_eval_list_fix(original, argument);
        self.result.diagnostics.push(Diagnostic::new(
            DiagCode::W101,
            word.span(),
            format!("{command} with substituted arguments risks code injection. Prefer direct invocation or {{*}}$cmdList to preserve argument boundaries."),
            Severity::Warning,
        ).with_subject(subject).with_fixes(fixes));
    }

    /// A new list command is unwritten source. Its binding and intended change
    /// in reparsing require review independently of the original word geometry.
    fn original_eval_list_fix(
        &self,
        original: &OriginalDiagnosticInvocation,
        argument: usize,
    ) -> Vec<crate::analyser::CodeFix> {
        let Some(word) = original.word(argument) else {
            return Vec::new();
        };
        if original.written_index(argument).is_none()
            || word.group().kind != tcl_lexer::WordKind::Quoted
            || word.image().bytes() != self.source.as_bytes()
        {
            return Vec::new();
        }
        let Some(inner) = word
            .content_span()
            .ok()
            .and_then(|span| self.source.get(span.as_range()))
        else {
            return Vec::new();
        };
        let arena = word.executable_parts();
        let changes_literal_quoting = arena.list(arena.root()).iter().any(|part| {
            arena
                .text(part)
                .is_some_and(|text| text.iter().any(|byte| matches!(byte, b'{' | b'}' | b'"')))
        });
        if inner.is_empty()
            || inner.contains('\n')
            || inner.contains('\\')
            || changes_literal_quoting
        {
            return Vec::new();
        }
        vec![crate::analyser::CodeFix {
            span: word.span(),
            new_text: format!("[list {inner}]"),
            description: "Rewrite as a list; review the list command binding and intended argument boundaries".to_owned(),
            safety: crate::analyser::FixSafety::RequiresReview,
        }]
    }

    /// Both reparse and substitution behavior come from actual selected source
    /// descriptors. Raw bracket text and reporting heads cannot choose either.
    pub(in crate::analyser) fn emit_w309_eval_subst_double_decode(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source)
            || original.with_schema(|schema| {
                schema
                    .semantics
                    .traits
                    .contains(Traits::EVALUATES_CODE | Traits::TAINT_SINK)
            }) != Some(true)
        {
            return;
        }
        for argument in 0..original.words().arguments().len() {
            let Some(word) = original.word(argument) else {
                continue;
            };
            let Some(child) = self.original_substitution_call(word) else {
                continue;
            };
            if child.with_schema(|schema| schema.substitutions_performed().is_some()) != Some(true)
            {
                continue;
            }
            let Some(subject) =
                original.subject(RegistrySourceDiagnosticKind::ScriptReparse, Some(argument))
            else {
                continue;
            };
            let command = original.command();
            let inner = child.command();
            self.result.diagnostics.push(Diagnostic::new(
                DiagCode::W309,
                word.span(),
                format!("{command} with [{inner}] creates double substitution: {inner} expands $var and [cmd], then {command} re-parses the result as Tcl. This is a code-injection risk. Use [format] or [string map] for safe templating."),
                Severity::Error,
            ).with_subject(subject));
            break;
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
        analyser.analyse(source, "tcl");
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
    fn original_reparse_diagnostics_keep_alias_targets_and_known_source_barriers() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        // Original source descriptor advice, not a result or entered worker.
        let input = current_input();
        for source in [
            "::eval $script",
            "rename eval Held; Held $script",
            "interp alias {} run {} eval; run $script",
            "interp alias {} run {} eval {set local}; run $script",
        ] {
            let analyser = analyse(source, &input);
            let diagnostics = findings(&analyser.result, DiagCode::W101);
            assert_eq!(
                diagnostics.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("original reparse subject")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::ScriptReparse);
            assert_eq!(subject.written_argument(), Some(0));
            if source.contains("{set local}") {
                assert_eq!(subject.argument(), Some(1));
            }
            assert_eq!(&source[diagnostics[0].span.as_range()], "$script");
        }
        for source in [
            "proc eval args {return CUSTOM}; eval $script",
            "rename eval {}; eval $script",
            "namespace eval custom {proc eval args {return CUSTOM}}; ::custom::eval $script",
            "interp alias {} run {} eval; rename eval Held; run $script",
            "interp eval $child $script",
            "uplevel 1 $script",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W101).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_list_idiom_uses_the_child_schema_and_retains_concat_and_shadow_warnings() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "eval [list set x $value]",
            "interp alias {} make {} list set x; eval [make $value]",
            "rename list Held; eval [Held set x $value]",
            "eval [{list} set x $value]",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W101).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
        for source in [
            "eval [concat set x $value]",
            "interp alias {} join {} concat; eval [join set x $value]",
            "proc list args {return CUSTOM}; eval [list set x $value]",
            "interp alias {} make {} list; proc list args {return CUSTOM}; eval [make set x $value]",
            "eval [list SAFE; set local $value]",
        ] {
            let analyser = analyse(source, &input);
            assert_eq!(
                findings(&analyser.result, DiagCode::W101).len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_reparse_fix_keeps_whole_quoted_unicode_span_and_review_premise() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        let source = "interp alias {} run {} eval; run \"pré $value\"";
        let analyser = analyse(source, &input);
        let diagnostic = findings(&analyser.result, DiagCode::W101)[0];
        assert_eq!(&source[diagnostic.span.as_range()], "\"pré $value\"");
        let [fix] = diagnostic.fixes.as_slice() else {
            panic!("one source rewrite suggestion")
        };
        assert_eq!(fix.span, diagnostic.span);
        assert_eq!(fix.new_text, "[list pré $value]");
        assert_eq!(fix.safety, crate::analyser::FixSafety::RequiresReview);
        let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostic.subject() else {
            panic!("original quoted subject")
        };
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(subject.written_argument(), Some(0));
        let quoting = analyse("eval \"pré {$value}\"", &input);
        let quoted = findings(&quoting.result, DiagCode::W101);
        assert_eq!(
            quoted.len(),
            1,
            "literal braces in a quoted word do not stop substitution"
        );
        assert!(
            quoted[0].fixes.is_empty(),
            "the proposal cannot reuse quoted literal braces as script delimiters"
        );
        for source in [r"eval pré\$value", "eval {pré $value}", "eval pré{literal}"] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W101).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_nested_reparse_keeps_both_aliases_and_declines_replaced_child_or_parent() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "eval [::subst $tmpl]",
            "rename subst Held; eval [Held $tmpl]",
            "interp alias {} run {} eval {set local}; interp alias {} render {} subst -nocommands; run [render $tmpl]",
            "uplevel 1 [subst $tmpl]",
        ] {
            let analyser = analyse(source, &input);
            let diagnostics = findings(&analyser.result, DiagCode::W309);
            assert_eq!(
                diagnostics.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("nested original subject")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::ScriptReparse);
            assert_eq!(
                subject.written_argument(),
                Some(if source.starts_with("uplevel") { 1 } else { 0 })
            );
            if source.contains("{set local}") {
                assert_eq!(subject.argument(), Some(1));
            }
            assert!(source[diagnostics[0].span.as_range()].starts_with('['));
        }
        for source in [
            "proc subst args {return CUSTOM}; eval [subst $tmpl]",
            "proc eval args {return CUSTOM}; eval [subst $tmpl]",
            "interp alias {} render {} subst; rename subst Held; eval [render $tmpl]",
            "eval [list subst $tmpl]",
            "eval [subst $tmpl; list SAFE]",
        ] {
            let analyser = analyse(source, &input);
            assert!(
                findings(&analyser.result, DiagCode::W309).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_reparse_selection_retains_actual_availability_and_unavailable_owner_refusals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut descriptor = registry.get("eval").unwrap().clone();
        descriptor.name = "run_current";
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(descriptor);
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        let input = input(current);
        let source = "run_current [subst $tmpl]";
        let analyser = analyse(source, &input);
        assert_eq!(findings(&analyser.result, DiagCode::W101).len(), 1);
        assert_eq!(findings(&analyser.result, DiagCode::W309).len(), 1);
        let unavailable = analyse(source, &self::input(older));
        assert!(findings(&unavailable.result, DiagCode::W101).is_empty());
        assert!(findings(&unavailable.result, DiagCode::W309).is_empty());
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let original = analyser
            .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
            .unwrap();
        for axis in 0..4 {
            let mut changed = analyse(source, &input);
            changed.result.diagnostics.clear();
            match axis {
                0 => changed.result.resolved_input = None,
                1 => {
                    let mut stale = input.clone();
                    stale.config.strict_quoting = !stale.config.strict_quoting;
                    changed.result.resolved_input = Some(stale);
                }
                2 => changed.source = "run_current [subst $other]".to_owned(),
                _ => {
                    changed.result.resolved_input = Some(self::input(
                        tcl_registry::model::ingress::resolve_environment("tcl8.6")
                            .default_context_registry(),
                    ))
                }
            }
            changed.emit_w101_eval_string_concat(Some(&original));
            changed.emit_w309_eval_subst_double_decode(Some(&original));
            assert!(
                findings(&changed.result, DiagCode::W101).is_empty(),
                "owner axis {axis}"
            );
            assert!(
                findings(&changed.result, DiagCode::W309).is_empty(),
                "owner axis {axis}"
            );
        }
    }
}
