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

//! Activations whose compilation went stale: the freshness queries at an activation boundary and unwinding a stale frame or coroutine.

use super::super::{DriveMode, Frame, PolicyAtStamp, RunExit};
use crate::interp::{Vm, err};
use crate::value::Value;
use tcl_runtime_api::Completion;

impl Vm {
    /// Reject a frame whose profile or compile service changed while it was
    /// live. Stored activations can have a PC and operand stack in the middle
    /// of a lowered command, so they cannot be safely recompiled like an
    /// unentered proc.
    /// `Ok(true)` means unwinding consumed the stale frame and the caller must
    /// continue its drive loop; an empty stack is returned as a completed run.
    pub(super) fn unwind_stale_compilation_frame(
        &mut self,
        acts: &mut Vec<Frame>,
        namespace: Option<&crate::interp::namespace_context::NamespaceExecutionContext>,
    ) -> Result<bool, RunExit> {
        let current_profile = self.profile_generation();
        let current_compiler = self.compiler_generation();
        if let Some(frame) = acts.last_mut()
            && (frame.profile_generation != current_profile
                || frame
                    .compiler_generation
                    .is_some_and(|generation| generation != current_compiler))
            && frame.each_loop.is_some()
        {
            // The foreach/lmap driver contains no bytecode or profile-sensitive
            // parser state: list grouping is invariant and variable writes use
            // the VM's live semantics. Its reusable compiled body owns the
            // originating profile separately, so it is safe to advance the
            // driver itself and let the next body activation enforce freshness.
            frame.profile_generation = current_profile;
            frame.compiler_generation = frame.compiler_generation.map(|_| current_compiler);
        }
        let stale_message = namespace.map_or_else(
            || self.top_activation_stale_message(acts),
            |namespace| self.top_activation_stale_message_in_context(acts, namespace),
        );
        if let Some(message) = stale_message {
            // This is a command-like failure at the current instruction
            // boundary. Route it through the ordinary settlement seam so an
            // enclosing inline `catch` observes it before the activation is
            // unwound.
            return match self.settle_completion(acts, err(message)) {
                Some(done) => Err(RunExit::Done(done)),
                None => Ok(true),
            };
        }
        Ok(false)
    }

    /// The single activation-boundary freshness query. Scanner-only drivers
    /// declare their exemption in `Frame::stale_compilation_message`; every
    /// execution, unwind, resume, and suspension boundary consults this owner.
    fn top_activation_stale_message(&self, acts: &[Frame]) -> Option<&'static str> {
        let current_profile = self.profile_generation();
        let current_compiler = self.compiler_generation();
        let policy = PolicyAtStamp::of(self);
        acts.last().and_then(|frame| {
            frame
                .stale_compilation_message(current_profile, current_compiler, &policy)
                .or_else(|| {
                    self.activation_namespace_stale_message_in_context(
                        frame,
                        &self.current_namespace_execution_context(),
                    )
                })
        })
    }

    fn activation_namespace_stale_message_in_context(
        &self,
        frame: &Frame,
        current: &crate::interp::namespace_context::NamespaceExecutionContext,
    ) -> Option<&'static str> {
        match &frame.source_namespace {
            crate::compiled::CompiledUnitNamespace::NativeScript(original) => (!original
                .same_context(current))
            .then_some("cannot continue original Script after namespace context changed"),
            crate::compiled::CompiledUnitNamespace::CompiledPath(path) => {
                if frame.each_loop.is_some() || frame.compiler_generation.is_none() {
                    return None;
                }
                let key = current.resolved_token().map(|token| {
                    (
                        self.name_world_generation(),
                        token,
                        std::ptr::from_ref(self.actual_native_execution_profile()).addr(),
                    )
                });
                let held = key.is_some() && frame.namespace_current_at.get() == key;
                if held && !crate::VERIFY_CURRENCY {
                    return None;
                }
                let stale = match current
                    .resolved_token()
                    .filter(|token| self.native_namespace_token_has_owner(*token))
                {
                    Some(token) if self.namespace_token_has_path(token, path) => None,
                    Some(_) => Some("cannot continue bytecode after namespace changed"),
                    None => Some("cannot continue bytecode without namespace coordinates"),
                };
                debug_assert!(
                    !held || stale.is_none(),
                    "name-world generation outlived a namespace change"
                );
                if stale.is_none() {
                    frame.namespace_current_at.set(key);
                }
                stale
            }
        }
    }

    pub(super) fn current_activation_stack_stale_message(
        &self,
        acts: &[Frame],
    ) -> Option<&'static str> {
        let (current_profile, current_compiler) =
            (self.profile_generation(), self.compiler_generation());
        let policy = PolicyAtStamp::of(self);
        acts.iter()
            .find_map(|frame| {
                frame.stale_compilation_message(current_profile, current_compiler, &policy)
            })
            .or_else(|| {
                acts.last().and_then(|frame| {
                    self.activation_namespace_stale_message_in_context(
                        frame,
                        &self.current_namespace_execution_context(),
                    )
                })
            })
    }

    pub(crate) fn activation_stack_stale_message_in_context(
        &self,
        acts: &[Frame],
        namespace: &crate::interp::namespace_context::NamespaceExecutionContext,
    ) -> Option<&'static str> {
        let (current_profile, current_compiler) =
            (self.profile_generation(), self.compiler_generation());
        let policy = PolicyAtStamp::of(self);
        acts.iter()
            .find_map(|frame| {
                frame.stale_compilation_message(current_profile, current_compiler, &policy)
            })
            .or_else(|| {
                acts.last().and_then(|frame| {
                    self.activation_namespace_stale_message_in_context(frame, namespace)
                })
            })
    }

    pub(super) fn top_activation_stale_message_in_context(
        &self,
        acts: &[Frame],
        namespace: &crate::interp::namespace_context::NamespaceExecutionContext,
    ) -> Option<&'static str> {
        let policy = PolicyAtStamp::of(self);
        acts.last().and_then(|frame| {
            frame
                .stale_compilation_message(
                    self.profile_generation(),
                    self.compiler_generation(),
                    &policy,
                )
                .or_else(|| self.activation_namespace_stale_message_in_context(frame, namespace))
        })
    }

    pub(super) fn settle_stale_activation(
        &mut self,
        acts: &mut Vec<Frame>,
        message: &'static str,
    ) -> Option<Completion<Value>> {
        let completion = err(message);
        self.observe_native_error_result(&completion.result);
        self.settle_completion(acts, completion)
    }

    pub(crate) fn unwind_stale_coroutine(
        &mut self,
        acts: &mut Vec<Frame>,
        message: &'static str,
    ) -> RunExit {
        match self.settle_stale_activation(acts, message) {
            Some(done) => RunExit::Done(done),
            None => self.drive(acts, DriveMode::Plain),
        }
    }

    /// A completion is about to cross from a child activation into its parent.
    /// A stale parent owns the boundary, so replace even an OK/return completion
    /// with its provenance error before catch or loop settlement sees it.
    pub(in super::super) fn validate_unwind_boundary(
        &self,
        acts: &[Frame],
        c: Completion<Value>,
    ) -> Completion<Value> {
        self.top_activation_stale_message(acts).map_or(c, err)
    }
}
