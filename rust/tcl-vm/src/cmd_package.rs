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

//! The `package` command — provided versions plus the standard-library
//! discovery protocol (`unknown` and `ifneeded`).

use std::cmp::Ordering;
use tcl_core_types::NameBytes;

use tcl_dialect::{
    PackagePrefer, compare_versions_bytes_for as cmp_version, select_package_version_bytes_for,
    select_package_version_exact_bytes_for, validate_requirement_bytes_for,
    validate_version_bytes_for, version_matches_exact_bytes_for,
    version_satisfies_bytes_for as vsatisfies,
};
use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    provide_core_packages(vm);
    vm.register_stock_builtin("package", cmd_package);
}

fn name_key(vm: &mut Vm, operand: &Value) -> Result<NameBytes, Completion<Value>> {
    let Some(protocol) = (if vm.package_table_is_authored() {
        Some(tcl_syntax::naming::NativeNameProtocol::C(
            tcl_dialect::TclVersion::V8_4,
        ))
    } else {
        vm.actual_native_invocation_dialect().native_name_protocol()
    }) else {
        return Err(vm.refuse_host_command("package name policy is unavailable".to_owned()));
    };
    let bytes = package_operand_bytes(vm, operand)?;
    Ok(NameBytes::from(protocol.package_key(&bytes).selected()))
}

fn package_protocol(
    vm: &mut Vm,
) -> Result<tcl_registry::native_package::NativePackageProtocol, Completion<Value>> {
    vm.selected_package_protocol()
        .ok_or_else(|| vm.refuse_host_command("package object protocol is unavailable".to_owned()))
}

fn package_operand_bytes(
    vm: &mut Vm,
    value: &Value,
) -> Result<std::rc::Rc<[u8]>, Completion<Value>> {
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return Err(vm.refuse_host_command("package string protocol is unavailable".to_owned()));
    };
    value.native_string_bytes(protocol).map_err(|error| {
        vm.refuse_host_command(format!(
            "package operand materialisation is unavailable: {error:?}"
        ))
    })
}

fn package_word(vm: &mut Vm, value: &Value) -> Result<NameBytes, Completion<Value>> {
    let protocol = package_protocol(vm)?;
    let bytes = package_operand_bytes(vm, value)?;
    Ok(NameBytes::from(protocol.c_word(&bytes)))
}

fn join_requirements(requirements: &[NameBytes]) -> Vec<u8> {
    let mut result = Vec::new();
    for (index, requirement) in requirements.iter().enumerate() {
        if index != 0 {
            result.push(b' ');
        }
        result.extend_from_slice(requirement.as_bytes());
    }
    result
}

fn package_wrong_args(vm: &mut Vm, member: &str) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    let Some(suffix) = protocol.member_usage(member) else {
        return vm.refuse_host_command("package member arity protocol is unavailable".into());
    };
    let original = vm
        .native_invocation
        .arguments
        .as_ref()
        .map(crate::NativeListItems::lifetime_view);
    let authored_words = original
        .as_ref()
        .filter(|_| vm.package_table_is_authored())
        .and_then(|words| {
            words.first().map(|head| {
                vec![
                    head.native_lifetime_lease().into_value(),
                    Value::string(member),
                ]
            })
        });
    let words = authored_words
        .as_deref()
        .or_else(|| original.as_ref().map(|words| &words[..words.len().min(2)]));
    let mut usage = if let Some(words) = words {
        match vm.native_argument_usage_header(words) {
            Ok(bytes) => bytes,
            Err(completion) => return completion,
        }
    } else {
        [b"package ".as_slice(), member.as_bytes()].concat()
    };
    if !suffix.is_empty() {
        usage.push(b' ');
        usage.extend_from_slice(suffix.as_bytes());
    }
    crate::command::native_wrong_args_bytes(vm, &usage)
}

fn package_list(vm: &mut Vm, members: Vec<Value>) -> Completion<Value> {
    let Some(strings) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return vm.refuse_host_command("package List result producer is unavailable".into());
    };
    ok(Value::native_list_constructor(members, strings))
}

fn package_error(parts: &[&[u8]], code: &[u8], vm: &mut Vm) -> Completion<Value> {
    crate::command::completion_from_cmd_error(
        vm,
        tcl_cmd_core::CmdError::with_error_code_bytes(parts.concat(), code),
    )
}

/// Pre-provide the core's own package entries for the release this VM is
/// pinned to, replacing whatever a previous pin left behind.
///
/// Which names exist and what version each carries is release data
/// ([`tcl_dialect::TclVersion::core_provided_packages`]), not an engine
/// constant: 8.x provides `Tcl` alone while 9.x co-provides the lowercase
/// `tcl` that `tm.tcl`'s version split reads, 8.4 provides the
/// two-component `8.4` rather than a patch level, and `TclOO` arrives at
/// 8.6 one minor version behind 9.x's.
///
/// Called again on every profile pin, so flipping 9.0 → 8.6 withdraws the
/// 9-only names instead of leaving them provided under an 8.x surface.
pub(crate) fn provide_core_packages(vm: &mut Vm) {
    // Withdraw every name *any* release pre-provides before installing this
    // one's. Derived from the same table rather than hand-listed, so a name
    // added to one release's row cannot be left behind by a re-pin.
    for version in tcl_dialect::TclVersion::ALL {
        for core in version.core_provided_packages() {
            vm.forget_package(core.name);
        }
    }
    if let Some(tcl_registry::native_package::NativePackageProtocol::C(release)) = vm
        .actual_native_invocation_dialect()
        .native_package_protocol()
    {
        for core in release.core_provided_packages() {
            vm.provide_package(core.name, core.version);
        }
    }
}

fn cmd_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    vm.with_package_table(false, |vm| dispatch_package(vm, args))
}

pub(crate) fn cmd_authored_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm.authored_package_provider().is_none() {
        return vm.refuse_host_command("authored package provider is unavailable".into());
    }
    let Some((_original_head, operands)) = args.split_first() else {
        return vm
            .refuse_host_command("authored package original invocation is unavailable".into());
    };
    let view = crate::NativeListItems::invocation_view(std::rc::Rc::new(
        args.iter()
            .map(|word| word.native_lifetime_lease().into_value())
            .collect(),
    ));
    let previous = vm.native_invocation.arguments.replace(view);
    let authored = vm.active_native_profile.is_none();
    let completion = vm.with_package_table(authored, |vm| dispatch_package(vm, operands));
    vm.native_invocation.arguments = previous;
    completion
}

fn package_subcommand(
    vm: &mut Vm,
    original: &Value,
    protocol: tcl_registry::native_package::NativePackageProtocol,
    options: &'static [&'static str],
) -> Result<&'static str, Completion<Value>> {
    let table =
        tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(options);
    let selection = if vm.package_table_is_authored() {
        let bytes = package_operand_bytes(vm, original)?;
        Ok(tcl_registry::native_package::select_authored_keyword(
            &bytes, options,
        ))
    } else {
        vm.native_index_from_original(original, &table, false, "option")
    };
    match selection {
        Ok(Ok(index)) => Ok(options[index]),
        Ok(Err(message)) => {
            let word = package_word(vm, original)?;
            Err(package_error(
                &[message.as_slice()],
                &protocol
                    .index_error_code(b"option", word.as_bytes())
                    .expect("C package index metadata"),
                vm,
            ))
        }
        Err(error) => Err(crate::command::completion_from_cmd_error(vm, error.into())),
    }
}

fn dispatch_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_args(vm, protocol.command_usage());
    };
    let tcl_registry::native_package::NativePackageProtocol::C(release) = protocol else {
        return jim_package(vm, protocol, sub, rest);
    };
    let options = protocol.c_members().expect("selected C package table");
    let sub = match package_subcommand(vm, sub, protocol, options) {
        Ok(sub) => sub,
        Err(completion) => return completion,
    };
    match sub {
        "provide" => pkg_provide(vm, rest, release),
        "require" => pkg_require(vm, rest, true, release),
        "present" => pkg_require(vm, rest, false, release),
        "vsatisfies" => pkg_vsatisfies(vm, rest, release),
        "vcompare" => pkg_vcompare(vm, rest, release),
        "names" => {
            if !rest.is_empty() {
                return package_wrong_args(vm, "names");
            }
            let names = match vm.package_names() {
                Ok(names) => names,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            };
            package_list(
                vm,
                names
                    .into_iter()
                    .map(|name| Value::from_native_string_bytes(name.as_bytes()))
                    .collect(),
            )
        }
        "versions" => match rest {
            [name] => {
                let name = match name_key(vm, name) {
                    Ok(name) => name,
                    Err(completion) => return completion,
                };
                let versions = vm.package_ifneeded_versions_bytes(&name);
                package_list(
                    vm,
                    versions
                        .into_iter()
                        .map(|version| {
                            Value::from_native_string_bytes(protocol.c_word(version.as_bytes()))
                        })
                        .collect(),
                )
            }
            _ => package_wrong_args(vm, "versions"),
        },
        "ifneeded" => pkg_ifneeded(vm, rest, release),
        "unknown" => pkg_unknown(vm, rest),
        "forget" => {
            for name in rest {
                let name = match name_key(vm, name) {
                    Ok(name) => name,
                    Err(completion) => return completion,
                };
                vm.forget_package_completely(&name);
            }
            ok(Value::empty())
        }
        "prefer" => pkg_prefer(vm, rest),
        "files" => match rest {
            [_] if !vm.package_file_inventory_active() => ok(Value::empty()),
            [name] => match name_key(vm, name) {
                Ok(name) => ok(vm.package_file_object(&name)),
                Err(completion) => completion,
            },
            _ => package_wrong_args(vm, "files"),
        },
        // Unreachable: every name in the release's table has an arm above.
        other => err(format!(
            "bad option \"{other}\": must be {}",
            tcl_cmd_core::prefix::choice_list(options)
        )),
    }
}

fn jim_package(
    vm: &mut Vm,
    protocol: tcl_registry::native_package::NativePackageProtocol,
    sub: &Value,
    rest: &[Value],
) -> Completion<Value> {
    let bytes = match package_operand_bytes(vm, sub) {
        Ok(bytes) => bytes,
        Err(completion) => return completion,
    };
    let help_target = if bytes.as_ref() == b"-help" {
        match rest
            .first()
            .map(|target| package_operand_bytes(vm, target))
            .transpose()
        {
            Ok(target) => target,
            Err(completion) => return completion,
        }
    } else {
        None
    };
    let member = match protocol.jim_dispatch(&bytes, help_target.as_deref()) {
        Some(tcl_registry::native_package::PackageDispatch::Invoke(member)) => member,
        Some(tcl_registry::native_package::PackageDispatch::Result(bytes)) => {
            return ok(Value::from_native_string_bytes(bytes));
        }
        Some(tcl_registry::native_package::PackageDispatch::Error(message)) => {
            return package_error(&[message.as_slice()], b"NONE", vm);
        }
        None => {
            return vm.refuse_host_command("Jim package member protocol is unavailable".to_owned());
        }
    };
    match member {
        "provide" if (1..=2).contains(&rest.len()) => {
            let name = match name_key(vm, &rest[0]) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
            if vm
                .package_version_bytes(&name)
                .is_some_and(|version| !version.is_empty())
            {
                return package_error(
                    &[b"package \"", name.as_bytes(), b"\" was already provided"],
                    b"NONE",
                    vm,
                );
            }
            vm.provide_package(&name, "1.0");
            ok(Value::empty())
        }
        "require" if (1..=2).contains(&rest.len()) => {
            let name = match name_key(vm, &rest[0]) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
            jim_require(vm, &name)
        }
        "forget" if !rest.is_empty() => {
            for name in rest {
                let name = match name_key(vm, name) {
                    Ok(name) => name,
                    Err(completion) => return completion,
                };
                vm.forget_package_completely(&name);
            }
            ok(Value::empty())
        }
        "names" | "list" if rest.is_empty() => {
            let names = match vm.package_names() {
                Ok(names) => names,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            };
            package_list(
                vm,
                names
                    .into_iter()
                    .map(|name| Value::from_native_string_bytes(name.as_bytes()))
                    .collect(),
            )
        }
        "provide" | "require" => package_wrong_args(vm, member),
        "forget" => package_wrong_args(vm, "forget"),
        "names" | "list" => package_wrong_args(vm, "names"),
        other => err(format!("unhandled selected Jim package member {other}")),
    }
}

fn jim_require(vm: &mut Vm, name: &NameBytes) -> Completion<Value> {
    if let Some(version) = vm.package_version_bytes(name) {
        return ok(Value::from_native_string_bytes(version.as_bytes()));
    }
    let paths = match vm.read_var_traced("::auto_path") {
        Ok(Some(paths)) => paths,
        Ok(None) => return package_error(&[b"Can't load package ", name.as_bytes()], b"NONE", vm),
        Err(completion) => return completion,
    };
    let directories = match tcl_syntax::value::ValueOps::list_elements(vm, &paths) {
        Ok(paths) => paths,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    for directory in &directories {
        let directory = match package_operand_bytes(vm, directory) {
            Ok(bytes) => bytes,
            Err(completion) => return completion,
        };
        for candidate in
            tcl_dialect::PackageProtocol::Jim.direct_files_bytes(&directory, name.as_bytes(), true)
        {
            let contents = match vm
                .host()
                .filesystem()
                .map(|fs| fs.read_bytes(&candidate.path))
            {
                Some(Ok(contents)) => contents,
                None | Some(Err(tcl_platform::HostError::Unsupported)) => {
                    return vm
                        .refuse_host_command("native package filesystem is unavailable".into());
                }
                Some(Err(_)) => continue,
            };
            vm.provide_package(name, "");
            let completion = match candidate.kind {
                tcl_dialect::DirectPackageFileKind::Script => {
                    let label = std::str::from_utf8(&candidate.path).ok();
                    if let Some(label) = label {
                        vm.push_script(label.to_owned());
                    }
                    let completion =
                        vm.eval_value_at_level(0, &Value::from_native_string_bytes(contents));
                    if label.is_some() {
                        vm.pop_script();
                    }
                    vm.settle_package_source_completion(completion)
                }
                tcl_dialect::DirectPackageFileKind::Native => {
                    let mut script = b"load ".to_vec();
                    tcl_syntax::list::append_list_element(&mut script, &candidate.path, true);
                    vm.eval_value_at_level(0, &Value::from_native_string_bytes(script))
                }
            };
            if !completion.code.is_ok() {
                vm.forget_package_completely(name);
                if let Some(refused) = vm.refused_completion() {
                    return refused;
                }
                let prior = match package_operand_bytes(vm, &completion.result) {
                    Ok(bytes) => bytes,
                    Err(refused) => return refused,
                };
                let message = [
                    prior.as_ref(),
                    if prior.is_empty() { b"" } else { b"\n" },
                    b"Can't load package ",
                    name.as_bytes(),
                ]
                .concat();
                return Completion::new(
                    completion.code,
                    Value::from_native_string_bytes(message),
                    completion.options,
                );
            }
            if vm
                .package_version_bytes(name)
                .is_none_or(|version| version.is_empty())
            {
                vm.provide_package(name, "1.0");
            }
            return ok(vm.package_version_object(name));
        }
    }
    package_error(&[b"Can't load package ", name.as_bytes()], b"NONE", vm)
}

fn pkg_provide(vm: &mut Vm, rest: &[Value], release: tcl_dialect::TclVersion) -> Completion<Value> {
    let [original_name, tail @ ..] = rest else {
        return package_wrong_args(vm, "provide");
    };
    if tail.len() > 1 {
        return package_wrong_args(vm, "provide");
    }
    let name = match name_key(vm, original_name) {
        Ok(name) => name,
        Err(completion) => return completion,
    };
    let Some(original_version) = tail.first() else {
        return ok(vm.package_version_object(&name));
    };
    let version = match package_word(vm, original_version) {
        Ok(version) => version,
        Err(completion) => return completion,
    };
    if !validate_version_bytes_for(version.as_bytes(), release) {
        return invalid_version(vm, &version);
    }
    if let Some(provided) = vm.package_version_bytes(&name) {
        if cmp_version(provided.as_bytes(), version.as_bytes(), release) != Ordering::Equal {
            let message = [
                b"conflicting versions provided for package \"".as_slice(),
                name.as_bytes(),
                b"\": ",
                provided.as_bytes(),
                b", then ",
                version.as_bytes(),
            ]
            .concat();
            let protocol = package_protocol(vm).expect("selected package protocol");
            return crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    message,
                    protocol.conflict_error_code().to_vec(),
                ),
            );
        }
        return ok(Value::empty());
    }
    vm.provide_package(&name, &version);
    ok(Value::empty())
}

fn pkg_vsatisfies(
    vm: &mut Vm,
    rest: &[Value],
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    let [original_version, requirements @ ..] = rest else {
        return package_wrong_args(vm, "vsatisfies");
    };
    if requirements.is_empty() || (!release.has_package_requirements() && requirements.len() != 1) {
        return package_wrong_args(vm, "vsatisfies");
    }
    let version = match package_word(vm, original_version) {
        Ok(version) => version,
        Err(completion) => return completion,
    };
    if !validate_version_bytes_for(version.as_bytes(), release) {
        return invalid_version(vm, &version);
    }
    let mut satisfied = false;
    for original in requirements {
        let requirement = match package_word(vm, original) {
            Ok(requirement) => requirement,
            Err(completion) => return completion,
        };
        if let Err(error) = validate_requirement_bytes_for(requirement.as_bytes(), release) {
            return invalid_requirement(vm, error);
        }
        satisfied |= vsatisfies(version.as_bytes(), requirement.as_bytes(), release);
    }
    ok(Value::int(i64::from(satisfied)))
}

fn pkg_vcompare(
    vm: &mut Vm,
    rest: &[Value],
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    match rest {
        [v1, v2] => {
            let v1 = match package_word(vm, v1) {
                Ok(word) => word,
                Err(completion) => return completion,
            };
            if !validate_version_bytes_for(v1.as_bytes(), release) {
                return invalid_version(vm, &v1);
            }
            let v2 = match package_word(vm, v2) {
                Ok(word) => word,
                Err(completion) => return completion,
            };
            if !tcl_dialect::validate_version_bytes_for(v2.as_bytes(), release) {
                return invalid_version(vm, &v2);
            }
            let order = cmp_version(v1.as_bytes(), v2.as_bytes(), release);
            ok(Value::int(match order {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }))
        }
        _ => package_wrong_args(vm, "vcompare"),
    }
}

fn pkg_ifneeded(
    vm: &mut Vm,
    rest: &[Value],
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    match rest {
        [name, version] => {
            let name = match name_key(vm, name) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
            let version = match package_word(vm, version) {
                Ok(word) => word,
                Err(completion) => return completion,
            };
            if !validate_version_bytes_for(version.as_bytes(), release) {
                return invalid_version(vm, &version);
            }
            ok(vm
                .package_ifneeded(&name, &version, release)
                .map_or_else(Value::empty, |script| {
                    Value::from_native_string_bytes(tcl_core_types::c_string_extent(script))
                }))
        }
        [name, version, script] => {
            let name = match name_key(vm, name) {
                Ok(name) => name,
                Err(completion) => return completion,
            };
            let version = match package_word(vm, version) {
                Ok(word) => word,
                Err(completion) => return completion,
            };
            if !validate_version_bytes_for(version.as_bytes(), release) {
                return invalid_version(vm, &version);
            }
            let version_bytes = match package_operand_bytes(vm, &rest[1]) {
                Ok(bytes) => bytes,
                Err(completion) => return completion,
            };
            let script_bytes = match package_operand_bytes(vm, script) {
                Ok(bytes) => bytes,
                Err(completion) => return completion,
            };
            vm.set_package_ifneeded(&name, &version_bytes, &script_bytes, release);
            ok(Value::empty())
        }
        _ => package_wrong_args(vm, "ifneeded"),
    }
}

fn pkg_unknown(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [] => ok(vm.package_unknown().map_or_else(Value::empty, |prefix| {
            Value::from_native_string_bytes(tcl_core_types::c_string_extent(prefix))
        })),
        [script] => {
            let bytes = match package_operand_bytes(vm, script) {
                Ok(bytes) => bytes,
                Err(completion) => return completion,
            };
            vm.set_package_unknown(
                (bytes.first().is_some_and(|first| *first != 0)).then(|| bytes.to_vec()),
            );
            ok(Value::empty())
        }
        _ => package_wrong_args(vm, "unknown"),
    }
}

/// `package prefer`'s preference word (`tclPkg.c`): C resolves it with
/// `Tcl_GetIndexFromObj(…, "preference", 0)`, so `l`/`s` abbreviate and the
/// empty word — a prefix of both entries — is `ambiguous preference ""`.
fn pkg_prefer(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [] => ok(Value::string(preference_name(vm.package_prefer()))),
        [preference] => {
            let protocol = match package_protocol(vm) {
                Ok(protocol) => protocol,
                Err(completion) => return completion,
            };
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
                    protocol.preference_members().expect("C preference table"),
                );
            let selected =
                match vm.native_index_from_original(preference, &table, false, "preference") {
                    Ok(result) => result,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(vm, error.into());
                    }
                };
            match selected {
                Ok(0) => {
                    vm.prefer_latest_packages();
                    ok(Value::string(preference_name(vm.package_prefer())))
                }
                // The preference is a monotone Tcl latch: once raised to
                // latest, asking for stable succeeds but leaves it at latest.
                Ok(_) => ok(Value::string(preference_name(vm.package_prefer()))),
                Err(message) => {
                    let protocol = match package_protocol(vm) {
                        Ok(protocol) => protocol,
                        Err(completion) => return completion,
                    };
                    let word = match package_word(vm, preference) {
                        Ok(word) => word,
                        Err(completion) => return completion,
                    };
                    let Some(code) = protocol.index_error_code(b"preference", word.as_bytes())
                    else {
                        return vm.refuse_host_command(
                            "package preference error protocol is unavailable".to_owned(),
                        );
                    };
                    package_error(&[message.as_slice()], &code, vm)
                }
            }
        }
        _ => package_wrong_args(vm, "prefer"),
    }
}

fn preference_name(preference: PackagePrefer) -> &'static str {
    match preference {
        PackagePrefer::Stable => "stable",
        PackagePrefer::Latest => "latest",
    }
}

fn pkg_require(
    vm: &mut Vm,
    rest: &[Value],
    discover: bool,
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    // `-exact NAME VERSION` is the requirement `VERSION-VERSION` — the same
    // rewrite `tclPkg.c`'s `PKG_REQUIRE` arm performs, so exactness needs no
    // second comparison rule.
    let exact = if let Some(value) = rest.first() {
        match package_word(vm, value) {
            Ok(word) => word.as_bytes() == b"-exact",
            Err(completion) => return completion,
        }
    } else {
        false
    };
    let rest = if exact { &rest[1..] } else { rest };
    let Some((name, reqs)) = rest.split_first() else {
        return package_wrong_args(vm, "require");
    };
    if (exact && reqs.len() != 1) || (!release.has_package_requirements() && reqs.len() > 1) {
        return package_wrong_args(vm, "require");
    }
    let name = match name_key(vm, name) {
        Ok(name) => name,
        Err(completion) => return completion,
    };
    let mut requested = Vec::with_capacity(reqs.len());
    for requirement in reqs {
        match package_word(vm, requirement) {
            Ok(word) => requested.push(word),
            Err(completion) => return completion,
        }
    }
    if exact {
        if !validate_version_bytes_for(requested[0].as_bytes(), release) {
            return invalid_version(vm, &requested[0]);
        }
    } else {
        for requirement in &requested {
            if let Err(error) = validate_requirement_bytes_for(requirement.as_bytes(), release) {
                return invalid_requirement(vm, error);
            }
        }
    }
    let reqs = requested.clone();
    match provided_status(vm, &name, &reqs, exact, release) {
        ProvidedStatus::Satisfies => {
            return ok(vm.package_version_object(&name));
        }
        ProvidedStatus::Conflicts(version) => {
            return version_conflict(vm, discover, &name, &version, &requested, exact);
        }
        ProvidedStatus::Absent => {}
    }
    if discover && let Some(version) = vm.package_loading_version_bytes(&name) {
        return circular_dependency(vm, &name, &version.clone(), &requested, exact);
    }
    if !discover {
        let protocol = package_protocol(vm).expect("selected package protocol");
        return package_error(
            &[b"package ", name.as_bytes(), b" is not present"],
            &protocol.failure_error_code(
                tcl_registry::native_package::NativePackageFailure::PresentMissing,
                name.as_bytes(),
            ),
            vm,
        );
    }

    vm.record_package_entry(name.as_bytes());
    // A loader already registered for a satisfying version wins before the
    // last-resort unknown callback. This is the ordinary fast path populated
    // by pkgIndex.tcl.
    if let Some(loader) = selected_loader(vm, &name, &reqs, exact, release) {
        return evaluate_loader(vm, &name, &loader);
    }

    if let Some(prefix) = vm.package_unknown().cloned()
        && let Err(completion) =
            invoke_package_unknown(vm, &prefix, &name, &requested, exact, release)
    {
        return completion;
    }

    // The callback may provide the package directly or register a suitable
    // ifneeded script. Re-check both forms, in that order, before failing.
    match provided_status(vm, &name, &reqs, exact, release) {
        ProvidedStatus::Satisfies => {
            return ok(vm.package_version_object(&name));
        }
        ProvidedStatus::Conflicts(version) => {
            return version_conflict(vm, discover, &name, &version, &requested, exact);
        }
        ProvidedStatus::Absent => {}
    }
    if let Some(loader) = selected_loader(vm, &name, &reqs, exact, release) {
        return evaluate_loader(vm, &name, &loader);
    }

    package_error(
        &[b"can't find package ", name.as_bytes()],
        &package_protocol(vm)
            .expect("selected package protocol")
            .failure_error_code(
                tcl_registry::native_package::NativePackageFailure::RequireMissing,
                name.as_bytes(),
            ),
        vm,
    )
}

enum ProvidedStatus {
    Absent,
    Satisfies,
    Conflicts(NameBytes),
}

fn invoke_package_unknown(
    vm: &mut Vm,
    prefix: &[u8],
    name: &NameBytes,
    requested: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> Result<(), Completion<Value>> {
    let mut callback = tcl_core_types::c_string_extent(prefix).to_vec();
    callback.push(b' ');
    tcl_syntax::list::append_list_element(&mut callback, name.as_bytes(), true);
    let callback_requirements = if !release.has_package_requirements() {
        vec![
            requested
                .first()
                .cloned()
                .unwrap_or_else(|| NameBytes::from(&b""[..])),
        ]
    } else if exact {
        vec![NameBytes::from(
            [requested[0].as_bytes(), b"-", requested[0].as_bytes()].concat(),
        )]
    } else if requested.is_empty() {
        vec![NameBytes::from("0-")]
    } else {
        requested.to_vec()
    };
    for requirement in &callback_requirements {
        callback.push(b' ');
        tcl_syntax::list::append_list_element(&mut callback, requirement.as_bytes(), true);
    }
    if exact && !release.has_package_requirements() {
        callback.extend_from_slice(b" -exact");
    }
    let completion = eval_package_script(vm, &Value::from_native_string_bytes(callback));
    if let Some(refused) = vm.refused_completion() {
        return Err(refused);
    }
    match completion.code {
        Code::Ok => {}
        Code::Error => {
            let message = package_operand_bytes(vm, &completion.result)?;
            append_loader_error_frame(vm, &message, None);
            return Err(completion);
        }
        _ => return Err(bad_return_code(vm, &completion, None)),
    }
    Ok(())
}

fn provided_status(
    vm: &Vm,
    name: &NameBytes,
    requirements: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> ProvidedStatus {
    let Some(version) = vm.package_version_bytes(name).cloned() else {
        return ProvidedStatus::Absent;
    };
    if requirements_satisfied(&version, requirements, exact, release) {
        ProvidedStatus::Satisfies
    } else {
        ProvidedStatus::Conflicts(version)
    }
}

fn version_conflict(
    vm: &mut Vm,
    discover: bool,
    name: &NameBytes,
    have: &NameBytes,
    requested: &[NameBytes],
    exact: bool,
) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    let code = if discover {
        protocol.conflict_error_code().to_vec()
    } else {
        protocol.failure_error_code(
            tcl_registry::native_package::NativePackageFailure::PresentMissing,
            name.as_bytes(),
        )
    };
    let need = if exact {
        [&b"exactly "[..], requested[0].as_bytes()].concat()
    } else {
        join_requirements(requested)
    };
    package_error(
        &[
            b"version conflict for package \"",
            name.as_bytes(),
            b"\": have ",
            have.as_bytes(),
            b", need ",
            &need,
        ],
        &code,
        vm,
    )
}

fn invalid_version(vm: &mut Vm, version: &NameBytes) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    let Some(code) = protocol.version_error_code(false) else {
        return vm.refuse_host_command("package version error protocol is unavailable".to_owned());
    };
    package_error(
        &[
            b"expected version number but got \"",
            version.as_bytes(),
            b"\"",
        ],
        code,
        vm,
    )
}

fn invalid_requirement(
    vm: &mut Vm,
    error: tcl_dialect::RequirementBytesValidationError<'_>,
) -> Completion<Value> {
    match error {
        tcl_dialect::RequirementBytesValidationError::InvalidVersion(version) => {
            invalid_version(vm, &NameBytes::from(version))
        }
        tcl_dialect::RequirementBytesValidationError::InvalidRange(requirement) => {
            let protocol = match package_protocol(vm) {
                Ok(protocol) => protocol,
                Err(completion) => return completion,
            };
            let Some(code) = protocol.version_error_code(true) else {
                return vm
                    .refuse_host_command("package range error protocol is unavailable".to_owned());
            };
            package_error(
                &[
                    b"expected versionMin-versionMax but got \"",
                    requirement,
                    b"\"",
                ],
                code,
                vm,
            )
        }
    }
}

fn circular_dependency(
    vm: &mut Vm,
    name: &NameBytes,
    loading_version: &NameBytes,
    requested: &[NameBytes],
    exact: bool,
) -> Completion<Value> {
    let mut required = name.as_bytes().to_vec();
    if !requested.is_empty() {
        required.extend_from_slice(if exact { b" exactly " } else { b" " });
        required.extend_from_slice(
            if exact {
                requested[0].as_bytes().to_vec()
            } else {
                join_requirements(requested)
            }
            .as_slice(),
        );
    }
    package_error(
        &[
            b"circular package dependency: attempt to provide ",
            name.as_bytes(),
            b" ",
            loading_version.as_bytes(),
            b" requires ",
            &required,
        ],
        &package_protocol(vm)
            .expect("selected package protocol")
            .failure_error_code(
                tcl_registry::native_package::NativePackageFailure::Circularity,
                name.as_bytes(),
            ),
        vm,
    )
}

struct SelectedLoader {
    version: NameBytes,
    script: Vec<u8>,
    release: tcl_dialect::TclVersion,
}

fn selected_loader(
    vm: &Vm,
    name: &NameBytes,
    requirements: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> Option<SelectedLoader> {
    let versions = vm.package_ifneeded_versions_bytes(name);
    let numeric_versions: Vec<&[u8]> = versions
        .iter()
        .map(|version| tcl_core_types::c_string_extent(version.as_bytes()))
        .collect();
    let selected = if exact {
        select_package_version_exact_bytes_for(
            &numeric_versions,
            requirements[0].as_bytes(),
            release,
        )?
    } else {
        let requirements: Vec<&[u8]> = requirements.iter().map(NameBytes::as_bytes).collect();
        select_package_version_bytes_for(
            &numeric_versions,
            &requirements,
            vm.package_prefer(),
            release,
        )?
    };
    let version = NameBytes::from(tcl_core_types::c_string_extent(
        versions[selected].as_bytes(),
    ));
    vm.package_ifneeded(name, &version, release)
        .cloned()
        .map(|script| SelectedLoader {
            version,
            script,
            release,
        })
}

/// Package discovery scripts are interpreter-global even when the require was
/// issued in a proc or namespace. Keeping the frame transition here prevents
/// the unknown and ifneeded paths from drifting apart.
fn eval_package_script(vm: &mut Vm, script: &Value) -> Completion<Value> {
    vm.eval_value_at_level(0, script)
}

fn bad_return_code(
    vm: &mut Vm,
    completion: &Completion<Value>,
    loader: Option<(&NameBytes, &NameBytes)>,
) -> Completion<Value> {
    // Only an explicit pending error return carries its trace into the newly
    // generated BADRESULT episode. Break/continue/custom traces are discarded.
    let pending_error = completion.code == Code::Return
        && crate::command::opt_get(&completion.options, "-code")
            .is_some_and(|value| value.as_int().is_ok_and(|code| code == 1));
    let carried = pending_error
        .then(|| crate::command::opt_get(&completion.options, "-errorinfo"))
        .flatten();
    vm.take_error_info();
    if let Some(info) = carried {
        let bytes = match package_operand_bytes(vm, &info) {
            Ok(bytes) => bytes,
            Err(refused) => return refused,
        };
        if !bytes.is_empty() {
            vm.seed_error_info_original(&info, &bytes);
        }
    }
    if pending_error
        && let Some(stack) = crate::command::opt_get(&completion.options, "-errorstack")
    {
        vm.seed_error_stack(&stack);
    }
    let message = match loader {
        Some((name, version)) => tcl_dialect::PackageProtocol::Tcl
            .ifneeded_completion_error_bytes(
                name.as_bytes(),
                version.as_bytes(),
                completion.code.as_int(),
            )
            .expect("non-error package loader completion"),
        None => format!("bad return code: {}", completion.code.as_int()).into_bytes(),
    };
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(refused) => return refused,
    };
    let code = protocol.failure_error_code(
        tcl_registry::native_package::NativePackageFailure::LoaderCompletion,
        loader.map_or(b"".as_slice(), |(name, _)| name.as_bytes()),
    );
    let mut failure = package_error(&[&message], &code, vm);
    append_loader_error_frame(vm, &message, loader);
    // Ancillary options retain their original member objects. The selected
    // error factory independently owns the new result and private error code.
    let Some(strings) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return vm.refuse_host_command("package return-option protocol is unavailable".into());
    };
    let pairs = match completion.options.native_object_dict_pairs(strings) {
        Ok(pairs) => pairs,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    let mut items = Vec::new();
    for (key, value) in pairs {
        let bytes = match package_operand_bytes(vm, &key) {
            Ok(bytes) => bytes,
            Err(refused) => return refused,
        };
        if pending_error
            || !matches!(
                bytes.as_ref(),
                b"-errorinfo" | b"-errorstack" | b"-errorline"
            )
        {
            items.extend([key, value]);
        }
    }
    let mut options = Value::native_list_constructor(items, strings);
    for (key, value) in [
        ("-code", Value::int(1)),
        ("-level", Value::int(0)),
        (
            "-errorcode",
            crate::command::opt_get(&failure.options, "-errorcode")
                .expect("selected package error metadata"),
        ),
    ] {
        options = crate::command::with_return_option(&options, key, value);
    }
    failure.options = options;
    failure
}

fn append_loader_error_frame(
    vm: &mut Vm,
    message: &[u8],
    loader: Option<(&NameBytes, &NameBytes)>,
) {
    let frame = match loader {
        Some((name, version)) => [
            b"\n    (\"package ifneeded ".as_slice(),
            name.as_bytes(),
            b" ",
            version.as_bytes(),
            b"\" script)",
        ]
        .concat(),
        None => b"\n    (\"package unknown\" script)".to_vec(),
    };
    vm.seed_error_info_frame(message, &frame);
}

fn evaluate_loader(vm: &mut Vm, name: &NameBytes, loader: &SelectedLoader) -> Completion<Value> {
    let source = &loader.script;

    vm.begin_package_loading(name, &loader.version);
    if let Err(error) = vm.record_package_loader_origin(name, &loader.version) {
        vm.end_package_loading(name, &loader.version);
        return crate::command::completion_from_cmd_error(vm, error.into());
    }
    let completion = eval_package_script(
        vm,
        &Value::from_native_string_bytes(tcl_core_types::c_string_extent(source)),
    );
    vm.end_package_loading(name, &loader.version);
    if let Some(refused) = vm.refused_completion() {
        return refused;
    }
    match completion.code {
        Code::Ok => {}
        Code::Error => {
            vm.forget_package(name);
            let message = match package_operand_bytes(vm, &completion.result) {
                Ok(bytes) => bytes,
                Err(refused) => return refused,
            };
            append_loader_error_frame(vm, &message, Some((name, &loader.version)));
            return completion;
        }
        _ => {
            vm.forget_package(name);
            return bad_return_code(vm, &completion, Some((name, &loader.version)));
        }
    }
    match vm.package_version_bytes(name).cloned() {
        Some(provided)
            if cmp_version(
                provided.as_bytes(),
                loader.version.as_bytes(),
                loader.release,
            ) == Ordering::Equal =>
        {
            ok(vm.package_version_object(name))
        }
        Some(provided) => {
            vm.forget_package(name);
            package_error(
                &[
                    b"attempt to provide package ",
                    name.as_bytes(),
                    b" ",
                    loader.version.as_bytes(),
                    b" failed: package ",
                    name.as_bytes(),
                    b" ",
                    provided.as_bytes(),
                    b" provided instead",
                ],
                &package_protocol(vm)
                    .expect("selected package protocol")
                    .failure_error_code(
                        tcl_registry::native_package::NativePackageFailure::LoaderWrongVersion,
                        name.as_bytes(),
                    ),
                vm,
            )
        }
        None => package_error(
            &[
                b"attempt to provide package ",
                name.as_bytes(),
                b" ",
                loader.version.as_bytes(),
                b" failed: no version of package ",
                name.as_bytes(),
                b" provided",
            ],
            &package_protocol(vm)
                .expect("selected package protocol")
                .failure_error_code(
                    tcl_registry::native_package::NativePackageFailure::LoaderMissing,
                    name.as_bytes(),
                ),
            vm,
        ),
    }
}

fn requirements_satisfied(
    version: &NameBytes,
    requirements: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> bool {
    if requirements.is_empty() {
        true
    } else if exact {
        version_matches_exact_bytes_for(version.as_bytes(), requirements[0].as_bytes(), release)
    } else {
        requirements
            .iter()
            .any(|requirement| vsatisfies(version.as_bytes(), requirement.as_bytes(), release))
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::{TclVersion, version_satisfies as vsatisfies};
    use tcl_runtime_api::Code;

    use crate::interp::Vm;
    use crate::value::Value;

    fn vm() -> Vm {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::default(),
        ));
        vm
    }

    fn package(vm: &mut Vm, words: &[&str]) -> tcl_runtime_api::Completion<Value> {
        let args: Vec<Value> = words.iter().map(|word| Value::string(*word)).collect();
        super::cmd_package(vm, &args)
    }

    #[test]
    fn package_operands_preserve_native_storage_extents_and_error_bytes() {
        for release in TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_runtime_version(release);
            let resident = super::cmd_package(
                &mut vm,
                &[Value::from_native_string_bytes(&b"names\0junk"[..])],
            );
            assert_eq!(resident.code, Code::Ok, "{release:?}");
            let binary = super::cmd_package(&mut vm, &[Value::byte_array(&b"names\0junk"[..])]);
            assert_eq!(binary.code, Code::Error, "{release:?}");
            assert!(
                binary
                    .result
                    .string_bytes()
                    .windows(2)
                    .any(|bytes| bytes == b"\xc0\x80")
            );
            let raw = super::cmd_package(
                &mut vm,
                &[
                    Value::string("vcompare"),
                    Value::from_native_string_bytes(&b"\xff"[..]),
                    Value::string("1"),
                ],
            );
            assert_eq!(
                raw.result.string_bytes().as_ref(),
                b"expected version number but got \"\xff\"",
                "{release:?}"
            );
            let counted = super::cmd_package(
                &mut vm,
                &[
                    Value::string("vcompare"),
                    Value::from_native_string_bytes(&b"1\0junk"[..]),
                    Value::string("1"),
                ],
            );
            assert_eq!(counted.code, Code::Ok, "{release:?}");
            assert_eq!(counted.result.string_bytes().as_ref(), b"0");
        }
    }

    #[test]
    fn actual_package_protocol_is_independent_of_logical_release() {
        for (native, logical, expected) in [
            (TclVersion::V8_4, TclVersion::V9_1, Code::Error),
            (TclVersion::V8_6, TclVersion::V9_1, Code::Error),
            (TclVersion::V9_0, TclVersion::V8_4, Code::Ok),
            (TclVersion::V9_1, TclVersion::V8_6, Code::Ok),
        ] {
            let mut vm = Vm::new();
            assert!(vm.set_native_engine_profile(
                tcl_dialect::DialectProfile::find(native.dialect_name()).unwrap(),
            ));
            vm.set_runtime_version(logical);
            let result = super::cmd_package(
                &mut vm,
                &[
                    Value::string("vcompare"),
                    Value::from_native_string_bytes(b"1+\xff".as_slice()),
                    Value::string("1"),
                ],
            );
            assert_eq!(
                result.code, expected,
                "native={native:?}, logical={logical:?}"
            );
            if expected == Code::Ok {
                assert_eq!(result.result.string_bytes().as_ref(), b"0");
            } else {
                assert_eq!(
                    result.result.string_bytes().as_ref(),
                    b"expected version number but got \"1+\xff\""
                );
            }
        }
    }

    #[test]
    fn package_loader_entries_keep_first_spelling_and_registration_order() {
        for release in TclVersion::ALL {
            let mut vm = Vm::new();
            vm.set_runtime_version(release);
            assert_eq!(
                package(&mut vm, &["ifneeded", "p", "1.0", "OLD"]).code,
                Code::Ok
            );
            assert_eq!(
                package(&mut vm, &["ifneeded", "p", "1", "NEW"]).code,
                Code::Ok
            );
            assert_eq!(
                package(&mut vm, &["ifneeded", "p", "1.00"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"NEW"
            );
            assert_eq!(
                package(&mut vm, &["ifneeded", "p", "2", "SECOND"]).code,
                Code::Ok
            );
            assert_eq!(
                package(&mut vm, &["versions", "p"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"1.0 2"
            );
            assert_eq!(package(&mut vm, &["provide", "q", "1"]).code, Code::Ok);
            assert_eq!(
                package(&mut vm, &["versions", "q"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b""
            );
            let registered = super::cmd_package(
                &mut vm,
                &[
                    Value::string("ifneeded"),
                    Value::string("script"),
                    Value::string("1"),
                    Value::from_native_string_bytes(&b"PREFIX\0SUFFIX"[..]),
                ],
            );
            assert_eq!(registered.code, Code::Ok);
            assert_eq!(
                package(&mut vm, &["ifneeded", "script", "1"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"PREFIX"
            );
        }
    }

    #[test]
    fn package_tcl9_loader_suffixes_retain_opaque_bytes() {
        for release in [TclVersion::V9_0, TclVersion::V9_1] {
            let mut vm = Vm::new();
            vm.set_runtime_version(release);
            let registered = super::cmd_package(
                &mut vm,
                &[
                    Value::string("ifneeded"),
                    Value::string("raw"),
                    Value::from_native_string_bytes(&b"1+\xff"[..]),
                    Value::string("RAW"),
                ],
            );
            assert_eq!(registered.code, Code::Ok);
            assert_eq!(
                package(&mut vm, &["ifneeded", "raw", "1.0+different", "REPLACED"]).code,
                Code::Ok
            );
            assert_eq!(
                package(&mut vm, &["versions", "raw"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"1+\xff"
            );
            assert_eq!(
                package(&mut vm, &["ifneeded", "raw", "1+query"])
                    .result
                    .string_bytes()
                    .as_ref(),
                b"REPLACED"
            );
        }
    }

    #[test]
    fn import_redefinition_rename_reexport_and_retirement_follow_dialect() {
        let jim: &'static tcl_dialect::DialectProfile =
            Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
        let script = r"namespace eval S {proc p {} {return ONE}; namespace export *}; namespace eval D {namespace import ::S::*; namespace export *}; namespace eval E {namespace import ::D::*}; namespace eval S {proc p {} {return TWO}}; set result [list [D::p] [E::p] [namespace origin E::p]]; rename S::p S::moved; lappend result [catch {D::p} value] [catch {namespace origin E::p} value]; proc S::p {} {return THREE}; lappend result [D::p] [E::p]; rename S::moved {}; lappend result [llength [info commands D::p]] [llength [info commands E::p]]; set result";
        for profile in TclVersion::ALL
            .into_iter()
            .map(|release| {
                tcl_dialect::DialectProfile::find(release.dialect_profile_name())
                    .expect("C profile")
            })
            .chain(std::iter::once(jim))
        {
            let mut vm = vm();
            vm.set_dialect_profile(profile);
            let result = vm.eval_source(script).expect("source");
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            let expected = if profile.namespace_import_binding()
                == Some(tcl_dialect::NamespaceImportBinding::SourceName)
            {
                "TWO TWO ::S::p 1 1 THREE THREE 1 1"
            } else {
                "TWO TWO ::S::p 0 0 TWO TWO 0 0"
            };
            assert_eq!(&*result.result.to_str(), expected, "{}", profile.name);
        }
    }

    #[test]
    fn jim_import_ignores_exports_overwrites_and_retains_missing_source_alias() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"namespace eval S {proc p {} {return SOURCE}}; namespace eval D {proc p {} {return DESTINATION}; namespace import -force ::S::*; set imported [namespace import]}; rename S::p {}; set result [list [llength [info commands D::p]] [catch {D::p}] [catch {namespace origin D::p}] [namespace eval D {namespace import ::missing::*}] [namespace eval D {namespace export}]]; proc S::p {} {return REVIVED}; lappend result [D::p] [namespace origin D::p] [catch {namespace path}]; set result").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "1 1 1 {} {} REVIVED ::S::p 1");
    }

    #[test]
    fn jim_interpreter_handle_factory_alias_frame_and_deletion() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"set ::x GLOBAL; namespace eval ::N {proc observe {} {set x LOCAL; set child [interp]; $child alias readX set x; $child alias where namespace current; set result [list [$child eval {readX}] [$child eval {where}]]; $child delete; return $result}}; ::N::observe").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "LOCAL ::N");
    }

    #[test]
    fn jim_interpreter_handle_isolation() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"namespace eval ::N {proc probe {} {return PARENT}}; package provide Isolated 2.0; set child [interp]; $child eval {namespace eval ::N {proc probe {} {return CHILD}}}; set result [list [::N::probe] [$child eval {::N::probe}] [$child eval {catch {package require Isolated}}]]; $child delete; set result").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "PARENT CHILD 1");
    }

    #[test]
    fn legacy_upvar_level_presence_rejects_numeric_variable_pair() {
        for (version, expected) in [
            (TclVersion::V8_4, "1"),
            (TclVersion::V8_5, "1"),
            (TclVersion::V8_6, "0"),
            (TclVersion::V9_0, "0"),
            (TclVersion::V9_1, "0"),
        ] {
            let mut vm = vm();
            vm.set_runtime_version(version);
            let result = vm
                .eval_source(
                    "proc probe {} {upvar 1 local}; proc caller {} {set 1 3; catch probe}; caller",
                )
                .expect("source");
            assert_eq!(result.code, Code::Ok);
            assert_eq!(&*result.result.to_str(), expected, "{version:?}");
        }
    }

    #[test]
    fn upvar_qualified_scalar_and_element_use_selected_frame() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut vm = vm();
            let expected = if dialect == "jim" {
                let profile =
                    Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                        "jim",
                        &[],
                        "Jim",
                        tcl_dialect::model::DialectPoint::canonical(
                            tcl_dialect::model::Release::JIM_0_84,
                        ),
                    )));
                vm.set_dialect_profile(profile);
                "11 12 4 5"
            } else {
                vm.set_dialect_profile(
                    tcl_dialect::DialectProfile::find(dialect).expect("profile"),
                );
                "11 12 11 12"
            };
            let result = vm.eval_source(r"namespace eval N {namespace eval R {variable x 4; variable a; set a(k) 5}; proc outer {} {inner::scalar; inner::array; list $R::x $R::a(k) $::N::R::x $::N::R::a(k)}; namespace eval inner {namespace eval R {variable x 99; variable a; set a(k) 98}; proc scalar {} {upvar 1 R::x alias; set alias 11}; proc array {} {upvar 1 R::a(k) alias; set alias 12}}}; N::outer").expect("source");
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            assert_eq!(&*result.result.to_str(), expected, "{dialect}");
            let result = vm
                .eval_source(
                    "proc global_alias {} {upvar #0 ::N::R::a(k) alias; incr alias}; global_alias",
                )
                .expect("source");
            assert_eq!(result.code, Code::Ok);
            assert_eq!(
                &*result.result.to_str(),
                if dialect == "jim" { "6" } else { "13" }
            );
        }
    }

    #[test]
    fn qualified_variable_fallback_depends_on_cell_and_release() {
        for (version, expected) in [
            (TclVersion::V8_4, "11 0"),
            (TclVersion::V8_6, "11 0"),
            (TclVersion::V9_0, "9 1"),
        ] {
            let mut vm = vm();
            vm.set_runtime_version(version);
            let result = vm.eval_source("namespace eval R {variable x 9}; namespace eval N {namespace eval R {}; proc p {} {set R::x 11}}; N::p; list $::R::x [info exists ::N::R::x]").expect("source");
            assert_eq!(result.code, Code::Ok);
            assert_eq!(&*result.result.to_str(), expected, "{version:?}");
        }
    }

    #[test]
    fn jim_namespace_delete_retires_flat_variable_cells() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source("namespace eval N {variable v 4; namespace eval C {variable x 3}}; namespace delete N; list [info exists ::N::v] [info exists ::N::C::x]").expect("source");
        assert_eq!(result.code, Code::Ok);
        assert_eq!(&*result.result.to_str(), "0 0");
    }

    #[test]
    fn jim_activation_variables() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"namespace eval N {set ephemeral 5; variable v 7; list $ephemeral $v [info locals] [info vars]}; list [info exists ::N::ephemeral] [set ::N::v]").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "0 7");
    }

    #[test]
    fn jim_relative_qualified_proc_variable() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"namespace eval N {variable R::x 9; proc p {} {set R::x 10; list $R::x $::N::R::x}}; N::p").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "10 9");
    }

    #[test]
    fn jim_flat_absolute_variable_names() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"set ::Missing::x 3; set ::N:::x 4; set ::N::x 5; list $::::Missing::x $::N:::x $::N::x").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "3 4 5");
    }

    #[test]
    fn jim_namespace_root_is_local_activation() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm
            .eval_source(r"namespace eval :: {set ephemeralRoot 11}; info exists ::ephemeralRoot")
            .expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "0");
    }

    #[test]
    fn jim_upvar_targets_selected_activation() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"proc inner {} {upvar 1 R::x link; set link 12}; proc outer {} {set R::x 3; inner; set R::x}; outer").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "12");
    }

    #[test]
    fn jim_variable_declaration_preserves_colon_runs() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let result = vm.eval_source(r"namespace eval N {variable R:::v 6; incr v}; list [set ::N::R:::v] [info exists ::N::R::v]").expect("source");
        assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
        assert_eq!(&*result.result.to_str(), "7 0");
    }

    #[test]
    fn jim_direct_packages_use_global_source_and_preserve_partial_effects() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let directory =
            std::env::temp_dir().join(format!("2286-jim-package-vm-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("package directory");
        for (name, source) in [
            (
                "plain",
                "set ::loaderContext [list [namespace current] [info level]]; return done",
            ),
            (
                "recursive",
                "set ::recursiveVersion [package require recursive 999]; package provide recursive invalid",
            ),
            (
                "bad",
                "proc ::retainedSideEffect {} {return yes}; package provide bad 999; error stop",
            ),
        ] {
            std::fs::write(directory.join(format!("{name}.tcl")), source).expect("package source");
        }
        let mut vm = vm();
        vm.set_dialect_profile(profile);
        let setup = format!(
            "set ::auto_path [list {}]",
            tcl_syntax::list::list_element(&directory.to_string_lossy())
        );
        assert!(vm.eval_source(&setup).expect("setup").code.is_ok());
        assert_eq!(
            &*vm.eval_source("namespace eval Probe {proc p {} {package require plain 999}; p}")
                .expect("require")
                .result
                .to_str(),
            "1.0"
        );
        assert_eq!(
            &*vm.eval_source("set ::loaderContext")
                .expect("context")
                .result
                .to_str(),
            ":: 0"
        );
        assert_eq!(
            &*package(&mut vm, &["require", "recursive"]).result.to_str(),
            "1.0"
        );
        assert_eq!(
            &*vm.eval_source("set ::recursiveVersion")
                .expect("recursive")
                .result
                .to_str(),
            ""
        );
        assert!(!package(&mut vm, &["require", "bad"]).code.is_ok());
        assert_eq!(vm.package_version("bad"), None);
        assert_eq!(
            &*vm.eval_source("retainedSideEffect")
                .expect("retained")
                .result
                .to_str(),
            "yes"
        );
        assert!(!package(&mut vm, &["provide", "plain"]).code.is_ok());
        std::fs::remove_dir_all(directory).expect("remove fixtures");
    }

    #[test]
    fn legacy_package_arity_and_unknown_callback_follow_core_protocol() {
        let mut vm = vm();
        vm.set_runtime_version(TclVersion::V8_4);
        assert!(!package(&mut vm, &["require", "p", "1", "2"]).code.is_ok());
        assert!(
            !package(&mut vm, &["vsatisfies", "1", "1", "2"])
                .code
                .is_ok()
        );
        assert!(!package(&mut vm, &["provide", "p", "1.0a1"]).code.is_ok());
        assert!(
            vm.eval_source("proc probe {args} {set ::observed $args}; package unknown probe")
                .expect("valid callback")
                .code
                .is_ok()
        );
        assert!(
            !package(&mut vm, &["require", "-exact", "absent", "1.0"])
                .code
                .is_ok()
        );
        assert_eq!(
            vm.eval_source("set ::observed")
                .expect("callback observation")
                .result
                .to_str()
                .as_ref(),
            "absent 1.0 -exact"
        );
    }

    /// The pre-provided core packages follow the pinned release, so
    /// `package require Tcl 8.5` fails under a 9.x pin exactly as `tclsh9.0`
    /// fails it (`version conflict for package "Tcl": have 9.0.4, need 8.5`).
    /// Hardcoding `9.0.4` and providing `Tcl`+`tcl` regardless of the pin
    /// would make that require wrongly *succeed*.
    ///
    /// Measured (`package provide <name>` in a fresh `tclsh`): 8.4.20 →
    /// `Tcl` = `8.4` and no `tcl`; 8.5.19 → `Tcl` = `8.5.19`; 8.6.14 →
    /// `Tcl` = `8.6.14` with `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all four
    /// names, at the patch level and `1.3.1`.
    #[test]
    fn core_provides_follow_the_pinned_release() {
        for version in TclVersion::ALL {
            let mut vm = vm();
            vm.set_runtime_version(version);
            for core in version.core_provided_packages() {
                assert_eq!(
                    vm.package_version(core.name),
                    Some(core.version),
                    "{version:?} provides {}",
                    core.name
                );
            }
            // FN guard: a name this release does not pre-provide is absent,
            // not left over from the construction-time default pin (9.0).
            if version < TclVersion::V9_0 {
                assert_eq!(vm.package_version("tcl"), None, "{version:?}");
                assert_eq!(vm.package_version("tcl::oo"), None, "{version:?}");
            }
            if version < TclVersion::V8_6 {
                assert_eq!(vm.package_version("TclOO"), None, "{version:?}");
            }
        }
    }

    /// `package require Tcl 8.5` means `[8.5, 9)`: satisfied on 8.5 and 8.6,
    /// a version conflict on 8.4 and on every 9.x — the tclsh answers
    /// measured on all five reference interpreters.
    #[test]
    fn package_require_tcl_85_matches_tclsh_per_release() {
        for version in TclVersion::ALL {
            let mut vm = vm();
            vm.set_runtime_version(version);
            let provided = vm
                .package_version("Tcl")
                .expect("Tcl is provided")
                .to_owned();
            let satisfied = vsatisfies(&provided, "8.5");
            assert_eq!(
                satisfied,
                matches!(version, TclVersion::V8_5 | TclVersion::V8_6),
                "{version:?}: `package require Tcl 8.5` against provided {provided}"
            );
        }
    }

    #[test]
    fn version_ranges() {
        assert!(vsatisfies("9.0", "8.5-"));
        assert!(vsatisfies("9.0", "9.0-"));
        assert!(!vsatisfies("8.4", "8.5-"));
        assert!(vsatisfies("8.6", "8.5"));
        assert!(!vsatisfies("9.0", "8.5")); // 8.5 → [8.5, 9)
        assert!(vsatisfies("8.5.2", "8.5-9.0"));
        assert!(!vsatisfies("9.0", "8.5-9.0"));
    }

    #[test]
    fn malformed_requirements_precede_an_active_loader_cycle() {
        let mut vm = vm();
        vm.begin_package_loading("p", "1.0");
        let malformed = package(&mut vm, &["require", "p", "invalid"]);
        assert_eq!(malformed.code, Code::Error);
        assert_eq!(
            &*malformed.result.to_str(),
            "expected version number but got \"invalid\""
        );

        let cycle = package(&mut vm, &["require", "p", "1.0"]);
        assert_eq!(cycle.code, Code::Error);
        assert_eq!(
            &*cycle.result.to_str(),
            "circular package dependency: attempt to provide p 1.0 requires p 1.0"
        );
        vm.end_package_loading("p", "1.0");
    }

    #[test]
    fn package_commands_apply_the_pinned_plus_suffix_policy() {
        let mut tcl8 = vm();
        tcl8.set_runtime_version(TclVersion::V8_6);
        let rejected = package(&mut tcl8, &["provide", "p", "1.2+platform"]);
        assert_eq!(rejected.code, Code::Error);
        assert_eq!(
            &*rejected.result.to_str(),
            "expected version number but got \"1.2+platform\""
        );
        assert_eq!(tcl8.package_version("p"), None);
        let rejected = package(&mut tcl8, &["ifneeded", "p", "1.2+platform", ""]);
        assert_eq!(rejected.code, Code::Error);
        assert!(tcl8.package_ifneeded_versions("p").is_empty());

        let mut tcl9 = vm();
        tcl9.set_runtime_version(TclVersion::V9_0);
        let provided = package(&mut tcl9, &["provide", "p", "1.2+platform"]);
        assert_eq!(provided.code, Code::Ok);
        assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
        let compared = package(&mut tcl9, &["vcompare", "1.2+platform", "1.2"]);
        assert_eq!(compared.code, Code::Ok);
        assert_eq!(&*compared.result.to_str(), "0");
        let registered = package(&mut tcl9, &["ifneeded", "q", "1.2+platform", ""]);
        assert_eq!(registered.code, Code::Ok);
        assert_eq!(tcl9.package_ifneeded_versions("q"), vec!["1.2+platform"]);

        let mut exact_provider = vm();
        exact_provider.set_runtime_version(TclVersion::V9_0);
        assert_eq!(
            package(&mut exact_provider, &["ifneeded", "q", "1.2+platform", ""]).code,
            Code::Ok
        );
        assert_eq!(
            package(&mut exact_provider, &["ifneeded", "q", "1.3", ""]).code,
            Code::Ok
        );
        let exact_loader = super::selected_loader(
            &exact_provider,
            &tcl_core_types::NameBytes::from("q"),
            &[tcl_core_types::NameBytes::from("1.2+platform")],
            true,
            TclVersion::V9_0,
        )
        .expect("exact Tcl 9 provider selection");
        assert_eq!(exact_loader.version, "1.2+platform");

        let mut exact = vm();
        exact.set_runtime_version(TclVersion::V9_0);
        assert_eq!(package(&mut exact, &["provide", "p", "1.3"]).code, Code::Ok);
        let mismatch = package(&mut exact, &["require", "-exact", "p", "1.2+x"]);
        assert_eq!(mismatch.code, Code::Error);
        assert_eq!(
            &*mismatch.result.to_str(),
            "version conflict for package \"p\": have 1.3, need exactly 1.2+x"
        );
        let match_result = package(&mut exact, &["require", "-exact", "p", "1.3+x"]);
        assert_eq!(match_result.code, Code::Ok);
        assert_eq!(&*match_result.result.to_str(), "1.3");

        // A profile change does not rewrite package state. Lookup and
        // provider selection must nevertheless apply the new release's
        // grammar, so a Tcl 9-only provider is not selected by an 8.6 pin.
        tcl9.set_runtime_version(TclVersion::V8_6);
        assert!(
            super::selected_loader(
                &tcl9,
                &tcl_core_types::NameBytes::from("q"),
                &[],
                false,
                TclVersion::V8_6
            )
            .is_none()
        );
        let conflict = package(&mut tcl9, &["require", "p", "1.2"]);
        assert_eq!(conflict.code, Code::Error);
        assert_eq!(tcl9.package_version("p"), Some("1.2+platform"));
    }
    #[test]
    fn provided_version_headers_match_all_native_30_windows() {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        use tcl_syntax::scalar_getter::NativeScalarGetterKind;
        const FIXTURES: [&str; 5] = [
            include_str!("../../tcl-registry/tests/data/native_package_versions/8.4.20.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_versions/8.5.19.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_versions/8.6.18.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_versions/9.0.4.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_versions/9.1.0.tsv"),
        ];
        fn class(value: &Value) -> &'static str {
            match value.native_object_snapshot().cache {
                Cache::None => "none",
                Cache::Numeric(_) => "int",
                other => panic!("unexpected version primary {other:?}"),
            }
        }
        let mut compared = 0;
        for (version, fixture) in TclVersion::ALL.into_iter().zip(FIXTURES) {
            let mut vm = crate::native_fixture::core(
                tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap(),
            );
            assert_eq!(package(&mut vm, &["provide", "probe", "12"]).code, Code::Ok);
            let mut latest = package(&mut vm, &["provide", "probe"]);
            let held = latest.result.clone();
            let rows: Vec<Vec<&str>> = fixture
                .lines()
                .map(|row| row.split('\t').collect())
                .collect();
            assert_eq!(class(&held), rows[0][2]);
            assert_eq!(held.native_object_reference_count().to_string(), rows[0][3]);
            compared += 1;
            held.native_scalar_probe(vm.native_invocation_dialect(), NativeScalarGetterKind::Int)
                .unwrap()
                .unwrap();
            assert_eq!(class(&held), rows[1][3]);
            assert_eq!(rows[1][2], "12");
            compared += 1;
            for (member, row) in ["provide", "require", "present"]
                .into_iter()
                .zip(&rows[2..5])
            {
                latest = package(&mut vm, &[member, "probe"]);
                assert_eq!(latest.code, Code::Ok);
                assert_eq!(row[1], "0");
                assert_eq!(
                    usize::from(
                        latest.result.native_object_identity() == held.native_object_identity()
                    )
                    .to_string(),
                    row[2]
                );
                assert_eq!(class(&latest.result), row[3]);
                assert_eq!(held.native_object_reference_count().to_string(), row[4]);
                compared += 1;
            }
            latest = package(&mut vm, &["forget", "probe"]);
            assert_eq!(latest.code, Code::Ok);
            assert_eq!(held.native_object_reference_count().to_string(), rows[5][2]);
            assert_eq!(class(&held), rows[5][3]);
            compared += 1;
        }
        assert_eq!(compared, 30);
    }

    #[test]
    fn package_files_headers_match_all_native_10_windows_and_scope_lifetimes() {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        const FIXTURES: [&str; 2] = [
            include_str!("../../tcl-registry/tests/data/native_package_files/9.0.4.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_files/9.1.0.tsv"),
        ];
        let mut compared = 0;
        for (version, fixture) in [TclVersion::V9_0, TclVersion::V9_1]
            .into_iter()
            .zip(FIXTURES)
        {
            let mut vm = crate::native_fixture::core(
                tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap(),
            );
            vm.begin_package_loading(b"probe", b"1");
            vm.record_package_source_file(b"leaf.tcl").unwrap();
            vm.end_package_loading(b"probe", b"1");
            let rows: Vec<Vec<&str>> = fixture
                .lines()
                .map(|row| row.split('\t').collect())
                .collect();
            vm.with_package_file_object(b"probe", |original| {
                assert!(matches!(
                    original.native_object_snapshot().cache,
                    Cache::List { .. }
                ));
                assert_eq!(
                    original.native_object_reference_count().to_string(),
                    rows[0][2]
                );
            });
            assert_eq!(rows[0][3], "list");
            compared += 1;
            let mut latest = package(&mut vm, &["files", "probe"]);
            assert_eq!(latest.code, Code::Ok);
            assert_eq!(rows[1][1], "0");
            assert_eq!(
                latest.result.native_object_reference_count().to_string(),
                rows[1][3]
            );
            compared += 1;
            let held = latest.result.clone();
            assert_eq!(held.native_object_reference_count().to_string(), rows[2][3]);
            compared += 1;
            latest = package(&mut vm, &["forget", "probe"]);
            assert_eq!(latest.code, Code::Ok);
            assert_eq!(held.native_object_reference_count().to_string(), rows[3][3]);
            compared += 1;
            latest = crate::interp::ok(held.clone());
            assert_eq!(
                latest.result.native_object_identity(),
                held.native_object_identity()
            );
            assert_eq!(held.native_object_reference_count().to_string(), rows[4][3]);
            compared += 1;
            drop(latest);
            drop(held);
            vm.begin_package_loading(b"outer", b"1");
            vm.record_package_source_file(b"first.tcl").unwrap();
            let detached = vm.take_package_file_scope();
            vm.record_package_source_file(b"excluded.tcl").unwrap();
            vm.begin_package_loading(b"nested", b"1");
            vm.record_package_source_file(b"nested.tcl").unwrap();
            vm.end_package_loading(b"nested", b"1");
            vm.restore_package_file_scope(detached);
            vm.forget_package_completely(b"outer");
            vm.record_package_source_file(b"after-forget.tcl").unwrap();
            vm.end_package_loading(b"outer", b"1");
            assert!(vm.take_package_file_scope().is_empty());
            assert_eq!(
                vm.package_file_object(b"outer").string_bytes().as_ref(),
                b"after-forget.tcl"
            );
            assert_eq!(
                vm.package_file_object(b"nested").string_bytes().as_ref(),
                b"nested.tcl"
            );
        }
        assert_eq!(compared, 10);
    }

    #[test]
    fn shared_package_files_append_is_an_outer_native_fatal_condition() {
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let mut vm = crate::native_fixture::core(
                tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap(),
            );
            vm.begin_package_loading(b"probe", b"1");
            vm.record_package_source_file(b"leaf.tcl").unwrap();
            let retained = vm.package_file_object(b"probe");
            let error = vm.record_package_source_file(b"second.tcl").unwrap_err();
            assert!(matches!(
                error,
                tcl_syntax::value::ValueError::NativeFatalCondition(
                    tcl_syntax::raw_string::NativeFatalCondition::SharedPackageFileListMutation
                )
            ));
            assert_eq!(retained.string_bytes().as_ref(), b"leaf.tcl");
        }
    }

    #[test]
    fn package_error_publication_matches_all_21_original_native_controls() {
        // Native proof: naming.package.error-publication-bad-member
        // docs/design/analysis/name-resolution-proofs/package-error-publication-bad-member.md
        // Native proof: naming.package.error-publication-bad-preference
        // docs/design/analysis/name-resolution-proofs/package-error-publication-bad-preference.md
        // Native proof: naming.package.error-publication-unknown-completion
        // docs/design/analysis/name-resolution-proofs/package-error-publication-unknown-completion.md
        // Native proof: naming.package.error-publication-loader-completion
        // docs/design/analysis/name-resolution-proofs/package-error-publication-loader-completion.md
        fn decode(text: &str) -> Vec<u8> {
            assert!(text.len().is_multiple_of(2));
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let fixture = include_str!(
            "../../tcl-registry/tests/data/native_package_error_publication/native.tsv"
        );
        let mut compared = 0;
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 4);
            let profile = crate::environment::profile_for_dialect(fields[0]);
            let mut vm = crate::native_fixture::interpreter(profile);
            let script = decode(fields[2]);
            let completion = vm
                .eval_source(std::str::from_utf8(&script).unwrap())
                .expect("original package failure source");
            assert!(
                vm.refused_completion().is_none(),
                "{} {}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{} {}",
                fields[0],
                fields[1]
            );
            let strings = vm
                .actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            assert_eq!(
                completion
                    .result
                    .native_string_bytes(strings)
                    .unwrap()
                    .as_ref(),
                decode(fields[3]).as_slice(),
                "{} {}",
                fields[0],
                fields[1]
            );
            // The direct primitive publishes the same private header that its
            // carried error metadata exposes; reading the guest global is a
            // separate trace operation in the original source controls above.
            let direct = package(&mut vm, &["bad"]);
            assert_eq!(direct.code, tcl_runtime_api::Code::Error);
            let private = vm
                .native_return_error_code()
                .expect("published primitive code");
            let carried = crate::command::opt_get(&direct.options, "-errorcode").unwrap();
            assert_eq!(
                private.native_object_identity(),
                carried.native_object_identity()
            );
            compared += 1;
        }
        assert_eq!(compared, 21);
    }

    #[test]
    fn package_completion_matches_all_six_native_84_controls() {
        const FIXTURES: [&str; 6] = [
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv"),
            include_str!("../../tcl-registry/tests/data/native_package_ordinary/Jim.tsv"),
        ];
        const FILES: &[(&str, &[u8])] = &[
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl",
                include_bytes!(
                    "../../tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl"
                )
                .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl",
                include_bytes!("../../tcl-registry/tests/data/native_package_ordinary/leaf.tcl")
                    .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/nested.tcl",
                include_bytes!("../../tcl-registry/tests/data/native_package_ordinary/nested.tcl")
                    .as_slice(),
            ),
            (
                "/workspace/.proofs/2286-packages-dialects/package-native-completion/skip.tcl",
                include_bytes!("../../tcl-registry/tests/data/native_package_ordinary/skip.tcl")
                    .as_slice(),
            ),
        ];
        fn unhex(text: &str) -> Vec<u8> {
            assert_eq!(text.len() % 2, 0);
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    let digits = std::str::from_utf8(pair).unwrap();
                    u8::from_str_radix(digits, 16).unwrap()
                })
                .collect()
        }
        let jim: &'static tcl_dialect::DialectProfile =
            Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
        let mut compared = 0;
        for (engine, fixture) in FIXTURES.into_iter().enumerate() {
            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                assert_eq!(fields.len(), 4);
                let name = fields[0];
                let script = unhex(fields[1]);
                let expected = unhex(fields[3]);
                let profile = if engine == 5 {
                    jim
                } else {
                    tcl_dialect::DialectProfile::find(TclVersion::ALL[engine].dialect_name())
                        .unwrap()
                };
                let mut vm = crate::native_fixture::interpreter(profile);
                let host =
                    tcl_test_support::fixed_sources::FixedSourceHost::new(vm.host_rc(), FILES);
                vm.set_host(std::rc::Rc::new(host));
                let completion = vm
                    .eval_source(std::str::from_utf8(&script).unwrap())
                    .expect("native package source");
                assert_eq!(
                    completion.code.as_int().to_string(),
                    fields[2],
                    "{engine} {name}"
                );
                let strings = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                assert_eq!(
                    completion
                        .result
                        .native_string_bytes(strings)
                        .unwrap()
                        .as_ref(),
                    expected.as_slice(),
                    "{engine} {name}"
                );
                compared += 1;
            }
        }
        assert_eq!(compared, 84);
    }
}
