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

//! `oo::define` and `oo::objdefine`: running a definition body and the definition-script commands, from `method` and `forward` to `definitionnamespace`.

use super::configurable::native_method_body;
use super::introspection::not_object;
use super::{err, native_context, native_method_cache, wrong_args, Class, DefTarget, Method};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use std::collections::BTreeSet;
use std::rc::Rc;
use tcl_core_types::OoId;

// oo::class / oo::object / oo::define / oo::objdefine

/// `oo::define class script` or `oo::define class subcommand ?arg ...?`.
pub(super) fn oo_define_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 3 {
            return wrong_args(interp, b"oo::define target ?arg ...?");
        }
        let fqn = oo_original_object!(interp, originals, 1);
        // C resolves the object first (`Tcl_GetObjectFromObj`): a name that is not
        // an object at all reports differently from an object that is not a class.
        if !interp.oo.borrow().objects.contains_key(&fqn) {
            let name = oo_original_bytes!(interp, originals, 1);
            return not_object(interp, &name);
        }
        if !interp.oo.borrow().classes.contains_key(&fqn) {
            let mut m = oo_original_bytes!(interp, originals, 1);
            m.extend_from_slice(b" does not refer to a class");
            return err(interp, &m);
        }
        interp.oo_run_def(DefTarget::Class(fqn), argv)
    })
}

/// `oo::objdefine object script` / `oo::objdefine object subcommand ?arg ...?`.
pub(super) fn oo_objdefine_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 3 {
            return wrong_args(interp, b"oo::objdefine target ?arg ...?");
        }
        let fqn = oo_original_object!(interp, originals, 1);
        if !interp.oo.borrow().objects.contains_key(&fqn) {
            let name = oo_original_bytes!(interp, originals, 1);
            return not_object(interp, &name);
        }
        interp.oo_run_def(DefTarget::Object(fqn), argv)
    })
}

// definition-script commands

/// The current definition target, or an error if outside a definition body.
pub(super) fn def_target(interp: &mut Interp) -> Result<DefTarget, Code> {
    let target = interp.active_def_target();
    match target {
        Some(t) => {
            let object = match t {
                DefTarget::Class(object) | DefTarget::Object(object) => object,
            };
            if !interp.oo.borrow().objects.contains_key(&object) {
                let code = crate::interp::error_code_list(&[b"TCL", b"OO", b"MONKEY_BUSINESS"]);
                return Err(interp.error_with_code(
                    b"this command cannot be called when the object has been deleted",
                    &code,
                ));
            }
            Ok(t)
        }
        None => Err(native_context::monkey_business(interp)),
    }
}

/// A method's declared visibility.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum MethodVis {
    /// Callable externally (a lowercase name or `-export`).
    Public,
    /// Hidden from external calls but listed by `info methods -private`
    /// (a non-lowercase name or `-unexport`).
    Unexported,
    /// TIP 500 private (`-private`/`private` block): hidden everywhere.
    Private,
}

/// Install `method` into the current definition target (class or object) with
/// the default visibility for its name (or `private` block).
fn install_method(interp: &mut Interp, name: Vec<u8>, original: *mut TclObj, m: Method) -> Code {
    let vis = default_vis(interp, &name);
    install_original_method_vis(interp, name, original, m, vis)
}

/// Default visibility for a method name in the current definition context.
fn default_vis(interp: &Interp, name: &[u8]) -> MethodVis {
    use tcl_registry::native_tcloo_method_definition::{
        NativeTclooMethodDefinitionProtocol, NativeTclooMethodVisibility,
    };
    let selected = NativeTclooMethodDefinitionProtocol::select(interp.native_invocation_dialect())
        .expect("registered native TclOO definition release");
    match selected.default_visibility(name, interp.oo.borrow().private_depth > 0) {
        NativeTclooMethodVisibility::Public => MethodVis::Public,
        NativeTclooMethodVisibility::Unexported => MethodVis::Unexported,
        NativeTclooMethodVisibility::Private => MethodVis::Private,
    }
}

pub(super) fn install_method_vis(
    interp: &mut Interp,
    name: Vec<u8>,
    m: Method,
    vis: MethodVis,
) -> Code {
    let original = obj::Owned::fresh(obj::new_string_bytes(&name));
    let code = install_original_method_vis(interp, name, original.as_ptr(), m, vis);
    drop(original);
    if let Err(first) = interp.check_entered_native_operation() {
        return interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(first));
    }
    code
}

fn install_original_method_vis(
    interp: &mut Interp,
    name: Vec<u8>,
    original: *mut TclObj,
    m: Method,
    vis: MethodVis,
) -> Code {
    let apply = |methods: &mut native_method_cache::MethodTable,
                 unexp: &mut BTreeSet<Vec<u8>>,
                 priv_set: &mut BTreeSet<Vec<u8>>| {
        let retired = methods.insert_original(name.clone(), original, m);
        methods
            .owner(&name)
            .expect("just installed Method record")
            .set_private(matches!(vis, MethodVis::Private));
        match vis {
            MethodVis::Public => {
                unexp.remove(&name);
                priv_set.remove(&name);
            }
            MethodVis::Unexported => {
                unexp.insert(name.clone());
                priv_set.remove(&name);
            }
            MethodVis::Private => {
                unexp.insert(name.clone());
                priv_set.insert(name.clone());
            }
        }
        retired
    };
    let target = match def_target(interp) {
        Ok(target) => target,
        Err(code) => return code,
    };
    let retired = match &target {
        DefTarget::Class(c) => {
            let mut oo = interp.oo.borrow_mut();
            oo.classes
                .get_mut(c)
                .map(|cl| apply(&mut cl.methods, &mut cl.unexported, &mut cl.private))
        }
        DefTarget::Object(o) => {
            let mut oo = interp.oo.borrow_mut();
            oo.objects
                .get_mut(o)
                .map(|ob| apply(&mut ob.methods, &mut ob.unexported, &mut ob.private))
        }
    };
    let Some(retired) = retired else {
        return interp.set_error(b"no current class/object to define on");
    };
    // A native clientData free can enter the interpreter. The new Method is
    // already installed, and neither the Method nor OO table is borrowed.
    drop(retired);
    if let Err(first) = interp.check_entered_native_operation() {
        return interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(first));
    }
    match target {
        DefTarget::Class(id) => {
            oo_checked!(interp, interp.native_property_method_created(id, true))
        }
        DefTarget::Object(id) => {
            oo_checked!(interp, interp.native_property_method_created(id, false))
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `private cmd ?arg ...?` / `private { script }` — a definition modifier that
/// marks the methods/variables it defines as private (`my`-only). TIP 500.
pub(super) fn def_private(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        // naming.tcloo.private-definition-mode
        // docs/design/analysis/name-resolution-proofs/private-definition-mode.md
        if interp.active_def_target().is_none() {
            return native_context::monkey_business(interp);
        }
        if argv.len() < 2 {
            let private = interp.oo.borrow().private_depth > 0;
            interp.set_result_bytes(if private { b"1" } else { b"0" });
            return Code::Ok;
        }
        interp.oo.borrow_mut().private_depth += 1;
        let code = if argv.len() == 2 {
            interp.eval_generic_control_body(argv[1])
        } else {
            interp.dispatch(&argv[1..])
        };
        interp.oo.borrow_mut().private_depth -= 1;
        oo_original_check!(interp, originals);
        code
    })
}

/// `deletemethod name ?name ...?` — remove method(s) from the current target.
pub(super) fn def_deletemethod(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 2 {
            return wrong_args(interp, b"deletemethod name ?name ...?");
        }
        let target = match def_target(interp) {
            Ok(t) => t,
            Err(c) => return c,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::for_definition(interp, &target)
        );
        for index in 1..argv.len() {
            let name = oo_original_bytes!(interp, originals, index);
            oo_checked!(interp, target_owner.check(interp));
            let retired = {
                let mut oo = interp.oo.borrow_mut();
                match &target {
                    DefTarget::Class(c) => oo.classes.get_mut(c).and_then(|cl| {
                        cl.unexported.remove(&name);
                        cl.methods.remove(&name)
                    }),
                    DefTarget::Object(o) => oo.objects.get_mut(o).and_then(|ob| {
                        ob.unexported.remove(&name);
                        ob.methods.remove(&name)
                    }),
                }
            };
            let removed = retired.is_some();
            drop(retired);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            if removed {
                match &target {
                    DefTarget::Class(id) => {
                        oo_checked!(interp, interp.native_property_structure_changed(*id, true))
                    }
                    DefTarget::Object(id) => {
                        oo_checked!(interp, interp.native_property_structure_changed(*id, false))
                    }
                }
            }
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            if !removed {
                let mut m = b"method ".to_vec();
                m.extend_from_slice(&name);
                m.extend_from_slice(b" does not exist");
                return err(interp, &m);
            }
        }
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

/// `renamemethod oldName newName` — rename the exact original table entry.
pub(super) fn def_renamemethod(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        use tcl_registry::native_tcloo_method_definition::{
            NativeTclooMethodDefinitionProtocol, NativeTclooMethodRenameFailure,
        };
        if argv.len() != 3 {
            return wrong_args(interp, b"renamemethod oldName newName");
        }
        let Some(selected) =
            NativeTclooMethodDefinitionProtocol::select(interp.native_invocation_dialect())
        else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native TclOO method rename",
                )
                .into(),
            );
        };
        let Some(policy) = interp.name_policy_protocol() else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native TclOO method name",
                )
                .into(),
            );
        };
        let target = match def_target(interp) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::for_definition(interp, &target)
        );
        let from = oo_original_bytes!(interp, originals, 1);
        oo_checked!(interp, target_owner.check(interp));
        let from = oo_checked!(
            interp,
            policy.recipe().oo_method_input(&from).map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native TclOO method name",
                )
            })
        )
        .selected()
        .to_vec();
        // Native source lookup precedes any destination String getter. This is
        // an observation of the exact entry, not an entered call-chain hold.
        let source = {
            let state = interp.oo.borrow();
            let methods = match &target {
                DefTarget::Class(id) => state.classes.get(id).map(|owner| &owner.methods),
                DefTarget::Object(id) => state.objects.get(id).map(|owner| &owner.methods),
            };
            methods.and_then(|methods| {
                methods
                    .get_key_value(&from)
                    .map(|(_, owner)| Rc::downgrade(owner))
            })
        };
        let Some(source) = source else {
            return method_rename_failure(
                interp,
                NativeTclooMethodRenameFailure::MissingSource,
                &from,
            );
        };
        let to = oo_original_bytes!(interp, originals, 2);
        oo_checked!(interp, target_owner.check(interp));
        let to = oo_checked!(
            interp,
            policy.recipe().oo_method_input(&to).map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native TclOO method name",
                )
            })
        )
        .selected()
        .to_vec();
        let source_observation = source;
        let Some(source) = source_observation.upgrade() else {
            return original_method_entry_unavailable(interp);
        };
        let verdict = {
            let state = interp.oo.borrow();
            let methods = match &target {
                DefTarget::Class(id) => state.classes.get(id).map(|owner| &owner.methods),
                DefTarget::Object(id) => state.objects.get(id).map(|owner| &owner.methods),
            };
            methods
                .filter(|methods| methods.is_original_entry(&from, &source))
                .map(|methods| {
                    let destination = methods.get_key_value(&to).map(|(_, owner)| owner);
                    selected.rename_destination(
                        destination.is_some_and(|owner| Rc::ptr_eq(owner, &source)),
                        destination.is_some(),
                    )
                })
        };
        let Some(verdict) = verdict else {
            return original_method_entry_unavailable(interp);
        };
        if let Err(failure) = verdict {
            drop(source);
            oo_original_check!(interp, originals);
            return method_rename_failure(interp, failure, &to);
        }
        let retired = {
            let mut state = interp.oo.borrow_mut();
            let fields = match &target {
                DefTarget::Class(id) => state.classes.get_mut(id).map(|owner| {
                    (
                        &mut owner.methods,
                        &mut owner.unexported,
                        &mut owner.exported,
                        &mut owner.private,
                    )
                }),
                DefTarget::Object(id) => state.objects.get_mut(id).map(|owner| {
                    (
                        &mut owner.methods,
                        &mut owner.unexported,
                        &mut owner.exported,
                        &mut owner.private,
                    )
                }),
            };
            fields.and_then(|(methods, unexported, exported, private)| {
                let retired = methods.rename_original(&from, to.clone(), &source, argv[2])?;
                // Rename keeps the original record's flags; case does not
                // recompute its declaration visibility.
                if unexported.remove(&from) {
                    unexported.insert(to.clone());
                }
                if exported.remove(&from) {
                    exported.insert(to.clone());
                }
                if private.remove(&from) {
                    private.insert(to.clone());
                }
                Some(retired)
            })
        };
        let Some(retired) = retired else {
            return original_method_entry_unavailable(interp);
        };
        // The old hash-key role can run a native free callback. The destination
        // already owns both real name roles, and the OO table is unborrowed.
        drop(source);
        drop(retired);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let still_original = {
            let state = interp.oo.borrow();
            source_observation
                .upgrade()
                .is_some_and(|source| match &target {
                    DefTarget::Class(id) => state
                        .classes
                        .get(id)
                        .is_some_and(|owner| owner.methods.is_original_entry(&to, &source)),
                    DefTarget::Object(id) => state
                        .objects
                        .get(id)
                        .is_some_and(|owner| owner.methods.is_original_entry(&to, &source)),
                })
        };
        oo_original_check!(interp, originals);
        if !still_original {
            return original_method_entry_unavailable(interp);
        }
        match target {
            DefTarget::Class(id) => {
                oo_checked!(interp, interp.native_property_structure_changed(id, true))
            }
            DefTarget::Object(id) => {
                oo_checked!(interp, interp.native_property_structure_changed(id, false))
            }
        }
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        // The selected stock worker preserves the result on successful rename.
        Code::Ok
    })
}

fn method_rename_failure(
    interp: &mut Interp,
    failure: tcl_registry::native_tcloo_method_definition::NativeTclooMethodRenameFailure,
    original_name: &[u8],
) -> Code {
    let message = failure.message(original_name);
    let code = crate::interp::error_code_list(&failure.error_code(original_name));
    interp.error_with_code(&message, &code)
}

fn original_method_entry_unavailable(interp: &mut Interp) -> Code {
    interp.report_cmd_error(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("original TclOO method entry")
            .into(),
    )
}

/// `class className` (objdefine only) — change the object's class.
pub(super) fn def_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 2 {
            return wrong_args(interp, b"class className");
        }
        let obj = match def_target(interp) {
            Ok(DefTarget::Object(o)) => o,
            Ok(DefTarget::Class(_)) => {
                return err(interp, b"attempt to misuse class as object");
            }
            Err(c) => return c,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::capture(interp, obj, false)
        );
        // The root classes' class may not be changed (each with its own message).
        if Some(obj) == interp.oo.borrow().object_root {
            return err(interp, b"may not modify the class of the root object class");
        }
        if Some(obj) == interp.oo.borrow().class_root {
            return err(interp, b"may not modify the class of the class of classes");
        }
        let new_cls = oo_original_object!(interp, originals, 1);
        oo_checked!(interp, target_owner.check(interp));
        if !interp.oo.borrow().objects.contains_key(&new_cls) {
            let name = oo_original_bytes!(interp, originals, 1);
            return not_object(interp, &name);
        }
        if !interp.oo.borrow().classes.contains_key(&new_cls) {
            let mut m = oo_original_bytes!(interp, originals, 1);
            m.extend_from_slice(b" does not refer to a class");
            return err(interp, &m);
        }
        // An object may not become an instance of itself.
        if new_cls == obj {
            return err(
                interp,
                b"may not change classes into an instance of themselves",
            );
        }
        // Changing the class can turn a plain object into a class (its new class is
        // `oo::class` or a metaclass) or back; keep the class-map membership in sync
        // so it gains/loses `create`/`new` and class introspection.
        let becomes_class = interp
            .mro(new_cls)
            .iter()
            .any(|c| Some(*c) == interp.oo.borrow().class_root);
        if let Some(ob) = interp.oo.borrow_mut().objects.get_mut(&obj) {
            ob.class = new_cls;
        }
        if becomes_class {
            if !interp.oo.borrow().classes.contains_key(&obj) {
                let supers = interp.oo.borrow().object_root.into_iter().collect();
                interp.oo.borrow_mut().classes.insert(
                    obj,
                    Class {
                        supers,
                        ..Class::default()
                    },
                );
            }
        } else if interp.oo.borrow().classes.contains_key(&obj) {
            // A class demoted to a non-class can no longer support its subclasses
            // or instances, so they are torn down (C's `TclOODeleteDescendants`
            // when the class facet goes away; oo-13.6). The object itself survives
            // as a plain object — only its class facet is removed.
            interp.oo_destroy_class_descendants(obj);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            let retired = interp.oo.borrow_mut().classes.remove(&obj);
            drop(retired);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
        }
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        oo_checked!(interp, interp.native_property_structure_changed(obj, false));
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

/// `definitionnamespace ?-class|-instance? namespace` (TIP 524) — set the
/// namespace in which this class (or its instances) are defined. An empty
/// namespace name resets to the default.
pub(super) fn def_definitionnamespace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        let cls = match def_target(interp) {
            Ok(DefTarget::Class(c)) => c,
            Ok(DefTarget::Object(_)) => {
                return err(
                    interp,
                    b"this command may only be called in a class definition",
                );
            }
            Err(c) => return c,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::capture(interp, cls, true)
        );
        if Some(cls) == interp.oo.borrow().object_root || Some(cls) == interp.oo.borrow().class_root
        {
            return err(
                interp,
                b"may not modify the definition namespace of the root classes",
            );
        }
        if argv.len() != 2 && argv.len() != 3 {
            return wrong_args(interp, b"definitionnamespace ?kind? namespace");
        }
        // Default kind is `-class`.
        let instance = if argv.len() == 3 {
            let selected =
                interp.native_static_option_index(argv[1], DEFINITION_KINDS.names(), false, "kind");
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            match selected {
                Ok(index) => index == 1,
                Err(m) => return interp.report_cmd_error(m),
            }
        } else {
            false
        };
        let ns_arg = oo_original_bytes!(interp, originals, argv.len() - 1);
        oo_checked!(interp, target_owner.check(interp));
        let stored = if ns_arg.is_empty() {
            None
        } else {
            let original = oo_checked!(interp, originals.original(argv.len() - 1));
            let selected = oo_checked!(interp, interp.oo_outer_namespace_name(original, originals));
            oo_checked!(interp, target_owner.check(interp));
            match selected {
                Some(qn) => Some(qn),
                None => {
                    let mut m = b"namespace \"".to_vec();
                    m.extend_from_slice(&ns_arg);
                    m.extend_from_slice(b"\" not found");
                    return err(interp, &m);
                }
            }
        };
        if let Some(c) = interp.oo.borrow_mut().classes.get_mut(&cls) {
            if instance {
                c.def_ns = stored;
            } else {
                c.class_def_ns = stored;
            }
        }
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

/// `oo::define … definitionnamespace`'s kind word (`tclOODefineCmds.c`),
/// resolved with `Tcl_GetIndexFromObj(…, "kind", 0)`: `-c`/`-i` abbreviate and
/// the empty word — a prefix of both — is `ambiguous kind ""`.
pub(super) const DEFINITION_KINDS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("kind", &[b"-class", b"-instance"]);

pub(super) fn def_method(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        use tcl_registry::native_tcloo_method_definition::{
            NativeTclooMethodDefinitionProtocol, NativeTclooMethodVisibility,
        };
        let Some(selected) =
            NativeTclooMethodDefinitionProtocol::select(interp.native_invocation_dialect())
        else {
            return interp.set_error(b"native method declaration policy is unavailable");
        };
        let Some(layout) = selected.layout(argv.len().saturating_sub(1)) else {
            return wrong_args(interp, selected.usage().as_bytes());
        };
        let target = match def_target(interp) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::for_definition(interp, &target)
        );
        let name = oo_original_bytes!(interp, originals, layout.name + 1);
        oo_checked!(interp, target_owner.check(interp));
        let flag_vis = if let Some(index) = layout.option {
            let index = interp.native_static_string_option_index(
                argv[index + 1],
                selected.export_modes().unwrap(),
                false,
                "export flag",
            );
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            let index = match index {
                Ok(index) => index,
                Err(error) => return interp.report_cmd_error(error),
            };
            selected
                .option_visibility(index)
                .map(|visibility| match visibility {
                    NativeTclooMethodVisibility::Public => MethodVis::Public,
                    NativeTclooMethodVisibility::Unexported => MethodVis::Unexported,
                    NativeTclooMethodVisibility::Private => MethodVis::Private,
                })
        } else {
            None
        };
        let chosen = interp.choose_original_procedure_body(argv[layout.body + 1]);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let chosen = match chosen {
            Ok(chosen) => chosen,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let params = crate::cmd_proc::parse_params_object(interp, argv[layout.parameters + 1], b"");
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let params = match params {
            Ok(p) => p,
            Err(error) => return interp.report_cmd_error(error),
        };
        let vis = flag_vis.unwrap_or_else(|| default_vis(interp, &name));
        // Source provenance for `info frame` (TIP 280): the body word is the last
        // argument; its file-absolute line (minus one) is the body's line base.
        let src = method_body_src(interp, argv.len() - 1);
        let method = native_method_body(interp, params, chosen, src);
        install_original_method_vis(interp, name, argv[layout.name + 1], method, vis)
    })
}

/// The source provenance `(file, body_line_base)` for a method/constructor body
/// at argument index `body_idx`, when defined while sourcing a file; `None`
/// otherwise (an eval-defined body keeps body-relative lines).
fn method_body_src(interp: &Interp, body_idx: usize) -> Option<(Rc<[u8]>, u32)> {
    interp
        .current_source_file()
        .map(|f| (f, interp.arg_line(body_idx).saturating_sub(1)))
}

/// `forward name cmdPrefix ?arg ...?` — a method that calls a command prefix.
pub(super) fn def_forward(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 3 {
            return wrong_args(interp, b"forward name cmdName ?arg ...?");
        }
        let target = match def_target(interp) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::for_definition(interp, &target)
        );
        let name = oo_original_bytes!(interp, originals, 1);
        oo_checked!(interp, target_owner.check(interp));
        let prefix = obj::Owned::fresh(interp.new_list_object(&argv[2..]));
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        install_method(interp, name, argv[1], Method::Forward { prefix })
    })
}

pub(super) fn def_constructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 3 {
            return wrong_args(interp, b"constructor arguments body");
        }
        let class = match def_target(interp) {
            Ok(DefTarget::Class(c)) => c,
            Ok(DefTarget::Object(_)) => return err(interp, b"constructors are only for classes"),
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::capture(interp, class, true)
        );
        if oo_original_bytes!(interp, originals, 2).is_empty() {
            oo_checked!(interp, target_owner.check(interp));
            let retired = interp
                .oo
                .borrow_mut()
                .classes
                .get_mut(&class)
                .unwrap()
                .constructor
                .take();
            if retired.is_some() {
                oo_checked!(
                    interp,
                    interp.native_property_structure_changed(class, true)
                );
            }
            drop(retired);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        let chosen = interp.choose_original_procedure_body(argv[2]);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let chosen = match chosen {
            Ok(chosen) => chosen,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let params = crate::cmd_proc::parse_params_object(interp, argv[1], b"");
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let params = match params {
            Ok(p) => p,
            Err(error) => return interp.report_cmd_error(error),
        };
        let src = method_body_src(interp, 2);
        let method = native_method_body(interp, params, chosen, src);
        let retired = interp
            .oo
            .borrow_mut()
            .classes
            .get_mut(&class)
            .unwrap()
            .constructor
            .install(method);
        drop(retired);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        oo_checked!(interp, interp.native_property_method_created(class, true));
        oo_checked!(
            interp,
            interp.native_property_structure_changed(class, true)
        );
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

pub(super) fn def_destructor(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 2 {
            return wrong_args(interp, b"destructor body");
        }
        let class = match def_target(interp) {
            Ok(DefTarget::Class(c)) => c,
            Ok(DefTarget::Object(_)) => return err(interp, b"destructors are only for classes"),
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::capture(interp, class, true)
        );
        if oo_original_bytes!(interp, originals, 1).is_empty() {
            oo_checked!(interp, target_owner.check(interp));
            let retired = interp
                .oo
                .borrow_mut()
                .classes
                .get_mut(&class)
                .unwrap()
                .destructor
                .take();
            if retired.is_some() {
                oo_checked!(
                    interp,
                    interp.native_property_structure_changed(class, true)
                );
            }
            drop(retired);
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        let src = method_body_src(interp, 1);
        let chosen = interp.choose_original_procedure_body(argv[1]);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        let chosen = match chosen {
            Ok(chosen) => chosen,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let method = native_method_body(interp, Vec::new(), chosen, src);
        let retired = interp
            .oo
            .borrow_mut()
            .classes
            .get_mut(&class)
            .unwrap()
            .destructor
            .install(method);
        drop(retired);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        oo_checked!(interp, interp.native_property_method_created(class, true));
        oo_checked!(
            interp,
            interp.native_property_structure_changed(class, true)
        );
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

/// `export`/`unexport name ...` — set method visibility on the current target.
/// Tracks both the `unexported` set (a method hidden from public dispatch) and
/// the `exported` set (a default-unexported built-in promoted to public).
/// Apply a reached flag change without holding an OO borrow across a getter.
fn apply_method_visibility(
    interp: &mut Interp,
    target: &DefTarget,
    name: &[u8],
    export: bool,
) -> bool {
    let apply = |unexp: &mut BTreeSet<Vec<u8>>,
                 exp: &mut BTreeSet<Vec<u8>>,
                 priv_set: &mut BTreeSet<Vec<u8>>| {
        let mut changed = priv_set.remove(name);
        if export {
            changed |= unexp.remove(name);
            changed |= exp.insert(name.to_vec());
        } else {
            changed |= exp.remove(name);
            changed |= unexp.insert(name.to_vec());
        }
        changed
    };
    let mut oo = interp.oo.borrow_mut();
    match target {
        DefTarget::Class(class) => oo.classes.get_mut(class).is_some_and(|owner| {
            if let Some(record) = owner.methods.owner(name) {
                record.set_private(false);
            }
            apply(
                &mut owner.unexported,
                &mut owner.exported,
                &mut owner.private,
            )
        }),
        DefTarget::Object(object) => {
            oo.native_methods.instance_table_created(*object);
            oo.objects.get_mut(object).is_some_and(|owner| {
                owner.methods.mark_allocated();
                if let Some(record) = owner.methods.owner(name) {
                    record.set_private(false);
                }
                apply(
                    &mut owner.unexported,
                    &mut owner.exported,
                    &mut owner.private,
                )
            })
        }
    }
}

pub(super) fn def_export(interp: &mut Interp, argv: &[*mut TclObj], export: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        use tcl_registry::native_tcloo_method_definition::{
            NativeTclooMethodDefinitionProtocol, NativeTclooMethodExportOperandOrder,
        };
        let target = match def_target(interp) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::for_definition(interp, &target)
        );
        let selected = oo_checked!(
            interp,
            NativeTclooMethodDefinitionProtocol::select(interp.native_invocation_dialect()).ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "TclOO export operand order",
                )
            )
        );
        let mut changed = false;
        match selected.export_operand_order() {
            NativeTclooMethodExportOperandOrder::OperandThenEffect => {
                for index in 1..argv.len() {
                    let name = oo_original_bytes!(interp, originals, index);
                    oo_checked!(interp, target_owner.check(interp));
                    changed |= apply_method_visibility(interp, &target, &name, export);
                    oo_original_check!(interp, originals);
                    oo_checked!(interp, target_owner.check(interp));
                }
            }
            NativeTclooMethodExportOperandOrder::CompatibilityReadAll => {
                let mut names = Vec::with_capacity(argv.len().saturating_sub(1));
                for index in 1..argv.len() {
                    names.push(oo_original_bytes!(interp, originals, index));
                    oo_checked!(interp, target_owner.check(interp));
                }
                for name in names {
                    changed |= apply_method_visibility(interp, &target, &name, export);
                }
            }
        }
        if changed {
            match target {
                DefTarget::Class(id) => {
                    oo_checked!(interp, interp.native_property_structure_changed(id, true))
                }
                DefTarget::Object(id) => {
                    oo_checked!(interp, interp.native_property_structure_changed(id, false))
                }
            }
        }
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        interp.set_result_bytes(b"");
        Code::Ok
    })
}

impl Interp {
    /// Run an `oo::define`/`oo::objdefine` body or single subcommand on `target`.
    fn oo_run_def(&mut self, target: DefTarget, argv: &[*mut TclObj]) -> Code {
        native_context::with_original_arguments(self, argv, |interp, originals| {
            if argv.len() == 3 {
                let body = oo_original_bytes!(interp, originals, 2);
                // The body word is argv[2]; capture its source provenance (TIP 280)
                // now, while the argument lines are still those of this command.
                let body_src = method_body_src(interp, 2);
                return interp.oo_define_body_with_argv(target, &body, body_src, argv);
            }
            // Single-command form: a subcommand's `wrong # args` names the whole
            // original command (`oo::define Foo method …`), via the rewrite prefix
            // `<oo::define|oo::objdefine> <as-written target>`.
            let mut prefix = oo_original_bytes!(interp, originals, 0);
            prefix.push(b' ');
            prefix.extend_from_slice(&oo_original_bytes!(interp, originals, 1));
            let saved = interp.oo.borrow_mut().def_rewrite.replace(prefix);
            let caller = match interp.oo_enter_definition(&target, argv) {
                Ok(caller) => caller,
                Err(code) => {
                    interp.oo.borrow_mut().def_rewrite = saved;
                    return code;
                }
            };
            // Re-base the TIP 280 argument lines onto the dispatched subcommand
            // (drop the `oo::define <target>` prefix) so a defined body word's line
            // is found at the subcommand's own index, then restore.
            let saved_lines = interp.arg_lines_snapshot();
            if saved_lines.len() >= 2 {
                interp.set_arg_lines(saved_lines[2..].to_vec());
            }
            let code = interp.oo_magic_definition_invoke(argv[2], &argv[3..]);
            interp.set_arg_lines(saved_lines);
            interp.oo_leave_definition(caller);
            interp.oo.borrow_mut().def_rewrite = saved;
            code
        })
    }

    /// Evaluate a definition `body` in its selected definition namespace frame.
    /// Class-name arguments retain the original caller's namespace. Shared by
    /// `oo_run_def` and the `oo::class` constructor (a metaclass instance body).
    pub(super) fn oo_define_body(
        &mut self,
        target: DefTarget,
        original_body: *mut TclObj,
        body_src: Option<(Rc<[u8]>, u32)>,
    ) -> Code {
        let object = match target {
            DefTarget::Class(object) | DefTarget::Object(object) => object,
        };
        let define = self
            .oo
            .borrow()
            .define_name
            .as_ref()
            .expect("native OO Foundation define head")
            .clone();
        let name = obj::Owned::retain(self.oo_original_name(object));
        let arguments = [define.as_ptr(), name.as_ptr(), original_body];
        native_context::with_original_arguments(self, &arguments, |interp, originals| {
            let body = oo_original_bytes!(interp, originals, 2);
            interp.oo_define_body_with_argv(target, &body, body_src, &arguments)
        })
    }

    fn oo_define_body_with_argv(
        &mut self,
        target: DefTarget,
        body: &[u8],
        body_src: Option<(Rc<[u8]>, u32)>,
        original: &[*mut TclObj],
    ) -> Code {
        let (kind, object): (&[u8], OoId) = match &target {
            DefTarget::Class(c) => (b"class", *c),
            DefTarget::Object(o) => (b"object", *o),
        };
        let fqn = self.oo_name(object);
        // The errorInfo frame names the object by its *current* command name, so
        // a `rename` inside the body is reflected (oo-18.6/18.7). The creation id
        // is stable across rename, so we re-find the entry by it at error time.
        let creation_id = self.oo.borrow().objects.get(&object).map(|o| o.creation_id);
        let caller = match self.oo_enter_definition(&target, original) {
            Ok(caller) => caller,
            Err(code) => return code,
        };
        // When sourced, run the body in a `type source` frame so the methods it
        // defines record file-absolute body lines (TIP 280).
        let code = self.eval_def_body(body, body_src);
        self.oo_leave_definition(caller);
        // On error, add the `(in definition script for class/object "X" line N)`
        // errorInfo frame (C's GenerateErrorInfo).
        if code == Code::Error {
            self.add_def_script_frame(kind, &fqn, creation_id);
        }
        code
    }

    /// Append the `(in definition script for <kind> "<name>" line N)` errorInfo
    /// frame (C's `GenerateErrorInfo`). The object is named by its *current*
    /// command (re-found by `creation_id` so a `rename` in the body shows), the
    /// name quoted and truncated to 30 bytes (`OBJNAME_LENGTH_IN_ERRORINFO_LIMIT`).
    pub(super) fn add_def_script_frame(
        &mut self,
        kind: &[u8],
        fqn: &[u8],
        creation_id: Option<u64>,
    ) {
        let current = creation_id
            .and_then(|id| {
                self.oo
                    .borrow()
                    .objects
                    .iter()
                    .find(|(_, o)| o.creation_id == id)
                    .map(|(k, _)| self.oo_name(*k))
            })
            .unwrap_or_else(|| fqn.to_vec());
        let mut inner = b"in definition script for ".to_vec();
        inner.extend_from_slice(kind);
        inner.extend_from_slice(b" \"");
        if current.len() > 30 {
            inner.extend_from_slice(&current[..30]);
            inner.extend_from_slice(b"...");
        } else {
            inner.extend_from_slice(&current);
        }
        inner.push(b'"');
        self.append_frame_line(&inner);
        // A frame boundary: let the enclosing command log its own `invoked from
        // within` frame.
        self.clear_error_logged();
    }

    /// The custom definition-resolution namespace for a definition `target`, or
    /// `None` for the built-in default (`::oo::define`/`::oo::objdefine`).
    /// For a class, it is the
    /// metaclass's `-class` namespace; for an object, its class's `-instance`.
    pub(super) fn definition_namespace_for(&self, target: &DefTarget) -> Option<Vec<u8>> {
        let oo = self.oo.borrow();
        let ns = match target {
            DefTarget::Class(c) => {
                let meta = oo.objects.get(c).map(|o| o.class)?;
                oo.classes.get(&meta)?.class_def_ns.clone()
            }
            DefTarget::Object(o) => {
                let cls = oo.objects.get(o).map(|ob| ob.class)?;
                oo.classes.get(&cls)?.def_ns.clone()
            }
        }?;
        // The caller selects the actual built-in support namespace token.
        if ns == b"::oo::define" || ns == b"::oo::objdefine" {
            None
        } else {
            Some(ns)
        }
    }
}
