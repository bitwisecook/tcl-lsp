// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual software role destruction and first-cause mapping controls.
use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

struct State {
    live: Cell<usize>,
    acquired: Cell<usize>,
    old_visible: Cell<bool>,
    old_drops: Cell<usize>,
    first: RefCell<Option<CmdError>>,
    hash_keys: RefCell<Vec<Vec<u8>>>,
    fail_old: bool,
}
struct Role {
    state: Rc<State>,
    old: bool,
}
impl Drop for Role {
    fn drop(&mut self) {
        self.state.live.set(self.state.live.get() - 1);
        if self.old {
            assert!(self.state.old_visible.get());
            assert_eq!(
                self.state.acquired.get(),
                3,
                "every incoming role precedes old retirement"
            );
            self.state.old_drops.set(self.state.old_drops.get() + 1);
            if self.state.fail_old && self.state.first.borrow().is_none() {
                *self.state.first.borrow_mut() = Some(CmdError::from_execution_refusal(
                    NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "actual declaration role release",
                        ),
                    ),
                ));
            }
        }
    }
}
struct Member {
    key: Vec<u8>,
    role: Option<Role>,
}
struct Ops {
    state: Rc<State>,
    old: Vec<Member>,
    installed: Vec<Member>,
}
impl NativeOoVariableMappingOps for Ops {
    type Member = Member;
    type Retirement = Vec<Member>;
    fn check_mapping(&mut self) -> Result<(), CmdError> {
        self.state.first.borrow().clone().map_or(Ok(()), Err)
    }
    fn retain_mapping_failure(&mut self, error: &CmdError) {
        if self.state.first.borrow().is_none() {
            *self.state.first.borrow_mut() = Some(error.clone());
        }
    }
    fn acquire_member(&mut self, original: &Member) -> Result<Member, CmdError> {
        self.state.acquired.set(self.state.acquired.get() + 1);
        self.state.live.set(self.state.live.get() + 1);
        Ok(Member {
            key: original.key.clone(),
            role: Some(Role {
                state: Rc::clone(&self.state),
                old: false,
            }),
        })
    }
    fn capture_old_mapping(&mut self) -> Result<Vec<Member>, CmdError> {
        self.state.old_visible.set(true);
        Ok(std::mem::take(&mut self.old))
    }
    fn retire_old_mapping(&mut self, retired: &mut Vec<Member>) -> Result<(), CmdError> {
        drop(std::mem::take(retired));
        self.check_mapping()
    }
    fn begin_new_mapping(&mut self, _: &mut Vec<Member>) -> Result<(), CmdError> {
        self.state.old_visible.set(false);
        Ok(())
    }
    fn member_key(&mut self, original: &Member) -> Result<Vec<u8>, CmdError> {
        assert!(!self.state.old_visible.get());
        assert_eq!(self.state.old_drops.get(), 2);
        self.state.hash_keys.borrow_mut().push(original.key.clone());
        Ok(original.key.clone())
    }
    fn prepare_unique_member(&mut self, original: Member, _: &[u8]) -> Result<Member, CmdError> {
        Ok(original)
    }
    fn publish_mapping(
        &mut self,
        members: Vec<Member>,
        _: &mut Vec<Member>,
    ) -> Result<(), CmdError> {
        self.installed = members;
        Ok(())
    }
    fn finish_mapping_retirement(&mut self, retired: Option<Vec<Member>>) -> Result<(), CmdError> {
        drop(retired);
        self.state.old_visible.set(false);
        self.check_mapping()
    }
}
fn setup(fail_old: bool) -> (Ops, Vec<Member>) {
    let state = Rc::new(State {
        live: Cell::new(2),
        acquired: Cell::new(0),
        old_visible: Cell::new(false),
        old_drops: Cell::new(0),
        first: RefCell::new(None),
        hash_keys: RefCell::new(Vec::new()),
        fail_old,
    });
    let old = [b"old1".as_slice(), b"old2"]
        .into_iter()
        .map(|key| Member {
            key: key.to_vec(),
            role: Some(Role {
                state: Rc::clone(&state),
                old: true,
            }),
        })
        .collect();
    let original = [b"a".as_slice(), b"a", b"b"]
        .into_iter()
        .map(|key| Member {
            key: key.to_vec(),
            role: None,
        })
        .collect();
    (
        Ops {
            state,
            old,
            installed: Vec::new(),
        },
        original,
    )
}
#[test]
fn original_mapping_role_drop_observes_all_incoming_and_old_visible_before_hash() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual Rust role Drop observer under the coordinator; no Native callback/count equivalence.
    let (mut ops, original) = setup(false);
    install_original_variable_mapping(&mut ops, &original).unwrap();
    assert_eq!(ops.state.old_drops.get(), 2);
    assert_eq!(
        *ops.state.hash_keys.borrow(),
        [b"a".to_vec(), b"a".to_vec(), b"b".to_vec()]
    );
    assert_eq!(
        ops.installed
            .iter()
            .map(|entry| entry.key.clone())
            .collect::<Vec<_>>(),
        [b"a".to_vec(), b"b".to_vec()]
    );
    assert!(ops.installed.iter().all(|entry| entry.role.is_some()));
    assert_eq!(ops.state.live.get(), 2);
    drop(std::mem::take(&mut ops.installed));
    assert_eq!(ops.state.live.get(), 0);
}
#[test]
fn original_mapping_first_role_refusal_stops_hash_and_preserves_mandatory_release() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut ops, original) = setup(true);
    let reached = install_original_variable_mapping(&mut ops, &original).unwrap_err();
    let first = ops.state.first.borrow().clone().unwrap();
    assert_eq!(
        reached.native_execution_refusal(),
        first.native_execution_refusal()
    );
    assert_eq!(ops.state.old_drops.get(), 2);
    assert!(!ops.state.old_visible.get());
    assert_eq!(ops.state.live.get(), 0);
    assert!(ops.state.hash_keys.borrow().is_empty());
    assert!(ops.installed.is_empty());
    let later = install_original_variable_mapping(&mut ops, &original).unwrap_err();
    assert_eq!(
        later.native_execution_refusal(),
        first.native_execution_refusal()
    );
    assert_eq!(ops.state.acquired.get(), 3);
}
