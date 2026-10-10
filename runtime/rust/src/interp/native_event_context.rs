// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event callbacks and wait subjects select the actual shared global frame.
use super::{Code, Interp, TclObj};
impl Interp {
    pub(crate) fn with_event_global_frame<R>(&mut self, action: impl FnOnce(&mut Self) -> R) -> R {
        let level = self.frames.borrow_mut().set_active_level(0);
        let namespace = self.current_ns.get();
        self.current_ns.set(self.frames.borrow().frame_ns(0));
        let result = action(self);
        self.frames.borrow_mut().set_active_level(level);
        self.current_ns.set(namespace);
        result
    }
    pub(crate) fn eval_original_event(&mut self, original: *mut TclObj) -> Code {
        self.with_event_global_frame(|interp| {
            let frame = interp.unlocated_frame();
            interp.eval_original_body_framed(
                tcl_registry::native_eval_object::EvalObjectPurpose::AfterCallback,
                original,
                frame,
            )
        })
    }
    pub(crate) fn original_global_event_value(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Option<*mut TclObj>, Code> {
        use tcl_syntax::value::ValueOps;
        self.with_event_global_frame(|interp| {
            if interp
                .current_jim_variable_cache(original)
                .map_err(|e| interp.report_cmd_error(e.into()))?
                .is_some()
            {
                return interp.read_original_named_variable(original).map(Some);
            }
            let name = interp
                .native_string_bytes(&original)
                .map_err(|e| interp.report_cmd_error(e.into()))?;
            if interp.is_native_jim_dictionary_name(original, &name) {
                return interp.read_original_named_variable(original).map(Some);
            }
            interp
                .install_original_jim_variable(original, &name)
                .map_err(|e| interp.report_cmd_error(e.into()))?;
            if interp
                .current_jim_variable_cache(original)
                .map_err(|e| interp.report_cmd_error(e.into()))?
                .is_none()
            {
                return Ok(None);
            }
            interp.read_original_named_variable(original).map(Some)
        })
    }
}
