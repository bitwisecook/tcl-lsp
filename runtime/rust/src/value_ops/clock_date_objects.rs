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

//! The `clock` date-list adapter: checked reads of the date fields and the result list.

use super::ValueCoercion;
use crate::interp::Interp;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_syntax::number::Number;
use tcl_syntax::value::ValueError;

pub(crate) struct RuntimeDateList(Rc<RuntimeDateListState>);

pub(super) struct RuntimeDateListState {
    pub(super) original: obj::NativeObjectLifetime,
    primary: obj::NativePrimaryReceipt,
    pub(super) backing: Option<crate::list::NativeListBacking>,
    pub(super) members: Vec<*mut TclObj>,
}

/// One inherited operation, with allocation-only views of reached originals.
pub(crate) struct RuntimeClockDateOps {
    pub(super) interpreter: Interp,
    pub(super) scope: crate::interp::native_operation_currency::NativeOperationScope,
    literals: tcl_cmd_core::clock::DateFieldLiterals<*mut TclObj>,
    literal_receipts: Vec<obj::NativePrimaryReceipt>,
    windows: Vec<Rc<RuntimeDateListState>>,
    children: Vec<obj::NativeObjectLifetime>,
}

impl RuntimeClockDateOps {
    pub(crate) fn new(
        interpreter: &Interp,
        literals: tcl_cmd_core::clock::DateFieldLiterals<*mut TclObj>,
    ) -> Result<Self, ValueError> {
        use tcl_syntax::{naming::NamePolicyAuthority, native_string::NativeStringProtocol};
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(interpreter)
                .map_err(|first| ValueCoercion::execution_error(interpreter, first))?;
        if interpreter
            .native_invocation_dialect()
            .clock_get_date_fields_registration()
            .is_none()
            || !interpreter.name_policy_protocol().is_some_and(|policy| {
                policy.authority() == NamePolicyAuthority::Native
                    && policy.string_protocol()
                        == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)
            })
        {
            return Err(ValueCoercion::value_error(
                interpreter,
                ValueError::CommandProtocolUnavailable("native C86 GetDateFields worker"),
            ));
        }
        let literal_receipts = literals
            .keys
            .iter()
            .copied()
            .chain([literals.ce, literals.bce])
            .map(obj::NativePrimaryReceipt::capture)
            .collect::<Result<Vec<_>, _>>()?;
        let result = Self {
            interpreter: interpreter.clone(),
            scope,
            literals,
            literal_receipts,
            windows: Vec::new(),
            children: Vec::new(),
        };
        result.check_value()?;
        Ok(result)
    }

    fn receiver_check(&self) -> Result<(), ValueError> {
        for (receipt, value) in self.literal_receipts.iter().zip(
            self.literals
                .keys
                .iter()
                .copied()
                .chain([self.literals.ce, self.literals.bce]),
        ) {
            receipt.check_for(value, "changed C86 clock literal primary")?;
        }
        for window in &self.windows {
            window.primary.check_for(
                window.original.as_ptr(),
                "changed C86 timezone List primary",
            )?;
            if let Some(backing) = &window.backing {
                backing.check_generation()?;
            }
        }
        for child in &self.children {
            obj::check_native_liveness(child.as_ptr())?;
        }
        Ok(())
    }

    fn check_execution(&self) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.scope.currency().ensure_current_or_refuse()?;
        self.receiver_check().map_err(|error| {
            let cause = error
                .native_access_refusal()
                .expect("physical clock receiver guard");
            let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause);
            self.interpreter
                .clone()
                .refuse_native_execution(first.clone());
            self.interpreter.native_execution_refusal().unwrap_or(first)
        })
    }

    pub(crate) fn check_value(&self) -> Result<(), ValueError> {
        self.check_execution()
            .map_err(|first| ValueCoercion::execution_error(&self.interpreter, first))
    }

    pub(super) fn after<T>(&self, result: Result<T, ValueError>) -> Result<T, ValueError> {
        self.check_value()?;
        result.map_err(|error| ValueCoercion::value_error(&self.interpreter, error))
    }

    pub(super) fn hold(&mut self, value: *mut TclObj) -> Result<*mut TclObj, ValueError> {
        self.check_value()?;
        obj::check_native_liveness(value)
            .map_err(|error| ValueCoercion::value_error(&self.interpreter, error))?;
        self.children.push(obj::NativeObjectLifetime::retain(value));
        Ok(value)
    }
    fn prepare_list(
        &mut self,
        value: &*mut TclObj,
        ordinary: bool,
    ) -> Result<RuntimeDateList, ValueError> {
        use tcl_syntax::native_string::NativeStringProtocol;
        self.check_value()?;
        obj::check_native_liveness(*value)?;
        let original = obj::NativeObjectLifetime::retain(*value);
        let check = || {
            self.check_value()?;
            obj::check_native_liveness(*value)
        };
        let retain = |error| ValueCoercion::value_error(&self.interpreter, error);
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let result = if ordinary {
            crate::list::ordinary_list_elements_native_using(*value, protocol, &check, &retain)
        } else {
            crate::list::list_elements_native_with_stage_handlers(
                *value,
                protocol,
                Some(self.scope.currency()),
                Some(&check),
                Some(&retain),
            )
        };
        let members = self.after(result)?;
        // The getter owns its intended coercion; capture the new primary only afterward.
        let primary = obj::NativePrimaryReceipt::capture(*value)?;
        let backing = crate::list::native_list_backing(*value);
        if backing.is_none() && !(members.is_empty() && obj::has_canonical_empty_string(*value)) {
            return Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("original C86 timezone List backing"),
            ));
        }
        let state = Rc::new(RuntimeDateListState {
            original,
            primary,
            backing,
            members,
        });
        self.windows.push(Rc::clone(&state));
        self.check_value()?;
        Ok(RuntimeDateList(state))
    }
}

impl tcl_cmd_core::clock::NativeClockDateOps for RuntimeClockDateOps {
    type Value = *mut TclObj;
    type List = RuntimeDateList;

    fn date_check(&self) -> Result<(), ValueError> {
        self.check_value()
    }

    fn date_wide(&mut self, value: &Self::Value) -> Result<i64, ValueError> {
        self.hold(*value)?;
        let conversion = ValueCoercion::enter(&self.interpreter, *value)?;
        let result = conversion.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide);
        let result = self.after(result)?;
        match result {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(number) => Ok(number),
            _ => Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn date_int(&mut self, value: &Self::Value) -> Result<i32, ValueError> {
        self.hold(*value)?;
        let conversion = ValueCoercion::enter(&self.interpreter, *value)?;
        let result = conversion.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Int);
        let result = self.after(result)?;
        match result {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(number) => {
                i32::try_from(number).map_err(|_| {
                    ValueCoercion::value_error(
                        &self.interpreter,
                        ValueError::ScalarNumericInputUnavailable,
                    )
                })
            }
            _ => Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn date_is_bignum(&mut self, value: &Self::Value) -> Result<bool, ValueError> {
        self.check_value()?;
        let snapshot = self.after(obj::native_object_snapshot(*value))?;
        Ok(matches!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::Numeric(
                tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Big { .. })
            )
        ))
    }

    fn date_elements(&mut self, value: &Self::Value) -> Result<Self::List, ValueError> {
        self.prepare_list(value, false)
    }

    fn date_len(&self, list: &Self::List) -> Result<usize, ValueError> {
        self.check_value()?;
        Ok(list.0.members.len())
    }

    fn date_child(&mut self, list: &Self::List, index: usize) -> Result<Self::Value, ValueError> {
        self.check_value()?;
        let child = list.0.members.get(index).copied().ok_or_else(|| {
            ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 timezone cell extent"),
            )
        })?;
        self.hold(child)
    }

    fn date_index_zero(&mut self, value: &Self::Value) -> Result<Self::Value, ValueError> {
        let list = self.prepare_list(value, true)?;
        if list.0.members.is_empty() {
            return Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 first transition cell"),
            ));
        }
        // Actual C86 ordinary ListIndex follows its own reached row conversion.
        let result = crate::list::list_index(*value, 0).map_err(|error| ValueError::ListParse {
            error: error.shared(),
            source: Vec::new(),
        });
        let child = self.after(result)?.ok_or_else(|| {
            ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 first transition cell"),
            )
        })?;
        self.hold(child)
    }

    fn date_result(
        &mut self,
        fields: &tcl_cmd_core::clock::C86CalendarFields,
        name: &Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        use tcl_cmd_core::clock::C86DateValue;
        self.check_value()?;
        let name_receipt = obj::NativePrimaryReceipt::capture(*name)?;
        let mut fresh = Vec::new();
        let values = tcl_cmd_core::clock::c86_date_values(fields, name, &self.literals)
            .into_iter()
            .map(|role| match role {
                C86DateValue::Original(value) => value,
                C86DateValue::Wide(number) => {
                    fresh.push(obj::Owned::fresh(obj::new_wide_int_obj(number)));
                    fresh.last().expect("fresh date field").as_ptr()
                }
                C86DateValue::Int(number) => {
                    fresh.push(obj::Owned::fresh(crate::capi::Tcl_NewIntObj(number)));
                    fresh.last().expect("fresh date field").as_ptr()
                }
            })
            .collect::<Vec<_>>();
        let pairs = self
            .literals
            .keys
            .iter()
            .copied()
            .zip(values)
            .collect::<Vec<_>>();
        let check = || {
            self.check_execution()?;
            name_receipt
                .check_for(*name, "changed C86 timezone name primary")
                .map_err(|error| {
                    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        error
                            .native_access_refusal()
                            .expect("physical clock name guard"),
                    );
                    self.interpreter
                        .clone()
                        .refuse_native_execution(first.clone());
                    self.interpreter.native_execution_refusal().unwrap_or(first)
                })
        };
        let result = crate::dict::new_dict_obj_native_with_stage_check(
            &pairs,
            None,
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            &check,
        )
        .inspect_err(|error| {
            if let Some(first) = error.native_execution_refusal() {
                self.interpreter
                    .clone()
                    .refuse_native_execution(first.clone());
            }
        });
        let result = result.map(obj::Owned::fresh);
        check()?;
        Ok(result?.into_native_unowned())
    }
}
