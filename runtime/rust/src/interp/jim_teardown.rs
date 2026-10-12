// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim's last-live-handle retirement, before ordinary state destruction.

use super::*;
use tcl_runtime_api::jim_interpreter::{JimInterpreterObjectRole as Role, JimInterpreterTeardown};

impl Interp {
    fn retire_jim_interpreter(&mut self) {
        if self.jim_teardown_started.replace(true) {
            return;
        }
        tcl_runtime_api::jim_interpreter::teardown_jim_interpreter(self);
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            context.retire();
        }
    }

    fn invoke_jim_frame_defers(&mut self, level: usize) {
        let Ok(context) = self.native_jim_object_context() else {
            return;
        };
        let name = match crate::dict::native_object_bytes(
            context.defer_object().as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        ) {
            Ok(name) => name,
            Err(_) => return,
        };
        let present = if level == 0 {
            self.namespaces
                .borrow()
                .var_table(crate::namespace::GLOBAL)
                .load_scalar(&name)
                .is_some()
        } else {
            self.frames
                .borrow()
                .table(level)
                .is_some_and(|table| table.load_scalar(&name).is_some())
        };
        if !present {
            return;
        }
        let Some(scripts) = self.var_get_at(&name, level) else {
            return;
        };
        let scripts = obj::Owned::retain(scripts);
        if crate::native_source::bind_context(scripts.as_ptr(), &context).is_err() {
            return;
        }
        if crate::list::list_elements_native_checked(
            scripts.as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .is_err()
        {
            return;
        }
        let Some(backing) = crate::list::native_list_backing(scripts.as_ptr()) else {
            return;
        };
        let saved_result = obj::Owned::retain(self.result.get());
        self.set_result(context.empty_object().as_ptr());
        let mut completed = true;
        let elements = match backing.elements() {
            Ok(elements) => elements,
            Err(error) => {
                self.report_cmd_error(error.into());
                return;
            }
        };
        for &script in elements.iter().rev() {
            if self.eval_body_obj(script) != Code::Ok {
                completed = false;
                break;
            }
        }
        if completed {
            self.set_result(saved_result.as_ptr());
        }
    }
}

impl JimInterpreterTeardown for Interp {
    fn release_frames(&mut self) {
        loop {
            let level = self.frames.borrow_mut().select_top_for_jim_retirement();
            let Some(level) = level else {
                break;
            };
            self.invoke_jim_frame_defers(level);
            self.clean_current_jim_local_commands();
            let retired = self.frames.borrow_mut().take_top_for_jim_retirement();
            drop(retired);
        }
        let retired = self.namespaces.borrow_mut().take_jim_frame_owners();
        drop(retired);
        let targets = std::mem::take(&mut *self.array_operation_targets.borrow_mut());
        drop(targets);
    }

    fn release_commands(&mut self) {
        let commands = self.namespaces.borrow_mut().take_jim_command_owners();
        drop(commands);
        let hidden = std::mem::take(&mut *self.hidden.borrow_mut());
        drop(hidden);
    }

    fn release_object(&mut self, role: Role) {
        if role == Role::Result {
            let result = self.result.replace(core::ptr::null_mut());
            // SAFETY: this slot owns exactly one reference until taken here.
            if !result.is_null() {
                unsafe { obj::decr_ref_count(result) };
            }
        } else if role == Role::StackTrace {
            let stack = std::mem::take(&mut *self.jim_error_stack.borrow_mut());
            drop(stack);
        }
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            context.release_object(role);
        }
    }

    fn invalidate_procedure_epoch(&mut self) {
        self.namespaces.borrow_mut().retire_jim_procedure_epoch();
    }

    fn release_packages_and_associations(&mut self) {
        let packages = std::mem::take(&mut *self.packages.borrow_mut());
        drop(packages);
    }

    fn release_trace_command(&mut self) {
        // No assocData or traceCmd original object is installed in this backend.
    }
}

impl Drop for Interp {
    fn drop(&mut self) {
        if Rc::strong_count(&self.0) == 1
            && self.native_invocation_dialect().native_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            self.retire_jim_interpreter();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static EXPECTED: Cell<Option<(*mut TclObj, *mut TclObj, *mut TclObj)>> = const { Cell::new(None) };
    }

    fn inspect_activation(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        assert_eq!(
            interp.frames.borrow().jim_activation_objects(),
            EXPECTED.with(Cell::get)
        );
        interp.set_result_bytes(b"observed");
        Code::Ok
    }

    #[test]
    fn ordinary_jim_activation_owns_the_original_declaration_objects() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        interp.register_builtin(b"inspect_activation", inspect_activation);
        let parameters = obj::Owned::fresh(new_string(b""));
        let body = obj::Owned::fresh(new_string(b"inspect_activation"));
        let parsed =
            crate::cmd_proc::parse_params_object(&mut interp, parameters.as_ptr(), b"originals")
                .unwrap();
        interp.define_proc_original_storage(
            b"originals",
            parsed,
            Some(parameters.as_ptr()),
            body.as_ptr(),
            None,
            None,
        );
        let Some(Command::Proc(definition)) =
            interp.resolve_dispatchable(crate::namespace::GLOBAL, b"originals")
        else {
            panic!("original procedure declaration was not published");
        };
        let definition = definition.declaration();
        let location = definition.location();
        let namespace = location.jim_namespace.as_ref().unwrap();
        let context = interp.native_jim_object_context().unwrap();
        assert_eq!(namespace.as_ptr(), context.empty_object().as_ptr());
        EXPECTED.with(|expected| {
            expected.set(Some((
                parameters.as_ptr(),
                body.as_ptr(),
                namespace.as_ptr(),
            )));
        });
        let parameter_refs = unsafe { (*parameters.as_ptr()).ref_count };
        let body_refs = unsafe { (*body.as_ptr()).ref_count };
        let head = obj::Owned::fresh(new_string(b"originals"));
        assert_eq!(interp.call_proc(&definition, &[head.as_ptr()]), Code::Ok);
        assert_eq!(interp.result_bytes(), b"observed");
        assert_eq!(unsafe { (*parameters.as_ptr()).ref_count }, parameter_refs);
        assert_eq!(unsafe { (*body.as_ptr()).ref_count }, body_refs);
        // Script preparation itself can retain the same empty filename
        // object. A cached activation must release only its frame's nsObj role.
        let namespace_refs = unsafe { (*namespace.as_ptr()).ref_count };
        assert_eq!(interp.call_proc(&definition, &[head.as_ptr()]), Code::Ok);
        assert_eq!(unsafe { (*parameters.as_ptr()).ref_count }, parameter_refs);
        assert_eq!(unsafe { (*body.as_ptr()).ref_count }, body_refs);
        assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, namespace_refs);
        assert_eq!(interp.frames.borrow().current_level(), 0);
        EXPECTED.with(|expected| expected.set(None));
    }
}
