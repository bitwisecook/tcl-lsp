// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Receipts for a native operation protecting its selected variable cell.
//!
//! A receipt identifies one reached capture, not a source allocation family.
//! Resetting protected contents preserves that receipt. Retiring its array or
//! namespace makes retirement permanent, even if identical names are recreated.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::place::{CellGeneration, CellOwner, Place, PlaceKind};
use crate::var_resolve::{RootContentsKind, VariableProofRelocation};

static NEXT_RECEIPT: AtomicU64 = AtomicU64::new(1);

/// Identity of one reached, currently active capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapturedCellLeaseId(u64);

/// Why the original protected cell can no longer receive a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapturedCellRetirement {
    /// Its containing array was destroyed.
    Array,
    /// Its namespace variable table was destroyed.
    Namespace,
}

/// Lifetime of the original receiver, independently of current name lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapturedCellState {
    /// The original cell remains live.
    Live,
    /// Unset cleared protected contents without retiring the original shell.
    ContentsReset,
    /// The original owner was destroyed; later recreation cannot revive it.
    Retired(CapturedCellRetirement),
    /// An unresolved operation or divergent predecessor lost lifetime evidence.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CapturedCellLease {
    receiver: Place,
    state: CapturedCellState,
}

/// Active operation receipts shared by every selected-frame view.
/// Finished receipts are removed; external identities are never compared by name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CapturedCellArena {
    leases: BTreeMap<CapturedCellLeaseId, CapturedCellLease>,
}

impl CapturedCellArena {
    /// Capture a proved receiver for one reached operation.
    /// The unique receipt does not claim uniqueness of its allocation family.
    pub fn capture(&mut self, receiver: &Place) -> Option<CapturedCellLeaseId> {
        let cell = receiver.cell.as_ref()?;
        if receiver.dynamic
            || receiver.kind == PlaceKind::Unknown
            || cell.generation == CellGeneration::Unknown
            || receiver
                .index
                .as_ref()
                .is_some_and(|index| index.kind != crate::place::IndexKind::Literal)
        {
            return None;
        }
        let id = CapturedCellLeaseId(NEXT_RECEIPT.fetch_add(1, Ordering::Relaxed));
        self.leases.insert(
            id,
            CapturedCellLease {
                receiver: receiver.clone(),
                state: CapturedCellState::Live,
            },
        );
        Some(id)
    }

    /// Query the original cell's lifetime, declining a missing receipt.
    #[must_use]
    pub fn state(&self, id: CapturedCellLeaseId) -> CapturedCellState {
        self.leases
            .get(&id)
            .map_or(CapturedCellState::Unknown, |lease| lease.state)
    }

    /// Original physical receiver; no spelling or alias is resolved again.
    #[must_use]
    pub fn receiver(&self, id: CapturedCellLeaseId) -> Option<&Place> {
        self.leases.get(&id).map(|lease| &lease.receiver)
    }

    /// Release on every normal and abrupt completion of the native operation.
    pub fn release(&mut self, id: CapturedCellLeaseId) {
        self.leases.remove(&id);
    }

    /// Preserve member guards after the proved original array lookup is detached.
    /// A recreated root cannot acquire these original operation receipts.
    pub(crate) fn retain_array_members(&mut self, original: &Place, retained: &Place) {
        for lease in self.leases.values_mut() {
            if lease.receiver.cell == original.cell
                && lease.receiver.index.is_some()
                && matches!(
                    lease.state,
                    CapturedCellState::Live | CapturedCellState::ContentsReset
                )
            {
                lease.receiver.cell.clone_from(&retained.cell);
            }
        }
    }

    /// Retiring an array member cannot be undone by a store in its unset callback.
    pub(crate) fn retire_array_member(&mut self, member: &Place) {
        for lease in self.leases.values_mut() {
            if lease.receiver.cell == member.cell && lease.receiver.index == member.index {
                lease.state = CapturedCellState::Retired(CapturedCellRetirement::Array);
            }
        }
    }

    /// Apply a reached destruction before any of its callbacks run.
    pub fn destroy(&mut self, target: &Place, root_kind: Option<RootContentsKind>) {
        for lease in self.leases.values_mut() {
            if matches!(
                lease.state,
                CapturedCellState::Retired(_) | CapturedCellState::Unknown
            ) {
                continue;
            }
            if target.kind == PlaceKind::Unknown {
                if crate::place::overlap(target, &lease.receiver) {
                    lease.state = CapturedCellState::Unknown;
                }
                continue;
            }
            let same_root = target.cell == lease.receiver.cell;
            if !same_root {
                if crate::place::overlap(target, &lease.receiver) {
                    lease.state = CapturedCellState::Unknown;
                }
                continue;
            }
            if let Some(index) = &target.index {
                if index.kind != crate::place::IndexKind::Literal {
                    if lease.receiver.index.is_some() {
                        lease.state = CapturedCellState::Unknown;
                    }
                } else if target.index == lease.receiver.index {
                    lease.state = CapturedCellState::ContentsReset;
                }
            } else if lease.receiver.index.is_some() {
                lease.state = match root_kind {
                    Some(RootContentsKind::Array) => {
                        CapturedCellState::Retired(CapturedCellRetirement::Array)
                    }
                    Some(RootContentsKind::Scalar) => CapturedCellState::ContentsReset,
                    None => CapturedCellState::Unknown,
                };
            } else {
                lease.state = CapturedCellState::ContentsReset;
            }
        }
    }

    /// Retire original namespace cells before the namespace's unset callbacks.
    pub fn delete_namespace(&mut self, namespace: &str) {
        for lease in self.leases.values_mut() {
            if lease.state != CapturedCellState::Unknown
                && let Some(CellOwner::Namespace(owner)) =
                    lease.receiver.cell.as_ref().map(|cell| &cell.owner)
                && (owner == namespace || owner.starts_with(&format!("{namespace}::")))
            {
                lease.state = CapturedCellState::Retired(CapturedCellRetirement::Namespace);
            }
        }
    }

    /// Retire cells belonging to the selected original namespace identity.
    /// Membership uses retained component boundaries and interpreter identity;
    /// presentation text cannot retire a different native namespace incarnation.
    pub fn delete_namespace_identity(
        &mut self,
        namespace: &crate::command_binding::SourceNamespaceKey,
    ) {
        for lease in self.leases.values_mut() {
            if lease.state != CapturedCellState::Unknown
                && crate::var_resolve::cell_key(&lease.receiver).is_in_namespace(namespace)
            {
                lease.state = CapturedCellState::Retired(CapturedCellRetirement::Namespace);
            }
        }
    }

    /// Retire cells when their retained original namespace tables are released.
    /// The caller supplies the exact incarnations selected at deletion, after
    /// any active frames have released them. Same-path replacements and later
    /// descendants are not members of this retirement receipt.
    pub fn delete_namespace_incarnations(
        &mut self,
        namespaces: &crate::var_resolve::VariableNamespaceSet,
    ) {
        for lease in self.leases.values_mut() {
            if matches!(
                lease.state,
                CapturedCellState::Live | CapturedCellState::ContentsReset
            ) && let Some(CellOwner::NamespaceIdentity(owner)) =
                lease.receiver.cell.as_ref().map(|cell| &cell.owner)
                && namespaces.contains(owner.as_ref())
            {
                lease.state = CapturedCellState::Retired(CapturedCellRetirement::Namespace);
            }
        }
    }

    /// Unknown code may destroy an owner; a later same-name store cannot repair it.
    pub fn widen(&mut self) {
        for lease in self.leases.values_mut() {
            if !matches!(lease.state, CapturedCellState::Retired(_)) {
                lease.state = CapturedCellState::Unknown;
            }
        }
    }

    /// Join receipts without identifying independent captures of the same family.
    pub fn join(&mut self, other: &Self) {
        for (id, lease) in &mut self.leases {
            if other.leases.get(id) != Some(lease) {
                lease.state = CapturedCellState::Unknown;
            }
        }
        for (id, lease) in &other.leases {
            if !self.leases.contains_key(id) {
                let mut absent = lease.clone();
                absent.state = CapturedCellState::Unknown;
                self.leases.insert(*id, absent);
            }
        }
    }

    /// Relocate physical dependencies while preserving reached capture identities.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            leases: self
                .leases
                .iter()
                .map(|(id, lease)| {
                    (
                        *id,
                        CapturedCellLease {
                            receiver: relocation.place(&lease.receiver),
                            state: lease.state,
                        },
                    )
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_resolve::{ResolveContext, VariableFrameKind, resolve_literal_access};
    use tcl_registry::TraceOperation;

    fn context() -> (
        ResolveContext,
        std::sync::Arc<tcl_registry::CommandRegistry>,
    ) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .clone();
        let mut context = ResolveContext {
            frame_kind: VariableFrameKind::Global,
            binding_identity: crate::var_resolve::BindingIdentity::Bound,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            )),
            ..Default::default()
        };
        context.define_literal("a(k)", "10", &registry);
        (context, registry)
    }

    #[test]
    fn element_contents_reset_keeps_the_captured_cell_but_root_retirement_is_permanent() {
        let (mut context, registry) = context();
        let element =
            resolve_literal_access("a(k)", &context, false, &registry, TraceOperation::Read);
        let id = context.captured_cells.capture(&element).unwrap();
        crate::variable_bindings::destroy_literal_binding(&mut context, "a(k)", 1, &registry);
        assert_eq!(
            context.captured_cells.state(id),
            CapturedCellState::ContentsReset
        );
        context.define_literal("a(k)", "100", &registry);
        assert!(context.captured_cell_receiver(id).is_some());
        crate::variable_bindings::destroy_literal_binding(&mut context, "a", 2, &registry);
        assert_eq!(
            context.captured_cells.state(id),
            CapturedCellState::Retired(CapturedCellRetirement::Array)
        );
        context.define_literal("a(k)", "200", &registry);
        assert!(context.captured_cell_receiver(id).is_none());
    }

    #[test]
    fn distinct_captures_of_one_allocation_family_never_join_as_one_receipt() {
        let (context, registry) = context();
        let element =
            resolve_literal_access("a(k)", &context, false, &registry, TraceOperation::Read);
        let mut first = context.captured_cells.clone();
        let mut second = first.clone();
        let first_id = first.capture(&element).unwrap();
        let second_id = second.capture(&element).unwrap();
        assert_ne!(first_id, second_id);
        first.join(&second);
        assert_eq!(first.state(first_id), CapturedCellState::Unknown);
        assert_eq!(first.state(second_id), CapturedCellState::Unknown);
        first.destroy(&element, Some(RootContentsKind::Array));
        assert_eq!(first.state(first_id), CapturedCellState::Unknown);
    }

    #[test]
    fn selected_frame_restoration_keeps_child_lifetime_receipts() {
        let (mut parent, registry) = context();
        let element =
            resolve_literal_access("a(k)", &parent, false, &registry, TraceOperation::Read);
        let id = parent.captured_cells.capture(&element).unwrap();
        let mut child =
            parent.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "callback".to_owned(),
            });
        crate::variable_bindings::destroy_literal_binding(&mut child, "::a", 9, &registry);
        let restored = crate::var_resolve::restore_execution_frame(&parent, &child);
        assert_eq!(
            restored.captured_cells.state(id),
            CapturedCellState::Retired(CapturedCellRetirement::Array)
        );
    }

    #[test]
    fn namespace_receipts_and_unknown_lifetimes_are_not_repaired_by_names() {
        let (mut context, registry) = context();
        context.define_literal("::N::x", "10", &registry);
        let scalar =
            resolve_literal_access("::N::x", &context, false, &registry, TraceOperation::Read);
        let id = context.captured_cells.capture(&scalar).unwrap();
        context.captured_cells.delete_namespace("::N");
        context.define_literal("::N::x", "100", &registry);
        assert_eq!(
            context.captured_cells.state(id),
            CapturedCellState::Retired(CapturedCellRetirement::Namespace)
        );
        context.captured_cells.release(id);
        let current =
            resolve_literal_access("a(k)", &context, false, &registry, TraceOperation::Read);
        let uncertain = context.captured_cells.capture(&current).unwrap();
        context.widen();
        context.captured_cells.delete_namespace("::");
        assert_eq!(
            context.captured_cells.state(uncertain),
            CapturedCellState::Unknown
        );
    }
    #[test]
    fn namespace_retirement_preserves_colliding_paths_and_recreated_owners() {
        use crate::command_binding::SourceNamespaceKey;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };
        let (context, registry) = context();
        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let namespace = |token, path| {
            SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter,
                token,
                path,
            })
        };
        let original = namespace(
            1,
            tcl_core_types::ByteNamespacePath::from_segments(["a:", "b"]),
        );
        let colliding = namespace(
            2,
            tcl_core_types::ByteNamespacePath::from_segments(["a", ":b"]),
        );
        let recreated = namespace(3, original.exact_native_path().unwrap().clone());
        assert_eq!(original.display(), colliding.display());
        let mut arena = CapturedCellArena::default();
        let mut receipts = Vec::new();
        for owner in [&original, &colliding, &recreated] {
            let mut receiver =
                resolve_literal_access("a(k)", &context, false, &registry, TraceOperation::Read);
            receiver.cell.as_mut().unwrap().owner =
                CellOwner::NamespaceIdentity(Box::new(owner.clone()));
            receipts.push(arena.capture(&receiver).unwrap());
        }
        arena.delete_namespace(&original.display().unwrap());
        assert!(
            receipts
                .iter()
                .all(|id| arena.state(*id) == CapturedCellState::Live)
        );
        arena.delete_namespace_identity(&original);
        assert_eq!(
            arena.state(receipts[0]),
            CapturedCellState::Retired(CapturedCellRetirement::Namespace)
        );
        assert_eq!(arena.state(receipts[1]), CapturedCellState::Live);
        assert_eq!(arena.state(receipts[2]), CapturedCellState::Live);
    }

    #[test]
    fn deferred_namespace_retirement_uses_only_original_incarnations() {
        use crate::command_binding::SourceNamespaceKey;
        use crate::var_resolve::VariableNamespaceSet;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let (context, registry) = context();
        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let namespace = |token, segments: &[&str]| {
            SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter,
                token,
                path: tcl_core_types::ByteNamespacePath::from_segments(segments.iter().copied()),
            })
        };
        let original = namespace(1, &["N"]);
        let old_child = namespace(2, &["N", "child"]);
        let replacement = namespace(3, &["N"]);
        let replacement_child = namespace(4, &["N", "child"]);
        let later_child = namespace(5, &["N", "later"]);
        let mut arena = CapturedCellArena::default();
        let receipts: Vec<_> = [
            &original,
            &old_child,
            &replacement,
            &replacement_child,
            &later_child,
        ]
        .into_iter()
        .map(|owner| {
            let mut receiver =
                resolve_literal_access("a(k)", &context, false, &registry, TraceOperation::Read);
            receiver.cell.as_mut().unwrap().owner =
                CellOwner::NamespaceIdentity(Box::new(owner.clone()));
            arena.capture(&receiver).unwrap()
        })
        .collect();
        let retiring = VariableNamespaceSet::from([original, old_child]);
        assert!(
            receipts
                .iter()
                .all(|id| arena.state(*id) == CapturedCellState::Live)
        );
        arena.delete_namespace_incarnations(&retiring);
        for id in &receipts[..2] {
            assert_eq!(
                arena.state(*id),
                CapturedCellState::Retired(CapturedCellRetirement::Namespace)
            );
        }
        for id in &receipts[2..] {
            assert_eq!(arena.state(*id), CapturedCellState::Live);
        }
    }

    #[test]
    fn repeated_active_frame_families_do_not_attest_one_physical_receiver() {
        let (_, registry) = context();
        let mut first = ResolveContext::for_function("::recursive");
        first.binding_identity = crate::var_resolve::BindingIdentity::Bound;
        first.define_literal("x", "1", &registry);
        let receiver = resolve_literal_access("x", &first, false, &registry, TraceOperation::Read);
        assert!(first.capture_protected_cell(&receiver).is_some());
        let mut repeated =
            first.enter_called_frame(&crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "::recursive".to_owned(),
            });
        repeated.define_literal("x", "2", &registry);
        let receiver =
            resolve_literal_access("x", &repeated, false, &registry, TraceOperation::Read);
        assert!(repeated.capture_protected_cell(&receiver).is_none());
    }
}
