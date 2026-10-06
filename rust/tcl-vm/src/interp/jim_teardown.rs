// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retirement of the actual Jim interpreter's physical owners.

use super::{Code, InterpId, Vm};
use tcl_runtime_api::jim_interpreter::{JimInterpreterObjectRole as Role, JimInterpreterTeardown};

impl Vm {
    pub(super) fn retire_jim_interpreter(&mut self) {
        if self.jim_teardown_started
            || self
                .actual_native_invocation_dialect()
                .native_string_protocol()
                != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return;
        }
        self.jim_teardown_started = true;
        tcl_runtime_api::jim_interpreter::teardown_jim_interpreter(self);
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            context.retire();
        }
    }

    fn invoke_jim_frame_defers(&mut self) {
        let Some(level) = self.frames.len().checked_sub(1) else {
            return;
        };
        let Ok(context) = self.native_jim_object_context() else {
            return;
        };
        let Ok(name) = self.native_name_operand_bytes(&context.defer_object()) else {
            return;
        };
        // JimInvokeDefer checks this exact frame table before name lookup.
        let present = if level == 0 {
            self.ns_vars
                .get(&tcl_core_types::ROOT_NS)
                .is_some_and(|table| table.get(&name).is_some())
        } else {
            self.frames[level].locals.get(&name).is_some()
        };
        if !present {
            return;
        }
        let Some(scripts) = self.get_var_from_bytes(level, &name) else {
            return;
        };
        let Ok(elements) = self.native_object_list_elements_in(
            &scripts,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        ) else {
            return;
        };
        let saved_result = context.result_object();
        context.publish_result(&context.empty_object());
        let mut completed = true;
        for script in elements.iter().rev() {
            let completion = self.eval_value_at_level(level, script);
            if completion.code != Code::Ok {
                completed = false;
                break;
            }
        }
        if completed {
            context.publish_result(&saved_result);
        }
    }
}

impl JimInterpreterTeardown for Vm {
    fn release_frames(&mut self) {
        while !self.frames.is_empty() {
            self.invoke_jim_frame_defers();
            // Jim frees locals without Tcl variable-trace callbacks. Unbind
            // each departing table while retaining procedure-owned statics.
            let mut frame = self.frames.pop().expect("selected live frame");
            self.clean_jim_local_commands(std::mem::take(&mut frame.jim_local_commands));
            frame.release_native_jim_owners();
            for (_, cell) in frame.into_local_bindings() {
                self.var_arena.unbind(cell);
            }
        }
        let root_namespace = self
            .name_world
            .borrow_mut()
            .jim_root_namespace_object
            .take();
        drop(root_namespace);
        for (_, table) in std::mem::take(&mut self.ns_vars) {
            for (_, cell) in table {
                self.var_arena.unbind(cell);
            }
        }
        drop(std::mem::take(&mut self.const_vars));
    }

    fn release_commands(&mut self) {
        let nodes = {
            let mut world = self.name_world.borrow_mut();
            for node in world.jim_command_nodes.values() {
                node.unpublish();
            }
            world.jim_command_receipts.clear();
            std::mem::take(&mut world.jim_command_nodes)
        };
        drop(std::mem::take(&mut self.commands));
        drop(std::mem::take(&mut self.hidden_commands));
        drop(std::mem::take(&mut self.module_procs));
        drop(nodes);
        drop(std::mem::take(&mut self.var_arena));
    }

    fn release_object(&mut self, role: Role) {
        if role == Role::StackTrace {
            drop(std::mem::take(&mut self.jim_errors.stack));
        }
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            context.release_object(role);
        }
    }

    fn invalidate_procedure_epoch(&mut self) {
        self.name_world.borrow_mut().advance_jim_procedure_epoch();
    }

    fn release_packages_and_associations(&mut self) {
        drop(std::mem::take(&mut self.package_state));
    }

    fn release_trace_command(&mut self) {
        // This backend has no installed Jim traceCmd or assocData owner.
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        // A parked child has its own native tables and roles. Enter it while
        // the VM can still execute its deferred scripts.
        let previous = self.cur;
        for index in 0..self.interps.len() {
            let id = InterpId(index);
            if id != self.cur && self.interps[index].parked.is_none() {
                continue;
            }
            self.switch_to(id);
            self.retire_jim_interpreter();
        }
        self.switch_to(previous);
        self.retire_jim_interpreter();
    }
}
