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

//! Destroying objects and classes: the destructor chain, background teardown and a class's descendants.

use super::super::slots::build_list;
use super::super::{native_method_cache, CallStep, MethodInvocation};
use crate::interp::{Code, Interp};
use tcl_core_types::OoId;
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;

impl Interp {
    /// Destroy an object during implicit teardown, routing a destructor error to
    /// the `interp bgerror` handler (C's `AfterNRDestructor` → background error).
    pub(in super::super) fn oo_destroy_bg(&mut self, obj: OoId) {
        if self.native_execution_refusal().is_some() {
            // A pending first cause skips the destructor and its background
            // report; mandatory teardown still completes.
            let _ = self.oo_destroy(obj);
            return;
        }
        let outcome = self.with_preserved_result_checked(|interp| {
            if interp.oo_destroy(obj) == Code::Error {
                let msg = match interp.completion_result_bytes_checked() {
                    Ok(message) => message,
                    Err(_) => return,
                };
                let ec = match interp.error_code_bytes_checked() {
                    Ok(code) => code,
                    Err(_) => return,
                };
                let options = match build_list(
                    interp,
                    &[
                        b"-code".to_vec(),
                        b"1".to_vec(),
                        b"-level".to_vec(),
                        b"0".to_vec(),
                        b"-errorcode".to_vec(),
                        ec,
                        b"-errorinfo".to_vec(),
                        msg.clone(),
                    ],
                ) {
                    Ok(options) => options,
                    Err(error) => {
                        interp.report_cmd_error(error);
                        return;
                    }
                };
                interp.report_bg_error(&msg, &options);
            }
        });
        if let Err(first) = outcome {
            self.refuse_native_execution(first);
        }
    }

    /// Destroy an object. Returns the destructor's result `Code`: explicit
    /// `obj destroy` propagates a destructor error to its caller (C's
    /// `AfterNRDestructor` returns the destructor result), whereas implicit
    /// teardown (`rename {}`/`namespace delete`/class cascade) reports it to
    /// `bgerror`. Cleanup of the command/namespace happens regardless of the
    /// destructor outcome.
    pub(in super::super) fn oo_destroy(&mut self, obj: OoId) -> Code {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self);
        if let Err(first) = &scope {
            self.refuse_native_execution(first.clone());
        }
        // Retain the actual instance namespace before any result/free callback.
        let original = self
            .oo
            .borrow()
            .objects
            .get(&obj)
            .map(|owner| (owner.destroyed, owner.var_ns));
        let mut dtor_code = Code::Ok;
        let Some((destroyed, var_ns)) = original else {
            return if self.check_entered_native_operation().is_err() {
                Code::Error
            } else {
                Code::Ok
            };
        };
        if !destroyed {
            // Set the existing reentrancy guard before a result retirement can call back.
            if let Some(owner) = self.oo.borrow_mut().objects.get_mut(&obj) {
                owner.destroyed = true;
            }
            if self.check_entered_native_operation().is_ok() {
                self.set_result_bytes(b"");
                if self.check_entered_native_operation().is_ok() {
                    dtor_code = self.oo_run_destructors(obj);
                }
            }
        } else if self.check_entered_native_operation().is_ok() {
            self.set_result_bytes(b"");
        }
        let _ = self.check_entered_native_operation();
        // The destructor has run; the object is now being dismantled. Mark it
        // torn-down *before* the descendant cascade so a nested-owned child whose
        // destructor calls back into this object sees "impossible to invoke
        // method" (C guts the object in `ObjectNamespaceDeleted` before its
        // child namespaces are deleted). The object stays in the registry — and
        // its command resolvable — until cleanup below.
        if let Some(o) = self.oo.borrow_mut().objects.get_mut(&obj) {
            o.torn_down = true;
        }
        // If the object is also a class, its subclasses and instances are
        // descendants too (C's `TclOODeleteDescendants`): destroy them, so e.g.
        // tearing down a class reached as another metaclass's instance also tears
        // down that class's own instances (oo-16.14 pollution).
        if self.oo.borrow().classes.contains_key(&obj) {
            self.oo_destroy_class_descendants(obj);
        }
        // Tcl retires all three command roles before tearing down the instance
        // namespace: public object, `myclass`, then `my`. An earlier callback
        // may relocate a later dispatcher, so the centralized role walk
        // re-resolves every token immediately before firing it.
        self.fire_oo_command_role_delete_traces(obj);
        let _ = self.check_entered_native_operation();
        // Private dispatchers stop existing before object-variable unset
        // callbacks. The public object command deliberately remains visible
        // through that namespace phase, matching Tcl's `FreeObject` boundary.
        self.remove_prefired_oo_private_commands(obj);
        let _ = self.check_entered_native_operation();
        // Then delete the instance namespace — this unsets its variables and
        // fires their unset traces while the object is still registered and
        // torn-down, so a trace callback sees `info object isa object` true, the
        // namespace already gone, and a method call as "impossible to invoke
        // method" (C's ObjectNamespaceDeleted order; oo-11.8).
        self.delete_namespace_by_id(var_ns);
        let _ = self.check_entered_native_operation();
        // Finally retire every remaining public/private command token by identity,
        // wherever rename or hide moved it, before dropping the registry
        // records those delete callbacks may inspect.
        self.retire_prefired_oo_command_identity(obj);
        let _ = self.check_entered_native_operation();
        let retired_name = self
            .oo
            .borrow()
            .objects
            .get(&obj)
            .and_then(|owner| owner.cached_name.borrow_mut().take());
        drop(retired_name);
        let _ = self.check_entered_native_operation();
        let class = self.oo.borrow().classes.contains_key(&obj);
        self.native_method_structure_changed(obj, class);
        let _ = self.retire_native_properties(obj);
        self.retire_original_oo_object_record(obj);
        self.retire_original_oo_class_record(obj);
        self.oo.borrow_mut().names.remove(&obj);
        if self.check_entered_native_operation().is_err() {
            Code::Error
        } else {
            dtor_code
        }
    }

    /// Run the object's destructor chain (every class in the MRO that declares a
    /// `destructor`, most-derived first, with `next` chaining), with the object
    /// still intact so the body's `my`/variables resolve.
    fn oo_run_destructors(&mut self, obj: OoId) -> Code {
        let Some(class) = self.oo.borrow().objects.get(&obj).map(|o| o.class) else {
            return Code::Ok;
        };
        let chain: Vec<CallStep> = self
            .mro(class)
            .into_iter()
            .filter(|c| {
                self.oo
                    .borrow()
                    .classes
                    .get(c)
                    .is_some_and(|cl| cl.destructor.is_some())
            })
            .map(|c| CallStep {
                method_owner: None,
                is_object: false,
                is_filter: false,
                synthetic_type: None,
                declaring_facet: None,
                provider: c,
                method: b"<destructor>".to_vec(),
            })
            .collect();
        if chain.is_empty() {
            return Code::Ok;
        }
        // The destructor result is returned to the caller: explicit `obj
        // destroy` propagates it (C's `AfterNRDestructor`); implicit teardown
        // ignores it (routing to `bgerror`).
        let chain = {
            let oo = self.oo.borrow();
            native_method_cache::ReachedMethodChain::from(native_method_cache::retain_owners(
                &oo,
                chain,
                NativeTclooMethodReportRole::Destructor,
            ))
            .with_report_role(NativeTclooMethodReportRole::Destructor)
        };
        self.oo_run(
            obj,
            chain,
            0,
            b"<destructor>",
            &[],
            MethodInvocation::internal(),
        )
    }

    pub(in super::super) fn oo_destroy_class(&mut self, class: OoId) {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self);
        if let Err(first) = &scope {
            self.refuse_native_execution(first.clone());
        }
        // Already being torn down (a cycle of mixins/superclasses, or reached
        // again via an instance cascade) — nothing more to do.
        if self
            .oo
            .borrow()
            .objects
            .get(&class)
            .is_none_or(|o| o.destroyed)
        {
            return;
        }
        // Destroying a class cascades to its subclasses and instances (TclOO),
        // then removes the class object itself. Mark it destroyed first so the
        // cascade does not re-enter it.
        if let Some(o) = self.oo.borrow_mut().objects.get_mut(&class) {
            o.destroyed = true;
            o.torn_down = true;
        }
        let var_ns = self
            .oo
            .borrow()
            .objects
            .get(&class)
            .map(|owner| owner.var_ns);
        self.oo_destroy_class_descendants(class);
        let _ = self.check_entered_native_operation();
        // Drop the registry entries, the command, the per-object `my`/`myclass`,
        // and the class's own instance namespace — a class created with an
        // explicit target namespace (`oo::copy … <ns>`) owns it, so destroying
        // the class must delete it too (oo-15.13.x), as `oo_destroy` does.
        self.retire_oo_command_identity(class);
        let _ = self.check_entered_native_operation();
        self.native_method_structure_changed(class, true);
        self.retire_original_oo_class_record(class);
        let retired_name = self
            .oo
            .borrow()
            .objects
            .get(&class)
            .and_then(|owner| owner.cached_name.borrow_mut().take());
        drop(retired_name);
        let _ = self.check_entered_native_operation();
        let _ = self.retire_native_properties(class);
        self.retire_original_oo_object_record(class);
        self.oo.borrow_mut().names.remove(&class);
        if let Some(ns) = var_ns {
            self.delete_namespace_by_id(ns);
        }
        let _ = self.check_entered_native_operation();
    }

    /// Destroy a class's subclasses and instances (C's `TclOODeleteDescendants`),
    /// leaving the class object itself intact. Shared by full class destruction
    /// and by demoting a class to a plain object (oo-13.6). Already-destroyed
    /// descendants are skipped so cyclic hierarchies don't recurse forever.
    pub(in super::super) fn oo_destroy_class_descendants(&mut self, class: OoId) {
        // Subclasses first: any class that lists this one as a superclass/mixin.
        let subs: Vec<OoId> = self
            .oo
            .borrow()
            .classes
            .iter()
            .filter(|(k, c)| {
                **k != class && (c.supers.contains(&class) || c.mixins.contains(&class))
            })
            .map(|(k, _)| *k)
            .filter(|k| {
                self.oo
                    .borrow()
                    .objects
                    .get(k)
                    .is_some_and(|o| !o.destroyed)
            })
            .collect();
        for s in subs {
            self.oo_destroy_class(s);
        }
        // Then this class's direct instances *and* any object that mixes it in
        // (C's instances list includes the objects we're mixed into).
        let insts: Vec<OoId> = self
            .oo
            .borrow()
            .objects
            .iter()
            .filter(|(k, o)| {
                **k != class && !o.destroyed && (o.class == class || o.mixins.contains(&class))
            })
            .map(|(k, _)| *k)
            .collect();
        for o in insts {
            self.oo_destroy_bg(o);
        }
    }
}
