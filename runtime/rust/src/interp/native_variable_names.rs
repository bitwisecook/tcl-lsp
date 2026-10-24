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
            check().map_err(|error| {
                if let Some(first) = error.native_execution_refusal() {
                    self.clone().refuse_native_execution(first.clone());
                }
                error
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
            selected.check(self, currency).and_then(|()| {
                self.check_original_c_name_operation_using(&[original], stage)
                    .map_err(Into::into)
            })
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
                    Err(error) => self.report_cmd_error_using(error.into(), stage),
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
            .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
                                .map_err(Into::into)
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
                    .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
                result.map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage))?
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
                                .map_err(Into::into)
                        })
                };
                let result = obj::discard_native_internal_representation_with_stage_check(
                    root,
                    &mut current,
                );
                self.check_original_c_name_operation_using(&[original, root], stage)
                    .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
                result.map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
                    .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
            .map_err(|error| self.report_cmd_error_using(error.into(), stage))?
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage));
        }
        let root = original_root
            .map_or_else(
                || Ok(fallback.0.to_vec()),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| bytes.to_vec())
                },
            )
            .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
        let element = original_element
            .map_or_else(
                || Ok(fallback.1.map(<[u8]>::to_vec)),
                |original| {
                    self.original_c_name_bytes_using(original, stage)
                        .map(|bytes| Some(bytes.to_vec()))
                },
            )
            .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
            Err(error) => return self.report_cmd_error_using(error.into(), stage),
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
            .map_err(|error| self.report_cmd_error_using(error.into(), stage))?;
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
            return reached.map(Some).map_err(|error| {
                // The captured owner has already retained the full first cause;
                // VarError is only the existing outer diagnostic unwind port.
                self.report_cmd_error(error);
                crate::frame::VarError::NameProtocolUnavailable
            });
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
        Ok(captured)
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
            Err(error) => return self.report_cmd_error_using(error.into(), stage),
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage))?
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
                .map_err(|error| self.report_cmd_error_using(error.into(), stage))?
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
            original,
            original,
            value,
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
            lookup,
            original,
            value,
            publish_result,
            purpose,
            selected,
            None,
        )
    }

    fn assign_prepared_original_c_selection_reported_using(
        &mut self,
        lookup: *mut TclObj,
        original: *mut TclObj,
        value: *mut TclObj,
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
mod tests {
    use super::*;

    thread_local! {
        static NAME_FREE_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
        static NAME_FREE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static NAME_FREE_BORROWS_RELEASED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
        static NAME_FREE_CONTEXT_ACCEPTED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
        static NAME_CALLBACK_BOUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static NAME_CALLBACK_TARGET_BORN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static NAME_CALLBACK_CODE: std::cell::Cell<Code> = const { std::cell::Cell::new(Code::Other(37)) };
    }

    struct CurrentNameCallbackInterp(*mut Interp);
    impl CurrentNameCallbackInterp {
        fn enter(interp: &mut Interp) -> Self {
            let previous = crate::codegen_abi::current_interp();
            crate::codegen_abi::tcl_runtime_set_current_interp(interp);
            Self(previous)
        }
    }
    impl Drop for CurrentNameCallbackInterp {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
        }
    }

    extern "C" fn name_free_callback(_original: *mut TclObj) {
        let call = NAME_FREE_CALLS.with(|calls| {
            let call = calls.get() + 1;
            calls.set(call);
            call
        });
        let Some(mut interp) = (unsafe { crate::codegen_abi::current_interp().as_ref() }).cloned()
        else {
            NAME_FREE_CONTEXT_ACCEPTED.with(|accepted| accepted.set(false));
            return;
        };
        let released =
            interp.frames.try_borrow_mut().is_ok() && interp.namespaces.try_borrow_mut().is_ok();
        NAME_FREE_BORROWS_RELEASED.with(|previous| previous.set(previous.get() && released));
        match NAME_FREE_MODE.with(std::cell::Cell::get) {
            1 => {
                interp.refuse_host_command("first original variable-name free callback");
            }
            2 => {
                let original = interp.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-name-callback".to_owned(), "1.0".to_owned())];
                let accepted =
                    interp.pin_context(&changed).is_ok() && interp.pin_context(&original).is_ok();
                NAME_FREE_CONTEXT_ACCEPTED.with(|value| value.set(value.get() && accepted));
            }
            3 if call == 2 => {
                interp.refuse_host_command("first canonical variable-name free callback");
            }
            _ => {}
        }
    }
    static NAME_FREE_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalVariableNameFree".as_ptr(),
        free_int_rep_proc: Some(name_free_callback),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    fn callback_name(bytes: &[u8]) -> Owned {
        let original = Owned::fresh(obj::alloc_typed(&NAME_FREE_TYPE, 0));
        // SAFETY: the fresh original header owns this exact resident spelling.
        unsafe { obj::set_native_updater_string_rep(original.as_ptr(), bytes, false) };
        original
    }

    fn reset_name_callback(mode: u8) {
        NAME_FREE_MODE.with(|value| value.set(mode));
        NAME_FREE_CALLS.with(|value| value.set(0));
        NAME_FREE_BORROWS_RELEASED.with(|value| value.set(true));
        NAME_FREE_CONTEXT_ACCEPTED.with(|value| value.set(true));
        NAME_CALLBACK_BOUND.with(|value| value.set(false));
        NAME_CALLBACK_TARGET_BORN.with(|value| value.set(false));
        NAME_CALLBACK_CODE.with(|value| value.set(Code::Other(37)));
    }

    fn direct_name_callback_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let command = Owned::fresh(obj::new_string_bytes(b"upvar"));
        let level = Owned::fresh(obj::new_string_bytes(b"#0"));
        let local = Owned::fresh(obj::new_string_bytes(b"callback_local"));
        let code = interp.dispatch(&[command.as_ptr(), level.as_ptr(), argv[1], local.as_ptr()]);
        let target_born = interp
            .namespaces
            .borrow()
            .var_table(crate::namespace::GLOBAL)
            .trace_binding_identity(b"callback_target")
            .is_some();
        NAME_CALLBACK_TARGET_BORN.with(|value| value.set(target_born));
        let bound = interp
            .frames
            .borrow()
            .table(interp.current_level())
            .unwrap()
            .binding_id(b"callback_local")
            .is_some();
        NAME_CALLBACK_BOUND.with(|value| value.set(bound));
        NAME_CALLBACK_CODE.with(|value| value.set(code));
        code
    }

    fn compiled_name_callback_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let slot = {
            let frames = interp.frames.borrow();
            frames
                .native_local_name_table()
                .expect("actual retained procedure name table")
                .names
                .iter()
                .position(|name| {
                    name.as_ref()
                        .is_some_and(|name| obj::bytes_of(name.as_ptr()) == b"callback_local")
                })
                .expect("actual compiled local slot")
        };
        assert!(interp.frames.borrow().compiled_slot_var(slot).is_none());
        let level = Owned::fresh(obj::new_string_bytes(b"#0"));
        let (_, target) = match crate::cmd_eval::select_frame(
            interp,
            &[level.as_ptr()],
            tcl_registry::FrameEffectSpec::UPVAR,
        ) {
            Ok(selected) => selected,
            Err(code) => return code,
        };
        let code = interp.link_original_compiled_upvar(argv[1], target, slot);
        let bound = interp.frames.borrow().compiled_slot_var(slot).is_some();
        NAME_CALLBACK_BOUND.with(|value| value.set(bound));
        NAME_CALLBACK_CODE.with(|value| value.set(code));
        code
    }

    #[test]
    fn original_upvar_free_callback_checks_currency_before_cells_and_aliases() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual selected C worker; software callback currency, not native equivalence.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in 0..=2 {
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.register_builtin(b"callbackUpvar", direct_name_callback_probe);
                assert_eq!(
                    interp.eval_str(b"proc callback_run {other} {callbackUpvar $other}"),
                    Code::Ok
                );
                let original = callback_name(b"::callback_target");
                let command = Owned::fresh(obj::new_string_bytes(b"callback_run"));
                reset_name_callback(mode);
                let _installed = CurrentNameCallbackInterp::enter(&mut interp);
                let code = interp.dispatch(&[command.as_ptr(), original.as_ptr()]);
                assert_eq!(
                    NAME_FREE_CALLS.with(std::cell::Cell::get),
                    1,
                    "{profile}/{mode}"
                );
                assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
                assert!(NAME_FREE_CONTEXT_ACCEPTED.with(std::cell::Cell::get));
                assert_eq!(obj::bytes_of(original.as_ptr()), b"::callback_target");
                assert_eq!(interp.current_level(), 0);
                assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
                assert_eq!(interp.entered_native_operation_count(), 0);
                if mode == 0 {
                    assert_eq!(code, Code::Ok, "{profile}");
                    assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Ok);
                    assert!(NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                    assert!(!interp.host_refusal_pending());
                } else {
                    assert_eq!(code, Code::Error, "{profile}/{mode}");
                    assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Error);
                    assert!(!NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                    assert!(interp.native_execution_refusal().is_some());
                }
                // Frame pop can reclaim the undefined target after its local alias.
                let target_born = NAME_CALLBACK_TARGET_BORN.with(std::cell::Cell::get);
                assert_eq!(
                    target_born,
                    mode == 0 || profile.starts_with("tcl9"),
                    "C9 cell birth precedes the reached cache callback; C8 retirement precedes lookup"
                );
            }
        }
    }

    #[test]
    fn original_compiled_upvar_free_callback_refuses_later_slot_binding() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Genuine procedure artifact/slot and shared compiled worker, no codegen claim.
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in 0..=2 {
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.register_builtin(b"callbackCompiledUpvar", compiled_name_callback_probe);
                assert_eq!(interp.eval_str(b"proc callback_run {other} {set callback_local {}; unset callback_local; callbackCompiledUpvar $other}"), Code::Ok);
                let procedure = interp.proc_def(b"callback_run").unwrap();
                let artifact = interp
                    .prepare_original_c_body(
                        procedure.body.as_ptr(),
                        procedure.namespace(),
                        Some(&procedure),
                    )
                    .unwrap()
                    .expect("actual native procedure artifact");
                assert!(artifact
                    .compiled_local_layout()
                    .unwrap()
                    .names
                    .iter()
                    .any(|name| {
                        name.as_ref()
                            .is_some_and(|name| name.as_bytes() == b"callback_local")
                    }));
                let original = callback_name(b"::callback_target");
                let command = Owned::fresh(obj::new_string_bytes(b"callback_run"));
                reset_name_callback(mode);
                let _installed = CurrentNameCallbackInterp::enter(&mut interp);
                let code = interp.dispatch(&[command.as_ptr(), original.as_ptr()]);
                assert_eq!(
                    NAME_FREE_CALLS.with(std::cell::Cell::get),
                    1,
                    "{profile}/{mode}"
                );
                assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
                assert!(NAME_FREE_CONTEXT_ACCEPTED.with(std::cell::Cell::get));
                assert_eq!(interp.current_level(), 0);
                assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
                assert_eq!(interp.entered_native_operation_count(), 0);
                if mode == 0 {
                    assert_eq!(code, Code::Ok, "{profile}");
                    assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Ok);
                    assert!(NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                } else {
                    assert_eq!(code, Code::Error, "{profile}/{mode}");
                    assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Error);
                    assert!(!NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                    assert!(interp.native_execution_refusal().is_some());
                }
            }
        }
    }

    fn canonical_name_callback_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        let (slot, canonical) = {
            let frames = interp.frames.borrow();
            let table = frames.native_local_name_table().unwrap();
            let slot = table
                .names
                .iter()
                .position(|name| {
                    name.as_ref()
                        .is_some_and(|name| obj::bytes_of(name.as_ptr()) == b"value")
                })
                .unwrap();
            (slot, table.names[slot].as_ref().unwrap().as_ptr())
        };
        // The genuine ordinal/cell and inherited operation are captured before
        // either original primary can reach its free callback.
        let operation =
            super::super::native_operation_currency::NativeOperationScope::enter(interp)
                .expect("original canonical-name operation");
        let selected = crate::vars::prepare_original_c_indexed_name_cell(
            &mut interp.frames.borrow_mut(),
            &interp.namespaces.borrow(),
            slot,
        )
        .expect("actual selected canonical-name cell");
        obj::change_type(canonical, &NAME_FREE_TYPE, 0);
        let original = callback_name(b"value");
        reset_name_callback(3);
        let result = interp.install_original_c_name_cache_using(
            original.as_ptr(),
            &selected,
            operation.currency(),
            None,
        );
        assert!(result.is_err());
        assert!(obj::native_variable_name::with_local(original.as_ptr(), |_| ()).is_some());
        assert!(obj::obj_type_ptr(canonical).is_null());
        assert_eq!(obj::bytes_of(canonical), b"value");
        match result {
            Ok(()) => Code::Ok,
            Err(error) => interp.report_cmd_error(error.into()),
        }
    }

    #[test]
    fn original_canonical_name_free_callback_releases_borrows_and_stops_next_cache() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Reached original cache stays installed; the later canonical cache is refused.
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"callbackCanonical", canonical_name_callback_probe);
        assert_eq!(
            interp.eval_str(b"proc callback_run {value} {callbackCanonical}"),
            Code::Ok
        );
        let _installed = CurrentNameCallbackInterp::enter(&mut interp);
        assert_eq!(interp.eval_str(b"callback_run VALUE"), Code::Error);
        assert_eq!(NAME_FREE_CALLS.with(std::cell::Cell::get), 2);
        assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
        assert!(interp.native_execution_refusal().is_some());
        assert_eq!(interp.current_level(), 0);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }

    fn exists_cache_purpose_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        let version = interp.native_c_variable_name_protocol().unwrap().version();
        for name in [b"x".as_slice(), b"a(k)", b"x\0tail"] {
            let original = Owned::fresh(obj::new_string_bytes(name));
            let before = obj::obj_type_ptr(original.as_ptr());
            assert!(interp
                .exists_original_c_parts(original.as_ptr(), None)
                .unwrap());
            if version == tcl_dialect::TclVersion::V8_4 {
                assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
            } else {
                assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
            }
        }
        let member = Owned::fresh(obj::new_string_bytes(b"x"));
        let original = Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]));
        let before = obj::obj_type_ptr(original.as_ptr());
        assert!(interp
            .exists_original_c_parts(original.as_ptr(), None)
            .unwrap());
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
            assert!(interp.read_original_c_variable(original.as_ptr()).is_ok());
            assert_ne!(
                obj::obj_type_ptr(original.as_ptr()),
                before,
                "actual object read still selects its original name cache"
            );
        } else {
            assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
        }
        interp.set_result_bytes(b"");
        Code::Ok
    }

    #[test]
    fn legacy_existence_preserves_original_name_caches_and_object_reads_still_adopt_them() {
        // naming.variable.original-upvar-and-exists-completion-and-name-windows
        // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
        // Native case16 binds the fresh scalar primary. The list-cache,
        // combined name and NUL controls separately test this source/API owner.
        for engine in ["tcl8.4", "tcl8.5"] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"existsCachePurpose", exists_cache_purpose_probe);
            let code = interp
                .eval_str(b"proc run {} {set x LOCAL; set a(k) VALUE; existsCachePurpose}; run");
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending());
        }
    }

    thread_local! {
        static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn alias_simple_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        for (case, name) in [b"v".as_slice(), b"d", b"n\0(k)"].into_iter().enumerate() {
            let local = if case == 2 {
                Owned::fresh(obj::new_string_bytes(name))
            } else {
                let member = Owned::fresh(obj::new_string_bytes(name));
                Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]))
            };
            let kind = if case == 2 { "NULL" } else { "list" };
            // SAFETY: local retains the original object throughout this callback.
            let references = unsafe { (*local.as_ptr()).ref_count };
            ALIAS_SIMPLE_ROWS.with(|rows| {
                rows.borrow_mut()
                    .push(format!("before{case}|{kind}|{references}"))
            });
            let original_type = obj::obj_type_ptr(local.as_ptr());
            let target = Owned::fresh(obj::new_string_bytes(b"x"));
            let mut link = match interp.prepare_original_c_link_target(target.as_ptr(), 0) {
                Ok(Some(link)) => link,
                Ok(None) => panic!("actual C target unavailable"),
                Err(code) => return code,
            };
            interp.prepare_upvar_target(&mut link).unwrap();
            let code = interp.bind_original_c_alias_local(local.as_ptr(), link);
            if code != Code::Ok {
                return code;
            }
            assert_eq!(obj::obj_type_ptr(local.as_ptr()), original_type);
            // SAFETY: local retains the original object after alias installation.
            let references = unsafe { (*local.as_ptr()).ref_count };
            ALIAS_SIMPLE_ROWS.with(|rows| {
                rows.borrow_mut()
                    .push(format!("after{case}|0|{kind}|{references}"))
            });
        }
        interp.set_result_bytes(b"");
        Code::Ok
    }

    thread_local! {
        static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    fn global_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let original = argv[1];
        let value = match interp.read_original_named_variable(original) {
            Ok(value) => value,
            Err(code) => return code,
        };
        let kind = obj::obj_type_ptr(original);
        assert!(
            !kind.is_null(),
            "successful original-name lookup owns a cache"
        );
        // SAFETY: original owns its selected descriptor throughout this callback.
        let name = unsafe { std::ffi::CStr::from_ptr((*kind).name).to_str().unwrap() };
        let bytes = interp.native_string_bytes(&value).unwrap();
        GLOBAL_CACHE_ROWS.with(|rows| {
            rows.borrow_mut().push(format!(
                "{name}|{}",
                String::from_utf8(bytes.to_vec()).unwrap(),
            ))
        });
        interp.set_result_bytes(b"");
        Code::Ok
    }
    #[test]
    fn dynamic_global_uses_original_compiler_token_tail_and_name_cache() {
        // Native proof: naming.variable.original-global-cache-token
        // docs/design/analysis/name-resolution-proofs/variable-original-global-cache-token.md
        for (engine, expected) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_global_cache_token/8.4.20.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_global_cache_token/8.5.19.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_global_cache_token/8.6.18.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_global_cache_token/9.0.4.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_global_cache_token/9.1.0.txt"),
            ),
        ] {
            crate::counters::reset();
            GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().clear());
            {
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.register_builtin(b"probe", global_cache_probe);
                let original = Owned::fresh(obj::new_string_bytes(b"v"));
                interp.var_set(b"name", original.as_ptr()).unwrap();
                drop(original);
                for (label, source) in [
                    ("simple", b"set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p".as_slice()),
                    ("qualified", b"namespace eval N {variable v QUALIFIED};set name ::N::v;p".as_slice()),
                ] {
                    let code = interp.eval_str(source);
                    GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().push(format!(
                        "{label}|{}|{}", code.as_int(), String::from_utf8(interp.result_bytes()).unwrap(),
                    )));
                    assert!(!interp.host_refusal_pending(), "{engine}");
                }
                let rows = GLOBAL_CACHE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
                assert_eq!(rows, expected, "{engine}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    #[test]
    fn alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners() {
        // Native proof: naming.alias.original-local-name-object
        // docs/design/analysis/name-resolution-proofs/alias-original-local-name-object.md
        for (engine, expected) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_alias_simple/8.4.20.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_alias_simple/8.5.19.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_alias_simple/8.6.18.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_alias_simple/9.0.4.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_alias_simple/9.1.0.txt"),
            ),
        ] {
            crate::counters::reset();
            ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().clear());
            {
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.register_builtin(b"alias_probe", alias_simple_probe);
                let code = interp.eval_str(b"set x X; proc p {} {alias_probe; return $v}; p");
                assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert_eq!(interp.result_bytes(), b"X");
                ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().push("completion|0|X".into()));
                let rows = ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
                assert_eq!(rows, expected, "{engine}");
                assert!(!interp.host_refusal_pending());
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    struct NamespaceOutputHost {
        inner: Rc<dyn tcl_platform::Host>,
        output: std::cell::RefCell<Vec<u8>>,
    }
    impl tcl_platform::StdIo for NamespaceOutputHost {
        fn write_stdout(&self, bytes: &[u8]) {
            self.output.borrow_mut().extend_from_slice(bytes);
        }
        fn write_stderr(&self, bytes: &[u8]) {
            self.inner.stdio().write_stderr(bytes);
        }
    }
    impl tcl_platform::Host for NamespaceOutputHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.inner.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.inner.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.inner.env()
        }
        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            self.inner.numeric_environment()
        }
        fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
            self.inner.native_integer_formatter()
        }
        fn system_encoding(&self) -> tcl_platform::SystemEncoding {
            self.inner.system_encoding()
        }
        fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
            self.inner.filesystem()
        }
        fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
            self.inner.sockets()
        }
        fn process(&self) -> Option<&dyn tcl_platform::Process> {
            self.inner.process()
        }
    }

    fn trace_source_observation(engine: &str, source: &[u8], expected: &[u8]) {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            let actual = host.output.borrow();
            // The version identifies the native capture, while this control
            // compares trace setup and callback observations after that line.
            let after_version = |bytes: &[u8]| {
                let end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
                bytes[end + 1..].to_vec()
            };
            assert_eq!(after_version(&actual), after_version(expected), "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }

    fn jim_trace_setup_observation(source: &[u8], expected: &[u8]) {
        // Retain the original probe, but stop at its recorded SETUP frontier
        // before the unrelated binary reporter, which this backend lacks.
        let source = std::str::from_utf8(source).unwrap();
        let (prefix, _) = source
            .split_once("binary scan $probeSetupResult H* probeSetupHex\n")
            .unwrap();
        let captured = std::str::from_utf8(expected).unwrap();
        let setup = captured
            .lines()
            .find(|line| line.starts_with("SETUP "))
            .unwrap();
        let hex = setup.strip_prefix("SETUP 1 ").unwrap();
        let expected_message: Vec<_> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert!(!captured
            .lines()
            .any(|line| line.starts_with("OBSERVATION ")));
        crate::counters::reset();
        {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(prefix.as_bytes()), Code::Ok);
            assert!(!interp.host_refusal_pending());
            for (name, expected) in [
                (b"probeSetupCode".as_slice(), b"1".as_slice()),
                (b"probeSetupResult".as_slice(), expected_message.as_slice()),
            ] {
                let value = interp.var_get(name).unwrap();
                assert_eq!(
                    interp.native_string_bytes(&value).unwrap().as_ref(),
                    expected
                );
            }
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    #[test]
    fn active_read_unset_retires_old_registrations_matches_five_native_source_controls() {
        // Native proof: naming.variable.unset-active-read-callback-retirement
        // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_active_read_unset/8.4.20.txt").as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_active_read_unset/8.5.19.txt").as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_active_read_unset/8.6.18.txt").as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_active_read_unset/9.0.4.txt").as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_active_read_unset/9.1.0.txt").as_slice(),
            ),
        ] {
            trace_source_observation(
                engine,
                include_bytes!("../../tests/data/native_active_read_unset/source.tcl"),
                expected,
            );
        }
    }

    #[test]
    fn active_read_unset_probe_preserves_actual_jim_unsupported_setup() {
        // Native proof: naming.variable.unset-active-read-callback-retirement
        // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
        // Jim's recorded setup failure supplies no read/unset callback answer.
        jim_trace_setup_observation(
            include_bytes!("../../tests/data/native_active_read_unset/source.tcl"),
            include_bytes!("../../tests/data/native_active_read_unset/jim.txt"),
        );
    }

    #[test]
    fn recreated_element_read_trace_matches_five_original_source_controls() {
        // Native proof: naming.variable.recreated-element-independent-read-trace
        // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_recreated_element_trace/8.4.20.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_recreated_element_trace/8.5.19.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_recreated_element_trace/8.6.18.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_recreated_element_trace/9.0.4.txt")
                    .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_recreated_element_trace/9.1.0.txt")
                    .as_slice(),
            ),
        ] {
            trace_source_observation(
                engine,
                include_bytes!("../../tests/data/native_recreated_element_trace/source.tcl"),
                expected,
            );
        }
    }

    #[test]
    fn recreated_element_trace_probe_preserves_actual_jim_unsupported_setup() {
        // Native proof: naming.variable.recreated-element-independent-read-trace
        // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
        // The captured Jim setup failure supplies no independent callback answer.
        jim_trace_setup_observation(
            include_bytes!("../../tests/data/native_recreated_element_trace/source.tcl"),
            include_bytes!("../../tests/data/native_recreated_element_trace/jim.txt"),
        );
    }

    #[test]
    fn generic_namespace_declarations_match_all_25_native_execution_results() {
        let source = include_bytes!(
            "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/source.tcl"
        );
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!(
                    "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.4.txt"
                )
                .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!(
                    "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.5.txt"
                )
                .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!(
                    "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.6.txt"
                )
                .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!(
                    "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.0.txt"
                )
                .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!(
                    "../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.1.txt"
                )
                .as_slice(),
            ),
        ] {
            crate::counters::reset();
            {
                let host = Rc::new(NamespaceOutputHost {
                    inner: super::super::default_host(),
                    output: std::cell::RefCell::new(Vec::new()),
                });
                let mut interp = Interp::with_native_core(
                    host.clone(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let code = interp.eval_str(source);
                assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert!(!interp.host_refusal_pending(), "{engine}");
                assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    // Native proof: naming.variable.namespace-alias-settlement-after-store
    // docs/design/analysis/name-resolution-proofs/variable.namespace-alias-settlement-after-store.md
    #[test]
    fn namespace_alias_settlement_matches_all_15_native_callback_and_error_results() {
        let source =
            include_bytes!("../../tests/data/native_namespace_alias_settlement/source.tcl");
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_namespace_alias_settlement/8.4.20.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_namespace_alias_settlement/8.5.19.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_namespace_alias_settlement/8.6.18.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_namespace_alias_settlement/9.0.4.txt")
                    .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_namespace_alias_settlement/9.1.0.txt")
                    .as_slice(),
            ),
        ] {
            crate::counters::reset();
            {
                let host = Rc::new(NamespaceOutputHost {
                    inner: super::super::default_host(),
                    output: std::cell::RefCell::new(Vec::new()),
                });
                let mut interp = Interp::with_native_core(
                    host.clone(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let code = interp.eval_str(source);
                assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert!(!interp.host_refusal_pending(), "{engine}");
                assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    #[test]
    fn qualified_variable_scope_matches_all_six_native_sequences() {
        let source = include_bytes!("../../tests/data/native_variable_qualified_target/source.tcl");
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_variable_qualified_target/tcl8.4.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_variable_qualified_target/tcl8.5.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_variable_qualified_target/tcl8.6.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_variable_qualified_target/tcl9.0.txt")
                    .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_variable_qualified_target/tcl9.1.txt")
                    .as_slice(),
            ),
            (
                "jim",
                include_bytes!("../../tests/data/native_variable_qualified_target/jim.txt")
                    .as_slice(),
            ),
        ] {
            crate::counters::reset();
            {
                let host = Rc::new(NamespaceOutputHost {
                    inner: super::super::default_host(),
                    output: std::cell::RefCell::new(Vec::new()),
                });
                let mut interp = Interp::with_native_core(
                    host.clone(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let code = interp.eval_str(source);
                assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert!(!interp.host_refusal_pending(), "{engine}");
                assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    fn namespace_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let original = argv[1];
        assert_eq!(
            super::super::obj_bytes(interp.read_original_c_variable(original).unwrap()),
            b"LOCAL"
        );
        let local_slot = interp.original_c_local_index(original).unwrap();
        let before = interp
            .frames
            .borrow()
            .native_compiled_cell_identity(local_slot)
            .unwrap();
        let level = interp.current_level();
        let alias_slot = interp
            .frames
            .borrow()
            .native_compiled_name_index(level, b"alias")
            .unwrap();
        let code = interp.link_original_compiled_namespace_variable(
            original,
            crate::namespace::GLOBAL,
            alias_slot,
            true,
        );
        assert_eq!(code, Code::Ok, "{:?}", interp.result_bytes());
        assert!(obj::native_variable_name::with_local(original, |_| ()).is_none());
        assert!(
            obj::native_variable_name::with_parsed(original, |cache| cache.array.is_none())
                .unwrap()
        );
        assert_eq!(
            interp
                .frames
                .borrow()
                .native_compiled_cell_identity(local_slot),
            Some(before)
        );
        assert_eq!(
            super::super::obj_bytes(interp.var_get(b"v").unwrap()),
            b"LOCAL"
        );
        let selected = interp
            .prepare_original_c_name_in(
                original,
                NativeVariableNameLookupPurpose::Read,
                Some(crate::namespace::GLOBAL),
            )
            .unwrap()
            .unwrap();
        let (receiver, _) = interp
            .capture_original_c_selection(
                original,
                &selected,
                NativeVariableNameLookupPurpose::Read,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            super::super::obj_bytes(receiver.read().unwrap().unwrap()),
            b"GLOBAL"
        );
        interp.set_result_bytes(b"");
        Code::Ok
    }

    #[test]
    fn namespace_opcodes_bypass_original_local_cache_and_retain_the_namespace_cell() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            crate::counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                interp.register_builtin(b"namespace_cache_probe", namespace_cache_probe);
                let original = Owned::fresh(obj::new_string_bytes(b"v"));
                interp.var_set(b"name", original.as_ptr()).unwrap();
                let code = interp.eval_str(b"set v GLOBAL; proc p {} {set v LOCAL; namespace_cache_probe $::name; return $alias}; p");
                assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert_eq!(interp.result_bytes(), b"GLOBAL", "{engine}");
                assert!(
                    obj::native_variable_name::with_parsed(original.as_ptr(), |cache| cache
                        .array
                        .is_none())
                    .unwrap()
                );
                assert!(!interp.host_refusal_pending(), "{engine}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{engine}");
            assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
        }
    }

    thread_local! {
        static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    struct CacheLookupObserver;
    impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp>
        for CacheLookupObserver
    {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            _interp: &mut Interp,
            _access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            CACHE_LOOKUP_CALLBACKS.with(|count| count.set(count.get() + 1));
            Ok(())
        }
    }
    fn cache_lookup_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation as Op;
        let original = argv[1];
        let case = CACHE_LOOKUP_CASE.with(std::cell::Cell::get);
        let version = interp.native_c_variable_name_protocol().unwrap().version();
        interp.read_original_c_variable(original).unwrap();
        let before = interp
            .original_c_local_index(original)
            .expect("genuine local cache");
        let before_slot_cell = interp
            .frames
            .borrow()
            .native_compiled_cell_identity(before)
            .expect("actual installed compiled cell");
        let selection = interp.prepare_original_c_name(original, false).unwrap();
        let before_cell = interp
            .capture_original_c_selection(
                original,
                &selection,
                NativeVariableNameLookupPurpose::Read,
            )
            .unwrap()
            .unwrap()
            .1
            .binding_id;
        if matches!(case, 2 | 3) {
            interp
                .add_native_variable_observer(
                    original,
                    &[Op::Read, Op::Write],
                    Rc::new(CacheLookupObserver),
                )
                .unwrap();
        }
        if case == 4 {
            assert!(interp.var_unset(b"x"));
        }
        obj::invalidate_string(original);
        let result = if matches!(case, 1 | 3) {
            let next = Owned::fresh(obj::new_string_bytes(b"NEXT"));
            interp.store_original_c_variable(original, next.as_ptr())
        } else {
            interp.read_original_c_variable(original).map(|_| ())
        };
        assert_eq!(interp.original_c_local_index(original), Some(before));
        assert_eq!(
            interp.frames.borrow().native_compiled_cell_identity(before),
            Some(before_slot_cell),
            "same original cell after {version:?}/{case}"
        );
        // Unset withdraws the value, while the actual compiled Var allocation
        // and the original cache keep their identity through undefinedness.
        assert_eq!(interp.trace_identity(b"x").binding_id, before_cell);
        if case == 4 {
            let frames = interp.frames.borrow();
            assert!(frames
                .table(frames.current_level())
                .unwrap()
                .binding_id(b"x")
                .is_none());
        }
        if version >= tcl_dialect::TclVersion::V8_5 && case >= 2 {
            assert!(result.is_err());
            assert!(
                interp.host_refusal_pending(),
                "native updater abort must remain a host refusal"
            );
            assert!(!obj::has_string_rep(original));
            assert_eq!(CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get), 0);
        } else {
            assert_eq!(result.is_err(), case == 4);
            assert_eq!(
                obj::has_string_rep(original),
                version == tcl_dialect::TclVersion::V8_4
            );
            assert_eq!(
                CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get),
                usize::from(matches!(case, 2 | 3))
            );
        }
        let version_label = match version {
            tcl_dialect::TclVersion::V8_4 => "8.4.20",
            tcl_dialect::TclVersion::V8_5 => "8.5.19",
            tcl_dialect::TclVersion::V8_6 => "8.6.18",
            tcl_dialect::TclVersion::V9_0 => "9.0.4",
            tcl_dialect::TclVersion::V9_1 => "9.1.0",
        };
        let row = if interp.host_refusal_pending() {
            format!("{version_label}|{case}|updater-unavailable")
        } else {
            format!(
                "{version_label}|{case}|{}|{}|localVarName|1",
                usize::from(result.is_err()),
                usize::from(obj::has_string_rep(original))
            )
        };
        CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().push(row));
        if case != 4 {
            // Inspect reached storage without entering another read after refusal.
            let frames = interp.frames.borrow();
            let stored = frames.compiled_slot_var(before).unwrap();
            let crate::frame::Var::Scalar(value) = &*stored else {
                panic!("actual compiled scalar contents");
            };
            assert!(obj::has_string_rep(*value));
            assert_eq!(
                obj::bytes_of(*value),
                if matches!(case, 1 | 3) {
                    b"NEXT".as_slice()
                } else {
                    b"VALUE".as_slice()
                }
            );
        }
        result.map_or_else(|code| code, |()| Code::Ok)
    }

    #[test]
    fn original_local_cache_getter_order_matches_all_25_native_paths() {
        // Native proof: naming.variable.original-local-cache-before-getter
        // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-before-getter.md
        // Native proof: naming.variable.original-local-cache-unavailable-updater
        // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-unavailable-updater.md
        use super::*;
        let expected = include_str!(
            "../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt"
        );
        assert_eq!(expected.lines().count(), 25);
        CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().clear());
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for case in 0..5 {
                CACHE_LOOKUP_CASE.with(|mode| mode.set(case));
                CACHE_LOOKUP_CALLBACKS.with(|count| count.set(0));
                let mut interp = Interp::with_native_core(
                    super::super::default_host(),
                    crate::environment::profile_for_dialect(environment),
                    tcl_registry::special_vars::NativeBootstrapInputs {
                        package_path: Vec::new(),
                        default_library: None,
                    },
                )
                .unwrap();
                interp.register_builtin(b"probe", cache_lookup_probe);
                let code = interp.eval_str(b"proc p {} {set x VALUE;probe x};p");
                assert_eq!(
                    code == Code::Ok,
                    case < 2 || (environment == "tcl8.4" && case < 4),
                    "{environment}/{case}"
                );
            }
        }
        CACHE_LOOKUP_ROWS.with(|rows| {
            assert_eq!(
                rows.borrow().as_slice(),
                expected.lines().collect::<Vec<_>>()
            )
        });
    }

    #[test]
    fn original_link_lookup_and_unset_preserve_target_name_cache() {
        use super::*;
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let target = Owned::fresh(obj::new_string_bytes(b"target"));
            let prepared = interp
                .prepare_original_c_link_target(target.as_ptr(), 0)
                .unwrap();
            let prepared = prepared.expect("actual selected target");
            assert_eq!(prepared.name, b"target");
            assert_eq!(prepared.elem, None);
            assert!(
                obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache
                    .array
                    .is_none())
                .unwrap()
            );
            let value = Owned::fresh(obj::new_string_bytes(b"7"));
            interp
                .assign_original_named_variable(target.as_ptr(), value.as_ptr())
                .unwrap();
            interp
                .unset_original_c_variable(target.as_ptr(), true)
                .unwrap();
            assert!(
                obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache
                    .array
                    .is_none())
                .unwrap()
            );
            assert!(interp
                .read_original_named_variable(target.as_ptr())
                .is_err());
        }
    }

    #[test]
    fn element_table_and_var_roles_match_all_25_actual_callback_windows() {
        // Native proof: naming.variable.original-element-unset-callback-roles
        // docs/design/analysis/name-resolution-proofs/variable-original-element-unset-callback-roles.md
        use super::*;
        use crate::frame::NativeElementEntryObserver;
        use crate::namespace::GLOBAL;
        use std::cell::RefCell;
        use tcl_runtime_api::native_variable_trace::{
            NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
        };
        struct Windows {
            version: &'static str,
            index_k: *mut TclObj,
            index_j: *mut TclObj,
            old_k: NativeElementEntryObserver,
            old_j: NativeElementEntryObserver,
            object_table: bool,
            rows: Rc<RefCell<Vec<String>>>,
            sequence: Rc<RefCell<usize>>,
        }
        impl Windows {
            fn record(&self, interp: &Interp, phase: &str) {
                let current = interp
                    .namespaces
                    .borrow()
                    .var_table(GLOBAL)
                    .capture_array_cell(b"arr");
                let (present_k, _, defined_k, dead_k, refs_k) =
                    self.old_k.observe(current.as_ref(), self.object_table);
                let (present_j, _, defined_j, dead_j, refs_j) =
                    self.old_j.observe(current.as_ref(), self.object_table);
                // SAFETY: the probe's two original index owners outlive every callback.
                let (key_k, key_j) =
                    unsafe { ((*self.index_k).ref_count, (*self.index_j).ref_count) };
                self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{present_k}|{present_j}", self.version));
            }
        }
        impl NativeVariableObserver<Interp> for Windows {
            type Error = tcl_cmd_core::CmdError;
            fn observe(
                &self,
                interp: &mut Interp,
                access: NativeVariableTraceAccess<'_>,
            ) -> Result<(), Self::Error> {
                let phase = if access.name2.is_empty() {
                    "root".to_owned()
                } else {
                    *self.sequence.borrow_mut() += 1;
                    format!(
                        "element{}-{}",
                        *self.sequence.borrow(),
                        std::str::from_utf8(access.name2).unwrap()
                    )
                };
                self.record(interp, &phase);
                Ok(())
            }
        }
        let rows = Rc::new(RefCell::new(Vec::new()));
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let root = Owned::fresh(obj::new_string_bytes(b"arr"));
            let index_k = Owned::fresh(obj::new_string_bytes(b"k"));
            let index_j = Owned::fresh(obj::new_string_bytes(b"j"));
            let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
            for index in [&index_k, &index_j] {
                let capture = interp
                    .capture_original_c_parts_report(
                        root.as_ptr(),
                        Some(index.as_ptr()),
                        NativeVariableNameLookupPurpose::Write,
                    )
                    .unwrap()
                    .unwrap();
                capture.receiver.store(one.as_ptr()).unwrap();
            }
            let array = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .capture_array_cell(b"arr")
                .unwrap();
            let names = [
                Owned::fresh(obj::new_string_bytes(b"arr(k)")),
                Owned::fresh(obj::new_string_bytes(b"arr(j)")),
            ];
            let mut aliases = Vec::new();
            for name in &names {
                let mut target = interp
                    .prepare_original_c_link_target(name.as_ptr(), 0)
                    .unwrap()
                    .unwrap();
                crate::vars::prepare_upvar_target(
                    &mut interp.frames.borrow_mut(),
                    &mut interp.namespaces.borrow_mut(),
                    &mut target,
                )
                .unwrap();
                aliases.push(target);
            }
            let windows = Rc::new(Windows {
                version,
                index_k: index_k.as_ptr(),
                index_j: index_j.as_ptr(),
                old_k: array.observe_element_entry(b"k").unwrap(),
                old_j: array.observe_element_entry(b"j").unwrap(),
                object_table: interp
                    .native_c_variable_name_protocol()
                    .unwrap()
                    .element_table_retains_original(),
                rows: Rc::clone(&rows),
                sequence: Rc::new(RefCell::new(0)),
            });
            for name in std::iter::once(&root).chain(names.iter()) {
                interp
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Unset],
                        windows.clone(),
                    )
                    .unwrap();
            }
            windows.record(&interp, "before");
            assert!(interp.var_unset(b"arr"));
            windows.record(&interp, "after");
            drop(aliases);
        }
        let expected = include_str!("../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt").lines().filter(|line| line.contains("|window|")).collect::<Vec<_>>();
        assert_eq!(rows.borrow().len(), 25);
        assert_eq!(*rows.borrow(), expected);
    }

    #[test]
    fn element_entry_lifecycle_matches_all_70_actual_c_windows() {
        // Native proof: naming.variable.original-element-alias-recreation
        // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-recreation.md
        // Native proof: naming.variable.original-element-alias-header-roles
        // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-header-roles.md
        use super::*;
        use crate::namespace::GLOBAL;
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            for whole in [false, true] {
                let mut interp = Interp::new();
                interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
                let protocol = interp.native_c_variable_name_protocol().unwrap();
                let original = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
                let fresh = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
                let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
                let two = Owned::fresh(obj::new_string_bytes(b"TWO"));
                interp
                    .assign_original_named_variable(original.as_ptr(), one.as_ptr())
                    .unwrap();
                let array = interp
                    .namespaces
                    .borrow()
                    .var_table(GLOBAL)
                    .capture_array_cell(b"arr")
                    .unwrap();
                let observer = array.observe_element_entry(b"k").unwrap();
                let root = obj::native_variable_name::with_parsed(original.as_ptr(), |cache| {
                    cache.array.as_ref().unwrap().0.as_ptr()
                })
                .unwrap();
                let key = observer.key();
                // This is the native probe's declared external key observer,
                // needed only for the separately allocated C85/86 table key.
                let _key_pin = (protocol.version() >= tcl_dialect::TclVersion::V8_5
                    && protocol.version() < tcl_dialect::TclVersion::V9_0)
                    .then(|| Owned::retain(key.unwrap()));
                let observe = |interp: &Interp, phase: &str, rows: &mut Vec<String>| {
                    let current = interp
                        .namespaces
                        .borrow()
                        .var_table(GLOBAL)
                        .capture_array_cell(b"arr");
                    let (present, same, defined, dead, refs) = observer
                        .observe(current.as_ref(), protocol.element_table_retains_original());
                    // SAFETY: original parser storage or the declared observer pin owns each header.
                    let (name_refs, root_refs, key_refs) = unsafe {
                        (
                            (*original.as_ptr()).ref_count,
                            (*root).ref_count,
                            key.map_or(-1, |key| (*key).ref_count),
                        )
                    };
                    rows.push(format!("{version}|window|{}|{phase}|{name_refs}|{root_refs}|{key_refs}|{present}|{same}|{defined}|{dead}|{refs}", if whole { "whole" } else { "element" }));
                };
                observe(&interp, "created", &mut rows);
                let mut first = interp
                    .prepare_original_c_link_target(original.as_ptr(), 0)
                    .unwrap()
                    .unwrap();
                crate::vars::prepare_upvar_target(
                    &mut interp.frames.borrow_mut(),
                    &mut interp.namespaces.borrow_mut(),
                    &mut first,
                )
                .unwrap();
                let mut second = interp
                    .prepare_original_c_link_target(original.as_ptr(), 0)
                    .unwrap()
                    .unwrap();
                crate::vars::prepare_upvar_target(
                    &mut interp.frames.borrow_mut(),
                    &mut interp.namespaces.borrow_mut(),
                    &mut second,
                )
                .unwrap();
                observe(&interp, "linked", &mut rows);
                let target: &[u8] = if whole { b"arr" } else { b"arr(k)" };
                assert!(interp.var_unset(target));
                observe(&interp, "unset-target", &mut rows);
                interp
                    .assign_original_named_variable(fresh.as_ptr(), two.as_ptr())
                    .unwrap();
                observe(&interp, "recreated", &mut rows);
                assert!(interp.var_unset(target));
                observe(&interp, "unset-recreated", &mut rows);
                drop(second);
                observe(&interp, "one-alias", &mut rows);
                drop(first);
                observe(&interp, "no-alias", &mut rows);
            }
        }
        let expected = include_str!("../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.txt").lines().filter(|line| line.contains("|window|") && !line.starts_with("Jim|")).collect::<Vec<_>>();
        assert_eq!(rows.len(), 70);
        assert_eq!(rows, expected);
    }

    #[test]
    fn search_free_slots_match_all_20_actual_c_windows() {
        // Native proof: naming.variable.array-search-free-slot-retirement
        // docs/design/analysis/name-resolution-proofs/variable-array-search-free-slot-retirement.md
        use super::*;
        use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let protocol = interp.array_search_protocol().unwrap();
            let name = Owned::fresh(obj::new_string_bytes(b"a(k)"));
            let member = Owned::fresh(obj::new_string_bytes(b"v"));
            interp
                .assign_original_named_variable(name.as_ptr(), member.as_ptr())
                .unwrap();
            let array = Owned::fresh(obj::new_string_bytes(b"a"));
            let command = Owned::fresh(obj::new_string_bytes(b"array"));
            let start = Owned::fresh(obj::new_string_bytes(b"startsearch"));
            assert_eq!(
                interp.dispatch(&[command.as_ptr(), start.as_ptr(), array.as_ptr()]),
                Code::Ok
            );
            let handle = Owned::retain(interp.get_obj_result());
            let anymore = Owned::fresh(obj::new_string_bytes(b"anymore"));
            assert_eq!(
                interp.dispatch(&[
                    command.as_ptr(),
                    anymore.as_ptr(),
                    array.as_ptr(),
                    handle.as_ptr()
                ]),
                Code::Ok
            );
            let observe = |phase: &str, original: &Owned| {
                let kind = obj::obj_type_ptr(original.as_ptr());
                // SAFETY: the test owns this original header and live descriptor.
                let free = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
                format!(
                    "{version}|{phase}|{}|{}|{}",
                    if obj::native_array_search_cache_in(original.as_ptr(), protocol)
                        .unwrap()
                        .is_some()
                    {
                        "array search"
                    } else if matches!(
                        obj::native_object_snapshot(original.as_ptr())
                            .unwrap()
                            .cache,
                        tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                    ) {
                        "string"
                    } else {
                        "none"
                    },
                    usize::from(free),
                    usize::from(obj::has_string_rep(original.as_ptr()))
                )
            };
            rows.push(observe("converted", &handle));
            let copy = Owned::fresh(obj::duplicate(handle.as_ptr()));
            rows.push(observe("duplicate", &copy));
            assert!(interp
                .read_original_named_variable(handle.as_ptr())
                .is_err());
            rows.push(observe("missing", &handle));
            assert!(interp.read_original_named_variable(copy.as_ptr()).is_err());
            rows.push(observe("duplicate-missing", &copy));
        }
        let expected = include_str!(
            "../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
        )
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
        assert_eq!(rows.len(), 20);
        assert_eq!(rows, expected);
    }
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

    #[test]
    fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
        // Native proof: naming.variable.original-scalar-alias-entry-lifetime
        // docs/design/analysis/name-resolution-proofs/variable-original-scalar-alias-entry-lifetime.md
        // Native proof: naming.variable.original-parsed-array-child-references
        // docs/design/analysis/name-resolution-proofs/variable-original-parsed-array-child-references.md
        use crate::namespace::GLOBAL;
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            for mode in 0..2 {
                let name = Owned::fresh(obj::new_string_bytes(b"k"));
                let value = Owned::fresh(obj::new_string_bytes(b"ONE"));
                interp
                    .assign_original_named_variable(name.as_ptr(), value.as_ptr())
                    .unwrap();
                let before = interp
                    .namespaces
                    .borrow()
                    .var_table(GLOBAL)
                    .native_name_cell_identity(b"k")
                    .unwrap();
                let observe = |interp: &Interp, phase: &str, fresh: Option<&Owned>| {
                    let namespaces = interp.namespaces.borrow();
                    let table = namespaces.var_table(GLOBAL);
                    let found = table.native_name_cell_identity(b"k");
                    assert!(matches!(
                        obj::native_object_snapshot(name.as_ptr()).unwrap().cache,
                        Cache::ParsedVariableName { .. }
                    ));
                    // SAFETY: these original object headers are owned throughout each window.
                    let (references, fresh_references) = unsafe {
                        (
                            (*name.as_ptr()).ref_count,
                            fresh.map_or(-1, |fresh| (*fresh.as_ptr()).ref_count),
                        )
                    };
                    format!(
                        "{version}|alias|{mode}|{phase}|parsedVarName|{references}|{fresh_references}|{}|{}|{}",
                        usize::from(found.is_some()),
                        usize::from(found == Some(before)),
                        usize::from(table.original_native_key_is(b"k", name.as_ptr()))
                    )
                };
                rows.push(observe(&interp, "created", None));
                interp.frames.borrow_mut().push(GLOBAL);
                let target = crate::vars::link_target_at(
                    &interp.frames.borrow(),
                    &interp.namespaces.borrow(),
                    b"k",
                    None,
                    0,
                )
                .unwrap();
                interp.make_upvar(target, b"a");
                rows.push(observe(&interp, "linked", None));
                assert!(interp.var_unset(b"::k"));
                rows.push(observe(&interp, "unset-target", None));
                let fresh = Owned::fresh(obj::new_string_bytes(b"k"));
                let written =
                    Owned::fresh(obj::new_string_bytes(if mode == 0 { b"::k" } else { b"a" }));
                interp
                    .assign_original_named_variable(written.as_ptr(), value.as_ptr())
                    .unwrap();
                rows.push(observe(&interp, "recreated", Some(&fresh)));
                assert!(interp.var_unset(b"::k"));
                rows.push(observe(&interp, "unset-recreated-target", Some(&fresh)));
                assert!(!interp.var_unset(b"a"));
                rows.push(observe(&interp, "unset-alias", Some(&fresh)));
                interp.frames.borrow_mut().pop();
                rows.push(observe(&interp, "frame-popped", Some(&fresh)));
                assert!(interp.read_original_named_variable(fresh.as_ptr()).is_err());
                rows.push(observe(&interp, "fresh-read", Some(&fresh)));
            }
            let name = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
            let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
            interp
                .assign_original_named_variable(name.as_ptr(), value.as_ptr())
                .unwrap();
            let observe = |phase: &str| {
                obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
                    let (root, element) = cache.array.as_ref().unwrap();
                    // SAFETY: cache children remain owned by the original parsed name.
                    let (root_refs, element_refs) = unsafe {
                        (
                            (*root.as_ptr()).ref_count,
                            match element {
                                NativeParsedVariableElement::Object(element) => {
                                    (*element.as_ptr()).ref_count
                                }
                                NativeParsedVariableElement::Bytes(_) => -1,
                            },
                        )
                    };
                    format!("{version}|parts|{phase}|{root_refs}|{element_refs}")
                })
                .unwrap()
            };
            rows.push(observe("stored"));
            let copy = Owned::fresh(obj::duplicate(name.as_ptr()));
            rows.push(observe("duplicated"));
            drop(copy);
            assert!(interp.var_unset(b"arr"));
            rows.push(observe("unset"));
        }
        let expected = include_str!(
            "../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
        )
        .lines()
        .collect::<Vec<_>>();
        assert_eq!(rows.len(), 95);
        assert_eq!(rows, expected);
    }

    fn observe(version: &str, case: &str, phase: &str, original: &Owned) -> String {
        let snapshot = obj::native_object_snapshot(original.as_ptr()).unwrap();
        let kind = match snapshot.cache {
            Cache::None => "none",
            Cache::Numeric(_) => "int",
            Cache::List { .. } => "list",
            Cache::ByteArray { .. } => "bytearray",
            Cache::ParsedVariableName { .. } => "parsedVarName",
            other => panic!("unexpected actual original primary: {other:?}"),
        };
        let descriptor = obj::obj_type_ptr(original.as_ptr());
        // SAFETY: both the retained original header and its descriptor are live.
        let (references, free_hook) = unsafe {
            (
                (*original.as_ptr()).ref_count,
                !descriptor.is_null() && (*descriptor).free_int_rep_proc.is_some(),
            )
        };
        let hex = snapshot
            .resident
            .as_deref()
            .map_or_else(String::new, |bytes| {
                bytes.iter().map(|byte| format!("{byte:02x}")).collect()
            });
        format!(
            "{version}|{case}|{phase}|{kind}|{references}|{}|{}|{hex}",
            usize::from(free_hook),
            usize::from(snapshot.resident.is_some())
        )
    }

    #[test]
    fn original_parsed_headers_match_all_150_actual_c_windows() {
        // Native proof: naming.variable.original-scalar-parsed-header
        // docs/design/analysis/name-resolution-proofs/variable-original-scalar-parsed-header.md
        // Native proof: naming.variable.original-counted-array-parsed-header
        // docs/design/analysis/name-resolution-proofs/variable-original-counted-array-parsed-header.md
        let mut rows = Vec::new();
        for (environment, version) in [
            ("tcl8.4", "8.4.20"),
            ("tcl8.5", "8.5.19"),
            ("tcl8.6", "8.6.18"),
            ("tcl9.0", "9.0.4"),
            ("tcl9.1", "9.1.0"),
        ] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let dialect = interp.native_invocation_dialect();
            let protocol = dialect.native_string_protocol().unwrap();
            for case in ["string", "int", "list", "bytearray", "array"] {
                let original = match case {
                    "string" => Owned::fresh(obj::new_string_bytes(b"missing")),
                    "int" => {
                        let original = Owned::fresh(obj::new_wide_int_obj(42));
                        if version == "8.4.20" {
                            obj::adopt_native_scalar_cache(
                                original.as_ptr(),
                                tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(42),
                                dialect.native_scalar_getter_protocol().unwrap(),
                            )
                            .unwrap();
                        }
                        original
                    }
                    "list" => {
                        let member = Owned::fresh(obj::new_string_bytes(b"missing"));
                        Owned::fresh(crate::list::new_list_obj_native(
                            &[member.as_ptr()],
                            protocol,
                        ))
                    }
                    "bytearray" => Owned::fresh(interp.new_native_byte_array(b"missing").unwrap()),
                    "array" => Owned::fresh(obj::new_string_bytes(b"arr(k\0z)")),
                    _ => unreachable!(),
                };
                rows.push(observe(version, case, "before", &original));
                assert!(interp
                    .read_original_named_variable(original.as_ptr())
                    .is_err());
                rows.push(observe(version, case, "missing", &original));
                let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
                interp
                    .store_original_named_variable(original.as_ptr(), value.as_ptr())
                    .unwrap();
                rows.push(observe(version, case, "stored", &original));
                assert_eq!(
                    interp
                        .read_original_named_variable(original.as_ptr())
                        .unwrap(),
                    value.as_ptr()
                );
                rows.push(observe(version, case, "read", &original));
                let copy = Owned::fresh(obj::duplicate(original.as_ptr()));
                rows.push(observe(version, case, "duplicate", &copy));
                drop(copy);
                for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                    interp.var_unset(name);
                }
                assert!(interp
                    .read_original_named_variable(original.as_ptr())
                    .is_err());
                rows.push(observe(version, case, "after-unset-missing", &original));
            }
        }
        let expected = include_str!(
            "../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt"
        )
        .lines()
        .collect::<Vec<_>>();
        assert_eq!(rows.len(), 150);
        assert_eq!(rows, expected);
    }
    #[test]
    fn list_assignment_arity_matches_five_original_source_controls() {
        // Native proof: naming.list.original-lassign-target-arity
        // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
        // Literal, dynamic and proc source entry are observations, not admission proofs.
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_list_assignment_arity/8.4.20.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_list_assignment_arity/8.5.19.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_list_assignment_arity/8.6.18.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_list_assignment_arity/9.0.4.txt")
                    .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_list_assignment_arity/9.1.0.txt")
                    .as_slice(),
            ),
        ] {
            trace_source_observation(
                engine,
                include_bytes!("../../tests/data/native_list_assignment_arity/source.tcl"),
                expected,
            );
        }
    }

    #[test]
    fn list_assignment_arity_matches_current_jim_original_source_controls() {
        // Native proof: naming.list.original-lassign-target-arity
        // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
        trace_source_observation(
            "jim",
            include_bytes!("../../tests/data/native_list_assignment_arity/source.tcl"),
            include_bytes!("../../tests/data/native_list_assignment_arity/jim.txt"),
        );
    }
    #[test]
    fn variable_read_diagnostics_match_five_original_source_controls() {
        // Native proof: naming.variable.original-read-diagnostic-input
        // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
        // These source-entry outcomes grant no private layout or compiler admission.
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_variable_read_input/8.4.20.txt").as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_variable_read_input/8.5.19.txt").as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_variable_read_input/8.6.18.txt").as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_variable_read_input/9.0.4.txt").as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_variable_read_input/9.1.0.txt").as_slice(),
            ),
        ] {
            trace_source_observation(
                engine,
                include_bytes!("../../tests/data/native_variable_read_input/source.tcl"),
                expected,
            );
        }
    }

    #[test]
    fn variable_read_input_preserves_current_jim_dictionary_results() {
        // Native proof: naming.variable.original-read-diagnostic-input
        // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
        // Jim's array-valued root reads succeed; its scalar element reads fail.
        trace_source_observation(
            "jim",
            include_bytes!("../../tests/data/native_variable_read_input/source.tcl"),
            include_bytes!("../../tests/data/native_variable_read_input/jim.txt"),
        );
    }

    #[test]
    fn retired_namespace_c86_source_entries_preserve_scalar_and_element_sites() {
        trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/input.tcl"
            ),
            include_bytes!(
                "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.6.18/stdout.txt"
            ),
        );
    }

    #[test]
    fn retired_namespace_c86_variable_heads_preserve_original_root_tuple() {
        trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/variable-head-input.tcl"
            ),
            include_bytes!(
                "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
            ),
        );
    }

    #[test]
    fn retired_namespace_original_object_vector_keeps_root_despite_compiled_cell() {
        fn original_vector(interp: &mut Interp, _args: &[*mut TclObj]) -> Code {
            let head = Owned::fresh(obj::new_string_bytes(b"set"));
            let name = Owned::fresh(obj::new_string_bytes(b"foo(3)"));
            let value = Owned::fresh(obj::new_string_bytes(b"3"));
            let code =
                interp.eval_original_object_vector(&[head.as_ptr(), name.as_ptr(), value.as_ptr()]);
            let root = obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
                cache.array.as_ref().map(|(root, _)| root.as_ptr())
            })
            .flatten()
            .expect("original parsed root retained after failure");
            assert!(interp.original_c_local_index(root).is_some());
            code
        }
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"original_vector_write", original_vector);
        assert_eq!(
            interp.eval_str(
                br#"
            namespace eval test_ns_var {variable foo 2}
            proc p {} {
                variable ::test_ns_var::foo
                set live [set foo]
                namespace delete ::test_ns_var
                set code [catch {original_vector_write} result]
                list $code $result $::errorCode
            }
            p
        "#
            ),
            Code::Ok,
            "{:?}",
            interp.native_execution_refusal()
        );
        let protocol =
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let capture = include_str!(
            "../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
        );
        let line = capture
            .lines()
            .find(|line| line.starts_with("ROW variable-head-element "))
            .unwrap();
        let row = tcl_syntax::list::split_native_list_bytes(line.as_bytes(), protocol).unwrap();
        let expected = tcl_syntax::list::split_native_list_bytes(&row[3], protocol).unwrap();
        let result = interp.result_bytes();
        let actual = tcl_syntax::list::split_native_list_bytes(&result, protocol).unwrap();
        assert_eq!(actual.len(), 3);
        for (actual, expected) in actual.iter().zip(&expected[2..]) {
            assert_eq!(actual.as_ref(), expected.as_ref());
        }
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn detached_element_c86_original_var17_write_preserves_late_diagnostic() {
        trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/original-var17.tcl"
            ),
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/original-var17.txt"
            ),
        );
    }

    #[test]
    fn detached_element_c86_recreated_root_keeps_old_link_write_detached() {
        trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/root-recreated-old-link.tcl"
            ),
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/root-recreated-old-link.txt"
            ),
        );
    }

    #[test]
    fn detached_element_c86_recreated_root_admits_new_link_write() {
        trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/root-recreated-current-link.tcl"
            ),
            include_bytes!(
                "../../tests/data/native_detached_element_write_sites/root-recreated-current-link.txt"
            ),
        );
    }

    #[test]
    fn array_root_write_diagnostics_match_five_original_source_controls() {
        // Native proof: naming.variable.original-array-root-write-diagnostic
        // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
        // Full source output is compared; no private receiver or compiler admission is inferred.
        for (engine, expected) in [
            (
                "tcl8.4",
                include_bytes!("../../tests/data/native_variable_array_write/8.4.20.txt")
                    .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!("../../tests/data/native_variable_array_write/8.5.19.txt")
                    .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!("../../tests/data/native_variable_array_write/8.6.18.txt")
                    .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!("../../tests/data/native_variable_array_write/9.0.4.txt").as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!("../../tests/data/native_variable_array_write/9.1.0.txt").as_slice(),
            ),
        ] {
            trace_source_observation(
                engine,
                include_bytes!("../../tests/data/native_variable_array_write/source.tcl"),
                expected,
            );
        }
    }

    #[test]
    fn array_root_write_preserves_current_jim_dictionary_replacement() {
        // Native proof: naming.variable.original-array-root-write-diagnostic
        // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
        // Jim replaces its dictionary-valued root with the scalar result.
        trace_source_observation(
            "jim",
            include_bytes!("../../tests/data/native_variable_array_write/source.tcl"),
            include_bytes!("../../tests/data/native_variable_array_write/jim.txt"),
        );
    }
}

#[cfg(test)]
mod original_assignment_tests {
    use super::*;
    use std::cell::Cell;
    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };

    thread_local! {
        static MODE: Cell<u8> = const { Cell::new(0) };
        static ELEMENT: Cell<bool> = const { Cell::new(false) };
        static EXPECTED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static SAW_NEW: Cell<bool> = const { Cell::new(false) };
        static SAW_OLD: Cell<bool> = const { Cell::new(false) };
        static INCOMING_REFS: Cell<isize> = const { Cell::new(0) };
        static RELEASED: Cell<bool> = const { Cell::new(false) };
        static WRITES: Cell<usize> = const { Cell::new(0) };
    }
    fn native(profile: &str) -> Interp {
        Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    struct Current(*mut Interp);
    impl Current {
        fn enter(interp: &mut Interp) -> Self {
            let old = crate::codegen_abi::current_interp();
            crate::codegen_abi::tcl_runtime_set_current_interp(interp);
            Self(old)
        }
    }
    impl Drop for Current {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
        }
    }
    extern "C" fn retire(original: *mut TclObj) {
        CALLS.with(|value| value.set(value.get() + 1));
        let Some(mut interp) = (unsafe { crate::codegen_abi::current_interp().as_ref() }).cloned()
        else {
            return;
        };
        let available =
            interp.frames.try_borrow_mut().is_ok() && interp.namespaces.try_borrow_mut().is_ok();
        RELEASED.with(|value| value.set(available));
        if interp
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let selected = interp
                .namespaces()
                .var_table(crate::namespace::GLOBAL)
                .published_scalar_header(b"target");
            SAW_OLD.with(|value| value.set(selected == Some(original)));
            assert_eq!(unsafe { (*original).ref_count }, 0);
            INCOMING_REFS.with(|value| {
                value.set(unsafe { (*EXPECTED.with(Cell::get)).ref_count });
            });
        } else {
            let selected = if ELEMENT.with(Cell::get) {
                interp.var_get_elem(b"arr", b"k")
            } else {
                interp.var_get(b"target")
            };
            SAW_NEW.with(|value| value.set(selected == Some(EXPECTED.with(Cell::get))));
        }
        match MODE.with(Cell::get) {
            1 => {
                interp.refuse_host_command("first original assignment retirement");
            }
            2 => {
                let actual = interp.runtime_context();
                let mut changed = actual.clone();
                changed.packages = vec![("assignment-retirement".to_owned(), "1.0".to_owned())];
                interp.pin_context(&changed).unwrap();
                interp.pin_context(&actual).unwrap();
            }
            _ => {}
        }
    }
    static RETIRE: obj::TclObjType = obj::TclObjType {
        name: c"originalAssignmentRetirement".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    struct Write;
    impl NativeVariableObserver<Interp> for Write {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            _interp: &mut Interp,
            _access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            WRITES.with(|value| value.set(value.get() + 1));
            Ok(())
        }
    }
    fn reset(mode: u8, element: bool, value: *mut TclObj) {
        MODE.with(|selected| selected.set(mode));
        ELEMENT.with(|selected| selected.set(element));
        EXPECTED.with(|selected| selected.set(value));
        CALLS.with(|selected| selected.set(0));
        SAW_NEW.with(|selected| selected.set(false));
        SAW_OLD.with(|selected| selected.set(false));
        INCOMING_REFS.with(|selected| selected.set(0));
        RELEASED.with(|selected| selected.set(false));
        WRITES.with(|selected| selected.set(0));
    }

    #[test]
    fn original_c_assignment_releases_old_value_before_trace_with_cell_borrows_released() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual receiver callbacks and ownership, without external refcount/order equivalence.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for element in [false, true] {
                for mode in 0..=2 {
                    let mut interp = native(profile);
                    let name = Owned::fresh(obj::new_string_bytes(if element {
                        b"arr(k)"
                    } else {
                        b"target"
                    }));
                    let old = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
                    unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"OLD", false) };
                    interp
                        .assign_original_named_variable(name.as_ptr(), old.as_ptr())
                        .unwrap();
                    drop(old);
                    interp
                        .add_native_variable_observer(
                            name.as_ptr(),
                            &[NativeVariableTraceOperation::Write],
                            Rc::new(Write),
                        )
                        .unwrap();
                    let next = Owned::fresh(obj::new_string_bytes(b"NEXT\0VALUE"));
                    interp.set_result_bytes(b"PRIOR");
                    let prior = interp.get_obj_result();
                    let refs = unsafe { (*next.as_ptr()).ref_count };
                    reset(mode, element, next.as_ptr());
                    let current = Current::enter(&mut interp);
                    let assigned =
                        interp.assign_original_named_variable(name.as_ptr(), next.as_ptr());
                    assert_eq!(assigned.is_ok(), mode == 0, "{profile}/{element}/{mode}");
                    assert_eq!(CALLS.with(Cell::get), 1);
                    assert!(RELEASED.with(Cell::get));
                    assert!(SAW_NEW.with(Cell::get));
                    assert_eq!(WRITES.with(Cell::get), usize::from(mode == 0));
                    assert_eq!(unsafe { (*next.as_ptr()).ref_count }, refs + 1);
                    assert_eq!(interp.get_obj_result(), prior);
                    if mode == 0 {
                        assert!(!interp.host_refusal_pending());
                    } else {
                        let cause = interp.native_execution_refusal().unwrap();
                        if mode == 1 {
                            let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(host) =
                                cause
                            else {
                                panic!("actual original Host command cause");
                            };
                            assert_eq!(host.reason, "first original assignment retirement");
                        }
                        assert_eq!(assigned, Err(Code::Error));
                    }
                    drop(current);
                    assert_eq!(interp.entered_native_operation_count(), 0);
                }
            }
        }
    }

    #[test]
    fn original_jim_assignment_updates_actual_cached_cell_before_retirement_callback() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Jim_SetVariable keeps the old VarVal pointer during its decrement.
        // A refused software continuation cannot perform the later assignment.
        for fresh_name in [false, true] {
            for mode in 0..=2 {
                let mut interp = native("jim");
                let original = Owned::fresh(obj::new_string_bytes(b"target"));
                let old = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
                unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"OLD", false) };
                interp
                    .assign_original_named_variable(original.as_ptr(), old.as_ptr())
                    .unwrap();
                drop(old);
                let selected = if fresh_name {
                    Owned::fresh(obj::new_string_bytes(b"target"))
                } else {
                    Owned::retain(original.as_ptr())
                };
                let next = Owned::fresh(obj::new_string_bytes(b"NEXT"));
                interp.set_result_bytes(b"PRIOR");
                let prior = interp.get_obj_result();
                reset(mode, false, next.as_ptr());
                let current = Current::enter(&mut interp);
                let assigned =
                    interp.assign_original_named_variable(selected.as_ptr(), next.as_ptr());
                assert_eq!(assigned.is_ok(), mode == 0, "{fresh_name}/{mode}");
                assert_eq!(CALLS.with(Cell::get), 1);
                assert!(SAW_OLD.with(Cell::get));
                assert_eq!(INCOMING_REFS.with(Cell::get), 2);
                assert!(RELEASED.with(Cell::get));
                let published = interp
                    .namespaces()
                    .var_table(crate::namespace::GLOBAL)
                    .published_scalar_header(b"target");
                assert_eq!(published, (mode == 0).then_some(next.as_ptr()));
                assert_eq!(interp.get_obj_result(), prior);
                assert_eq!(
                    unsafe { (*next.as_ptr()).ref_count },
                    if mode == 0 { 2 } else { 1 }
                );
                assert_eq!(interp.host_refusal_pending(), mode != 0);
                if mode == 0 {
                    assert_eq!(interp.var_get(b"target"), Some(next.as_ptr()));
                } else {
                    assert_eq!(assigned, Err(Code::Error));
                    let cause = interp.native_execution_refusal().unwrap();
                    if mode == 1 {
                        assert!(
                            matches!(cause, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(host)
                            if host.reason == "first original assignment retirement")
                        );
                    }
                }
                drop(current);
                assert_eq!(interp.entered_native_operation_count(), 0);
            }
        }
    }
}
