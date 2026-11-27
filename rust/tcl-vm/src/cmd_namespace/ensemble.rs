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

//! `namespace ensemble create`, `configure` and `exists`: option parsing, the subcommand map and configuration reports.

use super::namespace_command_lookup_error;
use crate::command::err_with_code;
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_registry::native_scripted_distribution::ScriptedHelperDialect;
use tcl_runtime_api::Completion;
use tcl_syntax::native_namespace_name::NativeEnsembleNamespaceResultStage;

/// `namespace ensemble create|exists|configure` (`TclNamespaceEnsembleCmd`,
/// `tclEnsemble.c:140`). The subcommand word resolves through the shared
/// `ensembleSubcommands` table, so `namespace ensemble cr` is `create` and a
/// miss reads `bad subcommand "…": must be configure, create, or exists`.
pub(super) fn ns_ensemble(vm: &mut Vm, selector: &Value, rest: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        if !rest.is_empty() {
            let original = match original_names.read(vm, selector) {
                Ok(original) => original,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            if let Some(helper) = vm
                .native_invocation_dialect()
                .native_namespace_scripted_helper(&original)
            {
                let target = Value::from_native_string_bytes(helper);
                return vm.invoke_command_value_at(
                    vm.current_ns_id(),
                    &target,
                    rest,
                    &[],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                );
            }
        }
        if let Some(protocol) = vm
            .native_invocation_dialect()
            .native_ensemble_configuration_protocol()
        {
            let lifecycle = match vm.native_namespace_name_token(vm.current_ns_id()) {
                Ok(token) => token.lifecycle(),
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            if !protocol.permits_namespace_lifecycle(lifecycle) {
                return crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::with_error_code_bytes(
                        b"tried to manipulate ensemble of deleted namespace".to_vec(),
                        b"TCL ENSEMBLE DEAD".to_vec(),
                    ),
                );
            }
        }

        let Some((op, args)) = rest.split_first() else {
            return crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"namespace ensemble subcommand ?arg ...?\"",
            );
        };
        let index = match vm.native_static_option_index_checked(
            op,
            tcl_cmd_core::ensemble::SUBCOMMANDS.names(),
            false,
            "subcommand",
        ) {
            Ok(index) => index,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        if let Err(error) = original_names.check(vm) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        match index {
            // configure
            0 => ns_ensemble_configure(vm, args),
            // create
            1 => ns_ensemble_create(vm, args),
            // exists
            _ => match args {
                [cmd] => {
                    let bytes = match original_names.read(vm, cmd) {
                        Ok(bytes) => bytes,
                        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
                    };
                    match vm.lookup_command_bytes_checked(vm.current_ns_id(), &bytes) {
                        Ok(command) => ok(Value::bool(matches!(
                            command,
                            Some((_, crate::command::Command::Ensemble(_)))
                        ))),
                        Err(error) => vm.refuse_host_command(error.to_string()),
                    }
                }
                _ => crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"namespace ensemble exists cmdname\"",
                ),
            },
        }
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

/// The mutable half of an ensemble definition — the options `create` and
/// `configure` share. `-command` is create-only and `-namespace` is
/// configure-read-only, so neither lives here.
struct EnsembleOptions {
    pub(super) map: Vec<(
        tcl_core_types::NameBytes,
        Vec<Option<tcl_core_types::NameBytes>>,
    )>,
    pub(super) originals: crate::command::native_ensemble_objects::NativeEnsembleObjects,
    pub(super) subcommands: Option<Vec<tcl_core_types::NameBytes>>,
    prefixes: bool,
    parameters: Vec<tcl_core_types::NameBytes>,
    unknown: Option<Vec<tcl_core_types::NameBytes>>,
}

impl EnsembleOptions {
    fn from_def(def: &crate::command::EnsembleDef) -> Self {
        Self {
            map: def.map.clone(),
            originals: def.originals.clone(),
            subcommands: def.subcommands.clone(),
            prefixes: def.prefixes,
            parameters: def.parameters.clone(),
            unknown: def.unknown.clone(),
        }
    }
}

fn ensemble_original_list_member(
    vm: &mut Vm,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    elements: &crate::NativeListItems,
    index: usize,
) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
    let value = elements.elements()?.get(index).ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original ensemble List member index",
        ),
    )?;
    let bytes = original_names.read(vm, value)?;
    elements.elements()?;
    Ok(bytes)
}

fn ensemble_original_list_words(
    vm: &mut Vm,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
    elements: &crate::NativeListItems,
) -> Result<Vec<tcl_core_types::NameBytes>, tcl_cmd_core::CmdError> {
    (0..elements.elements()?.len())
        .map(|index| {
            ensemble_original_list_member(vm, original_names, elements, index)
                .map(|bytes| tcl_core_types::NameBytes::from(bytes.as_ref()))
        })
        .collect()
}

/// Apply one `-option value` pair that `create` and `configure` share. The
/// caller has already resolved the option word against its own table, so this
/// only owns the value parsing (C's per-`case` bodies).
///
/// Relative `-map` targets are qualified against the current namespace, which
/// is what both C paths use: `CRT_MAP` against the ensemble's own namespace and
/// `CONF_MAP` against `TclGetCurrentNamespace(interp)` — and each is the current
/// namespace at the point its command runs.
fn apply_shared_option(
    opts: &mut EnsembleOptions,
    which: tcl_cmd_core::ensemble::SharedOption,
    val: &Value,
    vm: &mut Vm,
    creating: bool,
) -> Result<(), Completion<Value>> {
    let original_names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = (|| {
        use crate::command::native_ensemble_objects::NativeEnsembleRoot;
        use tcl_cmd_core::ensemble::{EnsembleObjectRole, SharedOption};
        let protocol = vm
            .native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or_else(|| vm.refuse_host_command("ensemble original configured objects".into()))?
            .protocol();
        match which {
            SharedOption::Map => apply_ensemble_map(opts, val, vm, creating, protocol)?,
            SharedOption::Subcommands => {
                let elems = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                let names = ensemble_original_list_words(vm, &original_names, &elems)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.subcommands = (!names.is_empty()).then_some(names);
                opts.originals.subcommands = opts
                    .subcommands
                    .as_ref()
                    .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
            SharedOption::Parameters => {
                let elems = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.parameters = ensemble_original_list_words(vm, &original_names, &elems)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.originals.parameters = (!opts.parameters.is_empty())
                    .then(|| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
            SharedOption::Prefixes => {
                opts.prefixes = tcl_syntax::value::ValueOps::as_bool(vm, val)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            }
            SharedOption::Unknown => {
                let elements = original_names
                    .list(vm, val, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                let words = ensemble_original_list_words(vm, &original_names, &elements)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                opts.unknown = (!words.is_empty()).then_some(words);
                opts.originals.unknown = opts
                    .unknown
                    .as_ref()
                    .map(|_| EnsembleObjectRole::new(NativeEnsembleRoot::pending(val)));
            }
        }
        Ok(())
    })();
    original_names
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    reached
}

fn apply_ensemble_map(
    opts: &mut EnsembleOptions,
    val: &Value,
    vm: &mut Vm,
    creating: bool,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<(), Completion<Value>> {
    let original_names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = (|| {
        use crate::command::native_ensemble_objects::NativeEnsembleRoot;
        use tcl_cmd_core::ensemble::EnsembleObjectRole;
        let mut search = original_names
            .dictionary_search(vm, val, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        let mut patched = None;
        let mut map = Vec::new();
        while let Some((key, value)) = search
            .next_original_pair_checked(&mut || original_names.current_for_value(vm, val))
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
        {
            let elements = original_names
                .list(vm, &value, protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            if elements
                .elements()
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
                .is_empty()
            {
                return Err(err_with_code(
                    "ensemble subcommand implementations must be non-empty lists",
                    "TCL ENSEMBLE EMPTY_TARGET",
                ));
            }
            let original = ensemble_original_list_member(vm, &original_names, &elements, 0)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let mut prefix = None;
            let selected = vm
                .native_ensemble_map_prefix_input(
                    tcl_runtime_api::Namespaces::current(vm),
                    &original,
                )
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            if let std::borrow::Cow::Owned(qualified) = selected {
                let mut words = elements
                    .elements()
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
                    .to_vec();
                words[0] = Value::from_native_string_bytes(qualified);
                let copy = if creating {
                    Value::native_list_constructor(
                        elements
                            .elements()
                            .map_err(|error| {
                                crate::command::completion_from_cmd_error(vm, error.into())
                            })?
                            .to_vec(),
                        protocol,
                    )
                } else {
                    value.duplicate_native_object_in(protocol)
                };
                let replacement = Value::native_list_replace_elements(&copy, &words, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
                let root = patched.get_or_insert_with(|| val.duplicate_native_object_in(protocol));
                let updated = root
                    .native_dictionary_set_member(key.clone(), replacement.clone(), protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
                *root = updated;
                prefix = Some(replacement);
            }
            let name = original_names
                .read(vm, &key)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let words = if let Some(prefix) = prefix {
                original_names
                    .list(vm, &prefix, protocol)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
            } else {
                elements
            };
            let words = ensemble_original_list_words(vm, &original_names, &words)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
                .into_iter()
                .map(Some)
                .collect();
            map.push((tcl_core_types::NameBytes::from(name.as_ref()), words));
        }
        drop(search);
        opts.map = map;
        opts.originals.map = (!opts.map.is_empty()).then(|| {
            EnsembleObjectRole::new(patched.map_or_else(
                || NativeEnsembleRoot::pending(val),
                NativeEnsembleRoot::owned,
            ))
        });
        Ok(())
    })();
    original_names
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    reached
}

fn ns_ensemble_create_entered(
    vm: &mut Vm,
    args: &[Value],
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
    use crate::command::EnsembleDef;
    let Some(options) = vm
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return vm.refuse_host_command("ensemble configuration options".into());
    };
    if !args.len().is_multiple_of(2) {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"namespace ensemble create ?option value ...?\"",
        );
    }
    let ns_id = tcl_runtime_api::Namespaces::current(vm);
    let mut command: Option<std::rc::Rc<[u8]>> = None;
    let mut opts = EnsembleOptions {
        map: Vec::new(),
        originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(),
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: None,
    };
    for pair in args.as_chunks::<2>().0 {
        let resolved = match vm.native_static_option_index_checked(
            &pair[0],
            options.create_options(),
            false,
            "option",
        ) {
            Ok(index) => options.create_option(index),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Some(shared) = resolved.shared() else {
            // `-command` names the command rather than configuring it.
            command = Some(match original_names.read(vm, &pair[1]) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            });
            continue;
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, true) {
            return completion;
        }
    }
    // The default command is the namespace itself; an explicit -command binds in
    // the current namespace when unqualified.
    let selected_current = vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble creation namespace",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let slot = match vm.native_namespace_ensemble_publication_slot(command.as_deref()) {
        Ok(slot) => slot,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let reporting =
        tcl_syntax::naming::root_unrooted_key_bytes(&vm.command_slot_display_bytes(&slot));
    let def = EnsembleDef {
        originals: opts.originals,
        native: None,
        namespace: ns_id,
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    };
    let token = std::rc::Rc::new(
        tcl_cmd_core::ensemble::EnsembleToken::with_configuration_retirement(
            def,
            tcl_core_types::NameBytes::from(reporting.as_slice()),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
            crate::command::native_ensemble_objects::retire_configuration,
        ),
    );
    let selected_current = vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble holder before publication",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    vm.register_namespace_ensemble_slot(slot, &token);
    if let Err(error) = original_names.check(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let selected_current = !token.is_deleted() && vm.native_namespace_token_has_owner(ns_id);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble before object-role adoption",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    token.config().originals.activate();
    ok(Value::from_native_string_bytes(reporting))
}

pub(super) fn ns_ensemble_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = ns_ensemble_create_entered(vm, args, &original_names);
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

type NamespaceEnsembleToken = std::rc::Rc<
    tcl_cmd_core::ensemble::EnsembleToken<crate::command::EnsembleDef, tcl_core_types::NameBytes>,
>;

fn lookup_namespace_ensemble_token(
    vm: &mut Vm,
    written: &[u8],
) -> Result<NamespaceEnsembleToken, Completion<Value>> {
    let selected =
        match vm.lookup_command_bytes_checked(tcl_runtime_api::Namespaces::current(vm), written) {
            Ok(selected) => selected,
            Err(error) => return Err(vm.refuse_host_command(error.to_string())),
        };
    match selected {
        Some((_, crate::command::Command::Ensemble(token))) => Ok(token),
        Some(_) => Err(namespace_command_lookup_error(
            vm,
            written,
            b"",
            b" is not an ensemble command",
            b"ENSEMBLE",
        )),
        None => Err(namespace_command_lookup_error(
            vm,
            written,
            b"unknown command ",
            b"",
            b"COMMAND",
        )),
    }
}

fn report_namespace_ensemble_configuration(
    vm: &mut Vm,
    token: &NamespaceEnsembleToken,
    def: &crate::command::EnsembleDef,
    rest: &[Value],
    options: tcl_native_model::native_ensemble::NativeEnsembleConfigurationProtocol,
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Option<Completion<Value>> {
    if rest.is_empty() {
        return Some(ensemble_configuration_snapshot(vm, def, options));
    }
    if let [only] = rest {
        return Some(
            match vm.native_static_option_index_checked(
                only,
                options.configure_options(),
                false,
                "option",
            ) {
                Ok(index) => {
                    let selected_current =
                        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
                    if let Err(error) = original_names.require(
                        vm,
                        selected_current,
                        "selected original ensemble report target",
                    ) {
                        return Some(crate::command::completion_from_cmd_error(vm, error));
                    }
                    match ensemble_option_value(
                        vm,
                        def,
                        options.configure_option(index),
                        NativeEnsembleNamespaceResultStage::Option,
                    ) {
                        Ok(value) => ok(value),
                        Err(error) => crate::command::completion_from_cmd_error(vm, error),
                    }
                }
                Err(error) => crate::command::completion_from_cmd_error(vm, error),
            },
        );
    }
    None
}

fn ns_ensemble_configure_entered(
    vm: &mut Vm,
    args: &[Value],
    original_names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
    const USAGE: &str = "wrong # args: should be \"namespace ensemble configure cmdname ?-option value ...? ?arg ...?\"";
    let Some(options) = vm
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return vm.refuse_host_command("ensemble configuration options".into());
    };
    let Some((cmd_val, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    };
    // C's arity gate: one bare option word is a read, anything else must be
    // `-option value` pairs.
    if rest.len() > 1 && !rest.len().is_multiple_of(2) {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    }
    let written = match original_names.read(vm, cmd_val) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    // `Tcl_FindEnsemble` reports the two failures separately: a name that
    // resolves to no command at all is `unknown command "x"` (the
    // `Tcl_FindCommand` miss), while a name that *is* a command but carries a
    // different implementation is `"x" is not an ensemble command`. Collapsing
    // them into the second message misreports a plain typo.
    let token = match lookup_namespace_ensemble_token(vm, &written) {
        Ok(token) => token,
        Err(error) => return error,
    };
    let selected_current = !token.is_deleted();
    if let Err(error) =
        original_names.require(vm, selected_current, "selected original ensemble token")
    {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let def = token.config();
    if let Some(report) =
        report_namespace_ensemble_configuration(vm, &token, &def, rest, options, original_names)
    {
        return report;
    }
    let mut opts = EnsembleOptions::from_def(&def);
    // `apply_shared_option` qualifies relative `-map` targets against `vm`'s
    // current namespace, which is what CONF_MAP wants here: it uses
    // `TclGetCurrentNamespace(interp)` — the namespace current at the
    // `configure` call, NOT the ensemble's own namespace (which CRT_MAP uses
    // at create time). They coincide in the common
    // `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    for pair in rest.as_chunks::<2>().0 {
        let resolved = match vm.native_static_option_index_checked(
            &pair[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => options.configure_option(index),
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let selected_current =
            !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
        if let Err(error) = original_names.require(
            vm,
            selected_current,
            "selected original ensemble target after option getter",
        ) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let Some(shared) = resolved.shared() else {
            return err_with_code("option -namespace is read-only", "TCL ENSEMBLE READ_ONLY");
        };
        if let Err(completion) = apply_shared_option(&mut opts, shared, &pair[1], vm, false) {
            return completion;
        }
    }
    let selected_current =
        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble target before adoption",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    opts.originals.activate();
    token.configure(crate::command::EnsembleDef {
        originals: opts.originals,
        native: def.native,
        namespace: def.namespace,
        map: opts.map,
        subcommands: opts.subcommands,
        prefixes: opts.prefixes,
        parameters: opts.parameters,
        unknown: opts.unknown,
    });
    let selected_current =
        !token.is_deleted() && vm.native_namespace_token_has_owner(def.namespace);
    if let Err(error) = original_names.require(
        vm,
        selected_current,
        "selected original ensemble after configuration retirement",
    ) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    vm.note_ensemble_configuration_changed(&token);
    ok(Value::empty())
}

/// `namespace ensemble configure cmdname ?-option? ?value ...?`
/// (`tclEnsemble.c:377-630`): with no options an ordered key/value List of settings, with a
/// single option that option's value, otherwise `-option value` updates.
/// `ensembleConfigOptions` differs from the create table — it carries
/// `-namespace` (readable, never writable) and has no `-command`.
pub(super) fn ns_ensemble_configure(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = ns_ensemble_configure_entered(vm, args, &original_names);
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

fn ensemble_configuration_snapshot(
    vm: &mut Vm,
    def: &crate::command::EnsembleDef,
    options: tcl_native_model::native_ensemble::NativeEnsembleConfigurationProtocol,
) -> Completion<Value> {
    let Some(recipe) = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
    else {
        return vm.refuse_host_command("ensemble configuration List recipe".into());
    };
    let mut pairs = Vec::new();
    for option in options.configuration_options() {
        let value = match ensemble_option_value(
            vm,
            def,
            option,
            NativeEnsembleNamespaceResultStage::Snapshot,
        ) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        pairs.push(Value::string(option.name()));
        pairs.push(value);
    }
    ok(Value::native_list_constructor(pairs, recipe.protocol()))
}

/// One `namespace ensemble configure` option's value.
pub(super) fn ensemble_option_value(
    vm: &mut Vm,
    def: &crate::command::EnsembleDef,
    option: tcl_cmd_core::ensemble::ConfigOption,
    stage: NativeEnsembleNamespaceResultStage,
) -> Result<Value, tcl_cmd_core::CmdError> {
    use tcl_cmd_core::ensemble::ConfigOption;
    let role = match option {
        ConfigOption::Map => def.originals.map.as_ref(),
        ConfigOption::Unknown => def.originals.unknown.as_ref(),
        ConfigOption::Subcommands => def.originals.subcommands.as_ref(),
        ConfigOption::Parameters => def.originals.parameters.as_ref(),
        ConfigOption::Namespace | ConfigOption::Prefixes => None,
    };
    if let Some(original) =
        role.and_then(|role| role.inspect(|root| root.inspect(Value::clone)).flatten())
    {
        return Ok(original);
    }
    let recipe = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configuration original values",
        ))?;
    let protocol = recipe.protocol();
    let list = |words: &[tcl_core_types::NameBytes]| {
        Value::native_list_constructor(
            words
                .iter()
                .map(|word| Value::from_native_string_bytes(word.as_bytes()))
                .collect(),
            protocol,
        )
    };
    Ok(match option {
        ConfigOption::Namespace => vm.original_ensemble_namespace_result(def.namespace, stage)?,
        ConfigOption::Prefixes => Value::bool(def.prefixes),
        ConfigOption::Parameters => {
            if def.parameters.is_empty() {
                Value::empty()
            } else {
                list(&def.parameters)
            }
        }
        ConfigOption::Unknown => def.unknown.as_deref().map_or_else(Value::empty, list),
        ConfigOption::Subcommands => def.subcommands.as_deref().map_or_else(Value::empty, list),
        ConfigOption::Map if def.map.is_empty() => Value::empty(),
        ConfigOption::Map => {
            let pairs = def
                .map
                .iter()
                .map(|(key, words)| {
                    let words = words.iter().cloned().collect::<Option<Vec<_>>>().ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "ensemble original target contents",
                        ),
                    )?;
                    Ok((
                        Value::from_native_string_bytes(key.as_bytes()),
                        list(&words),
                    ))
                })
                .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
            Value::native_dictionary_constructor(pairs, None, protocol)?
        }
    })
}
