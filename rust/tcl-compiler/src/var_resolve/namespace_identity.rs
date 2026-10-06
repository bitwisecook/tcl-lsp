// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Variable table selection without inverting a namespace display string.

use super::{
    CellGeneration, CellIdentity, CellOwner, Place, ResolveContext, authored_namespace_place, place,
};
use crate::command_binding::SourceNamespaceKey;
use tcl_core_types::ByteNamespacePath;
use tcl_syntax::naming::{NativeNameProtocol, NativeNameQualification};

impl ResolveContext {
    /// Select a retained current namespace without closing the namespace or contents world.
    #[must_use]
    pub fn with_namespace_identity(mut self, key: SourceNamespaceKey) -> Self {
        if let Some(display) = key.display() {
            self.namespace = display;
        }
        self.namespace_identities.insert(key.clone());
        self.namespace_identity = Some(key);
        self
    }

    /// Retain the selected namespace table and reached table inventory.
    /// Native geometry and incarnation remain authoritative when displays collide.
    pub fn retain_namespace_world(
        &mut self,
        current: SourceNamespaceKey,
        namespaces: impl IntoIterator<Item = SourceNamespaceKey>,
        protocol: Option<NativeNameProtocol>,
    ) {
        if let Some(display) = current.display() {
            self.namespace = display;
        }
        self.namespace_known = true;
        self.namespace_addressable_identities = namespaces.into_iter().collect();
        self.namespace_identities = self.namespace_addressable_identities.clone();
        self.namespace_identities.insert(current.clone());
        self.namespace_identity = Some(current);
        self.namespace_name_protocol = protocol;
    }

    /// Exact namespace footprint retained by a scoped unresolved access.
    pub(crate) fn namespace_footprint(&self, place: &Place) -> Option<SourceNamespaceKey> {
        if let Some(cell) = &place.cell {
            return match &cell.owner {
                CellOwner::NamespaceIdentity(namespace) => Some(namespace.as_ref().clone()),
                CellOwner::Namespace(namespace) => Some(SourceNamespaceKey::authored(namespace)),
                _ => None,
            };
        }
        // Legacy symbolic footprints never select an actual namespace by display.
        self.namespace_identity
            .as_ref()
            .is_none_or(|key| matches!(key, SourceNamespaceKey::Authored(_)))
            .then(|| SourceNamespaceKey::authored(&place.ns))
    }

    /// Native WRITE/UNSET publication callbacks after the original cell and links resolve.
    pub(super) fn authored_static_observer_may_run(
        &self,
        place: &Place,
        operation: tcl_registry::TraceOperation,
    ) -> bool {
        use tcl_runtime_api::authored_tmm::TmmStaticExecutionContext;
        if !matches!(
            operation,
            tcl_registry::TraceOperation::Write | tcl_registry::TraceOperation::Unset
        ) {
            return false;
        }
        let Some(receipt) = &self.authored_tmm_static else {
            return false;
        };
        if receipt.context == Some(TmmStaticExecutionContext::ExecutingWorker) {
            return false;
        }
        place
            .cell
            .as_ref()
            .and_then(authored_static_cell_owner)
            .and_then(SourceNamespaceKey::native_context)
            == Some(&receipt.namespace)
            && !receipt.outward_observers.permits_no_callbacks()
    }

    /// Original root namespace table from the retained inventory.
    pub(crate) fn root_namespace_identity(&self) -> Option<SourceNamespaceKey> {
        self.identity_for_path(&ByteNamespacePath::root())
            .or_else(|| {
                self.namespace_identity
                    .as_ref()
                    .filter(|identity| matches!(identity, SourceNamespaceKey::Authored(_)))
                    .map(|_| SourceNamespaceKey::authored("::"))
            })
    }

    fn identity_for_path(&self, path: &ByteNamespacePath) -> Option<SourceNamespaceKey> {
        let mut candidates = self
            .namespace_addressable_identities
            .iter()
            .filter(|key| key.exact_native_path() == Some(path));
        let first = candidates.next()?.clone();
        candidates.next().is_none().then_some(first)
    }

    /// Resolve an original written namespace operand using retained components.
    pub(crate) fn namespace_identity_for_written(
        &self,
        written: &str,
    ) -> Option<SourceNamespaceKey> {
        let current = self.namespace_identity.as_ref()?;
        if let SourceNamespaceKey::Authored(namespace) = current {
            return Some(SourceNamespaceKey::authored(
                tcl_syntax::naming::qualify_namespace(namespace, written),
            ));
        }
        let protocol = self.namespace_name_protocol?;
        if protocol.is_jim084() {
            let object = self.namespace_objects.get(current)?;
            let selected = protocol
                .namespace_address_input(
                    tcl_syntax::naming::NativeNameContext::with_jim_namespace(
                        current.exact_native_path()?,
                        object.as_bytes(),
                    ),
                    written.as_bytes(),
                )
                .ok()?;
            let mut keys = self.namespace_objects.iter().filter(|(key, object)| {
                self.namespace_addressable_identities.contains(*key)
                    && object.as_bytes() == selected.selected()
            });
            let (key, _) = keys.next()?;
            return keys.next().is_none().then(|| key.clone());
        }
        let path = protocol
            .namespace_address_path(
                tcl_syntax::naming::NativeNameContext::new(current.exact_native_path()?),
                written.as_bytes(),
            )
            .ok()?;
        self.identity_for_path(&path)
    }

    /// Closed absence of the original candidate table, distinct from absent variable contents.
    pub(super) fn namespace_candidate_absent(&self, name: &str, global: bool) -> bool {
        if !self.namespace_inventory.is_closed() {
            return false;
        }
        let Some(protocol) = self
            .namespace_name_protocol
            .filter(|protocol| !protocol.is_jim084())
        else {
            return false;
        };
        let Some(current) = self
            .namespace_identity
            .as_ref()
            .and_then(SourceNamespaceKey::exact_native_path)
        else {
            return false;
        };
        let root = ByteNamespacePath::root();
        let Some((path, _)) =
            c_variable_parts(protocol, if global { &root } else { current }, name)
        else {
            return false;
        };
        let original = protocol.variable_root_input(name.as_bytes());
        let retained_current = !global
            && original.qualification() != NativeNameQualification::Absolute
            && path == *current;
        !retained_current
            && !self
                .namespace_addressable_identities
                .iter()
                .any(|key| key.exact_native_path() == Some(&path))
    }

    pub(super) fn namespace_place(&self, name: &str, global: bool, observed: bool) -> Place {
        let Some(current) = self.namespace_identity.as_ref() else {
            return authored_namespace_place(
                name,
                if global { "::" } else { &self.namespace },
                observed,
            );
        };
        if let SourceNamespaceKey::Authored(namespace) = current {
            return authored_namespace_place(name, if global { "::" } else { namespace }, observed);
        }
        let identity = if global {
            self.identity_for_path(&ByteNamespacePath::root())
        } else {
            Some(current.clone())
        };
        let Some(identity) = identity else {
            return place::unknown_top();
        };
        self.namespace_place_in_identity(name, &identity, observed)
    }

    /// Resolve an original variable operand in a selected exact namespace table.
    pub(crate) fn namespace_place_in_identity(
        &self,
        name: &str,
        namespace: &SourceNamespaceKey,
        observed: bool,
    ) -> Place {
        let Some(protocol) = self.namespace_name_protocol else {
            return place::unknown_top();
        };
        let selected = protocol.variable_root_input(name.as_bytes());
        let Ok(selected_name) = selected.try_utf8() else {
            return place::unknown_top();
        };
        let Some(current_path) = namespace.exact_native_path() else {
            let SourceNamespaceKey::Authored(namespace) = namespace else {
                return place::unknown_top();
            };
            return authored_namespace_place(selected_name, namespace, observed);
        };
        let (path, simple) = if protocol.is_jim084() {
            let Some(object) = self.namespace_objects.get(namespace) else {
                return place::unknown_top();
            };
            let mut rooted = b"::".to_vec();
            rooted.extend_from_slice(object.as_bytes());
            let key = tcl_syntax::naming::jim_global_variable_key_bytes(
                &rooted,
                selected_name.as_bytes(),
            );
            let Ok(simple) = String::from_utf8(key) else {
                return place::unknown_top();
            };
            (ByteNamespacePath::root(), simple)
        } else {
            let Some(parts) = c_variable_parts(protocol, current_path, selected_name) else {
                return place::unknown_top();
            };
            parts
        };
        let unqualified_current = !protocol.is_jim084()
            && selected.qualification() != NativeNameQualification::Absolute
            && path == *current_path;
        let identity = if unqualified_current {
            Some(namespace.clone())
        } else {
            self.identity_for_path(&path)
        };
        let Some(identity) = identity else {
            return place::unknown_top();
        };
        let Some(display) = identity.display() else {
            return place::unknown_top();
        };
        let mut bound = place::scalar(&simple, display, observed);
        bound.cell = Some(CellIdentity {
            owner: CellOwner::NamespaceIdentity(Box::new(identity)),
            name: simple.clone(),
            generation: CellGeneration::Incoming,
            interpreter: self.interpreter.clone(),
            storage_domain: None,
            execution: self.execution,
        });
        bound
    }
}

fn c_variable_parts(
    protocol: NativeNameProtocol,
    current: &ByteNamespacePath,
    name: &str,
) -> Option<(ByteNamespacePath, String)> {
    let selected = protocol.variable_root_input(name.as_bytes());
    let mut path = if selected.qualification() == NativeNameQualification::Absolute {
        ByteNamespacePath::root()
    } else {
        current.clone()
    };
    let parts = tcl_syntax::naming::qualifier_segments(selected.selected());
    let simple = std::str::from_utf8(tcl_syntax::naming::written_command_tail(
        selected.selected(),
    ))
    .ok()?;
    let count = parts.len().saturating_sub(usize::from(!simple.is_empty()));
    for part in &parts[..count] {
        path.push(*part);
    }
    Some((path, simple.to_owned()))
}

fn authored_static_cell_owner(cell: &CellIdentity) -> Option<&SourceNamespaceKey> {
    match &cell.owner {
        CellOwner::NamespaceIdentity(key) => Some(key),
        CellOwner::RetainedSlot(slot) => match slot.as_ref() {
            crate::raw_binding::RawBindingSlotId::Variable(cell) => {
                authored_static_cell_owner(cell)
            }
            crate::raw_binding::RawBindingSlotId::Callable { .. } => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_resolve::{
        CommandRegistry, ContentsPresence, FrameLevel, PlaceKind, VariableCellKey,
        VariableExecutionFrame, VariableFrameKind, cell_key, project_access, resolve_literal_place,
        restore_execution_frame, trace_key,
    };
    use tcl_dialect::TclVersion;
    use tcl_runtime_api::native_compilation::{
        NativeInterpreterIdentity, NativeNamespaceContext, NativeVariableObserverPresence,
    };

    fn key(token: u64, segments: &[&str]) -> SourceNamespaceKey {
        SourceNamespaceKey::Native(NativeNamespaceContext {
            interpreter: NativeInterpreterIdentity {
                owner: 31,
                interpreter: 7,
            },
            token,
            path: ByteNamespacePath::from_segments(segments.iter().copied()),
        })
    }

    fn world(version: TclVersion, current: SourceNamespaceKey) -> ResolveContext {
        let mut state = ResolveContext::for_namespace(current.display().unwrap());
        state.namespace_cells.closed = true;
        state.namespace_inventory = crate::var_resolve::NamespaceInventoryClosure::Closed;
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::for_version(version));
        state.retain_namespace_world(
            current,
            [key(1, &[]), key(10, &["a:", "b"]), key(20, &["a", ":b"])],
            Some(NativeNameProtocol::C(version)),
        );
        state
    }

    #[test]
    fn native_namespace_variables_keep_equal_displays_distinct() {
        let registry = CommandRegistry::build_default();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut left = world(version, key(10, &["a:", "b"]));
            let mut right = world(version, key(20, &["a", ":b"]));
            assert_eq!(left.namespace, right.namespace);
            let l = resolve_literal_place("x", &left, false, &registry);
            let r = resolve_literal_place("x", &right, false, &registry);
            assert_ne!(cell_key(&l), cell_key(&r));
            assert!(!place::overlap(&l, &r));
            left.define_literal("x", "LEFT", &registry);
            right.define_literal("x", "RIGHT", &registry);
            assert_eq!(
                left.constant_values.get(&cell_key(&l)).map(String::as_str),
                Some("LEFT")
            );
            assert_eq!(
                right.constant_values.get(&cell_key(&r)).map(String::as_str),
                Some("RIGHT")
            );
            assert!(
                left.constant_values
                    .get(&cell_key(&l).compatibility_name())
                    .is_none()
            );
            assert_eq!(
                cell_key(&resolve_literal_place("::g", &left, false, &registry)),
                cell_key(&resolve_literal_place("::g", &right, false, &registry))
            );
        }
    }

    #[test]
    fn native_qualified_variable_fallback_keeps_release_and_candidate_order() {
        let registry = CommandRegistry::build_default();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let here = key(71, &["here"]);
            let global_n = key(72, &["N"]);
            let mut state = world(version, here);
            state.namespace_identities.insert(global_n.clone());
            state
                .namespace_addressable_identities
                .insert(global_n.clone());
            let expected = VariableCellKey::Namespace {
                identity: global_n,
                simple: "x".into(),
            };
            state.namespace_cells.present.insert(expected.clone());
            let target = resolve_literal_place("N::x", &state, false, &registry);
            if version < TclVersion::V9_0 {
                assert_eq!(cell_key(&target), expected);
            } else {
                assert_eq!(target.kind, PlaceKind::Unknown);
            }
            state.namespace_inventory = crate::var_resolve::NamespaceInventoryClosure::Open;
            if version < TclVersion::V9_0 {
                assert_eq!(
                    resolve_literal_place("N::x", &state, false, &registry).kind,
                    PlaceKind::Unknown
                );
            }
        }
    }

    #[test]
    fn native_namespace_links_selected_frames_and_traces_use_original_cells() {
        let registry = CommandRegistry::build_default();
        let mut left = world(TclVersion::V8_6, key(10, &["a:", "b"]));
        let right = world(TclVersion::V8_6, key(20, &["a", ":b"]));
        let l = resolve_literal_place("a(k)", &left, false, &registry);
        let r = resolve_literal_place("a(k)", &right, false, &registry);
        assert!(!place::overlap(&l, &r));
        left.trace_registrations.insert(
            trace_key(&l),
            vec![(
                vec![tcl_registry::TraceOperation::Write],
                "WATCH_LEFT".into(),
            )],
        );
        assert!(project_access(l.clone(), &left, tcl_registry::TraceOperation::Write).observed);
        assert!(!project_access(r, &left, tcl_registry::TraceOperation::Write).observed);
        let mut child = left.in_frame(
            &VariableExecutionFrame::Procedure {
                namespace: right.namespace.clone(),
                identity: "call:1".into(),
            }
            .with_namespace_identity(right.namespace_identity.clone().unwrap()),
        );
        child.caller = Some(std::sync::Arc::new(left.clone()));
        child.alias_bindings.insert("alias", l.clone());
        assert_eq!(
            cell_key(&resolve_literal_place("alias", &child, false, &registry)),
            cell_key(&l)
        );
        let caller = child
            .selected_frame_context(FrameLevel::Relative(1))
            .unwrap();
        assert_eq!(caller.namespace_identity, left.namespace_identity);
        assert_eq!(
            cell_key(&resolve_literal_place("a(k)", &caller, false, &registry)),
            cell_key(&l)
        );
    }

    #[test]
    fn native_namespace_retirement_keeps_equal_display_and_replacement_cells() {
        let selected = key(10, &["a:", "b"]);
        let replacement = key(11, &["a:", "b"]);
        let peer = key(20, &["a", ":b"]);
        let mut state = world(TclVersion::V8_6, selected.clone());
        for namespace in [&selected, &replacement, &peer] {
            let cell = VariableCellKey::Namespace {
                identity: namespace.clone(),
                simple: "x".into(),
            };
            state.constant_values.insert(cell, "KEPT".into());
            state.namespace_identities.insert(namespace.clone());
        }
        state = state.with_namespace_identity(key(1, &[]));
        state.frame_kind = VariableFrameKind::Global;
        crate::variable_bindings::delete_namespace_cells_in_identity(&mut state, &selected, 19);
        assert!(
            !state
                .constant_values
                .contains_key(&VariableCellKey::Namespace {
                    identity: selected,
                    simple: "x".into()
                })
        );
        for namespace in [replacement, peer] {
            assert!(
                state
                    .constant_values
                    .contains_key(&VariableCellKey::Namespace {
                        identity: namespace,
                        simple: "x".into()
                    })
            );
        }
    }

    #[test]
    fn native_namespace_teardown_waits_for_last_actual_frame_and_preserves_replacement() {
        let registry = CommandRegistry::build_default();
        let old = key(80, &["N"]);
        let new = key(81, &["N"]);
        let mut parent = world(TclVersion::V8_6, key(1, &[]));
        parent.frame_kind = VariableFrameKind::Global;
        let mut child = parent.in_frame(
            &VariableExecutionFrame::NamespaceActivation {
                namespace: "::N".into(),
                identity: "entered:1".into(),
            }
            .with_namespace_identity(old.clone()),
        );
        child.namespace_addressable_identities.insert(old.clone());
        child.define_literal("x", "OLD", &registry);
        let old_cell = VariableCellKey::Namespace {
            identity: old.clone(),
            simple: "x".into(),
        };
        crate::variable_bindings::delete_namespace_cells_in_identity(&mut child, &old, 30);
        assert_eq!(
            child.constant_values.get(&old_cell).map(String::as_str),
            Some("OLD")
        );
        assert!(child.pending_namespace_retirements.contains_key(&old));
        child.namespace_identities.insert(new.clone());
        child.namespace_addressable_identities.insert(new.clone());
        let new_cell = VariableCellKey::Namespace {
            identity: new,
            simple: "x".into(),
        };
        child.constant_values.insert(new_cell.clone(), "NEW".into());
        child.namespace_cells.present.insert(new_cell.clone());
        assert_eq!(
            cell_key(&resolve_literal_place("x", &child, false, &registry)),
            old_cell
        );
        assert_eq!(
            cell_key(&resolve_literal_place("::N::x", &child, false, &registry)),
            new_cell
        );
        let restored = restore_execution_frame(&parent, &child);
        assert!(!restored.constant_values.contains_key(&old_cell));
        assert_eq!(
            restored.constant_values.get(&new_cell).map(String::as_str),
            Some("NEW")
        );
        assert!(restored.pending_namespace_retirements.is_empty());
    }

    #[test]
    fn retained_deleted_current_does_not_replace_written_namespace_lookup() {
        let registry = CommandRegistry::build_default();
        let old = key(80, &["N"]);
        let new = key(81, &["N"]);
        let mut state = world(TclVersion::V8_6, old.clone());
        state.namespace_addressable_identities.remove(&old);
        state.namespace_addressable_identities.insert(new.clone());
        state.namespace_identities.insert(new.clone());
        let bare = resolve_literal_place("x", &state, false, &registry);
        let written = resolve_literal_place("::N::x", &state, false, &registry);
        assert_eq!(
            cell_key(&bare),
            VariableCellKey::Namespace {
                identity: old,
                simple: "x".into()
            }
        );
        assert_eq!(
            cell_key(&written),
            VariableCellKey::Namespace {
                identity: new,
                simple: "x".into()
            }
        );
    }

    #[test]
    fn native_unknown_namespace_write_cannot_clobber_equal_display_peer() {
        let registry = CommandRegistry::build_default();
        let mut state = world(TclVersion::V8_6, key(10, &["a:", "b"]));
        let l = resolve_literal_place("x", &state, false, &registry);
        let r = world(TclVersion::V8_6, key(20, &["a", ":b"])).namespace_place("x", false, false);
        state
            .contents_presence
            .insert(cell_key(&l), ContentsPresence::Defined);
        state
            .contents_presence
            .insert(cell_key(&r), ContentsPresence::Defined);
        let mut footprint = l.clone();
        footprint.kind = PlaceKind::Unknown;
        footprint.name.clear();
        state.record_contents_write(&footprint, 25, false);
        assert_eq!(state.contents_presence(&l), ContentsPresence::Unknown);
        assert_eq!(state.contents_presence(&r), ContentsPresence::Defined);
        assert!(place::overlap(&footprint, &l));
        assert!(!place::overlap(&footprint, &r));
    }

    #[test]
    fn authored_static_publication_observes_only_actual_link_target_namespace() {
        use tcl_runtime_api::authored_tmm::{
            AuthoredTmmStaticCompilationContext, AuthoredTmmStaticPolicy, TmmStaticExecutionContext,
        };
        let registry = CommandRegistry::build_default();
        let selected = key(30, &["static"]);
        let mut state = world(TclVersion::V8_6, selected.clone());
        state.authored_tmm_static = Some(AuthoredTmmStaticCompilationContext {
            policy: AuthoredTmmStaticPolicy::RuleInitPublication,
            context: Some(TmmStaticExecutionContext::InitialisationBroadcast),
            namespace: selected.native_context().unwrap().clone(),
            recipients: vec![],
            outward_observers: NativeVariableObserverPresence::Present,
        });
        let target = resolve_literal_place("a(k)", &state, false, &registry);
        let global = resolve_literal_place("::global", &state, false, &registry);
        state.alias_bindings.insert("linked", target.clone());
        let linked = resolve_literal_place("linked", &state, false, &registry);
        assert!(project_access(linked, &state, tcl_registry::TraceOperation::Write).observed);
        assert!(
            project_access(target.clone(), &state, tcl_registry::TraceOperation::Unset).observed
        );
        assert!(
            !project_access(target.clone(), &state, tcl_registry::TraceOperation::Read).observed
        );
        assert!(!project_access(global, &state, tcl_registry::TraceOperation::Write).observed);
        state.authored_tmm_static.as_mut().unwrap().context =
            Some(TmmStaticExecutionContext::ExecutingWorker);
        assert!(!project_access(target, &state, tcl_registry::TraceOperation::Write).observed);
    }

    #[test]
    fn jim_namespace_variable_uses_retained_flat_object_bytes() {
        let registry = CommandRegistry::build_default();
        let current = key(41, &["a:", "b"]);
        let root = key(1, &[]);
        let mut state = world(TclVersion::V8_6, current.clone());
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            tcl_registry::model::ingress::static_context_for("jim")
                .commands()
                .profile()
                .unwrap(),
        ));
        state.namespace_name_protocol = Some(NativeNameProtocol::Jim084);
        state.namespace_objects.insert(root.clone(), "".into());
        state.namespace_objects.insert(current, "a:::b".into());
        state.ns_vars.insert("x".into());
        let target = resolve_literal_place("x", &state, false, &registry);
        assert_eq!(
            cell_key(&target),
            VariableCellKey::Namespace {
                identity: root,
                simple: "a:::b::x".into()
            }
        );
    }
}
