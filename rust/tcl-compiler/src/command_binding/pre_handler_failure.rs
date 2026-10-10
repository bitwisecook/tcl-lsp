// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proven rejection of original operands before their command handler enters.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceNamespaceKey, SourceOutcomes,
};
use crate::ir::{CommandTokens, Provenance, WordExpr};
use crate::var_resolve::VariableExecutionFrame;
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ArgumentEntry {
    site: CommandAllocationSite,
    namespace: SourceNamespaceKey,
    frame: VariableExecutionFrame,
}

impl ArgumentEntry {
    pub(super) fn capture(
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        offset: u32,
    ) -> Option<Box<Self>> {
        Some(Box::new(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset,
            },
            namespace: context.namespace_identity(),
            frame: state.variable_frame.clone(),
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreHandlerFailure {
    entry: ArgumentEntry,
    words: Arc<[WordExpr]>,
}

pub(super) type PreHandlerFailures = BTreeMap<CommandAllocationSite, Vec<PreHandlerFailure>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnrepresentedEntry {
    namespace: Option<SourceNamespaceKey>,
    frame: Option<VariableExecutionFrame>,
}

pub(super) type UnrepresentedEntries =
    BTreeMap<Arc<super::SourceOriginId>, Vec<UnrepresentedEntry>>;

impl UnrepresentedEntry {
    fn may_share_namespace(&self, namespace: &SourceNamespaceKey) -> bool {
        self.namespace
            .as_ref()
            .or_else(|| {
                self.frame
                    .as_ref()
                    .and_then(VariableExecutionFrame::namespace_identity)
            })
            .is_none_or(|key| key == namespace)
    }
}

impl SourceCommandBindings {
    pub(super) fn record_unrepresented_entry(
        &mut self,
        origin: Option<&Arc<super::SourceOriginId>>,
        namespace: Option<SourceNamespaceKey>,
        frame: Option<VariableExecutionFrame>,
    ) {
        if self.declaration_preview_depth != 0 {
            return;
        }
        let Some(origin) = origin else { return };
        let entry = UnrepresentedEntry { namespace, frame };
        let entries = self
            .unrepresented_entries
            .entry(Arc::clone(origin))
            .or_default();
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }

    pub(super) fn record_unrepresented_called_body(
        &mut self,
        site: u32,
        state: &ModuleCommandBindings,
        target: &super::SourceCommandTarget,
        body: &super::DeferredSourceBody,
    ) {
        let namespace = super::called_body_namespace(state, target, body);
        let frame = namespace.as_ref().map(|namespace| {
            let identity = super::source_called_body_activation_name(
                state.current_source_origin.as_ref(),
                target,
                body,
                site,
            );
            let frame = if body.receiver_method {
                VariableExecutionFrame::ReceiverMethod { identity }
            } else {
                VariableExecutionFrame::Procedure {
                    namespace: namespace.display().unwrap_or_default(),
                    identity,
                }
            };
            frame.with_namespace_identity(namespace.clone())
        });
        self.record_unrepresented_entry(body.source_origin.as_ref(), namespace, frame);
    }

    pub(super) fn record_unrepresented_source_entry(
        &mut self,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        // An unknown context withdraws coverage for every namespace observation
        // of this same original source; it cannot donate a lexical label.
        let namespace = context
            .namespace_key
            .cloned()
            .or_else(|| context.frame.namespace_identity().cloned());
        self.record_unrepresented_entry(
            state.current_source_origin.as_ref(),
            namespace,
            Some(context.frame.clone()),
        );
    }

    pub(super) fn record_pre_handler_failure(
        &mut self,
        entry: Option<Box<ArgumentEntry>>,
        words: &[WordExpr],
        outcomes: &SourceOutcomes,
    ) {
        if self.declaration_preview_depth != 0
            || outcomes.normal.is_some()
            || outcomes.abrupt.is_empty()
            || outcomes
                .abrupt
                .iter()
                .any(|(route, _)| *route != InvocationCompletionRoute::Tcl(CompletionCode::Error))
        {
            return;
        }
        let Some(entry) = entry else { return };
        if entry.frame.layout() == &VariableExecutionFrame::Unknown {
            return;
        }
        let failure = PreHandlerFailure {
            entry: *entry,
            words: Arc::from(words),
        };
        let failures = self
            .pre_handler_failures
            .entry(failure.entry.site.clone())
            .or_default();
        if !failures.contains(&failure) {
            failures.push(failure);
        }
    }

    /// An original operand evaluation rejected every represented actual entry.
    /// This withdraws a normal handler transfer; declaration grammar remains
    /// available independently. A possible actual dispatch defeats this query.
    pub(crate) fn original_arguments_rejected_before_handler(
        &self,
        tokens: &CommandTokens,
    ) -> bool {
        let Some(head) = tokens.words().first() else {
            return false;
        };
        if tokens.synthetic.is_some() || head.source().provenance != Provenance::Source {
            return false;
        }
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: head.source().span.start(),
        };
        let Some(failures) = self.pre_handler_failures.get(&site) else {
            return false;
        };
        let Some(first) = failures.first() else {
            return false;
        };
        if self
            .unrepresented_entries
            .get(origin)
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.may_share_namespace(&first.entry.namespace))
            })
        {
            return false;
        }
        if self
            .dispatch_points_at(site.offset)
            .any(|point| !point.declaration_preview && point.dispatch)
        {
            return false;
        }
        failures
            .iter()
            .all(|failure| failure.entry == first.entry && failure.words.as_ref() == tokens.words())
    }

    pub(super) fn retain_pre_handler_failure_namespace(
        &mut self,
        origin: &Arc<super::SourceOriginId>,
        namespace: &SourceNamespaceKey,
    ) {
        self.unrepresented_entries.update_if_needed(
            |entries| {
                entries.get(origin).is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|entry| !entry.may_share_namespace(namespace))
                })
            },
            |entries| {
                if let Some(entries) = entries.get_mut(origin) {
                    entries.retain(|entry| entry.may_share_namespace(namespace));
                }
            },
        );
        self.pre_handler_failures.update_if_needed(
            |failures| {
                failures.iter().any(|(site, failures)| {
                    failures.is_empty()
                        || &site.source == origin
                            && failures
                                .iter()
                                .any(|failure| &failure.entry.namespace != namespace)
                })
            },
            |failures| {
                failures.retain(|site, failures| {
                    if &site.source == origin {
                        failures.retain(|failure| &failure.entry.namespace == namespace);
                    }
                    !failures.is_empty()
                })
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;

    fn analyse(
        source: &str,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn setter_tokens(source: &str, bindings: &SourceCommandBindings) -> CommandTokens {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        let offset = source.rfind("myset y").unwrap();
        let original = "myset y [e {$x+1}]";
        assert!(source[offset..].starts_with(original));
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            original,
            u32::try_from(offset).unwrap(),
            config,
        )
        .into_iter()
        .next()
        .unwrap();
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        tokens
    }

    #[test]
    fn original_operand_rejection_distinguishes_missing_defined_and_unknown_tables() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let prefix = "interp alias {} myset {} set; interp alias {} e {} expr; ";
        for (defined, expected) in [(false, true), (true, false)] {
            let source = format!(
                "{}{prefix}myset y [e {{$x+1}}]",
                if defined { "set x 1; " } else { "" }
            );
            let bindings = analyse(&source, &entry);
            let tokens = setter_tokens(&source, &bindings);
            assert_eq!(
                bindings.original_arguments_rejected_before_handler(&tokens),
                expected,
                "defined={defined}"
            );
        }
        let source = format!("{prefix}myset y [e {{$x+1}}]");
        let mut unknown = entry;
        unknown.namespace_variable_tables = None;
        let bindings = analyse(&source, &unknown);
        assert!(
            !bindings
                .original_arguments_rejected_before_handler(&setter_tokens(&source, &bindings))
        );
    }

    #[test]
    fn skipped_possible_normal_body_entry_withdraws_earlier_operand_failure() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let prefix = "interp alias {} myset {} set; interp alias {} e {} expr; \
            proc f {arg} {global x; myset y [e {$x+1}]}; catch {f first}; ";
        for (suffix, rejected) in [("", true), ("set x 1; f [mystery]", false)] {
            let source = format!("{prefix}{suffix}");
            let bindings = analyse(&source, &entry);
            let body = bindings.procedure_implementation_bodies().next().unwrap();
            let selected = bindings
                .selected_source_in_context(body.source, body.namespace_key)
                .unwrap();
            let tokens = setter_tokens(&source, &selected);
            assert_eq!(
                selected.original_arguments_rejected_before_handler(&tokens),
                rejected,
                "suffix={suffix}"
            );
            if !rejected {
                assert!(
                    selected
                        .unrepresented_entries
                        .contains_key(&body.source.origin)
                );
            }
        }
    }

    #[test]
    fn original_operand_failure_does_not_borrow_replaced_handler_or_read_trace_semantics() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for setup in [
            "rename expr native_expr; proc expr args {return 2}; ",
            "proc fill {name index op} {set ::x 1}; trace add variable x read fill; ",
        ] {
            let source = format!(
                "{setup}interp alias {{}} myset {{}} set; interp alias {{}} e {{}} expr; myset y [e {{$x+1}}]"
            );
            let bindings = analyse(&source, &entry);
            assert!(
                !bindings
                    .original_arguments_rejected_before_handler(&setter_tokens(&source, &bindings))
            );
        }
    }
}
