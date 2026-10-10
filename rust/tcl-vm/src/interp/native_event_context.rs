// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event callbacks and wait subjects select the actual shared global frame.
use super::{Completion, Value, Vm};
impl Vm {
    pub(crate) fn with_event_global_frame<R>(&mut self, action: impl FnOnce(&mut Self) -> R) -> R {
        let restore = self.select_execution_frame(0).expect("live global frame");
        let result = action(self);
        self.restore_execution_frame(restore);
        result
    }
    pub(crate) fn eval_original_event(&mut self, source: &Value) -> Completion<Value> {
        self.with_event_global_frame(|vm| {
            match vm.eval_original_script_value(
                source,
                tcl_registry::native_eval_object::EvalObjectPurpose::AfterCallback,
                source.source_location(),
            ) {
                Ok(c) => c,
                Err(error) => crate::command::completion_from_tcl_error(vm, error),
            }
        })
    }
    pub(crate) fn original_global_event_value(
        &mut self,
        original: &Value,
    ) -> Result<Option<Value>, Completion<Value>> {
        self.with_event_global_frame(|vm| {
            if vm
                .current_jim_variable_cell(original)
                .map_err(|e| vm.refuse_host_command(e.to_string()))?
                .is_some()
            {
                return vm.read_original_named_variable(original).map(Some);
            }
            let name = vm
                .native_name_operand_bytes(original)
                .map_err(|e| vm.refuse_host_command(e.to_string()))?;
            if Self::is_original_jim_dictionary_name(original, &name) {
                return vm.read_original_named_variable(original).map(Some);
            }
            vm.install_original_jim_variable(original, &name)
                .map_err(|e| vm.refuse_host_command(e.to_string()))?;
            if vm
                .current_jim_variable_cell(original)
                .map_err(|e| vm.refuse_host_command(e.to_string()))?
                .is_none()
            {
                return Ok(None);
            }
            vm.read_original_named_variable(original).map(Some)
        })
    }
}
