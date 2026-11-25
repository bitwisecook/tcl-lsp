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

use super::*;
use crate::signature_scan::original_name::SignatureSourceNameKey;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::{naming::ExecutionNamePolicy, word_rules::WordValueRules};

fn value(source: &[u8], context: &ResolveContext) -> OriginalProducedNameValue {
    let policy = context
        .execution_name_policy
        .unwrap()
        .native_recipe()
        .unwrap();
    let config = LexerConfig::from_grammar(context.invocation_dialect.unwrap().lexer_grammar);
    let parsed = tcl_lexer::native_script_words_in(
        SourceImage::native(source),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let key = SignatureSourceNameKey::from_original_native_word(
        &parsed.commands[0].words[0],
        WordValueRules::from_config(&config),
        policy,
    )
    .unwrap();
    OriginalProducedNameValue::from_source_input(
        &SignatureSourceNameInput::OriginalWord(key),
        context,
    )
    .unwrap()
}

#[test]
fn original_contents_currency_preserves_cache_changes_and_retires_unknown_effects() {
    // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
    for policy in tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(NamePolicyProtocol::authored_tcl)
        .chain([NamePolicyProtocol::authored_jim084()])
    {
        let mut context = ResolveContext::default();
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        context.invocation_dialect = Some(match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(version) => {
                tcl_registry::InvocationDialect::for_version(version)
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                tcl_registry::InvocationDialect::of_point(
                    tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
                )
            }
        });
        let frozen = value(b"p\xff\0tail", &context);
        assert!(frozen.is_current(&context));
        context.invalidate_shared_representations();
        assert!(frozen.is_current(&context));
        context.invalidate_original_contents();
        assert!(!frozen.is_current(&context));
        let fresh = value(b"p\xff\0tail", &context);
        assert!(fresh.is_current(&context));
        assert_ne!(fresh, frozen);
    }
}

#[test]
fn original_content_joins_keep_every_origin_and_do_not_reuse_unknown_currency() {
    // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
    let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let mut context = ResolveContext::default();
    context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
    context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    let plain = value(b"same", &context);
    let grouped = value(b"{same}", &context);
    let joined = plain.joined(&grouped).unwrap();
    assert_eq!(joined.bytes(), b"same");
    assert_eq!(joined.original_origins().len(), 2);
    assert!(plain.joined(&value(b"different", &context)).is_none());
    assert!(plain.concatenated(&value(b"\xff", &context)).is_none());
    assert!(plain.concatenated(&value(b"\0tail", &context)).is_none());
    let held = plain.concatenated(&value(b"{ suffix}", &context)).unwrap();
    assert_eq!(held.bytes(), b"same suffix");
    let mut changed = context.clone();
    changed.invalidate_original_contents();
    context.join(&changed);
    assert_eq!(context.original_contents_epoch(), None);
    context.invalidate_original_contents();
    assert_eq!(context.original_contents_epoch(), None);
    assert!(!held.is_current(&context));
}

#[test]
fn namespace_prefix_result_preserves_counted_script_and_producer_origins() {
    for version in tcl_dialect::TclVersion::ALL {
        let policy = NamePolicyProtocol::authored_tcl(version);
        let mut context = ResolveContext::default();
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(version));
        let operation = value(b"namespace", &context);
        let script = value(b"{helper 3}", &context);
        let result = script
            .namespace_prefix_result(&operation, [b"::namespace", b"inscope", b"::lib"])
            .unwrap();
        assert_eq!(result.bytes(), b"::namespace inscope ::lib {helper 3}");
        assert_eq!(result.original_origins().len(), 2);
        assert!(result.is_current(&context));

        context.invalidate_original_contents();
        let newer_script = value(b"{helper 3}", &context);
        assert!(
            newer_script
                .namespace_prefix_result(&operation, [b"::namespace", b"inscope", b"::lib"])
                .is_none()
        );
    }
}

#[test]
fn original_evaluated_index_keeps_its_exact_word_and_arena() {
    // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let registry = tcl_registry::CommandRegistry::build_default();
        for (source, expected) in [
            ("set index k; set a(${index}) value", b"k".as_slice()),
            (
                "set index k; set a(pre${index}post) value",
                b"prekpost".as_slice(),
            ),
        ] {
            let analysis = super::super::SourceCommandBindings::analyse_with_options(
                source,
                config,
                &registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(dialect),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            frame:
                                tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                            catch_depth: Some(0),
                            loop_depth: 0,
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.find("set a(").unwrap()).unwrap();
            let binding = analysis.invocation_at_source("set", offset);
            let tokens = binding.original_recorded_command_tokens().unwrap();
            let words = crate::registry_invocation::original_native_compiler_words(
                &tcl_lexer::SourceImage::document(source),
                tokens.words(),
                offset,
                config,
            )
            .unwrap();
            let word = &words[1];
            let tcl_syntax::native_variable_words::NativeVariableWordOperand::CompoundArray {
                index,
                ..
            } = tcl_syntax::native_variable_words::native_variable_word(
                word,
                version,
                NamePolicyProtocol::authored_tcl(version).string_protocol(),
            )
            .unwrap()
            else {
                panic!("expected original compound index");
            };
            let held = binding.frozen_written_names.as_ref().unwrap()[1]
                .as_deref()
                .unwrap();
            let context = &binding.variable_context;
            let child = held
                .evaluated_variable_index(word, &index, context)
                .unwrap();
            assert_eq!(child.bytes(), expected, "{version:?}/{source}");
            assert!(child.original_word_key().is_none());
            assert!(
                held.evaluated_variable_index(&words[2], &index, context)
                    .is_none()
            );
            assert!(
                held.concatenated(&value(b"suffix", context))
                    .unwrap()
                    .evaluated_variable_index(word, &index, context)
                    .is_none()
            );
            assert!(
                OriginalProducedNameValue::list_result(std::slice::from_ref(held), held)
                    .unwrap()
                    .evaluated_variable_index(word, &index, context)
                    .is_none()
            );
            assert_eq!(
                held.joined(held)
                    .unwrap()
                    .evaluated_variable_index(word, &index, context)
                    .unwrap(),
                child
            );
            let mut changed = (**context).clone();
            changed.invalidate_original_contents();
            assert!(
                held.evaluated_variable_index(word, &index, &changed)
                    .is_none()
            );
        }
    }
}

#[test]
fn original_child_lineage_is_bounded_across_repeated_capture() {
    // Implementation contract: naming.source.original-content-currency (docs/design/analysis/name-resolution-proofs/original-content-currency.md).
    let mut context = ResolveContext::default();
    context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
        NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
    ));
    context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    let mut held = value(b"7", &context);
    let mut captures = 0;
    loop {
        assert!(held.is_current(&context));
        assert_eq!(held.bytes(), b"7");
        let parent =
            crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(
                &held,
            );
        let Some(child) = parent.list_element(0) else {
            break;
        };
        let input = SignatureSourceNameInput::OriginalValue(child);
        let Some(next) = OriginalProducedNameValue::from_source_input(&input, &context) else {
            break;
        };
        held = next;
        captures += 1;
        assert!(captures < MAX_ORIGINS);
    }
    assert!(captures > 0);
    assert!(held.lineage_within(MAX_ORIGINS));
    assert!(!held.lineage_within(1));
}
