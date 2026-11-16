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

//! Original C name ingress; physical caches precede receiver observers.

#[path = "native_variable_names/native_array.rs"]
mod native_array;
#[path = "native_variable_names/native_exists.rs"]
mod native_exists;
#[path = "native_variable_names/native_tcloo.rs"]
mod native_tcloo;
#[path = "native_variable_names/native_upvar.rs"]
mod native_upvar;
use super::{native_arguments::OriginalArguments, Code, Interp};
mod native_unset;
use crate::obj::{self, Owned, TclObj};
use std::rc::{Rc, Weak};
use tcl_runtime_api::native_literal::NativeLocalNameTable;
#[cfg(test)]
use tcl_syntax::native_variable_name::NativeParsedVariableElement;
use tcl_syntax::native_variable_name::{
    NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableName,
    NativeVariableNameLookupPurpose,
};
use tcl_syntax::value::{ValueError, ValueOps};

mod alias_settlement;
mod links;
mod name_preparation;
mod read_and_assign;
mod report_capture;
mod selection_capture;
mod variable_errors;

pub(crate) type OriginalCNameStage<'a> = Option<&'a dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>;

/// Non-owning original argument addresses; existing entry checks retain authority.
struct OriginalVariableAssignmentObjects {
    lookup: *mut TclObj,
    original: *mut TclObj,
    value: *mut TclObj,
}

struct OriginalCNameSelection {
    root: Vec<u8>,
    element: Option<Vec<u8>>,
    materialised: Option<crate::vars::OriginalArrayMemberSelection>,
    simple: crate::vars::NativeOriginalNameCell,
    currency: super::native_operation_currency::NativeOperationCurrency,
}

/// A real C target selection carries the original source and target births
/// through local publication. Its Link metadata has no newly minted alias role.
pub(crate) struct OriginalCLinkTarget {
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
    link: crate::frame::Link,
    source: Option<crate::vars::NativeOriginalNameCell>,
    target: crate::vars::OriginalAliasTargetObservation,
    operation: super::native_operation_currency::NativeOperationScope,
}
impl std::ops::Deref for OriginalCLinkTarget {
    type Target = crate::frame::Link;
    fn deref(&self) -> &Self::Target {
        &self.link
    }
}
impl std::ops::DerefMut for OriginalCLinkTarget {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.link
    }
}
impl OriginalCLinkTarget {
    fn check(&self, interpreter: &Interp) -> Result<(), tcl_cmd_core::CmdError> {
        let currency = self.operation.currency();
        if let Some(source) = &self.source {
            source.check(interpreter, currency)?;
        }
        self.target.check(interpreter, &self.link, currency)
    }
}

/// Reporting bytes captured by the same original-name lookup that selected the
/// physical cell. They do not grant another lookup or materialise another name.
pub(super) struct OriginalCVariableCapture {
    pub receiver: crate::frame::VariableReceiver,
    pub home: crate::vars::TraceHome,
    pub root: Vec<u8>,
    pub element: Option<Vec<u8>>,
}

/// Reporting-only operand bytes; these never select a physical receiver.
pub(super) enum OriginalCVariableReportingInput {
    Combined(Vec<u8>),
    Separate {
        root: Vec<u8>,
        element: Option<Vec<u8>>,
    },
}
impl OriginalCVariableReportingInput {
    pub(super) fn input(&self) -> tcl_syntax::naming::NativeVariableInputForm<'_> {
        match self {
            Self::Combined(original) => {
                tcl_syntax::naming::NativeVariableInputForm::Combined(original)
            }
            Self::Separate { root, element } => {
                tcl_syntax::naming::NativeVariableInputForm::Separate {
                    root,
                    element: element.as_deref(),
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct NativeProcedureNameTable {
    layout: Vec<Option<tcl_core_types::NameBytes>>,
    table: Rc<NativeLocalNameTable<Owned>>,
}

impl Interp {
    pub(crate) fn native_c_variable_name_protocol(
        &self,
    ) -> Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol> {
        let selected = self.execution_name_policy()?.native_recipe()?;
        let protocol = self
            .native_invocation_dialect()
            .native_variable_name_protocol()?;
        (selected.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            && selected.recipe() == tcl_syntax::naming::NativeNameProtocol::C(protocol.version()))
        .then_some(protocol)
    }
    /// Share the real procedure's canonical-name header; activation retains
    /// its header, not another reference to each original name object.
    pub(super) fn refresh_native_local_name_table(&self) -> Result<(), tcl_cmd_core::CmdError> {
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return Ok(());
        };
        if protocol.local_cache_owns_procedure() {
            return Ok(());
        }
        let (procedure, layout) = {
            let frames = self.frames.borrow();
            let Some(procedure) = frames.current_c_procedure() else {
                return Ok(());
            };
            (Rc::clone(procedure), frames.native_compiled_names())
        };
        let arguments = super::native_arguments::OriginalArguments::enter(self, &[])?;
        let frame = super::native_arguments::OriginalArgumentFrame::capture(self);
        let before = procedure
            .native_local_names
            .borrow()
            .as_ref()
            .map(|table| Rc::downgrade(&table.table));
        let physical = || {
            frame.check(self, &arguments)?;
            let current = {
                let frames = self.frames.borrow();
                frames
                    .current_c_procedure()
                    .is_some_and(|current| Rc::ptr_eq(current, &procedure))
                    && frames.native_compiled_names() == layout
            };
            if !current {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable("original procedure local-name layout")
                        .into(),
                ));
            }
            Ok(())
        };
        let original_slot = || {
            physical()?;
            let retained = procedure.native_local_names.borrow();
            let same = match (&before, retained.as_ref()) {
                (None, None) => true,
                (Some(before), Some(current)) => {
                    Weak::ptr_eq(before, &Rc::downgrade(&current.table))
                }
                _ => false,
            };
            if !same {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable(
                        "original procedure local-name publication slot",
                    )
                    .into(),
                ));
            }
            Ok(())
        };
        original_slot()?;
        let reused = {
            let retained = procedure.native_local_names.borrow();
            retained
                .as_ref()
                .filter(|table| table.layout == layout)
                .map(|table| Rc::clone(&table.table))
        };
        let table = if let Some(table) = reused {
            table
        } else {
            let strings = tcl_syntax::native_string::NativeStringProtocol::C(protocol.version());
            let table = NativeLocalNameTable::create_using(
                &self.native_literal_world,
                &layout,
                strings,
                |value| obj::NativeObjectLifetime::retain(value.as_ptr()),
                |original, registration| {
                    self.native_object_string_bytes_using(
                        original.as_ptr(),
                        &|| {
                            registration()?;
                            original_slot()?;
                            obj::check_native_liveness(original.as_ptr())
                                .map_err(|error| arguments.retain_refusal(error.into()))
                        },
                        &|error| arguments.retain_refusal(error.into()),
                    )
                    .map(|bytes| bytes.to_vec())
                },
                |bytes| {
                    super::native_literal_pool::registered_string(
                        bytes,
                        self.native_invocation_dialect(),
                    )
                    .map_err(|error| arguments.retain_refusal(error.into()))
                },
                &tcl_runtime_api::native_literal::NativeLiteralCallbacks {
                    check: &original_slot,
                    refuse: &|error: ValueError| arguments.retain_refusal(error.into()),
                    retain: &|error| arguments.retain_refusal(error),
                },
            )?;
            original_slot()?;
            let incoming = NativeProcedureNameTable {
                layout: layout.clone(),
                table: Rc::clone(&table),
            };
            let retired = procedure.native_local_names.borrow_mut().take();
            drop(retired);
            physical()?;
            if procedure.native_local_names.borrow().is_some() {
                return Err(arguments.retain_refusal(
                    ValueError::CommandProtocolUnavailable(
                        "replaced original procedure local-name publication slot",
                    )
                    .into(),
                ));
            }
            *procedure.native_local_names.borrow_mut() = Some(incoming);
            table
        };
        physical()?;
        let retired = self.frames.borrow_mut().take_native_local_name_table();
        drop(retired);
        physical()?;
        let selected = procedure
            .native_local_names
            .borrow()
            .as_ref()
            .is_some_and(|selected| Rc::ptr_eq(&selected.table, &table));
        if !selected || self.frames.borrow().native_local_name_table().is_some() {
            return Err(arguments.retain_refusal(
                ValueError::CommandProtocolUnavailable(
                    "replaced original activation local-name publication slot",
                )
                .into(),
            ));
        }
        self.frames
            .borrow_mut()
            .install_native_local_name_table(table);
        physical()?;
        Ok(())
    }

    fn original_c_local_index(&self, original: *mut TclObj) -> Option<usize> {
        let frames = self.frames.borrow();
        let protocol = self.native_c_variable_name_protocol()?;
        obj::native_variable_name::with_local(original, |cache| {
            if cache.protocol != protocol || frames.compiled_slot_name(cache.index).is_none() {
                return None;
            }
            let matches = match &cache.owner {
                NativeLocalVariableOwner::Procedure(procedure) => frames
                    .current_c_procedure()
                    .is_some_and(|current| Rc::ptr_eq(current, procedure.owner())),
                NativeLocalVariableOwner::Name(canonical) => frames
                    .native_local_name_table()?
                    .names
                    .get(cache.index)?
                    .as_ref()
                    .is_some_and(|current| {
                        current.as_ptr() == canonical.as_ref().map_or(original, Owned::as_ptr)
                    }),
            };
            matches.then_some(cache.index)
        })
        .flatten()
    }

    /// Inspect the inherited operation after a reached name-cache callback.
    /// The callback's effects remain reached; later cell/cache work may not
    /// acquire a fresh receipt or use a retired original header.
    fn check_original_c_name_operation(&self, originals: &[*mut TclObj]) -> Result<(), ValueError> {
        self.check_entered_native_operation().map_err(|_| {
            ValueError::CommandProtocolUnavailable("entered original variable-name operation")
        })?;
        for &original in originals {
            obj::check_native_liveness(original)?;
        }
        Ok(())
    }

    pub(super) fn check_original_c_name_stage(
        &self,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some(check) = stage {
            check().inspect_err(|error| {
                if let Some(first) = error.native_execution_refusal() {
                    self.clone().refuse_native_execution(first.clone());
                }
            })?;
        }
        Ok(())
    }

    fn check_original_c_name_operation_using(
        &self,
        originals: &[*mut TclObj],
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check_original_c_name_stage(stage)?;
        self.check_original_c_name_operation(originals)
            .map_err(Into::into)
    }

    pub(crate) fn original_c_name_bytes_using(
        &self,
        original: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError> {
        match stage {
            None => self
                .native_object_string_bytes(original)
                .map_err(Into::into),
            Some(_) => self.native_object_string_bytes_using(
                original,
                &|| self.check_original_c_name_operation_using(&[original], stage),
                &|error| {
                    tcl_cmd_core::CmdError::from_execution_refusal(
                        self.clone().refuse_completion_value_access(error),
                    )
                },
            ),
        }
    }

    pub(super) fn read_original_c_variable_using(
        &mut self,
        original: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported_using(original, original, false, stage)
    }

    pub(super) fn store_original_c_variable_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.assign_original_c_selection_using(
            original,
            value,
            true,
            NativeVariableNameLookupPurpose::Write,
            stage,
        )
    }

    fn install_original_c_name_cache_using(
        &self,
        original: *mut TclObj,
        selected: &crate::vars::NativeOriginalNameCell,
        currency: &super::native_operation_currency::NativeOperationCurrency,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check_original_c_name_stage(stage)?;
        selected.check(self, currency)?;
        self.check_original_c_name_operation_using(&[original], stage)?;
        let retain = |error: &tcl_cmd_core::CmdError| {
            if let Some(first) = error.native_execution_refusal() {
                self.clone().refuse_native_execution(first.clone());
            }
        };
        let mut current = || {
            selected
                .check(self, currency)
                .and_then(|()| self.check_original_c_name_operation_using(&[original], stage))
        };
        let _original_lifetime = obj::NativeObjectLifetime::retain(original);
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_variable_name_protocol()
            .expect("C variable issuer");
        let Some(index) = selected.compiled else {
            let result = obj::native_variable_name::install_parsed_with_stage_check(
                original,
                NativeParsedVariableName {
                    protocol,
                    array: None,
                },
                dialect,
                &mut current,
                &retain,
            );
            selected.check(self, currency)?;
            self.check_original_c_name_operation_using(&[original], stage)?;
            return result;
        };
        if protocol.local_cache_owns_procedure() {
            let procedure = self.frames.borrow().current_c_procedure().cloned().ok_or(
                ValueError::CommandProtocolUnavailable("actual C8.4 procedure cache owner"),
            )?;
            procedure
                .retain_native_compiled_frame_names(self.frames.borrow().native_compiled_names());
            let result = obj::native_variable_name::install_local_with_stage_check(
                original,
                NativeLocalVariableName {
                    protocol,
                    index,
                    owner: NativeLocalVariableOwner::Procedure(
                        tcl_runtime_api::native_procedure_roles::NativeProcedureReference::acquire(
                            &procedure,
                        )
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable(
                                "retired C8.4 procedure cache owner",
                            )
                        })?,
                    ),
                },
                dialect,
                &mut current,
                &retain,
            );
            selected.check(self, currency)?;
            self.check_original_c_name_operation_using(&[original], stage)?;
            return result;
        }
        let canonical = {
            let frames = self.frames.borrow();
            frames
                .native_local_name_table()
                .and_then(|table| table.names.get(index))
                .and_then(Option::as_ref)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "actual canonical local name owner",
                ))?
                .as_ptr()
        };
        // Keep only allocation safety across a reached free callback. Native
        // sharing and the selected canonical header remain independent.
        let _canonical_lifetime = obj::NativeObjectLifetime::retain(canonical);
        let owner = NativeLocalVariableOwner::Name(
            (canonical != original).then(|| Owned::retain(canonical)),
        );
        let result = obj::native_variable_name::install_local_with_stage_check(
            original,
            NativeLocalVariableName {
                protocol,
                index,
                owner,
            },
            dialect,
            &mut current,
            &retain,
        );
        selected.check(self, currency)?;
        self.check_original_c_name_operation_using(&[original, canonical], stage)?;
        result?;
        if canonical != original && protocol.installs_canonical_local_self_cache() {
            let result = obj::discard_native_internal_representation_with_stage_check(
                canonical,
                &mut current,
            );
            selected.check(self, currency)?;
            self.check_original_c_name_operation_using(&[original, canonical], stage)?;
            result?;
            let result = obj::native_variable_name::install_local_with_stage_check(
                canonical,
                NativeLocalVariableName {
                    protocol,
                    index,
                    owner: NativeLocalVariableOwner::Name(None),
                },
                dialect,
                &mut current,
                &retain,
            );
            selected.check(self, currency)?;
            self.check_original_c_name_operation_using(&[original, canonical], stage)?;
            result?;
        } else if canonical != original
            && protocol.version() == tcl_dialect::TclVersion::V8_6
            && !obj::native_variable_name::with_local(canonical, |cache| {
                matches!(cache.owner, NativeLocalVariableOwner::Name(None))
            })
            .unwrap_or(false)
        {
            let result = obj::discard_native_internal_representation_with_stage_check(
                canonical,
                &mut current,
            );
            selected.check(self, currency)?;
            self.check_original_c_name_operation_using(&[original, canonical], stage)?;
            result?;
        }
        Ok(())
    }

    const fn changes_a_variable(purpose: NativeVariableNameLookupPurpose) -> bool {
        use NativeVariableNameLookupPurpose as Purpose;
        matches!(
            purpose,
            Purpose::Write
                | Purpose::QuietWrite
                | Purpose::IncrementRead
                | Purpose::IncrementExisting
                | Purpose::ArrayMake
                | Purpose::Define
                | Purpose::Unset
                | Purpose::QuietUnset
        )
    }

    /// Refuse a receiver a confined interpreter may not change: every store,
    /// increment, definition and unset passes here with the cell it selected,
    /// and only a cell of the running procedure's own frame is its to change.
    /// A link or a reference changes nothing by itself; the store through it
    /// selects the cell it reaches and is asked then.
    pub(in crate::interp) fn refuse_confined_receiver(
        &self,
        home: &crate::vars::TraceHome,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<(), crate::frame::VarError> {
        if !self.stores_confined() || !Self::changes_a_variable(purpose) {
            return Ok(());
        }
        let frames = self.frames.borrow();
        if frames.in_proc() && home.level == Some(frames.current_level()) {
            Ok(())
        } else {
            Err(crate::frame::VarError::Confined)
        }
    }
}

#[cfg(test)]
#[path = "native_variable_names/original_handoff_tests.rs"]
mod original_handoff_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_assignment_tests;
