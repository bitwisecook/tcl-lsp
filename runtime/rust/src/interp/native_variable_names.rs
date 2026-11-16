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
use tcl_syntax::{
    native_variable_name::{
        NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableElement,
        NativeParsedVariableName, NativeVariableNameLookupPurpose,
    },
    value::{ValueError, ValueOps},
};

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

    fn prepare_original_c_name(
        &mut self,
        original: *mut TclObj,
        create: bool,
    ) -> Result<OriginalCNameSelection, Code> {
        self.prepare_original_c_name_using(original, create, None)
    }

    fn prepare_original_c_name_using(
        &mut self,
        original: *mut TclObj,
        create: bool,
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCNameSelection, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let purpose = if create {
            NativeVariableNameLookupPurpose::Write
        } else {
            NativeVariableNameLookupPurpose::Read
        };
        self.prepare_original_c_name_for_using(original, purpose, stage)?
            .ok_or_else(|| {
                let bytes = self.original_c_name_bytes_using(original, stage);
                match bytes {
                    Ok(bytes) => self.no_such_variable_using(&bytes, None, stage),
                    Err(error) => self.report_cmd_error_using(error, stage),
                }
            })
    }

    fn prepare_original_c_name_for(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_for_using(original, purpose, None)
    }

    fn prepare_original_c_name_for_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.prepare_original_c_name_in_using(original, purpose, None, stage)
    }

    fn prepare_original_c_name_in(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_in_using(original, purpose, namespace, None)
    }

    fn prepare_original_c_name_in_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.prepare_original_c_name_parts_using(original, purpose, namespace, None, stage)
    }

    fn prepare_original_c_name_parts(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_parts_using(
            original,
            purpose,
            namespace,
            explicit_element,
            None,
        )
    }

    fn prepare_original_c_name_parts_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        self.check_original_c_name_operation_using(&[original], stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let _original_lifetime = obj::NativeObjectLifetime::retain(original);
        let create = purpose.creates_entries();
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_variable_name_protocol()
            .expect("C variable issuer");
        if !protocol.uses_original_name_cache(purpose) {
            return self.prepare_original_c_byte_name_parts_using(
                original,
                namespace,
                explicit_element,
                stage,
            );
        }
        if !protocol.cache_precedes_string_getter() {
            self.original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
        }
        // Project only allocation leases/bytes while the original Parsed owner
        // is borrowed. The reached element getter runs after that projection.
        let projected = explicit_element
            .is_none()
            .then(|| {
                obj::native_variable_name::with_parsed(original, |cache| {
                    if cache.protocol != protocol {
                        return None;
                    }
                    cache.array.as_ref().map(|(root, element)| {
                        let (bytes, object) = match element {
                            NativeParsedVariableElement::Bytes(bytes) => {
                                (Some(tcl_core_types::c_string_extent(bytes).to_vec()), None)
                            }
                            NativeParsedVariableElement::Object(object) => (
                                None,
                                Some(obj::NativeObjectLifetime::retain(object.as_ptr())),
                            ),
                        };
                        (
                            obj::NativeObjectLifetime::retain(root.as_ptr()),
                            bytes,
                            object,
                        )
                    })
                })
            })
            .flatten()
            .flatten();
        let parsed_primary = if stage.is_some() && projected.is_some() {
            Some(
                obj::NativePrimaryReceipt::capture(original).map_err(|error| {
                    self.report_cmd_error_using(
                        tcl_cmd_core::CmdError::from_execution_refusal(
                            self.clone().refuse_completion_value_access(error),
                        ),
                        stage,
                    )
                })?,
            )
        } else {
            None
        };
        let actual = self.clone();
        let original_stage = stage;
        let parsed_stage = || {
            actual.check_original_c_name_stage(original_stage)?;
            if let Some(primary) = &parsed_primary {
                primary
                    .check_for(original, "changed original Parsed variable primary")
                    .map_err(|error| {
                        tcl_cmd_core::CmdError::from_execution_refusal(
                            actual.clone().refuse_completion_value_access(error),
                        )
                    })?;
            }
            Ok(())
        };
        let stage = stage.map(|_| &parsed_stage as &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>);
        let mut parts = if let Some((root, bytes, element)) = &projected {
            let bytes = match bytes {
                Some(bytes) => bytes.clone(),
                None => self
                    .original_c_name_bytes_using(
                        element
                            .as_ref()
                            .expect("projected original element")
                            .as_ptr(),
                        stage,
                    )
                    .map_err(|error| self.report_cmd_error_using(error, stage))?
                    .to_vec(),
            };
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if let Some(primary) = &parsed_primary {
                primary
                    .check_for(original, "changed original Parsed variable primary")
                    .map_err(|error| {
                        let first = self.refuse_completion_value_access(error);
                        self.report_cmd_error_using(
                            tcl_cmd_core::CmdError::from_execution_refusal(first),
                            stage,
                        )
                    })?;
                self.check_original_c_name_operation_using(&[original, root.as_ptr()], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
            }
            Some((root.as_ptr(), bytes))
        } else {
            None
        };
        let parsed_scalar = obj::native_variable_name::with_parsed(original, |cache| {
            cache.protocol == protocol && cache.array.is_none()
        })
        .unwrap_or(false);
        if explicit_element.is_none()
            && parts.is_none()
            && !parsed_scalar
            && obj::native_variable_name::with_local(original, |_| ()).is_none()
        {
            let original_bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if let Some((root, element)) = protocol.parsed_array_parts(&original_bytes) {
                let root = Owned::fresh(obj::new_string_bytes(root));
                let element = match protocol.new_element_bytes(element) {
                    Some(bytes) => NativeParsedVariableElement::Bytes(bytes),
                    None => NativeParsedVariableElement::Object(Owned::fresh(
                        obj::new_string_bytes(element),
                    )),
                };
                let mut current = || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                        .and_then(|()| {
                            self.check_original_c_name_operation_using(&[original], stage)
                        })
                };
                let retain = |error: &tcl_cmd_core::CmdError| {
                    if let Some(first) = error.native_execution_refusal() {
                        self.clone().refuse_native_execution(first.clone());
                    }
                };
                let result = obj::native_variable_name::install_parsed_with_stage_check(
                    original,
                    NativeParsedVariableName {
                        protocol,
                        array: Some((root, element)),
                    },
                    dialect,
                    &mut current,
                    &retain,
                );
                self.check_original_c_name_operation_using(&[original], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                result.map_err(|error| self.report_cmd_error_using(error, stage))?;
                return self.prepare_original_c_name_in_using(original, purpose, namespace, stage);
            }
        }
        let (root, element) = parts.take().map_or(
            (original, explicit_element.map(<[u8]>::to_vec)),
            |(root, element)| (root, Some(element)),
        );
        let cached = namespace
            .is_none()
            .then(|| self.original_c_local_index(root))
            .flatten();
        let root_bytes = if let Some(index) = cached {
            self.frames
                .borrow()
                .compiled_slot_name(index)
                .expect("authenticated actual local cell")
                .to_vec()
        } else {
            let root_bytes = self
                .original_c_name_bytes_using(root, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            let kind = obj::obj_type_ptr(root);
            // SAFETY: a non-NULL type pointer names the live original descriptor.
            let free_hook = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
            if protocol.retires_before_simple_lookup(free_hook) {
                let _root_lifetime = obj::NativeObjectLifetime::retain(root);
                let mut current = || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                        .and_then(|()| {
                            self.check_original_c_name_operation_using(&[original, root], stage)
                        })
                };
                let result = obj::discard_native_internal_representation_with_stage_check(
                    root,
                    &mut current,
                );
                self.check_original_c_name_operation_using(&[original, root], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                result.map_err(|error| self.report_cmd_error_using(error, stage))?;
            }
            root_bytes
        };
        let selected = if let Some(index) = cached {
            crate::vars::prepare_original_c_indexed_name_cell(
                &mut self.frames.borrow_mut(),
                &self.namespaces.borrow(),
                index,
            )
            .map(Some)
        } else {
            crate::vars::prepare_original_c_variable_cell(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                crate::vars::NativeOriginalNameScope {
                    current: self.current_ns.get(),
                    namespace,
                },
                &root_bytes,
                root,
                create,
                protocol.version(),
            )
        };
        let selected = match selected {
            Ok(Some(selected)) => selected,
            Ok(None) => return Ok(None),
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original simple variable cell")
                        .into(),
                    stage,
                ));
            }
            Err(error) if create => {
                let reason = match error {
                    crate::frame::VarError::NoSuchNamespace => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace
                    }
                    crate::frame::VarError::IsScalar => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray
                    }
                    _ => {
                        return Err(
                            self.original_c_lookup_var_error_using(original, purpose, error, stage)
                        );
                    }
                };
                return Err(self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    reason,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                    stage,
                ));
            }
            Err(_) => return Ok(None),
        };
        if cached.is_none() {
            self.install_original_c_name_cache_using(root, &selected, operation.currency(), stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
        }
        let mut materialised = None;
        if create
            && selected.compiled.is_none()
            && (protocol.version() >= tcl_dialect::TclVersion::V8_5
                || purpose == NativeVariableNameLookupPurpose::Define)
        {
            if let Some(element) = &element {
                let operation = super::native_operation_currency::NativeOperationScope::enter(self)
                    .map_err(|error| {
                        self.report_cmd_error_using(
                            tcl_cmd_core::CmdError::from_execution_refusal(error),
                            stage,
                        )
                    })?;
                let retained_key = self.original_c_parsed_element_key(original);
                selected
                    .check(self, operation.currency())
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                let prepared = crate::vars::capture_original_c_selected_element_materialisation(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &selected,
                    element,
                    retained_key.as_ref().map(Owned::as_ptr),
                    purpose.creates_element_entries(),
                    protocol,
                );
                let reached = match prepared {
                    Ok(prepared) => prepared
                        .finish_using(self, operation.currency(), stage)
                        .map_err(Ok),
                    Err(crate::frame::JimLinkStoreError::Variable(error)) => Err(Err(error)),
                    Err(crate::frame::JimLinkStoreError::Command(error)) => Err(Ok(error)),
                };
                if let Err(Ok(error)) = &reached {
                    if let Some(first) = error.native_execution_refusal() {
                        self.refuse_native_execution(first.clone());
                    }
                }
                // The real parsed-key temporary retires after its selected
                // preparation, before any ordinary diagnostic continuation.
                drop(retained_key);
                operation
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                self.check_original_c_name_operation_using(&[original, root], stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                materialised = Some(reached.map_err(|error| {
                    let error = match error {
                        Ok(error) => return self.report_cmd_error_using(error, stage),
                        Err(error) => error,
                    };
                    let reason = match error {
                        crate::frame::VarError::IsScalar if purpose == NativeVariableNameLookupPurpose::Define => tcl_syntax::naming::NativeVariableDiagnosticReason::ArrayElement,
                        crate::frame::VarError::IsScalar => tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray,
                        crate::frame::VarError::NoSuchNamespace => tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace,
                        _ => return self.original_c_lookup_var_error_using(original, purpose, error, stage),
                    };
                    self.original_c_variable_failure_for_object_using(original, purpose, reason,
                        tcl_syntax::naming::NativeVariableFailureSite::NameLookup, stage)
                })?);
            }
        }
        Ok(Some(OriginalCNameSelection {
            root: root_bytes,
            element,
            materialised,
            simple: selected,
            currency: operation.currency().clone(),
        }))
    }

    /// Original VARIABLE/NSUPVAR object lookup and exact indexed alias binding.
    /// Legacy C84 existence obtains only bytes from the original name. Its
    /// selected cell and read observers remain the same receiver owners; no
    /// parsed or local primary is inspected, discarded or installed here.
    ///
    fn prepare_original_c_byte_name_parts_using(
        &mut self,
        original: *mut TclObj,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let bytes = self
            .original_c_name_bytes_using(original, stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?
            .to_vec();
        let protocol = self.native_c_variable_name_protocol().ok_or_else(|| {
            self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("original C byte variable lookup").into(),
                stage,
            )
        })?;
        let input = tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
            .combined_variable_input(tcl_core_types::c_string_extent(&bytes));
        if input.element().is_some() && explicit_element.is_some() {
            return Ok(None);
        }
        let root = input.root().selected().to_vec();
        let element = input
            .element()
            .map(|part| part.selected().to_vec())
            .or_else(|| {
                explicit_element.map(|part| tcl_core_types::c_string_extent(part).to_vec())
            });
        let selected = crate::vars::prepare_original_c_variable_cell(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            crate::vars::NativeOriginalNameScope {
                current: self.current_ns.get(),
                namespace,
            },
            &root,
            original,
            false,
            protocol.version(),
        );
        match selected {
            Ok(Some(selected)) => Ok(Some(OriginalCNameSelection {
                root,
                element,
                materialised: None,
                simple: selected,
                currency: operation.currency().clone(),
            })),
            Err(crate::frame::VarError::NameProtocolUnavailable) => Err(self
                .report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original byte variable cell")
                        .into(),
                    stage,
                )),
            Ok(None) | Err(_) => Ok(None),
        }
    }

    #[cfg(test)]
    pub(crate) fn link_original_compiled_namespace_variable(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        slot: usize,
        declare: bool,
    ) -> Code {
        self.link_original_compiled_namespace_variable_using(
            original, namespace, slot, declare, None,
        )
    }

    pub(crate) fn link_original_compiled_namespace_variable_using(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        slot: usize,
        declare: bool,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let purpose = NativeVariableNameLookupPurpose::Link;
        let selection = match self.prepare_original_c_name_in_using(
            original,
            purpose,
            Some(namespace),
            stage,
        ) {
            Ok(Some(selection)) => selection,
            Ok(None) => {
                return self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original namespace binding lookup")
                        .into(),
                    stage,
                );
            }
            Err(code) => return code,
        };
        let mut link =
            match self.prepare_selected_c_namespace_target_using(original, &selection, stage) {
                Ok(target) => target,
                Err(code) => return code,
            };
        if let Err(error) = self.prepare_upvar_target(&mut link) {
            return self.original_c_lookup_var_error_using(original, purpose, error, stage);
        }
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if declare {
            match link.home {
                crate::frame::VarHome::Namespace(namespace) => self
                    .namespaces
                    .borrow_mut()
                    .var_table_mut(namespace)
                    .mark_namespace_declared(&link.name),
                crate::frame::VarHome::Frame(_) => {
                    return self.report_cmd_error_using(
                        ValueError::CommandProtocolUnavailable(
                            "namespace variable declaration target",
                        )
                        .into(),
                        stage,
                    );
                }
            }
        }
        self.settle_original_c_local_alias_using(Some(slot), &selection.root, link, stage)
    }

    /// The automatic TclOO compiler resolver supplies an actual installed
    /// ordinal and declared namespace target, rather than an object-name query.
    pub(crate) fn link_original_tcloo_compiled_namespace_variable(
        &mut self,
        namespace: crate::namespace::NsId,
        slot: usize,
        name: &[u8],
    ) -> Code {
        let operation = match super::native_operation_currency::NativeOperationScope::enter(self) {
            Ok(operation) => operation,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let selected = crate::vars::select_tcloo_compiled_variable(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            namespace,
            name,
        );
        let (link, source) = match selected {
            Ok(selected) => selected,
            Err(crate::frame::JimLinkStoreError::Variable(error)) => {
                return crate::builtins::var_error(self, name, error);
            }
            Err(crate::frame::JimLinkStoreError::Command(error)) => {
                return self.report_cmd_error(error);
            }
        };
        if let Err(error) = source.check(self, operation.currency()) {
            return self.report_cmd_error(error);
        }
        let target_receipt = {
            let frames = self.frames.borrow();
            let namespaces = self.namespaces();
            crate::vars::OriginalAliasTargetObservation::capture(&frames, &namespaces, &link)
        };
        let target = match target_receipt {
            Ok(target) => target,
            Err(_) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "original compiled resolver target receipt",
                    )
                    .into(),
                );
            }
        };
        let target = OriginalCLinkTarget {
            protocol: self
                .native_c_variable_name_protocol()
                .expect("selected C alias recipe"),
            link,
            source: Some(source),
            target,
            operation,
        };
        let local = self
            .frames
            .borrow()
            .compiled_slot_name(slot)
            .map(<[u8]>::to_vec);
        let Some(local) = local else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original compiled resolver local ordinal")
                    .into(),
            );
        };
        self.settle_original_c_alias_at_with_origin(
            crate::frame::VarHome::Frame(self.current_level()),
            Some(slot),
            &local,
            target,
            tcl_runtime_api::FrameLinkOrigin::TclOoInstanceCompiled,
        )
    }

    fn original_c_alias_error_using(
        &mut self,
        message: &[u8],
        code: &[u8],
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        let code = if protocol.alias_rejection_sets_error_code() {
            tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec())
        } else {
            tcl_cmd_core::CmdErrorCodeUpdate::Default
        };
        self.original_c_alias_diagnostic_using(message, code, stage)
    }

    fn original_c_alias_diagnostic_using(
        &mut self,
        message: &[u8],
        code: tcl_cmd_core::CmdErrorCodeUpdate,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        self.report_cmd_error_using(
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message: message.to_vec(),
                string_result: protocol.diagnostic_string_protocol(),
                error_code: code,
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
            stage,
        )
    }

    fn settle_original_c_local_alias(
        &mut self,
        compiled: Option<usize>,
        name: &[u8],
        target: OriginalCLinkTarget,
    ) -> Code {
        self.settle_original_c_local_alias_using(compiled, name, target, None)
    }

    fn settle_original_c_local_alias_using(
        &mut self,
        compiled: Option<usize>,
        name: &[u8],
        target: OriginalCLinkTarget,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let local = if let Some(slot) = compiled {
            let name = self
                .frames
                .borrow()
                .compiled_slot_name(slot)
                .map(<[u8]>::to_vec);
            let Some(name) = name else {
                return self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original namespace alias local slot")
                        .into(),
                    stage,
                );
            };
            name
        } else {
            let protocol = self
                .native_c_variable_name_protocol()
                .expect("actual C alias names");
            tcl_syntax::naming::NativeNameProtocol::C(protocol.version())
                .variable_root_input(name)
                .selected()
                .to_vec()
        };
        let home = crate::frame::VarHome::Frame(self.frames.borrow().current_level());
        self.settle_original_c_alias_at_using(home, compiled, &local, target, stage)
    }

    fn settle_original_c_alias_at_using(
        &mut self,
        home: crate::frame::VarHome,
        compiled: Option<usize>,
        local: &[u8],
        target: OriginalCLinkTarget,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        self.settle_original_c_alias_at_with_origin_using(
            home,
            compiled,
            local,
            target,
            tcl_runtime_api::FrameLinkOrigin::Ordinary,
            stage,
        )
    }

    fn settle_original_c_alias_at_with_origin(
        &mut self,
        home: crate::frame::VarHome,
        compiled: Option<usize>,
        local: &[u8],
        target: OriginalCLinkTarget,
        origin: tcl_runtime_api::FrameLinkOrigin,
    ) -> Code {
        self.settle_original_c_alias_at_with_origin_using(
            home, compiled, local, target, origin, None,
        )
    }

    fn settle_original_c_alias_at_with_origin_using(
        &mut self,
        home: crate::frame::VarHome,
        compiled: Option<usize>,
        local: &[u8],
        target: OriginalCLinkTarget,
        origin: tcl_runtime_api::FrameLinkOrigin,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if let Err(error) = target.check(self) {
            return self.report_cmd_error_using(error, stage);
        }
        let link = target.link.clone();
        let (frame_level, namespace, identity) = match home {
            crate::frame::VarHome::Frame(level) => (
                Some(level),
                None,
                if let Some(slot) = compiled {
                    self.frames.borrow().original_compiled_slot_identity(slot)
                } else {
                    self.frames
                        .borrow()
                        .table(level)
                        .and_then(|table| table.binding_id(local))
                },
            ),
            crate::frame::VarHome::Namespace(namespace) => (
                None,
                Some(namespace),
                self.namespaces
                    .borrow()
                    .var_table(namespace)
                    .binding_id(local),
            ),
        };
        let traced = self.traces.borrow().traces.iter().any(|trace| {
            trace.frame_level == frame_level
                && trace.ns == namespace
                && trace.base == local
                && trace.binding_id == identity
        });
        let local_observation = match compiled {
            Some(slot) => crate::vars::OriginalAliasTargetObservation::capture_compiled(
                &mut self.frames.borrow_mut(),
                &self.namespaces(),
                slot,
            ),
            None => crate::vars::OriginalAliasTargetObservation::capture_home(
                &self.frames.borrow(),
                &self.namespaces(),
                home,
                local,
            ),
        };
        let local_observation = match local_observation {
            Ok(observation) => observation,
            Err(_) => {
                return self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original local alias table receipt")
                        .into(),
                    stage,
                );
            }
        };
        let bound = match home {
            crate::frame::VarHome::Frame(_) => {
                if let Some(slot) = compiled {
                    self.frames
                        .borrow_mut()
                        .prepare_original_compiled_alias(slot, &link, traced, origin)
                } else {
                    self.frames
                        .borrow_mut()
                        .prepare_original_local_alias(local, &link, traced)
                }
            }
            crate::frame::VarHome::Namespace(namespace) => self
                .namespaces
                .borrow_mut()
                .var_table_mut(namespace)
                .prepare_original_alias(local, &link, traced),
        };
        match bound {
            Ok(prepared) => {
                self.note_variable_link(local, &link.name);
                let reached = prepared.finish(
                    target.protocol.alias_publication_order(),
                    || {
                        self.check_original_c_name_stage(stage)?;
                        target.check(self).and_then(|()| {
                            local_observation.check_cell(self, target.operation.currency())
                        })
                    },
                    |error| {
                        tcl_cmd_core::CmdError::from_execution_refusal(
                            self.clone().refuse_completion_value_access(error),
                        )
                    },
                );
                match reached {
                    Ok(()) => Code::Ok,
                    Err(error) => self.report_cmd_error_using(error, stage),
                }
            }
            Err(crate::frame::NativeCompiledAliasError::Unavailable) => self
                .report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original namespace alias local cell")
                        .into(),
                    stage,
                ),
            Err(crate::frame::NativeCompiledAliasError::SelfLink) => self
                .original_c_alias_error_using(
                    b"can't upvar from variable to itself",
                    b"TCL UPVAR SELF",
                    stage,
                ),
            Err(error) => {
                let mut message = b"variable \"".to_vec();
                message.extend_from_slice(tcl_core_types::c_string_extent(local));
                message.extend_from_slice(match error {
                    crate::frame::NativeCompiledAliasError::Traced => {
                        b"\" has traces: can't use for upvar".as_slice()
                    }
                    crate::frame::NativeCompiledAliasError::Exists => {
                        b"\" already exists".as_slice()
                    }
                    _ => unreachable!("handled alias errors"),
                });
                self.original_c_alias_error_using(
                    &message,
                    if error == crate::frame::NativeCompiledAliasError::Traced {
                        b"TCL UPVAR TRACED"
                    } else {
                        b"TCL UPVAR EXISTS"
                    },
                    stage,
                )
            }
        }
    }

    pub(crate) fn define_original_c_namespace_variable(
        &mut self,
        original: *mut TclObj,
        value: Option<*mut TclObj>,
    ) -> Code {
        let spelling = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let namespace = self.current_ns.get();
        let selected = match self.prepare_original_c_name_in(
            original,
            NativeVariableNameLookupPurpose::Define,
            Some(namespace),
        ) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original namespace declaration cell")
                        .into(),
                );
            }
            Err(code) => return code,
        };
        if selected.element.is_some() {
            return self.original_c_variable_failure_for_object(
                original,
                NativeVariableNameLookupPurpose::Define,
                tcl_syntax::naming::NativeVariableDiagnosticReason::ArrayElement,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            );
        }
        let target = crate::vars::original_namespace_link_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            namespace,
            &selected.root,
            None,
        );
        let link = match target {
            Ok(link) => link,
            Err(error) => {
                return self.original_c_lookup_var_error(
                    original,
                    NativeVariableNameLookupPurpose::Define,
                    error,
                );
            }
        };
        match link.home {
            crate::frame::VarHome::Namespace(namespace) => self
                .namespaces
                .borrow_mut()
                .var_table_mut(namespace)
                .mark_namespace_declared(&link.name),
            crate::frame::VarHome::Frame(_) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("namespace declaration target").into(),
                );
            }
        }
        drop(link);
        if let Some(value) = value {
            if let Err(code) = self.assign_prepared_original_c_selection(
                original,
                value,
                false,
                NativeVariableNameLookupPurpose::Write,
                &selected,
            ) {
                return code;
            }
        }
        let in_proc = self.frames.borrow().in_proc();
        if in_proc {
            let protocol = self
                .native_c_variable_name_protocol()
                .expect("actual C declaration names");
            let local = tcl_syntax::naming::variable_local_name_bytes(
                tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                &spelling,
            );
            return self.link_original_c_namespace_alias(original, namespace, &local, &spelling);
        }
        Code::Ok
    }

    /// Namespace-upvar retains its original target object. Its local CString
    /// API creates a fresh scalar name before the shared simple alias lookup.
    pub(crate) fn link_original_c_namespace_objects_using(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        local: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        let target =
            match self.prepare_original_c_namespace_link_target_using(original, namespace, stage) {
                Ok(target) => target,
                Err(code) => return code,
            };
        self.bind_selected_c_alias_local_using(local, target, stage)
    }

    /// Prepare the original namespace receiver before a caller's local-link
    /// publication. The caller retains its actual frame and namespace purpose.
    pub(crate) fn prepare_original_c_namespace_link_target(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
    ) -> Result<OriginalCLinkTarget, Code> {
        self.prepare_original_c_namespace_link_target_using(original, namespace, None)
    }

    fn prepare_original_c_namespace_link_target_using(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCLinkTarget, Code> {
        let selected = match self.prepare_original_c_name_in_using(
            original,
            NativeVariableNameLookupPurpose::Link,
            Some(namespace),
            stage,
        ) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return Err(self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original namespace upvar target")
                        .into(),
                    stage,
                ));
            }
            Err(code) => return Err(code),
        };
        self.prepare_selected_c_namespace_target_using(original, &selected, stage)
    }

    pub(crate) fn link_original_c_namespace_alias(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        local: &[u8],
        spelling: &[u8],
    ) -> Code {
        let purpose = NativeVariableNameLookupPurpose::Link;
        let selected = match self.prepare_original_c_name_in(original, purpose, Some(namespace)) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original namespace alias target")
                        .into(),
                );
            }
            Err(code) => return code,
        };
        let mut target = match self.prepare_selected_c_namespace_target(original, &selected) {
            Ok(target) => target,
            Err(code) => return code,
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            return self.original_c_lookup_var_error(original, purpose, error);
        }
        let qualified = self
            .native_c_variable_name_protocol()
            .expect("actual C alias names")
            .alias_local_input(spelling)
            .qualification()
            != tcl_syntax::naming::NativeNameQualification::Unqualified;
        let tail = qualified.then(|| Owned::fresh(obj::new_string_bytes(local)));
        let local_original = tail.as_ref().map_or(original, Owned::as_ptr);
        self.bind_selected_c_alias_local(local_original, target)
    }

    /// ObjMakeUpvar's local side uses simple-name lookup, without converting
    /// the caller's original object to a parsed or indexed variable name.
    #[cfg(test)]
    pub(crate) fn bind_original_c_alias_local(
        &mut self,
        original: *mut TclObj,
        target: crate::frame::Link,
    ) -> Code {
        let operation = match super::native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return self.refuse_native_execution(first),
        };
        let observed = crate::vars::OriginalAliasTargetObservation::capture(
            &self.frames.borrow(),
            &self.namespaces(),
            &target,
        );
        let observed = match observed {
            Ok(observed) => observed,
            Err(error) => {
                return self.original_c_lookup_var_error(
                    original,
                    NativeVariableNameLookupPurpose::Link,
                    error,
                );
            }
        };
        self.bind_selected_c_alias_local(
            original,
            OriginalCLinkTarget {
                protocol: self
                    .native_c_variable_name_protocol()
                    .expect("selected C alias recipe"),
                link: target,
                source: None,
                target: observed,
                operation,
            },
        )
    }
    pub(crate) fn bind_selected_c_alias_local(
        &mut self,
        original: *mut TclObj,
        target: OriginalCLinkTarget,
    ) -> Code {
        self.bind_selected_c_alias_local_using(original, target, None)
    }

    fn bind_selected_c_alias_local_using(
        &mut self,
        original: *mut TclObj,
        target: OriginalCLinkTarget,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias local");
        if let Err(error) = target.check(self) {
            return self.report_cmd_error_using(error, stage);
        }
        let actual = self.clone();
        let current = || {
            actual.check_original_c_name_stage(stage)?;
            target.check(&actual)
        };
        let reached = if stage.is_some() {
            self.original_c_name_bytes_using(original, Some(&current))
        } else {
            self.native_string_bytes(&original)
                .map_err(tcl_cmd_core::CmdError::from)
        };
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if let Err(error) = target.check(self) {
            return self.report_cmd_error_using(error, stage);
        }
        let bytes = match reached {
            Ok(bytes) => bytes,
            Err(error) => return self.report_cmd_error_using(error, stage),
        };
        if self.upvar_would_invert(&target, &bytes) {
            let mut message = b"bad variable name \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(protocol.alias_namespace_inversion_reason());
            return self.original_c_alias_error_using(&message, b"TCL UPVAR INVERTED", stage);
        }
        if protocol.alias_local_is_element(&bytes) {
            let mut message = b"bad variable name \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
            message.extend_from_slice(
                b"\": can't create a scalar variable that looks like an array element",
            );
            return self.original_c_alias_error_using(&message, b"TCL UPVAR LOCAL_ELEMENT", stage);
        }
        let selected = crate::vars::prepare_original_c_alias_local(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            &bytes,
            original,
            protocol,
        );
        let selected = match selected {
            Ok(selected) => selected,
            Err(crate::frame::VarError::NoSuchNamespace) => {
                let name = tcl_core_types::c_string_extent(&bytes);
                let mut message = b"can't create \"".to_vec();
                message.extend_from_slice(name);
                message.extend_from_slice(b"\": parent namespace doesn't exist");
                let code = super::error_code_list(&[b"TCL", b"LOOKUP", b"VARNAME", name]);
                return self.original_c_alias_diagnostic_using(
                    &message,
                    tcl_cmd_core::CmdErrorCodeUpdate::Set(code),
                    stage,
                );
            }
            Err(error) => {
                return self.original_c_lookup_var_error_using(
                    original,
                    NativeVariableNameLookupPurpose::Link,
                    error,
                    stage,
                );
            }
        };
        self.settle_original_c_alias_at_using(
            selected.home,
            selected.compiled,
            &selected.name,
            target,
            stage,
        )
    }

    pub(crate) fn link_original_c_upvar_objects(
        &mut self,
        original: *mut TclObj,
        level: usize,
        local: *mut TclObj,
    ) -> Code {
        let mut target = match self.prepare_selected_c_link_target(original, level) {
            Ok(Some(target)) => target,
            Ok(None) => {
                #[cfg(test)]
                crate::env_trace::remember_upvar_failure("missing-C-target", Code::Error, None);
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original upvar target cell").into(),
                );
            }
            Err(code) => {
                #[cfg(test)]
                crate::env_trace::remember_upvar_failure("select-C-target", code, None);
                return code;
            }
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            #[cfg(test)]
            crate::env_trace::remember_upvar_failure("prepare-C-target", Code::Error, Some(error));
            return self.original_c_lookup_var_error(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
            );
        }
        let code = self.bind_selected_c_alias_local(local, target);
        #[cfg(test)]
        crate::env_trace::remember_upvar_failure("bind-C-local", code, None);
        code
    }

    #[cfg(test)]
    pub(crate) fn prepare_original_c_link_target(
        &mut self,
        original: *mut TclObj,
        level: usize,
    ) -> Result<Option<crate::frame::Link>, Code> {
        self.prepare_original_c_link_target_using(original, level, true)
            .map(|selected| selected.map(|(target, _)| target))
    }
    pub(crate) fn prepare_selected_c_link_target(
        &mut self,
        original: *mut TclObj,
        level: usize,
    ) -> Result<Option<OriginalCLinkTarget>, Code> {
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let Some((mut target, source)) =
            self.prepare_original_c_link_target_using(original, level, false)?
        else {
            return Ok(None);
        };
        let reached = crate::vars::prepare_selected_upvar_target(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            &mut target,
        );
        source
            .check(self, operation.currency())
            .map_err(|error| self.report_cmd_error(error))?;
        reached.map_err(|error| {
            self.original_c_lookup_var_error(original, NativeVariableNameLookupPurpose::Link, error)
        })?;
        let target_receipt = {
            let frames = self.frames.borrow();
            let namespaces = self.namespaces();
            crate::vars::OriginalAliasTargetObservation::capture(&frames, &namespaces, &target)
        };
        let observed = target_receipt.map_err(|error| {
            self.original_c_lookup_var_error(original, NativeVariableNameLookupPurpose::Link, error)
        })?;
        let target = OriginalCLinkTarget {
            protocol: self
                .native_c_variable_name_protocol()
                .expect("selected C alias recipe"),
            link: target,
            source: Some(source),
            target: observed,
            operation,
        };
        target
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        Ok(Some(target))
    }
    fn prepare_original_c_link_target_using(
        &mut self,
        original: *mut TclObj,
        level: usize,
        acquire: bool,
    ) -> Result<Option<(crate::frame::Link, crate::vars::NativeOriginalNameCell)>, Code> {
        if self.native_c_variable_name_protocol().is_none() {
            return Ok(None);
        }
        let saved = self.frames.borrow_mut().set_active_level(level);
        let saved_namespace = self
            .current_ns
            .replace(self.frames.borrow().frame_ns(level));
        let result = (|| {
            let Some(selection) =
                self.prepare_original_c_name_for(original, NativeVariableNameLookupPurpose::Link)?
            else {
                return Ok(None);
            };
            if let Some(target) = self.capture_original_c_materialised_link(&selection)? {
                return Ok(Some((target, selection.simple)));
            }
            selection
                .simple
                .check(self, &selection.currency)
                .map_err(|error| self.report_cmd_error(error))?;
            let select = if acquire {
                crate::vars::original_c_selected_link_target
            } else {
                crate::vars::selected_c_link_target
            };
            let target = select(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                &selection.simple,
                selection.element.clone(),
            );
            selection
                .simple
                .check(self, &selection.currency)
                .map_err(|error| self.report_cmd_error(error))?;
            target
                .map(|target| Some((target, selection.simple)))
                .map_err(|error| {
                    self.original_c_lookup_var_error(
                        original,
                        NativeVariableNameLookupPurpose::Link,
                        error,
                    )
                })
        })();
        self.current_ns.set(saved_namespace);
        self.frames.borrow_mut().set_active_level(saved);
        result
    }

    pub(super) fn original_c_variable_reporting_input(
        &mut self,
        fallback: (&[u8], Option<&[u8]>),
        originals: (Option<*mut TclObj>, Option<*mut TclObj>, bool),
    ) -> Result<OriginalCVariableReportingInput, Code> {
        self.original_c_variable_reporting_input_using(fallback, originals, None)
    }

    pub(super) fn original_c_variable_reporting_input_using(
        &mut self,
        fallback: (&[u8], Option<&[u8]>),
        originals: (Option<*mut TclObj>, Option<*mut TclObj>, bool),
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCVariableReportingInput, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        if self.native_c_variable_name_protocol().is_none() {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C variable reporting input",
                ),
            ));
        }
        let (original_root, original_element, combined) = originals;
        if combined {
            let original = original_root.ok_or_else(|| {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "combined original variable reporting operand",
                    ),
                )
            })?;
            if original_element.is_some() {
                return Err(self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "combined variable reporting geometry",
                    ),
                ));
            }
            return self
                .original_c_name_bytes_using(original, stage)
                .map(|bytes| OriginalCVariableReportingInput::Combined(bytes.to_vec()))
                .map_err(|error| self.report_cmd_error_using(error, stage));
        }
        let root = original_root
            .map_or_else(
                || Ok(fallback.0.to_vec()),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| bytes.to_vec())
                },
            )
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let element = original_element
            .map_or_else(
                || Ok(fallback.1.map(<[u8]>::to_vec)),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| Some(bytes.to_vec()))
                },
            )
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        Ok(OriginalCVariableReportingInput::Separate { root, element })
    }

    fn original_c_variable_failure_for_object(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        self.original_c_variable_failure_for_object_using(original, purpose, reason, site, None)
    }

    fn original_c_variable_failure_for_object_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        let bytes = match self.original_c_name_bytes_using(original, stage) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error_using(error, stage),
        };
        self.original_c_variable_failure_using(&bytes, purpose, reason, site, stage)
    }

    fn original_c_variable_failure_using(
        &mut self,
        original: &[u8],
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        self.original_c_variable_failure_input_using(
            tcl_syntax::naming::NativeVariableInputForm::Combined(original),
            purpose,
            reason,
            site,
            stage,
        )
    }

    pub(super) fn original_c_variable_failure_input(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        self.original_c_variable_failure_input_using(input, purpose, reason, site, None)
    }

    pub(super) fn original_c_variable_failure_input_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        use tcl_syntax::naming::{
            report_native_variable_diagnostic_at, NativeVariableDiagnosticOperation as Operation,
        };
        let operation = match purpose {
            NativeVariableNameLookupPurpose::Read
            | NativeVariableNameLookupPurpose::Exists
            | NativeVariableNameLookupPurpose::Array
            | NativeVariableNameLookupPurpose::IncrementRead
            | NativeVariableNameLookupPurpose::IncrementExisting => Operation::Read,
            NativeVariableNameLookupPurpose::Unset
            | NativeVariableNameLookupPurpose::QuietUnset => Operation::Unset,
            NativeVariableNameLookupPurpose::Write
            | NativeVariableNameLookupPurpose::QuietWrite
            | NativeVariableNameLookupPurpose::Link
            | NativeVariableNameLookupPurpose::Refer
            | NativeVariableNameLookupPurpose::ArrayMake => Operation::Write,
            NativeVariableNameLookupPurpose::Define => Operation::Define,
        };
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C original lookup");
        let diagnostic = match report_native_variable_diagnostic_at(
            tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
            operation,
            reason,
            site,
            input,
        ) {
            Ok(diagnostic) => diagnostic,
            Err(_) => {
                return self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("original variable lookup diagnostic")
                        .into(),
                    stage,
                );
            }
        };
        let mut message = format!("can't {} \"", purpose.diagnostic_verb()).into_bytes();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        if protocol.diagnostic_resets_interpreter_result() {
            if let Err(error) = self.reset_native_global_error_episode_using(stage) {
                return self.report_cmd_error_using(error, stage);
            }
        }
        self.report_cmd_error_using(
            tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: diagnostic.error_code.map_or(
                    tcl_cmd_core::CmdErrorCodeUpdate::Default,
                    |words| {
                        let words = words.iter().map(Vec::as_slice).collect::<Vec<_>>();
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(super::error_code_list(&words))
                    },
                ),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            }),
            stage,
        )
    }

    pub(super) fn capture_original_c_variable_report(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.capture_original_c_variable_report_using(original, purpose, None)
    }

    pub(super) fn capture_original_c_variable_report_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let selected =
            self.prepare_original_c_name_using(original, purpose.creates_entries(), stage)?;
        Ok(self
            .capture_original_c_selection(original, &selected, purpose)?
            .map(|(receiver, home)| OriginalCVariableCapture {
                receiver,
                home,
                root: selected.root,
                element: selected.element,
            }))
    }

    /// Generic original-object lookup retains its actual selected name purpose.
    /// It does not infer or admit an indexed/compiled caller.
    pub(super) fn capture_original_c_variable_for(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<OriginalCVariableCapture, Code> {
        let reached = self.prepare_original_c_name_for(original, purpose);
        self.check_original_c_name_operation(&[original])
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let selected = match reached? {
            Some(selected) => selected,
            None => {
                return Err(self.original_c_variable_failure_for_object(
                    original,
                    purpose,
                    tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                ));
            }
        };
        let captured = self.capture_original_c_selection(original, &selected, purpose);
        self.check_original_c_name_operation(&[original])
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let Some((receiver, home)) = captured? else {
            return Err(self.original_c_variable_failure_for_object(
                original,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        };
        Ok(OriginalCVariableCapture {
            receiver,
            home,
            root: selected.root,
            element: selected.element,
        })
    }

    /// Capture the two original operands of a selected array stack opcode.
    /// C85+ entry creation retains the same original index object.
    pub(super) fn capture_original_c_parts_report(
        &mut self,
        root: *mut TclObj,
        element: Option<*mut TclObj>,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.capture_original_c_parts_report_using(root, element, purpose, None)
    }

    pub(super) fn capture_original_c_parts_report_using(
        &mut self,
        root: *mut TclObj,
        element: Option<*mut TclObj>,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        // The root lookup cannot publish a root-only error before the original
        // element operand reaches the caller's separate-input presenter.
        let Some(mut selected) = self.prepare_original_c_name_for_using(root, purpose, stage)?
        else {
            return Ok(None);
        };
        if selected.element.is_some() {
            return Err(self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("compiled separate base-name geometry")
                    .into(),
                stage,
            ));
        }
        selected.element = element
            .map(|element| {
                self.original_c_name_bytes_using(element, stage)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let captured = self
            .capture_original_c_parts_selection_using(root, element, &selected, purpose, stage)?;
        if purpose.creates_entries()
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.element_table_retains_original())
        {
            if let Some((receiver, _)) = &captured {
                receiver.retain_original_element_key(element);
            }
        }
        Ok(captured.map(|(receiver, home)| OriginalCVariableCapture {
            receiver,
            home,
            root: selected.root,
            element: selected.element,
        }))
    }

    fn original_c_parsed_element_key(&self, original: *mut TclObj) -> Option<Owned> {
        obj::native_variable_name::with_parsed(original, |cache| {
            let (_, element) = cache.array.as_ref()?;
            match cache.protocol.parsed_element_table_key(element)? {
                tcl_syntax::native_variable_name::NativeElementTableKey::FreshString(bytes) => {
                    Some(Owned::fresh(obj::new_string_bytes(bytes)))
                }
                tcl_syntax::native_variable_name::NativeElementTableKey::Original(original) => {
                    Some(original.clone())
                }
            }
        })
        .flatten()
    }

    fn capture_original_c_selection(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.capture_original_c_selection_using(original, selected, purpose, None)
    }

    fn capture_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let reached = self.capture_original_c_selection_receiver(original, selected, purpose);
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        reached.map_err(|error| {
            if error == crate::frame::VarError::DeletedNamespace
                && selected.element.is_some()
                && matches!(
                    purpose,
                    NativeVariableNameLookupPurpose::Write
                        | NativeVariableNameLookupPurpose::QuietWrite
                )
            {
                // TclLookupArrayElement fails before the value write. This
                // original-object entry still supplies the root to its error
                // tuple even when that root selected a compiled local cell.
                return self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    tcl_syntax::naming::NativeVariableDiagnosticReason::RetiredNamespace,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                    stage,
                );
            }
            self.original_c_lookup_var_error_using(original, purpose, error, stage)
        })
    }

    fn capture_original_c_parts_selection(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.capture_original_c_parts_selection_using(original, element, selected, purpose, None)
    }

    fn capture_original_c_parts_selection_using(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let captured = self.capture_original_c_selection_receiver(original, selected, purpose);
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        captured.map_err(|error| {
            if !purpose.leaves_error_message() || element.is_none() {
                return self.original_c_lookup_var_error_using(original, purpose, error, stage);
            }
            let input = match self.original_c_variable_reporting_input_using(
                (&selected.root, selected.element.as_deref()),
                (Some(original), element, false),
                stage,
            ) {
                Ok(input) => input,
                Err(code) => return code,
            };
            self.original_c_variable_receiver_error_using(
                input.input(),
                purpose,
                error,
                Some(tcl_syntax::naming::NativeVariableFailureSite::NameLookup),
                stage,
            )
        })
    }

    fn prepare_selected_c_namespace_target(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
    ) -> Result<OriginalCLinkTarget, Code> {
        self.prepare_selected_c_namespace_target_using(original, selected, None)
    }

    fn prepare_selected_c_namespace_target_using(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<OriginalCLinkTarget, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let mut target =
            if let Some(target) = self.capture_original_c_materialised_link(selected)? {
                target
            } else {
                let reached = crate::vars::selected_c_link_target(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &selected.simple,
                    selected.element.clone(),
                );
                selected
                    .simple
                    .check(self, &selected.currency)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                reached.map_err(|error| {
                    self.original_c_lookup_var_error_using(
                        original,
                        NativeVariableNameLookupPurpose::Link,
                        error,
                        stage,
                    )
                })?
            };
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let reached = crate::vars::prepare_selected_upvar_target(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            &mut target,
        );
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        reached.map_err(|error| {
            self.original_c_lookup_var_error_using(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
                stage,
            )
        })?;
        let observed = crate::vars::OriginalAliasTargetObservation::capture(
            &self.frames.borrow(),
            &self.namespaces(),
            &target,
        );
        let observed = observed.map_err(|error| {
            self.original_c_lookup_var_error_using(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
                stage,
            )
        })?;
        let target = OriginalCLinkTarget {
            protocol: self
                .native_c_variable_name_protocol()
                .expect("selected C alias recipe"),
            link: target,
            source: Some(selected.simple.clone()),
            target: observed,
            operation,
        };
        target
            .check(self)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        Ok(target)
    }

    fn capture_original_c_materialised_link(
        &mut self,
        selected: &OriginalCNameSelection,
    ) -> Result<Option<crate::frame::Link>, Code> {
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| self.report_cmd_error(error))?;
        let Some(original_target) = &selected.materialised else {
            return Ok(None);
        };
        let operation = super::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let target = original_target.capture_link_target(self, operation.currency());
        operation
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        target
            .map(Some)
            .map_err(|error| self.report_cmd_error(error))
    }

    fn capture_original_c_selection_receiver(
        &mut self,
        original: *mut TclObj,
        selected: &OriginalCNameSelection,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<
        Option<(crate::frame::VariableReceiver, crate::vars::TraceHome)>,
        crate::frame::VarError,
    > {
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| {
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
        if let Some(original_target) = &selected.materialised {
            let operation = super::native_operation_currency::NativeOperationScope::enter(self)
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    crate::frame::VarError::NameProtocolUnavailable
                })?;
            let reached = original_target.capture_receiver(self, operation.currency());
            operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    crate::frame::VarError::NameProtocolUnavailable
                })?;
            let reached = reached.map_err(|error| {
                // The captured owner has already retained the full first cause;
                // VarError is only the existing outer diagnostic unwind port.
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
            self.refuse_confined_receiver(&reached.1, purpose)?;
            return Ok(Some(reached));
        }
        // Asked by name before the capture, which may create the entry.
        if Self::changes_a_variable(purpose) && self.store_escapes(&selected.root) {
            return Err(crate::frame::VarError::Confined);
        }
        let captured = crate::vars::capture_original_c_selected_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            &selected.simple,
            selected.element.clone(),
            purpose.creates_entries(),
        );
        selected
            .simple
            .check(self, &selected.currency)
            .map_err(|error| {
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            })?;
        let captured = captured?
            .map(|(receiver, home)| {
                let home = home
                    .for_selected_receiver(&receiver)
                    .ok_or(crate::frame::VarError::NameProtocolUnavailable)?;
                Ok((receiver, home))
            })
            .transpose()?;
        if purpose.creates_entries()
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        {
            if let (Some((receiver, _)), Some(_)) = (&captured, &selected.element) {
                let retained_key = self.original_c_parsed_element_key(original);
                receiver.retain_original_element_key(retained_key.as_ref().map(Owned::as_ptr));
            }
        }
        if let Some((_, home)) = &captured {
            self.refuse_confined_receiver(home, purpose)?;
        }
        Ok(captured)
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

    fn original_c_variable_error_without_operand_using(
        &mut self,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        stage: OriginalCNameStage<'_>,
    ) -> Option<Code> {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return Some(self.report_cmd_error_using(error, stage));
        }
        if !purpose.leaves_error_message()
            && error != crate::frame::VarError::NameProtocolUnavailable
        {
            #[cfg(test)]
            if purpose == NativeVariableNameLookupPurpose::Link {
                crate::env_trace::remember_original_c_link_error(error);
            }
            self.traces.borrow_mut().pending_err.take();
            return Some(Code::Error);
        }
        (error == crate::frame::VarError::NameProtocolUnavailable).then(|| {
            self.report_cmd_error_using(
                ValueError::CommandProtocolUnavailable("actual indexed variable receiver").into(),
                stage,
            )
        })
    }

    fn original_c_lookup_var_error(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
    ) -> Code {
        self.original_c_lookup_var_error_using(original, purpose, error, None)
    }

    fn original_c_lookup_var_error_using(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        if let Some(code) =
            self.original_c_variable_error_without_operand_using(purpose, error, stage)
        {
            return code;
        }
        // Retire the reached callback failure before materialising its name,
        // just as the original-object access did before it selected a presenter.
        let trace_reason = (error == crate::frame::VarError::TraceError).then(|| {
            self.traces
                .borrow_mut()
                .pending_err
                .take()
                .unwrap_or_default()
        });
        let bytes = match self.original_c_name_bytes_using(original, stage) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error_using(error, stage),
        };
        let input = tcl_syntax::naming::NativeVariableInputForm::Combined(&bytes);
        if let Some(reason) = trace_reason {
            return self.original_c_variable_trace_error_using(input, purpose, &reason, stage);
        }
        self.original_c_variable_receiver_error_using(input, purpose, error, None, stage)
    }

    fn original_c_variable_trace_error_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: &[u8],
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original C variable trace report",
                ),
            );
        };
        let name =
            match tcl_syntax::naming::report_native_c_variable_value_name(
                tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                input,
            ) {
                Ok(name) => name,
                Err(_) => return self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original C variable trace report name",
                    ),
                ),
            };
        self.var_trace_error_using(
            &name,
            if matches!(
                purpose,
                NativeVariableNameLookupPurpose::Read
                    | NativeVariableNameLookupPurpose::IncrementRead
                    | NativeVariableNameLookupPurpose::IncrementExisting
            ) {
                b"read"
            } else {
                b"write"
            },
            reason,
            stage,
        )
    }

    pub(super) fn original_c_increment_constant_error(&mut self, original: &[u8]) -> Code {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original increment constant presenter")
                    .into(),
            );
        };
        let diagnostic = match tcl_syntax::naming::report_native_variable_diagnostic_at(
            tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
            Operation::Increment,
            Reason::Constant,
            Site::ValueWrite,
            tcl_syntax::naming::NativeVariableInputForm::Combined(original),
        ) {
            Ok(diagnostic) => diagnostic,
            Err(_) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("selected increment constant failure")
                        .into(),
                );
            }
        };
        let mut message = b"can't incr \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
            tcl_cmd_core::CmdErrorDetails {
                message,
                string_result: protocol.diagnostic_string_protocol(),
                error_code: diagnostic.error_code.map_or(
                    tcl_cmd_core::CmdErrorCodeUpdate::Default,
                    |words| {
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(super::error_code_list(
                            &words.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                        ))
                    },
                ),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            },
        ))
    }

    pub(super) fn original_c_variable_receiver_error(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        reached_site: Option<tcl_syntax::naming::NativeVariableFailureSite>,
    ) -> Code {
        self.original_c_variable_receiver_error_using(input, purpose, error, reached_site, None)
    }

    pub(super) fn original_c_variable_receiver_error_using(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
        reached_site: Option<tcl_syntax::naming::NativeVariableFailureSite>,
        stage: OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(error) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error_using(error, stage);
        }
        use crate::frame::VarError;
        use tcl_syntax::naming::{
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        if let Some(code) =
            self.original_c_variable_error_without_operand_using(purpose, error, stage)
        {
            return code;
        }
        let reason = match error {
            VarError::IsScalar => Reason::NotArray,
            VarError::IsArray => Reason::IsArray,
            VarError::NoSuchNamespace => Reason::MissingParentNamespace,
            VarError::DeletedArray => Reason::DetachedElement,
            VarError::DeletedNamespace => Reason::RetiredNamespace,
            VarError::IsConstant => Reason::Constant,
            VarError::Confined => {
                let Some(protocol) = self.native_c_variable_name_protocol() else {
                    return self.report_cmd_error_using(
                        ValueError::CommandProtocolUnavailable(
                            "original C confined variable report",
                        )
                        .into(),
                        stage,
                    );
                };
                let name = match tcl_syntax::naming::report_native_c_variable_value_name(
                    tcl_syntax::naming::NativeNameProtocol::C(protocol.version()),
                    input,
                ) {
                    Ok(name) => name,
                    Err(_) => {
                        return self.report_cmd_error_using(
                            ValueError::CommandProtocolUnavailable(
                                "original C confined variable report name",
                            )
                            .into(),
                            stage,
                        );
                    }
                };
                return if matches!(
                    purpose,
                    NativeVariableNameLookupPurpose::Unset
                        | NativeVariableNameLookupPurpose::QuietUnset
                ) {
                    self.confined_unset_error_using(&name, stage)
                } else {
                    self.confined_store_error_using(&name, stage)
                };
            }

            VarError::NameProtocolUnavailable => {
                unreachable!("operand-free refusal returned above")
            }
            VarError::TraceError => {
                let reason = self
                    .traces
                    .borrow_mut()
                    .pending_err
                    .take()
                    .unwrap_or_default();
                return self.original_c_variable_trace_error_using(input, purpose, &reason, stage);
            }
        };
        let site = reached_site.unwrap_or_else(|| match purpose {
            NativeVariableNameLookupPurpose::Read if reason == Reason::IsArray => Site::ValueRead,
            NativeVariableNameLookupPurpose::Write
            | NativeVariableNameLookupPurpose::QuietWrite
                if matches!(
                    reason,
                    Reason::IsArray
                        | Reason::Constant
                        | Reason::DetachedElement
                        | Reason::RetiredNamespace
                ) =>
            {
                Site::ValueWrite
            }
            NativeVariableNameLookupPurpose::Unset
            | NativeVariableNameLookupPurpose::QuietUnset
                if reason == Reason::Constant =>
            {
                Site::ValueUnset
            }
            _ => Site::NameLookup,
        });
        self.original_c_variable_failure_input_using(input, purpose, reason, site, stage)
    }

    pub(super) fn original_variable_trace_requires_name(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
        operation: &[u8],
    ) -> bool {
        // Geometry comes from the selected receiver; the placeholder spelling
        // is not read or exposed by this trace-selection query.
        let access = self.trace_access(&home.base, &home.base, element, home, false);
        let cell = self.variable_trace_scope(home, access.match_elem.as_deref());
        let active = self.active_var_trace_scopes.borrow();
        // Unset callbacks run on the destroyed cell even when its read or
        // write callback is active. The containing array keeps its own guard.
        let destroyed = tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::from_bytes(operation)
            .is_some_and(tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::uses_destroyed_cell_callbacks);
        if !destroyed && active.contains(&cell) {
            return false;
        }
        let array_active = access.match_elem.is_some() && active.contains(&cell.array());
        let traces = self.traces.borrow();
        let registered = traces.traces.iter().any(|trace| {
            (trace.elem.is_some() || (access.whole_array && !array_active))
                && cell.matches(trace, operation)
        });
        let intrinsic = access.whole_array
            && !array_active
            && ((access.match_elem.is_none()
                && matches!(operation, b"read" | b"unset")
                && self.native_error_variable_at(home).is_some())
                || (self.native_precision_trace_at(home)
                    && matches!(operation, b"read" | b"write" | b"unset"))
                || (self.env_trace_reaches(home)
                    && matches!(operation, b"read" | b"write" | b"unset" | b"array")));
        registered || intrinsic
    }

    pub(super) fn read_original_c_variable(
        &mut self,
        original: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported(original, original, false)
    }

    pub(crate) fn read_original_c_property_variable(
        &mut self,
        lookup: *mut TclObj,
        reported: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported(lookup, reported, true)
    }

    fn read_original_c_variable_reported(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        create: bool,
    ) -> Result<*mut TclObj, Code> {
        self.read_original_c_variable_reported_using(lookup, original, create, None)
    }

    fn read_original_c_variable_reported_using(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        create: bool,
        stage: OriginalCNameStage<'_>,
    ) -> Result<*mut TclObj, Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let purpose = NativeVariableNameLookupPurpose::Read;
        let selected = self.prepare_original_c_name_using(lookup, create, stage)?;
        let capture_purpose = if create {
            NativeVariableNameLookupPurpose::Link
        } else {
            purpose
        };
        let captured =
            self.capture_original_c_selection_using(lookup, &selected, capture_purpose, stage)?;
        let Some((receiver, home)) = captured else {
            let bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            return Err(self.original_c_variable_failure_using(
                &bytes,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                stage,
            ));
        };
        if self.original_variable_trace_requires_name(&home, selected.element.as_deref(), b"read") {
            let bytes = self
                .original_c_name_bytes_using(original, stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?
                .to_vec();
            let access = self.trace_access(
                &bytes,
                &selected.root,
                selected.element.as_deref(),
                &home,
                false,
            );
            let traced = self
                .fire_var_trace_resolved(&home, &access, b"read")
                .map_err(|first| self.refuse_native_execution(first))?;
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            if traced.is_guest_error() {
                return Err(self.original_c_lookup_var_error_using(
                    original,
                    purpose,
                    crate::frame::VarError::TraceError,
                    stage,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        receiver
            .read()
            .map_err(|error| {
                self.original_c_lookup_var_error_using(original, purpose, error, stage)
            })?
            .ok_or_else(|| {
                self.original_c_variable_failure_for_object_using(
                    original,
                    purpose,
                    if selected.element.is_some() && receiver.is_array() {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchElement
                    } else {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable
                    },
                    tcl_syntax::naming::NativeVariableFailureSite::ValueRead,
                    stage,
                )
            })
    }

    fn assign_original_c_selection(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<(), Code> {
        self.assign_original_c_selection_using(original, value, publish_result, purpose, None)
    }

    fn assign_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let arguments = OriginalArguments::enter(self, &[original, value])
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let result = (|| {
            let reached = self.prepare_original_c_name_for_using(original, purpose, stage);
            arguments
                .original(1)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            let selected = reached?.ok_or_else(|| {
                self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual original write selection")
                        .into(),
                    stage,
                )
            })?;
            self.assign_prepared_original_c_selection_using(
                original,
                value,
                publish_result,
                purpose,
                &selected,
                stage,
            )
        })();
        arguments
            .check()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        result
    }

    fn assign_prepared_original_c_selection(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
    ) -> Result<(), Code> {
        self.assign_prepared_original_c_selection_using(
            original,
            value,
            publish_result,
            purpose,
            selected,
            None,
        )
    }

    fn assign_prepared_original_c_selection_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.assign_prepared_original_c_selection_reported_using(
            OriginalVariableAssignmentObjects {
                lookup: original,
                original,
                value,
            },
            publish_result,
            purpose,
            selected,
            stage,
        )
    }

    fn assign_prepared_original_c_selection_reported(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
    ) -> Result<(), Code> {
        self.assign_prepared_original_c_selection_reported_using(
            OriginalVariableAssignmentObjects {
                lookup,
                original,
                value,
            },
            publish_result,
            purpose,
            selected,
            None,
        )
    }

    fn assign_prepared_original_c_selection_reported_using(
        &mut self,
        OriginalVariableAssignmentObjects {
            lookup,
            original,
            value,
        }: OriginalVariableAssignmentObjects,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let arguments = OriginalArguments::enter(self, &[lookup, original, value])
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        let result = (|| {
            let reached = self.capture_original_c_selection_using(lookup, selected, purpose, stage);
            arguments
                .original(2)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            let (receiver, home) = reached?.ok_or_else(|| {
                self.report_cmd_error_using(
                    ValueError::CommandProtocolUnavailable("actual variable store receiver").into(),
                    stage,
                )
            })?;
            let protective_value = publish_result.then(|| Owned::retain(value));
            let stored = receiver.store(value);
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            arguments
                .original(2)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            stored.map_err(|error| {
                self.original_c_lookup_var_error_using(original, purpose, error, stage)
            })?;
            if self.original_variable_trace_requires_name(
                &home,
                selected.element.as_deref(),
                b"write",
            ) {
                let bytes = match stage {
                    Some(_) => self.original_c_name_bytes_using(original, stage),
                    None => arguments.bytes(1),
                }
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
                let access = self.trace_access(
                    &bytes,
                    &selected.root,
                    selected.element.as_deref(),
                    &home,
                    false,
                );
                let traced = self
                    .fire_var_trace_resolved_with_errors(
                        &home,
                        &access,
                        b"write",
                        purpose.leaves_error_message(),
                    )
                    .map_err(|first| self.refuse_native_execution(first))?
                    .is_guest_error();
                arguments
                    .check()
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                self.check_original_c_name_stage(stage)
                    .map_err(|error| self.report_cmd_error_using(error, stage))?;
                if traced {
                    return Err(self.original_c_lookup_var_error_using(
                        original,
                        purpose,
                        crate::frame::VarError::TraceError,
                        stage,
                    ));
                }
            }
            if publish_result {
                match receiver.read().map_err(|error| {
                    self.original_c_lookup_var_error_using(original, purpose, error, stage)
                })? {
                    Some(value) => self.set_result(value),
                    None => self.set_result_bytes(b""),
                }
            }
            self.check_original_c_name_stage(stage)
                .map_err(|error| self.report_cmd_error_using(error, stage))?;
            drop(protective_value);
            Ok(())
        })();
        // Receiver and protective result roles retire before final settlement.
        arguments
            .check()
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        self.check_original_c_name_stage(stage)
            .map_err(|error| self.report_cmd_error_using(error, stage))?;
        result
    }

    pub(crate) fn store_original_c_property_variable(
        &mut self,
        lookup: *mut TclObj,
        reported: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let purpose = NativeVariableNameLookupPurpose::Write;
        let selected = self.prepare_original_c_name(lookup, true)?;
        self.assign_prepared_original_c_selection_reported(
            lookup, reported, value, false, purpose, &selected,
        )
    }

    pub(super) fn store_original_c_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        self.assign_original_c_selection(
            original,
            value,
            true,
            NativeVariableNameLookupPurpose::Write,
        )
    }
    pub(crate) fn assign_original_regex_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        let Some(protocol) = self.native_c_variable_name_protocol() else {
            return self.assign_original_named_variable(original, value);
        };
        match self.assign_original_c_selection(
            original,
            value,
            false,
            protocol.regex_write_purpose(),
        ) {
            Ok(()) => Ok(()),
            Err(code)
                if !protocol.regex_write_purpose().leaves_error_message()
                    && !self.host_refusal_pending() =>
            {
                let name = self
                    .native_string_bytes(&original)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let mut message = self.result_bytes();
                message.extend_from_slice(b"couldn't set variable \"");
                message.extend_from_slice(tcl_core_types::c_string_extent(&name));
                message.push(b'"');
                self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
                    tcl_cmd_core::CmdErrorDetails {
                        message,
                        string_result: protocol.diagnostic_string_protocol(),
                        error_code: tcl_cmd_core::CmdErrorCodeUpdate::Unchanged,
                        error_info: None,
                        error_line: None,
                        primitive_getter: None,
                    },
                ));
                Err(code)
            }
            Err(code) => Err(code),
        }
    }
    pub(super) fn assign_original_c_variable(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        self.assign_original_c_variable_using(original, value, None)
    }

    pub(crate) fn assign_original_c_variable_using(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        stage: OriginalCNameStage<'_>,
    ) -> Result<(), Code> {
        self.assign_original_c_selection_using(
            original,
            value,
            false,
            NativeVariableNameLookupPurpose::Write,
            stage,
        )
    }
}

#[cfg(test)]
#[path = "native_variable_names/original_handoff_tests.rs"]
mod original_handoff_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_assignment_tests;
