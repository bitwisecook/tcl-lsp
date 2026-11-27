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
use tcl_names::place::{LOCAL_NS, places_read_to_form, scalar};
use tcl_names::variable::VariableExecutionFrame;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

fn ctx() -> ResolveContext {
    ResolveContext::default()
}

fn called_frame_with_unknown_namespace_contents(identity: &str) -> ResolveContext {
    let mut incoming = ResolveContext::for_namespace("::");
    incoming.contents_world = ContentsWorld::Unknown;
    incoming.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: identity.to_owned(),
    })
}

#[test]
fn fresh_called_activation_closure_is_independent_of_incoming_namespace_contents() {
    let registry = registry();
    let state = called_frame_with_unknown_namespace_contents("actual-call");
    let local = resolve_literal_place("missing", &state, false, &registry);
    let global = resolve_literal_place("::missing", &state, false, &registry);
    assert_eq!(state.contents_presence(&local), ContentsPresence::Undefined);
    assert_eq!(state.contents_presence(&global), ContentsPresence::Unknown);
    let preview =
        ResolveContext::for_namespace("::").in_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".to_owned(),
            identity: "unentered-frame".to_owned(),
        });
    assert!(preview.activation_contents_world.is_none());
}

#[test]
fn fresh_activation_observers_are_independent_of_incoming_namespace_traces() {
    let registry = registry();
    let mut incoming = ResolveContext::for_namespace("::");
    incoming.dynamic_traces = true;
    let frame = VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "actual-call".to_owned(),
    };
    let mut state = incoming.enter_called_frame(&frame);
    let local = resolve_literal_place("missing", &state, false, &registry);
    let global = resolve_literal_place("::missing", &state, false, &registry);
    assert!(!local.observed);
    assert!(global.observed);
    assert_eq!(state.contents_presence(&local), ContentsPresence::Undefined);
    assert!(
        resolve_literal_place("missing", &incoming.in_frame(&frame), false, &registry).observed
    );
    state.traced.insert(cell_key(&local));
    assert!(resolve_literal_place("missing", &state, false, &registry).observed);
    assert!(!resolve_literal_place("other", &state, false, &registry).observed);
    state.mark_unenumerated_variable_observers();
    assert!(resolve_literal_place("other", &state, false, &registry).observed);
}

#[test]
fn observed_names_project_finite_bytes_without_donating_authored_storage() {
    use tcl_syntax::naming::{
        ExecutionNamePolicy, MeasuredBigIpNameScope, NamePolicyProtocol, NativeVariableInputForm,
        ObservedBigIpNamePolicy, ObservedVariableNamePurpose,
    };
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
    let mut context = ResolveContext::for_function("::p");
    let original = "__tcl_lsp_2286_r2286m_nul_A\0B";
    context.execution_name_policy = Some(ExecutionNamePolicy::ObservedBigIp(
        ObservedBigIpNamePolicy::for_measured_scope(
            MeasuredBigIpNameScope::BigIp21_1_0_1Build0_0_26TmmHttpRequest,
        ),
    ));
    assert_eq!(
        context
            .execution_variable_name_projection(
                NativeVariableInputForm::Combined(original.as_bytes()),
                ObservedVariableNamePurpose::ScalarReceiver
            )
            .unwrap()
            .root(),
        original.as_bytes()
    );
    assert_eq!(
        resolve_literal_place(original, &context, false, registry).kind,
        PlaceKind::Unknown
    );
    assert_eq!(
        context.namespace_place(original, true, false).kind,
        PlaceKind::Unknown
    );
    assert!(
        context
            .execution_variable_name_projection(
                NativeVariableInputForm::Combined(b"qualified::unmeasured\0name"),
                ObservedVariableNamePurpose::ScalarReceiver
            )
            .is_err()
    );
    context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
        NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
    ));
    assert_eq!(
        resolve_literal_place("plain", &context, false, registry).kind,
        PlaceKind::Scalar
    );
}

#[test]
fn unknown_child_trace_registration_withdraws_exact_caller_observer_closure() {
    let registry = registry();
    let parent = called_frame_with_unknown_namespace_contents("parent-call");
    let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "child-call".to_owned(),
    });
    let untouched = restore_execution_frame(&parent, &child);
    assert!(!resolve_literal_place("missing", &untouched, false, &registry).observed);
    child.mark_unenumerated_variable_observers();
    let restored = restore_execution_frame(&parent, &child);
    assert!(resolve_literal_place("missing", &restored, false, &registry).observed);
    let mut mixed = untouched;
    mixed.join(&restored);
    assert!(resolve_literal_place("missing", &mixed, false, &registry).observed);
}

#[test]
fn called_activation_closure_withdraws_on_unknown_writes_callbacks_aliases_and_joins() {
    let registry = registry();
    let state = called_frame_with_unknown_namespace_contents("actual-call");
    let mut withdrawn = state.clone();
    withdrawn.record_contents_write(&tcl_names::place::unknown_top(), 0, true);
    let local = resolve_literal_place("missing", &withdrawn, false, &registry);
    assert_eq!(
        withdrawn.contents_presence(&local),
        ContentsPresence::Unknown
    );
    let mut callback = state.clone();
    callback.widen();
    let local = resolve_literal_place("missing", &callback, false, &registry);
    assert_eq!(
        callback.contents_presence(&local),
        ContentsPresence::Unknown
    );
    let mut mixed = state.clone();
    mixed.join(&callback);
    let local = resolve_literal_place("missing", &mixed, false, &registry);
    assert_eq!(mixed.contents_presence(&local), ContentsPresence::Unknown);
    let mut different_frame = state.clone();
    different_frame.join(&called_frame_with_unknown_namespace_contents("other-call"));
    let local = resolve_literal_place("missing", &different_frame, false, &registry);
    assert_eq!(
        different_frame.contents_presence(&local),
        ContentsPresence::Unknown
    );
    let mut alias = state;
    alias.globals.insert("missing".to_owned());
    let global_alias = resolve_literal_place("missing", &alias, false, &registry);
    assert_eq!(
        alias.contents_presence(&global_alias),
        ContentsPresence::Unknown
    );
}

#[test]
fn called_activation_closure_restores_only_its_exact_selected_frame() {
    let registry = registry();
    let parent = called_frame_with_unknown_namespace_contents("parent-call");
    let child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "child-call".to_owned(),
    });
    let restored = restore_execution_frame(&parent, &child);
    let local = resolve_literal_place("missing", &restored, false, &registry);
    assert_eq!(
        restored.contents_presence(&local),
        ContentsPresence::Undefined
    );
    let mut selected_callback = parent.clone();
    selected_callback.widen();
    let restored = restore_execution_frame(&parent, &selected_callback);
    let local = resolve_literal_place("missing", &restored, false, &registry);
    assert_eq!(
        restored.contents_presence(&local),
        ContentsPresence::Unknown
    );
}

#[test]
fn namespace_presence_join_preserves_identical_definite_cells() {
    let mut cells = NamespaceCellPresence {
        present: VariableCellSet::from(["::x".to_owned()]),
        possible: VariableCellSet::default(),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: true,
    };
    let original = cells.clone();
    assert!(!cells.join(&original));
    assert_eq!(cells, original);
    assert!(cells.absent("::y"));
}

#[test]
fn namespace_presence_join_retains_all_may_cells_and_reports_real_changes() {
    let mut left = NamespaceCellPresence {
        present: VariableCellSet::from(["::common".to_owned(), "::left".to_owned()]),
        possible: VariableCellSet::from(["::prior".to_owned()]),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: true,
    };
    let right = NamespaceCellPresence {
        present: VariableCellSet::from(["::common".to_owned(), "::right".to_owned()]),
        possible: VariableCellSet::from(["::other".to_owned()]),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: false,
    };
    assert!(left.join(&right));
    assert_eq!(left.present, VariableCellSet::from(["::common".to_owned()]));
    assert_eq!(
        left.possible,
        VariableCellSet::from([
            "::left".to_owned(),
            "::right".to_owned(),
            "::prior".to_owned(),
            "::other".to_owned(),
        ])
    );
    assert!(!left.closed);
    let fixed = left.clone();
    assert!(!left.join(&right));
    assert_eq!(left, fixed);
}

#[test]
fn namespace_presence_join_is_independent_of_predecessor_grouping() {
    let a = NamespaceCellPresence {
        present: VariableCellSet::from(["::x".to_owned(), "::y".to_owned()]),
        possible: VariableCellSet::default(),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: true,
    };
    let b = NamespaceCellPresence {
        present: VariableCellSet::from(["::y".to_owned()]),
        possible: VariableCellSet::from(["::z".to_owned()]),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: true,
    };
    let c = NamespaceCellPresence {
        present: VariableCellSet::from(["::x".to_owned()]),
        possible: VariableCellSet::from(["::x".to_owned()]),
        closed_namespaces: VariableNamespaceSet::default(),
        closed: false,
    };
    let mut left = a.clone();
    left.join(&b);
    left.join(&c);
    let mut tail = b.clone();
    tail.join(&c);
    let mut right = a.clone();
    right.join(&tail);
    assert_eq!(left, right);
    let mut reverse = c;
    reverse.join(&b);
    reverse.join(&a);
    assert_eq!(left, reverse);
}

#[test]
fn local_scalar_resolves_to_local_ns() {
    let p = resolve_place("x", &ctx(), false, &registry());
    assert_eq!(p, scalar("x", LOCAL_NS, false));
}

#[test]
fn absolute_scalar_splits_namespace_and_tail() {
    assert_eq!(
        resolve_place("::ns::v", &ctx(), false, &registry()),
        scalar("v", "::ns", false)
    );
    assert_eq!(
        resolve_place("::g", &ctx(), false, &registry()),
        scalar("g", "::", false)
    );
}

#[test]
fn global_and_variable_declarations_change_ns() {
    let mut c = ctx();
    c.globals.insert("g".to_owned());
    assert_eq!(
        resolve_place("g", &c, false, &registry()),
        scalar("g", "::", false)
    );
    let mut c = ctx();
    c.namespace = "::ns".to_owned();
    c.ns_vars.insert("v".to_owned());
    assert_eq!(
        resolve_place("v", &c, false, &registry()),
        scalar("v", "::ns", false)
    );
}

#[test]
fn literal_array_element() {
    let p = resolve_place("a(k)", &ctx(), false, &registry());
    assert_eq!(p.kind, PlaceKind::ArrayElem);
    assert_eq!(p.name, "a");
    assert_eq!(p.ns, LOCAL_NS);
    assert_eq!(p.index, Some(Index::literal("k")));
    assert_eq!(places_read_to_form(&p), [] as [tcl_names::place::Place; 0]);
}

#[test]
fn dynamic_array_element_records_index_reads() {
    // a($i) — element index reads the scalar `i`.
    let p = resolve_place("a($i)", &ctx(), false, &registry());
    assert_eq!(p.kind, PlaceKind::ArrayElem);
    assert_eq!(p.name, "a");
    assert_eq!(places_read_to_form(&p), vec![scalar("i", LOCAL_NS, false)]);
}

#[test]
fn detached_index_command_keeps_an_unknown_read_without_a_named_cell() {
    let bound = resolve_place("a([set named])", &ctx(), false, &registry());
    let reads = places_read_to_form(&bound);
    assert!(reads.iter().any(|read| read.kind == PlaceKind::Unknown));
    assert!(!reads.iter().any(|read| read.name == "named"));
    assert!(
        reads
            .iter()
            .filter(|read| read.kind == PlaceKind::Unknown)
            .all(|read| read.cell.is_none())
    );
}

#[test]
fn dynamic_name_target_is_unknown_but_reads_the_name_var() {
    // set $X v → the *target* is UNKNOWN, but `X` is read.
    let p = resolve_place("$X", &ctx(), false, &registry());
    assert_eq!(p.kind, PlaceKind::Unknown);
    assert_eq!(places_read_to_form(&p), vec![scalar("X", LOCAL_NS, false)]);
}

#[test]
fn resolved_array_elements_feed_the_overlap_precision() {
    // Distinct literal elements of the same array resolve to
    // non-overlapping places, so a store to one is not a dead store
    // (W220) against the other; a dynamic index conservatively overlaps
    // any sibling.
    let r = registry();
    let ak = resolve_place("a(k)", &ctx(), false, &r);
    let aj = resolve_place("a(j)", &ctx(), false, &r);
    assert!(!tcl_names::place::overlap(&ak, &aj));
    let dyn_elem = resolve_place("a($i)", &ctx(), false, &r);
    assert!(tcl_names::place::overlap(&dyn_elem, &ak));
}

#[test]
fn upvar_alias_binds_to_caller_target() {
    let mut c = ctx();
    c.upvar_aliases
        .insert("y".to_owned(), "::caller::x".to_owned());
    let p = resolve_place("y", &c, false, &registry());
    assert_eq!(p.kind, PlaceKind::UpvarAlias);
    assert_eq!(p.owner, "::caller::x");
    assert!(!p.dynamic);
    // A dynamic (empty-target) upvar alias is marked dynamic.
    let mut c = ctx();
    c.upvar_aliases.insert("z".to_owned(), String::new());
    let p = resolve_place("z", &c, false, &registry());
    assert_eq!(p.kind, PlaceKind::UpvarAlias);
    assert!(p.dynamic);
}

#[test]
fn whole_array_flag_yields_array_whole() {
    let p = resolve_place("a", &ctx(), true, &registry());
    assert_eq!(p.kind, PlaceKind::ArrayWhole);
    assert_eq!(p.name, "a");
}

#[test]
fn traced_name_is_observed() {
    let mut c = ctx();
    c.traced.insert("x".to_owned());
    let p = resolve_place("x", &c, false, &registry());
    assert!(p.observed);
}

#[test]
fn dict_path_classifies_keys() {
    let keys = vec!["a".to_owned(), "b".to_owned()];
    let p = resolve_dict_path("d", &keys, &ctx(), &registry());
    assert_eq!(p.kind, PlaceKind::DictPath);
    assert_eq!(p.name, "d");
    assert_eq!(p.keys, vec![Index::literal("a"), Index::literal("b")]);
}
#[test]
fn unresolved_array_bindings_stay_unknown_and_dict_elements_keep_indices() {
    let registry = registry();
    let mut context = ResolveContext::for_function("::f");
    context.unknown_bindings.insert("a".to_owned());
    assert_eq!(
        resolve_place("a(k)", &context, false, &registry).kind,
        PlaceKind::Unknown
    );
    assert_eq!(
        resolve_dict_path("a(k)", &["key".to_owned()], &context, &registry).kind,
        PlaceKind::Unknown
    );
    context.unknown_bindings.clear();
    let first = resolve_dict_path("a(k)", &["key".to_owned()], &context, &registry);
    let second = resolve_dict_path("a(j)", &["key".to_owned()], &context, &registry);
    assert_eq!(first.index, Some(Index::literal("k")));
    assert!(!place::overlap(&first, &second));
}

#[test]
fn receiver_method_locals_do_not_imply_a_receiver_namespace() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut root = ResolveContext::for_function("::top");
    root.invocation_dialect = registry
        .profile()
        .map(tcl_native_model::InvocationDialect::of_profile);
    let frame = VariableExecutionFrame::ReceiverMethod {
        identity: "receiver-method:17".to_owned(),
    };
    let method = root.in_frame(&frame);
    assert!(!method.namespace_known);
    assert_eq!(
        method.alias_frame(),
        tcl_registry::VariableAliasFrame::Procedure
    );
    let local = resolve_literal_place("x", &method, false, registry);
    assert!(matches!(local.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Activation(identity)) if identity == "receiver-method:17"));
    assert_eq!(
        resolve_literal_place("N::x", &method, false, registry).kind,
        PlaceKind::Unknown
    );
    assert_eq!(
        canonical_binding_value_name(&resolve_literal_place("::x", &method, false, registry)),
        Some("::x".to_owned())
    );
    let relocation = VariableProofRelocation {
        activations: std::collections::BTreeMap::from([(
            "receiver-method:17".to_owned(),
            "receiver-method:29".to_owned(),
        )]),
        ..VariableProofRelocation::default()
    };
    assert_eq!(
        relocation.frame(&frame),
        VariableExecutionFrame::ReceiverMethod {
            identity: "receiver-method:29".to_owned(),
        }
    );
}

#[test]
fn restored_namespace_alias_uses_the_latest_slot_binding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut parent = ResolveContext::for_function("::top");
    parent.invocation_dialect = Some(tcl_native_model::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    parent.define_literal("a", "OLD", registry);
    parent.define_literal("b", "NEW", registry);
    let a = resolve_literal_place("a", &parent, false, registry);
    let b = resolve_literal_place("b", &parent, false, registry);
    let slot = resolve_alias_destination_slot("link", &parent, registry);
    parent.alias_bindings.insert("link".to_owned(), a.clone());
    parent
        .namespace_alias_bindings
        .insert(cell_key(&slot), a.clone());
    let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "callback:17".to_owned(),
    });
    child.namespace_alias_bindings.insert(cell_key(&slot), b);
    let restored = restore_execution_frame(&parent, &child);
    assert_eq!(restored.literal_value("link", registry), Some("NEW"));
    assert_eq!(restored.literal_value("::link", registry), Some("NEW"));
    // A procedure-local link still selects its own activation slot.
    child.alias_bindings.insert("link".to_owned(), a);
    assert_eq!(child.literal_value("link", registry), Some("OLD"));
}

#[test]
fn frame_restoration_preserves_outward_trace_and_invalidates_deleted_aliases() {
    let registry = registry();
    let mut parent = ResolveContext::for_function("::N::p");
    let target = resolve_place("::N::x", &parent, false, &registry);
    parent.alias_bindings.insert("alias".to_owned(), target);
    let mut child = parent.in_frame(&VariableExecutionFrame::NamespaceActivation {
        namespace: "::N".to_owned(),
        identity: "namespace:20".to_owned(),
    });
    child.traced.insert("::N::x".to_owned());
    let restored = restore_execution_frame(&parent, &child);
    assert!(restored.alias_bindings.contains_key("alias"));
    assert!(resolve_place("alias", &restored, false, &registry).observed);
    child.known_namespaces.remove("::N");
    let restored = restore_execution_frame(&parent, &child);
    assert_eq!(
        resolve_place("alias", &restored, false, &registry).kind,
        PlaceKind::Unknown
    );
}

#[test]
fn jim_namespace_activations_have_separate_cells_from_caller_and_siblings() {
    let registry = registry();
    let mut driver = ResolveContext::default();
    let mut dialect =
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    dialect.variable_lookup_policy = Some(tcl_dialect::VariableLookupPolicy::Jim);
    dialect.namespace_var_global_fallback = Some(false);
    driver.invocation_dialect = Some(dialect);
    let caller = driver.in_frame(&VariableExecutionFrame::Procedure {
        namespace: "::N".to_owned(),
        identity: "proc:N::p".to_owned(),
    });
    let first = caller.in_frame(&VariableExecutionFrame::NamespaceActivation {
        namespace: "::N".to_owned(),
        identity: "namespace:20".to_owned(),
    });
    let second = caller.in_frame(&VariableExecutionFrame::NamespaceActivation {
        namespace: "::N".to_owned(),
        identity: "namespace:40".to_owned(),
    });
    let cells: Vec<_> = [&caller, &first, &second]
        .into_iter()
        .map(|context| resolve_place("x", context, false, &registry))
        .collect();
    assert!(cells.iter().all(|cell| matches!(
        cell.cell.as_ref().map(|cell| &cell.owner),
        Some(CellOwner::Activation(_))
    )));
    assert!(!place::overlap(&cells[0], &cells[1]));
    assert!(!place::overlap(&cells[1], &cells[2]));
    assert_ne!(
        canonical_place_name(&cells[0]),
        canonical_place_name(&cells[1])
    );
}

#[test]
fn tcl8_relative_namespace_lookup_requires_cell_existence_not_namespace_alone() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut context = ResolveContext::for_function("::N::p");
    context.known_namespaces.insert("::N::R".to_owned());
    context.namespace_cells.present.insert("::R::x".to_owned());
    assert!(canonical_variable_name("R::x", &context, registry).is_none());
    context.namespace_cells.closed = true;
    assert_eq!(
        canonical_variable_name("R::x", &context, registry).as_deref(),
        Some("::R::x")
    );
    context
        .namespace_cells
        .present
        .insert("::N::R::x".to_owned());
    assert_eq!(
        canonical_variable_name("R::x", &context, registry).as_deref(),
        Some("::N::R::x")
    );
}

#[test]
fn tcl9_relative_namespace_lookup_never_uses_global_variable_fallback() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let mut context = ResolveContext::for_function("::N::p");
    context.known_namespaces.insert("::N::R".to_owned());
    context.namespace_cells.present.insert("::R::x".to_owned());
    assert_eq!(
        canonical_variable_name("R::x", &context, registry).as_deref(),
        Some("::N::R::x")
    );
}

#[test]
fn unknown_interpreter_policy_does_not_choose_a_qualified_variable_table() {
    let registry = CommandRegistry::build_default();
    let mut context = ResolveContext::for_function("::N::p");
    context.known_namespaces.insert("::N::R".to_owned());
    context
        .namespace_cells
        .present
        .insert("::N::R::x".to_owned());
    assert!(canonical_variable_name("R::x", &context, &registry).is_none());
    assert_eq!(
        canonical_variable_name("::N::R::x", &context, &registry).as_deref(),
        Some("::N::R::x")
    );
}
#[test]
fn written_target_braces_preserve_literal_indices_and_dollar_names() {
    let registry = registry();
    let context = ResolveContext::for_function("::p");
    let dynamic = resolve_target_access(
        "a($i)",
        false,
        &context,
        &registry,
        tcl_registry::TraceOperation::Write,
    );
    assert_eq!(
        dynamic.index.as_ref().map(|index| index.kind),
        Some(place::IndexKind::Dynamic)
    );
    assert!(
        places_read_to_form(&dynamic)
            .iter()
            .any(|place| place.name == "i")
    );
    let literal = resolve_target_access(
        "a($i)",
        true,
        &context,
        &registry,
        tcl_registry::TraceOperation::Write,
    );
    assert_eq!(
        literal.index.as_ref().map(|index| index.kind),
        Some(place::IndexKind::Literal)
    );
    assert_eq!(places_read_to_form(&literal), [] as [Place; 0]);
    assert_eq!(
        resolve_target_access(
            "$x",
            true,
            &context,
            &registry,
            tcl_registry::TraceOperation::Write
        )
        .name,
        "$x"
    );
    assert_eq!(
        resolve_target_access(
            "$x",
            false,
            &context,
            &registry,
            tcl_registry::TraceOperation::Write
        )
        .kind,
        PlaceKind::Unknown
    );
}
#[test]
fn namespace_access_origin_retains_explicit_selection_and_imported_aliases() {
    use tcl_registry::TraceOperation::Unset;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let root = ResolveContext::for_function("::top");
    assert_eq!(
        literal_namespace_access_origin("bare", &root, registry, Unset),
        NamespaceAccessOrigin::CurrentFrame
    );
    assert_eq!(
        literal_namespace_access_origin("::bare", &root, registry, Unset),
        NamespaceAccessOrigin::ExplicitName
    );
    assert_eq!(
        literal_namespace_access_origin("a(::index)", &root, registry, Unset),
        NamespaceAccessOrigin::CurrentFrame
    );
    let mut local = ResolveContext::for_function("::p");
    let target = resolve_literal_place("::shared", &local, false, registry);
    local.alias_bindings.insert("alias".to_owned(), target);
    assert_eq!(
        literal_namespace_access_origin("alias", &local, registry, Unset),
        NamespaceAccessOrigin::NonLocalBinding
    );
    assert_eq!(
        literal_namespace_access_origin("ordinary", &local, registry, Unset),
        NamespaceAccessOrigin::Unknown
    );
}

#[test]
fn unknown_incoming_value_has_provenance_until_a_later_unknown_store() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut frame = ResolveContext::for_function("::p");
    frame.bind_unknown_incoming("parameter", registry);
    let parameter = resolve_literal_access(
        "parameter",
        &frame,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        frame.contents_presence(&parameter),
        ContentsPresence::Defined
    );
    assert_eq!(frame.contents_origin(&parameter), ContentsOrigin::Incoming);
    assert_eq!(frame.literal_contents_at(&parameter, registry), None);
    assert_eq!(
        frame.incoming_activation_slot("${parameter}", "parameter", registry),
        parameter.cell
    );
    frame.define_unknown_contents("parameter", registry);
    assert_eq!(frame.contents_origin(&parameter), ContentsOrigin::Unknown);
    assert!(
        frame
            .incoming_activation_slot("${parameter}", "parameter", registry)
            .is_none()
    );
}

#[test]
fn incoming_activation_binding_does_not_donate_provenance_to_namespace_storage() {
    let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
    let mut frame = ResolveContext::for_function("::p");
    frame.bind_unknown_incoming("::external", registry);
    let external = resolve_literal_access(
        "::external",
        &frame,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(frame.contents_origin(&external), ContentsOrigin::Unknown);
    assert!(
        frame
            .incoming_activation_slot("${::external}", "::external", registry)
            .is_none()
    );
    assert_eq!(
        frame.contents_presence(&external),
        ContentsPresence::Defined
    );
}

#[test]
fn existence_proof_distinguishes_unknown_formal_contents_from_missing_cell() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut frame = ResolveContext::for_function("::p");
    frame.define_unknown_contents("parameter", registry);
    let parameter = resolve_literal_access(
        "parameter",
        &frame,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    let missing = resolve_literal_access(
        "missing",
        &frame,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        frame.existence_after_read_at(&parameter, registry),
        Some(true)
    );
    assert_eq!(
        frame.existence_after_read_at(&missing, registry),
        Some(false)
    );
    frame.define_unknown_contents("a(b)", registry);
    let array = resolve_literal_access(
        "a",
        &frame,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(frame.existence_after_read_at(&array, registry), Some(true));
    let mut observed = parameter;
    observed.observed = true;
    assert_eq!(frame.existence_after_read_at(&observed, registry), None);
}

#[test]
fn closed_container_coercion_is_cost_only_and_withdraws_unknown_inputs() {
    use tcl_syntax::value::ValueRepresentation;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::f");
    state.define_literal_with_representation("d", "a 1", ValueRepresentation::Dict, registry);
    let place = resolve_literal_access(
        "d",
        &state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    state.coerce_ordinary_container_representations(None, ValueRepresentation::List);
    let domain = state
        .container_representation_alternatives_at(&place)
        .unwrap();
    assert!(domain.contains(ValueRepresentation::List));
    assert!(domain.contains(ValueRepresentation::Dict));
    assert_eq!(
        state.contents_representation_at(&place),
        ValueRepresentation::Unknown
    );
    let mut observed = place.clone();
    observed.observed = true;
    assert!(
        state
            .container_representation_alternatives_at(&observed)
            .is_none()
    );
    state.invalidate_shared_representations();
    assert!(
        state
            .container_representation_alternatives_at(&place)
            .is_none()
    );
}

#[test]
fn closed_representation_joins_keep_only_complete_ordinary_alternatives() {
    use tcl_syntax::value::ValueRepresentation;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut left = ResolveContext::for_function("::f");
    left.define_literal_with_representation("d", "a 1", ValueRepresentation::Dict, registry);
    let mut right = left.clone();
    right.define_literal_with_representation("d", "a 1", ValueRepresentation::List, registry);
    left.join(&right);
    let place = resolve_literal_access(
        "d",
        &left,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    let domain = left
        .container_representation_alternatives_at(&place)
        .unwrap();
    assert!(domain.contains(ValueRepresentation::List));
    assert!(domain.contains(ValueRepresentation::Dict));
    assert_eq!(
        left.contents_representation_at(&place),
        ValueRepresentation::Unknown
    );
    right.invalidate_shared_representations();
    left.join(&right);
    assert!(
        left.container_representation_alternatives_at(&place)
            .is_none()
    );
}

#[test]
fn known_coerced_bytes_preserve_only_disjoint_current_container_representations() {
    use tcl_syntax::value::ValueRepresentation;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::f");
    state.define_literal_with_representation("same", "a 1", ValueRepresentation::Dict, registry);
    state.define_literal_with_representation(
        "different",
        "b 2",
        ValueRepresentation::Dict,
        registry,
    );
    state.define_unknown_contents_with_representation(
        "unknown",
        ValueRepresentation::List,
        registry,
    );
    let before = state.representation_epoch;
    state.invalidate_representations_for_values(Some(&["a 1".to_owned()]));
    assert_ne!(state.representation_epoch, before);
    for (name, expected) in [
        ("same", ValueRepresentation::Unknown),
        ("different", ValueRepresentation::Dict),
        ("unknown", ValueRepresentation::Unknown),
    ] {
        let place = resolve_literal_place(name, &state, false, registry);
        assert_eq!(state.contents_representation_at(&place), expected, "{name}");
    }
    let mut unknown = state.clone();
    unknown.invalidate_representations_for_values(None);
    let different = resolve_literal_place("different", &unknown, false, registry);
    assert_eq!(
        unknown.contents_representation_at(&different),
        ValueRepresentation::Unknown
    );
}

#[test]
fn coercion_in_child_frame_does_not_restore_ancestor_representation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut parent = ResolveContext::for_function("::parent");
    parent.define_literal_with_representation(
        "items",
        "1 2",
        tcl_syntax::value::ValueRepresentation::List,
        registry,
    );
    let mut child = parent.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "child".to_owned(),
    });
    child.invalidate_shared_representations();
    let selected = child
        .selected_frame_context(tcl_registry::FrameLevel::Relative(1))
        .unwrap();
    let selected_items = resolve_literal_place("items", &selected, false, registry);
    assert_eq!(
        selected.contents_representation_at(&selected_items),
        tcl_syntax::value::ValueRepresentation::Unknown
    );
    let restored = restore_execution_frame(&parent, &child);
    let items = resolve_literal_place("items", &restored, false, registry);
    assert_eq!(restored.literal_contents_at(&items, registry), Some("1 2"));
    assert_eq!(
        restored.contents_representation_at(&items),
        tcl_syntax::value::ValueRepresentation::Unknown
    );
}

#[test]
fn implicit_precision_observer_blocks_raw_values_through_aliases_but_not_locals() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::top");
    state.define_literal("tcl_precision", "0x4", registry);
    assert!(state.literal_value("tcl_precision", registry).is_none());
    assert!(
        state
            .substitution_literal_value("$tcl_precision", registry)
            .is_none()
    );
    let stored = resolve_literal_access(
        "tcl_precision",
        &state,
        false,
        registry,
        tcl_registry::TraceOperation::Write,
    );
    assert_eq!(
        state.literal_contents_for_access_at(
            &stored,
            registry,
            tcl_registry::TraceOperation::Write
        ),
        Some("0x4")
    );
    let global = resolve_literal_place("::tcl_precision", &state, false, registry);
    state
        .alias_bindings
        .insert("alias".to_owned(), global.clone());
    assert!(state.literal_value("alias", registry).is_none());
    assert_eq!(
        state.read_contents_origin(&global, registry),
        ContentsOrigin::Unknown
    );
    let mut local = ResolveContext::for_function("::p");
    local.define_literal("tcl_precision", "0x4", registry);
    assert_eq!(local.literal_value("tcl_precision", registry), Some("0x4"));
}

#[test]
fn physical_observer_projection_keeps_root_order_lifo_and_unknown_residual() {
    use tcl_registry::TraceOperation::{Read, Write};
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    let root = resolve_literal_place("a", &state, false, registry);
    let element = resolve_literal_place("a(k)", &state, false, registry);
    let root_key = trace_key(&root);
    let element_key = trace_key(&element);
    state.traced.extend([root_key.clone(), element_key.clone()]);
    state.trace_registrations.insert(
        root_key,
        vec![(vec![Write], "first".into()), (vec![Write], "last".into())],
    );
    state
        .trace_registrations
        .insert(element_key.clone(), vec![(vec![Write], "element".into())]);
    let projection = state.variable_observers_at(&element, Write, registry);
    assert!(!projection.unknown_residual);
    assert_eq!(
        projection
            .callbacks
            .iter()
            .map(|callback| match &callback.prefix {
                VariableTracePrefix::Authored(prefix) => prefix.as_str(),
                VariableTracePrefix::Original(_) => panic!("authored fixture"),
            })
            .collect::<Vec<_>>(),
        ["last", "first", "element"]
    );
    assert_eq!(
        state
            .variable_observers_at(&element, Read, registry)
            .callbacks
            .len(),
        0
    );
    state.untracked_traces.insert(element_key);
    assert!(
        state
            .variable_observers_at(&element, Write, registry)
            .unknown_residual
    );
}

#[test]
fn an_absent_physical_root_proves_its_elements_absent() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    let element = resolve_literal_access(
        "a(key)",
        &state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        state.contents_presence(&element),
        ContentsPresence::Undefined
    );
    let any = resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Read);
    assert_eq!(state.contents_presence(&any), ContentsPresence::Undefined);
    state.contents_world = ContentsWorld::Unknown;
    assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
    state.contents_world = ContentsWorld::Tracked;
    state.define_unknown_contents("a", registry);
    assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
    state.dynamic_traces = true;
    let observed = resolve_literal_access(
        "missing(key)",
        &state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        state.contents_presence(&observed),
        ContentsPresence::Unknown
    );
}

#[test]
fn unknown_element_store_masks_only_its_array_and_later_named_store_recovers() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    state.activation = Some("frame".to_owned());
    let element = resolve_literal_place("a(x)", &state, false, registry);
    let untouched = resolve_literal_place("b(x)", &state, false, registry);
    state.record_contents_write(&element, 10, false);
    state.define_literal("a(x)", "OLD", registry);
    state.record_contents_write(&untouched, 11, false);
    state.define_literal("b(x)", "KEPT", registry);
    let any =
        resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Write);
    state.record_contents_write(&any, 20, false);
    assert!(state.literal_value("a(x)", registry).is_none());
    let untracked = resolve_literal_place("a(untracked)", &state, false, registry);
    assert_eq!(state.contents_origin(&untracked), ContentsOrigin::Unknown);
    assert_eq!(
        state.contents_origin(&untouched),
        ContentsOrigin::WrittenAt(11)
    );
    assert_eq!(state.literal_value("b(x)", registry), Some("KEPT"));
    state.record_contents_write(&element, 30, false);
    assert_eq!(
        state.contents_origin(&element),
        ContentsOrigin::WrittenAt(30)
    );
    let mut branch = state.clone();
    branch
        .contents_origins
        .remove(&canonical_binding_value_key(&element).unwrap());
    state.join(&branch);
    assert_eq!(state.contents_origin(&element), ContentsOrigin::Unknown);
}

#[test]
fn unknown_element_destruction_keeps_root_but_not_element_existence() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    let element = resolve_literal_place("a(x)", &state, false, registry);
    let root = resolve_literal_place("a", &state, false, registry);
    state.record_contents_write(&element, 1, false);
    let any =
        resolve_array_element_root("a", &state, registry, tcl_registry::TraceOperation::Unset);
    state.record_unknown_element_destruction(&any);
    assert_eq!(state.contents_presence(&element), ContentsPresence::Unknown);
    assert_eq!(state.contents_presence(&root), ContentsPresence::Defined);
    assert_eq!(state.contents_origin(&element), ContentsOrigin::Unknown);
}

#[test]
fn physical_proof_relocation_round_trips_aliases_contents_and_observers() {
    let registry = CommandRegistry::build_default();
    let mut context = ResolveContext::for_function("::p");
    context.activation = Some("actual-frame".to_owned());
    let mut target = resolve_literal_place("a(k)", &context, false, &registry);
    target.cell.as_mut().expect("bound cell").generation = CellGeneration::After(77);
    context
        .alias_bindings
        .insert("alias".to_owned(), target.clone());
    context
        .name_alias_bindings
        .insert("name_alias".to_owned(), target.clone());
    context
        .generations
        .insert(cell_key(&target), CellGeneration::After(77));
    context.contents_origins.insert(
        canonical_place_name(&target).unwrap(),
        ContentsOrigin::WrittenAt(77),
    );
    context
        .constant_values
        .insert(canonical_place_name(&target).unwrap(), "literal".to_owned());
    context.traced.insert(trace_key(&target));
    context.trace_registrations.insert(
        trace_key(&target),
        vec![(vec![tcl_registry::TraceOperation::Read], "callback".into())],
    );
    context.caller = Some(std::sync::Arc::new(ResolveContext::for_function(
        "::external",
    )));
    context
        .contents_unknown_arrays
        .insert(physical_array_key(&target).unwrap());
    let relocation = VariableProofRelocation {
        activations: std::collections::BTreeMap::from([(
            "actual-frame".to_owned(),
            "template-frame".to_owned(),
        )]),
        source_offsets: std::collections::BTreeMap::from([(77, 7)]),
        ..VariableProofRelocation::default()
    };
    let relocated = context.relocated(&relocation);
    assert_eq!(relocated.activation.as_deref(), Some("template-frame"));
    assert_eq!(
        relocated
            .alias_bindings
            .get("alias")
            .unwrap()
            .cell
            .as_ref()
            .unwrap()
            .generation,
        CellGeneration::After(7)
    );
    assert_eq!(
        relocated.caller, context.caller,
        "unmapped caller stays an exact dependency"
    );
    assert_eq!(relocated.relocated(&relocation.inverse().unwrap()), context);
}

#[test]
fn worker_overlap_preserves_unknown_selection_and_initialisation_epochs() {
    use crate::var_resolve::{ResolveContext, resolve_literal_place};
    use tcl_names::place::overlap;
    use tcl_registry::f5::WorkerExecution;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let resolve = |worker, initialisation_epoch, connection| {
        let mut context = ResolveContext::for_function("::event");
        context.known_namespaces.insert("::static".to_owned());
        context.execution = Some(WorkerExecution {
            worker,
            initialisation_epoch,
            connection,
        });
        resolve_literal_place("::static::counter", &context, false, registry)
    };
    let first = resolve(Some(0), 1, Some(10));
    assert!(overlap(&first, &resolve(Some(0), 1, Some(20))));
    assert!(!overlap(&first, &resolve(Some(1), 1, Some(10))));
    assert!(!overlap(&first, &resolve(Some(0), 2, Some(10))));
    let unknown = resolve(None, 1, None);
    assert!(overlap(&first, &unknown));
    assert!(overlap(&unknown, &resolve(Some(1), 1, None)));
    assert_ne!(first.cell, unknown.cell);
}
