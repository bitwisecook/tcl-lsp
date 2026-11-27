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

//! Registering the `oo::*` commands, the root classes, the definition namespaces and the singleton and abstract support classes.

use super::configurable::install_configurable;
use super::copying::{oo_copy_cmd, oo_object_cloned};
use super::definition::{
    def_class, def_constructor, def_definitionnamespace, def_deletemethod, def_destructor,
    def_export, def_forward, def_method, def_private, def_renamemethod, oo_define_cmd,
    oo_objdefine_cmd,
};
use super::engine::oo_object_unknown;
use super::method_context::{classvariable_cmd, next_cmd, nextto_cmd, self_cmd};
use super::slots::{install_slot_class, install_slot_instances};
use super::{
    native_context, Class, FoundationMethodReports, Method, Object, OriginalObjectPublication,
};
use crate::interp::{Command, Interp};
use crate::obj;
use tcl_native_model::native_tcloo_method_definition::{
    NativeTclooCoreMethod, NativeTclooMethodPayloadType, NativeTclooMethodReportProtocol,
};

/// Register the `oo::*` commands and the definition / context commands.
pub fn install(interp: &mut Interp) {
    let scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
    {
        Ok(scope) => scope,
        Err(error) => {
            interp.report_cmd_error(error.into());
            return;
        }
    };
    let reports = NativeTclooMethodReportProtocol::select(interp.native_invocation_dialect())
        .map(FoundationMethodReports::new);
    interp.oo.borrow_mut().method_reports = reports;
    interp.oo.borrow_mut().define_name =
        Some(obj::Owned::fresh(obj::new_string_bytes(b"::oo::define")));
    interp.register_stock_builtin(b"oo::define", oo_define_cmd);
    interp.register_stock_builtin(b"oo::objdefine", oo_objdefine_cmd);
    interp.register_stock_builtin(b"oo::copy", oo_copy_cmd);
    // Method-context commands. `my` is created *per object* (in each object's
    // namespace) like C TclOO — never global — so a test's `rename ::my {}`
    // can't break it. `self`/`next` resolve via the call stack.
    interp.register_stock_builtin(b"::oo::Helpers::self", native_context::helper_self);
    interp.register_stock_builtin(b"::oo::Helpers::next", next_cmd);
    interp.register_stock_builtin(b"::oo::Helpers::nextto", nextto_cmd);
    interp.register_stock_builtin(b"classvariable", classvariable_cmd);
    // Root classes (so `superclass`-less classes inherit `object` and
    // `superclass oo::class`/`oo::object` validate). Both are themselves
    // objects (instances of `::oo::class`) and dispatch through `oo_dispatch`
    // like any other class command — so `create`/`new`/`destroy`, unknown-
    // method errors and the empty-name check are all handled uniformly.
    let (object_root, class_root) = {
        let mut oo = interp.oo.borrow_mut();
        let object_root = oo.allocate(b"::oo::object".to_vec());
        let class_root = oo.allocate(b"::oo::class".to_vec());
        oo.property_foundation_epoch = 1;
        if interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
        {
            oo.property_my_name = Some(obj::Owned::fresh(obj::new_string_bytes(b"my")));
        }
        oo.object_root = Some(object_root);
        oo.class_root = Some(class_root);
        oo.classes.insert(object_root, Class::default());
        oo.classes.insert(
            class_root,
            Class {
                supers: vec![object_root],
                ..Class::default()
            },
        );
        (object_root, class_root)
    };
    // `oo::object`'s instance-definition namespace (TIP 524, `-instance`) is
    // `::oo::objdefine` — defining any object (via `oo::objdefine`) runs there.
    // (`oo::class`'s `-class` namespace, `::oo::define`, is handled in the
    // `definitionnamespace` introspection.)
    if let Some(c) = interp.oo.borrow_mut().classes.get_mut(&object_root) {
        c.def_ns = Some(b"::oo::objdefine".to_vec());
    }
    // Defining a class (an instance of `oo::class`) happens in `::oo::define`.
    if let Some(c) = interp.oo.borrow_mut().classes.get_mut(&class_root) {
        c.class_def_ns = Some(b"::oo::define".to_vec());
    }
    for (fqn, id) in [
        (b"::oo::object".as_slice(), object_root),
        (b"::oo::class".as_slice(), class_root),
    ] {
        let var_ns = interp.fresh_native_oo_namespace();
        native_context::install_object_helpers(interp, var_ns);
        let creation_id = interp.oo_next_id();
        interp
            .oo
            .borrow_mut()
            .objects
            .insert(id, Object::new(class_root, var_ns, creation_id));
        let original = match OriginalObjectPublication::capture(interp, id, scope.currency()) {
            Ok(original) => original,
            Err(error) => {
                interp.oo_refused_publication(id, error);
                return;
            }
        };
        interp.ns_register(fqn, Command::OoObject(id));
        if let Err(error) = original.check(interp, scope.currency()) {
            interp.oo_refused_publication(id, error);
            return;
        }
        // Engine-installed, not script-created: the registry dates these
        // (TCL86_PLUS) and the availability gate must honour that.
        interp.declare_registry_object_root(id, fqn);
        if let Err(error) = interp.oo_register_my(id) {
            interp.oo_refused_publication(id, error);
            return;
        }
    }
    // `oo::object` has a built-in (unexported) `unknown` method — the standard
    // "unknown method" error, and the terminus of the `unknown` call chain.
    if let Some(c) = interp.oo.borrow_mut().classes.get_mut(&object_root) {
        drop(c.methods.insert_generated(
            b"unknown".to_vec(),
            Method::builtin(
                oo_object_unknown,
                NativeTclooMethodPayloadType::Core(NativeTclooCoreMethod::Unknown),
            ),
        ));
        c.unexported.insert(b"unknown".to_vec());
        // `<cloned>` copies the instance namespace during `oo::copy`; reachable
        // via `next` from a user-defined `<cloned>` override.
        drop(c.methods.insert_generated(
            b"<cloned>".to_vec(),
            Method::builtin(
                oo_object_cloned,
                NativeTclooMethodPayloadType::Core(NativeTclooCoreMethod::Cloned),
            ),
        ));
        c.unexported.insert(b"<cloned>".to_vec());
    }
    // Script initialization requires the actual installed OO root command.
    // An unavailable root must not create an error episode during construction.
    if interp
        .resolve_dispatchable(crate::namespace::GLOBAL, b"::oo::class")
        .is_none()
    {
        return;
    }
    let _ =
        interp.eval_str(b"namespace eval ::oo {variable version 1.3.1; variable patchlevel 1.3.1}");
    // Definition workers are original commands in their actual support namespaces.
    let _ = interp.eval_str(b"namespace eval ::oo::define {}; namespace eval ::oo::objdefine {}");
    register_define_ns_commands(interp);
    install_slot_class(interp);
    if interp.native_execution_refusal().is_some() {
        return;
    }
    let _ = interp.eval_str(b"namespace eval ::oo::configuresupport {}");
    // The seven definition slots and the four TIP 558 property slots are real
    // `::oo::Slot` instances (C's `TclOODefineSlots`), invoked e.g. as
    // `oo::define c ::oo::configuresupport::readableproperties -set …`. Their
    // per-instance `Get`/`Set` read/write the active definition target's lists.
    install_slot_instances(interp);
    if interp.native_execution_refusal().is_some() {
        return;
    }
    install_configurable(interp);
    if interp.native_execution_refusal().is_some() {
        return;
    }
    native_context::install_unknown(interp);
}

/// Register the definition subcommands as real commands in `::oo::define` /
/// `::oo::objdefine` (e.g. `::oo::define::method`). Invoked outside a definition
/// context they report C's "this command may only be called …" error; inside
/// one they dispatch like the bare definition subcommand. The list-valued slots
/// (`filter`/`mixin`/`superclass`/`variable`) are *not* listed here: they are
/// real `::oo::Slot` instances installed by `install_slot_instances`.
fn register_define_ns_commands(interp: &mut Interp) {
    let workers: &[(
        &[u8],
        crate::interp::BuiltinFn,
        Option<crate::interp::BuiltinFn>,
    )] = &[
        (
            b"constructor",
            |i, a| native_context::definition_worker(i, a, true, def_constructor),
            None,
        ),
        (
            b"definitionnamespace",
            |i, a| native_context::definition_worker(i, a, true, def_definitionnamespace),
            None,
        ),
        (
            b"deletemethod",
            |i, a| native_context::definition_worker(i, a, true, def_deletemethod),
            Some(|i, a| native_context::definition_worker(i, a, false, def_deletemethod)),
        ),
        (
            b"destructor",
            |i, a| native_context::definition_worker(i, a, true, def_destructor),
            None,
        ),
        (
            b"export",
            |i, a| native_context::definition_worker(i, a, true, |i, a| def_export(i, a, true)),
            Some(|i, a| {
                native_context::definition_worker(i, a, false, |i, a| def_export(i, a, true))
            }),
        ),
        (
            b"forward",
            |i, a| native_context::definition_worker(i, a, true, def_forward),
            Some(|i, a| native_context::definition_worker(i, a, false, def_forward)),
        ),
        (
            b"method",
            |i, a| native_context::definition_worker(i, a, true, def_method),
            Some(|i, a| native_context::definition_worker(i, a, false, def_method)),
        ),
        (
            b"private",
            |i, a| native_context::definition_worker(i, a, true, def_private),
            Some(|i, a| native_context::definition_worker(i, a, false, def_private)),
        ),
        (
            b"renamemethod",
            |i, a| native_context::definition_worker(i, a, true, def_renamemethod),
            Some(|i, a| native_context::definition_worker(i, a, false, def_renamemethod)),
        ),
        (
            b"self",
            |i, a| native_context::definition_worker(i, a, true, self_cmd),
            Some(|i, a| native_context::definition_worker(i, a, false, self_cmd)),
        ),
        (
            b"unexport",
            |i, a| native_context::definition_worker(i, a, true, |i, a| def_export(i, a, false)),
            Some(|i, a| {
                native_context::definition_worker(i, a, false, |i, a| def_export(i, a, false))
            }),
        ),
    ];
    for &(name, class_handler, object_handler) in workers {
        let mut qualified = b"::oo::define::".to_vec();
        qualified.extend_from_slice(name);
        interp.ns_register(&qualified, Command::Builtin(class_handler));
        if let Some(handler) = object_handler {
            let mut qualified = b"::oo::objdefine::".to_vec();
            qualified.extend_from_slice(name);
            interp.ns_register(&qualified, Command::Builtin(handler));
        }
    }
    interp.ns_register(
        b"::oo::objdefine::class",
        Command::Builtin(|i, a| native_context::definition_worker(i, a, false, def_class)),
    );
}

/// Foundation support bootstrap. The existing singleton workers retain
/// their separate installation schedule; the abstract class consumes the
/// release-selected C9 initializer from the shared Registry descriptor.
pub(super) fn install_abstract_singleton(interp: &mut Interp) {
    // `oo::singleton`: a metaclass (superclass oo::class) whose instances only
    // permit a single instance; it unexports `create`/`createWithNamespace`.
    let _ = interp.eval_str(
        b"oo::class create ::oo::singleton { superclass ::oo::class }\n\
          oo::define ::oo::singleton unexport create createWithNamespace",
    );
    // `oo::SingletonInstance`: a plain class (superclass oo::object) mixed into
    // singleton instances so they can't easily be destroyed or cloned.
    let _ = interp.eval_str(b"oo::class create ::oo::SingletonInstance {}");
    // The shared release recipe describes the C9 class initializer only.
    // The existing script workers still create and publish its genuine class.
    if let Some(recipe) = tcl_registry::native_tcloo_bootstrap::NativeAbstractClassBootstrap::select(
        interp.native_invocation_dialect(),
    ) {
        let _ = interp.eval_str(recipe.initialization_script());
    }
}
