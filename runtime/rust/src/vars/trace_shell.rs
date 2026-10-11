// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cleanup consumes the actual resolved trace home without another name lookup.

use super::{
    entered_container::{same_table, TableObservation},
    selected_table, TraceHome, VarHome,
};
use crate::{frame::TraceShellState, interp::Interp};
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

pub(crate) struct CapturedTraceShell {
    home: VarHome,
    name: Vec<u8>,
    key: Option<Vec<u8>>,
    binding: Option<tcl_runtime_api::VarId>,
    table: TableObservation,
    state: TraceShellState,
}

impl CapturedTraceShell {
    /// Observe the actual original table/frame/root before any cleanup callback.
    /// A stale registration can be removed, but cannot clean up a reborn cell.
    pub(crate) fn capture(
        interpreter: &Interp,
        home: &TraceHome,
        key: Option<&[u8]>,
    ) -> Option<Self> {
        let frames = interpreter.frames.borrow();
        let namespaces = interpreter.namespaces();
        let place = match (home.ns, home.level) {
            (Some(namespace), _) => VarHome::Namespace(namespace),
            (_, Some(level)) => VarHome::Frame(level),
            _ => return None,
        };
        let original_table = TableObservation::capture(&frames, &namespaces, place)?;
        let selected = selected_table(&frames, &namespaces, place, true)?;
        if let (Some(root), Some(key), Some((selected_key, member))) =
            (home.binding_id, key, home.selected_member.as_ref())
        {
            if selected_key.as_slice() == key
                && selected.native_trace_element_identity(root, key) != Some(*member)
            {
                return None;
            }
        }
        let state = selected.observe_trace_shell(&home.base, key, home.binding_id)?;
        Some(Self {
            home: place,
            name: home.base.clone(),
            key: key.map(<[u8]>::to_vec),
            binding: home.binding_id,
            table: original_table,
            state,
        })
    }

    fn retain(interpreter: &Interp, error: ValueError) -> CmdError {
        CmdError::from_execution_refusal(interpreter.clone().refuse_completion_value_access(error))
    }
    fn check_state(&self, interpreter: &Interp, state: &TraceShellState) -> Result<(), CmdError> {
        let physical = same_table(interpreter, self.home, &self.table).and_then(|()| {
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            let selected = selected_table(&frames, &namespaces, self.home, true).ok_or(
                ValueError::CommandProtocolUnavailable("original trace-shell table"),
            )?;
            state.check(selected, &self.name)
        });
        physical.map_err(|error| Self::retain(interpreter, error))
    }

    /// Same-target cleanup remains mandatory under a refused saved operation.
    /// This carrier only withdraws the selected owners; it adopts no new value.
    pub(crate) fn finish(
        self,
        interpreter: &Interp,
        current: impl Fn() -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        let mut first = current().err();
        if let Err(error) = self.check_state(interpreter, &self.state) {
            return Err(first.unwrap_or(error));
        }
        let (post, retired) = {
            let mut frames = interpreter.frames.borrow_mut();
            let mut namespaces = interpreter.namespaces_mut();
            match self.home {
                VarHome::Frame(level) => frames
                    .prepare_original_trace_shell_cleanup_at(
                        level,
                        &self.name,
                        self.key.as_deref(),
                        self.binding,
                    )
                    .expect("checked original trace-shell frame"),
                VarHome::Namespace(namespace) => namespaces
                    .original_variable_table_at_mut(namespace)
                    .expect("checked original trace-shell namespace")
                    .prepare_trace_shell_cleanup(&self.name, self.key.as_deref(), self.binding),
            }
        };
        let reached = retired.finish(|| {
            current()?;
            match &post {
                Some(post) => self.check_state(interpreter, post),
                None => Err(Self::retain(
                    interpreter,
                    ValueError::CommandProtocolUnavailable("original trace-shell withdrawal"),
                )),
            }
        });
        if let Err(error) = reached {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }
}
