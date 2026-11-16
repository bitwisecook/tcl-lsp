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
use tcl_registry::native_package::NativePackageErrorCode;

use tcl_dialect::{
    PackagePrefer, compare_versions_bytes_for as cmp_version, select_package_version_bytes_for,
    select_package_version_exact_bytes_for, validate_requirement_bytes_for,
    validate_version_bytes_for, version_matches_exact_bytes_for,
    version_satisfies_bytes_for as vsatisfies,
};
use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

mod original;
use original::OriginalPackage;

pub(crate) fn register(vm: &mut Vm) {
    provide_core_packages(vm);
    vm.register_stock_builtin("package", cmd_package);
}

fn name_key(vm: &mut OriginalPackage<'_>, operand: &Value) -> Result<NameBytes, Completion<Value>> {
    let Some(protocol) = (if vm.package_table_is_authored() {
        Some(tcl_syntax::naming::NativeNameProtocol::C(
            tcl_dialect::TclVersion::V8_4,
        ))
    } else {
        vm.actual_native_name_protocol()
    }) else {
        return Err(vm.refuse_host_command("package name policy is unavailable".to_owned()));
    };
    let bytes = package_operand_bytes(vm, operand)?;
    Ok(NameBytes::from(protocol.package_key(&bytes).selected()))
}

fn package_protocol(
    vm: &mut OriginalPackage<'_>,
) -> Result<tcl_registry::native_package::NativePackageProtocol, Completion<Value>> {
    vm.check()
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    vm.selected_package_protocol()
        .ok_or_else(|| vm.refuse_host_command("package object protocol is unavailable".to_owned()))
}

fn package_operand_bytes(
    vm: &mut OriginalPackage<'_>,
    value: &Value,
) -> Result<std::rc::Rc<[u8]>, Completion<Value>> {
    vm.string(value)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

fn package_word(
    vm: &mut OriginalPackage<'_>,
    value: &Value,
) -> Result<NameBytes, Completion<Value>> {
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

fn package_wrong_args(vm: &mut OriginalPackage<'_>, member: &str) -> Completion<Value> {
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

fn package_list(vm: &mut OriginalPackage<'_>, members: Vec<Value>) -> Completion<Value> {
    let Some(strings) = vm.object_protocol() else {
        return vm.refuse_host_command("package List result producer is unavailable".into());
    };
    ok(Value::native_list_constructor(members, strings))
}

fn package_error(
    parts: &[&[u8]],
    code: NativePackageErrorCode,
    vm: &mut OriginalPackage<'_>,
) -> Completion<Value> {
    let message = parts.concat();
    let error = match code {
        NativePackageErrorCode::Default => tcl_cmd_core::CmdError::new_bytes(message),
        NativePackageErrorCode::Set(code) => {
            tcl_cmd_core::CmdError::with_error_code_bytes(message, code)
        }
    };
    crate::command::completion_from_cmd_error(vm, error)
}

/// Pre-provide the core's own package entries for the release this VM is
/// pinned to, with the `ifneeded` stub C's `TclOO` init script registers,
/// replacing whatever a previous pin left behind.
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
            if core.ifneeded_stub {
                vm.forget_package_completely(core.name);
            } else {
                vm.forget_package(core.name);
            }
        }
    }
    if let Some(tcl_registry::native_package::NativePackageProtocol::C(release)) = vm
        .actual_native_invocation_dialect()
        .native_package_protocol()
    {
        for core in release.core_provided_packages() {
            vm.provide_package(core.name, core.version);
            if core.ifneeded_stub {
                vm.set_package_ifneeded(
                    core.name,
                    core.version,
                    b"# Already present, OK?",
                    release,
                );
            }
        }
    }
}

fn cmd_package(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    OriginalPackage::run(vm, false, args, |original| dispatch_package(original, args))
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
    let authored = vm.active_native_profile.is_none();
    OriginalPackage::run(vm, authored, operands, |original| {
        let previous = original.native_invocation.arguments.replace(view);
        let completion = dispatch_package(original, operands);
        let restored = original.cleanup(|vm| {
            let displaced = std::mem::replace(&mut vm.native_invocation.arguments, previous);
            drop(displaced);
        });
        match restored {
            Ok(()) => completion,
            Err(error) => crate::command::completion_from_cmd_error(original, error),
        }
    })
}

fn package_subcommand(
    vm: &mut OriginalPackage<'_>,
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
        vm.checked(|vm| {
            vm.native_index_from_original_checked(original, &table, false, "option")
                .map_err(Into::into)
        })
    };
    match selection {
        Ok(Ok(index)) => Ok(options[index]),
        Ok(Err(message)) => {
            let word = package_word(vm, original)?;
            Err(package_error(
                &[message.as_slice()],
                protocol
                    .index_error_code_update(b"option", word.as_bytes())
                    .expect("C package index metadata"),
                vm,
            ))
        }
        Err(error) => Err(crate::command::completion_from_cmd_error(vm, error)),
    }
}

fn dispatch_package(vm: &mut OriginalPackage<'_>, args: &[Value]) -> Completion<Value> {
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
        "provide" => pkg_provide_original(vm, rest, release),
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
    vm: &mut OriginalPackage<'_>,
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
            return package_error(&[message.as_slice()], NativePackageErrorCode::Default, vm);
        }
        None => {
            return vm.refuse_host_command("Jim package member protocol is unavailable".to_owned());
        }
    };
    match member {
        "provide" if (1..=2).contains(&rest.len()) => jim_provide(vm, rest),
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

fn jim_provide(vm: &mut OriginalPackage<'_>, rest: &[Value]) -> Completion<Value> {
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
            NativePackageErrorCode::Default,
            vm,
        );
    }
    vm.provide_package(&name, "1.0");
    ok(Value::empty())
}

fn evaluate_jim_package_file(
    vm: &mut OriginalPackage<'_>,
    candidate: &tcl_dialect::DirectPackageFileBytes,
    contents: Vec<u8>,
) -> Completion<Value> {
    match candidate.kind {
        tcl_dialect::DirectPackageFileKind::Script => {
            let label = std::str::from_utf8(&candidate.path).ok();
            if let Some(label) = label {
                vm.push_script(label.to_owned());
            }
            let completion = vm.eval_script(&Value::from_native_string_bytes(contents));
            if label.is_some()
                && let Err(error) = vm.cleanup(Vm::pop_script)
            {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let completion = vm.settle_callback(completion);
            vm.settle_package_source_completion(completion)
        }
        tcl_dialect::DirectPackageFileKind::Native => {
            let mut script = b"load ".to_vec();
            tcl_syntax::list::append_list_element(&mut script, &candidate.path, true);
            vm.eval_script(&Value::from_native_string_bytes(script))
        }
    }
}

fn jim_require(vm: &mut OriginalPackage<'_>, name: &NameBytes) -> Completion<Value> {
    if let Some(version) = vm.package_version_bytes(name) {
        return ok(Value::from_native_string_bytes(version.as_bytes()));
    }
    let paths = vm.read_var_traced("::auto_path");
    if let Err(error) = vm.check() {
        drop(paths);
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let paths = match paths {
        Ok(Some(paths)) => paths,
        Ok(None) => {
            return package_error(
                &[b"Can't load package ", name.as_bytes()],
                NativePackageErrorCode::Default,
                vm,
            );
        }
        Err(completion) => return completion,
    };
    let directories = match vm.list(&paths) {
        Ok(paths) => paths,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    for directory in directories.iter() {
        let directory = match package_operand_bytes(vm, directory) {
            Ok(bytes) => bytes,
            Err(completion) => return completion,
        };
        for candidate in
            tcl_dialect::PackageProtocol::Jim.direct_files_bytes(&directory, name.as_bytes(), true)
        {
            let host = vm.host_rc();
            let filesystem = host.filesystem();
            if let Err(error) = vm.check() {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let contents = filesystem.map(|fs| fs.read_bytes(&candidate.path));
            if let Err(error) = vm.check() {
                drop(contents);
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let contents = match contents {
                Some(Ok(contents)) => contents,
                None | Some(Err(tcl_platform::HostError::Unsupported)) => {
                    return vm
                        .refuse_host_command("native package filesystem is unavailable".into());
                }
                Some(Err(_)) => continue,
            };
            vm.provide_package(name, "");
            let completion = evaluate_jim_package_file(vm, &candidate, contents);
            if !completion.code.is_ok() || vm.check().is_err() {
                if let Err(error) = vm.cleanup(|vm| vm.forget_package_completely(name)) {
                    return crate::command::completion_from_cmd_error(vm, error);
                }
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
    package_error(
        &[b"Can't load package ", name.as_bytes()],
        NativePackageErrorCode::Default,
        vm,
    )
}

pub(crate) fn pkg_provide(
    vm: &mut Vm,
    rest: &[Value],
    _release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    OriginalPackage::run(vm, false, rest, |original| {
        match package_protocol(original) {
            Ok(tcl_registry::native_package::NativePackageProtocol::C(actual)) => {
                pkg_provide_original(original, rest, actual)
            }
            Ok(_) if (1..=2).contains(&rest.len()) => jim_provide(original, rest),
            Ok(_) => package_wrong_args(original, "provide"),
            Err(completion) => completion,
        }
    })
}

fn pkg_provide_original(
    vm: &mut OriginalPackage<'_>,
    rest: &[Value],
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
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
            return package_error(&[&message], protocol.conflict_error_code_update(), vm);
        }
        return ok(Value::empty());
    }
    vm.provide_package(&name, &version);
    ok(Value::empty())
}

fn pkg_vsatisfies(
    vm: &mut OriginalPackage<'_>,
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
    vm: &mut OriginalPackage<'_>,
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
    vm: &mut OriginalPackage<'_>,
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

fn pkg_unknown(vm: &mut OriginalPackage<'_>, rest: &[Value]) -> Completion<Value> {
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
fn pkg_prefer(vm: &mut OriginalPackage<'_>, rest: &[Value]) -> Completion<Value> {
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
            let selected = match vm.checked(|vm| {
                vm.native_index_from_original_checked(preference, &table, false, "preference")
                    .map_err(Into::into)
            }) {
                Ok(result) => result,
                Err(error) => {
                    return crate::command::completion_from_cmd_error(vm, error);
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
                    let Some(code) =
                        protocol.index_error_code_update(b"preference", word.as_bytes())
                    else {
                        return vm.refuse_host_command(
                            "package preference error protocol is unavailable".to_owned(),
                        );
                    };
                    package_error(&[message.as_slice()], code, vm)
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
    vm: &mut OriginalPackage<'_>,
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
            return version_conflict(vm, discover, &name, &version, &requested, exact, release);
        }
        ProvidedStatus::Absent => {}
    }
    if discover && let Some(version) = vm.package_loading_version_bytes(&name) {
        return circular_dependency(vm, &name, &version.clone(), &requested, exact);
    }
    if !discover {
        let protocol = package_protocol(vm).expect("selected package protocol");
        // C's `package present` names the version it asked for when the first
        // requirement is a plain version (`Tcl_PkgPresentEx`'s `version`).
        let version = requested
            .first()
            .filter(|word| exact || validate_version_bytes_for(word.as_bytes(), release));
        let message: Vec<&[u8]> = match version {
            Some(version) => vec![
                b"package ",
                name.as_bytes(),
                b" ",
                version.as_bytes(),
                b" is not present",
            ],
            None => vec![b"package ", name.as_bytes(), b" is not present"],
        };
        return package_error(
            &message,
            protocol.failure_error_code_update(
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
    let require = PackageRequire {
        name,
        requested,
        exact,
        release,
        step: RequireStep::Unknown,
    };
    if let Some(loader) = selected_loader(vm, &require.name, &reqs, exact, release) {
        return evaluate_loader(vm, require, loader);
    }

    if let Some(prefix) = vm.package_unknown().cloned() {
        return invoke_package_unknown(vm, &prefix, require);
    }
    after_package_unknown(vm, require)
}

/// A `package require` whose discovery script (`package unknown` or `package
/// ifneeded`) runs as an activation of its own on the explicit stack, so a
/// coroutine can yield inside it (C's `PkgRequireCore` → `SelectPackage` →
/// `SelectPackageFinal` NRE chain). The require resumes from `step` once the
/// script completes ([`advance_require`]).
pub(crate) struct PackageRequire {
    name: NameBytes,
    requested: Vec<NameBytes>,
    exact: bool,
    release: tcl_dialect::TclVersion,
    step: RequireStep,
}

enum RequireStep {
    /// The `package unknown` callback is running.
    Unknown,
    /// The selected `package ifneeded` script is running; its loading marker
    /// is held until it completes.
    Loader(SelectedLoader),
}

/// The authored-table selection a suspended require resumes against.
pub(crate) struct SuspendedRequire {
    authored: bool,
    require: PackageRequire,
}

impl SuspendedRequire {
    /// The loading marker this require holds while its loader runs, which a
    /// coroutine deleted while suspended in the loader retires.
    pub(crate) fn loading_marker(&self) -> Option<(&NameBytes, &NameBytes)> {
        match &self.require.step {
            RequireStep::Loader(loader) => Some((&self.require.name, &loader.version)),
            RequireStep::Unknown => None,
        }
    }
}

/// Run a package discovery script at level 0 as a deferred activation that
/// resumes `require` when it completes. A script that has nothing to run (or
/// does not parse) completes here and the require continues at once.
fn defer_package_script(
    vm: &mut OriginalPackage<'_>,
    script: &Value,
    require: PackageRequire,
) -> Completion<Value> {
    let selected = match vm.checked(|vm| vm.select_execution_frame_checked(0)) {
        Ok(selected) => selected,
        Err(error) => {
            let completion = crate::command::completion_from_cmd_error(vm, error);
            return resume_require(vm, require, completion);
        }
    };
    let location = script.source_location();
    match vm.prepare_script_commands_value_for(
        script,
        tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
    ) {
        Ok(prepared) => {
            if let Some(unit) = prepared.prefix {
                let authored = vm.authored;
                vm.pending.eval = Some(crate::exec::EvalReq {
                    script: unit.with_source_location(location),
                    label: None,
                    cleanup_proc: None,
                    fatal_tail: prepared.fatal_tail,
                    selected_frame_restore: selected,
                    package: Some(Box::new(SuspendedRequire { authored, require })),
                });
                ok(Value::empty())
            } else {
                let completion = prepared.fatal_tail.map_or_else(
                    || ok(Value::empty()),
                    |tail| vm.raise_script_parse_failure(tail, location),
                );
                vm.restore_execution_frame(selected);
                resume_require(vm, require, completion)
            }
        }
        Err(error) => {
            vm.restore_execution_frame(selected);
            let completion = crate::command::completion_from_tcl_error(vm, error);
            resume_require(vm, require, completion)
        }
    }
}

/// Continue a `package require` whose discovery script completed with
/// `completion`. The result is the require's completion, unless it deferred
/// its next script (`vm.pending.eval` then holds it).
pub(crate) fn advance_require(
    vm: &mut Vm,
    suspended: SuspendedRequire,
    completion: Completion<Value>,
) -> Completion<Value> {
    let SuspendedRequire { authored, require } = suspended;
    OriginalPackage::run(vm, authored, &[], |original| {
        resume_require(original, require, completion)
    })
}

fn resume_require(
    vm: &mut OriginalPackage<'_>,
    require: PackageRequire,
    completion: Completion<Value>,
) -> Completion<Value> {
    match require.step {
        RequireStep::Loader(loader) => {
            let PackageRequire { name, .. } = require;
            finish_loader(vm, &name, &loader, completion)
        }
        RequireStep::Unknown => {
            let completion = vm.settle_callback(completion);
            if let Some(refused) = vm.refused_completion() {
                return refused;
            }
            match completion.code {
                Code::Ok => {
                    drop(completion);
                    if let Err(error) = vm.check() {
                        return crate::command::completion_from_cmd_error(vm, error);
                    }
                }
                Code::Error => {
                    let message = match package_operand_bytes(vm, &completion.result) {
                        Ok(bytes) => bytes,
                        Err(refused) => return refused,
                    };
                    append_loader_error_frame(vm, &message, None);
                    return completion;
                }
                _ => return bad_return_code(vm, &completion, None),
            }
            after_package_unknown(vm, require)
        }
    }
}

/// The callback may provide the package directly or register a suitable
/// ifneeded script. Re-check both forms, in that order, before failing.
fn after_package_unknown(
    vm: &mut OriginalPackage<'_>,
    require: PackageRequire,
) -> Completion<Value> {
    let PackageRequire {
        ref name,
        ref requested,
        exact,
        release,
        ..
    } = require;
    let reqs = requested.clone();
    let name = name.clone();
    match provided_status(vm, &name, &reqs, exact, release) {
        ProvidedStatus::Satisfies => {
            return ok(vm.package_version_object(&name));
        }
        ProvidedStatus::Conflicts(version) => {
            return version_conflict(vm, true, &name, &version, &reqs, exact, release);
        }
        ProvidedStatus::Absent => {}
    }
    if let Some(loader) = selected_loader(vm, &name, &reqs, exact, release) {
        return evaluate_loader(vm, require, loader);
    }

    // C names the requirements it could not satisfy after the package.
    let need = requirement_text(&reqs, exact, release);
    let separator: &[u8] = if need.is_empty() { b"" } else { b" " };
    package_error(
        &[b"can't find package ", name.as_bytes(), separator, &need],
        package_protocol(vm)
            .expect("selected package protocol")
            .failure_error_code_update(
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
    vm: &mut OriginalPackage<'_>,
    prefix: &[u8],
    require: PackageRequire,
) -> Completion<Value> {
    let PackageRequire {
        ref name,
        ref requested,
        exact,
        release,
        ..
    } = require;
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
        requested.clone()
    };
    for requirement in &callback_requirements {
        callback.push(b' ');
        tcl_syntax::list::append_list_element(&mut callback, requirement.as_bytes(), true);
    }
    if exact && !release.has_package_requirements() {
        callback.extend_from_slice(b" -exact");
    }
    defer_package_script(vm, &Value::from_native_string_bytes(callback), require)
}

fn provided_status(
    vm: &OriginalPackage<'_>,
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
    vm: &mut OriginalPackage<'_>,
    discover: bool,
    name: &NameBytes,
    have: &NameBytes,
    requested: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    // `package present` reports a conflict as `package require` does
    // (8.5.19: NONE; 8.6.18: `TCL PACKAGE VERSIONCONFLICT`).
    let _ = discover;
    let code = protocol.conflict_error_code_update();
    // The requirement is spelled as the selected package protocol's release
    // spells it, not as the runtime's own version.
    let need = requirement_text(requested, exact, release);
    package_error(
        &[
            b"version conflict for package \"",
            name.as_bytes(),
            b"\": have ",
            have.as_bytes(),
            b", need ",
            &need,
        ],
        code,
        vm,
    )
}

/// A require's requirements as its diagnostics spell them: `exactly V` for
/// `-exact` from 8.5 (8.4 names the bare version), else the requirements
/// joined by spaces.
fn requirement_text(
    requested: &[NameBytes],
    exact: bool,
    release: tcl_dialect::TclVersion,
) -> Vec<u8> {
    match requested.first() {
        Some(version) if exact && release.has_package_requirements() => {
            [&b"exactly "[..], version.as_bytes()].concat()
        }
        Some(version) if exact => version.as_bytes().to_vec(),
        _ => join_requirements(requested),
    }
}

fn invalid_version(vm: &mut OriginalPackage<'_>, version: &NameBytes) -> Completion<Value> {
    let protocol = match package_protocol(vm) {
        Ok(protocol) => protocol,
        Err(completion) => return completion,
    };
    let Some(code) = protocol.version_error_code_update(false) else {
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
    vm: &mut OriginalPackage<'_>,
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
            let Some(code) = protocol.version_error_code_update(true) else {
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
    vm: &mut OriginalPackage<'_>,
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
        package_protocol(vm)
            .expect("selected package protocol")
            .failure_error_code_update(
                tcl_registry::native_package::NativePackageFailure::Circularity,
                name.as_bytes(),
            ),
        vm,
    )
}

pub(crate) struct SelectedLoader {
    version: NameBytes,
    script: Vec<u8>,
    release: tcl_dialect::TclVersion,
}

fn selected_loader(
    vm: &OriginalPackage<'_>,
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

fn restore_pending_package_error(
    vm: &mut OriginalPackage<'_>,
    completion: &Completion<Value>,
) -> Result<bool, Completion<Value>> {
    // Only an explicit pending error return carries its trace into the newly
    // generated BADRESULT episode. Break/continue/custom traces are discarded.
    let pending_error = completion.code == Code::Return
        && match completion.option_origin {
            tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, .. } => code == 1,
            _ => match vm.checked_tcl(|vm| {
                crate::command::option_code_checked(vm, &completion.options, Code::Ok)
            }) {
                Ok(code) => code == Code::Error,
                Err(error) => return Err(crate::command::completion_from_tcl_error(vm, error)),
            },
        };
    let carried = if pending_error {
        match vm.checked_tcl(|vm| {
            crate::command::opt_get_checked(vm, &completion.options, b"-errorinfo")
        }) {
            Ok(value) => value,
            Err(error) => return Err(crate::command::completion_from_tcl_error(vm, error)),
        }
    } else {
        None
    };
    vm.take_error_info();
    if let Some(info) = carried {
        let bytes = package_operand_bytes(vm, &info)?;
        if !bytes.is_empty() {
            vm.seed_error_info_original(&info, &bytes);
        }
    }
    if pending_error {
        match vm.checked_tcl(|vm| {
            crate::command::opt_get_checked(vm, &completion.options, b"-errorstack")
        }) {
            Ok(Some(stack)) => {
                if let Err(error) = vm.checked_tcl(|vm| vm.seed_error_stack(&stack)) {
                    return Err(crate::command::completion_from_tcl_error(vm, error));
                }
            }
            Ok(None) => {}
            Err(error) => return Err(crate::command::completion_from_tcl_error(vm, error)),
        }
    }
    Ok(pending_error)
}

fn bad_return_code(
    vm: &mut OriginalPackage<'_>,
    completion: &Completion<Value>,
    loader: Option<(&NameBytes, &NameBytes)>,
) -> Completion<Value> {
    let pending_error = match restore_pending_package_error(vm, completion) {
        Ok(pending_error) => pending_error,
        Err(error) => return error,
    };
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
    let code = protocol.failure_error_code_update(
        tcl_registry::native_package::NativePackageFailure::LoaderCompletion,
        loader.map_or(b"".as_slice(), |(name, _)| name.as_bytes()),
    );
    let mut failure = package_error(&[&message], code, vm);
    append_loader_error_frame(vm, &message, loader);
    // Ancillary options retain their original member objects. The selected
    // error factory independently owns the new result and private error code.
    let Some(strings) = vm.object_protocol() else {
        return vm.refuse_host_command("package return-option protocol is unavailable".into());
    };
    let pairs = match vm
        .checked_tcl(|vm| crate::command::completion_option_rows_checked(vm, &completion.options))
    {
        Ok(pairs) => pairs,
        Err(error) => return crate::command::completion_from_tcl_error(vm, error),
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
            match vm.checked_tcl(|vm| {
                crate::command::opt_get_checked(vm, &failure.options, b"-errorcode")
            }) {
                Ok(Some(code)) => code,
                Ok(None) => {
                    return vm
                        .refuse_host_command("selected package error metadata is absent".into());
                }
                Err(error) => return crate::command::completion_from_tcl_error(vm, error),
            },
        ),
    ] {
        options = match vm
            .checked_tcl(|vm| crate::command::with_return_option(vm, &options, key, value))
        {
            Ok(options) => options,
            Err(error) => return crate::command::completion_from_tcl_error(vm, error),
        };
    }
    failure.options = options;
    failure
}

fn append_loader_error_frame(
    vm: &mut OriginalPackage<'_>,
    message: &[u8],
    loader: Option<(&NameBytes, &NameBytes)>,
) {
    let frame = match loader {
        Some((name, version)) => match package_protocol(vm) {
            Ok(protocol) => protocol.loader_error_frame(name.as_bytes(), version.as_bytes()),
            Err(_) => return,
        },
        None => b"\n    (\"package unknown\" script)".to_vec(),
    };
    vm.seed_error_info_frame(message, &frame);
}

fn evaluate_loader(
    vm: &mut OriginalPackage<'_>,
    require: PackageRequire,
    loader: SelectedLoader,
) -> Completion<Value> {
    let name = &require.name;
    vm.begin_package_loading(name, &loader.version);
    if let Err(error) = vm.checked(|vm| {
        vm.record_package_loader_origin(name, &loader.version)
            .map_err(Into::into)
    }) {
        vm.retain_failure(&error);
        if let Err(cleanup) = vm.end_loading(name, &loader.version) {
            return crate::command::completion_from_cmd_error(vm, cleanup);
        }
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let script = Value::from_native_string_bytes(tcl_core_types::c_string_extent(&loader.script));
    let require = PackageRequire {
        step: RequireStep::Loader(loader),
        ..require
    };
    defer_package_script(vm, &script, require)
}

/// The rest of `PkgRequireCore` once the selected loader completed.
fn finish_loader(
    vm: &mut OriginalPackage<'_>,
    name: &NameBytes,
    loader: &SelectedLoader,
    completion: Completion<Value>,
) -> Completion<Value> {
    if let Err(error) = vm.end_loading(name, &loader.version) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let completion = vm.settle_callback(completion);
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
            let message = [
                b"attempt to provide package ".as_slice(),
                name.as_bytes(),
                b" ",
                loader.version.as_bytes(),
                b" failed: package ",
                name.as_bytes(),
                b" ",
                provided.as_bytes(),
                b" provided instead",
            ]
            .concat();
            let code = package_protocol(vm)
                .expect("selected package protocol")
                .failure_error_code_update(
                    tcl_registry::native_package::NativePackageFailure::LoaderWrongVersion,
                    name.as_bytes(),
                );
            let failure = package_error(&[&message], code, vm);
            append_loader_error_frame(vm, &message, Some((name, &loader.version)));
            failure
        }
        None => {
            let message = [
                b"attempt to provide package ".as_slice(),
                name.as_bytes(),
                b" ",
                loader.version.as_bytes(),
                b" failed: no version of package ",
                name.as_bytes(),
                b" provided",
            ]
            .concat();
            let code = package_protocol(vm)
                .expect("selected package protocol")
                .failure_error_code_update(
                    tcl_registry::native_package::NativePackageFailure::LoaderMissing,
                    name.as_bytes(),
                );
            let failure = package_error(&[&message], code, vm);
            append_loader_error_frame(vm, &message, Some((name, &loader.version)));
            failure
        }
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
mod tests;
