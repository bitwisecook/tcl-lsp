// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical Tcl frame selection owned by an execution activation.

use super::{CallFrame, NsId, Vm};

/// Actual caller frames hidden while a selected-frame script executes.
/// The execution activation owns this state across suspension; variable cells
/// and namespace tokens are moved, never copied or prematurely retired.
pub(crate) struct SelectedFrameRestore {
    target_len: usize,
    frames: Vec<CallFrame>,
    namespaces: Vec<tcl_core_types::NameBytes>,
    namespace_ids: Vec<NsId>,
    namespace_scripts: Vec<usize>,
    recursion_depth: usize,
    shifted: bool,
}

impl SelectedFrameRestore {
    /// Namespace of the execution frame hidden by this exact selection.
    pub(crate) fn original_namespace(&self) -> Option<NsId> {
        self.namespace_ids.last().copied()
    }
}

impl Vm {
    /// Select a Tcl execution frame without changing the bytecode call stack.
    pub(crate) fn select_execution_frame(
        &mut self,
        target: usize,
    ) -> Result<Option<SelectedFrameRestore>, String> {
        if target >= self.frames.len() {
            return Err(format!("bad level \"{target}\""));
        }
        let target_len = target + 1;
        if target_len == self.frames.len() {
            return Ok(None);
        }
        let frames = self.frames.split_off(target_len);
        // TclLogCommandInfo tests the original execution frame's objc before
        // comparing execution and variable frames. A special frame remains
        // unreported even when uplevel redirects its variable frame.
        let original_frame = if frames
            .last()
            .is_some_and(|frame| !frame.call_argv.is_empty())
        {
            let delta = frames
                .last()
                .expect("redirect hides an execution frame")
                .level
                .saturating_sub(self.frames[target].level);
            tcl_runtime_api::error_stack::ShiftedErrorStackFrame::Redirect(delta)
        } else {
            tcl_runtime_api::error_stack::ShiftedErrorStackFrame::Unreported
        };
        let namespace_cut = target_len.min(self.resolution_stacks.ns_stack.len());
        let namespaces = self.resolution_stacks.ns_stack.split_off(namespace_cut);
        let namespace_ids = self.resolution_stacks.ns_id_stack.split_off(namespace_cut);
        let script_cut = self
            .ns_script_frames
            .partition_point(|depth| *depth <= target_len);
        let namespace_scripts = self.ns_script_frames.split_off(script_cut);
        self.native_errors
            .error_stack
            .enter_shifted_context(target_len, original_frame);
        Ok(Some(SelectedFrameRestore {
            target_len,
            frames,
            namespaces,
            namespace_ids,
            namespace_scripts,
            recursion_depth: self.recursion_depth,
            shifted: true,
        }))
    }

    /// Restore the exact hidden frames before a selected body returns to its caller.
    pub(crate) fn restore_execution_frame(&mut self, selected: Option<SelectedFrameRestore>) {
        let Some(selected) = selected else {
            return;
        };
        if selected.shifted {
            self.native_errors.error_stack.leave_shifted_context();
        }
        // Ordinary retirement owns callbacks and namespace-token release for
        // children left above the selected frame by an abrupt completion.
        self.drain_call_frames_to(selected.target_len);
        self.frames.extend(selected.frames);
        self.resolution_stacks
            .ns_stack
            .truncate(selected.target_len);
        self.resolution_stacks.ns_stack.extend(selected.namespaces);
        self.resolution_stacks
            .ns_id_stack
            .truncate(selected.target_len);
        self.resolution_stacks
            .ns_id_stack
            .extend(selected.namespace_ids);
        self.ns_script_frames
            .retain(|depth| *depth <= selected.target_len);
        self.ns_script_frames.extend(selected.namespace_scripts);
        self.recursion_depth = selected.recursion_depth;
    }
}
