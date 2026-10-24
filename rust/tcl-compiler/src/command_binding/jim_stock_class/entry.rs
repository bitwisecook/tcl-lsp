// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional stock object-call layout for a selected original method body.

use super::{
    Arc, BindingKind, ByteCommandSlot, MayBinding, ModuleCommandBindings, NameBytes,
    NamePolicyProtocol, NativeNameContext, NativeWord, OriginalJimStockClass, OriginalJimStockName,
    SignatureSourceNameInput, SignatureSourceNameKey, SourceCommandBindings,
};

/// Source entry conditions and original dictionary-link names, never a live
/// object, entered caller frame, native variable cell or Normal certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::command_binding) struct OriginalJimStockMethodEntry {
    world: Arc<ModuleCommandBindings>,
    class: Arc<OriginalJimStockClass>,
    method: Arc<super::method::OriginalJimStockMethod>,
    local_links: Vec<NameBytes>,
    reference_prefix: Option<Arc<super::reference_prefix::OriginalJimFreshReferencePrefix>>,
}

impl OriginalJimStockMethodEntry {
    pub(in crate::command_binding) fn capture(
        bindings: &SourceCommandBindings,
        world: Arc<ModuleCommandBindings>,
        slot: &ByteCommandSlot,
        original: &[NativeWord],
    ) -> Option<Arc<Self>> {
        let policy = world.baseline.execution_name_policy?.native_recipe()?;
        super::bounded_source_entry(&world, policy)?;
        let name = current_stock_name(&world, slot, policy)?;
        let method = Arc::clone(name.method.as_ref()?);
        let class = Arc::clone(&name.owner);
        if method.original_words() != original
            || !class.dependencies_are_current(&world)
            || !super::inheritance::parent_helpers_are_current(&class, &world)
            || !entry_dependencies_are_current(&class, &world)
            || !only_original_stock_bindings(&world, policy)
        {
            return None;
        }
        let local_links = checked_link_layout(&class, &method)?;
        let reference_prefix = class.metadata.source_analysis_input().and_then(|input| {
            super::reference_prefix::OriginalJimFreshReferencePrefix::capture(
                bindings, &world, input, policy,
            )
            .filter(|prefix| prefix.matches(bindings, input))
        });
        Some(Arc::new(Self {
            world,
            class,
            method,
            local_links,
            reference_prefix,
        }))
    }

    pub(in crate::command_binding) fn has_reference_free_prefix(&self) -> bool {
        self.reference_prefix.is_some()
    }

    pub(in crate::command_binding) fn may_be_caller_command(&self, name: &[u8]) -> bool {
        self.class
            .recipe
            .object_command_footprint()
            .may_be_named_by(name)
    }

    pub(in crate::command_binding) fn policy(&self) -> NamePolicyProtocol {
        self.class.policy
    }

    pub(in crate::command_binding) fn inert_entry_argument(&self, word: &NativeWord) -> bool {
        if word.group().expand {
            return false;
        }
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&word.config());
        if SignatureSourceNameKey::from_original_native_word(word, rules, self.class.policy)
            .is_some()
        {
            return true;
        }
        let Some(name) = word.try_text().ok().and_then(|text| {
            tcl_syntax::naming::whole_scalar_variable_reference_name(text, word.config())
        }) else {
            return false;
        };
        name == self.self_name()
            || self
                .local_links
                .iter()
                .any(|link| link.as_bytes() == name.as_bytes())
    }

    pub(in crate::command_binding) fn first_command_presence(
        &self,
        name: &SignatureSourceNameInput,
        namespace: &super::super::SourceNamespaceKey,
    ) -> super::super::SourceCommandSlotPresence {
        use super::super::SourceCommandSlotPresence as Presence;
        if !self.has_reference_free_prefix()
            || name.policy() != self.class.policy
            || self.may_be_caller_command(name.bytes())
        {
            return Presence::Unknown;
        }
        let presence = self.world.original_slot_presence_for_input(name, namespace);
        self.world.original_diagnostic_presence_after_absence(
            namespace,
            name.policy(),
            presence,
            self.world
                .original_diagnostic_qualifier_is_retained(name, namespace),
        )
    }

    pub(in crate::command_binding) fn original_words(&self) -> &[NativeWord] {
        self.method.original_words()
    }

    pub(in crate::command_binding) fn input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.class.metadata.source_analysis_input()
    }

    pub(in crate::command_binding) fn owns_world(&self, world: &ModuleCommandBindings) -> bool {
        std::ptr::eq(self.world.as_ref(), world)
    }

    pub(in crate::command_binding) fn self_name(&self) -> &'static str {
        self.class.recipe.method_entry_self_name()
    }
}

pub(super) fn current_stock_name<'a>(
    world: &'a ModuleCommandBindings,
    slot: &ByteCommandSlot,
    policy: NamePolicyProtocol,
) -> Option<&'a OriginalJimStockName> {
    let publication = world.original_publication_at(slot, policy)?;
    let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
        return None;
    };
    let name = value.jim_stock_name()?;
    if !name.matches_publication_state(world)
        || name.owner.policy != policy
        || publication.declaration_site() != name.publication_site()
    {
        return None;
    }
    let key = world.original_command_key_for_slot(slot, policy)?;
    let bindings = world.bindings.get(&key)?;
    let MayBinding::Target(target) = bindings.first()? else {
        return None;
    };
    if bindings.len() != 1
        || target.registry_backed
        || !target.prepended.is_empty()
        || target.implementation_generation != name.publication_site().offset
        || !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?)
    {
        return None;
    }
    let alias = publication.alias_target();
    let kind = if alias.is_some() {
        super::super::source_command_world::OriginalCommandPublicationKind::Alias
    } else {
        super::super::source_command_world::OriginalCommandPublicationKind::Command
    };
    if publication.kind() != kind {
        return None;
    }
    let expected = if let Some(alias) = alias {
        if alias.lookup() != tcl_registry::AliasTargetLookup::Global
            || !alias.arguments().is_empty()
        {
            return None;
        }
        policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), alias.name_input().bytes())
            .ok()?
    } else {
        slot.clone()
    };
    if target.kind
        != if alias.is_some() {
            BindingKind::Alias
        } else {
            BindingKind::Command
        }
        || target.terminal == alias.is_some()
        || target.target_lookup != tcl_registry::AliasTargetLookup::Global
        || target.command
            != tcl_syntax::naming::native_command_source_spelling(policy.recipe(), &expected)?
    {
        return None;
    }
    Some(name)
}

fn only_original_stock_bindings(world: &ModuleCommandBindings, policy: NamePolicyProtocol) -> bool {
    world.bindings.keys().all(|key| {
        world
            .original_byte_slot_for_key(key, policy)
            .and_then(|slot| current_stock_name(world, &slot, policy))
            .is_some()
    })
}

fn entry_dependencies_are_current(
    class: &OriginalJimStockClass,
    world: &ModuleCommandBindings,
) -> bool {
    let Some(root) = world.source_root_namespace_key() else {
        return false;
    };
    class
        .recipe
        .method_entry_dependencies()
        .iter()
        .all(|dependency| {
            world
                .original_registry_metadata_target(&root, dependency, class.policy)
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
}

fn checked_link_layout(
    class: &OriginalJimStockClass,
    method: &super::method::OriginalJimStockMethod,
) -> Option<Vec<NameBytes>> {
    let input = class.metadata.source_analysis_input()?;
    let original = method.original_words();
    let producer = SignatureSourceNameKey::from_original_native_word(
        original.get(3)?,
        tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
        class.policy,
    )?;
    let parameters = super::super::formal_topology::OriginalFormalTopology::from_original_key(
        producer,
        super::super::source_analysis_entry::source_input_dialect(input),
    )?;
    let formals = parameters.fixed_scalar_binding_names(parameters.parameters().len())?;
    let reserved = class.recipe.method_entry_reserved_locals();
    if class.variables.len() > super::super::original_name_value::MAX_ORIGINS
        || formals
            .iter()
            .any(|name| reserved.contains(&name.as_bytes()))
        || class.variables.iter().any(|name| {
            !name
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                || name.as_bytes().is_empty()
                || reserved.contains(&name.as_bytes())
                || formals.contains(name)
        })
    {
        return None;
    }
    Some(class.variables.clone())
}
