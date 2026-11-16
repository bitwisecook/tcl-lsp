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

//! The object-system engine: creating objects and classes, dispatch and method chains, running method bodies, destruction and class linearisation.

use super::{native_context, wrong_args};
use crate::interp::{Code, Interp};
use crate::namespace::NsId;
use crate::obj::TclObj;
use tcl_core_types::OoId;

mod builtin_methods;
mod class_hierarchy;
mod construction;
mod destruction;
mod method_execution;
mod method_invocation;

pub(super) use method_invocation::oo_object_unknown;

impl Interp {
    fn retire_original_oo_object_record(&mut self, object: OoId) {
        let retired = self.oo.borrow_mut().objects.remove(&object);
        drop(retired);
        let _ = self.check_entered_native_operation();
    }

    fn retire_original_oo_class_record(&mut self, object: OoId) {
        let retired = self.oo.borrow_mut().classes.remove(&object);
        drop(retired);
        let _ = self.check_entered_native_operation();
    }

    fn mark_original_oo_record_torn_down(&self, object: OoId) {
        if let Some(owner) = self.oo.borrow_mut().objects.get_mut(&object) {
            owner.torn_down = true;
        }
    }

    /// An OO object's/class's command was renamed or deleted (e.g. `rename obj
    /// {}`): keep the OO registry in sync so the name frees up / follows the
    /// command. Tcl ties an object's lifetime to its command.
    pub(crate) fn oo_command_renamed(&mut self, object: OoId, new_fqn: Option<&[u8]>) {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self);
        if let Err(first) = &scope {
            self.refuse_native_execution(first.clone());
        }
        let owner = new_fqn
            .and_then(|_| native_context::OriginalOoTarget::capture(self, object, false).ok());
        let retired = self
            .oo
            .borrow()
            .objects
            .get(&object)
            .and_then(|owner| owner.cached_name.borrow_mut().take());
        drop(retired);
        let current = self.check_entered_native_operation();
        match new_fqn {
            Some(name) => {
                if current.is_err()
                    || owner
                        .as_ref()
                        .is_none_or(|owner| owner.check(self).is_err())
                {
                    return;
                }
                self.oo.borrow_mut().names.insert(object, name.to_vec());
            }
            None => {
                // A dismantling owner already fired its semantic role walk.
                // Final token retirement must not replay newly attached traces.
                let dismantling = self
                    .oo
                    .borrow()
                    .objects
                    .get(&object)
                    .is_some_and(|owner| owner.torn_down);
                // Deletion completes original cleanup even when the name callback refused.
                if !dismantling {
                    if self.oo.borrow().classes.contains_key(&object) {
                        self.oo_destroy_class(object);
                    } else if self.oo.borrow().objects.contains_key(&object) {
                        self.oo_destroy_bg(object);
                    }
                }
            }
        }
        let _ = self.check_entered_native_operation();
    }

    /// Destroy every OO object whose instance-variable namespace is the exact
    /// namespace token `ns`. Namespace recursion invokes this only after that
    /// token's own activation check, so an active child retains its OO state
    /// while an inactive parent generation tears down around it.
    pub(crate) fn oo_namespace_deleted(&mut self, ns: NsId) {
        if self.oo_is_empty() {
            return;
        }
        let victims: Vec<OoId> = self
            .oo
            .borrow()
            .objects
            .iter()
            // Skip objects already being torn down (e.g. the object whose own
            // namespace deletion triggered this cascade) — only the not-yet-
            // destroyed descendants are destroyed here.
            .filter(|(_, o)| o.var_ns == ns && !o.destroyed)
            .map(|(k, _)| *k)
            .collect();
        for v in victims {
            self.oo_destroy_bg(v);
        }
    }

    /// Whether the OO registry is empty (the rename hot-path early-out).
    pub(crate) fn oo_is_empty(&self) -> bool {
        let oo = self.oo.borrow();
        oo.objects.is_empty() && oo.classes.is_empty()
    }

    /// Whether `obj` has any active filter (its own object filters, or a class
    /// filter anywhere along its precedence) — so a built-in target like
    /// `destroy` must be dispatched through the filter chain, not directly.
    fn object_has_filters(&self, obj: OoId) -> bool {
        if self
            .oo
            .borrow()
            .objects
            .get(&obj)
            .is_some_and(|o| !o.filters.is_empty())
        {
            return true;
        }
        self.method_chain(obj).iter().any(|p| {
            self.oo
                .borrow()
                .classes
                .get(p)
                .is_some_and(|c| !c.filters.is_empty())
        })
    }

    /// Dispatch a command bound to a stable OO object/class identity.
    pub(crate) fn oo_dispatch(&mut self, object: OoId, argv: &[*mut TclObj]) -> Code {
        native_context::with_original_arguments(self, argv, |interp, originals| {
            let invoked = oo_original_bytes!(interp, originals, 0);
            if interp.oo.borrow().classes.contains_key(&object) {
                // The class instantiation built-ins honour the class's own
                // `export`/`unexport` for this external call.
                if let Some(sub) = if argv.len() > 1 {
                    Some(oo_original_bytes!(interp, originals, 1))
                } else {
                    None
                } {
                    if let Some(code) = interp.oo_class_factory(
                        object,
                        &invoked,
                        &sub,
                        &argv[2..],
                        true,
                        Some(argv),
                    ) {
                        return code;
                    }
                }
                match if argv.len() > 1 {
                    Some(oo_original_bytes!(interp, originals, 1))
                } else {
                    None
                }
                .as_deref()
                {
                    Some(b"destroy") => {
                        interp.oo_destroy_class(object);
                        interp.set_result_bytes(b"");
                        Code::Ok
                    }
                    // Any other subcommand is a class-object method (defined via
                    // `oo::define … interp method`); dispatch it on the class object.
                    // An unknown method funnels through `oo_invoke` →
                    // `oo_unknown_method` for the C error text.
                    Some(other) => interp.oo_invoke_with_original(
                        object,
                        other,
                        &argv[2..],
                        true,
                        Some(&invoked),
                        Some(argv),
                    ),
                    // No method name: C forces the unknown handler (`FORCE_UNKNOWN`)
                    // with an empty method, so a *user* `unknown` runs with no args.
                    // With only the default handler, report the `wrong # args` usage
                    // naming the command as invoked (`argv[0]`, not the FQN).
                    None if interp.has_user_unknown(object) => interp.oo_invoke_with_original(
                        object,
                        b"unknown",
                        &[],
                        false,
                        None,
                        Some(argv),
                    ),
                    None => {
                        let mut u = invoked;
                        u.extend_from_slice(b" method ?arg ...?");
                        wrong_args(interp, &u)
                    }
                }
            } else if interp.oo.borrow().objects.contains_key(&object) {
                match if argv.len() > 1 {
                    Some(oo_original_bytes!(interp, originals, 1))
                } else {
                    None
                } {
                    // Explicit `obj destroy` propagates a destructor error to its
                    // caller; on success the method yields the empty string. A
                    // class that unexports `destroy` (e.g. `::oo::Slot`) hides it
                    // from external calls, which then funnel through `oo_invoke` to
                    // the unknown handler instead of tearing the object down.
                    Some(m)
                        if m == b"destroy"
                            && (interp.method_exported(object, b"destroy")
                                || !interp.method_visibility_flags(object, b"destroy").1)
                            // An active filter must wrap `destroy` too, so route it
                            // through oo_invoke (which prepends the filter chain) in
                            // that case rather than tearing down directly (oo-12.2).
                            && !interp.object_has_filters(object) =>
                    {
                        interp.oo_destroy(object)
                    }
                    Some(method) => interp.oo_invoke_with_original(
                        object,
                        &method,
                        &argv[2..],
                        true,
                        Some(&invoked),
                        Some(argv),
                    ),
                    // No method name: force a *user* `unknown` (C's `FORCE_UNKNOWN`)
                    // with empty args; else the `wrong # args` usage (as invoked).
                    None if interp.has_user_unknown(object) => interp.oo_invoke_with_original(
                        object,
                        b"unknown",
                        &[],
                        false,
                        None,
                        Some(argv),
                    ),
                    None => {
                        let mut u = invoked;
                        u.extend_from_slice(b" method ?arg ...?");
                        wrong_args(interp, &u)
                    }
                }
            } else {
                interp.invalid_command(&interp.oo_name(object))
            }
        })
    }

    /// Tcl-facing command spelling for one stable TclOO identity.
    pub(crate) fn oo_name(&self, id: OoId) -> Vec<u8> {
        self.oo.borrow().name(id).to_vec()
    }
}
