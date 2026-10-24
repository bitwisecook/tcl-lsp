// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fresh source method publications from independently current stock helpers.

use super::{
    Arc, BindingKind, ByteCommandSlot, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    NamePolicyProtocol, NativeNameContext, NativeWord, OriginalJimStockClass, OriginalJimStockName,
    PlannedJimSourceHelper, SignatureSourceNameInput, SignatureSourceNameKey,
    SourceCommandBindings, SourceCommandTarget, SourceExecutionContext, SourceJimClassHelper,
    SourceOutcomes,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalJimStockMethod {
    original: Vec<NativeWord>,
    member: SignatureSourceNameInput,
    registration: CommandAllocationSite,
}

impl OriginalJimStockMethod {
    pub(super) fn original_words(&self) -> &[NativeWord] {
        &self.original
    }

    pub(super) fn registration(&self) -> &CommandAllocationSite {
        &self.registration
    }

    pub(super) fn lineage_nodes_in(&self, remaining: &mut usize) -> bool {
        self.member.lineage_nodes_in(remaining)
    }

    pub(super) fn is_current(
        &self,
        context: &crate::var_resolve::ResolveContext,
        owner: &OriginalJimStockClass,
    ) -> bool {
        self.member.is_current(context)
            && self.registration.source == owner.site.source
            && self.original.first().is_some_and(|first| {
                first.span().start() == self.registration.offset
                    && owner
                        .original
                        .first()
                        .is_some_and(|class| class.config() == first.config())
                    && self.original.iter().all(|word| {
                        word.image() == owner.site.source.source_image()
                            && word.config() == first.config()
                    })
            })
    }

    fn capture(
        original: &[NativeWord],
        owner: &OriginalJimStockClass,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Arc<Self>> {
        if original.len() != 5 || original.iter().any(|word| word.group().expand) {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            original,
            owner.policy.string_protocol(),
        )
        .ok()?;
        let input = owner.metadata.source_analysis_input()?;
        if input.lexer_config() != context.config {
            return None;
        }
        let supplied = input.borrowed_context_registry();
        let grammar = supplied
            .context()
            .resolve_spec(context.registry, &owner.factory.command)?
            .definition_body?;
        if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
            return None;
        }
        let member = grammar.member(std::str::from_utf8(words.literal(1)?).ok()?)?;
        if member.keyword.as_bytes() != SourceJimClassHelper::Method.publication_suffix()?
            || !matches!(
                member.effect,
                tcl_registry::definer::MemberEffect::Callable {
                    receiver: tcl_registry::definer::MemberReceiver::Instance,
                    role: tcl_registry::definer::CallableRole::Method,
                    name_slot: Some(_),
                    params_slot: Some(_),
                    body_slot: Some(_),
                }
            )
        {
            return None;
        }
        let values = (2..original.len())
            .map(|index| std::str::from_utf8(words.literal(index)?).ok())
            .collect::<Option<Vec<_>>>()?;
        let mask = Some(supplied.context().authoring_query());
        for (ordinal, role) in [
            tcl_registry::ArgRole::Name,
            tcl_registry::ArgRole::ParamList,
            tcl_registry::ArgRole::Body,
        ]
        .into_iter()
        .enumerate()
        {
            let positions = member
                .indices_for_call_in(&values, mask, role)
                .collect::<Vec<_>>();
            if positions != [ordinal] {
                return None;
            }
        }
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
        let parameter = SignatureSourceNameKey::from_original_native_word(
            original.get(3)?,
            rules,
            owner.policy,
        )?;
        super::super::formal_topology::OriginalFormalTopology::from_original_key(
            parameter,
            state.baseline.dialect?,
        )?;
        let member = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                original.get(2)?,
                rules,
                owner.policy,
            )?,
        );
        if member.bytes().is_empty() || member.bytes().contains(&0) {
            return None;
        }
        Some(Arc::new(Self {
            original: original.to_vec(),
            member,
            registration: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: original.first()?.span().start(),
            },
        }))
    }
}

pub(super) fn original_stock_helper<'a>(
    state: &'a ModuleCommandBindings,
    slot: &ByteCommandSlot,
    policy: NamePolicyProtocol,
    helper: SourceJimClassHelper,
) -> Option<(
    &'a OriginalJimStockName,
    &'a super::super::source_command_world::OriginalCommandPublication,
)> {
    let publication = state.original_publication_at(slot, policy)?;
    let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
        return None;
    };
    let name = value.jim_stock_name()?;
    if name.helper != helper || name.method.is_some() || !name.matches_publication_state(state) {
        return None;
    }
    let key = state.original_command_key_for_slot(slot, policy)?;
    let bindings = state.bindings.get(&key)?;
    let MayBinding::Target(target) = bindings.first()? else {
        return None;
    };
    if bindings.len() != 1
        || target.registry_backed
        || target.kind != BindingKind::Command
        || !target.terminal
        || target.implementation_generation != name.owner.site.offset
        || !target.prepended.is_empty()
        || !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?)
    {
        return None;
    }
    Some((name, publication))
}

fn prepare_method(
    site: u32,
    words: &[crate::ir::WordExpr],
    state: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<PlannedJimSourceHelper> {
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    super::bounded_source_entry(state, policy)?;
    if words.len() != 5
        || state.source_root_namespace_key().as_ref() != Some(&context.namespace_identity())
        || target.registry_backed
        || target.kind != BindingKind::Command
        || !target.prepended.is_empty()
    {
        return None;
    }
    state
        .baseline
        .metadata_context
        .metadata_context(context.registry)?;
    let source = state.current_source_origin.as_ref()?;
    let lexical = source.original_lexical_command(words, site, context.config)?;
    let original = lexical.native_words();
    let head =
        SignatureSourceNameInput::OriginalWord(SignatureSourceNameKey::from_original_native_word(
            original.first()?,
            tcl_syntax::word_rules::WordValueRules::from_config(&context.config),
            policy,
        )?);
    if super::super::source_binding_from_original_input(
        state,
        &head,
        &context.namespace_identity(),
    )?
    .proved_target()
        != Some(target)
    {
        return None;
    }
    let dispatcher_slot = policy
        .recipe()
        .command_publication_slot(NativeNameContext::root(), head.bytes())
        .ok()?;
    let (dispatcher, publication) = original_stock_helper(
        state,
        &dispatcher_slot,
        policy,
        SourceJimClassHelper::Dispatcher,
    )?;
    if !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?)
        || dispatcher.owner.metadata != state.baseline.metadata_context
        || !dispatcher.owner.dependencies_are_current(state)
    {
        return None;
    }
    let owner = &dispatcher.owner;
    let creator_bytes = policy
        .recipe()
        .jim_two_word_member_value(
            owner.class_name.bytes(),
            SourceJimClassHelper::Method.publication_suffix()?,
        )
        .ok()?;
    let creator_slot = policy
        .recipe()
        .command_publication_slot(NativeNameContext::root(), &creator_bytes)
        .ok()?;
    let (creator, _) =
        original_stock_helper(state, &creator_slot, policy, SourceJimClassHelper::Method)?;
    if creator.owner != *owner {
        return None;
    }
    plan_method_publication(owner, &creator_slot, original, state, context)
}

fn plan_method_publication(
    owner: &Arc<OriginalJimStockClass>,
    creator_slot: &ByteCommandSlot,
    original: &[NativeWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<PlannedJimSourceHelper> {
    let policy = owner.policy;
    let method = OriginalJimStockMethod::capture(original, owner, state, context)?;
    let bytes = policy
        .recipe()
        .jim_two_word_member_value(owner.class_name.bytes(), method.member.bytes())
        .ok()?;
    if bytes.len() > super::super::original_name_value::MAX_BYTES {
        return None;
    }
    let creator_namespace = policy
        .recipe()
        .jim_procedure_namespace(creator_slot.simple.as_bytes())
        .ok()?;
    let slot = policy
        .recipe()
        .jim_two_word_member_publication_slot(
            NativeNameContext::with_jim_namespace(&creator_slot.namespace, creator_namespace),
            owner.class_name.bytes(),
            method.member.bytes(),
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        )
        .ok()?;
    if !policy
        .recipe()
        .jim_procedure_namespace(slot.simple.as_bytes())
        .ok()?
        .is_empty()
    {
        return None;
    }
    let key = state.original_command_key_for_slot(&slot, policy)?;
    if state.original_bindings_for_key(&key)? != super::super::BTreeSet::from([MayBinding::Missing])
        || state
            .allocation_counts
            .contains_key(&(method.registration.clone(), key.clone()))
    {
        return None;
    }
    Some(PlannedJimSourceHelper {
        name: OriginalJimStockName {
            owner: Arc::clone(owner),
            helper: SourceJimClassHelper::Method,
            bytes: bytes.into(),
            method: Some(method),
        },
        slot,
        key,
        alias_target: None,
    })
}

impl SourceCommandBindings {
    pub(in crate::command_binding) fn walk_jim_stock_method(
        site: u32,
        words: &[crate::ir::WordExpr],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let plan = prepare_method(site, words, state, target, context)?;
        let mut candidate = super::super::boxed_source_branch(state);
        let mut outcomes = SourceOutcomes::default();
        let error = tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::completion::CompletionCode::Error,
        );
        outcomes.add_abrupt(error, state);
        let installed = plan.install(&mut candidate)?;
        let mut world = (*candidate.original_command_world).clone();
        world.publish_jim_stock_helper(&candidate, &installed)?;
        candidate.original_command_world = Arc::new(world);
        outcomes.add_abrupt(error, &candidate);
        outcomes.normal = Some(candidate);
        outcomes.publish(state);
        Some(outcomes)
    }
}
