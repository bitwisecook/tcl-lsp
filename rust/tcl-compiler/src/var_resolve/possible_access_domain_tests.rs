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

#[test]
fn unknown_namespace_binding_retains_only_a_may_storage_domain() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::top");
    state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    state.widen();
    assert_eq!(
        possible_access_domain("err", &state, registry),
        PossibleVariableAccessDomain::NamespaceOnly
    );
    assert_eq!(
        resolve_literal_place("err", &state, false, registry).kind,
        PlaceKind::Unknown
    );
}

#[test]
fn native_read_success_requires_contents_kind_lifetime_and_observer_proofs() {
    for profile in ["tcl8.6", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state.bind_unknown_incoming("parameter", registry);
        let parameter = resolve_literal_place("parameter", &state, false, registry);
        assert!(state.read_produces_value(&parameter, registry));
        let missing = resolve_literal_place("missing", &state, false, registry);
        assert!(!state.read_produces_value(&missing, registry));
        state.define_literal("a(k)", "VALUE", registry);
        let root = resolve_literal_place("a", &state, false, registry);
        let element = resolve_literal_place("a(k)", &state, false, registry);
        assert_eq!(state.read_produces_value(&root, registry), profile == "jim");
        assert!(state.read_produces_value(&element, registry));
        let mut opaque = state.clone();
        opaque.invocation_dialect = None;
        assert!(!opaque.read_produces_value(&parameter, registry));
        let mut observed = state.clone();
        observed.trace_registrations.insert(
            trace_key(&parameter),
            vec![(vec![tcl_registry::TraceOperation::Read], "callback".into())],
        );
        assert!(!observed.read_produces_value(&parameter, registry));
        let mut retired = parameter.clone();
        retired.cell.as_mut().unwrap().generation = CellGeneration::After(999);
        assert!(!state.read_produces_value(&retired, registry));
        if profile == "jim" {
            state.define_literal("a", "odd", registry);
            assert!(!state.read_produces_value(&element, registry));
            state.define_literal("a", "other VALUE", registry);
            assert!(!state.read_produces_value(&element, registry));
        }
    }
}

#[test]
fn current_local_scalar_store_proves_contents_without_a_unique_incarnation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    let target = resolve_literal_place("value", &state, false, registry);
    state
        .generations
        .insert(cell_key(&target), CellGeneration::Unknown);
    let target = resolve_literal_place("value", &state, false, registry);
    state.record_contents_write(&target, 17, false);
    state.value_representations.insert(
        canonical_binding_value_name(&target).unwrap(),
        tcl_syntax::value::ValueRepresentation::Dict.into(),
    );
    assert!(state.read_produces_value(&target, registry));
    assert_eq!(
        state.contents_representation_at(&target),
        tcl_syntax::value::ValueRepresentation::Dict
    );
    assert!(canonical_place_name(&target).is_none());
    let mut uncertain = state.clone();
    uncertain.contents_origins.insert(
        canonical_binding_value_name(&target).unwrap(),
        ContentsOrigin::Unknown,
    );
    assert!(!uncertain.read_produces_value(&target, registry));
    let mut foreign = target.clone();
    foreign.cell.as_mut().unwrap().owner = CellOwner::Namespace("::".to_owned());
    assert!(!state.current_scalar_slot_contents(&foreign));
    let element = resolve_literal_place("value(k)", &state, false, registry);
    assert!(!state.read_produces_value(&element, registry));
}

#[test]
fn proved_tcl_variable_policy_does_not_require_native_compiler_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let mut state = ResolveContext::for_function("::top");
    let mut dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
    dialect.tcl_version = None;
    assert_eq!(
        dialect.variable_lookup_policy,
        Some(tcl_dialect::VariableLookupPolicy::Tcl)
    );
    state.invocation_dialect = Some(dialect);
    state.widen();
    assert_eq!(
        possible_access_domain("err", &state, registry),
        PossibleVariableAccessDomain::NamespaceOnly
    );
    assert_eq!(
        resolve_literal_place("err", &state, false, registry).kind,
        PlaceKind::Unknown
    );
}

#[test]
fn procedure_local_or_unselected_native_engine_does_not_claim_namespace_domain() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let mut state = ResolveContext::for_function("::p");
    state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    state.widen();
    assert_eq!(
        possible_access_domain("err", &state, registry),
        PossibleVariableAccessDomain::Any
    );
    assert_eq!(
        possible_access_domain("::err", &state, registry),
        PossibleVariableAccessDomain::NamespaceOnly
    );
    state.invocation_dialect = None;
    assert_eq!(
        possible_access_domain("::err", &state, registry),
        PossibleVariableAccessDomain::Any
    );
}
#[test]
fn deferred_namespace_uncertainty_preserves_local_missing_and_formal_ingress() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    state.binding_identity = BindingIdentity::Bound;
    state.define_literal("::N::external", "old", registry);
    state.bind_unknown_incoming("parameter", registry);
    state.widen_deferred_namespace_inputs();
    let external = resolve_literal_place("::N::external", &state, false, registry);
    let parameter = resolve_literal_place("parameter", &state, false, registry);
    let missing = resolve_literal_place("missing", &state, false, registry);
    assert_eq!(
        state.contents_presence(&external),
        ContentsPresence::Unknown
    );
    assert_eq!(
        state.contents_presence(&parameter),
        ContentsPresence::Defined
    );
    assert_eq!(state.contents_origin(&parameter), ContentsOrigin::Incoming);
    assert_eq!(
        state.contents_presence(&missing),
        ContentsPresence::Undefined
    );
}
#[test]
fn conditional_store_preserves_only_closed_presence_alternatives() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut state = ResolveContext::for_function("::p");
    state.binding_identity = BindingIdentity::Bound;
    state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    let target = resolve_literal_place("x", &state, false, registry);
    assert_eq!(
        state.closed_contents_presence(&target),
        Some(ContentsPresence::Undefined)
    );
    state.record_contents_write(&target, 10, true);
    assert_eq!(
        state.closed_contents_presence(&target),
        Some(ContentsPresence::DefinedOrUndefined)
    );
    assert_eq!(state.contents_presence(&target), ContentsPresence::Unknown);
    assert!(!state.read_produces_value(&target, registry));
    state.record_contents_write(&target, 20, false);
    assert_eq!(
        state.closed_contents_presence(&target),
        Some(ContentsPresence::Defined)
    );
    state.widen();
    state.record_contents_write(&target, 30, true);
    assert_eq!(state.closed_contents_presence(&target), None);
}

#[test]
fn closed_presence_join_does_not_create_a_must_value_or_survive_opacity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut missing = ResolveContext::for_function("::p");
    missing.binding_identity = BindingIdentity::Bound;
    missing.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
        registry.profile().unwrap(),
    ));
    let mut defined = missing.clone();
    defined.define_literal("x", "1", registry);
    defined.join(&missing);
    let read = resolve_literal_place("x", &defined, false, registry);
    assert_eq!(defined.contents_presence(&read), ContentsPresence::Unknown);
    assert_eq!(
        defined.closed_contents_presence(&read),
        Some(ContentsPresence::DefinedOrUndefined)
    );
    defined.widen();
    assert_eq!(defined.closed_contents_presence(&read), None);
    missing.bind_unknown_incoming("x", registry);
    let mut assigned = missing.clone();
    assigned.define_literal("x", "2", registry);
    assigned.join(&missing);
    let read = resolve_literal_place("x", &assigned, false, registry);
    assert_eq!(
        assigned.closed_contents_presence(&read),
        Some(ContentsPresence::Defined)
    );
}
