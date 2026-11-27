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

//! `namespace ensemble create`, `configure` and `exists`, and the ensemble option table.

use crate::ensemble::EnsembleConfig;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{error_code_list, Code, Interp};
use crate::namespace::NsId;
use crate::obj::TclObj;
use crate::{list, obj};
use tcl_syntax::value::ValueOps;

// ensemble

/// `namespace ensemble create|exists ...` — the canonical `ens sub`→target
/// redirect (the generalised `dict for`→`::tcl::dict::for` mechanism).
pub(super) fn ns_ensemble(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_native_model::native_jim_lookup::NativeJimNamespaceHelperKind::Ensemble,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
    if let Some(protocol) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    {
        let lifecycle = match interp.native_current_namespace_lifecycle() {
            Ok(lifecycle) => lifecycle,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        if !protocol.permits_namespace_lifecycle(lifecycle) {
            return interp.report_cmd_error(tcl_cmd_core::CmdError::with_error_code_bytes(
                b"tried to manipulate ensemble of deleted namespace".to_vec(),
                b"TCL ENSEMBLE DEAD".to_vec(),
            ));
        }
    }

    if argv.len() < 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"subcommand ?arg ...?");
    }
    // The shared `ensembleSubcommands` table (flags 0, so `cr` is `create`).
    match interp.native_static_string_option_index(
        argv[2],
        tcl_cmd_core::ensemble::SUBCOMMANDS.names(),
        false,
        "subcommand",
    ) {
        Ok(0) => ens_configure(interp, argv),
        Ok(1) => ens_create(interp, argv),
        Ok(_) => ens_exists(interp, argv),
        Err(message) => interp.report_cmd_error(message),
    }
}

/// `namespace ensemble create ?-command name? ?-map dict? ?-subcommands list?
/// ?-prefixes bool?` — register an ensemble over the current namespace.
pub(super) fn ens_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(options) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble configuration options",
            )
            .into(),
        );
    };

    let ns = interp.current_ns();
    // Mapping targets use the implementation namespace; publication retains
    // the original -command operand and its independent token routing.
    let mut command = None;
    let mut cfg = EnsembleConfig {
        originals: Default::default(),
        ns,
        map: None,
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: Vec::new(),
    };

    let opts = &argv[3..];
    // C checks the pair arity before it looks at any option word
    // (`if (objc & 1)` → `wrong # args`, `tclEnsemble.c:192-196`).
    if opts.len() % 2 != 0 {
        return interp.wrong_args_for_prefix(argv, 2, b"create ?option value ...?");
    }
    for pair in opts.chunks_exact(2) {
        // `ensembleCreateOptions`: `-command` (create-only) names the ensemble
        // command, the rest are the shared configuration options, and there is
        // deliberately no `-namespace`.
        let resolved = match interp.native_static_string_option_index(
            pair[0],
            options.create_options(),
            false,
            "option",
        ) {
            Ok(index) => options.create_option(index),
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(shared) = resolved.shared() else {
            command = Some(match interp.native_string_bytes(&pair[1]) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error.into()),
            });
            continue;
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, pair[1], ns, interp, true) {
            return interp.report_cmd_error(e);
        }
    }

    let selected = interp
        .namespaces_mut()
        .ensemble_publication_at(ns, command.as_deref());
    let Some((holder, simple)) = selected else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble publication context",
            )
            .into(),
        );
    };
    let reporting = interp.namespaces().command_fqn_at(holder, &simple);
    interp.create_ensemble_in_slot(holder, &simple, cfg);
    interp.set_result_bytes(&reporting);
    Code::Ok
}

fn native_ensemble_map_prefix_input<'a>(
    interp: &Interp,
    namespace: NsId,
    original: &'a [u8],
) -> Result<std::borrow::Cow<'a, [u8]>, tcl_syntax::value::ValueError> {
    let unavailable = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "ensemble map-prefix construction",
    );
    let protocol = interp
        .invocation_name_protocol()
        .ok_or_else(|| unavailable.clone())?;
    let report = interp.namespaces().qualified_name(namespace);
    let has_parent = interp.namespaces().parent(namespace).is_some();
    protocol
        .ensemble_map_prefix_input(&report, has_parent, original)
        .map_err(|_| unavailable)
}

/// Apply one already-resolved shared `-option value` to an
/// [`EnsembleConfig`] (`namespace ensemble create` and `configure` both land
/// here; the option word itself is resolved by the caller's own table).
/// Returns the C error text on a bad value.
///
/// `map_ns` is the namespace unqualified `-map` targets are resolved against.
/// C uses the ensemble's own namespace on `create` and the namespace current
/// at the call on `configure`; both are the current namespace at the point
/// each command runs, which is what every caller passes.
fn apply_ensemble_option(
    cfg: &mut EnsembleConfig,
    opt: tcl_cmd_core::ensemble::SharedOption,
    value: *mut TclObj,
    map_ns: NsId,
    interp: &Interp,
    creating: bool,
) -> Result<(), tcl_cmd_core::CmdError> {
    use tcl_cmd_core::ensemble::{EnsembleObjectRole, SharedOption};
    if opt == SharedOption::Prefixes {
        cfg.prefixes =
            crate::typed_value::native_boolean(value, interp.native_invocation_dialect())?;
        return Ok(());
    }
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configured objects",
        ))?
        .protocol();
    if opt == SharedOption::Map {
        let pairs = crate::dict::native_dict_pairs(value, protocol)?;
        let mut patched: Option<obj::Owned> = None;
        let mut map = Vec::with_capacity(pairs.len());
        for (key, prefix) in pairs {
            let mut members = list::list_elements_native_checked(prefix, protocol)?;
            if members.is_empty() {
                return Err(tcl_cmd_core::CmdError::with_error_code_bytes(
                    b"ensemble subcommand implementations must be non-empty lists".to_vec(),
                    b"TCL ENSEMBLE EMPTY_TARGET".to_vec(),
                ));
            }
            let target = crate::dict::native_object_bytes(members[0], protocol)?;
            let selected = native_ensemble_map_prefix_input(interp, map_ns, &target)?;
            if let std::borrow::Cow::Owned(qualified) = selected {
                let qualified = obj::Owned::fresh(obj::new_string_bytes(&qualified));
                let copy = obj::Owned::fresh(if creating {
                    list::new_list_obj_native(&members, protocol)
                } else {
                    obj::duplicate(prefix)
                });
                members[0] = qualified.as_ptr();
                let replacement = list::replace_elements_native(copy.as_ptr(), &members, protocol)?;
                let root = patched.get_or_insert_with(|| obj::Owned::fresh(obj::duplicate(value)));
                crate::dict::dict_set(root.as_ptr(), key, replacement.as_ptr()).map_err(
                    |error| tcl_cmd_core::CmdError::new_bytes(error.message_bytes().to_vec()),
                )?;
            }
            let name = crate::dict::native_object_bytes(key, protocol)?;
            let words = members
                .iter()
                .map(|&word| crate::dict::native_object_bytes(word, protocol))
                .collect::<Result<Vec<_>, _>>()?;
            map.push((name, words));
        }
        cfg.map = (!map.is_empty()).then_some(map);
        cfg.originals.map = cfg.map.as_ref().map(|_| {
            EnsembleObjectRole::new(patched.map_or_else(
                || crate::ensemble::NativeEnsembleRoot::pending(value),
                crate::ensemble::NativeEnsembleRoot::owned,
            ))
        });
        return Ok(());
    }
    let members = list::list_elements_native_checked(value, protocol)?;
    let words = members
        .iter()
        .map(|&word| crate::dict::native_object_bytes(word, protocol))
        .collect::<Result<Vec<_>, _>>()?;
    let role = (!members.is_empty())
        .then(|| EnsembleObjectRole::new(crate::ensemble::NativeEnsembleRoot::pending(value)));
    match opt {
        SharedOption::Subcommands => {
            cfg.subcommands = (!words.is_empty()).then_some(words);
            cfg.originals.subcommands = role;
        }
        SharedOption::Parameters => {
            cfg.parameters = words;
            cfg.originals.parameters = role;
        }
        SharedOption::Unknown => {
            cfg.unknown = words;
            cfg.originals.unknown = role;
        }
        SharedOption::Map | SharedOption::Prefixes => unreachable!("handled above"),
    }
    Ok(())
}

/// `namespace ensemble configure cmd ?-option? ?value …?` — read or update an
/// existing ensemble's configuration (`tclEnsemble.c`). No options: a dict of
/// all settings; one bare `-option`: its value; `-option value …` pairs: update.
pub(super) fn ens_configure(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let Some(options) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble configuration options",
            )
            .into(),
        );
    };

    if argv.len() < 4 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    let cmd = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    // Follow a `namespace import` alias to the ensemble that owns the config:
    // reads and writes both act on the origin, so both spellings observe one
    // configuration and the alias stays an alias.
    let Some(token) = interp.ensemble_config_at(&cmd) else {
        // Distinguish a missing command from a non-ensemble one (C's wording).
        if interp.command_exists(&cmd) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(&cmd);
            m.extend_from_slice(b"\" is not an ensemble command");
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"ENSEMBLE", &cmd]);
            return interp.error_with_code(&m, &error_code);
        }
        let mut m = b"unknown command \"".to_vec();
        m.extend_from_slice(&cmd);
        m.push(b'"');
        let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &cmd]);
        return interp.error_with_code(&m, &error_code);
    };
    let mut cfg = token.config();
    let rest = &argv[4..];
    // Native configure returns the ordered key/value List.
    if rest.is_empty() {
        let d = match ensemble_config_list(interp, &cfg) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        // The checked owned List remains live through result publication.
        unsafe {
            interp.set_obj_result(d.as_ptr());
        }
        return Code::Ok;
    }
    // Read a single option's value (`ensembleConfigOptions`, abbreviating).
    if rest.len() == 1 {
        return match interp.native_static_string_option_index(
            rest[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => match ensemble_option_value(interp, &cfg, options.configure_option(index))
            {
                Ok(value) => {
                    // The selected original option root is retained by this owner.
                    unsafe {
                        interp.set_obj_result(value.as_ptr());
                    }
                    Code::Ok
                }
                Err(error) => interp.report_cmd_error(error.into()),
            },
            Err(error) => interp.report_cmd_error(error),
        };
    }
    // Update: `-option value` pairs. C's arity gate is
    // `objc != 4 && !(objc & 1)`, i.e. anything but 0, 1, or an even number of
    // trailing words is `wrong # args`.
    if rest.len() % 2 != 0 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    // CONF_MAP qualifies against `TclGetCurrentNamespace(interp)` — the
    // namespace current at the `configure` call, NOT the ensemble's own
    // namespace (which CRT_MAP uses at create time). They coincide in the
    // common `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    let map_ns = interp.current_ns();
    for pair in rest.chunks_exact(2) {
        let resolved = match interp.native_static_string_option_index(
            pair[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => options.configure_option(index),
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(shared) = resolved.shared() else {
            return interp
                .error_with_code(b"option -namespace is read-only", b"TCL ENSEMBLE READ_ONLY");
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, pair[1], map_ns, interp, false) {
            return interp.report_cmd_error(e);
        }
    }
    cfg.originals.activate();
    token.configure(cfg);
    interp.note_native_ensemble_configuration_changed(&token);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// One ensemble `configure`/cget option's value (string form).
fn ensemble_option_value(
    interp: &Interp,
    cfg: &EnsembleConfig,
    opt: tcl_cmd_core::ensemble::ConfigOption,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    use tcl_cmd_core::ensemble::ConfigOption;
    let original = match opt {
        ConfigOption::Map => &cfg.originals.map,
        ConfigOption::Unknown => &cfg.originals.unknown,
        ConfigOption::Parameters => &cfg.originals.parameters,
        ConfigOption::Subcommands => &cfg.originals.subcommands,
        ConfigOption::Namespace | ConfigOption::Prefixes => &None,
    };
    if let Some(pointer) = crate::ensemble::NativeEnsembleObjects::pointer(original) {
        return Ok(obj::Owned::retain(pointer));
    }
    if opt == ConfigOption::Prefixes {
        return Ok(obj::Owned::fresh(obj::new_boolean_obj(i32::from(
            cfg.prefixes,
        ))));
    }
    let bytes = match opt {
        ConfigOption::Namespace => interp.namespaces().qualified_name(cfg.ns),
        ConfigOption::Prefixes => {
            if cfg.prefixes {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        ConfigOption::Parameters => join_words(interp, &cfg.parameters)?,
        ConfigOption::Unknown => join_words(interp, &cfg.unknown)?,
        ConfigOption::Subcommands => cfg
            .subcommands
            .as_deref()
            .map(|words| join_words(interp, words))
            .transpose()?
            .unwrap_or_default(),
        ConfigOption::Map => match &cfg.map {
            Some(map) => {
                let mut words = Vec::new();
                for (key, prefix) in map {
                    words.push(key.clone());
                    words.push(join_words(interp, prefix)?);
                }
                join_words(interp, &words)?
            }
            None => Vec::new(),
        },
    };
    Ok(obj::Owned::fresh(obj::new_string_bytes(&bytes)))
}

pub(super) fn join_words(
    interp: &Interp,
    words: &[Vec<u8>],
) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace ensemble configuration List string",
        ))?
        .protocol();
    let elements: Vec<_> = words
        .iter()
        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
        .collect();
    let pointers: Vec<_> = elements.iter().map(obj::Owned::as_ptr).collect();
    let value = obj::Owned::fresh(list::new_list_obj_native(&pointers, protocol));
    crate::dict::native_object_bytes(value.as_ptr(), protocol)
}

fn ensemble_config_list(
    interp: &Interp,
    cfg: &EnsembleConfig,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace ensemble configuration List",
        ))?
        .protocol();
    let mut members = Vec::new();
    let options = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configuration options",
        ))?;
    for option in options.configuration_options() {
        members.push(obj::Owned::fresh(obj::new_string_bytes(
            option.name().as_bytes(),
        )));
        members.push(ensemble_option_value(interp, cfg, option)?);
    }
    let pointers: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
    Ok(obj::Owned::fresh(list::new_list_obj_native(
        &pointers, protocol,
    )))
}

/// `namespace ensemble exists command` — 1 if it resolves to an ensemble.
fn ens_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    if argv.len() != 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"exists cmdname");
    }
    let name = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let exists = interp.is_ensemble(&name);
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}
