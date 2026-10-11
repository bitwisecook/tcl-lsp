// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled List headers and actual variable-frame/temporary selection.
use super::*;
use crate::interp::native_arguments::OriginalArguments;
use tcl_cmd_core::CmdError;

/// The actually entered variable-frame birth. This weak observation grants
/// no variable/native role and remains usable after actual header cleanup.
pub(super) struct BodyListFrame {
    activation: u64,
    birth: Weak<()>,
}
impl BodyListFrame {
    pub(super) fn capture(interpreter: &Interp) -> Self {
        let frames = interpreter.frames.borrow();
        Self {
            activation: frames.current_activation(),
            birth: frames.current_activation_owner(),
        }
    }
    pub(super) fn check(
        &self,
        actual: &Interp,
        arguments: &OriginalArguments,
    ) -> Result<(), CmdError> {
        arguments.check_for(actual)?;
        let same = {
            let frames = actual.frames.borrow();
            self.birth.strong_count() != 0
                && frames.current_activation() == self.activation
                && Weak::ptr_eq(&frames.current_activation_owner(), &self.birth)
        };
        if !same {
            return Err(arguments
                .retain_refusal(unavailable("original compiled List frame or temporary").into()));
        }
        Ok(())
    }
}

/// An allocation-only original header projection. The frame birth and optional
/// anonymous local are observations, not new native or variable owning roles.
pub(super) struct BodyListArguments {
    arguments: OriginalArguments,
    original: *mut TclObj,
    frame: BodyListFrame,
    temporary: Option<usize>,
}
impl BodyListArguments {
    pub(super) fn capture(
        interpreter: &Interp,
        original: *mut TclObj,
        temporary: Option<usize>,
    ) -> Result<Self, CmdError> {
        let arguments = OriginalArguments::enter(interpreter, &[original])?;
        let captured = Self {
            arguments,
            original,
            frame: BodyListFrame::capture(interpreter),
            temporary,
        };
        captured.check(interpreter)?;
        Ok(captured)
    }
    pub(super) fn check(&self, actual: &Interp) -> Result<(), CmdError> {
        self.arguments
            .check_original_for(actual, 0, self.original)?;
        self.check_selection(actual)?;
        Ok(())
    }
    pub(super) fn check_selection(&self, actual: &Interp) -> Result<(), CmdError> {
        self.frame.check(actual, &self.arguments)?;
        let same = self.temporary.is_none_or(|slot| {
            actual
                .frames
                .borrow()
                .native_compiled_temporary(slot)
                .ok()
                .flatten()
                == Some(self.original)
        });
        if !same {
            return Err(self
                .arguments
                .retain_refusal(unavailable("original compiled List frame or temporary").into()));
        }
        Ok(())
    }
    pub(super) fn refuse(&self, error: ValueError) -> CmdError {
        self.arguments.retain_refusal(error.into())
    }
    pub(super) fn length(&self, interpreter: &mut Interp) -> Result<usize, CmdError> {
        crate::value_ops::list_length_using(
            interpreter,
            &self.original,
            &mut |actual, _original| self.check(actual),
            |actual, original| {
                let protocol = actual.native_invocation_dialect().native_string_materialization(
                    Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
                ).ok_or_else(|| self.refuse(unavailable("native object list conversion")))?.protocol();
                if protocol.is_jim084() {
                    let context = actual
                        .native_jim_object_context()
                        .map_err(|error| self.refuse(error))?;
                    self.check(actual)?;
                    crate::native_source::bind_context(*original, &context)
                        .map_err(|error| self.refuse(error))?;
                    self.check(actual)?;
                }
                crate::list::list_elements_native_using(
                    *original,
                    protocol,
                    &|| self.check(actual),
                    &|error| self.refuse(error),
                )
            },
        )
    }
}

#[cfg(test)]
mod tests;
