// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact runtime variable operands. Source compiler operands have an independent purpose.

use super::{
    CellGeneration, CellIdentity, CellOwner, Index, Place, PlaceKind, ResolveContext,
    bind_cell_identity, bind_scalar_receiver, cell_key, project_access, resolve_name_alias,
};
use tcl_core_types::ByteNamespacePath;
use tcl_registry::{CommandRegistry, TraceOperation};
use tcl_syntax::naming::{
    NativeNameContext, NativeNameProtocol, NativeVariableInputForm, NativeVariableRootGeometry,
};

/// Select an authentic readonly naming input through runtime lookup. Complete
/// original words and produced values remain distinct provenance categories.
pub(crate) fn resolve_original_name_input(
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
    context: &ResolveContext,
    registry: &CommandRegistry,
    whole_array: bool,
    operation: TraceOperation,
) -> Place {
    if !input.is_current(context) {
        return crate::place::unknown_top();
    }
    resolve_evaluated_variable_input(
        NativeVariableInputForm::Combined(input.bytes()),
        context,
        whole_array,
        registry,
        operation,
    )
}

/// Resolve independently produced separate root/index inputs. The index is not
/// encoded from a display String or coerced into an original source word.
pub(crate) fn resolve_original_element_inputs(
    root: &crate::signature_scan::scope::SignatureSourceNameInput,
    element: &crate::signature_scan::scope::SignatureSourceNameInput,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    if root.policy() != element.policy()
        || !root.is_current(context)
        || !element.is_current(context)
    {
        return crate::place::unknown_top();
    }
    resolve_evaluated_variable_input(
        NativeVariableInputForm::Separate {
            root: root.bytes(),
            element: Some(element.bytes()),
        },
        context,
        false,
        registry,
        operation,
    )
}

/// Resolve an already evaluated counted runtime name through its retained recipe.
/// This selects cells and applicable observers; it issues no compiler-local,
/// object-header, native frame, or guaranteed-completion certificate.
/// Missing recipe/namespace currency and conflicting dialects remain unknown.
#[must_use]
pub fn resolve_evaluated_variable_input(
    input: NativeVariableInputForm<'_>,
    context: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    let Some(policy) = context
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return crate::place::unknown_top();
    };
    let protocol = policy.recipe();
    if context.invocation_dialect.and_then(|dialect| {
        dialect.native_name_protocol().or_else(|| {
            dialect
                .authored_name_policy()
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
        })
    }) != Some(protocol)
        || context
            .namespace_name_protocol
            .is_some_and(|selected| selected != protocol)
        || context.observed_variable_storage_unavailable()
    {
        return crate::place::unknown_top();
    }
    let selected = match input {
        NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
        NativeVariableInputForm::Separate { root, element } => {
            let parsed = match protocol {
                NativeNameProtocol::C(version) => {
                    tcl_syntax::native_variable_name::NativeVariableNameProtocol::for_tcl_version(
                        version,
                    )
                    .parsed_array_parts(root)
                    .is_some()
                }
                NativeNameProtocol::Jim084 => {
                    protocol.combined_variable_input(root).element().is_some()
                }
            };
            if parsed {
                if element.is_some() {
                    return crate::place::unknown_top();
                }
                protocol.combined_variable_input(root)
            } else {
                protocol.separate_variable_input(root, element)
            }
        }
    };
    let mut place = exact_root(selected.root().selected(), context, registry, protocol);
    if place.kind == PlaceKind::Unknown {
        return place;
    }
    if let Some(element) = selected.element() {
        if whole_array || place.index.is_some() {
            return crate::place::unknown_top();
        }
        place.kind = PlaceKind::ArrayElem;
        place.index = Some(Index::literal(element.selected()));
    } else if whole_array {
        if place.index.is_some() {
            return crate::place::unknown_top();
        }
        place.kind = PlaceKind::ArrayWhole;
    }
    project_access(place, context, operation)
}

/// Resolve a direct runtime alias destination without following an older link.
/// The caller owns the materialised operand and selected alias grammar.
pub(crate) fn resolve_original_alias_destination_bytes(
    bytes: &[u8],
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let mut direct = context.clone();
    direct.raw_bindings = crate::raw_binding::RawBindingArena::default();
    direct.alias_bindings.clear();
    direct.namespace_alias_bindings.clear();
    direct.name_alias_bindings.clear();
    direct.namespace_name_alias_bindings.clear();
    direct.globals.clear();
    direct.ns_vars.clear();
    direct.upvar_aliases.clear();
    direct.instance_vars.clear();
    direct.original_receiver_variable_candidates = None;
    direct.unknown_bindings.clear();
    direct.dynamic_bindings = false;
    // Namespace alias creation never uses the ordinary global fallback.
    let Some(policy) = direct
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return crate::place::unknown_top();
    };
    let parsed = policy.recipe().combined_variable_input(bytes);
    if !policy.recipe().is_jim084()
        && (direct.namespace_scope()
            || parsed.root().qualification()
                != tcl_syntax::naming::NativeNameQualification::Unqualified)
    {
        let Some(namespace) = direct.namespace_identity.as_ref() else {
            return crate::place::unknown_top();
        };
        return resolve_original_namespace_variable_bytes(
            bytes, namespace, &direct, registry, true,
        );
    }
    let mut slot = resolve_evaluated_variable_input(
        NativeVariableInputForm::Combined(bytes),
        &direct,
        false,
        registry,
        TraceOperation::Read,
    );
    slot.observed = false;
    slot
}

/// A namespace variable operand selects an independently retained table. The
/// combined-name parser and index projection remain the runtime naming owner.
pub(crate) fn resolve_original_namespace_variable_bytes(
    bytes: &[u8],
    namespace: &crate::command_binding::SourceNamespaceKey,
    context: &ResolveContext,
    registry: &CommandRegistry,
    destination: bool,
) -> Place {
    let Some(policy) = context
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return crate::place::unknown_top();
    };
    let protocol = policy.recipe();
    if context.invocation_dialect.and_then(|dialect| {
        dialect.native_name_protocol().or_else(|| {
            dialect
                .authored_name_policy()
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
        })
    }) != Some(protocol)
        || !context.namespace_identities.contains(namespace)
        || context.observed_variable_storage_unavailable()
    {
        return crate::place::unknown_top();
    }
    let parsed = protocol.combined_variable_input(bytes);
    let mut bound = context.namespace_place_bytes_in_identity(
        parsed.root().selected(),
        namespace,
        false,
        protocol,
    );
    if bound.kind == PlaceKind::Unknown {
        return bound;
    }
    bind_cell_identity(&mut bound, context);
    let key = cell_key(&bound);
    if !destination {
        if context.unknown_bindings.contains(&key) {
            return crate::place::unknown_top();
        }
        if let Some(raw) = context.raw_bindings.bindings.get(&key) {
            bound = context.raw_bindings.resolve(raw, context, registry);
        } else if let Some(alias) = context
            .namespace_name_alias_bindings
            .get(&key)
            .or_else(|| context.name_alias_bindings.get(&key))
        {
            bound = resolve_name_alias(alias, context);
        } else if let Some(alias) = context
            .namespace_alias_bindings
            .get(&key)
            .or_else(|| context.alias_bindings.get(&key))
        {
            bound = alias.clone();
        }
    }
    if let Some(index) = parsed.element() {
        if bound.index.is_some() {
            return crate::place::unknown_top();
        }
        bound.kind = PlaceKind::ArrayElem;
        bound.index = Some(Index::literal(index.selected()));
    }
    if destination {
        bound.observed = false;
        bound
    } else {
        project_access(bound, context, TraceOperation::Read)
    }
}

/// A compiler-selected primary keeps its full counted declaration key. Its
/// local owner remains the independently entered activation, never an LVT token.
pub(crate) fn resolve_original_compiled_variable(
    name: &[u8],
    index: Option<&[u8]>,
    lookup: tcl_syntax::naming::NativeCompiledVariableLookup,
    compiler: tcl_syntax::naming::NativeCompiledVariableProtocol,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    use tcl_syntax::naming::NativeCompiledVariableLookup;
    if !compiled_policy_matches(compiler, context) {
        return crate::place::unknown_top();
    }
    if lookup == NativeCompiledVariableLookup::DynamicName {
        return resolve_evaluated_variable_input(
            NativeVariableInputForm::Separate {
                root: name,
                element: index,
            },
            context,
            false,
            registry,
            operation,
        );
    }
    let Some(primary) = original_compiled_variable_primary(name, lookup, compiler, context) else {
        return crate::place::unknown_top();
    };
    resolve_compiled_local_primary(primary.as_bytes(), index, context, registry, operation)
}

fn compiled_policy_matches(
    compiler: tcl_syntax::naming::NativeCompiledVariableProtocol,
    context: &ResolveContext,
) -> bool {
    use tcl_syntax::naming::NativeCompiledVariableRecipe;
    let Some(policy) = context
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    else {
        return false;
    };
    if !matches!((compiler.recipe(), policy.recipe()),
        (NativeCompiledVariableRecipe::C(left), NativeNameProtocol::C(right)) if left == right)
        && !matches!(
            (compiler.recipe(), policy.recipe()),
            (
                NativeCompiledVariableRecipe::Jim084,
                NativeNameProtocol::Jim084
            )
        )
    {
        return false;
    }
    true
}

/// Consume a primary issued only from a complete original compiler inventory.
/// Runtime resolver, observer and exact-cell lifetime gates remain independent.
pub(crate) fn resolve_original_compiled_primary(
    primary: &crate::command_binding::original_variable_compilation::OriginalCompiledVariablePrimary,
    index: Option<&[u8]>,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    if !compiled_policy_matches(primary.compiler(), context) {
        return crate::place::unknown_top();
    }
    resolve_compiled_local_primary(primary.bytes(), index, context, registry, operation)
}

fn resolve_compiled_local_primary(
    primary: &[u8],
    index: Option<&[u8]>,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    let Some(activation) = context.activation.as_ref() else {
        return crate::place::unknown_top();
    };
    if context.dynamic_bindings {
        return crate::place::unknown_top();
    }
    let mut bound = crate::place::scalar(
        std::str::from_utf8(primary).unwrap_or_default(),
        crate::place::LOCAL_NS,
        false,
    );
    bound.cell = Some(CellIdentity {
        owner: CellOwner::Activation(activation.clone()),
        name: primary.into(),
        generation: CellGeneration::Incoming,
        interpreter: context.interpreter.clone(),
        storage_domain: None,
        execution: context.execution,
    });
    bind_cell_identity(&mut bound, context);
    let key = cell_key(&bound);
    let authored = std::str::from_utf8(primary).ok();
    if context.unknown_bindings.contains(&key)
        || authored.is_some_and(|name| context.unknown_bindings.contains(name))
    {
        return crate::place::unknown_top();
    }
    // These are explicitly authored declaration/link records. Their local
    // spelling is queried only after the independent compiler selected this
    // primary; it cannot choose or truncate a different compiled name.
    if let Some(raw) = context.raw_bindings.bindings.get(&key) {
        bound = context.raw_bindings.resolve(raw, context, registry);
    } else if let Some(alias) = context
        .namespace_name_alias_bindings
        .get(&key)
        .or_else(|| context.name_alias_bindings.get(&key))
        .or_else(|| authored.and_then(|name| context.name_alias_bindings.get(name)))
    {
        bound = resolve_name_alias(alias, context);
    } else if let Some(alias) = context
        .namespace_alias_bindings
        .get(&key)
        .or_else(|| context.alias_bindings.get(&key))
        .or_else(|| authored.and_then(|name| context.alias_bindings.get(name)))
    {
        bound = alias.clone();
    } else if let Some(selected) = context
        .original_receiver_variable_candidates
        .as_ref()
        .and_then(|candidates| {
            candidates.select(
                primary,
                tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                context,
            )
        })
    {
        bound = selected;
    } else if let Some(name) = authored
        && (context.globals.contains(name)
            || context.ns_vars.contains(name)
            || context.instance_vars.contains(name)
            || context.upvar_aliases.contains_key(name))
    {
        bound = bind_scalar_receiver(name, context, Some(false), registry);
    }
    if let Some(index) = index {
        if bound.index.is_some() || bound.kind == PlaceKind::Unknown {
            return crate::place::unknown_top();
        }
        bound.kind = PlaceKind::ArrayElem;
        bound.index = Some(Index::literal(index));
    }
    project_access(bound, context, operation)
}

/// An independently selected compiler local declaration denotes its direct
/// activation slot before existing aliases are followed. This names the slot
/// only; contents, observers and successful link installation remain separate.
pub(crate) fn original_compiled_local_destination(
    primary: &tcl_core_types::NameBytes,
    context: &ResolveContext,
) -> Place {
    let Some(activation) = context.activation.as_ref() else {
        return crate::place::unknown_top();
    };
    if context.selected_frame.is_some() {
        return crate::place::unknown_top();
    }
    let mut slot = crate::place::scalar(
        primary.try_utf8().unwrap_or_default(),
        crate::place::LOCAL_NS,
        false,
    );
    slot.cell = Some(CellIdentity {
        owner: CellOwner::Activation(activation.clone()),
        name: primary.clone(),
        generation: CellGeneration::Incoming,
        interpreter: context.interpreter.clone(),
        storage_domain: None,
        execution: context.execution,
    });
    bind_cell_identity(&mut slot, context);
    slot
}

pub(crate) fn original_compiled_variable_primary(
    name: &[u8],
    lookup: tcl_syntax::naming::NativeCompiledVariableLookup,
    compiler: tcl_syntax::naming::NativeCompiledVariableProtocol,
    context: &ResolveContext,
) -> Option<tcl_core_types::NameBytes> {
    context
        .original_formal_topology
        .as_ref()
        .and_then(|formals| formals.first_compiled_formal_name(name, compiler, false))
        .or_else(|| {
            (lookup == tcl_syntax::naming::NativeCompiledVariableLookup::CreateLocal
                && compiler.compiled_local_name_is_byte_unique(name))
            .then_some(name)
        })
        .map(Into::into)
}

pub(super) fn exact_root(
    root: &[u8],
    context: &ResolveContext,
    registry: &CommandRegistry,
    protocol: NativeNameProtocol,
) -> Place {
    // Explicit authored links and instance declarations remain checked text adapters.
    // Native/opaque keys continue below through exact cell ownership.
    let authored_bound = if let Ok(text) = std::str::from_utf8(root) {
        let bound = bind_scalar_receiver(text, context, Some(false), registry);
        if bound.kind != PlaceKind::Unknown && bound.cell.is_some() {
            Some(bound)
        } else {
            if context.unknown_bindings.contains(text) || context.dynamic_bindings {
                return crate::place::unknown_top();
            }
            None
        }
    } else {
        None
    };
    let root_path = ByteNamespacePath::root();
    let namespace_path = context
        .namespace_identity
        .as_ref()
        .and_then(|key| key.exact_native_path())
        .unwrap_or(&root_path);
    let geometry = protocol.variable_root_geometry(NativeNameContext::new(namespace_path), root);
    let bound_was_selected = authored_bound.is_some();
    let mut bound = match authored_bound {
        Some(bound) => bound,
        None => exact_unbound_root(root, context, protocol, geometry),
    };
    if bound.kind == PlaceKind::Unknown {
        return bound;
    }
    if !bound_was_selected {
        bind_cell_identity(&mut bound, context);
    }
    let key = cell_key(&bound);
    if context.unknown_bindings.contains(&key) {
        return crate::place::unknown_top();
    }
    if let Some(raw) = context.raw_bindings.bindings.get(&key) {
        return context.raw_bindings.resolve(raw, context, registry);
    }
    if let Some(alias) = context
        .namespace_name_alias_bindings
        .get(&key)
        .or_else(|| context.name_alias_bindings.get(&key))
    {
        return resolve_name_alias(alias, context);
    }
    if let Some(alias) = context
        .namespace_alias_bindings
        .get(&key)
        .or_else(|| context.alias_bindings.get(&key))
    {
        return alias.clone();
    }
    if bound.cell.as_ref().is_some_and(|cell| {
        matches!(&cell.owner, CellOwner::Activation(activation)
            if context.activation.as_ref() == Some(activation))
            && cell.name.as_bytes() == root
    }) && let Some(selected) = context
        .original_receiver_variable_candidates
        .as_ref()
        .and_then(|candidates| {
            candidates.select(
                root,
                tcl_syntax::naming::NativeOoVariableResolverPurpose::RuntimeRoot,
                context,
            )
        })
    {
        return selected;
    }
    bound
}

fn exact_unbound_root(
    root: &[u8],
    context: &ResolveContext,
    protocol: NativeNameProtocol,
    geometry: NativeVariableRootGeometry,
) -> Place {
    match geometry {
        NativeVariableRootGeometry::JimAbsolute(_)
        | NativeVariableRootGeometry::CNamespace { .. } => {
            let Some(namespace) = context.namespace_identity.as_ref() else {
                return crate::place::unknown_top();
            };
            exact_namespace_root(root, namespace, context, protocol)
        }
        NativeVariableRootGeometry::Local(simple)
            if context.namespace_scope() && !protocol.is_jim084() || context.global_frame() =>
        {
            let Some(namespace) = context.namespace_identity.as_ref() else {
                return crate::place::unknown_top();
            };
            exact_namespace_root(simple.as_bytes(), namespace, context, protocol)
        }
        NativeVariableRootGeometry::Local(simple) => {
            if context.dynamic_bindings {
                return crate::place::unknown_top();
            }
            let Some(activation) = context.activation.as_ref() else {
                return crate::place::unknown_top();
            };
            let mut bound = crate::place::scalar(
                simple.try_utf8().unwrap_or_default(),
                crate::place::LOCAL_NS,
                false,
            );
            bound.cell = Some(CellIdentity {
                owner: CellOwner::Activation(activation.clone()),
                name: simple,
                generation: CellGeneration::Incoming,
                interpreter: context.interpreter.clone(),
                storage_domain: None,
                execution: context.execution,
            });
            bound
        }
    }
}

fn exact_namespace_root(
    root: &[u8],
    namespace: &crate::command_binding::SourceNamespaceKey,
    context: &ResolveContext,
    protocol: NativeNameProtocol,
) -> Place {
    let current = context.namespace_place_bytes_in_identity(root, namespace, false, protocol);
    if current.kind == PlaceKind::Unknown
        || protocol.is_jim084()
        || context.global_frame()
        || protocol.variable_root_input(root).qualification()
            == tcl_syntax::naming::NativeNameQualification::Absolute
    {
        return current;
    }
    let Some(fallback) = context
        .invocation_dialect
        .and_then(|dialect| dialect.namespace_var_global_fallback)
    else {
        return crate::place::unknown_top();
    };
    if !fallback
        || context
            .namespace_cells
            .present
            .contains(&cell_key(&current))
    {
        return current;
    }
    let Some(global_namespace) = context.root_namespace_identity() else {
        return crate::place::unknown_top();
    };
    let global =
        context.namespace_place_bytes_in_identity(root, &global_namespace, false, protocol);
    if global.kind == PlaceKind::Unknown {
        return crate::place::unknown_top();
    }
    if cell_key(&current) == cell_key(&global) {
        return current;
    }
    if context.namespace_cells.absent(&cell_key(&current))
        && context.namespace_cells.present.contains(&cell_key(&global))
    {
        return global;
    }
    if context.namespace_cells.absent(&cell_key(&global))
        && current.cell.as_ref().is_some_and(|cell| {
            matches!(&cell.owner, CellOwner::NamespaceIdentity(identity)
                if context.namespace_identities.contains(identity.as_ref()))
        })
    {
        return current;
    }
    crate::place::unknown_top()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceNamespaceKey;
    use tcl_dialect::TclVersion;
    use tcl_syntax::naming::{ExecutionNamePolicy, NamePolicyProtocol};

    fn context(version: TclVersion) -> ResolveContext {
        let mut context = ResolveContext::for_function("::f");
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            NamePolicyProtocol::authored_tcl(version),
        ));
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(version));
        context.retain_namespace_world(
            SourceNamespaceKey::authored("::"),
            [SourceNamespaceKey::authored("::")],
            Some(NativeNameProtocol::C(version)),
        );
        context
    }

    // Native proof: naming.variable.namespace-store-frame-and-fallback
    // docs/design/analysis/name-resolution-proofs/namespace-store-frame-and-fallback.md
    fn assert_opaque_namespace_root_membership(
        state: &mut ResolveContext,
        namespace: &SourceNamespaceKey,
        root: &SourceNamespaceKey,
        protocol: NativeNameProtocol,
        version: TclVersion,
        registry: &CommandRegistry,
    ) {
        // Opaque byte-cell equality is an independent Rust correspondence
        // assertion; the native address control above uses ASCII x.
        let opaque_global =
            state.namespace_place_bytes_in_identity(b"x\xed\xa0\x80", root, false, protocol);
        let opaque_current =
            state.namespace_place_bytes_in_identity(b"x\xed\xa0\x80", namespace, false, protocol);
        state
            .namespace_cells
            .present
            .insert(cell_key(&opaque_global));
        state
            .namespace_cells
            .closed_namespaces
            .insert(namespace.clone());
        let opaque = resolve_evaluated_variable_input(
            NativeVariableInputForm::Combined(b"x\xed\xa0\x80"),
            state,
            false,
            registry,
            TraceOperation::Read,
        );
        assert_eq!(
            cell_key(&opaque),
            cell_key(if version < TclVersion::V9_0 {
                &opaque_global
            } else {
                &opaque_current
            })
        );
    }

    #[test]
    fn namespace_runtime_root_requires_current_or_global_table_membership() {
        // naming.variable.namespace-store-frame-and-fallback
        // docs/design/analysis/name-resolution-proofs/namespace-store-frame-and-fallback.md
        let registry = CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            let root = SourceNamespaceKey::authored("::");
            let namespace = SourceNamespaceKey::authored("::N");
            let mut state = ResolveContext::for_namespace("::N");
            state.execution_name_policy = context(version).execution_name_policy;
            state.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(version));
            state.retain_namespace_world(
                namespace.clone(),
                [root.clone(), namespace.clone()],
                Some(protocol),
            );
            let current =
                state.namespace_place_bytes_in_identity(b"x", &namespace, false, protocol);
            let global = state.namespace_place_bytes_in_identity(b"x", &root, false, protocol);
            state.namespace_cells.present.insert(cell_key(&global));
            state
                .namespace_cells
                .closed_namespaces
                .insert(namespace.clone());
            let read = resolve_evaluated_variable_input(
                NativeVariableInputForm::Combined(b"x"),
                &state,
                false,
                &registry,
                TraceOperation::Read,
            );
            assert_eq!(
                cell_key(&read),
                cell_key(if version < TclVersion::V9_0 {
                    &global
                } else {
                    &current
                })
            );
            state.namespace_cells.closed_namespaces.clear();
            if version < TclVersion::V9_0 {
                assert_eq!(
                    resolve_evaluated_variable_input(
                        NativeVariableInputForm::Combined(b"x"),
                        &state,
                        false,
                        &registry,
                        TraceOperation::Read
                    )
                    .kind,
                    PlaceKind::Unknown
                );
            }
            state.namespace_cells.present.insert(cell_key(&current));
            assert_eq!(
                cell_key(&resolve_evaluated_variable_input(
                    NativeVariableInputForm::Combined(b"x"),
                    &state,
                    false,
                    &registry,
                    TraceOperation::Read
                )),
                cell_key(&current)
            );
            state.namespace_cells.present.clear();
            state.namespace_cells.closed_namespaces.insert(root.clone());
            assert_eq!(
                cell_key(&resolve_evaluated_variable_input(
                    NativeVariableInputForm::Combined(b"x"),
                    &state,
                    false,
                    &registry,
                    TraceOperation::Read
                )),
                cell_key(&current)
            );
            assert_opaque_namespace_root_membership(
                &mut state, &namespace, &root, protocol, version, &registry,
            );
        }
    }

    #[test]
    fn compiled_primary_and_dynamic_name_do_not_share_raw_nul_projection() {
        use tcl_syntax::naming::{NativeCompiledVariableLookup, NativeCompiledVariableProtocol};
        for version in TclVersion::ALL {
            let registry = tcl_registry::model::ingress::static_context_for(match version {
                TclVersion::V8_4 => "tcl8.4",
                TclVersion::V8_5 => "tcl8.5",
                TclVersion::V8_6 => "tcl8.6",
                TclVersion::V9_0 => "tcl9.0",
                TclVersion::V9_1 => "tcl9.1",
            })
            .commands();
            let context = context(version);
            let compiler = NativeCompiledVariableProtocol::authored_tcl(version);
            let primary_lookup = resolve_original_compiled_variable(
                b"v\0tail",
                None,
                NativeCompiledVariableLookup::CreateLocal,
                compiler,
                &context,
                registry,
                TraceOperation::Read,
            );
            // A raw-NUL source key requires an ordered same-owner candidate
            // inventory; the compiler comparison can select a different name.
            assert_eq!(primary_lookup.kind, PlaceKind::Unknown);
            let runtime = resolve_original_compiled_variable(
                b"v\0tail",
                None,
                NativeCompiledVariableLookup::DynamicName,
                compiler,
                &context,
                registry,
                TraceOperation::Read,
            );
            assert_eq!(
                runtime.cell.as_ref().unwrap().name.as_bytes(),
                if version == TclVersion::V8_4 {
                    b"v".as_slice()
                } else {
                    b"v\0tail"
                }
            );
            let unique = resolve_original_compiled_variable(
                b"v\xc0\x80tail",
                Some(b"k\0tail"),
                NativeCompiledVariableLookup::CreateLocal,
                compiler,
                &context,
                registry,
                TraceOperation::Read,
            );
            assert_eq!(
                unique.cell.as_ref().unwrap().name.as_bytes(),
                b"v\xc0\x80tail"
            );
            assert_eq!(unique.index.as_ref().unwrap().value.as_bytes(), b"k\0tail");
        }
    }

    #[test]
    // Implementation contract: naming.variable.byte-cell-correspondence
    // docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
    fn compiler_primary_retains_authored_unknown_links_and_exact_namespace_targets() {
        use tcl_syntax::naming::{NativeCompiledVariableLookup, NativeCompiledVariableProtocol};
        for version in TclVersion::ALL {
            let registry = CommandRegistry::build_default();
            let mut context = context(version);
            let compiler = NativeCompiledVariableProtocol::authored_tcl(version);
            let select = |name: &[u8], context: &ResolveContext| {
                resolve_original_compiled_variable(
                    name,
                    None,
                    NativeCompiledVariableLookup::CreateLocal,
                    compiler,
                    context,
                    &registry,
                    TraceOperation::Read,
                )
            };
            context.unknown_bindings.insert("linked");
            assert_eq!(select(b"linked", &context).kind, PlaceKind::Unknown);
            context.unknown_bindings.clear();
            let namespace = context.namespace_identity.as_ref().unwrap().clone();
            let target = context.namespace_place_bytes_in_identity(
                b"counter",
                &namespace,
                false,
                NativeNameProtocol::C(version),
            );
            context.alias_bindings.insert("linked", target.clone());
            assert_eq!(cell_key(&select(b"linked", &context)), cell_key(&target));
            context.globals.insert("counter".into());
            assert_eq!(cell_key(&select(b"counter", &context)), cell_key(&target));
            // A later exact alias can replace an earlier global declaration.
            let replacement = context.namespace_place_bytes_in_identity(
                b"other",
                &namespace,
                false,
                NativeNameProtocol::C(version),
            );
            let mut local = crate::place::scalar("counter", crate::place::LOCAL_NS, false);
            bind_cell_identity(&mut local, &context);
            context
                .alias_bindings
                .insert(cell_key(&local), replacement.clone());
            assert_eq!(
                cell_key(&select(b"counter", &context)),
                cell_key(&replacement)
            );
        }
    }

    #[test]
    // Implementation contract: naming.variable.byte-cell-correspondence
    // docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
    fn counted_runtime_roots_indices_aliases_and_observers_share_one_cell_owner() {
        let registry = CommandRegistry::build_default();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut context = context(version);
            let root = b"v\xed\xa0\x80";
            let input = NativeVariableInputForm::Separate {
                root,
                element: Some(b"k\xed\xa0\x81"),
            };
            let indexed = resolve_evaluated_variable_input(
                input,
                &context,
                false,
                &registry,
                TraceOperation::Read,
            );
            assert_eq!(indexed.kind, PlaceKind::ArrayElem);
            assert_eq!(indexed.cell.as_ref().unwrap().name.as_bytes(), root);
            assert_eq!(
                indexed.index.as_ref().unwrap().value.as_bytes(),
                b"k\xed\xa0\x81"
            );
            let exact = cell_key(&indexed);
            context.trace_registrations.insert(
                exact.clone(),
                vec![(vec![TraceOperation::Write], "watch".into())],
            );
            assert!(
                !resolve_evaluated_variable_input(
                    input,
                    &context,
                    false,
                    &registry,
                    TraceOperation::Read
                )
                .observed
            );
            assert!(
                resolve_evaluated_variable_input(
                    input,
                    &context,
                    false,
                    &registry,
                    TraceOperation::Write
                )
                .observed
            );
            let target = resolve_evaluated_variable_input(
                NativeVariableInputForm::Separate {
                    root: b"other\xff",
                    element: None,
                },
                &context,
                false,
                &registry,
                TraceOperation::Read,
            );
            context.alias_bindings.insert(exact, target.clone());
            let alias = resolve_evaluated_variable_input(
                NativeVariableInputForm::Separate {
                    root,
                    element: None,
                },
                &context,
                false,
                &registry,
                TraceOperation::Read,
            );
            assert_eq!(cell_key(&alias), cell_key(&target));
        }
    }

    // Implementation contract: naming.variable.byte-cell-correspondence
    // docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
    #[test]
    fn counted_runtime_input_rejects_missing_or_conflicting_recipe_and_preserves_extent() {
        let registry = CommandRegistry::build_default();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut context = context(version);
            let input = NativeVariableInputForm::Separate {
                root: b"v\0tail",
                element: Some(b"k\0tail"),
            };
            let exact = resolve_evaluated_variable_input(
                input,
                &context,
                false,
                &registry,
                TraceOperation::Read,
            );
            assert_eq!(
                exact.cell.as_ref().unwrap().name.as_bytes(),
                if version == TclVersion::V8_4 {
                    b"v".as_slice()
                } else {
                    b"v\0tail".as_slice()
                }
            );
            // Separate array-element operands have a distinct version-selected extent.
            assert_eq!(
                exact.index.as_ref().unwrap().value.as_bytes(),
                if version == TclVersion::V8_4 {
                    b"k".as_slice()
                } else {
                    b"k\0tail".as_slice()
                }
            );
            context.execution_name_policy = None;
            assert_eq!(
                resolve_evaluated_variable_input(
                    input,
                    &context,
                    false,
                    &registry,
                    TraceOperation::Read
                )
                .kind,
                PlaceKind::Unknown
            );
            context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
                NamePolicyProtocol::authored_jim084(),
            ));
            assert_eq!(
                resolve_evaluated_variable_input(
                    input,
                    &context,
                    false,
                    &registry,
                    TraceOperation::Read
                )
                .kind,
                PlaceKind::Unknown
            );
        }
    }
}
