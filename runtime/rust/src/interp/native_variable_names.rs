// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C name ingress; physical caches precede receiver observers.

#[path = "native_variable_names/native_exists.rs"]
mod native_exists;
#[path = "native_variable_names/native_array.rs"]
mod native_array;
#[path = "native_variable_names/native_upvar.rs"]
mod native_upvar;
use super::{Code, Interp};
#[path = "native_variable_names/native_unset.rs"]
mod native_unset;
use crate::obj::{self, Owned, TclObj};
use std::rc::Rc;
use tcl_runtime_api::native_literal::NativeLocalNameTable;
use tcl_syntax::{
    native_variable_name::{
        NativeLocalVariableName, NativeLocalVariableOwner, NativeParsedVariableElement,
        NativeParsedVariableName, NativeVariableNameLookupPurpose,
    },
    value::{ValueError, ValueOps},
};

struct OriginalCNameSelection {
    root: Vec<u8>,
    element: Option<Vec<u8>>,
    compiled: Option<usize>,
    namespace: Option<crate::namespace::NsId>,
}

/// Reporting bytes captured by the same original-name lookup that selected the
/// physical cell. They do not grant another lookup or materialise another name.
pub(super) struct OriginalCVariableCapture {
    pub receiver: crate::frame::VariableReceiver,
    pub home: crate::vars::TraceHome,
    pub root: Vec<u8>,
    pub element: Option<Vec<u8>>,
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
        let selected = self.name_policy_protocol()?;
        let protocol = self
            .native_invocation_dialect()
            .native_variable_name_protocol()?;
        (selected.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            && selected.recipe() == tcl_syntax::naming::NativeNameProtocol::C(protocol.version()))
        .then_some(protocol)
    }
    /// Share the real procedure's canonical-name header; activation retains
    /// its header, not another reference to each original name object.
    pub(super) fn refresh_native_local_name_table(&self) -> Result<(), ValueError> {
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
        let table = {
            let mut retained = procedure.native_local_names.borrow_mut();
            if retained.as_ref().is_none_or(|table| table.layout != layout) {
                let table = NativeLocalNameTable::create(
                    &self.native_literal_world,
                    &layout,
                    tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                    |original| {
                        self.native_object_string_bytes(original.as_ptr())
                            .map(|bytes| bytes.to_vec())
                    },
                    |bytes| {
                        super::native_literal_pool::registered_string(
                            bytes,
                            self.native_invocation_dialect(),
                        )
                    },
                )?;
                *retained = Some(NativeProcedureNameTable { layout, table });
            }
            Rc::clone(&retained.as_ref().expect("canonical local table").table)
        };
        self.frames
            .borrow_mut()
            .install_native_local_name_table(table);
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

    fn install_original_c_name_cache(
        &self,
        original: *mut TclObj,
        index: Option<usize>,
    ) -> Result<(), ValueError> {
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_variable_name_protocol()
            .expect("C variable issuer");
        let Some(index) = index else {
            return obj::native_variable_name::install_parsed(
                original,
                NativeParsedVariableName {
                    protocol,
                    array: None,
                },
                dialect,
            );
        };
        if protocol.local_cache_owns_procedure() {
            let procedure = self.frames.borrow().current_c_procedure().cloned().ok_or(
                ValueError::CommandProtocolUnavailable("actual C8.4 procedure cache owner"),
            )?;
            procedure
                .retain_native_compiled_frame_names(self.frames.borrow().native_compiled_names());
            return obj::native_variable_name::install_local(
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
            );
        }
        let frames = self.frames.borrow();
        let canonical = frames
            .native_local_name_table()
            .and_then(|table| table.names.get(index))
            .and_then(Option::as_ref)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "actual canonical local name owner",
            ))?;
        let canonical = canonical.as_ptr();
        let owner = NativeLocalVariableOwner::Name(
            (canonical != original).then(|| Owned::retain(canonical)),
        );
        obj::native_variable_name::install_local(
            original,
            NativeLocalVariableName {
                protocol,
                index,
                owner,
            },
            dialect,
        )?;
        if canonical != original && protocol.installs_canonical_local_self_cache() {
            obj::discard_native_internal_representation(canonical)?;
            obj::native_variable_name::install_local(
                canonical,
                NativeLocalVariableName {
                    protocol,
                    index,
                    owner: NativeLocalVariableOwner::Name(None),
                },
                dialect,
            )?;
        } else if canonical != original
            && protocol.version() == tcl_dialect::TclVersion::V8_6
            && !obj::native_variable_name::with_local(canonical, |cache| {
                matches!(cache.owner, NativeLocalVariableOwner::Name(None))
            })
            .unwrap_or(false)
        {
            obj::discard_native_internal_representation(canonical)?;
        }
        Ok(())
    }

    fn prepare_original_c_name(
        &mut self,
        original: *mut TclObj,
        create: bool,
    ) -> Result<OriginalCNameSelection, Code> {
        let purpose = if create {
            NativeVariableNameLookupPurpose::Write
        } else {
            NativeVariableNameLookupPurpose::Read
        };
        self.prepare_original_c_name_for(original, purpose)?
            .ok_or_else(|| {
                let bytes = self.native_string_bytes(&original);
                match bytes {
                    Ok(bytes) => self.no_such_variable(&bytes, None),
                    Err(error) => self.report_cmd_error(error.into()),
                }
            })
    }

    fn prepare_original_c_name_for(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_in(original, purpose, None)
    }

    fn prepare_original_c_name_in(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        self.prepare_original_c_name_parts(original, purpose, namespace, None)
    }

    fn prepare_original_c_name_parts(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<crate::namespace::NsId>,
        explicit_element: Option<&[u8]>,
    ) -> Result<Option<OriginalCNameSelection>, Code> {
        let create = purpose.creates_entries();
        let dialect = self.native_invocation_dialect();
        let protocol = dialect
            .native_variable_name_protocol()
            .expect("C variable issuer");
        if !protocol.cache_precedes_string_getter() {
            self.native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
        }
        let mut parts = explicit_element
            .is_none()
            .then(|| {
                obj::native_variable_name::with_parsed(original, |cache| {
                    if cache.protocol != protocol {
                        return None;
                    }
                    cache.array.as_ref().map(|(root, element)| {
                        let element = match element {
                            NativeParsedVariableElement::Bytes(bytes) => {
                                Ok(tcl_core_types::c_string_extent(bytes).to_vec())
                            }
                            NativeParsedVariableElement::Object(object) => self
                                .native_string_bytes(&object.as_ptr())
                                .map(|bytes| bytes.to_vec()),
                        };
                        element.map(|element| (root.as_ptr(), element))
                    })
                })
            })
            .flatten()
            .flatten()
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
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
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            if let Some((root, element)) = protocol.parsed_array_parts(&original_bytes) {
                let root = Owned::fresh(obj::new_string_bytes(root));
                let element = match protocol.new_element_bytes(element) {
                    Some(bytes) => NativeParsedVariableElement::Bytes(bytes),
                    None => NativeParsedVariableElement::Object(Owned::fresh(
                        obj::new_string_bytes(element),
                    )),
                };
                obj::native_variable_name::install_parsed(
                    original,
                    NativeParsedVariableName {
                        protocol,
                        array: Some((root, element)),
                    },
                    dialect,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?;
                return self.prepare_original_c_name_in(original, purpose, namespace);
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
                .native_string_bytes(&root)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec();
            let kind = obj::obj_type_ptr(root);
            // SAFETY: a non-NULL type pointer names the live original descriptor.
            let free_hook = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
            if protocol.retires_before_simple_lookup(free_hook) {
                obj::discard_native_internal_representation(root)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
            }
            root_bytes
        };
        let selected = if let Some(index) = cached {
            if !self
                .frames
                .borrow_mut()
                .prepare_native_compiled_name_cell(index)
            {
                return Err(self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual original indexed variable cell")
                        .into(),
                ));
            }
            Ok(Some(crate::vars::NativeOriginalNameCell {
                compiled: Some(index),
            }))
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
            Err(error) if create => {
                let reason = match error {
                    crate::frame::VarError::NoSuchNamespace => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace
                    }
                    crate::frame::VarError::IsScalar => {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray
                    }
                    _ => return Err(self.original_c_lookup_var_error(original, purpose, error)),
                };
                return Err(self.original_c_variable_failure_for_object(
                    original,
                    purpose,
                    reason,
                    tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
                ));
            }
            Err(_) => return Ok(None),
        };
        if cached.is_none() {
            self.install_original_c_name_cache(root, selected.compiled)
                .map_err(|error| self.report_cmd_error(error.into()))?;
        }
        if create
            && selected.compiled.is_none()
            && (protocol.version() >= tcl_dialect::TclVersion::V8_5
                || purpose == NativeVariableNameLookupPurpose::Define)
        {
            if let Some(element) = &element {
                let retained_key = self.original_c_parsed_element_key(original);
                let prepared = crate::vars::prepare_original_c_element_cell(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    crate::vars::NativeOriginalNameScope {
                        current: self.current_ns.get(),
                        namespace,
                    },
                    &root_bytes,
                    element,
                    retained_key.as_ref().map(Owned::as_ptr),
                    purpose.creates_element_entries(),
                );
                prepared.map_err(|error| {
                    let reason = match error {
                        crate::frame::VarError::IsScalar if purpose == NativeVariableNameLookupPurpose::Define => tcl_syntax::naming::NativeVariableDiagnosticReason::ArrayElement,
                        crate::frame::VarError::IsScalar => tcl_syntax::naming::NativeVariableDiagnosticReason::NotArray,
                        crate::frame::VarError::NoSuchNamespace => tcl_syntax::naming::NativeVariableDiagnosticReason::MissingParentNamespace,
                        _ => return self.original_c_lookup_var_error(original, purpose, error),
                    };
                    self.original_c_variable_failure_for_object(original, purpose, reason,
                        tcl_syntax::naming::NativeVariableFailureSite::NameLookup)
                })?;
            }
        }
        Ok(Some(OriginalCNameSelection {
            root: root_bytes,
            element,
            compiled: selected.compiled,
            namespace,
        }))
    }

    /// Original VARIABLE/NSUPVAR object lookup and exact indexed alias binding.
    pub(crate) fn link_original_compiled_namespace_variable(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        slot: usize,
        declare: bool,
    ) -> Code {
        let purpose = NativeVariableNameLookupPurpose::Link;
        let selection = match self.prepare_original_c_name_in(original, purpose, Some(namespace)) {
            Ok(Some(selection)) => selection,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original namespace binding lookup")
                        .into(),
                )
            }
            Err(code) => return code,
        };
        let target = crate::vars::original_namespace_link_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            namespace,
            &selection.root,
            selection.element.clone(),
        );
        let mut link = match target {
            Ok(link) => link,
            Err(error) => return self.original_c_lookup_var_error(original, purpose, error),
        };
        if let Err(error) = self.prepare_upvar_target(&mut link) {
            return self.original_c_lookup_var_error(original, purpose, error);
        }
        if declare {
            match link.home {
                crate::frame::VarHome::Namespace(namespace) => self
                    .namespaces
                    .borrow_mut()
                    .var_table_mut(namespace)
                    .mark_namespace_declared(&link.name),
                crate::frame::VarHome::Frame(_) => {
                    return self.report_cmd_error(
                        ValueError::CommandProtocolUnavailable(
                            "namespace variable declaration target",
                        )
                        .into(),
                    )
                }
            }
        }
        self.settle_original_c_local_alias(Some(slot), &selection.root, link)
    }

    fn original_c_alias_error(&mut self, message: &[u8], code: &[u8]) -> Code {
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias diagnostic");
        self.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(
            tcl_cmd_core::CmdErrorDetails {
                message: message.to_vec(),
                string_result: protocol.diagnostic_string_protocol(),
                error_code: tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec()),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            },
        ))
    }

    fn settle_original_c_local_alias(
        &mut self,
        compiled: Option<usize>,
        name: &[u8],
        link: crate::frame::Link,
    ) -> Code {
        let local = if let Some(slot) = compiled {
            let name = self
                .frames
                .borrow()
                .compiled_slot_name(slot)
                .map(<[u8]>::to_vec);
            let Some(name) = name else {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original namespace alias local slot")
                        .into(),
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
        self.settle_original_c_alias_at(home, compiled, &local, link)
    }

    fn settle_original_c_alias_at(
        &mut self,
        home: crate::frame::VarHome,
        compiled: Option<usize>,
        local: &[u8],
        link: crate::frame::Link,
    ) -> Code {
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
        let bound = match home {
            crate::frame::VarHome::Frame(_) => {
                if let Some(slot) = compiled {
                    self.frames
                        .borrow_mut()
                        .bind_original_compiled_alias(slot, link, traced)
                } else {
                    self.frames
                        .borrow_mut()
                        .bind_original_local_alias(local, link, traced)
                }
            }
            crate::frame::VarHome::Namespace(namespace) => self
                .namespaces
                .borrow_mut()
                .var_table_mut(namespace)
                .bind_original_alias(local, link, traced),
        };
        match bound {
            Ok(()) => Code::Ok,
            Err(crate::frame::NativeCompiledAliasError::Unavailable) => self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("original namespace alias local cell")
                    .into(),
            ),
            Err(crate::frame::NativeCompiledAliasError::SelfLink) => self
                .original_c_alias_error(b"can't upvar from variable to itself", b"TCL UPVAR SELF"),
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
                self.original_c_alias_error(
                    &message,
                    if error == crate::frame::NativeCompiledAliasError::Traced {
                        b"TCL UPVAR TRACED"
                    } else {
                        b"TCL UPVAR EXISTS"
                    },
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
                )
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
                )
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
                )
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
    pub(crate) fn link_original_c_namespace_objects(
        &mut self,
        original: *mut TclObj,
        namespace: crate::namespace::NsId,
        local: *mut TclObj,
    ) -> Code {
        let selected = match self.prepare_original_c_name_in(
            original,
            NativeVariableNameLookupPurpose::Link,
            Some(namespace),
        ) {
            Ok(Some(selected)) => selected,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original namespace upvar target")
                        .into(),
                )
            }
            Err(code) => return code,
        };
        let target = crate::vars::original_namespace_link_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            namespace,
            &selected.root,
            selected.element,
        );
        let mut target = match target {
            Ok(target) => target,
            Err(error) => {
                return self.original_c_lookup_var_error(
                    original,
                    NativeVariableNameLookupPurpose::Link,
                    error,
                )
            }
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            return self.original_c_lookup_var_error(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
            );
        }
        self.bind_original_c_alias_local(local, target)
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
                )
            }
            Err(code) => return code,
        };
        let target = crate::vars::original_namespace_link_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            namespace,
            &selected.root,
            selected.element.clone(),
        );
        let mut target = match target {
            Ok(target) => target,
            Err(error) => return self.original_c_lookup_var_error(original, purpose, error),
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
        self.bind_original_c_alias_local(local_original, target)
    }

    /// ObjMakeUpvar's local side uses simple-name lookup, without converting
    /// the caller's original object to a parsed or indexed variable name.
    pub(crate) fn bind_original_c_alias_local(
        &mut self,
        original: *mut TclObj,
        target: crate::frame::Link,
    ) -> Code {
        let protocol = self
            .native_c_variable_name_protocol()
            .expect("actual C alias local");
        let bytes = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if self.upvar_would_invert(&target, &bytes) {
            let mut message = b"bad variable name \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
            message.extend_from_slice(
                b"\": can't create namespace variable that refers to procedure variable",
            );
            return self.original_c_alias_error(&message, b"TCL UPVAR INVERTED");
        }
        if protocol.alias_local_is_element(&bytes) {
            let mut message = b"bad variable name \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
            message.extend_from_slice(
                b"\": can't create a scalar variable that looks like an array element",
            );
            return self.original_c_alias_error(&message, b"TCL UPVAR LOCAL_ELEMENT");
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
                return self.original_c_alias_error(&message, &code);
            }
            Err(error) => {
                return self.original_c_lookup_var_error(
                    original,
                    NativeVariableNameLookupPurpose::Link,
                    error,
                )
            }
        };
        self.settle_original_c_alias_at(selected.home, selected.compiled, &selected.name, target)
    }

    pub(crate) fn link_original_c_upvar_objects(
        &mut self,
        original: *mut TclObj,
        level: usize,
        local: *mut TclObj,
    ) -> Code {
        let mut target = match self.prepare_original_c_link_target(original, level) {
            Ok(Some(target)) => target,
            Ok(None) => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original upvar target cell").into(),
                )
            }
            Err(code) => return code,
        };
        if let Err(error) = self.prepare_upvar_target(&mut target) {
            return self.original_c_lookup_var_error(
                original,
                NativeVariableNameLookupPurpose::Link,
                error,
            );
        }
        self.bind_original_c_alias_local(local, target)
    }

    pub(crate) fn prepare_original_c_link_target(
        &mut self,
        original: *mut TclObj,
        level: usize,
    ) -> Result<Option<crate::frame::Link>, Code> {
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
            if let Some(index) = selection.compiled {
                let selected = crate::vars::original_indexed_link_target(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    index,
                    selection.element,
                );
                return selected.map(Some).map_err(|error| {
                    self.original_c_lookup_var_error(
                        original,
                        NativeVariableNameLookupPurpose::Link,
                        error,
                    )
                });
            }
            Ok(crate::vars::link_target_at(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                &selection.root,
                selection.element,
                level,
            ))
        })();
        self.current_ns.set(saved_namespace);
        self.frames.borrow_mut().set_active_level(saved);
        result
    }

    fn original_c_variable_failure_for_object(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        let bytes = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error(error.into()),
        };
        self.original_c_variable_failure(&bytes, purpose, reason, site)
    }

    fn original_c_variable_failure(
        &mut self,
        original: &[u8],
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        self.original_c_variable_failure_input(
            tcl_syntax::naming::NativeVariableInputForm::Combined(original),
            purpose,
            reason,
            site,
        )
    }

    fn original_c_variable_failure_input(
        &mut self,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        purpose: NativeVariableNameLookupPurpose,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Code {
        if !purpose.leaves_error_message() {
            return Code::Error;
        }
        use tcl_syntax::naming::{
            report_native_variable_diagnostic_at, NativeVariableDiagnosticOperation as Operation,
        };
        let operation = match purpose {
            NativeVariableNameLookupPurpose::Read | NativeVariableNameLookupPurpose::Exists | NativeVariableNameLookupPurpose::Array => Operation::Read,
            NativeVariableNameLookupPurpose::Unset
            | NativeVariableNameLookupPurpose::QuietUnset => Operation::Unset,
            NativeVariableNameLookupPurpose::Write
            | NativeVariableNameLookupPurpose::QuietWrite
            | NativeVariableNameLookupPurpose::Link
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
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("original variable lookup diagnostic")
                        .into(),
                );
            }
        };
        let mut message = format!("can't {} \"", purpose.diagnostic_verb()).into_bytes();
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
                        let words = words.iter().map(Vec::as_slice).collect::<Vec<_>>();
                        tcl_cmd_core::CmdErrorCodeUpdate::Set(super::error_code_list(&words))
                    },
                ),
                error_info: None,
                error_line: None,
                primitive_getter: None,
            },
        ))
    }

    pub(super) fn capture_original_c_variable_report(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        let selected = self.prepare_original_c_name(original, purpose.creates_entries())?;
        Ok(self
            .capture_original_c_selection(original, &selected, purpose)?
            .map(|(receiver, home)| OriginalCVariableCapture {
                receiver,
                home,
                root: selected.root,
                element: selected.element,
            }))
    }

    /// Capture the two original operands of a selected array stack opcode.
    /// C85+ entry creation retains the same original index object.
    pub(super) fn capture_original_c_parts_report(
        &mut self,
        root: *mut TclObj,
        element: Option<*mut TclObj>,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Option<OriginalCVariableCapture>, Code> {
        let mut selected = self.prepare_original_c_name(root, purpose.creates_entries())?;
        if selected.element.is_some() {
            return Err(self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("compiled separate base-name geometry")
                    .into(),
            ));
        }
        selected.element = element
            .map(|element| {
                ValueOps::native_string_bytes(self, &element).map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let captured = self.capture_original_c_selection(root, &selected, purpose)?;
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
        let captured = if let Some(namespace) = selected.namespace {
            crate::vars::capture_original_namespace_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                namespace,
                &selected.root,
                selected.element.clone(),
                purpose.creates_entries(),
            )
        } else if let Some(index) = selected.compiled {
            crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                index,
                selected.element.clone(),
                purpose.creates_entries(),
            )
        } else {
            let receiver = if purpose.creates_entries() {
                crate::vars::capture_variable_receiver(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    self.current_ns.get(),
                    &selected.root,
                    selected.element.as_deref(),
                )
                .map(Some)
            } else {
                crate::vars::capture_get_variable_receiver(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    self.current_ns.get(),
                    &selected.root,
                    selected.element.as_deref(),
                )
            };
            receiver.map(|receiver| {
                receiver.map(|receiver| {
                    let home = crate::vars::trace_home(
                        &self.frames.borrow(),
                        &self.namespaces.borrow(),
                        self.current_ns.get(),
                        &selected.root,
                    );
                    (receiver, home)
                })
            })
        };
        let captured =
            captured.map_err(|error| self.original_c_lookup_var_error(original, purpose, error))?;
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

    fn original_c_lookup_var_error(
        &mut self,
        original: *mut TclObj,
        purpose: NativeVariableNameLookupPurpose,
        error: crate::frame::VarError,
    ) -> Code {
        use crate::frame::VarError;
        use tcl_syntax::naming::{
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
        };
        if !purpose.leaves_error_message() && !matches!(error, VarError::NameProtocolUnavailable) {
            self.traces.borrow_mut().pending_err.take();
            return Code::Error;
        }
        let reason = match error {
            VarError::IsScalar => Reason::NotArray,
            VarError::IsArray => Reason::IsArray,
            VarError::NoSuchNamespace => Reason::MissingParentNamespace,
            VarError::DeletedArray => Reason::DetachedElement,
            VarError::DeletedNamespace => Reason::RetiredNamespace,
            VarError::IsConstant => Reason::Constant,
            VarError::NameProtocolUnavailable => {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual indexed variable receiver")
                        .into(),
                );
            }
            VarError::TraceError => {
                let reason = self
                    .traces
                    .borrow_mut()
                    .pending_err
                    .take()
                    .unwrap_or_default();
                let bytes = match self.native_string_bytes(&original) {
                    Ok(bytes) => bytes.to_vec(),
                    Err(error) => return self.report_cmd_error(error.into()),
                };
                return self.var_trace_error(
                    &bytes,
                    if purpose == NativeVariableNameLookupPurpose::Read {
                        b"read"
                    } else {
                        b"write"
                    },
                    &reason,
                );
            }
        };
        let bytes = match self.native_string_bytes(&original) {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let site = match purpose {
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
        };
        self.original_c_variable_failure(&bytes, purpose, reason, site)
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
        let cell = crate::cmd_trace::VarTraceScope::cell(
            &home.base,
            access.match_elem.as_deref(),
            home.ns,
            home.level,
            home.binding_id,
        );
        let active = self.active_var_trace_scopes.borrow();
        if active.contains(&cell) {
            return false;
        }
        let array_active = access.match_elem.is_some() && active.contains(&cell.array());
        let traces = self.traces.borrow();
        let registered = traces.traces.iter().any(|trace| {
            (trace.elem.is_some() || (access.whole_array && !array_active))
                && crate::cmd_trace::matches(
                    trace,
                    &home.base,
                    access.match_elem.as_deref(),
                    operation,
                    home.ns,
                    home.level,
                    home.binding_id,
                )
        });
        let intrinsic = access.whole_array
            && !array_active
            && ((access.match_elem.is_none()
                && matches!(operation, b"read" | b"unset")
                && self.native_error_variable_at(home).is_some())
                || (home.ns == Some(super::GLOBAL)
                    && home.level.is_none()
                    && self
                        .native_invocation_dialect()
                        .double_string_policy()
                        .and_then(|policy| policy.precision_variable())
                        .is_some_and(|name| {
                            home.base == name.trim_start_matches("::").as_bytes()
                        })
                    && matches!(operation, b"read" | b"write")));
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
        let purpose = NativeVariableNameLookupPurpose::Read;
        let selected = self.prepare_original_c_name(lookup, create)?;
        let capture_purpose = if create {
            NativeVariableNameLookupPurpose::Link
        } else {
            purpose
        };
        let captured = self.capture_original_c_selection(lookup, &selected, capture_purpose)?;
        let Some((receiver, home)) = captured else {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec();
            return Err(self.original_c_variable_failure(
                &bytes,
                purpose,
                tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable,
                tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
            ));
        };
        if self.original_variable_trace_requires_name(&home, selected.element.as_deref(), b"read") {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec();
            let access = self.trace_access(
                &bytes,
                &selected.root,
                selected.element.as_deref(),
                &home,
                false,
            );
            if self.fire_var_trace_resolved(&home, &access, b"read") {
                return Err(self.original_c_lookup_var_error(
                    original,
                    purpose,
                    crate::frame::VarError::TraceError,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        receiver
            .read()
            .map_err(|error| self.original_c_lookup_var_error(original, purpose, error))?
            .ok_or_else(|| {
                self.original_c_variable_failure_for_object(
                    original,
                    purpose,
                    if selected.element.is_some() && receiver.is_array() {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchElement
                    } else {
                        tcl_syntax::naming::NativeVariableDiagnosticReason::NoSuchVariable
                    },
                    tcl_syntax::naming::NativeVariableFailureSite::ValueRead,
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
        let selected = self
            .prepare_original_c_name_for(original, purpose)?
            .ok_or_else(|| {
                self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual original write selection")
                        .into(),
                )
            })?;
        self.assign_prepared_original_c_selection(
            original,
            value,
            publish_result,
            purpose,
            &selected,
        )
    }

    fn assign_prepared_original_c_selection(
        &mut self,
        original: *mut TclObj,
        value: *mut TclObj,
        publish_result: bool,
        purpose: NativeVariableNameLookupPurpose,
        selected: &OriginalCNameSelection,
    ) -> Result<(), Code> {
        self.assign_prepared_original_c_selection_reported(
            original,
            original,
            value,
            publish_result,
            purpose,
            selected,
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
        let (receiver, home) = self
            .capture_original_c_selection(lookup, selected, purpose)?
            .ok_or_else(|| {
                self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual variable store receiver").into(),
                )
            })?;
        let protective_value = publish_result.then(|| Owned::retain(value));
        receiver
            .store(value)
            .map_err(|error| self.original_c_lookup_var_error(original, purpose, error))?;
        if self.original_variable_trace_requires_name(&home, selected.element.as_deref(), b"write")
        {
            let bytes = self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec();
            let access = self.trace_access(
                &bytes,
                &selected.root,
                selected.element.as_deref(),
                &home,
                false,
            );
            if self.fire_var_trace_resolved_with_errors(
                &home,
                &access,
                b"write",
                purpose.leaves_error_message(),
            ) {
                return Err(self.original_c_lookup_var_error(
                    original,
                    purpose,
                    crate::frame::VarError::TraceError,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        if publish_result {
            match receiver
                .read()
                .map_err(|error| self.original_c_lookup_var_error(original, purpose, error))?
            {
                Some(value) => self.set_result(value),
                None => self.set_result_bytes(b""),
            }
        }
        drop(protective_value);
        Ok(())
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
        self.assign_original_c_selection(
            original,
            value,
            false,
            NativeVariableNameLookupPurpose::Write,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Unset removes the visible binding; the retained original cache above
        // still identifies the same compiled slot rather than a replacement.
        assert_eq!(
            interp.trace_identity(b"x").binding_id,
            if case == 4 { None } else { before_cell }
        );
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
            assert_eq!(
                super::super::obj_bytes(interp.var_get(b"x").unwrap()),
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
}
