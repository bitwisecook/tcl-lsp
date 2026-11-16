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

//! The `encoding` ensemble: registration, its members and `convertfrom`/`convertto`.

use super::error_completion::completion_from_cmd_error;
use super::{native_encoding_source, native_wrong_arguments_message};
use crate::interp::{Vm, err, err_wrong_args, ok};
use crate::value::Value;
use tcl_runtime_api::Completion;

/// `encoding subcommand ?arg …?`: `convertto utf-8` uses the common native
/// unit conversion and proper binary-result owner; other targets and
/// `convertfrom` convert through `tcl_cmd_core::encoding` for the encodings
/// `names` lists. `system` reports the typed host locale, and `dirs` has no
/// encoding-file search. Other codepages (cp1252, shiftjis, …) are not
/// implemented on either side.
/// `encoding`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// 9.0's table also carries `profiles` and `user`, which need the encoding
/// machinery this engine does not model; like its other ensembles it names
/// only what it dispatches.
const ENCODING_SUBS: &[&str] = &[
    "convertfrom",
    "convertto",
    "dirs",
    "names",
    "profiles",
    "system",
    "user",
];

/// From 8.6 `encoding` is `TclMakeEnsemble`'s ensemble over the
/// `::tcl::encoding` workers; before, `Tcl_EncodingObjCmd` is one command.
pub(super) fn register_encoding(vm: &mut Vm) {
    match vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Encoding)
    {
        Some(namespace) => {
            let subs = crate::environment::release_subcommands(
                vm.actual_native_execution_profile().name,
                "encoding",
                ENCODING_SUBS,
            );
            vm.register_stock_namespace_ensemble("encoding", namespace, ENCODING_MEMBERS, subs);
        }
        None => vm.register_stock_builtin("encoding", cmd_encoding),
    }
}

/// Repinning reselects the release's `encoding` while preserving a replaced
/// public command.
pub(crate) fn refresh_encoding_profile(vm: &mut Vm) {
    if vm.stock_native_identity("encoding").as_deref() != Some("encoding") {
        return;
    }
    for &(member, _) in ENCODING_MEMBERS {
        let target = tcl_syntax::naming::unrooted_member_key("tcl::encoding", member);
        if vm.stock_native_identity(&target).as_deref() == Some(target.as_str()) {
            vm.remove_registered_command(&target);
        }
    }
    vm.remove_registered_command("encoding");
    register_encoding(vm);
    if vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Encoding)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("encoding");
    }
}

const ENCODING_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
    ("convertfrom", encoding_convertfrom),
    ("convertto", encoding_convertto),
    ("dirs", encoding_dirs),
    ("names", encoding_names),
    ("profiles", encoding_profiles),
    ("system", encoding_system),
    ("user", encoding_user),
];

fn encoding_member(vm: &mut Vm, member: &str, args: &[Value]) -> Completion<Value> {
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(Value::string(member));
    words.extend_from_slice(args);
    cmd_encoding(vm, &words)
}

fn encoding_convertfrom(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "convertfrom", args)
}

fn encoding_convertto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "convertto", args)
}

fn encoding_dirs(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "dirs", args)
}

pub(super) fn encoding_names(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "names", args)
}

fn encoding_system(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "system", args)
}

fn encoding_profiles(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "profiles", args)
}

fn encoding_user(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    encoding_member(vm, "user", args)
}

fn cmd_encoding(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let names = match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
        Ok(names) => names,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let operation = names.operation_scope().clone();
    let reached = cmd_encoding_in_scope(vm, args, &names);
    if let Err(error) = names.finish(vm) {
        drop(reached);
        let first = operation.check(vm).err().unwrap_or(error);
        return completion_from_cmd_error(vm, first);
    }
    if let Err(error) = operation.check(vm) {
        drop(reached);
        let first = operation.check(vm).err().unwrap_or(error);
        return completion_from_cmd_error(vm, first);
    }
    reached
}

fn cmd_encoding_in_scope(
    vm: &mut Vm,
    args: &[Value],
    names: &crate::interp::native_name_consumers::OriginalNameConsumerScope,
) -> Completion<Value> {
    // Before 8.6 `Tcl_EncodingObjCmd` is a plain command over
    // `Tcl_GetIndexFromObj`, not an ensemble.
    let classic = match vm.native_invocation_dialect().tcl_version {
        Some(release @ (tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5)) => {
            Some(release)
        }
        _ => None,
    };
    let Some(sub) = args.first() else {
        return native_wrong_arguments_message(
            vm,
            if classic.is_some() {
                "wrong # args: should be \"encoding option ?arg ...?\""
            } else {
                "wrong # args: should be \"encoding subcommand ?arg ...?\""
            },
        );
    };
    let sub = match names.read(vm, sub) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    // `dirs` arrives in 8.5, so the table follows the emulated release.
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "encoding",
        ENCODING_SUBS,
    );
    let canon = if let Some(release) = classic {
        match tcl_cmd_core::prefix::OptionTable::abbreviating("option", subs)
            .index_of_in(sub.as_ref(), release)
        {
            Ok(index) => subs[index],
            Err(error) => return completion_from_cmd_error(vm, error),
        }
    } else {
        match tcl_cmd_core::ensemble::resolve_subcommand(subs, sub.as_ref(), true) {
            Some(index) => subs[index],
            None => {
                return err(tcl_cmd_core::ensemble::unknown_subcommand_message(
                    subs,
                    sub.as_ref(),
                    true,
                    b"::tcl::encoding",
                ));
            }
        }
    };
    match canon {
        "dirs" => ok(Value::empty()),
        "system" => match args {
            [_] => ok(Value::string(vm.system_encoding().as_str())),
            [_, value] => native_encoding_source::set_system(vm, value),
            _ => err_wrong_args(vm, "encoding system ?encoding?"),
        },
        "names" => match args {
            [_] => ok(Value::string("utf-8 unicode ascii iso8859-1")),
            _ => err_wrong_args(vm, "encoding names"),
        },
        // TIP 656's profiles, as `EncodingProfilesObjCmd` lists them.
        "profiles" => match args {
            [_] => ok(Value::string("replace strict tcl8")),
            _ => err_wrong_args(vm, "encoding profiles"),
        },
        // Away from Windows the user encoding is the system encoding.
        // `EncodingUserObjCmd` passes an empty message, hence the space.
        "user" => match args {
            [_] => ok(Value::string(vm.system_encoding().as_str())),
            _ => native_wrong_arguments_message(vm, "wrong # args: should be \"encoding user \""),
        },
        "convertto" => {
            // naming.encoding.original-utf8-convertto-object-storage
            // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-object-storage.md
            let data = match args {
                [_, data] if vm.system_encoding() == tcl_platform::SystemEncoding::Utf8 => data,
                [_, encoding, data] => {
                    let encoding = match vm.native_name_operand_bytes_checked(encoding) {
                        Ok(bytes) => bytes,
                        Err(error) => return completion_from_cmd_error(vm, error),
                    };
                    if encoding.as_ref() != b"utf-8" {
                        // Another target converts through the shared owner.
                        return encoding_convert(
                            vm,
                            tcl_cmd_core::encoding::Direction::To,
                            &args[1..],
                        );
                    }
                    data
                }
                // The system target, `-profile`/`-failindex` and the usage
                // error follow the shared grammar and conversions.
                _ => {
                    return encoding_convert(vm, tcl_cmd_core::encoding::Direction::To, &args[1..]);
                }
            };
            match tcl_cmd_core::encoding::convert_to_utf8(vm, data) {
                Ok(result) => ok(result),
                Err(error) => completion_from_cmd_error(vm, error),
            }
        }
        _ => encoding_convert(vm, tcl_cmd_core::encoding::Direction::From, &args[1..]),
    }
}

/// An operand's characters from Tcl's internal UTF-8, where NUL is `C0 80`
/// and a character beyond the BMP may be a surrogate pair. A lone surrogate
/// has no Unicode scalar to convert, so it refuses.
fn encoding_operand_text(
    vm: &mut Vm,
    value: &Value,
    version: tcl_dialect::TclVersion,
) -> Result<String, Completion<Value>> {
    let bytes = vm
        .native_name_operand_bytes_checked(value)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_units(&bytes);
    char::decode_utf16(units.iter().flat_map(|&unit| match char::from_u32(unit) {
        Some(c) if unit > 0xFFFF => {
            let mut pair = [0; 2];
            c.encode_utf16(&mut pair);
            pair.to_vec()
        }
        _ => vec![u16::try_from(unit).unwrap_or(0xFFFF)],
    }))
    .collect::<Result<String, _>>()
    .map_err(|_| vm.refuse_host_command("encoding operand holds a lone surrogate".into()))
}

/// `encoding convertfrom|convertto ?options? ?encoding? data` through the
/// shared `tcl_cmd_core::encoding` grammar and conversions.
fn encoding_convert(
    vm: &mut Vm,
    direction: tcl_cmd_core::encoding::Direction,
    args: &[Value],
) -> Completion<Value> {
    use tcl_cmd_core::encoding;
    let version = vm.runtime_version();
    let mut decoded = Vec::with_capacity(args.len());
    for arg in args {
        match encoding_operand_text(vm, arg, version) {
            Ok(text) => decoded.push(text),
            Err(refusal) => return refusal,
        }
    }
    let words: Vec<&str> = decoded.iter().map(String::as_str).collect();
    let parsed = match encoding::parse_convert_args(version, direction, &words) {
        Ok(parsed) => parsed,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let name = parsed.encoding.unwrap_or_else(|| vm.system_encoding());
    let data = words[parsed.data];
    let (value, failure) = match direction {
        encoding::Direction::From => {
            let bytes = match encoding::text_bytes(data, version.byte_string_encoding()) {
                Ok(bytes) => bytes,
                Err(error) => return completion_from_cmd_error(vm, error.into()),
            };
            let converted = encoding::convert_from(&bytes, name, version, parsed.profile);
            (vm.new_host_text(&converted.value), converted.failure)
        }
        encoding::Direction::To => {
            let converted = encoding::convert_to(data, name, version, parsed.profile);
            let text: String = converted.value.iter().copied().map(char::from).collect();
            (Value::string(text), converted.failure)
        }
    };
    match (parsed.fail_var, failure) {
        (None, Some(failure)) => completion_from_cmd_error(vm, failure.error),
        (None, None) => ok(value),
        (Some(var), failure) => {
            let index = failure.map_or(-1, |failure| {
                i64::try_from(failure.index).unwrap_or(i64::MAX)
            });
            match vm.var_set(words[var], Value::int(index)) {
                Ok(()) => ok(value),
                Err(completion) => completion,
            }
        }
    }
}
