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

//! `info class subcommand class ?arg?`.

use super::super::configurable::parse_property_opts;
use super::super::definition::DEFINITION_KINDS;
use super::super::slots::set_declared_variable_list;
use super::super::{err, native_context, variable_mapping, wrong_args, Method};
use super::call_chain_reports::info_call;
use super::object_info::{method_info_options, variable_info_options};
use super::{
    fqn_glob_ok, info_definition, info_forward, info_methodtype, list_procedure_body, not_object,
    set_list, set_oo_list,
};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_registry::commands::tcl::{info_oo_subcommands, InfoOoEnsembleKind};

/// `info class subcommand class ?arg?`.
pub(crate) fn info_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() < 3 {
            return wrong_args(interp, b"info class subcommand ?arg ...?");
        }
        let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Class, interp.runtime_version());
        let sub = match subcommands.resolve(&oo_original_bytes!(interp, originals, 2)) {
            Ok(sub) => sub.as_bytes(),
            Err(message) => return interp.set_error(&message),
        };
        if sub == b"call" && argv.len() != 5 {
            return wrong_args(interp, b"info class call className methodName");
        }
        let private_variables = if sub == b"variables" {
            match variable_info_options(interp, InfoOoEnsembleKind::Class, &argv[3..]) {
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
                        InfoOoEnsembleKind::Class,
                    )
                    .as_bytes(),
                );
            }
            return wrong_args(interp, b"info class subcommand className ?arg ...?");
        }
        let cls = oo_original_object!(interp, originals, 3);
        // C (`TclOOGetClassFromObj`) resolves the object first: a non-object name
        // reports `X does not refer to an object` (no quotes); an object that is
        // not a class reports `"X" is not a class`.
        if !interp.oo.borrow().objects.contains_key(&cls) {
            return not_object(interp, &oo_original_bytes!(interp, originals, 3));
        }
        if !interp.oo.borrow().classes.contains_key(&cls) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(&oo_original_bytes!(interp, originals, 3));
            m.extend_from_slice(b"\" is not a class");
            return err(interp, &m);
        }
        let target_owner = oo_checked!(
            interp,
            native_context::OriginalOoTarget::capture(interp, cls, true)
        );
        match sub {
            b"superclasses" => {
                let s = interp.oo.borrow().classes[&cls].supers.clone();
                set_oo_list(interp, &s);
                Code::Ok
            }
            b"mixins" => {
                let m = interp.oo.borrow().classes[&cls].mixins.clone();
                set_oo_list(interp, &m);
                Code::Ok
            }
            b"variables" | b"variable" => {
                let cl = interp.oo.borrow();
                let v = if private_variables {
                    cl.classes[&cls]
                        .private_variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                } else {
                    cl.classes[&cls]
                        .variables
                        .iter()
                        .map(variable_mapping::observation)
                        .collect::<Vec<_>>()
                };
                drop(cl);
                oo_checked!(interp, set_declared_variable_list(interp, &v));
                Code::Ok
            }
            b"properties" => {
                // `info class properties cls ?-all? ?-readable|-writable?` (TIP 558).
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
                    return match interp.native_all_property_header(cls, true, writable) {
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
                    return match interp.native_declared_property_header(cls, true, writable) {
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
                let props = interp.class_property_list(cls, writable, all);
                set_list(interp, &props);
                Code::Ok
            }
            b"instances" => {
                // `info class instances class ?pattern?` — direct instances, the
                // optional pattern glob-matching the instance command FQN.
                let pat = if argv.len() > 4 {
                    Some(oo_original_bytes!(interp, originals, 4))
                } else {
                    None
                };
                oo_checked!(interp, target_owner.check(interp));
                let mut insts: Vec<Vec<u8>> = interp
                    .oo
                    .borrow()
                    .objects
                    .iter()
                    .filter(|(_, o)| o.class == cls)
                    .map(|(k, _)| interp.oo_name(*k))
                    .filter(|name| fqn_glob_ok(pat.as_deref(), name))
                    .collect();
                insts.sort();
                set_list(interp, &insts);
                Code::Ok
            }
            b"subclasses" => {
                // `info class subclasses class ?pattern?` — direct subclasses, the
                // optional pattern glob-matching the subclass FQN (oo-17.8).
                let pat = if argv.len() > 4 {
                    Some(oo_original_bytes!(interp, originals, 4))
                } else {
                    None
                };
                oo_checked!(interp, target_owner.check(interp));
                let mut subs: Vec<Vec<u8>> = interp
                    .oo
                    .borrow()
                    .classes
                    .iter()
                    .filter(|(k, c)| **k != cls && c.supers.contains(&cls))
                    .map(|(k, _)| interp.oo_name(*k))
                    .filter(|name| fqn_glob_ok(pat.as_deref(), name))
                    .collect();
                subs.sort();
                set_list(interp, &subs);
                Code::Ok
            }
            b"methods" => {
                let reached = method_info_options(interp, &argv[4..]);
                oo_original_check!(interp, originals);
                oo_checked!(interp, target_owner.check(interp));
                let selected = match reached {
                    Ok(selected) => selected,
                    Err(code) => return code,
                };
                let mut names: Vec<Vec<u8>> = Vec::new();
                // `-all` traverses the class's full precedence (its mixins and the
                // mixins of its superclasses too), not just the superclass MRO
                // (oo-35.5).
                let chain = if selected.recurse() {
                    interp.class_precedence(cls)
                } else {
                    vec![cls]
                };
                for c in &chain {
                    if let Some(cl) = interp.oo.borrow().classes.get(c) {
                        for n in cl.methods.keys() {
                            let public = !cl.unexported.contains(n);
                            let show = if selected.recurse() {
                                !cl.private.contains(n)
                                    && (selected.includes_unexported() || public)
                            } else {
                                selected.matches_local(public, false, cl.private.contains(n))
                            };
                            if show && !names.contains(n) {
                                names.push(n.clone());
                            }
                        }
                    }
                }
                // `-all` reaches oo::object's built-ins: `destroy` (public, unless
                // unexported in the chain), plus the unexported set under `-private`
                // (oo-17.9, oo-17.10).
                if selected.recurse() {
                    let add = |n: &[u8], names: &mut Vec<Vec<u8>>| {
                        if !names.iter().any(|x| x == n) {
                            names.push(n.to_vec());
                        }
                    };
                    let destroy_unexported = chain.iter().any(|c| {
                        interp
                            .oo
                            .borrow()
                            .classes
                            .get(c)
                            .is_some_and(|cl| cl.unexported.contains(b"destroy".as_slice()))
                    });
                    if selected.includes_unexported() || !destroy_unexported {
                        add(b"destroy", &mut names);
                    }
                    if selected.includes_unexported() {
                        for b in [
                            &b"<cloned>"[..],
                            b"eval",
                            b"unknown",
                            b"variable",
                            b"varname",
                        ] {
                            add(b, &mut names);
                        }
                    }
                }
                names.sort();
                set_list(interp, &names);
                Code::Ok
            }
            b"constructor" => {
                let body = match interp.oo.borrow().classes[&cls].constructor.get() {
                    Some(Method::Body { procedure, .. }) => {
                        list_procedure_body(interp, &procedure.declaration())
                    }
                    _ => obj::new_string_bytes(b""),
                };
                interp.set_result(body);
                Code::Ok
            }
            b"destructor" => {
                let procedure = interp.oo.borrow().classes[&cls].destructor.get().and_then(
                    |method| match method {
                        Method::Body { procedure, .. } => Some(procedure.declaration()),
                        _ => None,
                    },
                );
                match procedure {
                    Some(procedure) => match procedure.body.checked_ptr() {
                        Ok(original) => interp.set_result(original),
                        Err(error) => return interp.report_cmd_error(error.into()),
                    },
                    None => interp.set_result_bytes(b""),
                }
                Code::Ok
            }
            b"definitionnamespace" => {
                // `info class definitionnamespace className ?kind?` (TIP 524).
                if argv.len() > 5 {
                    return wrong_args(interp, b"info class definitionnamespace className ?kind?");
                }
                let kind = if let Some(&original) = argv.get(4) {
                    interp.native_static_option_index(
                        original,
                        DEFINITION_KINDS.names(),
                        false,
                        "kind",
                    )
                } else {
                    Ok(0)
                };
                oo_original_check!(interp, originals);
                oo_checked!(interp, target_owner.check(interp));
                let ns = match kind {
                    // The namespace used to define this class itself.
                    Ok(0) => interp.oo.borrow().classes[&cls].class_def_ns.clone(),
                    // The namespace used to define this class's instances.
                    Ok(_) => interp.oo.borrow().classes[&cls].def_ns.clone(),
                    Err(error) => return interp.report_cmd_error(error),
                };
                interp.set_result_bytes(&ns.unwrap_or_default());
                Code::Ok
            }
            b"filters" => {
                let f = interp.oo.borrow().classes[&cls].filters.clone();
                set_list(interp, &f);
                Code::Ok
            }
            b"forward" => info_forward(interp, cls, argv, true),
            b"definition" => info_definition(interp, cls, argv, true),
            b"methodtype" => info_methodtype(interp, cls, argv, true),
            b"call" => info_call(interp, cls, argv, true),
            // `properties` is not yet modelled.
            other => {
                let mut m = b"unsupported info class subcommand \"".to_vec();
                m.extend_from_slice(other);
                m.push(b'"');
                err(interp, &m)
            }
        }
    })
}
