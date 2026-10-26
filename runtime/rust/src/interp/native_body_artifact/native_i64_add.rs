// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked scalar addition selected from the actual retained body instructions.
use super::*;
use crate::interp::native_i64_body::FutureNativeI64AddPlan;
use tcl_registry::native_return_compilation::{NativeReturnExit, NativeReturnOptionsOperand};

macro_rules! i64_observed_some {
    ($expression:expr, $stage:literal) => {{
        let value = $expression;
        if value.is_none() {
            crate::interp::native_i64_body::observe($stage, None);
        }
        value
    }};
}

pub(super) struct NativeI64AddBody<'a> {
    expression_command: &'a CommandInstruction,
    slots: [usize; 2],
}

impl NativeBodyArtifact {
    fn native_i64_add_layout<'a>(
        &'a self,
        procedure: &Rc<ProcDef>,
        candidate: &FutureNativeI64AddPlan,
    ) -> Option<&'a NativeCompiledLocalLayout> {
        if !matches!(&self.owner, BodyContext::Procedure(owner) if Weak::ptr_eq(owner, &Rc::downgrade(procedure)))
            || tcl_registry::InvocationDialect::for_version(self.stamp.physical).arithmetic()
                != Some(tcl_dialect::NativeArithmetic::TclBignum)
            || self.image.bytes() != candidate.body()
            || procedure.params.len() != 2
            || procedure
                .params
                .iter()
                .any(|parameter| parameter.default.is_some())
            || procedure.native_statics().is_some()
            || self.scripts.len() != 2
        {
            crate::interp::native_i64_body::observe("layout-base", None);
            return None;
        }
        if procedure.params[0].name == procedure.params[1].name {
            crate::interp::native_i64_body::observe("layout-duplicate", None);
            return None;
        }
        let parameters = tcl_syntax::list::split_native_list_bytes(
            candidate.parameters(),
            self.stamp.source_protocol,
        )
        .ok()?;
        if parameters.len() != 2
            || procedure
                .params
                .iter()
                .zip(&parameters)
                .any(|(actual, requested)| actual.name.as_slice() != requested.as_ref())
        {
            crate::interp::native_i64_body::observe("layout-formals", None);
            return None;
        }
        let layout = i64_observed_some!(self.locals.as_ref(), "layout-missing")?;
        if layout.kind != NativeCompiledLocalLayoutKind::Procedure
            || layout.owner != self.stamp.interpreter
            || layout.names.len() != 2
        {
            crate::interp::native_i64_body::observe("layout-owner", None);
            return None;
        }
        Some(layout)
    }

    pub(super) fn native_i64_add_body<'a>(
        &'a self,
        procedure: &Rc<ProcDef>,
        candidate: &FutureNativeI64AddPlan,
    ) -> Option<NativeI64AddBody<'a>> {
        let layout =
            i64_observed_some!(self.native_i64_add_layout(procedure, candidate), "layout")?;
        let root = i64_observed_some!(self.scripts.get(&self.region), "root-missing")?;
        if root.fatal.is_some() || root.empty.is_some() {
            crate::interp::native_i64_body::observe("root-fatal", None);
            return None;
        }
        let [return_command] = root.commands.as_slice() else {
            crate::interp::native_i64_body::observe("root-count", None);
            return None;
        };
        let Operation::SelectedReturn(recipe, _, _) = &return_command.operation else {
            crate::interp::native_i64_body::observe("return-operation", None);
            return None;
        };
        let NativeReturnOptionsOperand::Static(options) = &recipe.options else {
            crate::interp::native_i64_body::observe("return-options", None);
            return None;
        };
        if recipe.exit != NativeReturnExit::Done
            || recipe.value_word != Some(1)
            || options.code != 0
            || options.level != 1
            || options.protocol != self.stamp.source_protocol
            || return_command.words.len() != 2
            || return_command.guard.is_none()
        {
            crate::interp::native_i64_body::observe("return-recipe", None);
            return None;
        }
        let word = &return_command.words[1];
        let [part] = word.arena.original.list(word.arena.original.root()) else {
            crate::interp::native_i64_body::observe("return-word", None);
            return None;
        };
        let ExecutablePart::Command { body } = &part.part else {
            crate::interp::native_i64_body::observe("return-substitution", None);
            return None;
        };
        let nested = i64_observed_some!(self.scripts.get(body), "expression-missing")?;
        if nested.fatal.is_some() || nested.empty.is_some() {
            crate::interp::native_i64_body::observe("expression-fatal", None);
            return None;
        }
        let [expression_command] = nested.commands.as_slice() else {
            crate::interp::native_i64_body::observe("expression-count", None);
            return None;
        };
        let Operation::Expression(expression) = &expression_command.operation else {
            crate::interp::native_i64_body::observe("expression-operation", None);
            return None;
        };
        if expression_command.guard.is_none() {
            crate::interp::native_i64_body::observe("expression-guard", None);
            return None;
        }
        let operands = i64_observed_some!(
            expression.native_i64_add_operands(&self.image),
            "expression-operands"
        )?;
        if tcl_registry::native_expression_program::expression_program_emission_for_policy(
            operands.program,
            self.stamp.expression_policy.as_ref(),
            tcl_registry::InvocationDialect::for_version(self.stamp.physical),
        ) != tcl_registry::native_expression_program::ExpressionProgramEmission::Native
        {
            crate::interp::native_i64_body::observe("expression-policy", None);
            return None;
        }
        if operands.slots != [0, 1]
            || operands
                .names
                .iter()
                .zip(&procedure.params)
                .any(|(name, parameter)| *name != parameter.name.as_slice())
            || layout
                .names
                .iter()
                .zip(&procedure.params)
                .any(|(name, parameter)| {
                    name.as_ref()
                        .is_none_or(|name| name.as_ref() != parameter.name.as_slice())
                })
        {
            crate::interp::native_i64_body::observe("compiled-slots", None);
            return None;
        }
        Some(NativeI64AddBody {
            expression_command,
            slots: operands.slots,
        })
    }
}

pub(super) struct EnteredI64AddPlan {
    artifact: Weak<NativeBodyArtifact>,
    procedure: Weak<ProcDef>,
    frame: Weak<()>,
    expression_program: *const tcl_registry::native_expression_program::NativeExpressionProgram,
}

impl EnteredI64AddPlan {
    pub(super) fn matches(
        &self,
        artifact: &NativeBodyArtifact,
        program: &tcl_registry::native_expression_program::NativeExpressionProgram,
        node: &tcl_syntax::expr::NativeExprNode,
    ) -> bool {
        let Some(retained) = self.artifact.upgrade() else {
            return false;
        };
        let tcl_registry::native_expression_program::NativeExpressionTree::Parsed(root) =
            &program.tree
        else {
            return false;
        };
        core::ptr::eq(retained.as_ref(), artifact)
            && core::ptr::eq(self.expression_program, program)
            && root == node
    }

    pub(super) fn optimization_is_current(&self, interpreter: &Interp) -> bool {
        let Some(artifact) = self.artifact.upgrade() else {
            return false;
        };
        let Some(procedure) = self.procedure.upgrade() else {
            return false;
        };
        let Ok(original) = procedure.body.checked_ptr() else {
            return false;
        };
        cache(original).is_some_and(|current| Rc::ptr_eq(&current, &artifact))
            && interpreter.original_procedure_artifact_is_current(&procedure)
    }

    pub(super) fn check(
        &self,
        interpreter: &Interp,
        operation: &native_list_conversion::BodyObjectOperation,
    ) -> Result<(), crate::expr_error::ExprError> {
        operation
            .check(interpreter)
            .map_err(crate::expr_error::ExprError::from_cmd_error)?;
        let Some(procedure) = self.procedure.upgrade() else {
            return Err(crate::expr_error::ExprError::host_refusal(
                unavailable("native i64 original declaration")
                    .native_access_refusal()
                    .expect("typed refusal"),
            ));
        };
        procedure
            .check_native_liveness()
            .map_err(|error| crate::expr_error::ExprError::from_cmd_error(error.into()))?;
        if !interpreter
            .frames
            .borrow()
            .native_i64_execution_matches(&procedure, &self.frame)
        {
            return Err(crate::expr_error::ExprError::host_refusal(
                unavailable("native i64 entered frame")
                    .native_access_refusal()
                    .expect("typed refusal"),
            ));
        }
        Ok(())
    }
}

impl Interp {
    pub(super) fn capture_entered_i64_body(
        &self,
        artifact: &Rc<NativeBodyArtifact>,
    ) -> Option<Box<EnteredI64AddPlan>> {
        let procedure = self.frames.borrow().current_c_procedure()?.clone();
        let candidate = procedure.future_i64_add.get()?;
        crate::interp::native_i64_body::observe("entered-candidate", None);
        i64_observed_some!(procedure.check_native_liveness().ok(), "procedure-live")?;
        if self.native_execution_refusal().is_some()
            || !self.original_procedure_artifact_is_current(&procedure)
        {
            crate::interp::native_i64_body::observe("entry-currency", None);
            return None;
        }
        let original = i64_observed_some!(procedure.body.checked_ptr().ok(), "body-live")?;
        if !Rc::ptr_eq(&cache(original)?, artifact) {
            crate::interp::native_i64_body::observe("cache-identity", None);
            return None;
        }
        let selected = i64_observed_some!(
            artifact.native_i64_add_body(&procedure, candidate),
            "body-recipe"
        )?;
        let traces = self.traces.borrow();
        if !traces.cmd_traces.is_empty()
            || !traces.traces.is_empty()
            || !traces.step_active.is_empty()
        {
            crate::interp::native_i64_body::observe("trace-active", None);
            return None;
        }
        drop(traces);
        let protocol = self
            .native_invocation_dialect()
            .native_scalar_getter_protocol()?;
        let frame = i64_observed_some!(
            self.frames
                .borrow()
                .quiet_native_i64_frame(&procedure, selected.slots, protocol),
            "quiet-frame"
        )?;
        let Operation::Expression(expression) = &selected.expression_command.operation else {
            return None;
        };
        let program =
            core::ptr::from_ref(expression.native_i64_add_operands(&artifact.image)?.program);
        crate::interp::native_i64_body::observe("strategy-captured", None);
        Some(Box::new(EnteredI64AddPlan {
            artifact: Rc::downgrade(artifact),
            procedure: Rc::downgrade(&procedure),
            frame,
            expression_program: program,
        }))
    }
}

#[cfg(test)]
mod tests;
