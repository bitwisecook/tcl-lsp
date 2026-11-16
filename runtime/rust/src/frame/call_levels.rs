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

//! Levels of the frame stack: push and pop, the active variable frame, caller chains, pending tailcalls and invocation words.

use super::{Frame, FrameStack, PendingTailcall};
use crate::namespace::NsId;
use std::rc::Rc;

impl FrameStack {
    /// The active logical variable frame level (`varFramePtr`) used by names.
    pub fn current_level(&self) -> usize {
        self.active_level
    }

    /// Whether the active frame is a proc call frame. Unqualified names resolve
    /// frame-local only here; at global / `namespace eval` scope they resolve to
    /// the current namespace.
    pub fn in_proc(&self) -> bool {
        self.index_of_level(self.active_level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether the frame at logical `level` is a proc call frame (vs the global
    /// or a `namespace eval` frame) — the frame-addressed analogue of
    /// [`in_proc`](Self::in_proc), for `FrameId`-addressed variable access.
    pub(crate) fn is_proc_at(&self, level: usize) -> bool {
        self.index_of_level(level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether the selected engine stores ordinary variables in this activation.
    pub(crate) fn owns_local_variables_at(&self, level: usize) -> bool {
        self.is_proc_at(level)
            || (self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim && level > 0)
    }

    /// The true top-of-stack level (`framePtr`), independent of any `uplevel`
    /// redirection of the active level.
    pub fn top_level(&self) -> usize {
        self.frames.last().map_or(0, |f| f.level)
    }

    /// Push a new proc call frame running in namespace `ns`; returns its level
    /// and makes it the active frame.
    pub fn push(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Push a proc call frame that shares the *current* level (it still gets its
    /// own local-variable table). Used for a TclOO method invoked via `next`:
    /// the whole call chain runs at the level of the original invocation, so
    /// `info level` / `upvar` / `uplevel` resolve through the chain. Returns the
    /// (unchanged) level; the new frame becomes active and, being topmost at
    /// that level, is what unqualified names and `info level` resolve to.
    pub fn push_same_level(&mut self, ns: NsId) -> usize {
        let level = self.active_level;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Record the procedure body the top frame activates.
    pub(crate) fn set_unit(&mut self, unit: Rc<crate::compiled_locals::ProcUnit>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.unit = Some(unit);
        }
    }

    /// The procedure body the active variable frame activates, if it is a
    /// procedure frame whose body is recorded.
    pub(crate) fn active_unit(&self) -> Option<Rc<crate::compiled_locals::ProcUnit>> {
        self.index_of_level(self.active_level)
            .and_then(|i| self.frames[i].unit.clone())
    }

    /// Push a *namespace* frame (`namespace eval`/`inscope`): a new scope whose
    /// unqualified variables resolve to the namespace (not frame-local). Returns
    /// its level and makes it active.
    pub fn push_namespace(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol); // is_proc = false
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Pop the current (top) frame, releasing its locals (via `VarTable::drop`),
    /// and restore the active level to whatever was in effect when it was pushed
    /// (correct even under an `uplevel` redirection). The global level is never
    /// popped. Returns the namespace the popped frame was running in — the
    /// activation C's `Tcl_PopCallFrame` gives back to the namespace token.
    pub fn pop(&mut self) -> Option<NsId> {
        let (namespace, owners) = self.take_frame_for_pop()?;
        let storage = owners.release();
        self.recycle_jim_storage(storage);
        Some(namespace)
    }

    /// The stack index of the topmost frame with logical `level` (levels are not
    /// stack indices once `uplevel` is in play; the most recent wins).
    pub(super) fn index_of_level(&self, level: usize) -> Option<usize> {
        self.frames.iter().rposition(|f| f.level == level)
    }

    /// The stack index of the current active (var) frame — the identity a new
    /// `CmdFrame` records as its CallFrame (C's `framePtr->framePtr`).
    pub(crate) fn current_frame_index(&self) -> usize {
        self.index_of_level(self.active_level).unwrap_or(0)
    }

    /// Issued activation identity of the current variable frame, including uplevel selection.
    pub(crate) fn current_activation(&self) -> u64 {
        self.frames[self.current_frame_index()].jim_id
    }

    pub(crate) fn current_activation_owner(&self) -> std::rc::Weak<()> {
        Rc::downgrade(&self.frames[self.current_frame_index()].jim_link_birth)
    }

    /// The actual frame incarnation at a reached logical level. A later frame
    /// at the same level cannot revive this observation.
    pub(crate) fn original_activation_owner_at(&self, level: usize) -> Option<std::rc::Weak<()>> {
        Some(Rc::downgrade(
            &self.frames[self.index_of_level(level)?].jim_link_birth,
        ))
    }

    /// The set of stack indices on the active frame's caller chain (C's
    /// `callerVarPtr` walk from `varFramePtr`). Each frame's caller is the topmost
    /// frame *below* it at the level that was active when it was pushed
    /// (`saved_active`) — so an `uplevel`-redirected call's chain skips the frame
    /// it bypassed, even when that frame shares a level. Used by `info frame` to
    /// decide whether to report a frame's `level`.
    pub(crate) fn caller_chain_indices(&self) -> std::collections::HashSet<usize> {
        let mut set = std::collections::HashSet::new();
        let mut idx = self.current_frame_index();
        loop {
            if !set.insert(idx) || idx == 0 {
                break;
            }
            let caller_level = self.frames[idx].saved_active;
            match self.frames[..idx]
                .iter()
                .rposition(|f| f.level == caller_level)
            {
                Some(c) => idx = c,
                None => break,
            }
        }
        set
    }

    /// Redirect the active variable frame to `level` (for `uplevel`), returning
    /// the previous active level so the caller can restore it.
    pub fn set_active_level(&mut self, level: usize) -> usize {
        let prev = self.active_level;
        self.active_level = level;
        prev
    }

    /// Move retired owners out of the frame before their native free callbacks run.
    pub(crate) fn replace_tailcall(
        &mut self,
        request: Option<PendingTailcall>,
    ) -> Result<Option<PendingTailcall>, Option<PendingTailcall>> {
        let Some(index) = self.index_of_level(self.active_level) else {
            return Err(request);
        };
        if !self.frames[index].is_proc {
            return Err(request);
        }
        Ok(std::mem::replace(&mut self.frames[index].tailcall, request))
    }

    pub(crate) fn take_tailcall(&mut self) -> Option<PendingTailcall> {
        self.frames.last_mut()?.tailcall.take()
    }

    /// Record the invoking command words on the top frame (`info level N`).
    pub(crate) fn set_words(&mut self, words: Vec<Vec<u8>>) {
        if let Some(f) = self.frames.last_mut() {
            f.words = words;
        }
    }

    /// Original top call-frame invocation and its active-variable-frame redirect.
    /// Stack identity is retained even when two TclOO frames share a level.
    pub(crate) fn error_stack_frame(&self) -> Option<(Option<usize>, &[Vec<u8>])> {
        let top_index = self.frames.len().checked_sub(1)?;
        let top = &self.frames[top_index];
        if top
            .original_error_stack_argv
            .as_ref()
            .map_or_else(|| top.words.is_empty(), Vec::is_empty)
        {
            return None;
        }
        let delta = if self.current_frame_index() != top_index {
            Some(top.level.saturating_sub(self.active_level))
        } else {
            None
        };
        Some((delta, &top.words))
    }

    /// The command words that invoked frame `level` (`info level N`); empty if
    /// the level has none (e.g. the global frame).
    pub(crate) fn words_at(&self, level: usize) -> Option<&[Vec<u8>]> {
        let i = self.index_of_level(level)?;
        Some(self.frames[i].words.as_slice())
    }

    /// The namespace frame `level` runs in (for `uplevel` ns restoration).
    pub fn frame_ns(&self, level: usize) -> NsId {
        self.index_of_level(level)
            .map_or(crate::namespace::GLOBAL, |i| self.frames[i].ns)
    }

    /// Temporarily change the existing variable frame's lookup namespace.
    pub(crate) fn replace_active_namespace(&mut self, namespace: NsId) -> NsId {
        let Some(index) = self.index_of_level(self.active_level) else {
            return crate::namespace::GLOBAL;
        };
        std::mem::replace(&mut self.frames[index].ns, namespace)
    }
}
