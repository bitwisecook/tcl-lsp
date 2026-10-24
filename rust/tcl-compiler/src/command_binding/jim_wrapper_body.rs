// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional Jim wrapper bodies, separate from Native receiver execution.

use super::{
    Arc, CommandAllocationSite, DeferredSourceBody, ExecutedScriptSource, ModuleCommandBindings,
    SourceCommandBindings,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameKey};
use tcl_lexer::{LexerConfig, NativeWord};
use tcl_registry::CommandRegistry;
use tcl_syntax::naming::{
    NamePolicyAuthority, NativeNameContext, NativeNameProtocol, NativeNamePurpose,
};

/// The selected source factory/call and original captured wrapper body. This
/// conditional recipe allocates no command, receiver, method or native frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalJimWrapperBody {
    call: super::OriginalAuthoredJimClassCall,
    input: ResolvedAnalysisInput,
    published: tcl_core_types::ByteCommandSlot,
    body: NativeWord,
    root: DeferredSourceBody,
}

impl OriginalJimWrapperBody {
    pub(super) fn owns_source(
        &self,
        source: &ExecutedScriptSource,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) -> bool {
        self.call.matches_source_context(
            self.body.image(),
            self.input.lexer_config(),
            self.input.borrowed_context_registry(),
        ) && self
            .root
            .executed_script
            .as_ref()
            .is_some_and(|(_, _, original)| original == source)
            && self.root.future_frame.as_ref() == Some(frame)
            && Self::wrapper_namespace(&self.published).as_ref() == Some(&self.root.namespace_key)
    }

    fn wrapper_namespace(
        published: &tcl_core_types::ByteCommandSlot,
    ) -> Option<super::SourceNamespaceKey> {
        let namespace = NativeNameProtocol::Jim084
            .jim_procedure_namespace(published.simple.as_bytes())
            .ok()?;
        Some(super::SourceNamespaceKey::authored(
            SignatureNamespaceScope::Jim(namespace.into()).display()?,
        ))
    }

    fn capture(
        call: &super::OriginalAuthoredJimClassCall,
        input: &ResolvedAnalysisInput,
        incoming: &ModuleCommandBindings,
    ) -> Option<Arc<Self>> {
        let policy = incoming.baseline.execution_name_policy?.native_recipe()?;
        if policy.authority() != NamePolicyAuthority::AuthoredSimulation
            || policy.recipe() != NativeNameProtocol::Jim084
            || !call.matches_source_context(
                call.class().factory().site().source.source_image(),
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
        {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            call.original_words(),
            policy.string_protocol(),
        )
        .ok()?;
        let selected = SelectedWrapperWords::capture(call, &words, input)?;
        let published = selected.publication(call, &words, policy)?;
        let (root, body) = selected.body_root(call, &words, policy, &published, input, incoming)?;
        Some(Arc::new(Self {
            call: call.clone(),
            input: input.clone(),
            published,
            body,
            root,
        }))
    }
}

struct SelectedWrapperWords {
    member: &'static tcl_registry::definer::MemberSpec,
    name: usize,
    parameters: usize,
    body: usize,
}

impl SelectedWrapperWords {
    fn capture(
        call: &super::OriginalAuthoredJimClassCall,
        words: &tcl_registry::native_compiler_words::NativeCompilerWords<'_>,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        use tcl_registry::definer::{CallableRole, MemberEffect, MemberReceiver};
        let context = input.borrowed_context_registry();
        let grammar = context
            .context()
            .resolve_spec(context.commands(), call.class().factory().command())?
            .definition_body?;
        if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
            return None;
        }
        let member = grammar.member(std::str::from_utf8(words.literal(1)?).ok()?)?;
        if !matches!(
            member.effect,
            MemberEffect::Callable {
                receiver: MemberReceiver::Instance,
                role: CallableRole::Method,
                name_slot: Some(_),
                params_slot: Some(_),
                body_slot: Some(_),
            }
        ) {
            return None;
        }
        let values = (2..call.original_words().len())
            .map(|index| std::str::from_utf8(words.literal(index)?).ok())
            .collect::<Option<Vec<_>>>()?;
        let mask = Some(context.context().authoring_query());
        let names = member
            .indices_for_call_in(&values, mask, tcl_registry::ArgRole::Name)
            .collect::<Vec<_>>();
        let parameters = member
            .indices_for_call_in(&values, mask, tcl_registry::ArgRole::ParamList)
            .collect::<Vec<_>>();
        let bodies = member
            .indices_for_call_in(&values, mask, tcl_registry::ArgRole::Body)
            .collect::<Vec<_>>();
        let ([name], [parameters], [body]) =
            (names.as_slice(), parameters.as_slice(), bodies.as_slice())
        else {
            return None;
        };
        let mut positions = [*name, *parameters, *body];
        positions.sort_unstable();
        if values.len() != 3 || positions != [0, 1, 2] {
            return None;
        }
        Some(Self {
            member,
            name: *name,
            parameters: *parameters,
            body: *body,
        })
    }

    fn publication(
        &self,
        call: &super::OriginalAuthoredJimClassCall,
        words: &tcl_registry::native_compiler_words::NativeCompilerWords<'_>,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        let classname = call
            .class()
            .factory()
            .arguments()
            .first()?
            .value
            .as_deref()?;
        // oo.tcl's stock class procedure publishes its creator at root. The
        // creator then publishes the wrapper in its own stored Jim namespace.
        let creator = policy
            .recipe()
            .jim_two_word_member_publication_slot(
                NativeNameContext::root(),
                classname,
                self.member.keyword.as_bytes(),
                NativeNamePurpose::CommandPublication,
            )
            .ok()?;
        let creator_namespace = policy
            .recipe()
            .jim_procedure_namespace(creator.simple.as_bytes())
            .ok()?;
        policy
            .recipe()
            .jim_two_word_member_publication_slot(
                NativeNameContext::with_jim_namespace(&creator.namespace, creator_namespace),
                classname,
                words.literal(self.name + 2)?,
                NativeNamePurpose::CommandPublication,
            )
            .ok()
    }

    fn body_root(
        &self,
        call: &super::OriginalAuthoredJimClassCall,
        words: &tcl_registry::native_compiler_words::NativeCompilerWords<'_>,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        published: &tcl_core_types::ByteCommandSlot,
        input: &ResolvedAnalysisInput,
        incoming: &ModuleCommandBindings,
    ) -> Option<(DeferredSourceBody, NativeWord)> {
        let native = call.original_words();
        let registration = CommandAllocationSite {
            source: Arc::clone(&call.class().factory().site().source),
            offset: native.first()?.span().start(),
        };
        let namespace_key = OriginalJimWrapperBody::wrapper_namespace(published)?;
        let namespace = namespace_key.display()?;
        let producer = SignatureSourceNameKey::from_original_native_word(
            native.get(self.parameters + 2)?,
            tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
            policy,
        )?;
        let topology = super::formal_topology::OriginalFormalTopology::from_original_key(
            producer,
            incoming.baseline.dialect?,
        )?;
        let body = native.get(self.body + 2)?.clone();
        let source = ExecutedScriptSource::from_original_word_value(
            registration.clone(),
            self.body + 1,
            &body,
            words.literal(self.body + 2)?,
            policy.string_protocol(),
        );
        if !matches!(
            source.mapping,
            super::ExecutedScriptMapping::Contiguous { .. }
        ) || source.origin != registration.source
        {
            return None;
        }
        let identity = format!(
            "jim-source-wrapper:{}",
            registration.variable_storage_identity()
        );
        let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: namespace.clone(),
            identity: super::source_activation_name(
                Some(&registration.source),
                &identity,
                registration.offset,
            ),
        }
        .with_namespace_identity(namespace_key.clone());
        let root = DeferredSourceBody {
            realm: incoming.baseline.invocation_realm,
            identity,
            implementation_generation: registration.offset,
            implementation_allocation: None,
            source_origin: Some(Arc::clone(&registration.source)),
            executed_script: Some((registration, self.body + 1, source.clone())),
            executed_word: None,
            source: source.try_text().ok()?.to_owned(),
            offset: source.base(),
            namespace,
            namespace_key,
            event: None,
            receiver_method: false,
            future_frame: Some(frame),
            parameters: topology.advisory_parameters(),
            original_parameters: Some(topology),
            statics: None,
        };
        Some((root, body))
    }
}

impl SourceCommandBindings {
    pub(super) fn retain_jim_wrapper_future_bodies(
        &mut self,
        config: LexerConfig,
        registry: &CommandRegistry,
    ) {
        let Some(policy) = self
            .final_state
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        else {
            return;
        };
        if policy.authority() != NamePolicyAuthority::AuthoredSimulation
            || policy.recipe() != NativeNameProtocol::Jim084
        {
            return;
        }
        let incoming = Arc::clone(&self.final_state);
        if incoming.baseline.source_point_entry.is_some()
            || !self
                .conditional_source
                .root_outcomes
                .as_ref()
                .and_then(|outcomes| outcomes.normal.as_deref())
                .is_some_and(|normal| normal.same_state(&incoming))
        {
            return;
        }
        let Some(input) = incoming
            .baseline
            .metadata_context
            .source_analysis_input()
            .cloned()
        else {
            return;
        };
        if input.lexer_config() != config {
            return;
        }
        let Some(tape) =
            self.original_source_transition_advice_tape(input.borrowed_context_registry())
        else {
            return;
        };
        let wrappers = tape
            .jim_wrapper_calls()
            .filter(|call| {
                call.original_words()
                    .first()
                    .is_some_and(|word| self.dispatch_points.contains_key(&word.span().start()))
            })
            .filter_map(|call| OriginalJimWrapperBody::capture(call, &input, &incoming))
            .collect::<Vec<_>>();
        for wrapper in wrappers {
            self.walk_future_body(
                &wrapper.root,
                &incoming,
                config,
                registry,
                Some(Arc::clone(&wrapper)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn jim_wrapper_body_keeps_a_distinct_opaque_future_world() {
        tcl_spectcl::core_surfaces::ensure();
        let source = "class Point {x 0}; Point method m {arg} {missing_jim_method $arg $self}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "jim");
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let offset = u32::try_from(source.find("missing_jim_method").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("", offset)
                .lookup_state
                .is_none()
        );
        let futures = bindings.future_body_inventories();
        assert_eq!(futures.len(), 1);
        assert_eq!(
            futures[0].registration().offset,
            u32::try_from(source.find("Point method").unwrap()).unwrap()
        );
        assert_eq!(
            futures[0].source().origin.source_image().bytes(),
            source.as_bytes()
        );
        assert_eq!(
            futures[0]
                .frame()
                .namespace_identity()
                .unwrap()
                .display()
                .as_deref(),
            Some("::")
        );
        let future = futures[0].invocation_at(offset).unwrap();
        let state = &future.lookup_state.as_ref().unwrap().state;
        assert!(state.opaque_domain);
        assert!(state.baseline.native_entry.is_none());
        assert_eq!(
            future.selected_slot_diagnostic_presence(),
            super::super::SourceCommandSlotPresence::Unknown
        );
    }

    #[test]
    fn jim_wrapper_namespace_follows_both_published_procedure_keys() {
        tcl_spectcl::core_surfaces::ensure();
        let source = "class n::C {}; n::C method {m::k} {} {missing_jim_method}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "jim");
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let future = bindings.future_body_inventories().first().unwrap();
        assert_eq!(
            future
                .frame()
                .namespace_identity()
                .unwrap()
                .display()
                .as_deref(),
            Some("::n::n::C m")
        );
        assert_eq!(future.source().try_text().unwrap(), "missing_jim_method");
        assert!(matches!(
            future.frame().layout(),
            crate::var_resolve::VariableExecutionFrame::Procedure { .. }
        ));
    }

    #[test]
    fn jim_wrapper_does_not_borrow_computed_or_recooked_body_geometry() {
        tcl_spectcl::core_surfaces::ensure();
        for source in [
            "set body {missing_jim_method}; class C {}; C method m {} $body",
            "class C {}; C method m {} \"missing_jim_method\\n\"",
            "oo::class create C {method m {} {missing_jim_method}}",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "jim");
            assert!(
                analysis
                    .retained_command_realm()
                    .unwrap()
                    .source_bindings_ref()
                    .future_body_inventories()
                    .is_empty(),
                "{source}"
            );
        }
    }
}
