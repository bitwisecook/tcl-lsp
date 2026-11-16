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

//! Settling a local alias onto its target: the alias diagnostics and the publication with its origin.

use super::super::{Code, Interp};
use super::{OriginalCLinkTarget, OriginalCNameStage};
use tcl_syntax::value::ValueError;

impl Interp {
    pub(super) fn original_c_alias_error_using(
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

    pub(super) fn original_c_alias_diagnostic_using(
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

    pub(super) fn settle_original_c_local_alias(
        &mut self,
        compiled: Option<usize>,
        name: &[u8],
        target: OriginalCLinkTarget,
    ) -> Code {
        self.settle_original_c_local_alias_using(compiled, name, target, None)
    }

    pub(super) fn settle_original_c_local_alias_using(
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

    pub(super) fn settle_original_c_alias_at_using(
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

    pub(super) fn settle_original_c_alias_at_with_origin(
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
}
