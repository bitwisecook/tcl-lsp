// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One original empty-base stock parent with a complete current helper inventory.

use super::*;

fn simple_class_operand(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

pub(super) fn merged_variables(
    base: Option<&OriginalJimStockClass>,
    fields: &[std::borrow::Cow<'_, [u8]>],
) -> Vec<NameBytes> {
    let mut variables = base.map_or_else(Vec::new, |base| base.variables.clone());
    let mut seen = variables
        .iter()
        .cloned()
        .collect::<super::super::BTreeSet<_>>();
    for pair in fields.chunks_exact(2) {
        let name: NameBytes = pair[0].as_ref().into();
        if seen.insert(name.clone()) {
            variables.push(name);
        }
    }
    variables
}

pub(super) fn capture_parent(
    fields: &[std::borrow::Cow<'_, [u8]>],
    class: &[u8],
    policy: NamePolicyProtocol,
    state: &ModuleCommandBindings,
) -> Option<Arc<OriginalJimStockClass>> {
    let [base] = fields else {
        return None;
    };
    if !simple_class_operand(class) || !simple_class_operand(base) || base.as_ref() == class {
        return None;
    }
    let slot = policy
        .recipe()
        .command_publication_slot(NativeNameContext::root(), base)
        .ok()?;
    let (name, _) = super::method::original_stock_helper(
        state,
        &slot,
        policy,
        SourceJimClassHelper::Dispatcher,
    )?;
    let owner = &name.owner;
    if owner.base.is_some()
        || owner.class_name.bytes() != base.as_ref()
        || !owner.dependencies_are_current(state)
        || !parent_inventory_is_current(owner, state)
    {
        return None;
    }
    Some(Arc::clone(owner))
}

fn base_slot(
    owner: &OriginalJimStockClass,
    helper: SourceJimClassHelper,
) -> Option<ByteCommandSlot> {
    let bytes = if let Some(suffix) = helper.publication_suffix() {
        owner
            .policy
            .recipe()
            .jim_two_word_member_value(owner.class_name.bytes(), suffix)
            .ok()?
    } else {
        owner.class_name.bytes().to_vec()
    };
    owner
        .policy
        .recipe()
        .command_publication_slot(NativeNameContext::root(), &bytes)
        .ok()
}

fn current_parent_helper<'a>(
    owner: &Arc<OriginalJimStockClass>,
    state: &'a ModuleCommandBindings,
    helper: SourceJimClassHelper,
) -> Option<&'a OriginalJimStockName> {
    let slot = base_slot(owner, helper)?;
    let publication = state.original_publication_at(&slot, owner.policy)?;
    let SignatureSourceNameInput::OriginalValue(value) = publication.name_input() else {
        return None;
    };
    let name = value.jim_stock_name()?;
    if name.owner != *owner
        || name.helper != helper
        || name.method.is_some()
        || !name.matches_publication_state(state)
    {
        return None;
    }
    let key = state.original_command_key_for_slot(&slot, owner.policy)?;
    let bindings = state.bindings.get(&key)?;
    let MayBinding::Target(target) = bindings.first()? else {
        return None;
    };
    let alias = helper == SourceJimClassHelper::ConstructorAlias;
    let selected_slot = if alias {
        base_slot(owner, SourceJimClassHelper::DefaultConstructor)?
    } else {
        slot.clone()
    };
    let command =
        tcl_syntax::naming::native_command_source_spelling(owner.policy.recipe(), &selected_slot)?;
    let kind = if alias {
        super::super::source_command_world::OriginalCommandPublicationKind::Alias
    } else {
        super::super::source_command_world::OriginalCommandPublicationKind::Command
    };
    if publication.kind() != kind
        || target.command != command
        || target.target_lookup != tcl_registry::AliasTargetLookup::Global
        || bindings.len() != 1
        || target.registry_backed
        || !target.prepended.is_empty()
        || target.kind
            != if alias {
                BindingKind::Alias
            } else {
                BindingKind::Command
            }
        || target.terminal == alias
        || target.implementation_generation != owner.site.offset
        || !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?)
    {
        return None;
    }
    if alias {
        let expected = base_slot(owner, SourceJimClassHelper::DefaultConstructor)?;
        let target = publication.alias_target()?;
        if target.lookup() != tcl_registry::AliasTargetLookup::Global
            || !target.arguments().is_empty()
            || owner
                .policy
                .recipe()
                .command_publication_slot(NativeNameContext::root(), target.name_input().bytes())
                .ok()?
                != expected
        {
            return None;
        }
    } else if publication.alias_target().is_some() {
        return None;
    }
    Some(name)
}

pub(super) fn parent_helpers_are_current(
    owner: &Arc<OriginalJimStockClass>,
    state: &ModuleCommandBindings,
) -> bool {
    owner
        .recipe
        .publications()
        .iter()
        .all(|helper| current_parent_helper(owner, state, *helper).is_some())
}

fn parent_inventory_is_current(
    owner: &Arc<OriginalJimStockClass>,
    state: &ModuleCommandBindings,
) -> bool {
    if !parent_helpers_are_current(owner, state) {
        return false;
    }
    let keys = owner
        .recipe
        .publications()
        .iter()
        .map(|helper| {
            state.original_command_key_for_slot(&base_slot(owner, *helper)?, owner.policy)
        })
        .collect::<Option<super::super::BTreeSet<_>>>();
    let Some(keys) = keys else {
        return false;
    };
    if state.bindings.keys().any(|key| !keys.contains(key)) {
        return false;
    }
    let mut prefix = owner.class_name.bytes().to_vec();
    prefix.push(b' ');
    // The exact source method-list query must contain no fixed foreign row.
    for name in state
        .baseline
        .semantics
        .binding_names()
        .iter()
        .chain(state.baseline.declared_commands.keys())
        .chain(state.baseline.catalogue_commands.keys())
    {
        let Ok(slot) = owner
            .policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), name.as_bytes())
        else {
            return false;
        };
        if slot.simple.as_bytes().starts_with(&prefix) {
            return false;
        }
    }
    true
}

pub(super) fn publication_prefix(
    owner: &Arc<OriginalJimStockClass>,
    state: &ModuleCommandBindings,
) -> Option<Vec<PlannedJimSourceHelper>> {
    let Some(base) = &owner.base else {
        return Some(Vec::new());
    };
    let mut plan = Vec::new();
    for helper in owner.recipe.inherited_aliases() {
        let target = current_parent_helper(base, state, *helper)?.clone();
        plan.push(inherited_publication(owner, state, *helper, Some(target))?);
    }
    plan.push(inherited_publication(
        owner,
        state,
        SourceJimClassHelper::BaseClass,
        None,
    )?);
    Some(plan)
}

fn inherited_publication(
    owner: &Arc<OriginalJimStockClass>,
    state: &ModuleCommandBindings,
    helper: SourceJimClassHelper,
    alias_target: Option<OriginalJimStockName>,
) -> Option<PlannedJimSourceHelper> {
    let bytes = owner
        .policy
        .recipe()
        .jim_two_word_member_value(owner.class_name.bytes(), helper.publication_suffix()?)
        .ok()?;
    if bytes.len() > super::super::original_name_value::MAX_BYTES {
        return None;
    }
    let slot = if alias_target.is_some() {
        owner
            .policy
            .recipe()
            .alias_publication_slot(NativeNameContext::root(), &bytes)
    } else {
        owner
            .policy
            .recipe()
            .command_publication_slot(NativeNameContext::root(), &bytes)
    }
    .ok()?;
    let key = state.original_command_key_for_slot(&slot, owner.policy)?;
    if state.original_bindings_for_key(&key)? != super::super::BTreeSet::from([MayBinding::Missing])
        || state
            .allocation_counts
            .contains_key(&(owner.site.clone(), key.clone()))
    {
        return None;
    }
    Some(PlannedJimSourceHelper {
        name: OriginalJimStockName {
            owner: Arc::clone(owner),
            helper,
            bytes: bytes.into(),
            method: None,
        },
        slot,
        key,
        alias_target,
    })
}
