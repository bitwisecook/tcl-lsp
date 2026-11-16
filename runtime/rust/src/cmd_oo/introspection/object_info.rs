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

//! `info object subcommand object ?arg?` and its option tables.

use super::super::configurable::parse_property_opts;
use super::super::slots::set_declared_variable_list;
use super::super::{err, native_context, native_object_info, variable_mapping, wrong_args};
use super::call_chain_reports::info_call;
use super::{
    fqn_glob_ok, info_definition, info_forward, info_methodtype, not_object, set_list, set_oo_list,
};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_registry::commands::tcl::{info_oo_subcommands, InfoOoEnsembleKind};
use tcl_syntax::value::ValueOps;

// info object / info class (called from cmd_info)

pub(super) fn variable_info_options(
    interp: &mut Interp,
    kind: InfoOoEnsembleKind,
    rest: &[*mut TclObj],
) -> Result<bool, Code> {
    let selected = tcl_registry::native_tcloo_info::NativeTclooVariableInfoProtocol::select(
        interp.native_invocation_dialect(),
    )
    .ok_or_else(|| {
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("TclOO variable info").into(),
        )
    })?;
    if !selected.accepts_count(rest.len()) {
        return Err(wrong_args(interp, selected.usage(kind).as_bytes()));
    }
    let Some(option) = rest.get(1) else {
        return Ok(false);
    };
    let original = interp
        .native_string_bytes(option)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    selected
        .private_option(&original)
        .map_err(|error| interp.report_cmd_error(error))
}

pub(super) fn method_info_options(
    interp: &mut Interp,
    extra: &[*mut TclObj],
) -> Result<tcl_registry::native_tcloo_info::NativeTclooMethodInfoSelection, Code> {
    let Some(selected) = tcl_registry::native_tcloo_info::NativeTclooMethodInfoProtocol::select(
        interp.native_invocation_dialect(),
    ) else {
        return Err(interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("TclOO method info options")
                .into(),
        ));
    };
    selected
        .parse_original(interp, extra)
        .map_err(|error| interp.report_cmd_error(error))
}

/// `info object subcommand object ?arg?`.
pub(crate) fn info_object(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 3 {
            return wrong_args(interp, b"info object subcommand ?arg ...?");
        }
        let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Object, interp.runtime_version());
        let sub = match subcommands.resolve(&oo_original_bytes!(interp, originals, 2)) {
            Ok(sub) => sub.as_bytes(),
            Err(message) => return interp.set_error(&message),
        };
        // `creationid` has its own arg-count message (it errors at 0 *or* 2+ names).
        if sub == b"creationid" && argv.len() != 4 {
            return wrong_args(interp, b"info object creationid objName");
        }
        // `call` takes exactly objName + methodName.
        if sub == b"call" && argv.len() != 5 {
            return wrong_args(interp, b"info object call objName methodName");
        }
        // `isa` reads `category objName ?className?`, and C checks that both of
        // the first two are present *before* it resolves the category
        // (`InfoObjectIsACmd`, tclOOInfo.c) — so `info object isa object` is this
        // message, not a category error. The per-category counts follow in the
        // arm, once the word is known.
        if sub == b"isa" && argv.len() < 5 {
            return wrong_args(interp, b"info object isa category objName ?arg ...?");
        }
        let private_variables = if sub == b"variables" {
            match variable_info_options(interp, InfoOoEnsembleKind::Object, &argv[3..]) {
                Ok(private) => private,
                Err(code) => return code,
            }
        } else {
            false
        };
        if argv.len() < 4 {
            if sub == b"methods" {
                return wrong_args(
                    interp,
                    tcl_registry::native_tcloo_info::NativeTclooMethodInfoProtocol::usage(
                        InfoOoEnsembleKind::Object,
                    )
                    .as_bytes(),
                );
            }
            return wrong_args(interp, b"info object subcommand objName ?arg ...?");
        }
        if let Some(code) = native_object_info::selected_object_info_handler(interp, sub, argv) {
            return code;
        }
        let obj = oo_original_object!(interp, originals, 3);
        match sub {
            b"class" => {
                let class = interp.oo.borrow().objects.get(&obj).map(|o| o.class);
                let Some(class) = class else {
                    return not_object(interp, &oo_original_bytes!(interp, originals, 3));
                };
                // Two-arg form `info object class obj className`: a membership test —
                // whether `className` is in the object's precedence (its class MRO +
                // mixins), like `info object isa typeof` (C's `InfoObjectClassCmd`).
                if argv.get(4).is_some() {
                    let target_owner = oo_checked!(
                        interp,
                        native_context::OriginalOoTarget::capture(interp, obj, false)
                    );
                    let want = oo_original_object!(interp, originals, 4);
                    oo_checked!(interp, target_owner.check(interp));
                    let yes = interp.method_chain(obj).iter().skip(1).any(|p| *p == want);
                    interp.set_result_bytes(if yes { b"1" } else { b"0" });
                    return Code::Ok;
                }
                interp.set_result(obj::new_string_bytes(&interp.oo_name(class)));
                Code::Ok
            }
            b"isa" => {
                // `info object isa category objName ?arg?`. C resolves the category
                // with `Tcl_GetIndexFromObj(…, "category", 0)` (`categories[]`,
                // `tclOOInfo.c`), so `cl`/`ob`/`t` abbreviate, `m` is ambiguous
                // (metaclass/mixin), and an unknown word is an error.
                const ISA_CATEGORIES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
                    tcl_cmd_core::prefix::OptionTable::abbreviating(
                        "category",
                        &[b"class", b"metaclass", b"mixin", b"object", b"typeof"],
                    );
                let cat = match interp.native_static_option_index(
                    argv[3],
                    ISA_CATEGORIES.names(),
                    false,
                    "category",
                ) {
                    Ok(index) => ISA_CATEGORIES.names()[index],
                    Err(m) => return interp.report_cmd_error(m),
                };
                // Each category then pins an *exact* count — C's second stage,
                // `Tcl_WrongNumArgs(interp, 2, objv, …)`, so the noun carries the
                // resolved category word (an index-typed argument prints as its
                // table entry): `info object isa t o` is
                // `"info object isa typeof objName className"`, and a trailing
                // extra word is an error rather than being ignored.
                let (want, tail) = match cat {
                    b"mixin" | b"typeof" => (6, b" objName className".as_slice()),
                    // class / metaclass / object.
                    _ => (5, b" objName".as_slice()),
                };
                if argv.len() != want {
                    let mut m = b"info object isa ".to_vec();
                    m.extend_from_slice(cat);
                    m.extend_from_slice(tail);
                    return wrong_args(interp, &m);
                }
                // Resolve the object name following import aliases to the origin
                // command, so an imported object command is recognised (oo-1.10).
                let target = oo_original_object!(interp, originals, 4);
                let target_owner = if interp.oo.borrow().objects.contains_key(&target) {
                    Some(oo_checked!(
                        interp,
                        native_context::OriginalOoTarget::capture(interp, target, false)
                    ))
                } else {
                    None
                };
                // An initially missing receiver is the selected false precondition;
                // it does not reach a later relation operand.
                let Some(target_owner) = target_owner else {
                    interp.set_result_bytes(b"0");
                    return Code::Ok;
                };
                let yes = match cat {
                    b"object" => interp.oo.borrow().objects.contains_key(&target),
                    b"class" => interp.oo.borrow().classes.contains_key(&target),
                    // A metaclass is a class whose own hierarchy includes
                    // `::oo::class` (so its instances are themselves classes).
                    b"metaclass" => {
                        let is_class = interp.oo.borrow().classes.contains_key(&target);
                        is_class
                            && interp
                                .mro(target)
                                .iter()
                                .any(|c| Some(*c) == interp.oo.borrow().class_root)
                    }
                    // `mixin`: `mixinClass` is mixed into the object directly or via
                    // its class.
                    b"mixin" => {
                        let mixin = oo_original_object!(interp, originals, 5);
                        oo_checked!(interp, target_owner.check(interp));
                        let oo = interp.oo.borrow();
                        oo.objects.get(&target).is_some_and(|o| {
                            o.mixins.contains(&mixin)
                                || oo
                                    .classes
                                    .get(&o.class)
                                    .is_some_and(|c| c.mixins.contains(&mixin))
                        })
                    }
                    b"typeof" => {
                        // The object's full precedence list (mixins included), not
                        // just its class's superclass MRO (oo-16.9). The chain leads
                        // with the object itself, so drop it before matching classes.
                        let want = oo_original_object!(interp, originals, 5);
                        oo_checked!(interp, target_owner.check(interp));
                        interp.oo.borrow().objects.contains_key(&target)
                            && interp
                                .method_chain(target)
                                .iter()
                                .skip(1)
                                .any(|p| *p == want)
                    }
                    // Unreachable: `ISA_CATEGORIES` has exactly these five names.
                    _ => false,
                };
                interp.set_result_bytes(if yes { b"1" } else { b"0" });
                Code::Ok
            }
            // `vars` lists *set* instance variables (glob-filtered); `variables`
            // lists the object's *declared* variables (the `variable` slot).
            b"vars" => {
                let ns = interp.oo.borrow().objects.get(&obj).map(|o| o.var_ns);
                let Some(ns) = ns else {
                    return not_object(interp, &oo_original_bytes!(interp, originals, 3));
                };
                let target_owner = oo_checked!(
                    interp,
                    native_context::OriginalOoTarget::capture(interp, obj, false)
                );
                let namespace_owner = oo_checked!(
                    interp,
                    interp.native_namespace_token_at(ns).ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original OO report namespace",
                        )
                    )
                );
                // `info object vars obj ?pattern?` — the optional glob filters the
                // (simple) variable names (oo-16.10).
                let pat = if argv.len() > 4 {
                    Some(oo_original_bytes!(interp, originals, 4))
                } else {
                    None
                };
                oo_checked!(
                    interp,
                    target_owner.check_instance_namespace(interp, ns, &namespace_owner)
                );
                let mut names: Vec<Vec<u8>> = interp
                    .namespaces()
                    .var_names(ns)
                    .into_iter()
                    .filter(|n| fqn_glob_ok(pat.as_deref(), n))
                    .collect();
                names.sort();
                set_list(interp, &names);
                Code::Ok
            }
            b"variables" => {
                let v = interp.oo.borrow().objects.get(&obj).map(|o| {
                    if private_variables {
                        o.private_variables
                            .iter()
                            .map(variable_mapping::observation)
                            .collect::<Vec<_>>()
                    } else {
                        o.variables
                            .iter()
                            .map(variable_mapping::observation)
                            .collect::<Vec<_>>()
                    }
                });
                match v {
                    Some(v) => {
                        oo_checked!(interp, set_declared_variable_list(interp, &v));
                        Code::Ok
                    }
                    None => not_object(interp, &oo_original_bytes!(interp, originals, 3)),
                }
            }
            b"properties" => {
                // `info object properties obj ?-all? ?-readable|-writable?` (TIP 558).
                if !interp.oo.borrow().objects.contains_key(&obj) {
                    return not_object(interp, &oo_original_bytes!(interp, originals, 3));
                }
                let target_owner = oo_checked!(
                    interp,
                    native_context::OriginalOoTarget::capture(interp, obj, false)
                );
                let reached = parse_property_opts(interp, &argv[4..]);
                oo_original_check!(interp, originals);
                oo_checked!(interp, target_owner.check(interp));
                let (all, writable) = match reached {
                    Ok(x) => x,
                    Err(code) => return code,
                };
                if all
                    && interp
                        .native_invocation_dialect()
                        .native_property_lookup_protocol()
                        .is_some()
                {
                    return match interp.native_all_property_header(obj, false, writable) {
                        Ok(header) => {
                            interp.set_result(header.header);
                            match header.check(interp) {
                                Ok(()) => Code::Ok,
                                Err(error) => interp.report_cmd_error(error),
                            }
                        }
                        Err(error) => interp.report_cmd_error(error),
                    };
                }
                if interp
                    .native_invocation_dialect()
                    .native_property_lookup_protocol()
                    .is_some()
                {
                    return match interp.native_declared_property_header(obj, false, writable) {
                        Ok(header) => {
                            interp.set_result(header.header);
                            match header.check(interp) {
                                Ok(()) => Code::Ok,
                                Err(error) => interp.report_cmd_error(error),
                            }
                        }
                        Err(error) => interp.report_cmd_error(error),
                    };
                }
                let props = interp.object_property_list(obj, writable, all);
                set_list(interp, &props);
                Code::Ok
            }
            b"methods" => {
                if !interp.oo.borrow().objects.contains_key(&obj) {
                    return not_object(interp, &oo_original_bytes!(interp, originals, 3));
                }
                let target_owner = oo_checked!(
                    interp,
                    native_context::OriginalOoTarget::capture(interp, obj, false)
                );
                let reached = method_info_options(interp, &argv[4..]);
                oo_original_check!(interp, originals);
                oo_checked!(interp, target_owner.check(interp));
                let selected = match reached {
                    Ok(selected) => selected,
                    Err(code) => return code,
                };
                let mut names: Vec<Vec<u8>> = Vec::new();
                // The object's own methods, plus (with `-all`, no `-scope`) the chain.
                let chain = if selected.recurse() {
                    interp.method_chain(obj)
                } else {
                    vec![obj]
                };
                for p in &chain {
                    let is_object = *p == obj;
                    let oo = interp.oo.borrow();
                    let entry = if is_object {
                        oo.objects
                            .get(p)
                            .map(|o| (&o.methods, &o.unexported, &o.private))
                    } else {
                        oo.classes
                            .get(p)
                            .map(|c| (&c.methods, &c.unexported, &c.private))
                    };
                    if let Some((methods, unexp, priv_set)) = entry {
                        for n in methods.keys() {
                            let public = !unexp.contains(n);
                            let show = if selected.recurse() {
                                !priv_set.contains(n) && (selected.includes_unexported() || public)
                            } else {
                                selected.matches_local(public, false, priv_set.contains(n))
                            };
                            if show && !names.contains(n) {
                                names.push(n.clone());
                            }
                        }
                    }
                }
                // `-all` also surfaces the inherited `oo::object` built-in methods,
                // honouring any `export`/`unexport` applied to them (but `-scope`
                // restricts to the object's own methods).
                if selected.recurse() {
                    for b in [
                        b"<cloned>".as_slice(),
                        b"destroy",
                        b"eval",
                        b"unknown",
                        b"variable",
                        b"varname",
                    ] {
                        let (exp, unexp, priv_) = interp.method_visibility_flags(obj, b);
                        // `destroy` is exported by default; the rest are unexported.
                        let eff_unexp = !exp && (unexp || b != b"destroy");
                        let show = !priv_ && (selected.includes_unexported() || !eff_unexp);
                        if show && !names.iter().any(|n| n == b) {
                            names.push(b.to_vec());
                        }
                    }
                }
                names.sort();
                set_list(interp, &names);
                Code::Ok
            }
            b"namespace" => {
                let ns = interp.oo.borrow().objects.get(&obj).map(|o| o.var_ns);
                let Some(ns) = ns else {
                    return not_object(interp, &oo_original_bytes!(interp, originals, 3));
                };
                let name = interp.namespaces().qualified_name(ns);
                interp.set_result(obj::new_string_bytes(&name));
                Code::Ok
            }
            b"mixins" => {
                let mx = interp
                    .oo
                    .borrow()
                    .objects
                    .get(&obj)
                    .map(|o| o.mixins.clone());
                match mx {
                    Some(m) => {
                        set_oo_list(interp, &m);
                        Code::Ok
                    }
                    None => not_object(interp, &oo_original_bytes!(interp, originals, 3)),
                }
            }
            b"creationid" => {
                // Exactly one object name; the prior `argv.len() < 4` gate already
                // rejected the no-name case, but this also rejects extra args.
                if argv.len() != 4 {
                    return wrong_args(interp, b"info object creationid objName");
                }
                let id = interp.oo.borrow().objects.get(&obj).map(|o| o.creation_id);
                match id {
                    Some(id) => {
                        interp.set_result_bytes(id.to_string().as_bytes());
                        Code::Ok
                    }
                    None => not_object(interp, &oo_original_bytes!(interp, originals, 3)),
                }
            }
            b"filters" => {
                let f = interp
                    .oo
                    .borrow()
                    .objects
                    .get(&obj)
                    .map(|o| o.filters.clone());
                match f {
                    Some(f) => {
                        set_list(interp, &f);
                        Code::Ok
                    }
                    None => not_object(interp, &oo_original_bytes!(interp, originals, 3)),
                }
            }
            b"forward" => info_forward(interp, obj, argv, false),
            b"definition" => info_definition(interp, obj, argv, false),
            b"methodtype" => info_methodtype(interp, obj, argv, false),
            b"call" => info_call(interp, obj, argv, false),
            // `properties` is not yet modelled.
            other => {
                let mut m = b"unsupported info object subcommand \"".to_vec();
                m.extend_from_slice(other);
                m.push(b'"');
                err(interp, &m)
            }
        }
    })
}

/// Validate the already selected receiver before retaining its continuation.
pub(super) fn original_info_target(
    interp: &mut Interp,
    originals: &crate::interp::native_arguments::OriginalArguments,
    id: OoId,
    class: bool,
) -> Result<native_context::OriginalOoTarget, Code> {
    if !interp.oo.borrow().objects.contains_key(&id) {
        let name = originals
            .bytes(3)
            .map_err(|error| interp.report_cmd_error(error))?;
        return Err(not_object(interp, &name));
    }
    if class && !interp.oo.borrow().classes.contains_key(&id) {
        let name = originals
            .bytes(3)
            .map_err(|error| interp.report_cmd_error(error))?;
        let mut message = b"\"".to_vec();
        message.extend_from_slice(&name);
        message.extend_from_slice(b"\" is not a class");
        return Err(err(interp, &message));
    }
    native_context::OriginalOoTarget::capture(interp, id, class)
        .map_err(|error| interp.report_cmd_error(error))
}
