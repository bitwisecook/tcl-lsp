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

//! Formal cells and generated-code slots of the active frame.

use super::materialisation::{PreparedAliasReplacement, PreparedVariableReplacement};
#[cfg(test)]
use super::VarError;
use super::{materialisation, FrameStack, Link, NativeCompiledAliasError, Var};
use crate::obj;
use crate::obj::TclObj;
use std::cell::Ref;
use tcl_runtime_api::{FrameLinkOrigin, VarId};

impl FrameStack {
    /// Bind a generated-code slot to `name`'s cell in the active frame,
    /// reserving that cell if the name has none yet.
    pub(crate) fn install_formal_cells(
        &mut self,
        names: &[Vec<u8>],
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
    ) {
        if !protocol.has_indexed_locals() {
            return;
        }
        let index = self.current_frame_index();
        let frame = &mut self.frames[index];
        frame.table.compiled_protocol = Some(protocol);
        for name in names {
            let cell = frame.table.declare_compiled_cell(name);
            frame.compiled_slots.push(Some(cell));
        }
    }

    pub(crate) fn prepare_formal_cell_store(
        &mut self,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<PreparedVariableReplacement, tcl_syntax::value::ValueError> {
        let (frame, cell) = self.compiled_cell(slot).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original formal cell"),
        )?;
        // A pending allocation has no cell +1 to supply ordinary formal-store
        // authority. Refuse before acquiring the incoming stored reference or
        // extracting any original member/search metadata.
        self.frames[frame].table.cells[cell]
            .contents
            .borrow()
            .check_owned_value_role()?;
        // The same actual incoming stored role is acquired before reached
        // metadata retirement; it is not an allocation-safety reference.
        unsafe { obj::incr_ref_count(value) };
        let table = &self.frames[frame].table;
        Ok(PreparedVariableReplacement::capture(
            &table.cells[cell].contents,
            Var::Scalar(value),
            table.hash_recipe.get(),
        ))
    }

    pub(crate) fn original_formal_cell_is_current(
        &self,
        slot: usize,
        original: &materialisation::VariablePublicationObservation,
    ) -> bool {
        self.compiled_cell(slot).is_some_and(|(frame, cell)| {
            original.matches(&self.frames[frame].table.cells[cell].contents)
        })
    }

    #[cfg(test)]
    pub(crate) fn store_formal_cell(
        &mut self,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let prepared = self
            .prepare_formal_cell_store(slot, value)
            .map_err(|_| VarError::NameProtocolUnavailable)?;
        let old = prepared
            .finish(|| Ok::<_, tcl_syntax::value::ValueError>(()))
            .map_err(|_| VarError::NameProtocolUnavailable)?;
        drop(old);
        Ok(())
    }

    pub(crate) fn bind_compiled_slot(&mut self, slot: usize, name: &[u8]) {
        let Some(i) = self.index_of_level(self.active_level) else {
            return;
        };
        let frame = &mut self.frames[i];
        if frame.compiled_slots.get(slot).is_some_and(Option::is_some) {
            return;
        }
        let cell = frame
            .table
            .lookup_slot(name)
            .unwrap_or_else(|| frame.table.declare_compiled_cell(name));
        if frame.compiled_slots.len() <= slot {
            frame.compiled_slots.resize(slot + 1, None);
        }
        frame.compiled_slots[slot] = Some(cell);
    }

    /// The active frame's table cell a generated-code slot is bound to.
    pub(super) fn compiled_cell(&self, slot: usize) -> Option<(usize, usize)> {
        let i = self.index_of_level(self.active_level)?;
        let cell = (*self.frames[i].compiled_slots.get(slot)?)?;
        Some((i, cell))
    }

    pub(crate) fn prepare_original_compiled_alias(
        &mut self,
        slot: usize,
        link: &Link,
        traced: bool,
        origin: FrameLinkOrigin,
    ) -> Result<PreparedAliasReplacement, NativeCompiledAliasError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        PreparedAliasReplacement::capture(
            &self.frames[frame].table.cells[cell].contents,
            link,
            traced,
            origin,
        )
    }

    pub(crate) fn prepare_original_local_alias(
        &mut self,
        name: &[u8],
        link: &Link,
        traced: bool,
    ) -> Result<PreparedAliasReplacement, NativeCompiledAliasError> {
        let frame = self
            .index_of_level(self.active_level)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        let cell = self.frames[frame]
            .table
            .lookup_slot(name)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        PreparedAliasReplacement::capture(
            &self.frames[frame].table.cells[cell].contents,
            link,
            traced,
            FrameLinkOrigin::Ordinary,
        )
    }

    /// Identity of the physical compiled cell before following its alias.
    pub(crate) fn original_compiled_slot_identity(&self, slot: usize) -> Option<VarId> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cells[cell]
            .contents
            .borrow()
            .binding_id
    }

    /// Tcl-visible name associated with a generated-code slot.
    pub(crate) fn compiled_slot_name(&self, slot: usize) -> Option<&[u8]> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.slot_name(cell)
    }

    /// The cached trace answer for the cell a generated-code slot addresses.
    pub(crate) fn compiled_slot_trace_flag(&self, slot: usize, epoch: u64) -> Option<bool> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cached_trace_flag(cell, epoch)
    }

    /// Record the trace answer for the cell a generated-code slot addresses.
    pub(crate) fn set_compiled_slot_trace_flag(&self, slot: usize, epoch: u64, traced: bool) {
        if let Some((frame, cell)) = self.compiled_cell(slot) {
            self.frames[frame]
                .table
                .set_cached_trace_flag(cell, epoch, traced);
        }
    }

    /// The variable a generated-code slot addresses — the O(1) read behind
    /// `tcl_codegen_slot_get`. `None` when the slot is unbound or its cell is
    /// currently undefined; a [`Var::Link`] is returned as-is so the caller can
    /// decline the fast path and take the coordinator's link walk.
    pub(crate) fn compiled_slot_var(&self, slot: usize) -> Option<Ref<'_, Var>> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cell_at(cell)
    }
}
