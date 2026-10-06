// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Frozen expansion values and their original written argument ownership.

use super::{Arc, ModuleCommandBindings, SourceScriptOperands};
use crate::ir::WordExpr;
use crate::registry_invocation::{EffectiveInvocationWord as Word, InvocationWordOrigin as Origin};

pub(super) fn freeze_word(
    word: &WordExpr,
    value: Option<&super::native_result::EvaluatedSourceValue>,
    state: &mut ModuleCommandBindings,
    registry: &tcl_registry::CommandRegistry,
) -> Result<Word, ()> {
    let expanded = matches!(word, WordExpr::Expand { .. });
    let inner = match word {
        WordExpr::Expand { word, .. } => word.as_ref(),
        _ => word,
    };
    let frozen = value.map_or_else(
        || {
            super::source_effective_words(
                std::slice::from_ref(inner),
                state.baseline.dialect,
                Some((&state.source_variables, registry)),
            )
            .into_iter()
            .next()
            .unwrap_or(Word::Opaque)
        },
        |value| Word::Literal(value.text.clone()),
    );
    if !expanded {
        return Ok(frozen);
    }
    Arc::make_mut(&mut state.source_variables).invalidate_shared_representations();
    let Word::Literal(text) = frozen else {
        return Ok(Word::Expanded);
    };
    let Some(dialect) = state.baseline.dialect else {
        return Ok(Word::Expanded);
    };
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let values = tcl_syntax::list::split_list_bytes_in(
        text.as_bytes(),
        rules.list,
        dialect.lexer_grammar.escapes,
    )
    .map_err(|_| ())?;
    let unicode = values
        .iter()
        .map(|value| std::str::from_utf8(value).ok().map(str::to_owned))
        .collect::<Option<Vec<_>>>();
    // The compatibility expansion carrier cannot represent non-Unicode
    // element values; do not replace their bytes with a Unicode spelling.
    Ok(unicode.map_or(Word::Expanded, Word::KnownExpansion))
}

pub(super) fn runtime_words(word: &Word) -> Vec<Word> {
    match word {
        Word::KnownExpansion(elements) => elements.iter().cloned().map(Word::Literal).collect(),
        word => vec![word.clone()],
    }
}

impl<'a> SourceScriptOperands<'a> {
    pub(super) fn written_origin(self, argument: usize) -> Option<Origin> {
        let argument = argument.checked_sub(self.target.prepended.len())?;
        let Some(written) = self.written_arguments else {
            return Some(Origin::Written(argument + 1));
        };
        let mut actual = argument.checked_add(1)?;
        for (index, word) in written.iter().enumerate() {
            let count = match word {
                Word::KnownExpansion(values) => values.len(),
                Word::Expanded => return None,
                _ => 1,
            };
            if actual < count {
                return Some(match word {
                    Word::KnownExpansion(_) => Origin::ExpandedElement {
                        written: index,
                        element: actual,
                    },
                    _ => Origin::Written(index),
                });
            }
            actual = actual.checked_sub(count)?;
        }
        None
    }

    pub(super) fn written_word(self, argument: usize) -> Option<&'a WordExpr> {
        match self.written_origin(argument)? {
            Origin::Written(index) => self.words.get(index),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationMode, NativeCompilationSelection,
    };

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn known_written_expansion_flattens_before_later_argument_mutation() {
        let source = "set words {a before}; list {*}$words [set words {b after}]";
        let bindings = analyse(source);
        let offset = u32::try_from(source.find("list").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("list", offset);
        assert!(binding.proved_handler_target().is_some(), "{binding:#?}");
        assert_eq!(
            binding.frozen_written_words().unwrap()[1],
            Word::KnownExpansion(vec!["a".to_owned(), "before".to_owned()])
        );
        let map = tcl_lexer::SourceMap::new(source);
        let config = tcl_lexer::LexerConfig::from_grammar(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar,
        );
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .find(|segment| segment.span.start() == offset)
            .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        let effective = crate::registry_invocation::effective_command_words(&tokens).unwrap();
        assert_eq!(
            effective.origins[1],
            Origin::ExpandedElement {
                written: 1,
                element: 0
            }
        );
        assert_eq!(
            effective.origins[2],
            Origin::ExpandedElement {
                written: 1,
                element: 1
            }
        );
        assert!(matches!(effective.words[1], WordExpr::Opaque { .. }));
    }

    #[test]
    fn expansion_dispatch_does_not_change_original_compiler_shape() {
        let source = "set words {x VALUE}; set {*}$words; set y $x";
        let bindings = analyse(source);
        let offset = u32::try_from(source.find("set {*}").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("set", offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            NativeCompilationSelection::Generic
        );
        assert!(binding.proved_handler_target().is_some());
        let last = u32::try_from(source.rfind("set").unwrap()).unwrap();
        assert_eq!(
            bindings
                .invocation_at_source("set", last)
                .evaluated_written_argument_value(1),
            Some("VALUE")
        );
    }

    #[test]
    fn malformed_expansion_stops_later_substitutions() {
        let source = "set words \\{; catch {list {*}$words [set after YES]}; info exists after";
        let bindings = analyse(source);
        assert!(
            !bindings
                .points
                .iter()
                .any(|point| point.offset
                    == u32::try_from(source.find("set after").unwrap()).unwrap())
        );
        let offset = u32::try_from(source.find("info exists").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("info", offset);
        assert!(binding.proved_handler_target().is_some());
    }

    #[test]
    fn expanded_script_value_has_a_materialised_source_origin() {
        let source = "set body {set x VALUE}; eval {*}[list $body]";
        let bindings = analyse(source);
        assert!(
            bindings
                .entered_scripts
                .values()
                .flat_map(|entries| entries.values())
                .flatten()
                .map(|observation| &observation.source)
                .any(|script| {
                    script.text.try_text().unwrap() == "set x VALUE"
                        && matches!(
                            script.origin.kind(),
                            super::super::SourceOriginKind::Derived { .. }
                        )
                })
        );
    }
}
