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

//! Original C array members: selecting the member before retirement stages and materialising the element cell.

#[cfg(test)]
use super::follow_links;
#[cfg(test)]
use super::original_name_cells::NativeOriginalNameScope;
use super::trace_homes::TraceHome;
#[cfg(test)]
use super::Resolved;
use super::{
    entered_container, follow_selected_links_observed, table, table_mut, NativeOriginalNameCell,
    OriginalCLinkSelection, Place,
};
use crate::frame::{FrameStack, Link, VarError, VarHome};
use crate::namespace::Namespaces;
use crate::obj::TclObj;
#[cfg(test)]
use crate::vars::original_name_cells::classify_original_name;
use tcl_runtime_api::VarId;

/// The selected cell and actual table/frame birth survive the reached array
/// and search retirement stages without retaining their Native roles.
pub(crate) struct OriginalArrayMemberPreparation {
    selected: crate::frame::PreparedArrayMember,
    pub(super) home: VarHome,
    pub(super) root: Vec<u8>,
    pub(super) table: entered_container::TableObservation,
    pub(super) links: Vec<OriginalCLinkSelection>,
    element: Vec<u8>,
    retained_array: bool,
}

/// The actual target chosen before metadata retirement, without native holds.
pub(crate) struct OriginalArrayMemberSelection {
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
    observation: crate::frame::VariablePublicationObservation,
    epoch: u64,
    pub(super) home: VarHome,
    pub(super) root: Vec<u8>,
    element: Vec<u8>,
    pub(super) table: entered_container::TableObservation,
    pub(super) links: Vec<OriginalCLinkSelection>,
    retained_array: bool,
    member: Option<VarId>,
}

impl OriginalArrayMemberSelection {
    pub(super) fn check(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        let physical = (|| {
            entered_container::same_table(interpreter, self.home, &self.table)?;
            for link in &self.links {
                link.check(interpreter)?;
            }
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            if !table(&frames, &namespaces, self.home)
                .is_some_and(|table| self.observation.matches_root(table, &self.root))
            {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original materialised root slot",
                ));
            }
            self.observation
                .check_array_current(self.epoch)
                .and_then(|()| {
                    self.member.map_or(Ok(()), |member| {
                        self.observation.check_member_current(&self.element, member)
                    })
                })
        })();
        physical.map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })
    }
    pub(crate) fn capture_receiver(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(crate::frame::VariableReceiver, TraceHome), tcl_cmd_core::CmdError> {
        self.check(interpreter, currency)?;
        let member = self.member.ok_or_else(|| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "unissued original array member",
                    ),
                ),
            )
        })?;
        let receiver = self
            .observation
            .capture_array_receiver(
                self.epoch,
                member,
                self.element.clone(),
                self.root.clone(),
                self.retained_array,
            )
            .map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        let (ns, level) = match self.home {
            VarHome::Namespace(namespace) => (Some(namespace), None),
            VarHome::Frame(level) => (None, Some(level)),
        };
        let home = TraceHome {
            binding_id: receiver.binding_id(),
            selected_member: receiver.trace_member(),
            ns,
            level,
            base: self.root.clone(),
            link_elem: Some(self.element.clone()),
        };
        self.check(interpreter, currency)?;
        Ok((receiver, home))
    }
    pub(crate) fn capture_link_target(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Link, tcl_cmd_core::CmdError> {
        self.check(interpreter, currency)?;
        let member = self.member.ok_or_else(|| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "unissued original array member",
                    ),
                ),
            )
        })?;
        let array = self
            .observation
            .capture_array_cell(self.epoch, &self.element, member)
            .map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        let target = Link {
            original_jim_target: None,
            native_scalar_entry: None,
            native_element_entry: None,
            array_identity: Some(array.identity()),
            array_cell: Some(array),
            home: self.home,
            name: self.root.clone(),
            elem: Some(self.element.clone()),
        };
        self.check(interpreter, currency)?;
        Ok(target)
    }
}

impl OriginalArrayMemberPreparation {
    #[cfg(test)]
    #[cfg(debug_assertions)]
    pub(crate) fn finish(
        self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<OriginalArrayMemberSelection, tcl_cmd_core::CmdError> {
        self.finish_using(interpreter, currency, None)
    }

    pub(crate) fn finish_using(
        self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
        stage: Option<&dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
    ) -> Result<OriginalArrayMemberSelection, tcl_cmd_core::CmdError> {
        let observation = self.selected.observation();
        let epoch = self.selected.array_epoch();
        let home = self.home;
        let root = self.root;
        let original_table = self.table;
        let links = self.links;
        let selected = self.selected;
        let check = || {
            if let Some(check) = stage {
                check()?;
            }
            currency
                .ensure_current_or_refuse()
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
            let physical = entered_container::same_table(interpreter, home, &original_table)
                .and_then(|()| {
                    for link in &links {
                        link.check(interpreter)?;
                    }
                    let frames = interpreter.frames.borrow();
                    let namespaces = interpreter.namespaces();
                    let table = table(&frames, &namespaces, home).ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original member preparation table",
                        ),
                    )?;
                    if observation.matches_root(table, &root) {
                        Ok(())
                    } else {
                        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original member preparation root slot",
                        ))
                    }
                });
            physical.map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })
        };
        selected.finish(check, |error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })?;
        let member = observation.member_identity(&self.element);
        Ok(OriginalArrayMemberSelection {
            currency: currency.clone(),
            observation,
            epoch,
            home,
            root,
            table: original_table,
            links,
            element: self.element,
            retained_array: self.retained_array,
            member,
        })
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
pub(crate) struct OriginalElementOptions {
    pub(crate) original: Option<*mut TclObj>,
    pub(crate) create_element: bool,
}

/// Same original classifier and link walk, returning before any old owner is
/// released so the actual caller can leave its frame/namespace borrows first.
#[cfg(test)]
#[cfg(debug_assertions)]
pub(crate) fn capture_original_c_element_materialisation(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    scope: NativeOriginalNameScope,
    root: &[u8],
    element: &[u8],
    options: OriginalElementOptions,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    let OriginalElementOptions {
        original,
        create_element,
    } = options;
    let NativeOriginalNameScope { current, namespace } = scope;
    let place = match classify_original_name(frames, ns, current, root, namespace) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error.into()),
    };
    capture_original_c_element_materialisation_using(
        frames,
        ns,
        place,
        element,
        original,
        create_element,
        protocol,
    )
}

pub(crate) fn capture_original_c_selected_element_materialisation(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    selected.check_selected(frames, ns)?;
    capture_original_c_element_materialisation_using(
        frames,
        ns,
        selected.place(),
        element,
        original,
        create_element,
        protocol,
    )
}

fn capture_original_c_element_materialisation_using(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    place: Place,
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    let mut links = Vec::new();
    let place = follow_selected_links_observed(frames, ns, place, false, Some(&mut links))?;
    if place.elem.is_some() {
        return Err(VarError::IsScalar.into());
    }
    let observation = entered_container::TableObservation::capture(frames, ns, place.home)
        .ok_or(VarError::NameProtocolUnavailable)?;
    let member = create_element.then_some(element);
    let original = protocol
        .element_table_retains_original()
        .then_some(original)
        .flatten();
    let selected = if let Some(array) = &place.array_cell {
        array.prepare_original_array_member(member, original)?
    } else if let Some(entry) = &place.scalar_entry {
        entry.prepare_original_array_member(member, original)?
    } else {
        table_mut(frames, ns, place.home).prepare_original_array_member(
            &place.name,
            member,
            original,
        )?
    };
    Ok(OriginalArrayMemberPreparation {
        selected,
        home: place.home,
        root: place.name,
        table: observation,
        links,
        element: element.to_vec(),
        retained_array: place.array_cell.is_some(),
    })
}

/// Existing neutral fixture preparation retains its independent caller contract.
#[cfg(test)]
/// The actual array hash owns the selected original element object at birth.
#[cfg(debug_assertions)]
pub(crate) fn prepare_original_c_element_cell(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    scope: NativeOriginalNameScope,
    root: &[u8],
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
) -> Result<(), VarError> {
    let NativeOriginalNameScope { current, namespace } = scope;
    let place = match classify_original_name(frames, ns, current, root, namespace) {
        Resolved::Place(place) => {
            follow_links(frames, ns, place).map_or_else(Resolved::Error, Resolved::Place)
        }
        error => error,
    };
    let place = match place {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    if let Some(array) = place.array_cell {
        return if create_element {
            array.prepare_original_native_element(element, original)
        } else {
            Ok(())
        };
    }
    if let Some(entry) = place.scalar_entry {
        return if create_element {
            entry.prepare_original_element(element, original)
        } else {
            entry.ensure_array()
        };
    }
    if !create_element {
        return table_mut(frames, ns, place.home).ensure_array(&place.name);
    }
    table_mut(frames, ns, place.home).prepare_original_native_element(
        &place.name,
        element,
        original,
    )
}
