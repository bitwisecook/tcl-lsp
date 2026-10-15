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

//! Original Jim result replacement and resident observation in the actual finalizer phase.

use super::{native_operation_currency::SavedNativeOperationCurrency, Interp, WeakInterp};
use crate::obj::{self, TclObj};
use std::{cell::RefCell, rc::Rc};
use tcl_syntax::value::ValueError;

struct RetiringJimResult {
    interpreter: WeakInterp,
    original: *mut TclObj,
    header: obj::RetiringResultHeader,
    primary_admission: Result<(), ValueError>,
    currency: SavedNativeOperationCurrency,
}
thread_local! {
    static RETIRING_RESULTS: RefCell<Vec<Rc<RetiringJimResult>>> = const { RefCell::new(Vec::new()) };
}

pub(super) struct JimResultRetirement(Option<Rc<RetiringJimResult>>);
impl JimResultRetirement {
    pub(super) fn enter(interpreter: &Interp, original: *mut TclObj) -> Self {
        if interpreter
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
            || original != interpreter.get_obj_result()
        {
            interpreter.clone().refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "actual Jim result retirement issuer and slot",
                ),
            );
            return Self(None);
        }
        let header: obj::RetiringResultHeader = match obj::RetiringResultHeader::capture(original) {
            Ok(header) => header,
            Err(error) => {
                if let Some(first) = error.native_access_refusal() {
                    interpreter.clone().refuse_native_access(first);
                }
                return Self(None);
            }
        };
        let observation: Rc<RetiringJimResult> = Rc::new(RetiringJimResult {
            interpreter: interpreter.downgrade(),
            original,
            header,
            primary_admission: Interp::native_string_primary_admission(interpreter, original),
            currency: SavedNativeOperationCurrency::capture(interpreter),
        });
        RETIRING_RESULTS.with(|active| active.borrow_mut().push(observation.clone()));
        Self(Some(observation))
    }
}
impl Drop for JimResultRetirement {
    fn drop(&mut self) {
        let Some(observation) = &self.0 else { return };
        let retired = RETIRING_RESULTS.with(|active| {
            let mut active = active.borrow_mut();
            let position = active
                .iter()
                .rposition(|candidate| Rc::ptr_eq(candidate, observation))
                .expect("actual Jim result replacement phase");
            active.remove(position)
        });
        drop(retired);
    }
}

impl Interp {
    pub(crate) fn retiring_jim_result_bytes(
        &self,
        original: *mut TclObj,
    ) -> Result<Option<Rc<[u8]>>, ValueError> {
        if original.is_null()
            || original != self.get_obj_result()
            || obj::allocation_is_live(original)
        {
            return Ok(None);
        }
        let observation = RETIRING_RESULTS.with(|active| {
            active
                .borrow()
                .iter()
                .rev()
                .find(|candidate| {
                    candidate.original == original
                        && candidate
                            .interpreter
                            .upgrade()
                            .is_some_and(|actual| Rc::ptr_eq(&actual.0, &self.0))
                })
                .cloned()
        });
        let Some(observation) = observation else {
            return Ok(None);
        };
        observation.currency.check(self).map_err(|_| {
            ValueError::CommandProtocolUnavailable("entered original retiring result String getter")
        })?;
        observation.primary_admission.clone()?;
        obj::RetiringResultHeader::resident_bytes(&observation.header).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::Cell;

    thread_local! {
        static ACTUAL: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
        static WITHDRAW: Cell<bool> = const { Cell::new(false) };
        static OBSERVED: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
        static FOREIGN_PRIMARY_REFUSED: Cell<bool> = const { Cell::new(false) };
    }
    extern "C" fn finalizer(original: *mut TclObj) {
        let mut actual = ACTUAL
            .with(|slot| slot.borrow().as_ref().and_then(WeakInterp::upgrade))
            .unwrap();
        assert!(obj::check_native_liveness(original).is_err());
        // SAFETY: the actual finalizer owns this still-allocated native header.
        assert_eq!(unsafe { (*original).ref_count }, 0);
        if WITHDRAW.with(Cell::get) {
            actual.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert!(actual.native_object_string_bytes(original).is_err());
            let first = actual
                .native_execution_refusal()
                .expect("saved original issuer refused");
            assert!(actual.native_object_string_bytes(original).is_err());
            assert_eq!(actual.native_execution_refusal(), Some(first));
            return;
        }
        let observed = actual
            .native_object_string_bytes(actual.get_obj_result())
            .unwrap();
        OBSERVED.with(|slot| *slot.borrow_mut() = Some(observed.to_vec()));
        let foreign = Interp::new();
        assert!(foreign.native_object_string_bytes(original).is_err());
        if actual
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            assert_eq!(actual.get_obj_result(), original);
            assert!(actual
                .retiring_jim_result_bytes(original)
                .unwrap()
                .is_some());
        } else {
            assert_ne!(actual.get_obj_result(), original);
            assert!(actual
                .retiring_jim_result_bytes(original)
                .unwrap()
                .is_none());
            assert!(actual.native_object_string_bytes(original).is_err());
        }
    }
    extern "C" fn foreign_primary_child_free(_: *mut TclObj) {
        let actual = ACTUAL
            .with(|slot| slot.borrow().as_ref().and_then(WeakInterp::upgrade))
            .unwrap();
        let parent = actual.get_obj_result();
        assert!(obj::check_native_liveness(parent).is_err());
        assert!(obj::has_string_rep(parent));
        FOREIGN_PRIMARY_REFUSED.with(|observed| {
            observed.set(matches!(
                actual.native_object_string_bytes(parent),
                Err(ValueError::CommandProtocolUnavailable(
                    "foreign property-name primary"
                ))
            ))
        });
    }
    static FOREIGN_PRIMARY_CHILD: obj::TclObjType = obj::TclObjType {
        name: c"foreign-primary-child-observer".as_ptr(),
        free_int_rep_proc: Some(foreign_primary_child_free),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    static RESULT: obj::TclObjType = obj::TclObjType {
        name: c"retiring-result-observer".as_ptr(),
        free_int_rep_proc: Some(finalizer),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    fn previous(actual: &mut Interp) -> obj::NativeObjectLifetime {
        let value = Owned::fresh(obj::alloc_typed(&RESULT, 0));
        // SAFETY: the fresh original owns its resident String.
        unsafe { obj::set_string_rep(value.as_ptr(), b"PRIOR") };
        let allocation = obj::NativeObjectLifetime::retain(value.as_ptr());
        actual.set_result(value.as_ptr());
        drop(value);
        allocation
    }
    fn native_jim() -> Interp {
        Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    #[test]
    fn retiring_jim_result_getter_is_scoped_to_original_slot_and_actual_free_phase() {
        for jim in [false, true] {
            let mut actual = if jim { native_jim() } else { Interp::new() };
            WITHDRAW.with(|slot| slot.set(false));
            OBSERVED.with(|slot| *slot.borrow_mut() = None);
            ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.downgrade()));
            let allocation = previous(&mut actual);
            actual.set_result_bytes(b"AFTER");
            assert_eq!(
                OBSERVED.with(|slot| slot.borrow().clone()),
                Some(if jim {
                    b"PRIOR".to_vec()
                } else {
                    b"AFTER".to_vec()
                })
            );
            assert!(actual
                .retiring_jim_result_bytes(allocation.as_ptr())
                .unwrap()
                .is_none());
            assert!(allocation.checked_ptr().is_err());
            assert!(actual
                .native_object_string_bytes(allocation.as_ptr())
                .is_err());
            assert_eq!(actual.result_bytes(), b"AFTER");
            assert!(actual.native_execution_refusal().is_none());
            ACTUAL.with(|slot| *slot.borrow_mut() = None);
        }
        let dialect = tcl_registry::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("tcl9.1"),
        );
        let property = Owned::fresh(obj::new_string_bytes(b"-original"));
        let reader = obj::native_property_name::accessor(
            property.as_ptr(),
            dialect.native_property_lookup_protocol().unwrap(),
            false,
        )
        .unwrap();
        obj::change_type(reader, &FOREIGN_PRIMARY_CHILD, 0);
        let mut actual = native_jim();
        ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.downgrade()));
        FOREIGN_PRIMARY_REFUSED.with(|observed| observed.set(false));
        actual.set_result(property.as_ptr());
        drop(property);
        actual.set_result_bytes(b"AFTER");
        assert!(FOREIGN_PRIMARY_REFUSED.with(Cell::get));
        assert!(actual.native_execution_refusal().is_none());
        ACTUAL.with(|slot| *slot.borrow_mut() = None);
        let foreign = Interp::new();
        let current = foreign.get_obj_result();
        let scope = JimResultRetirement::enter(&foreign, current);
        assert!(scope.0.is_none());
        assert_eq!(foreign.get_obj_result(), current);
        assert!(foreign.native_execution_refusal().is_some());
    }
    #[test]
    fn withdrawn_jim_result_observation_keeps_first_refusal_and_cannot_resurrect_header() {
        let mut actual = native_jim();
        ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.downgrade()));
        WITHDRAW.with(|slot| slot.set(true));
        let allocation = previous(&mut actual);
        actual.set_result_bytes(b"AFTER");
        let first = actual
            .native_execution_refusal()
            .expect("actual issuing currency withdrew");
        assert!(allocation.checked_ptr().is_err());
        assert!(actual
            .native_object_string_bytes(allocation.as_ptr())
            .is_err());
        assert_eq!(actual.native_execution_refusal(), Some(first));
        WITHDRAW.with(|slot| slot.set(false));
        ACTUAL.with(|slot| *slot.borrow_mut() = None);
    }
}
