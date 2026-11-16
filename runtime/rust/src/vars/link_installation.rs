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

//! Installing links for `global`, `variable` and `upvar`: the published link and its displaced owners.

use super::original_name_cells::OriginalAliasTargetObservation;
use super::{
    current_home, prepare_selected_upvar_target, prepare_upvar_target, table, table_mut,
    unqualified_home, NativeOriginalNameCell,
};
use crate::frame::{FrameStack, Link, VarError, VarHome};
use crate::namespace::{Namespaces, NsId, GLOBAL};
use tcl_runtime_api::FrameLinkOrigin;

// link installation (global / variable / upvar)

/// Install a link from the current context's `local` name to `target`, unless it
/// would be a self-link (already that exact cell — the no-op `global`/`variable`
/// at namespace scope produces).
/// An adopted generic link and only its actual displaced owning parts.
/// The caller finishes it after frame and namespace borrows end.
pub(crate) struct PublishedVariableLink {
    retired: crate::frame::RetiredVariableParts,
    local: Option<NativeOriginalNameCell>,
    target: Option<OriginalAliasTargetObservation>,
    pub(super) link: Option<Link>,
    error: Option<VarError>,
}

impl PublishedVariableLink {
    pub(super) fn empty() -> Self {
        Self {
            retired: Default::default(),
            local: None,
            target: None,
            link: None,
            error: None,
        }
    }
    fn declined(link: Link, error: VarError) -> Self {
        Self {
            retired: crate::frame::RetiredVariableParts::from_original_link(link),
            error: Some(error),
            ..Self::empty()
        }
    }
    pub(super) fn check(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        currency.ensure_current_or_refuse()?;
        if let Some(local) = &self.local {
            local.check(interpreter, currency)?;
        }
        if let (Some(target), Some(link)) = (&self.target, &self.link) {
            target.check(interpreter, link, currency)?;
        }
        if let Some(link) = &self.link {
            link.check_stored_roles().map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        }
        currency.ensure_current_or_refuse().map_err(Into::into)
    }
    pub(crate) fn finish(
        mut self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), crate::frame::JimLinkStoreError> {
        let mut initial = self.check(interpreter, currency);
        // An unavailable actual role remains a structured cause before cleanup.
        if self.error == Some(VarError::NameProtocolUnavailable) && initial.is_ok() {
            initial = Err(tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original generic alias publication",
                    ),
                ),
            ));
        }
        let retired = std::mem::take(&mut self.retired);
        let cleanup = retired.finish(|| self.check(interpreter, currency));
        initial.map_err(crate::frame::JimLinkStoreError::Command)?;
        cleanup.map_err(crate::frame::JimLinkStoreError::Command)?;
        if let Some(error) = self.error.take() {
            return Err(error.into());
        }
        Ok(())
    }
}

fn link_local_with_origin(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    local: &[u8],
    target: Link,
    origin: FrameLinkOrigin,
) -> PublishedVariableLink {
    let Some(protocol) = ns.variable_name_protocol else {
        return PublishedVariableLink::declined(target, VarError::NameProtocolUnavailable);
    };
    let input = protocol.variable_root_input(local);
    let (here, local) = unqualified_home(
        ns,
        current_home(frames, current_ns),
        input.selected(),
        protocol,
    );
    if target.home == here && target.elem.is_none() && target.name == local {
        return PublishedVariableLink {
            retired: crate::frame::RetiredVariableParts::from_original_link(target),
            ..PublishedVariableLink::empty()
        };
    }
    publish_link_at(frames, ns, here, &local, target, origin)
}

fn publish_link_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    here: VarHome,
    local: &[u8],
    target: Link,
    origin: FrameLinkOrigin,
) -> PublishedVariableLink {
    let observed_target =
        if target.native_scalar_entry.is_some() || target.native_element_entry.is_some() {
            match OriginalAliasTargetObservation::capture(frames, ns, &target) {
                Ok(target) => Some(target),
                Err(error) => return PublishedVariableLink::declined(target, error),
            }
        } else {
            None
        };
    let selected = target.clone(); // allocation/role metadata only
    let retired = table_mut(frames, ns, here).publish_link_with_origin(local, target, origin);
    let observed_local = table(frames, ns, here)
        .ok_or(VarError::NameProtocolUnavailable)
        .and_then(|table| table.observe_original_name_cell(local))
        .and_then(|(cell, link)| {
            NativeOriginalNameCell::capture(frames, ns, here, local.to_vec(), None, cell, link)
        });
    match observed_local {
        Ok(local) => PublishedVariableLink {
            retired,
            local: Some(local),
            target: observed_target,
            link: Some(selected),
            error: None,
        },
        Err(error) => PublishedVariableLink {
            retired,
            target: observed_target,
            link: Some(selected),
            error: Some(error),
            ..PublishedVariableLink::empty()
        },
    }
}

#[cfg(test)]
pub(crate) fn make_variable(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    tail: &[u8],
) {
    drop(publish_variable_mapped(
        frames, ns, current_ns, target_ns, tail, tail,
    ));
}

pub(crate) fn publish_variable_mapped(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    local: &[u8],
    name: &[u8],
) -> PublishedVariableLink {
    let mut target = declared_namespace_variable(ns, target_ns, name);
    // Check the existing no-op before acquiring a genuine new alias reference.
    if let Some(protocol) = ns.variable_name_protocol {
        let input = protocol.variable_root_input(local);
        let (here, local) = unqualified_home(
            ns,
            current_home(frames, current_ns),
            input.selected(),
            protocol,
        );
        if target.home == here && target.name == local {
            return PublishedVariableLink::empty();
        }
    }
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    link_local_with_origin(
        frames,
        ns,
        current_ns,
        local,
        target,
        FrameLinkOrigin::Ordinary,
    )
}

/// Select the actual compiled resolver's declared namespace receiver without
/// taking its new alias role before the selected old-target retirement.
pub(crate) fn select_tcloo_compiled_variable(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target_ns: NsId,
    name: &[u8],
) -> Result<(Link, NativeOriginalNameCell), crate::frame::JimLinkStoreError> {
    let mut target = declared_namespace_variable(ns, target_ns, name);
    let (cell, link) = ns.var_table(target_ns).observe_original_name_cell(name)?;
    let source = NativeOriginalNameCell::capture(
        frames,
        ns,
        VarHome::Namespace(target_ns),
        name.to_vec(),
        None,
        cell,
        link,
    )?;
    source.check_selected(frames, ns)?;
    prepare_selected_upvar_target(frames, ns, &mut target)?;
    Ok((target, source))
}

fn declared_namespace_variable(ns: &mut Namespaces, target_ns: NsId, name: &[u8]) -> Link {
    if ns.var_table(target_ns).cell(name).is_none() {
        ns.var_table_mut(target_ns).insert_link(
            name,
            Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: VarHome::Namespace(target_ns),
                name: name.to_vec(),
                elem: None,
            },
        );
    }
    ns.var_table_mut(target_ns)
        .mark_undefined_root(name, VarHome::Namespace(target_ns));
    ns.var_table_mut(target_ns).mark_namespace_declared(name);
    Link {
        original_jim_target: None,
        native_scalar_entry: None,
        native_element_entry: None,
        array_identity: None,
        array_cell: None,
        home: VarHome::Namespace(target_ns),
        name: name.to_vec(),
        elem: None,
    }
}

#[cfg(test)]
pub(crate) fn make_upvar(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target: Link,
    local: &[u8],
) {
    drop(publish_upvar(frames, ns, current_ns, target, local));
}

pub(crate) fn publish_upvar(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    mut target: Link,
    local: &[u8],
) -> PublishedVariableLink {
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    if target.home == VarHome::Frame(0) {
        target.home = VarHome::Namespace(GLOBAL);
    }
    link_local_with_origin(
        frames,
        ns,
        current_ns,
        local,
        target,
        FrameLinkOrigin::Ordinary,
    )
}

#[cfg(test)]
pub(crate) fn make_upvar_in(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    home_ns: NsId,
    tail: &[u8],
    target: Link,
) {
    drop(publish_upvar_in(frames, ns, home_ns, tail, target));
}

pub(crate) fn publish_upvar_in(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    home_ns: NsId,
    tail: &[u8],
    mut target: Link,
) -> PublishedVariableLink {
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    if target.home == VarHome::Frame(0) {
        target.home = VarHome::Namespace(GLOBAL);
    }
    publish_link_at(
        frames,
        ns,
        VarHome::Namespace(home_ns),
        tail,
        target,
        FrameLinkOrigin::Ordinary,
    )
}
