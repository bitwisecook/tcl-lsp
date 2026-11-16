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

//! TIP 558 configurable objects: the `oo::configurable` metaclass, the `property` definition command, the `configure` method and the property sets.

use super::definition::{def_target, install_method_vis, MethodVis};
use super::installation::install_abstract_singleton;
use super::introspection::set_list;
use super::{native_context, native_properties, wrong_args, DefTarget, Method};
use crate::interp::{Code, Command, Interp, Param};
use crate::namespace::GLOBAL;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_core_types::OoId;
use tcl_registry::commands::tcl::{
    resolve_info_oo_properties_option_original, InfoOoPropertiesOption, TclOoPropertyOption,
};
use tcl_registry::native_tcloo_method_definition::{
    NativeTclooCoreMethod, NativeTclooMethodPayloadType,
};

/// TIP 558 `oo::configurable` metaclass + the `property` definition command and
/// the `configure` method (C's tclOOProp.c / configurable wiring). A class made
/// with `oo::configurable create` mixes in `::oo::configuresupport::configurable`
/// (for `configure`) and resolves `property` in the configurableclass /
/// configurableobject definition namespaces.
pub(super) fn install_configurable(interp: &mut Interp) {
    // naming.tcloo.original-configurable-factory-support
    // docs/design/analysis/name-resolution-proofs/tcloo-original-configurable-factory-support.md
    let registry = crate::environment::store_for_profile(interp.dialect_profile());
    if let Some(support) = registry
        .native_class_factory_recipe(
            "::oo::configurable",
            interp.native_invocation_dialect(),
            tcl_dialect::model::InvocationRealm::default(),
        )
        .and_then(tcl_registry::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
    {
        for receiver in [
            tcl_registry::definer::DefinitionReceiver::Instance,
            tcl_registry::definer::DefinitionReceiver::Class,
        ] {
            let namespace =
                interp.ensure_namespace(support.definition_namespace(receiver).as_bytes());
            let path = support
                .definition_path(receiver)
                .iter()
                .map(|name| interp.ensure_namespace(name.as_bytes()))
                .collect();
            interp.namespaces_mut().set_path(namespace, path);
        }
    }
    // The two definition namespaces hold the `property` command (class /
    // instance variants), reached via TIP 524 definition-namespace resolution.
    let _ = interp.eval_str(
        b"namespace eval ::oo::configuresupport::configurableclass {}; \
          namespace eval ::oo::configuresupport::configurableobject {}",
    );
    interp.ns_register(
        b"::oo::configuresupport::configurableclass::property",
        Command::Builtin(prop_define_class),
    );
    interp.ns_register(
        b"::oo::configuresupport::configurableobject::property",
        Command::Builtin(prop_define_object),
    );
    // The mixin that gives configurable objects their `configure` method.
    let _ = interp.eval_str(b"oo::class create ::oo::configuresupport::configurable {}");
    let configurable_support =
        match interp.oo_resolve_object_bytes(b"::oo::configuresupport::configurable") {
            Ok(object) => object,
            Err(error) => {
                interp.report_cmd_error(error);
                return;
            }
        };
    if let Some(c) = interp
        .oo
        .borrow_mut()
        .classes
        .get_mut(&configurable_support)
    {
        drop(c.methods.insert_generated(
            b"configure".to_vec(),
            Method::builtin(
                oo_configure,
                NativeTclooMethodPayloadType::Core(NativeTclooCoreMethod::Configure),
            ),
        ));
    }
    // The `oo::configurable` metaclass. Its instances (configurable classes) get
    // the configurable support set up natively in `oo_new_ns` — mixing in the
    // support class and pointing instance definitions at configurableobject —
    // so the definition script still resolves class-name arguments in the
    // caller's namespace. Its own definition namespace is configurableclass so
    // `property` resolves while defining those classes.
    let _ = interp.eval_str(
        b"oo::class create ::oo::configurable { superclass ::oo::class }\n\
          oo::define ::oo::configurable definitionnamespace ::oo::configuresupport::configurableclass",
    );
    let configurable_root = match interp.oo_resolve_object_bytes(b"::oo::configurable") {
        Ok(object) => object,
        Err(error) => {
            interp.report_cmd_error(error);
            return;
        }
    };
    interp.oo.borrow_mut().configurable_root = Some(configurable_root);
    install_abstract_singleton(interp);
    // The 9.0 metaclasses are engine-installed on the registry's behalf too,
    // so the release gate hides them below their introducing release the way
    // it hides a builtin — real tclsh 8.6.16 has no `oo::configurable`.
    let mut roots = vec![b"::oo::configurable".as_slice(), b"::oo::singleton"];
    if let Some(recipe) = tcl_registry::native_tcloo_bootstrap::NativeAbstractClassBootstrap::select(
        interp.native_invocation_dialect(),
    ) {
        roots.push(recipe.command().as_bytes());
    }
    for root in roots {
        let owner = match interp.oo_resolve_object_bytes(root) {
            Ok(object) => object,
            Err(error) => {
                interp.report_cmd_error(error);
                return;
            }
        };
        interp.declare_registry_object_root(owner, root);
    }
    // NOTE: `::oo::Slot` and `::oo::SingletonInstance` are engine-installed
    // too, but marking them roots here would be inert — the release gate dates
    // a root through the registry, and the registry has no spec for either
    // name, so `profile_admits_registry_builtin` admits them on every surface.
    // They are therefore still present (and consistently callable) on an 8.4
    // surface that should have no TclOO at all. That is a registry-content
    // gap, not a gate gap; the release gate itself is mirrored correctly
    // without them.
}

fn prop_define_class(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    prop_define(interp, argv, false)
}

fn prop_define_object(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    prop_define(interp, argv, true)
}

/// `property name ?-get body? ?-set body? ?-kind k? …` (TIP 558,
/// `TclOODefinePropertyCmd`): declare configurable properties on the current
/// definition target. Default props read/write the like-named instance
/// variable through generated `<ReadProp-name>` / `<WriteProp-name>` methods;
/// `-get`/`-set` supply custom bodies. Registers `-name` in the readable /
/// writable property set per `-kind` (readable / readwrite [default] / writable).
pub(super) fn prop_define(interp: &mut Interp, argv: &[*mut TclObj], use_instance: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        prop_define_original(interp, argv, use_instance, originals)
    })
}

fn prop_define_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    use_instance: bool,
    originals: &crate::interp::native_arguments::OriginalArguments,
) -> Code {
    let target = match def_target(interp) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let target_owner = oo_checked!(
        interp,
        native_context::OriginalOoTarget::for_definition(interp, &target)
    );
    let Some(recipe) = interp
        .native_invocation_dialect()
        .native_property_name_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native property name protocol",
            )
            .into(),
        );
    };
    let mut i = 1;
    while i < argv.len() {
        let original_property = argv[i];
        let prop = oo_original_bytes!(interp, originals, i);
        oo_checked!(interp, target_owner.check(interp));
        i += 1;
        // Parse the property's options.
        let (mut kind_ro, mut kind_wo) = (false, false);
        let mut getter: Option<Vec<u8>> = None;
        let mut setter: Option<Vec<u8>> = None;
        while i < argv.len() {
            if oo_original_bytes!(interp, originals, i).first() != Some(&b'-') {
                break;
            }
            oo_checked!(interp, target_owner.check(interp));
            let option =
                interp.native_index_operand(argv[i], &recipe.definition_options(), false, "option");
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            let option = match option {
                Ok(0) => TclOoPropertyOption::Get,
                Ok(1) => TclOoPropertyOption::Kind,
                Ok(2) => TclOoPropertyOption::Set,
                Ok(_) => unreachable!("closed property option roster"),
                Err(error) => return interp.report_cmd_error(error),
            };
            if i + 1 >= argv.len() {
                let what: &[u8] = if option == TclOoPropertyOption::Kind {
                    b"kind value"
                } else {
                    b"body"
                };
                let mut m = b"missing ".to_vec();
                m.extend_from_slice(what);
                m.extend_from_slice(b" to go with ");
                m.extend_from_slice(option.name().as_bytes());
                m.extend_from_slice(b" option");
                return interp.error_with_code(&m, b"TCL WRONGARGS");
            }
            let original_value = argv[i + 1];
            i += 2;
            match option {
                TclOoPropertyOption::Get => {
                    getter = Some(oo_original_bytes!(interp, originals, i - 1))
                }
                TclOoPropertyOption::Set => {
                    setter = Some(oo_original_bytes!(interp, originals, i - 1))
                }
                TclOoPropertyOption::Kind => {
                    let kind = interp.native_index_operand(
                        original_value,
                        &recipe.definition_kinds(),
                        false,
                        "kind",
                    );
                    oo_original_check!(interp, originals);
                    oo_checked!(interp, target_owner.check(interp));
                    match kind {
                        Ok(0) => {
                            kind_ro = true;
                            kind_wo = false;
                        }
                        Ok(1) => {
                            kind_ro = false;
                            kind_wo = false;
                        }
                        Ok(2) => {
                            kind_ro = false;
                            kind_wo = true;
                        }
                        Ok(_) => unreachable!("closed property kind roster"),
                        Err(error) => return interp.report_cmd_error(error),
                    }
                }
            }
            oo_checked!(interp, target_owner.check(interp));
        }

        oo_checked!(interp, target_owner.check(interp));
        if let Err(message) = recipe.validate_declaration(&prop) {
            return interp.report_cmd_error(
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    message,
                    b"TCL OO PROPERTY_FORMAT".to_vec(),
                )
                .with_native_string_result(recipe.strings()),
            );
        }
        let prop = tcl_core_types::c_string_extent(&prop).to_vec();
        let readable = !kind_wo;
        let writable = !kind_ro;
        // Install the accessor methods (`<ReadProp-name>` / `<WriteProp-name>`),
        // unexported (their `<…>` names are non-lowercase). Default impls read /
        // write the like-named instance variable.
        if readable {
            let mname = recipe.accessor_names(&prop).0;
            let code = if getter.is_none() {
                install_method_vis(
                    interp,
                    mname,
                    Method::Property(Rc::new(native_properties::NativePropertyAccessor::new(
                        original_property,
                        false,
                    ))),
                    MethodVis::Unexported,
                )
            } else {
                let body = getter.clone().expect("selected custom property getter");
                install_property_method(interp, &mname, &[], &body)
            };
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            if code != Code::Ok {
                return code;
            }
        }
        if writable {
            let mname = recipe.accessor_names(&prop).1;
            let code = if setter.is_none() {
                install_method_vis(
                    interp,
                    mname,
                    Method::Property(Rc::new(native_properties::NativePropertyAccessor::new(
                        original_property,
                        true,
                    ))),
                    MethodVis::Unexported,
                )
            } else {
                let body = setter.clone().expect("selected custom property setter");
                install_property_method(interp, &mname, b"value", &body)
            };
            oo_original_check!(interp, originals);
            oo_checked!(interp, target_owner.check(interp));
            if code != Code::Ok {
                return code;
            }
        }
        oo_checked!(interp, target_owner.check(interp));
        // Register `-name`: add to (or remove from) each set per the kind, so a
        // later declaration with a different `-kind` replaces the membership
        // (C's TclOORegisterProperty add/remove; ooProp-3.16).
        let mut hyph = b"-".to_vec();
        hyph.extend_from_slice(&prop);
        if interp
            .native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
        {
            let (id, class) = match &target {
                DefTarget::Class(id) => (*id, !use_instance),
                DefTarget::Object(id) => (*id, false),
            };
            if let Err(error) =
                native_properties::register_property(interp, id, class, &hyph, readable, writable)
            {
                return interp.report_cmd_error(error);
            }
        } else {
            set_property_membership(interp, &target, use_instance, false, &hyph, readable);
            set_property_membership(interp, &target, use_instance, true, &hyph, writable);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

pub(super) fn native_method_body(
    interp: &Interp,
    params: Vec<Param>,
    body: obj::Owned,
    source: Option<(Rc<[u8]>, u32)>,
) -> Method {
    let (source, base) = source.map_or((None, 0), |(file, base)| (Some(file), base));
    let procedure = interp.create_native_callable_from_chosen(params, body, GLOBAL, source, base);
    Method::Body {
        procedure,
        report_type: NativeTclooMethodPayloadType::Procedure,
    }
}

fn install_property_method(interp: &mut Interp, name: &[u8], params: &[u8], body: &[u8]) -> Code {
    let params = match crate::cmd_proc::parse_params_in(interp, params) {
        Ok(p) => p,
        Err(error) => return interp.report_cmd_error(error),
    };
    let original = obj::Owned::fresh(obj::new_string_bytes(body));
    let method = native_method_body(interp, params, original, None);
    install_method_vis(interp, name.to_vec(), method, MethodVis::Unexported)
}

/// Add or remove `prop` (already hyphenated) in a definition target's
/// readable/writable property set (C's `BuildPropertyList`).
fn set_property_membership(
    interp: &mut Interp,
    target: &DefTarget,
    use_instance: bool,
    writable: bool,
    prop: &[u8],
    member: bool,
) {
    let mut set = read_property_set(interp, target, use_instance, writable);
    let present = set.iter().any(|p| p.as_slice() == prop);
    if member && !present {
        set.push(prop.to_vec());
    } else if !member && present {
        set.retain(|p| p.as_slice() != prop);
    } else {
        return; // no change
    }
    write_property_set(interp, target, use_instance, writable, set);
}

/// `configure` method (TIP 558, `TclOO_Configurable_Configure`): with no args
/// returns the readable properties as an `-opt value …` dict; with one `-opt`
/// reads it; with `-opt value …` pairs writes them.
fn oo_configure(interp: &mut Interp, obj: OoId, args: &[*mut TclObj]) -> Code {
    let n = args.len();
    if n != 1 && n % 2 == 1 {
        let mut u = interp.oo_name(obj);
        u.extend_from_slice(b" configure ?-option value ...?");
        return wrong_args(interp, &u);
    }
    if interp
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return native_properties::configure_native(interp, obj, args);
    }
    if n == 0 {
        // Read every readable property into a dict (sorted by name).
        let props = interp.object_property_list(obj, false, true);
        let mut pairs: Vec<Vec<u8>> = Vec::with_capacity(props.len() * 2);
        for p in props {
            let code = read_property(interp, obj, &p);
            if code != Code::Ok {
                return code;
            }
            pairs.push(p);
            pairs.push(interp.result_bytes());
        }
        set_list(interp, &pairs);
        return Code::Ok;
    }
    if n == 1 {
        let name = match get_property_name(interp, obj, args[0], false, &mut None, false) {
            Ok(n) => n,
            Err(c) => return c,
        };
        return read_property(interp, obj, &name);
    }
    let mut i = 0;
    let mut property_table = None;
    while i < n {
        let name = match get_property_name(interp, obj, args[i], true, &mut property_table, true) {
            Ok(n) => n,
            Err(c) => return c,
        };
        let code = write_property(interp, obj, &name, args[i + 1]);
        if code != Code::Ok {
            return code;
        }
        i += 2;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Read a property by invoking the object's `<ReadProp-name>` method via `my`.
/// A `break`/`continue` from the getter is turned into an error (C's
/// ReadProperty).
fn read_property(interp: &mut Interp, obj: OoId, prop_hyph: &[u8]) -> Code {
    let mname = interp
        .native_invocation_dialect()
        .native_property_name_protocol()
        .expect("selected property name")
        .accessor_names(prop_hyph.strip_prefix(b"-").unwrap_or(prop_hyph))
        .0;
    let code = interp.oo_invoke(obj, &mname, &[], false, None);
    property_loopword_error(interp, code, b"getter", prop_hyph)
}

fn write_property(interp: &mut Interp, obj: OoId, prop_hyph: &[u8], value: *mut TclObj) -> Code {
    let mname = interp
        .native_invocation_dialect()
        .native_property_name_protocol()
        .expect("selected property name")
        .accessor_names(prop_hyph.strip_prefix(b"-").unwrap_or(prop_hyph))
        .1;
    let code = interp.oo_invoke(obj, &mname, &[value], false, None);
    property_loopword_error(interp, code, b"setter", prop_hyph)
}

/// Map a property accessor's `break`/`continue` to the C error message
/// (`property getter|setter for -x did a break|continue`).
pub(super) fn property_loopword_error(
    interp: &mut Interp,
    code: Code,
    role: &[u8],
    prop_hyph: &[u8],
) -> Code {
    let word: &[u8] = match code {
        Code::Break => b"break",
        Code::Continue => b"continue",
        _ => return code,
    };
    let mut m = b"property ".to_vec();
    m.extend_from_slice(role);
    m.extend_from_slice(b" for ");
    m.extend_from_slice(prop_hyph);
    m.extend_from_slice(b" did a ");
    m.extend_from_slice(word);
    interp.set_error(&m)
}

/// Resolve a `configure` option to a property name in the object's readable /
/// writable set, or produce the C error (`bad property` / `is read|write only`).
fn get_property_name(
    interp: &mut Interp,
    obj: OoId,
    given: *mut TclObj,
    writable: bool,
    _table: &mut Option<obj::Owned>,
    _retain_table: bool,
) -> Result<Vec<u8>, Code> {
    let Some(recipe) = interp
        .native_invocation_dialect()
        .native_property_name_protocol()
    else {
        return Err(interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native property name protocol",
            )
            .into(),
        ));
    };
    let given = match interp.native_object_string_bytes(given) {
        Ok(bytes) => bytes,
        Err(error) => return Err(interp.report_cmd_error(error.into())),
    };
    let candidates = interp.object_property_list(obj, writable, true);
    let names: Vec<_> = candidates.iter().map(Vec::as_slice).collect();
    let miss = match recipe.lookup(&given, &names) {
        Ok(index) => return Ok(candidates[index].clone()),
        Err(message) => message,
    };
    let other = interp.object_property_list(obj, !writable, true);
    let names: Vec<_> = other.iter().map(Vec::as_slice).collect();
    let mut message = miss;
    if let Ok(index) = recipe.lookup(&given, &names) {
        message = b"property \"".to_vec();
        message.extend_from_slice(tcl_core_types::c_string_extent(names[index]));
        message.extend_from_slice(if writable {
            b"\" is read only"
        } else {
            b"\" is write only"
        });
    }
    Err(interp.report_cmd_error(
        tcl_cmd_core::CmdError::lookup_index_bytes(
            message,
            b"property",
            tcl_core_types::c_string_extent(&given),
        )
        .with_native_string_result(recipe.strings()),
    ))
}

/// Parse the `info … properties` option flags (`-all`, `-readable` [default],
/// `-writable`) from `opts`, returning `(all, writable)` or an error code.
pub(super) fn parse_property_opts(
    interp: &mut Interp,
    opts: &[*mut TclObj],
) -> Result<(bool, bool), Code> {
    let (mut all, mut writable) = (false, false);
    for &a in opts {
        match resolve_info_oo_properties_option_original(interp, &a) {
            Ok(InfoOoPropertiesOption::All) => all = true,
            Ok(InfoOoPropertiesOption::Readable) => writable = false,
            Ok(InfoOoPropertiesOption::Writable) => writable = true,
            Err(error) => return Err(interp.report_cmd_error(error)),
        }
    }
    Ok((all, writable))
}

/// The current value of a definition target's readable/writable property set.
/// `want_object` reads the per-object set (the object's entry); otherwise the
/// class set (the class entry). A class is registered in both maps, so the
/// target FQN is the key in either case.
pub(super) fn read_property_set(
    interp: &Interp,
    target: &DefTarget,
    want_object: bool,
    writable: bool,
) -> Vec<Vec<u8>> {
    let fqn = match target {
        DefTarget::Class(c) | DefTarget::Object(c) => c,
    };
    let oo = interp.oo.borrow();
    if want_object {
        oo.objects.get(fqn).map(|o| {
            if writable {
                o.writable_properties.clone()
            } else {
                o.readable_properties.clone()
            }
        })
    } else {
        oo.classes.get(fqn).map(|c| {
            if writable {
                c.writable_properties.clone()
            } else {
                c.readable_properties.clone()
            }
        })
    }
    .unwrap_or_default()
}

pub(super) fn write_property_set(
    interp: &mut Interp,
    target: &DefTarget,
    want_object: bool,
    writable: bool,
    set: Vec<Vec<u8>>,
) {
    let fqn = match target {
        DefTarget::Class(c) | DefTarget::Object(c) => *c,
    };
    let mut oo = interp.oo.borrow_mut();
    if want_object {
        if let Some(o) = oo.objects.get_mut(&fqn) {
            if writable {
                o.writable_properties = set;
            } else {
                o.readable_properties = set;
            }
        }
    } else if let Some(c) = oo.classes.get_mut(&fqn) {
        if writable {
            c.writable_properties = set;
        } else {
            c.readable_properties = set;
        }
    }
}
