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

//! Linking through original names: compiled namespace variables, `variable`, `global` and `upvar` targets, and alias locals.

use super::super::{Code, Interp};
use super::{OriginalCLinkTarget, OriginalCNameStage};
use crate::obj;
use crate::obj::{Owned, TclObj};
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;
use tcl_syntax::value::{ValueError, ValueOps};

impl Interp {
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
        let operation =
            match super::super::native_operation_currency::NativeOperationScope::enter(self) {
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
        let operation =
            match super::super::native_operation_currency::NativeOperationScope::enter(self) {
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
                let code = super::super::error_code_list(&[b"TCL", b"LOOKUP", b"VARNAME", name]);
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
        let operation = super::super::native_operation_currency::NativeOperationScope::enter(self)
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
}
