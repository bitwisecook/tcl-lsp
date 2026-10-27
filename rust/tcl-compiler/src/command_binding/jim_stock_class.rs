// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded authored stock Jim class effects, without Native entry or class objects.

mod entry;
mod inheritance;
mod method;
mod reference_prefix;
mod source_advice;
pub(super) use entry::OriginalJimStockMethodEntry;
pub(in crate::command_binding) use source_advice::OriginalJimStockSourceEffects;

use super::{
    Arc, BindingKind, CommandAllocation, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandBindings, SourceCommandKey, SourceCommandTarget,
    SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
};
use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use tcl_core_types::{ByteCommandSlot, NameBytes};
use tcl_lexer::NativeWord;
use tcl_registry::definer::{SourceJimClassHelper, SourceJimStockClassRecipe};
use tcl_syntax::naming::{
    NamePolicyAuthority, NamePolicyProtocol, NativeNameContext, NativeNameProtocol,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalJimStockClass {
    factory: SourceCommandTarget,
    metadata: crate::registry_invocation::OwnedInvocationMetadataContext,
    original: Vec<NativeWord>,
    class_name: SignatureSourceNameInput,
    site: CommandAllocationSite,
    policy: NamePolicyProtocol,
    recipe: SourceJimStockClassRecipe,
    base: Option<Arc<OriginalJimStockClass>>,
    variables: Vec<NameBytes>,
}

/// Readonly generated-name data from one captured stock source recipe.
/// It supplies no complete word, editable span, Native argument or live helper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalJimStockName {
    owner: Arc<OriginalJimStockClass>,
    helper: SourceJimClassHelper,
    bytes: NameBytes,
    method: Option<Arc<method::OriginalJimStockMethod>>,
}

impl OriginalJimStockName {
    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    pub(crate) fn policy(&self) -> NamePolicyProtocol {
        self.owner.policy
    }

    pub(super) fn matches_publication_state(&self, state: &ModuleCommandBindings) -> bool {
        self.owner.metadata == state.baseline.metadata_context
            && state.current_source_origin.as_ref() == Some(&self.owner.site.source)
            && self.is_current(&state.source_variables)
    }

    fn publication_site(&self) -> &CommandAllocationSite {
        self.method
            .as_ref()
            .map_or(&self.owner.site, |method| method.registration())
    }

    pub(crate) fn lineage_nodes_in(&self, remaining: &mut usize) -> bool {
        self.owner.class_name.lineage_nodes_in(remaining)
            && self
                .owner
                .base
                .as_ref()
                .is_none_or(|base| base.class_name.lineage_nodes_in(remaining))
            && self
                .method
                .as_ref()
                .is_none_or(|method| method.lineage_nodes_in(remaining))
    }

    pub(crate) fn is_current(&self, context: &crate::var_resolve::ResolveContext) -> bool {
        self.owner.class_name.is_current(context)
            && self
                .owner
                .base
                .as_ref()
                .is_none_or(|base| base.class_name.is_current(context))
            && self
                .method
                .as_ref()
                .is_none_or(|method| method.is_current(context, &self.owner))
            && context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                == Some(self.owner.policy)
            && self.owner.original.first().is_some_and(|first| {
                self.owner.original.iter().all(|word| {
                    word.image() == self.owner.site.source.source_image()
                        && word.config() == first.config()
                })
            })
    }
}

pub(super) struct InstalledJimSourceHelper {
    pub(super) name: OriginalJimStockName,
    pub(super) slot: ByteCommandSlot,
    pub(super) key: SourceCommandKey,
    pub(super) allocation: CommandAllocation,
    pub(super) alias_target: Option<SignatureSourceNameInput>,
}

struct PlannedJimSourceHelper {
    name: OriginalJimStockName,
    slot: ByteCommandSlot,
    key: SourceCommandKey,
    alias_target: Option<OriginalJimStockName>,
}

impl OriginalJimStockClass {
    fn capture(
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Arc<Self>> {
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        let input = state.baseline.metadata_context.source_analysis_input()?;
        let supplied = input.borrowed_context_registry();
        let schema = tcl_registry::model::resolve_invocation_words_in_context(
            context.registry,
            Some(supplied.context()),
            *native.invocation,
        )?;
        let layout = schema.authored_source_jim_class_lists(
            tcl_registry::source_name_ownership::SourceRolePurpose::Original,
        )?;
        let recipe = layout.stock_source_recipe()?;
        if layout.policy != Some(policy) || input.lexer_config() != context.config {
            return None;
        }
        let source = state.current_source_origin.as_ref()?;
        let lexical = source.original_lexical_command(
            native.words,
            native.segment.span.start(),
            context.config,
        )?;
        let original = lexical.native_words();
        if original.iter().any(|word| word.group().expand) {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            original,
            policy.string_protocol(),
        )
        .ok()?;
        if original.len()
            != native
                .invocation
                .arguments()
                .exact_argv_len()?
                .checked_add(1)?
        {
            return None;
        }
        let fields = tcl_syntax::list::split_native_list_bytes(
            words.literal(layout.variables + 1)?,
            policy.string_protocol(),
        )
        .ok()?;
        if !fields.len().is_multiple_of(2) {
            return None;
        }
        let class_name = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                original.get(1)?,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config),
                policy,
            )?,
        );
        if class_name.bytes().is_empty() || class_name.bytes().contains(&0) {
            return None;
        }
        let base = if let Some(bases) = layout.bases {
            let bytes = words.literal(bases + 1)?;
            let fields =
                tcl_syntax::list::split_native_list_bytes(bytes, policy.string_protocol()).ok()?;
            if fields.is_empty() {
                None
            } else {
                Some(inheritance::capture_parent(
                    &fields,
                    class_name.bytes(),
                    policy,
                    state,
                )?)
            }
        } else {
            None
        };
        let variables = inheritance::merged_variables(base.as_deref(), &fields);
        Some(Arc::new(Self {
            factory: native.target.clone(),
            metadata: state.baseline.metadata_context.clone(),
            original: original.to_vec(),
            class_name,
            site: CommandAllocationSite {
                source: Arc::clone(source),
                offset: native.segment.span.start(),
            },
            policy,
            recipe,
            base,
            variables,
        }))
    }

    fn current_factory(
        &self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        if self.metadata != state.baseline.metadata_context
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || self.site.offset != native.segment.span.start()
        {
            return false;
        }
        super::original_name_value::original_static_command_head_input(
            native.words,
            native.segment.span.start(),
            state,
            context.config,
        )
        .and_then(|input| {
            super::source_binding_from_original_input(state, &input, &context.namespace_identity())
        })
        .is_some_and(|binding| binding.proved_target() == Some(&self.factory))
    }

    fn dependencies_are_current(&self, state: &ModuleCommandBindings) -> bool {
        let Some(root) = state.source_root_namespace_key() else {
            return false;
        };
        self.recipe
            .direct_dependencies()
            .iter()
            .chain(
                self.base
                    .iter()
                    .flat_map(|_| self.recipe.inheritance_dependencies()),
            )
            .all(|dependency| {
                state
                    .original_registry_metadata_target(&root, dependency, self.policy)
                    .is_some_and(|target| {
                        target.registry_backed
                            && target.kind == BindingKind::Builtin
                            && target.implementation_generation == 0
                            && target.prepended.is_empty()
                            && tcl_registry::command_descriptor_labels_match(
                                &target.command,
                                dependency,
                            )
                    })
            })
            && self
                .base
                .as_ref()
                .is_none_or(|base| inheritance::parent_helpers_are_current(base, state))
    }

    fn publication_plan(
        self: &Arc<Self>,
        state: &ModuleCommandBindings,
    ) -> Option<Vec<PlannedJimSourceHelper>> {
        let mut plan = Vec::new();
        for helper in self.recipe.publications() {
            let bytes = if let Some(suffix) = helper.publication_suffix() {
                self.policy
                    .recipe()
                    .jim_two_word_member_value(self.class_name.bytes(), suffix)
                    .ok()?
            } else {
                self.class_name.bytes().to_vec()
            };
            if bytes.len() > super::original_name_value::MAX_BYTES {
                return None;
            }
            let slot = if *helper == SourceJimClassHelper::ConstructorAlias {
                self.policy
                    .recipe()
                    .alias_publication_slot(NativeNameContext::root(), &bytes)
            } else {
                self.policy
                    .recipe()
                    .command_publication_slot(NativeNameContext::root(), &bytes)
            }
            .ok()?;
            if !self
                .policy
                .recipe()
                .jim_procedure_namespace(slot.simple.as_bytes())
                .ok()?
                .is_empty()
            {
                return None;
            }
            let key = state.original_command_key_for_slot(&slot, self.policy)?;
            if state.original_bindings_for_key(&key)?
                != super::BTreeSet::from([MayBinding::Missing])
                || state
                    .allocation_counts
                    .contains_key(&(self.site.clone(), key.clone()))
                || plan
                    .iter()
                    .any(|old: &PlannedJimSourceHelper| old.slot == slot)
            {
                return None;
            }
            plan.push(PlannedJimSourceHelper {
                name: OriginalJimStockName {
                    owner: Arc::clone(self),
                    helper: *helper,
                    bytes: bytes.into(),
                    method: None,
                },
                slot,
                key,
                alias_target: None,
            });
        }
        if self.base.is_some() {
            let mut inherited = inheritance::publication_prefix(self, state)?;
            inherited.extend(plan);
            Some(inherited)
        } else {
            Some(plan)
        }
    }
}

#[cfg(debug_assertions)]
fn trace_jim_stock_entry(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    policy: NamePolicyProtocol,
) {
    if std::env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
        eprintln!(
            "JIM_SOURCE_OWNER stage=jim.stock.fields offset={} policy={policy:?} native_entry={} source_point={} unknown_entry={} opaque={} unclosed={} unnameable={} dynamic_traces={} traced={} possible_traces={} namespace_known={} registry={} generation={} kind={:?} prepended={} traits={:?}",
            native.segment.span.start(),
            state.baseline.native_entry.is_some(),
            state.baseline.source_point_entry.is_some(),
            state.baseline.unknown_entry,
            state.opaque_domain,
            state.unclosed_command_world,
            state.unnameable_rebinding_subject,
            state.source_variables.dynamic_traces,
            state.source_variables.traced.len(),
            state.source_variables.possible_trace_registrations.len(),
            state.source_variables.namespace_known,
            native.target.registry_backed,
            native.target.implementation_generation,
            native.target.kind,
            native.target.prepended.len(),
            facts.traits,
        );
    }
}

fn bounded_source_entry(state: &ModuleCommandBindings, policy: NamePolicyProtocol) -> Option<()> {
    if policy.authority() != NamePolicyAuthority::AuthoredSimulation
        || policy.recipe() != NativeNameProtocol::Jim084
        || state.baseline.native_entry.is_some()
        || state.baseline.source_point_entry.is_some()
        || state.baseline.unknown_entry
        || state.has_opaque_domain()
        || state.unclosed_command_world
        || state.has_unnameable_rebinding_subject()
        || state.source_variables.dynamic_traces
        || !state.source_variables.traced.is_empty()
        || !state
            .source_variables
            .possible_trace_registrations
            .is_empty()
        || crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.stock.observers_active",
            (!state.command_observers.is_quiet()).then_some(()),
        )
        .is_some()
        || !state.source_variables.namespace_known
    {
        return None;
    }
    Some(())
}

fn bounded_entry(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    #[cfg(debug_assertions)]
    trace_jim_stock_entry(native, facts, state, policy);
    bounded_source_entry(state, policy)?;
    if !native.target.registry_backed
        || native.target.implementation_generation != 0
        || native.target.kind != BindingKind::Builtin
        || !native.target.prepended.is_empty()
        || !facts
            .traits
            .contains(tcl_registry::Traits::CREATES_DYNAMIC_BARRIER)
        || crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.stock.root_mismatch",
            (state.source_root_namespace_key().as_ref() != Some(&context.namespace_identity()))
                .then_some(()),
        )
        .is_some()
    {
        return None;
    }
    state
        .baseline
        .metadata_context
        .metadata_context(context.registry)?;
    let input = state.baseline.metadata_context.source_analysis_input()?;
    let selected = input
        .borrowed_context_registry()
        .context()
        .resolve_spec(context.registry, &native.target.command)?;
    (selected.definition_body?.family == tcl_registry::definer::DefinerFamily::JimClass)
        .then_some(())
}

impl PlannedJimSourceHelper {
    fn install(&self, state: &mut ModuleCommandBindings) -> Option<InstalledJimSourceHelper> {
        let alias_target = if let Some(target) = &self.alias_target {
            Some(target.clone())
        } else if self.name.helper == SourceJimClassHelper::ConstructorAlias {
            let suffix = SourceJimClassHelper::DefaultConstructor.publication_suffix()?;
            let bytes = self
                .name
                .owner
                .policy
                .recipe()
                .jim_two_word_member_value(self.name.owner.class_name.bytes(), suffix)
                .ok()?;
            Some(OriginalJimStockName {
                owner: Arc::clone(&self.name.owner),
                helper: SourceJimClassHelper::DefaultConstructor,
                bytes: bytes.into(),
                method: None,
            })
        } else {
            None
        };
        let command_slot = if let Some(target) = &alias_target {
            self.name
                .owner
                .policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), target.bytes())
                .ok()?
        } else {
            self.slot.clone()
        };
        let command = tcl_syntax::naming::native_command_source_spelling(
            self.name.owner.policy.recipe(),
            &command_slot,
        )?;
        state.install(
            self.key.clone(),
            MayBinding::Target(ResolvedCommandTarget {
                command,
                prepended: Vec::new(),
                registry_backed: false,
                kind: if alias_target.is_some() {
                    BindingKind::Alias
                } else {
                    BindingKind::Command
                },
                implementation_generation: self.name.publication_site().offset,
                implementation_allocation: None,
                terminal: alias_target.is_none(),
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
            }),
        );
        let alternatives = state.bindings.get(&self.key)?;
        let MayBinding::Target(target) = alternatives.first()? else {
            return None;
        };
        if alternatives.len() != 1 {
            return None;
        }
        Some(InstalledJimSourceHelper {
            name: self.name.clone(),
            slot: self.slot.clone(),
            key: self.key.clone(),
            allocation: target.implementation_allocation.clone()?,
            alias_target: alias_target.map(|name| {
                SignatureSourceNameInput::OriginalValue(
                    crate::signature_scan::scope::SignatureSourceNameValue::from_jim_stock_name(
                        &name,
                    ),
                )
            }),
        })
    }
}

impl SourceCommandBindings {
    pub(super) fn walk_jim_stock_class(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        super::super::registry_invocation::source_structure::trace_jim_source_option(
            "jim.stock.entry",
            bounded_entry(native, facts, state, context),
        )?;
        let owner = super::super::registry_invocation::source_structure::trace_jim_source_option(
            "jim.stock.capture",
            OriginalJimStockClass::capture(native, state, context),
        )?;
        if !owner.current_factory(native, state, context) || !owner.dependencies_are_current(state)
        {
            return None;
        }
        let plan = owner.publication_plan(state)?;
        let mut candidate = super::boxed_source_branch(state);
        let mut outcomes = SourceOutcomes::default();
        let error = tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::completion::CompletionCode::Error,
        );
        outcomes.add_abrupt(error, &candidate);
        for publication in plan {
            if !owner.dependencies_are_current(&candidate) {
                return None;
            }
            let installed = publication.install(&mut candidate)?;
            let mut world = (*candidate.original_command_world).clone();
            world.publish_jim_stock_helper(&candidate, &installed)?;
            candidate.original_command_world = Arc::new(world);
            outcomes.add_abrupt(error, &candidate);
        }
        outcomes.normal = Some(candidate);
        // This is only a possible source continuation. No Normal certificate,
        // native result, entered frame or executable class is manufactured.
        outcomes.publish(state);
        Some(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_before_unknown(prefix: &str) -> ModuleCommandBindings {
        tcl_spectcl::core_surfaces::ensure();
        let source = format!("{prefix}\nmissing_stock_contract\n");
        let result = crate::analyser::Analyser::new().analyse(&source, "jim");
        let offset = u32::try_from(source.find("missing_stock_contract").unwrap()).unwrap();
        result
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", offset)
            .lookup_state
            .as_ref()
            .unwrap()
            .state
            .clone()
    }

    #[test]
    fn empty_base_class_retains_every_stock_publication_without_native_entry() {
        let state = state_before_unknown("class C {x 0}");
        assert!(!state.unclosed_command_world);
        assert!(!state.has_opaque_domain());
        assert!(state.baseline.native_entry.is_none());
        assert!(state.baseline.source_point_entry.is_none());
        let policy = state
            .baseline
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        for name in [
            "C",
            "C new",
            "C finalize",
            "C method",
            "C vars",
            "C classvars",
            "C classname",
            "C methods",
            "C defaultconstructor",
            "C constructor",
            "C destroy",
            "C get",
            "C eval",
        ] {
            let slot = policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), name.as_bytes())
                .unwrap();
            let publication = state.original_publication_at(&slot, policy).unwrap();
            assert!(publication.name_input().original_word_key().is_none());
            assert!(publication.definition().is_none());
            if name == "C constructor" {
                assert_eq!(
                    publication.alias_target().unwrap().name_input().bytes(),
                    b"C defaultconstructor"
                );
            }
        }
    }

    #[test]
    fn earlier_unknown_and_changed_dependencies_do_not_recover_a_stock_world() {
        for source in [
            "unknown_stock_mutator; class C {x 0}",
            "proc proc args {}; class C {x 0}",
            "class Base {x 0}; class Other {z 0}; class C {Base Other} {y 0}",
        ] {
            let state = state_before_unknown(source);
            assert!(
                state.unclosed_command_world || state.has_opaque_domain(),
                "{source}"
            );
            let policy = state
                .baseline
                .execution_name_policy
                .unwrap()
                .native_recipe()
                .unwrap();
            let slot = policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), b"C vars")
                .unwrap();
            assert!(
                state.original_publication_at(&slot, policy).is_none(),
                "{source}"
            );
        }
    }
    #[test]
    fn fresh_stock_method_retains_original_source_publication_without_entry() {
        let state = state_before_unknown(
            "class C {x 0}; C method m {arg} {missing_jim_method $arg $self $x}",
        );
        assert!(!state.has_opaque_domain());
        assert!(!state.unclosed_command_world);
        assert!(state.baseline.native_entry.is_none());
        assert!(state.baseline.source_point_entry.is_none());
        let policy = state
            .baseline
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        let slot = policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), b"C m")
            .unwrap();
        let publication = state.original_publication_at(&slot, policy).unwrap();
        assert!(publication.definition().is_none());
        let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
            panic!("sealed generated source method name");
        };
        let name = value.jim_stock_name().unwrap();
        let method = name.method.as_ref().unwrap();
        assert_eq!(name.bytes(), b"C m");
        assert_eq!(publication.declaration_site(), method.registration());
    }

    #[test]
    fn stock_method_publication_keeps_missing_and_replaced_owners_opaque() {
        for prefix in [
            "class C {}; proc proc args {}; C method m {} {}",
            "class C {}; proc {C method} args {}; C method m {} {}",
            "class C {}; proc C args {}; C method m {} {}",
            "class Base {}; class Other {}; class C {Base Other} {}; C method m {} {}",
            "unknown_stock_mutator; class C {}; C method m {} {}",
            "class C {}; C method m {} {}; C method m {} {}",
        ] {
            let state = state_before_unknown(prefix);
            assert!(
                state.has_opaque_domain() || state.unclosed_command_world,
                "{prefix}"
            );
        }
    }

    #[test]
    fn one_stock_parent_retains_alias_overwrite_allocations_and_variable_owners() {
        let state = state_before_unknown("class Base {x 0}; class C Base {y 0}");
        assert!(!state.unclosed_command_world);
        assert!(!state.has_opaque_domain());
        assert!(state.baseline.native_entry.is_none());
        let policy = state
            .baseline
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        let slot = policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), b"C vars")
            .unwrap();
        let publication = state.original_publication_at(&slot, policy).unwrap();
        let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
            panic!("original stock name");
        };
        let name = value.jim_stock_name().unwrap();
        assert_eq!(
            name.owner.variables,
            vec![NameBytes::from(&b"x"[..]), NameBytes::from(&b"y"[..])]
        );
        assert!(name.owner.base.is_some());
        let key = state.original_command_key_for_slot(&slot, policy).unwrap();
        assert_eq!(
            state.allocation_counts.get(&(name.owner.site.clone(), key)),
            Some(&2)
        );
        let baseclass = policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), b"C baseclass")
            .unwrap();
        assert!(state.original_publication_at(&baseclass, policy).is_some());
    }

    #[test]
    fn former_single_base_method_refusal_has_its_original_publication_owner() {
        let state = state_before_unknown("class Base {}; class C Base {}; C method m {} {}");
        assert!(!state.unclosed_command_world);
        assert!(!state.has_opaque_domain());
        assert!(state.baseline.native_entry.is_none());
        assert!(state.baseline.source_point_entry.is_none());
        let policy = state
            .baseline
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        let slot = policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), b"C m")
            .unwrap();
        let publication = state.original_publication_at(&slot, policy).unwrap();
        assert!(publication.definition().is_none());
        let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
            panic!("original stock name");
        };
        let name = value.jim_stock_name().unwrap();
        let method = name.method.as_ref().unwrap();
        assert!(name.owner.base.is_some());
        assert_eq!(name.bytes(), b"C m");
        assert_eq!(publication.declaration_site(), method.registration());
        let key = state.original_command_key_for_slot(&slot, policy).unwrap();
        assert_eq!(
            state
                .allocation_counts
                .get(&(method.registration().clone(), key)),
            Some(&1)
        );
    }

    #[test]
    fn extra_parent_commands_and_changed_parent_helpers_keep_inheritance_opaque() {
        for source in [
            "class Base {x 0}; proc {Base foreign} {} {}; class C Base {y 0}",
            "class Base {x 0}; proc {Base vars} {} {}; class C Base {y 0}",
            "class Base {x 0}; unknown_stock_mutator; class C Base {y 0}",
        ] {
            let state = state_before_unknown(source);
            assert!(
                state.unclosed_command_world || state.has_opaque_domain(),
                "{source}"
            );
        }
    }
}
