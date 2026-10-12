// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Callback operands retained separately from their receiving command head.

use super::{OriginalCommandLookup, SourceInvocationBinding};
use crate::registry_invocation::{InvocationWordOrigin, ResolvedStatementInvocation};
use crate::signature_scan::scope::SignatureSourceNameInput;
use std::sync::Arc;
use tcl_registry::{AppendedArity, ScriptLookupScope};

/// Original callback prefix and its independently selected lookup coordinates.
/// Readonly prefix children supply no editable word, installed callback,
/// future binding, normal completion or native argument vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalCallbackPrefix {
    input: SignatureSourceNameInput,
    lookup: Option<OriginalCommandLookup>,
    source_registration: Option<Arc<super::OriginalSourceCallbackRegistration>>,
    scope: Option<ScriptLookupScope>,
    appended: Option<AppendedArity>,
    baked: usize,
}

impl std::hash::Hash for OriginalCallbackPrefix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.input.hash(state);
        self.lookup.hash(state);
        self.scope.hash(state);
        self.appended.hash(state);
        self.baked.hash(state);
        self.source_registration
            .as_ref()
            .map(|registration| registration.site())
            .hash(state);
    }
}

impl OriginalCallbackPrefix {
    /// Exact readonly head child of the authentic callback operand.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// UTF-8 reporting label and exact original source extent of the prefix head.
    /// Readonly child geometry supplies no editable word or command selection.
    #[must_use]
    pub fn reported_source_head(&self) -> Option<(&str, tcl_lexer::Span)> {
        let name = std::str::from_utf8(self.input.bytes()).ok()?;
        if name.is_empty() {
            return None;
        }
        let span = self
            .input
            .original_static_value_source_extent(0..self.input.bytes().len())?;
        Some((name, span))
    }
    /// Lookup geometry when its callback frame coordinates are available.
    /// An unreached trigger never borrows the registration frame.
    #[must_use]
    pub const fn lookup(&self) -> Option<&OriginalCommandLookup> {
        self.lookup.as_ref()
    }
    /// Genuine complete source installer and conditional selected schema.
    /// This is separate from Native lookup and the operand's own producer site.
    #[must_use]
    pub fn source_registration(&self) -> Option<&super::OriginalSourceCallbackRegistration> {
        self.source_registration.as_deref()
    }
    pub(crate) fn with_source_registration(
        &self,
        lookup: &super::OriginalSourceCallbackProcedureLookup,
    ) -> Option<Self> {
        lookup.matches_prefix(self).then_some(())?;
        let mut retained = self.clone();
        retained.source_registration = Some(Arc::new(lookup.registration().clone()));
        Some(retained)
    }
    /// Independently selected callback lookup-frame descriptor, when supplied.
    /// Prefix layout and suffix arity never fill an unavailable frame purpose.
    #[must_use]
    pub const fn scope(&self) -> Option<ScriptLookupScope> {
        self.scope
    }
    /// Arguments appended by the independently selected prefix receiver.
    #[must_use]
    pub const fn appended_arity(&self) -> Option<AppendedArity> {
        self.appended
    }
    /// Fixed arguments preceding the receiving callback suffix.
    #[must_use]
    pub const fn baked_argument_count(&self) -> usize {
        self.baked
    }

    pub(super) fn from_original_builder_head(
        input: SignatureSourceNameInput,
        facts: &tcl_registry::InvocationFacts,
        argument: usize,
        baked: usize,
    ) -> Option<Self> {
        let appended = facts.command_prefix_arity(argument)?;
        Some(Self {
            input,
            lookup: None,
            source_registration: None,
            scope: facts.script_lookup_scope(argument),
            appended: Some(appended),
            baked,
        })
    }

    pub(crate) fn from_original_static_operand(
        word: &tcl_lexer::NativeWord,
        facts: &tcl_registry::InvocationFacts,
        argument: usize,
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<Self> {
        let policy = dialect.authored_name_policy()?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&word.config());
        let value =
            crate::signature_scan::scope::SignatureSourceNameValue::from_original_static_word(
                word, rules, policy,
            )?;
        let prefix = SignatureSourceNameInput::OriginalValue(value);
        let scope = facts.script_lookup_scope(argument);
        let appended = facts.command_prefix_arity(argument);
        let inputs = original_callback_words(&prefix, dialect, facts, argument)?;
        Some(Self {
            input: inputs.first()?.clone(),
            lookup: None,
            source_registration: None,
            scope,
            appended,
            baked: inputs.len() - 1,
        })
    }
}

impl SourceInvocationBinding {
    /// Retain a callback at an actual original written operand ordinal. The
    /// selected effective argv, original producer, evaluator and frame purpose
    /// must agree; a displayed prefix or installer head supplies no fallback.
    #[must_use]
    pub fn original_callback_prefix(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCallbackPrefix> {
        let invocation =
            crate::registry_invocation::original_callback_invocation(registry, tokens)?;
        self.original_callback_prefix_from_invocation(tokens, written, registry, invocation)
    }

    /// Retain the same original callback using the caller's complete context.
    /// A foreign generation refuses without nominal or profile assistance.
    #[must_use]
    pub fn original_callback_prefix_in_context(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<OriginalCallbackPrefix> {
        let invocation =
            crate::registry_invocation::original_callback_invocation_with_metadata_context(
                context.commands(),
                context.into(),
                tokens,
            )?;
        self.original_callback_prefix_from_invocation(
            tokens,
            written,
            context.commands(),
            invocation,
        )
    }

    fn original_callback_prefix_from_invocation(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
        registry: &tcl_registry::CommandRegistry,
        invocation: crate::registry_invocation::ResolvedStatementInvocation,
    ) -> Option<OriginalCallbackPrefix> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let mut selected = invocation
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, origin)| {
                (*origin == InvocationWordOrigin::Written(written)).then_some(index - 1)
            });
        let argument = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let scope = invocation.facts.script_lookup_scope(argument)?;
        let appended = invocation.facts.command_prefix_arity(argument);
        let prefix = original_callback_input(self, tokens, &invocation, argument)?;
        if !prefix.is_current(&self.variable_context) {
            return None;
        }
        if invocation.dialect?.native_string_protocol() != Some(prefix.policy().string_protocol()) {
            return None;
        }
        let inputs =
            original_callback_words(&prefix, invocation.dialect?, &invocation.facts, argument)?;
        let input = inputs.first()?.clone();
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        for row in rows.clone() {
            if row.config != config
                || !row.entry.owns_source(
                    &self.invocation_site()?.source,
                    self.invocation_site()?.offset,
                )
            {
                return None;
            }
            if scope == ScriptLookupScope::InvokingFrame
                && !self.original_argument_completion.is_normal()
                && !self.original_operands_preserve_lookup(tokens, registry, row, 0)
            {
                return None;
            }
        }
        let lookup = self.original_callback_lookup_from_rows(&input, rows.collect(), scope);
        Some(OriginalCallbackPrefix {
            input,
            lookup,
            source_registration: None,
            scope: Some(scope),
            appended,
            baked: inputs.len() - 1,
        })
    }
}

fn original_callback_words(
    prefix: &SignatureSourceNameInput,
    dialect: tcl_registry::InvocationDialect,
    facts: &tcl_registry::InvocationFacts,
    argument: usize,
) -> Option<Vec<SignatureSourceNameInput>> {
    let scope = facts.script_lookup_scope(argument);
    if let Some(kind) = original_trace_kind(facts, argument) {
        return Some(
            super::command_observers::original_trace_callback_words_in_dialect(
                prefix, dialect, kind,
            )?
            .inputs,
        );
    }
    if facts.command_prefix_arity(argument).is_some() {
        // A readonly prefix can retain trigger-frame syntax independently of
        // lookup. The source target owner refuses unavailable relative scope.
        return prefix.original_list_elements();
    }
    if scope == Some(ScriptLookupScope::TriggerFrame) || scope.is_none() {
        return None;
    }
    facts.body_execution?.deferred_entry_frame(dialect)?;
    original_single_script_words(prefix, dialect)
}

fn original_callback_input(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    invocation: &ResolvedStatementInvocation,
    argument: usize,
) -> Option<SignatureSourceNameInput> {
    match *invocation.effective.origins.get(argument.checked_add(1)?)? {
        InvocationWordOrigin::Written(written) => {
            binding.original_written_name_input(tokens, written)
        }
        InvocationWordOrigin::ExpandedElement { written, element } => binding
            .original_written_name_input(tokens, written)?
            .original_list_element(element),
        InvocationWordOrigin::BindingPrefix(index) => binding
            .proved_handler_target()?
            .original_prepended_name_inputs()?
            .get(index)
            .cloned(),
        InvocationWordOrigin::ResolvedHead => None,
    }
}

fn original_trace_kind(
    facts: &tcl_registry::InvocationFacts,
    argument: usize,
) -> Option<super::command_observers::OriginalTraceCallbackKind> {
    use super::command_observers::OriginalTraceCallbackKind;
    use tcl_registry::{StateTransition, TraceTarget, TraceTransition};
    for fact in facts.state_transitions.declared()?.facts() {
        let StateTransition::Trace(TraceTransition::Add { target, prefix, .. }) = &fact.transition
        else {
            continue;
        };
        if prefix.argument_index() != Some(argument) {
            continue;
        }
        return Some(match target {
            TraceTarget::Variable(_) => OriginalTraceCallbackKind::Variable,
            TraceTarget::Command(_) => OriginalTraceCallbackKind::Command,
            TraceTarget::Execution(_) => OriginalTraceCallbackKind::Execution,
        });
    }
    None
}

fn original_single_script_words(
    prefix: &SignatureSourceNameInput,
    dialect: tcl_registry::InvocationDialect,
) -> Option<Vec<SignatureSourceNameInput>> {
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.execution_point()?.grammar());
    let inputs = prefix.original_list_elements()?;
    if inputs.is_empty() {
        return None;
    }
    let image = tcl_lexer::SourceImage::native(prefix.bytes());
    let extent = tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?);
    let plan = tcl_lexer::native_script_words_in(image, extent, config).ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some()
        || command.words.len() != inputs.len()
        || command.words.iter().any(|word| word.group().expand)
    {
        return None;
    }
    let native = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &command.words,
        prefix.policy().string_protocol(),
    )
    .ok()?;
    inputs
        .iter()
        .enumerate()
        .all(|(index, input)| native.literal(index) == Some(input.bytes()))
        .then_some(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

    fn bindings(
        source: &str,
        dialect: tcl_registry::InvocationDialect,
        registry: &tcl_registry::CommandRegistry,
    ) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn prefix(
        source: &str,
        written: usize,
        dialect: tcl_registry::InvocationDialect,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCallbackPrefix> {
        let bindings = bindings(source, dialect, registry);
        let binding = bindings.invocation_at_source("", 0);
        let Some(mut tokens) = binding.original_recorded_command_tokens() else {
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CALLBACK").is_some() {
                eprintln!(
                    "ORIGINAL_CALLBACK source={source:?} dialect={:?} original_vector=false",
                    dialect.execution_point()
                );
            }
            return None;
        };
        tokens.source_binding = Some(binding.clone());
        let result = binding.original_callback_prefix(&tokens, written, registry);
        if result.is_none() && std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CALLBACK").is_some() {
            let invocation =
                crate::registry_invocation::original_callback_invocation(registry, &tokens);
            let input = binding.original_written_name_input(&tokens, written);
            let rows = binding
                .declaration_layout_observations
                .as_deref()
                .and_then(super::super::declaration_layout::original_declaration_layouts);
            eprintln!(
                "ORIGINAL_CALLBACK source={source:?} dialect={:?} config={} metadata={} argument_normal={} input={} input_current={} rows={:?}",
                dialect.execution_point(),
                binding.original_lexer_config_for_tokens(&tokens).is_some(),
                invocation.is_some(),
                binding.original_argument_completion.is_normal(),
                input.is_some(),
                input
                    .as_ref()
                    .is_some_and(|input| input.is_current(&binding.variable_context)),
                rows.as_ref().map(|rows| rows.clone().count())
            );
            if let Some(invocation) = invocation {
                let argument = written - 1;
                eprintln!(
                    "ORIGINAL_CALLBACK effective={:?} scope={:?} arity={:?} body={:?} words={:?}",
                    invocation.effective.origins,
                    invocation.facts.script_lookup_scope(argument),
                    invocation.facts.command_prefix_arity(argument),
                    invocation.facts.body_execution,
                    input
                        .as_ref()
                        .and_then(|prefix| original_callback_words(
                            prefix,
                            dialect,
                            &invocation.facts,
                            argument
                        ))
                        .map(|words| words.len())
                );
            }
        }
        result
    }

    #[test]
    fn original_callback_heads_keep_selected_operand_evaluator_and_frame_scope() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let point =
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(environment)).unwrap();
            let dialect = tcl_registry::InvocationDialect::of_point(point);
            let registry = tcl_registry::model::ingress::static_context_for(environment).commands();
            let callback = prefix("lsort -command {::cmp fixed} {b a}", 2, dialect, registry)
                .unwrap_or_else(|| panic!("{environment}: original lsort prefix"));
            assert_eq!(callback.name_input().bytes(), b"::cmp");
            assert!(callback.name_input().original_word_key().is_none());
            assert_eq!(callback.appended_arity(), Some(AppendedArity::Exactly(2)));
            assert_eq!(callback.baked_argument_count(), 1);
            assert_eq!(callback.scope(), Some(ScriptLookupScope::InvokingFrame));
            let lookup = callback.lookup().unwrap();
            assert_eq!(
                lookup.callback_lookup_scope(),
                Some(ScriptLookupScope::InvokingFrame)
            );
            assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), b"cmp");
            assert!(prefix("lsort -command {::cmp fixed} {b a}", 3, dialect, registry).is_none());
            let literal = prefix(
                "lsort -command {::cmp {$not_substituted}} {b a}",
                2,
                dialect,
                registry,
            )
            .unwrap();
            assert_eq!(literal.baked_argument_count(), 1);
            let deferred = prefix("after idle {::later fixed}", 2, dialect, registry).unwrap();
            assert_eq!(deferred.name_input().bytes(), b"::later");
            assert_eq!(deferred.appended_arity(), None);
            assert_eq!(deferred.scope(), Some(ScriptLookupScope::GlobalFrame));
            assert_eq!(
                deferred.lookup().unwrap().callback_lookup_scope(),
                Some(ScriptLookupScope::GlobalFrame)
            );
            assert!(prefix("after idle {::later; ::other}", 2, dialect, registry).is_none());
            assert!(prefix("after cancel ::later", 2, dialect, registry).is_none());
            if point.family() != tcl_dialect::model::Family::Tcl {
                continue;
            }
            for (kind, operations) in [
                ("variable", "write"),
                ("command", "rename"),
                ("execution", "enter"),
            ] {
                let source = format!("trace add {kind} v {operations} {{::trace_cb fixed}}");
                let callback = prefix(&source, 5, dialect, registry).unwrap();
                assert_eq!(callback.name_input().bytes(), b"::trace_cb");
                assert_eq!(callback.scope(), Some(ScriptLookupScope::TriggerFrame));
                let lookup = callback.lookup().expect("absolute source target geometry");
                assert_eq!(
                    lookup.callback_lookup_scope(),
                    Some(ScriptLookupScope::TriggerFrame)
                );
                assert!(
                    lookup.original_naming_scope().is_none(),
                    "the actual triggering frame is still unknown"
                );
                assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), b"trace_cb");
                let relative = source.replace("::trace_cb", "trace_cb");
                assert!(
                    prefix(&relative, 5, dialect, registry)
                        .unwrap()
                        .lookup()
                        .is_none()
                );
                let removed = source.replacen(" add ", " remove ", 1);
                assert!(prefix(&removed, 5, dialect, registry).is_none());
                let script = format!("trace add {kind} v {operations} {{$head fixed}}");
                assert!(prefix(&script, 5, dialect, registry).is_none());
                let script = format!("trace add {kind} v {operations} {{::trace_cb; ::other}}");
                assert!(prefix(&script, 5, dialect, registry).is_none());
            }
        }
    }

    #[test]
    fn original_callback_invocation_rows_retain_readonly_scope_lookup() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        let result = crate::analyser::Analyser::new()
            .analyse("lsort -command {::cmp fixed} {b a}", "tcl8.6");
        let callback = result
            .command_invocations
            .iter()
            .find(|invocation| invocation.callback_arity.is_some())
            .unwrap();
        assert_eq!(
            callback.original_name_input.as_ref().unwrap().bytes(),
            b"::cmp"
        );
        assert_eq!(callback.callback_baked_args, 1);
        assert_eq!(
            callback
                .original_lookup
                .as_ref()
                .unwrap()
                .callback_lookup_scope(),
            Some(ScriptLookupScope::InvokingFrame)
        );
        assert!(!callback.rename_safe);
    }

    #[test]
    fn original_callback_scope_refuses_foreign_vectors_and_installer_head_lookup() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        let source = "lsort -command {::cmp fixed} {b a}";
        let owner = bindings(source, dialect, &registry);
        let binding = owner.invocation_at_source("", 0);
        let mut tokens = binding.original_recorded_command_tokens().unwrap();
        assert!(
            binding
                .original_callback_prefix(&tokens, 2, &registry)
                .is_none()
        );
        tokens.source_binding = Some(binding.clone());
        let callback = binding
            .original_callback_prefix(&tokens, 2, &registry)
            .unwrap();
        assert!(
            binding
                .original_command_lookup(&tokens, callback.name_input())
                .is_none()
        );
        let mut shortened = tokens.clone();
        shortened.word_exprs.pop();
        assert!(
            binding
                .original_callback_prefix(&shortened, 2, &registry)
                .is_none()
        );
    }
}
