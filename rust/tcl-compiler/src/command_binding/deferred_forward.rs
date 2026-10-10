// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original forward prefix storage, separate from future target dispatch.

use super::{Arc, BTreeMap, ExecutedScriptSource, SourceExecutionContext};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::NameBytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalForwardMethod {
    name: SignatureSourceNameInput,
    prefix: Vec<SignatureSourceNameInput>,
}

pub(super) type OriginalForwardEntries = BTreeMap<NameBytes, OriginalForwardMethod>;

impl OriginalForwardMethod {
    pub(super) fn name_input(&self) -> &SignatureSourceNameInput {
        &self.name
    }

    pub(super) fn closed_for_policy(&self, policy: tcl_syntax::naming::NamePolicyProtocol) -> bool {
        !self.prefix.is_empty()
            && self.name.policy() == policy
            && self.name.original_word_key().is_some()
            && policy.recipe().oo_method_input(self.name.bytes()).is_ok()
            && self
                .prefix
                .iter()
                .all(|input| input.policy() == policy && input.original_word_key().is_some())
    }
}

pub(super) fn original_forward_method(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    words: &crate::ir::CommandTokens,
    offset: u32,
    dialect: tcl_registry::InvocationDialect,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    context: SourceExecutionContext<'_>,
) -> Option<OriginalForwardMethod> {
    // naming.tcloo.original-forward-registration-prefix
    // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
    let values = super::source_effective_words(words.words(), Some(dialect), None);
    let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
    let recipe =
        grammar.native_deferred_forward_setter(member, dialect, values.len().checked_sub(1)?)?;
    if policy.recipe() != recipe || values.len() != words.words().len() {
        return None;
    }
    let inputs = (1..words.words().len())
        .map(|ordinal| {
            super::deferred_method::original_member_input(
                &definition.origin,
                words,
                offset,
                ordinal,
                policy,
                context.config,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let (name, prefix) = inputs.split_first()?;
    let retained = OriginalForwardMethod {
        name: name.clone(),
        prefix: prefix.to_vec(),
    };
    retained.closed_for_policy(policy).then_some(retained)
}

pub(super) fn retained_forward_methods(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    dialect: Option<tcl_registry::InvocationDialect>,
    source: super::deferred_method::MethodParameterSource,
    context: SourceExecutionContext<'_>,
) -> Arc<OriginalForwardEntries> {
    let mut forwards = OriginalForwardEntries::new();
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let Some(dialect) = dialect else {
            continue;
        };
        if let Some(entry) = original_forward_method(
            grammar,
            definition,
            &words,
            segment.span.start(),
            dialect,
            source.policy,
            context,
        ) {
            forwards.insert(NameBytes::from(entry.name.bytes()), entry);
        } else if let Some(member) =
            super::source_effective_words(words.words(), Some(dialect), None)
                .first()
                .and_then(|word| word.as_registry_word().literal())
                .and_then(|head| grammar.member(head))
            && grammar.construction_member_effect(member)
                == tcl_registry::definer::MemberConstructionEffect::DeferredInstanceMethod
            && let Some(name) = super::deferred_method::original_member_input(
                &definition.origin,
                &words,
                segment.span.start(),
                1,
                source.policy,
                context.config,
            )
        {
            forwards.remove(&NameBytes::from(name.bytes()));
        }
    }
    Arc::new(forwards)
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_dialect::TclVersion;

    fn analyse(source: &str, version: TclVersion) -> SourceCommandBindings {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_forward_registration_retains_prefix_without_entering_target() {
        // naming.tcloo.original-forward-registration-prefix
        // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let source = r"oo::class create C {forward a::b ::later FIXED; forward m\uD800 ::later; forward m\uD801 {}}";
            let bindings = analyse(source, version);
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{version:?}"
            );
            let state = bindings.original_completed_root_state.as_ref().unwrap();
            let class = state.class_definitions.values().next().unwrap();
            let forward = class
                .forward_method_entries
                .get(b"a::b".as_slice())
                .unwrap();
            assert_eq!(forward.name_input().bytes(), b"a::b");
            assert_eq!(
                forward
                    .prefix
                    .iter()
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
                    .collect::<Vec<_>>(),
                [b"::later".as_slice(), b"FIXED"]
            );
            assert_eq!(class.forward_method_entries.len(), 3);
            assert!(
                class
                    .forward_method_entries
                    .contains_key(b"m\xed\xa0\x80".as_slice())
            );
            assert!(
                class
                    .forward_method_entries
                    .contains_key(b"m\xed\xa0\x81".as_slice())
            );
            assert!(!class.receiver_method_entries.contains_key(&(
                super::super::SourceMethodReceiver::Instance,
                tcl_core_types::NameBytes::from(b"a::b".as_slice())
            )));
            assert!(class.lifecycle_entries_closed);
            assert!(class.constructor_entry.is_none());
        }
    }

    #[test]
    fn original_forward_masks_script_entries_and_future_dispatch_stays_unknown() {
        // naming.tcloo.original-forward-registration-prefix
        // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for (source, forwarded) in [
                (
                    "oo::class create C {method pass {} {return OLD}; forward pass ::later}",
                    true,
                ),
                (
                    "oo::class create C {forward pass ::later; method pass {} {return SCRIPT}}",
                    false,
                ),
                (
                    "oo::class create Base {method pass {} {return BASE}}; oo::class create C {superclass Base; forward pass ::later}",
                    true,
                ),
                (
                    "oo::class create Base {forward pass ::later}; oo::class create C {superclass Base; method pass {} {next}}",
                    false,
                ),
            ] {
                let bindings = analyse(source, version);
                assert!(
                    bindings.original_completed_command_world().is_some(),
                    "{version:?}: {source}"
                );
                let state = bindings.original_completed_root_state.as_ref().unwrap();
                let site = u32::try_from(source.find("oo::class create C").unwrap()).unwrap();
                let class = state
                    .class_definitions
                    .iter()
                    .find(|(identity, _)| {
                        identity
                            .allocation
                            .as_ref()
                            .is_some_and(|allocation| allocation.site.offset == site)
                    })
                    .unwrap()
                    .1;
                let key = tcl_core_types::NameBytes::from(b"pass".as_slice());
                assert_eq!(class.forward_method_entries.contains_key(&key), forwarded);
                assert_eq!(
                    class
                        .receiver_method_entries
                        .contains_key(&(super::super::SourceMethodReceiver::Instance, key)),
                    !forwarded
                );
            }
            let source = "oo::class create C {forward pass ::later}; C create object; object pass";
            let bindings = analyse(source, version);
            let offset = u32::try_from(source.rfind("object pass").unwrap()).unwrap();
            assert!(
                bindings
                    .invocation_at_source("object", offset)
                    .named_object_receiver_method_entry()
                    .is_none()
            );
            assert!(
                bindings.original_completed_command_world().is_none(),
                "unproved forward invocation does not acquire Normal"
            );
        }
    }

    #[test]
    fn original_forward_registration_refuses_incomplete_or_unowned_prefixes() {
        // naming.tcloo.original-forward-registration-prefix
        // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for source in [
                "oo::class create C {forward pass}",
                "oo::class create C {forward pass $target}",
                "oo::class create C {forward pass {*}{::later FIXED}}",
                "oo::class create C {self forward pass ::later}",
                "proc ::oo::define::forward args {}; oo::class create C {forward pass ::later}",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
    }
}
