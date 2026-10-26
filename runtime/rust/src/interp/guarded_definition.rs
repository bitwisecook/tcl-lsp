// SPDX-License-Identifier: AGPL-3.0-or-later
//! A compiled definition candidate carried through the actual command dispatch.

use super::native_arguments::OriginalArgumentFrame;
use super::native_i64_body::FutureNativeI64AddPlan;
use super::{Code, Interp, NativeProcEntry};

enum DefinitionCandidate {
    Native(NativeProcEntry),
    I64Add(FutureNativeI64AddPlan),
}
use crate::{namespace::GLOBAL, obj::TclObj};
use std::rc::Rc;
use tcl_runtime_api::guard::GuardIdentity;

pub(super) struct DefinitionRequest {
    birth: Rc<()>,
    argv: Vec<*mut TclObj>,
    depth: u32,
    frame: OriginalArgumentFrame,
    expected: GuardIdentity,
    candidate: Option<DefinitionCandidate>,
}

struct DefinitionLease {
    interpreter: Interp,
    birth: Rc<()>,
}

impl Drop for DefinitionLease {
    fn drop(&mut self) {
        let mut requests = self.interpreter.guarded_definitions.borrow_mut();
        if let Some(index) = requests
            .iter()
            .position(|request| Rc::ptr_eq(&request.birth, &self.birth))
        {
            requests.remove(index);
        }
    }
}

impl Interp {
    /// The ordinary dispatcher owns every head getter, lookup and trace stage.
    pub(crate) fn dispatch_guarded_definition(
        &mut self,
        argv: &[*mut TclObj],
        entry: Option<NativeProcEntry>,
        expected: GuardIdentity,
    ) -> Code {
        let Some(entry) = entry else {
            return self.dispatch(argv);
        };
        self.dispatch_definition_candidate(argv, DefinitionCandidate::Native(entry), expected)
    }

    pub(crate) fn dispatch_guarded_i64_add_definition(
        &mut self,
        argv: &[*mut TclObj],
        plan: FutureNativeI64AddPlan,
        expected: GuardIdentity,
    ) -> Code {
        self.dispatch_definition_candidate(argv, DefinitionCandidate::I64Add(plan), expected)
    }

    fn dispatch_definition_candidate(
        &mut self,
        argv: &[*mut TclObj],
        candidate: DefinitionCandidate,
        expected: GuardIdentity,
    ) -> Code {
        let birth = Rc::new(());
        self.guarded_definitions
            .borrow_mut()
            .push(DefinitionRequest {
                birth: Rc::clone(&birth),
                argv: argv.to_vec(),
                depth: self.native_dispatch_depth.get().saturating_add(1),
                frame: OriginalArgumentFrame::capture(self),
                expected,
                candidate: Some(candidate),
            });
        let _lease = DefinitionLease {
            interpreter: self.clone(),
            birth,
        };
        self.dispatch(argv)
    }

    /// Consume only this exact ordinary invocation, after its actual selection.
    fn take_definition_candidate(
        &self,
        argv: &[*mut TclObj],
    ) -> Option<(DefinitionCandidate, GuardIdentity)> {
        let mut requests = self.guarded_definitions.borrow_mut();
        let request = requests.last_mut()?;
        if request.depth != self.native_dispatch_depth.get()
            || request.argv != argv
            || request.frame.check_frame(self).is_err()
        {
            return None;
        }
        Some((request.candidate.take()?, request.expected))
    }

    /// The selected token is already resolved; this performs no second lookup.
    fn selected_definition_guard(&self, generation: u64, expected: GuardIdentity) -> bool {
        let dialect = self.native_invocation_dialect();
        if self.current_ns.get() != GLOBAL
            || self.frames.borrow().current_level() != 0
            || self.frames.borrow().top_level() != 0
            || self.name_policy_protocol().map(|policy| policy.recipe())
                != dialect.native_name_protocol()
            || !self
                .native_compilation
                .borrow()
                .stock
                .contains_key(&generation)
            || tcl_registry::NativeLowering::Definition
                .definition_guard_identity(self.native_invocation_dialect())
                != Some(expected)
        {
            return false;
        }
        self.synchronise_command_environment_guards();
        let traces = self.traces.borrow();
        if !traces.cmd_traces.is_empty() || !traces.traces.is_empty() {
            return false;
        }
        drop(traces);
        if !self
            .guarded_commands
            .borrow()
            .get(&generation)
            .is_some_and(|identities| identities.contains(&expected))
        {
            return false;
        }
        let Some(domains) =
            tcl_registry::NativeLowering::Definition.selected_definition_guard_domains()
        else {
            return false;
        };
        let Ok(token) =
            self.guards
                .borrow_mut()
                .prepare(expected, Some(expected), domains, generation)
        else {
            return false;
        };
        let valid =
            self.guards
                .borrow()
                .check_expected(token, expected, Some(expected), &generation);
        let released = self.guards.borrow_mut().release(token);
        valid && released
    }

    /// Runs inside the original builtin activation, immediately before its handler.
    pub(super) fn invoke_guarded_definition(
        &mut self,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> Option<Code> {
        let (candidate, expected) = self.take_definition_candidate(argv)?;
        let generation = generation?;
        if self.check_entered_native_operation().is_err() {
            return Some(Code::Error);
        }
        if !self.selected_definition_guard(generation, expected) {
            return None;
        }
        Some(match candidate {
            DefinitionCandidate::Native(entry) => {
                crate::cmd_proc::proc_cmd_with_native_entry(self, argv, Some(entry))
            }
            DefinitionCandidate::I64Add(plan) => {
                crate::cmd_proc::proc_cmd_with_i64_add_plan(self, argv, plan)
            }
        })
    }
}
