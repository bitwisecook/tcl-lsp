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

//! `oo::copy` and the `<cloned>` built-in: cloning an object, its methods and its namespace.

use super::{err, native_context, wrong_args, Object, OriginalObjectPublication};
use crate::interp::{Code, Command, Interp};
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;
use std::cell::RefCell;
use std::rc::Rc;
use tcl_core_types::OoId;

/// `<cloned>` object built-in (C's `TclOO_Object_Cloned`): copy the procedures
/// and variables of the origin object's instance namespace into this object's.
/// Invoked by `oo::copy`; a user `<cloned>` runs first and reaches this via
/// `next`.
pub(super) fn oo_object_cloned(interp: &mut Interp, obj: OoId, args: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, args, |interp, originals| {
        if args.len() != 1 {
            return wrong_args(interp, b"my <cloned> originObject");
        }
        let src = oo_original_object!(interp, originals, 0);
        let pair = {
            let oo = interp.oo.borrow();
            match (oo.objects.get(&src), oo.objects.get(&obj)) {
                (Some(s), Some(d)) => Some((s.var_ns, d.var_ns)),
                _ => None,
            }
        };
        if let Some((src_ns, dst_ns)) = pair {
            if let Err(code) = interp.oo_clone_namespace(src_ns, dst_ns) {
                return code;
            }
        }
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

/// `oo::copy srcObject ?targetObject?` — clone an object (its class + per-object
/// methods/mixins).
pub(super) fn oo_copy_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 2 || argv.len() > 4 {
            return wrong_args(
                interp,
                b"oo::copy sourceName ?targetName? ?targetNamespace?",
            );
        }
        // The source resolves like a command (current namespace then global), so a
        // copy from inside a namespace still finds a global object (oo-15.1).
        let src = oo_original_object!(interp, originals, 1);
        let source = {
            let state = interp.oo.borrow();
            state
                .objects
                .get(&src)
                .map(|source| {
                    for variable in source.variables.iter().chain(&source.private_variables) {
                        variable.check_originals()?;
                    }
                    Ok::<_, tcl_syntax::value::ValueError>(source.clone())
                })
                .transpose()
        };
        let Some(mut src_obj) = oo_checked!(interp, source) else {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(&oo_original_bytes!(interp, originals, 1));
            m.extend_from_slice(b"\" does not refer to an object");
            return err(interp, &m);
        };
        let duplicated = src_obj.methods.duplicate_declarations();
        oo_original_check!(interp, originals);
        src_obj.methods = match duplicated {
            Ok(methods) => methods,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        // An empty (or omitted) target name auto-generates an anonymous name.
        let original_dst = match if argv.len() > 2 {
            Some(oo_original_bytes!(interp, originals, 2))
        } else {
            None
        } {
            Some(n) if !n.is_empty() => n,
            _ => {
                let n = interp.oo.borrow().counter;
                interp.oo.borrow_mut().counter += 1;
                format!("::oo::Obj{n}").into_bytes()
            }
        };
        let publication = match interp.oo_publication(&original_dst) {
            Ok(publication) => publication,
            Err(code) => return code,
        };
        let dst = publication.report.clone();
        // If the source is also a class, clone the class definition too, so the
        // copy is a working class (TclOO copies both the object and class facets).
        let source = {
            let state = interp.oo.borrow();
            state
                .classes
                .get(&src)
                .map(|source| {
                    for variable in source.variables.iter().chain(&source.private_variables) {
                        variable.check_originals()?;
                    }
                    Ok::<_, tcl_syntax::value::ValueError>(source.clone())
                })
                .transpose()
        };
        let mut src_cls = oo_checked!(interp, source);
        if let Some(class) = &mut src_cls {
            let methods = class.methods.duplicate_declarations();
            oo_original_check!(interp, originals);
            let methods = match methods {
                Ok(methods) => methods,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            let constructor = class.constructor.duplicate_declaration();
            oo_original_check!(interp, originals);
            let constructor = match constructor {
                Ok(constructor) => constructor,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            let destructor = class.destructor.duplicate_declaration();
            oo_original_check!(interp, originals);
            let destructor = match destructor {
                Ok(destructor) => destructor,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            class.methods = methods;
            class.constructor = constructor;
            class.destructor = destructor;
        }
        // An explicit target namespace becomes the copy's instance-variable
        // namespace; otherwise it allocates a fresh hidden native namespace.
        let var_ns = match if argv.len() > 3 {
            Some(oo_original_bytes!(interp, originals, 3))
        } else {
            None
        } {
            Some(ref ns) if !ns.is_empty() => {
                // The target namespace is *created*; an existing one is an error
                // (C's `TclOO_Copy`; oo-15.12).
                if oo_checked!(interp, interp.resolve_namespace_name(ns)).is_some() {
                    let mut m = ns.clone();
                    m.extend_from_slice(b" refers to an existing namespace");
                    return err(interp, &m);
                }
                oo_checked!(interp, interp.ensure_namespace_address(ns))
            }
            _ => interp.fresh_native_oo_namespace(),
        };
        native_context::install_object_helpers(interp, var_ns);
        let creation_id = interp.oo_next_id();
        let dst_id = interp.oo.borrow_mut().allocate(dst.clone());
        interp.oo.borrow_mut().objects.insert(
            dst_id,
            Object {
                cached_name: Rc::new(RefCell::new(None)),
                class: src_obj.class,
                var_ns,
                creation_id,
                methods: src_obj.methods,
                mixins: src_obj.mixins,
                unexported: src_obj.unexported,
                exported: src_obj.exported,
                private: src_obj.private,
                filters: src_obj.filters,
                variables: src_obj.variables,
                private_variables: src_obj.private_variables,
                readable_properties: src_obj.readable_properties,
                writable_properties: src_obj.writable_properties,
                destroyed: false,
                torn_down: false,
            },
        );
        if let Some(mut cls) = src_cls {
            // The genuine copied Class is a new facet, not a shallow observation.
            cls.facet_birth = Rc::new(());
            interp.oo.borrow_mut().classes.insert(dst_id, cls);
        }
        let original =
            match OriginalObjectPublication::capture(interp, dst_id, originals.currency()) {
                Ok(original) => original,
                Err(error) => return interp.oo_refused_publication(dst_id, error),
            };
        interp.bind_command_replacement(
            publication.namespace,
            &publication.simple,
            Command::OoObject(dst_id),
        );
        let continued = original
            .check(interp, originals.currency())
            .and_then(|()| interp.oo_register_my(dst_id));
        if let Err(error) = continued {
            return interp.oo_refused_publication(dst_id, error);
        }
        // Run the `<cloned>` method (a user override first, then oo::object's native
        // implementation via `next`) to copy the instance namespace's procedures
        // and variables from the source (oo-15.6, oo-15.8).
        let src_obj = obj::new_string_bytes(&interp.oo_name(src));
        unsafe { obj::incr_ref_count(src_obj) };
        // A `wrong # args` from a user `<cloned>` names the new object (C reports the
        // copy as `<obj> <cloned> …`), not the internal `my` (oo-15.9). Prime the
        // usage prefix the method body's run_proc consumes.
        let mut usage = dst.clone();
        usage.extend_from_slice(b" <cloned>");
        interp.oo.borrow_mut().fwd_usage = Some(usage);
        let code = interp.oo_invoke(dst_id, b"<cloned>", &[src_obj], false, None);
        interp.oo.borrow_mut().fwd_usage = None;
        unsafe { obj::decr_ref_count(src_obj) };
        oo_original_check!(interp, originals);
        if code == Code::Error {
            return Code::Error;
        }
        interp.set_result(obj::new_string_bytes(&dst));
        Code::Ok
    })
}

impl Interp {
    /// Copy the procedures and variables of namespace `src_ns` into `dst_ns`
    /// (C's `TclCopyNamespaceProcedures`/`Variables`, used by `<cloned>`).
    pub(super) fn oo_clone_namespace(&mut self, src_ns: NsId, dst_ns: NsId) -> Result<(), Code> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let source_holder = self.native_namespace_token_at(src_ns).ok_or_else(|| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original clone source namespace",
                ),
            )
        })?;
        let destination_holder = self.native_namespace_token_at(dst_ns).ok_or_else(|| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original clone destination namespace",
                ),
            )
        })?;
        let source_lifecycle = source_holder.lifecycle();
        let destination_lifecycle = destination_holder.lifecycle();
        if !self.original_publication_namespace_is_current(src_ns, &source_holder, source_lifecycle)
            || !self.original_publication_namespace_is_current(
                dst_ns,
                &destination_holder,
                destination_lifecycle,
            )
        {
            return Err(Code::Error);
        }
        // Procedures (skip built-ins like the per-object `my`/`myclass`).
        let names: Vec<Vec<u8>> = self
            .namespaces()
            .command_names(src_ns)
            .iter()
            .map(|n| n.to_vec())
            .collect();
        // The destination namespace's FQN, for re-pointing copied procs.
        let dst_qual = self.namespaces().qualified_name(dst_ns);
        for n in &names {
            // Bind the resolved command before borrowing mutably (the `Ref` from
            // `namespaces()` must be dropped first).
            let cmd = self.namespaces().resolve(src_ns, n);
            if let Some(Command::Proc(def)) = cmd {
                // Re-point the copy to the destination namespace, so its body's
                // `variable`/unqualified names resolve there, not in the source.
                let fqn = tcl_syntax::naming::rooted_member_key_bytes(&dst_qual, n);
                let duplicate = def.duplicate(dst_ns, fqn);
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                if !self.original_publication_namespace_is_current(
                    src_ns,
                    &source_holder,
                    source_lifecycle,
                ) || !self.original_publication_namespace_is_current(
                    dst_ns,
                    &destination_holder,
                    destination_lifecycle,
                ) {
                    return Err(Code::Error);
                }
                let duplicate = duplicate.map_err(|error| self.report_cmd_error(error.into()))?;
                self.bind_command_replacement(dst_ns, n, Command::Proc(duplicate));
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                if !self.original_publication_namespace_is_current(
                    src_ns,
                    &source_holder,
                    source_lifecycle,
                ) || !self.original_publication_namespace_is_current(
                    dst_ns,
                    &destination_holder,
                    destination_lifecycle,
                ) {
                    return Err(Code::Error);
                }
            }
        }
        if !self.original_publication_namespace_is_current(src_ns, &source_holder, source_lifecycle)
            || !self.original_publication_namespace_is_current(
                dst_ns,
                &destination_holder,
                destination_lifecycle,
            )
        {
            return Err(Code::Error);
        }
        // Variables (scalars and array elements; `store_*` retains the values).
        type ArrayCopy = (Vec<u8>, Vec<(Vec<u8>, obj::NativeObjectLifetime)>);
        let mut scalars: Vec<(Vec<u8>, obj::NativeObjectLifetime)> = Vec::new();
        let mut arrays: Vec<ArrayCopy> = Vec::new();
        {
            let ns = self.namespaces();
            let table = ns.var_table(src_ns);
            for name in ns.var_names(src_ns) {
                match table.cell(&name).as_deref() {
                    Some(crate::frame::Var::Scalar(p)) => {
                        scalars.push((name, obj::NativeObjectLifetime::retain(*p)))
                    }
                    Some(crate::frame::Var::Array(map)) => {
                        arrays.push((
                            name,
                            map.iter()
                                .map(|(k, v)| (k.clone(), obj::NativeObjectLifetime::retain(*v)))
                                .collect(),
                        ));
                    }
                    _ => {}
                }
            }
        }
        for (name, value) in scalars {
            if !self.original_publication_namespace_is_current(
                dst_ns,
                &destination_holder,
                destination_lifecycle,
            ) {
                return Err(Code::Error);
            }
            let reached =
                self.store_captured_byte_variable(dst_ns, None, &name, None, value.as_ptr());
            drop(value);
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            if !self.original_publication_namespace_is_current(
                dst_ns,
                &destination_holder,
                destination_lifecycle,
            ) {
                return Err(Code::Error);
            }
            reached.map_err(|error| crate::builtins::var_error(self, &name, error))?;
        }
        for (name, elems) in arrays {
            for (key, value) in elems {
                if !self.original_publication_namespace_is_current(
                    dst_ns,
                    &destination_holder,
                    destination_lifecycle,
                ) {
                    return Err(Code::Error);
                }
                let reached = self.store_captured_byte_variable(
                    dst_ns,
                    None,
                    &name,
                    Some(&key),
                    value.as_ptr(),
                );
                drop(value);
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                if !self.original_publication_namespace_is_current(
                    dst_ns,
                    &destination_holder,
                    destination_lifecycle,
                ) {
                    return Err(Code::Error);
                }
                reached.map_err(|error| crate::builtins::var_error(self, &name, error))?;
            }
        }
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        Ok(())
    }
}
