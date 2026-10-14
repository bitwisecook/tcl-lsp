// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible original body names retain source geometry without entering a frame.

use std::collections::HashSet;
use tcl_lexer::NativeWord;
use tcl_registry::{ArgRole, CommandRegistry, InvocationWord, InvocationWords};

pub(crate) struct OriginalBodyNameCandidates {
    pub(crate) names: HashSet<String>,
    pub(crate) unknown_residual: bool,
}

pub(crate) fn original_authored_body_name_candidates(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<OriginalBodyNameCandidates> {
    let source = super::original_authored_source_structure(tokens, registry)?;
    let bodies = source.with_source_schema(registry, |schema| {
        if schema.semantics.frame_effect.is_some_and(|frame| {
            frame.layout == tcl_registry::FrameArgLayout::ScriptInSelectedFrame
        }) && matches!(
            schema.authored_logical_source_frame_argument_layout(),
            Some(tcl_registry::FrameArgumentResolution::Valid { level, .. })
                if level.is_global_frame()
        ) {
            return Some(Vec::new());
        }
        Some(
            schema
                .authored_source_plain_script_arguments()?
                .into_iter()
                .filter(|&argument| {
                    schema.authored_source_script_timing_at(argument)
                        == Some(tcl_registry::ScriptTiming::SameInvocation)
                })
                .collect::<Vec<_>>(),
        )
    })?;
    if bodies.is_empty() {
        return None;
    }
    let mut result = OriginalBodyNameCandidates {
        names: HashSet::new(),
        unknown_residual: true,
    };
    for argument in bodies {
        if let Some(body) = source.original_argument_word(argument) {
            collect(
                body,
                registry,
                source.dialect(),
                source.realm(),
                0,
                &mut result,
            );
        }
    }
    Some(result)
}

fn collect(
    body: &NativeWord,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
    depth: u32,
    result: &mut OriginalBodyNameCandidates,
) {
    if tcl_core_types::RecursionLimit(32).exceeded(depth) {
        return;
    }
    let Some(span) = super::source_scoped_body::original_written_script_body_span(body) else {
        return;
    };
    let Ok(text) = body.image().try_text() else {
        return;
    };
    let Some(content) = text.get(span.as_range()) else {
        return;
    };
    let mut scanner = crate::var_refs::VarReferenceScanner::with_config(
        crate::var_refs::VarScanOptions::default(),
        body.config(),
    );
    result
        .names
        .extend(scanner.scan_lexical_word(content, registry));
    let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
        body.image().clone(),
        span,
        body.config(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    ) else {
        return;
    };
    for command in plan.commands {
        let original = command.words;
        if original.is_empty() {
            continue;
        }
        let values = original
            .iter()
            .map(tcl_syntax::word_rules::original_static_word_unicode_value)
            .collect::<Vec<_>>();
        let word = |at: usize| {
            if original[at].group().expand {
                return InvocationWord::Expanded;
            }
            values[at]
                .as_deref()
                .map_or(InvocationWord::Dynamic, |bytes| {
                    super::source_structure::source_schema_word(InvocationWord::KnownBytes(bytes))
                })
        };
        let arguments = (1..original.len()).map(word).collect::<Vec<_>>();
        let mut words = InvocationWords::structured(word(0), &arguments);
        if let Some(dialect) = dialect {
            words = words.with_dialect(dialect);
        }
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry, None, words, realm,
            );
        let Some(schema) = selected.resolved() else {
            continue;
        };
        let facts = schema.facts();
        let (roles, _) = schema.authored_source_argument_roles();
        for (ordinal, role) in roles {
            let argument = facts.argument_offset + usize::from(ordinal);
            let Some(value) = values
                .get(argument + 1)
                .and_then(Option::as_deref)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            else {
                continue;
            };
            match role {
                ArgRole::VarRead | ArgRole::VarWrite => {
                    result.names.insert(value.to_owned());
                }
                ArgRole::LoopVarList | ArgRole::StaticVarList => {
                    if let Ok(names) =
                        tcl_syntax::word_rules::WordValueRules::from_config(&body.config())
                            .split_list(value)
                    {
                        result
                            .names
                            .extend(names.into_iter().map(std::borrow::Cow::into_owned));
                    }
                }
                _ => {}
            }
        }
        if let Some(bodies) = schema.authored_source_plain_script_arguments() {
            for argument in bodies {
                if schema.authored_source_script_timing_at(argument)
                    == Some(tcl_registry::ScriptTiming::SameInvocation)
                    && let Some(child) = original.get(argument + 1)
                {
                    collect(child, registry, dialect, realm, depth + 1, result);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_global_body_names_do_not_name_current_frame_locals() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("uplevel #0 {set x 2}", &registry);
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        assert!(original_authored_body_name_candidates(tokens, &registry).is_none());
        for source in ["uplevel 0 {set x 2}", "uplevel $level {set x 2}"] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(&module.top_level.statements[0])
                .unwrap();
            let candidates = original_authored_body_name_candidates(tokens, &registry)
                .expect("current or unknown source frame retains possible local names");
            assert!(candidates.names.contains("x"));
            assert!(candidates.unknown_residual);
        }
        let mut replaced = CommandRegistry::build_default();
        replaced.insert(tcl_registry::CommandSpec {
            name: "uplevel",
            arity: tcl_registry::Arity::exact(2),
            arg_roles: &[(1, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let module = crate::lowering::lower_to_ir("uplevel #0 {set x 2}", &replaced);
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        let candidates = original_authored_body_name_candidates(tokens, &replaced)
            .expect("spelling cannot donate the old frame descriptor");
        assert!(candidates.names.contains("x"));
        assert!(candidates.unknown_residual);
        assert!(original_authored_body_name_candidates(tokens, &registry).is_none());
    }

    #[test]
    fn original_evaluated_body_names_remain_candidates_without_frame_entry() {
        let registry = CommandRegistry::build_default();
        for source in [
            "eval {if {$x > 1} {set y 2}}",
            "uplevel 0 {set x 2}",
            "eval {set {$literal} VALUE}",
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            assert!(module.source_entry.native_entry.is_none());
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(&module.top_level.statements[0])
                .unwrap();
            let candidates = original_authored_body_name_candidates(tokens, &registry)
                .expect("actual original body source owner");
            let expected = if source.contains("$literal") {
                "$literal"
            } else {
                "x"
            };
            assert!(candidates.names.contains(expected));
            assert!(candidates.unknown_residual);
            let mut changed = tokens.clone();
            changed.argv_texts.last_mut().unwrap().push_str(" changed");
            assert!(original_authored_body_name_candidates(&changed, &registry).is_none());
        }
    }
}
