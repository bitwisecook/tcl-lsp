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

//! `binary format` / `binary scan` — the common subset.
//!
//! Bytes are carried in a string with the Tcl byte-array convention: byte `b`
//! is the scalar `U+00b`, so `string length` of the result is the byte count.
//! Supported specifiers: `a`/`A` (char strings), `c`/`s`/`S`/`i`/`I`/`w`/`W`
//! (8/16/32/64-bit integers, little/big-endian), `H`/`h` (hex, high/low nibble
//! first) and `x` (null padding).

use tcl_runtime_api::Completion;

use crate::error::TclError;
use crate::expr::{
    OriginalNumericCommandContext, original_numeric_command_ops, with_original_numeric_command,
};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_registry::native_numeric_error::NativeNumericCommandPurpose;

pub(crate) fn register(vm: &mut Vm) {
    if let Some(scripted) = vm
        .actual_native_invocation_dialect()
        .binary_scripted_ingress()
    {
        // These are actual extension callbacks, not C private worker identities.
        vm.register(scripted.members[0], binary_format);
        vm.register(scripted.members[1], binary_scan);
        let _ = vm.register_bootstrap_procedure(scripted.name, scripted.parameters, scripted.body);
        return;
    }
    let Some(namespace) = vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Binary)
    else {
        vm.register_stock_builtin("binary", cmd_binary);
        return;
    };
    vm.register_stock_namespace_ensemble("binary", namespace, BINARY_MEMBERS, BINARY_SUBS);
    vm.register_stock_namespace_ensemble_with_prefixes(
        "tcl::binary::encode",
        "::tcl::binary::encode",
        ENCODE_MEMBERS,
        BINARY_FORMATS,
        false,
    );
    vm.register_stock_namespace_ensemble_with_prefixes(
        "tcl::binary::decode",
        "::tcl::binary::decode",
        DECODE_MEMBERS,
        BINARY_FORMATS,
        false,
    );
}

/// Repinning changes native bootstrap while preserving replaced public commands.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("binary").as_deref() != Some("binary") {
        return;
    }
    for (namespace, members) in [
        ("::tcl::binary::encode", ENCODE_MEMBERS),
        ("::tcl::binary::decode", DECODE_MEMBERS),
        ("::tcl::binary", BINARY_MEMBERS),
    ] {
        for &(member, _) in members {
            let target = format!("{namespace}::{member}");
            if vm.stock_native_identity(&target).as_deref()
                == tcl_syntax::naming::unroot_rooted_key(&target)
            {
                vm.remove_registered_command(
                    tcl_syntax::naming::unroot_rooted_key(&target).unwrap_or(&target),
                );
            }
        }
    }
    register(vm);
    if vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Binary)
        .is_none()
    {
        for identity in ["tcl::binary::encode", "tcl::binary::decode", "binary"] {
            vm.retire_unused_stock_ensemble_namespace(identity);
        }
    }
}

const BINARY_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
    ("decode", binary_decode_member),
    ("encode", binary_encode_member),
    ("format", binary_format),
    ("scan", binary_scan),
];
fn binary_encode_member(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    binary_encode(vm, args, BinaryInvocation::CodecRoot)
}
fn binary_decode_member(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    binary_decode(vm, args, BinaryInvocation::CodecRoot)
}

macro_rules! codec_members {
    ($table:ident, $codec:ident; $($function:ident => $format:literal),+ $(,)?) => {
        const $table: &[(&str, crate::command::BuiltinFn)] = &[$(($format, $function)),+];
        $(fn $function(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            let mut invocation = Vec::with_capacity(args.len() + 1);
            invocation.push(Value::string($format));
            invocation.extend_from_slice(args);
            $codec(vm, &invocation, BinaryInvocation::Worker)
        })+
    };
}
codec_members!(ENCODE_MEMBERS, binary_encode; encode_hex => "hex", encode_base64 => "base64", encode_uuencode => "uuencode");
codec_members!(DECODE_MEMBERS, binary_decode; decode_hex => "hex", decode_base64 => "base64", decode_uuencode => "uuencode");

/// Retain the selected producer's actual byte-array or string result.
fn bytes_to_value(vm: &Vm, bytes: &[u8]) -> Value {
    if vm
        .native_invocation_dialect()
        .binary_scripted_ingress()
        .is_some()
    {
        return Value::from_string_bytes(bytes);
    }
    if let Some(materialization) = vm.object_materialization()
        && let Some(recipe) = materialization.byte_array_string_recipe()
    {
        return Value::from_byte_array_recipe(std::rc::Rc::from(bytes), recipe);
    }
    // The unselected portable carrier retains its old neutral constructor;
    // this branch issues no byte-array updater or Native getter authority.
    Value::byte_array(bytes)
}

/// Enter the selected native byte conversion door on the actual object.
fn value_to_bytes(vm: &mut Vm, value: &Value) -> Result<Vec<u8>, Completion<Value>> {
    let Some(conversion) = vm.native_invocation_dialect().binary_data_conversion() else {
        return Err(
            vm.refuse_host_command("native binary byte conversion is unresolved".to_owned())
        );
    };
    let Some(protocol) = vm.native_invocation_dialect().native_string_protocol() else {
        return Err(
            vm.refuse_host_command("native binary string protocol is unresolved".to_owned())
        );
    };
    value
        .as_native_byte_array(conversion, protocol)
        .map(|bytes| bytes.to_vec())
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))
}

/// `binary`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
const BINARY_SUBS: &[&str] = &["decode", "encode", "format", "scan"];

/// `binary encode`/`binary decode`'s format set. These two are ensembles with
/// **`-prefixes` off** (`TclMakeEnsemble` with `TCL_ENSEMBLE_PREFIX` cleared),
/// so nothing abbreviates and the miss is worded `unknown subcommand`, never
/// `unknown or ambiguous` (tclsh: `binary encode h a` → `unknown subcommand
/// "h": must be base64, hex, or uuencode`).
const BINARY_FORMATS: &[&str] = &["base64", "hex", "uuencode"];

#[derive(Clone, Copy)]
enum BinaryInvocation {
    Public,
    CodecRoot,
    Worker,
}

fn binary_wrong_args(
    vm: &mut Vm,
    invocation: BinaryInvocation,
    member: &str,
    codec: Option<&str>,
    operation: tcl_registry::native_binary_usage::NativeBinaryArgumentUsage,
) -> Completion<Value> {
    let mut header = vec![
        vm.invoked_name_value()
            .unwrap_or_else(|| Value::string("binary")),
    ];
    if matches!(invocation, BinaryInvocation::Public) {
        header.push(Value::string(member));
    }
    if !matches!(invocation, BinaryInvocation::Worker)
        && let Some(codec) = codec
    {
        header.push(Value::string(codec));
    }
    let header = tcl_cmd_core::ensemble::rewrite_argument_usage(
        &header,
        &vm.native_invocation.usage_rewrites,
    );
    let Some(tail) = vm
        .native_invocation_dialect()
        .binary_argument_usage(operation)
    else {
        return vm.refuse_host_command(
            "native binary argument usage protocol is not selected".to_owned(),
        );
    };
    let Some(protocol) = vm.native_invocation_dialect().usage_protocol(Some(
        tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
    )) else {
        return vm.refuse_host_command("native argument usage protocol is not selected".to_owned());
    };
    let bytes = match header
        .iter()
        .map(|word| vm.native_name_operand_bytes_checked(word))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let operands: Vec<_> = bytes
        .iter()
        .map(|word| tcl_registry::native_usage::NativeUsageWord::Original(word))
        .collect();
    let Some(mut usage) = protocol.render_header(&operands) else {
        return vm.refuse_host_command("native argument usage protocol is not selected".to_owned());
    };
    usage.push(b' ');
    usage.extend_from_slice(tail.as_bytes());
    crate::command::native_wrong_args_bytes(vm, &usage)
}

/// Resolve a `binary` ensemble word, or the ensemble's own miss sentence.
fn resolve_binary_sub(
    subs: &'static [&'static str],
    word: &[u8],
    prefixes: bool,
    ns: &[u8],
) -> Result<&'static str, Vec<u8>> {
    match tcl_cmd_core::ensemble::resolve_subcommand(subs, word, prefixes) {
        Some(index) => Ok(subs[index]),
        None => Err(tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs, word, prefixes, ns,
        )),
    }
}

fn cmd_binary(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        Ok(cmd_binary_in_original(vm, args, original))
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn binary_name_operand(
    vm: &mut Vm,
    value: &Value,
    original: OriginalNumericCommandContext<'_>,
    authored: Option<&crate::interp::original_authored_operands::OriginalAuthoredOperandScope>,
) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
    original.current(vm)?;
    let reached = if original.authored.is_some() {
        authored
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original authored binary operand observation",
            ))?
            .string(vm, value)
    } else {
        vm.native_name_operand_bytes_checked(value)
    };
    original.current(vm)?;
    reached
}

fn cmd_binary_in_original(
    vm: &mut Vm,
    args: &[Value],
    original: OriginalNumericCommandContext<'_>,
) -> Completion<Value> {
    if original.authored.is_some() {
        return crate::interp::original_authored_operands::with_original_authored_operands(
            vm,
            |vm, authored| {
                Ok(cmd_binary_in_original_using(
                    vm,
                    args,
                    original,
                    Some(authored),
                ))
            },
        );
    }
    cmd_binary_in_original_using(vm, args, original, None)
}

fn cmd_binary_in_original_using(
    vm: &mut Vm,
    args: &[Value],
    original: OriginalNumericCommandContext<'_>,
    authored: Option<&crate::interp::original_authored_operands::OriginalAuthoredOperandScope>,
) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"binary subcommand ?arg ...?\"",
        );
    };
    // `encode`/`decode` (TIP 317) arrive in 8.6: under an earlier pin `binary
    // d` must not resolve to `decode`.
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "binary",
        BINARY_SUBS,
    );
    let canon = if vm.native_invocation_dialect().binary_root_dispatch()
        == Some(tcl_registry::native_binary_usage::NativeBinaryRootDispatch::Indexed)
    {
        match vm.native_static_option_index_checked(sub, subs, false, "option") {
            Ok(index) => subs[index],
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    } else {
        let bytes = match binary_name_operand(vm, sub, original, authored) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        match resolve_binary_sub(subs, &bytes, true, b"::tcl::binary") {
            Ok(name) => name,
            Err(m) => return err(m),
        }
    };
    if let Err(first) = original.current(vm) {
        return crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::from_execution_refusal(first),
        );
    }
    match canon {
        "format" => binary_format_with_invocation(vm, rest, BinaryInvocation::Public),
        "scan" => binary_scan_with_invocation(vm, rest, BinaryInvocation::Public),
        "encode" => binary_encode(vm, rest, BinaryInvocation::Public),
        // Unreachable: `BINARY_SUBS` has exactly the four arms here.
        _ => binary_decode(vm, rest, BinaryInvocation::Public),
    }
}

/// `binary encode hex|base64|uuencode ?options? data`. The byte codecs live in
/// the shared `tcl_cmd_core::binary`; bytes cross the VM's value boundary via the
/// byte-array convention (`value_to_bytes`/`bytes_to_value`).
fn binary_encode(vm: &mut Vm, rest: &[Value], invocation: BinaryInvocation) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        Ok(binary_encode_in_original(vm, rest, invocation, original))
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn binary_encode_in_original(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
    original: OriginalNumericCommandContext<'_>,
) -> Completion<Value> {
    if original.authored.is_some() {
        return crate::interp::original_authored_operands::with_original_authored_operands(
            vm,
            |vm, authored| {
                Ok(binary_encode_in_original_using(
                    vm,
                    rest,
                    invocation,
                    original,
                    Some(authored),
                ))
            },
        );
    }
    binary_encode_in_original_using(vm, rest, invocation, original, None)
}

fn binary_encode_in_original_using(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
    original: OriginalNumericCommandContext<'_>,
    authored: Option<&crate::interp::original_authored_operands::OriginalAuthoredOperandScope>,
) -> Completion<Value> {
    let Some((fmt, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"binary encode format ?options? data\"",
        );
    };
    let bytes = match binary_name_operand(vm, fmt, original, authored) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let canon = match resolve_binary_sub(BINARY_FORMATS, &bytes, false, b"::tcl::binary::encode") {
        Ok(name) => name,
        Err(m) => return err(m),
    };
    match canon {
        "hex" => {
            let [data] = args else {
                return binary_wrong_args(
                    vm,
                    invocation,
                    "encode",
                    Some(canon),
                    tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::EncodeHex,
                );
            };
            let bytes = match value_to_bytes(vm, data) {
                Ok(bytes) => bytes,
                Err(error) => return error,
            };
            let out = tcl_cmd_core::binary::hex_encode(&bytes);
            ok(bytes_to_value(vm, &out))
        }
        "base64" => binary_encode_wrapped(vm, args, false, invocation, canon),
        // Unreachable: `BINARY_FORMATS` has exactly the three arms here.
        _ => binary_encode_wrapped(vm, args, true, invocation, canon),
    }
}

/// `binary encode base64|uuencode ?-maxlen n? ?-wrapchar c? data`.
fn binary_encode_wrapped(
    vm: &mut Vm,
    args: &[Value],
    uu: bool,
    invocation: BinaryInvocation,
    codec: &str,
) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        binary_encode_wrapped_in_original(vm, args, uu, invocation, codec, original)
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn wrapped_line_length(
    vm: &mut Vm,
    value: &Value,
    uu: bool,
    original: OriginalNumericCommandContext<'_>,
) -> Result<i64, TclError> {
    if let Some(authored) = original.authored {
        let reached = value.as_int_with_stages(Some(authored));
        original
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        reached
    } else {
        let purpose = if uu {
            NativeNumericCommandPurpose::UuencodeLineLength
        } else {
            NativeNumericCommandPurpose::Base64LineLength
        };
        let reached =
            original_numeric_command_ops(vm, original, purpose, |ops, selected| {
                match ops.getter(value, selected.getter_kind())? {
                    tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(n) if n >= 0 => Ok(n),
                    tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(_) => {
                        let error = selected.negative_line_length_error().ok_or(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "line length diagnostic",
                            ),
                        )?;
                        Err(tcl_cmd_core::CmdError::with_error_code_bytes(
                            error.message,
                            error.error_code,
                        ))
                    }
                    _ => Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "line length integer output",
                    )
                    .into()),
                }
            });
        original
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        reached.map_err(|error| crate::command::completion_option_failure(vm, error))
    }
}

fn binary_encode_wrapped_in_original(
    vm: &mut Vm,
    args: &[Value],
    uu: bool,
    invocation: BinaryInvocation,
    codec: &str,
    original: OriginalNumericCommandContext<'_>,
) -> Result<Completion<Value>, TclError> {
    let mut maxlen: usize = if uu { 61 } else { 0 };
    let mut wrapchar: Vec<u8> = b"\n".to_vec();
    let mut string_wrap = false;
    let mut i = 0;
    while i + 1 < args.len() {
        let option = if original.authored.is_none() {
            match vm.native_name_operand_bytes_checked(&args[i]) {
                Ok(bytes) => bytes,
                Err(error) => return Ok(crate::command::completion_from_cmd_error(vm, error)),
            }
        } else {
            args[i].string_bytes()
        };
        original
            .current(vm)
            .map_err(TclError::from_execution_failure)?;
        match option.as_ref() {
            b"-maxlen" => {
                let n = wrapped_line_length(vm, &args[i + 1], uu, original)?;
                if n < 0 {
                    return Ok(err("line length out of range"));
                }
                maxlen = usize::try_from(n).unwrap_or(usize::MAX);
                i += 2;
            }
            b"-wrapchar" => {
                let Some(policy) = vm.native_invocation_dialect().binary_decode_source() else {
                    return Ok(vm.refuse_host_command(
                        "native binary wrap conversion is unresolved".to_owned(),
                    ));
                };
                let input = match args[i + 1].binary_decode_input(policy) {
                    Ok(input) => input,
                    Err(error) => return Ok(vm.refuse_host_command(error.to_string())),
                };
                original
                    .current(vm)
                    .map_err(TclError::from_execution_failure)?;
                string_wrap = matches!(
                    input,
                    tcl_registry::native_binary_value::NativeBinaryDecodeInput::String(_)
                );
                wrapchar = input.bytes().to_vec();
                i += 2;
            }
            _ => {
                return Ok(binary_wrong_args(
                    vm,
                    invocation,
                    "encode",
                    Some(codec),
                    tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::EncodeWrapped,
                ));
            }
        }
    }
    let [data] = &args[i..] else {
        return Ok(binary_wrong_args(
            vm,
            invocation,
            "encode",
            Some(codec),
            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::EncodeWrapped,
        ));
    };
    let bytes = match value_to_bytes(vm, data) {
        Ok(bytes) => bytes,
        Err(error) => return Ok(error),
    };
    original
        .current(vm)
        .map_err(TclError::from_execution_failure)?;
    let out = if uu {
        tcl_cmd_core::binary::uu_encode(&bytes, maxlen, &wrapchar)
    } else {
        tcl_cmd_core::binary::base64_encode(&bytes, maxlen, &wrapchar)
    };
    Ok(
        if !uu
            && string_wrap
            && maxlen > 0
            && !wrapchar.is_empty()
            && tcl_cmd_core::binary::base64_encode(&bytes, 0, &[]).len() > maxlen
        {
            match String::from_utf8(out) {
                Ok(text) => ok(Value::string(text)),
                Err(_) => vm.refuse_host_command(
                    "native binary string wrapping requires an unrepresentable string".to_owned(),
                ),
            }
        } else {
            ok(bytes_to_value(vm, &out))
        },
    )
}

/// `binary decode hex|base64|uuencode ?-strict? data`.
fn binary_decode(vm: &mut Vm, rest: &[Value], invocation: BinaryInvocation) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, original| {
        Ok(binary_decode_in_original(vm, rest, invocation, original))
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn binary_decode_in_original(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
    original: OriginalNumericCommandContext<'_>,
) -> Completion<Value> {
    if original.authored.is_some() {
        return crate::interp::original_authored_operands::with_original_authored_operands(
            vm,
            |vm, authored| {
                Ok(binary_decode_in_original_using(
                    vm,
                    rest,
                    invocation,
                    original,
                    Some(authored),
                ))
            },
        );
    }
    binary_decode_in_original_using(vm, rest, invocation, original, None)
}

fn binary_decode_in_original_using(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
    original: OriginalNumericCommandContext<'_>,
    authored: Option<&crate::interp::original_authored_operands::OriginalAuthoredOperandScope>,
) -> Completion<Value> {
    use tcl_cmd_core::binary::UuDecodeError;
    let Some((fmt, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"binary decode format ?options? data\"",
        );
    };
    let bytes = match binary_name_operand(vm, fmt, original, authored) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let canon = match resolve_binary_sub(BINARY_FORMATS, &bytes, false, b"::tcl::binary::decode") {
        Ok(name) => name,
        Err(m) => return err(m),
    };
    let (strict, input) = match decode_args(vm, args, invocation, canon) {
        Ok(selected) => selected,
        Err(error) => return error,
    };
    let decoded = match canon {
        "hex" => tcl_cmd_core::binary::hex_decode_with_strict(input.bytes(), strict)
            .map_err(UuDecodeError::Invalid),
        "base64" => tcl_cmd_core::binary::base64_decode(input.bytes(), strict)
            .map_err(UuDecodeError::Invalid),
        _ => tcl_cmd_core::binary::uu_decode_with_strict(input.bytes(), strict),
    };
    match decoded {
        Ok(out) => ok(bytes_to_value(vm, &out)),
        Err(UuDecodeError::Short) => {
            crate::command::err_with_code("short uuencode data", "TCL BINARY DECODE SHORT")
        }
        Err(UuDecodeError::Invalid(error)) => decode_invalid(
            vm,
            match canon {
                "hex" => "hexadecimal digit",
                "base64" => "base64 character",
                _ => "uuencode character",
            },
            &input,
            error,
        ),
    }
}

/// Parse a decode subcommand's `?-strict? data` → `(strict, bytes)`.
fn decode_args(
    vm: &mut Vm,
    args: &[Value],
    invocation: BinaryInvocation,
    codec: &str,
) -> Result<
    (
        bool,
        tcl_registry::native_binary_value::NativeBinaryDecodeInput,
    ),
    Completion<Value>,
> {
    let option = (args.len() == 2).then(|| args[0].to_str());
    let layout = match tcl_cmd_core::binary::decode_argument_layout(
        args.len(),
        option.as_deref().map(str::as_bytes),
    ) {
        Ok(layout) => layout,
        Err(error) => {
            return Err(match &error {
                tcl_cmd_core::binary::DecodeArgumentsError::UnknownOption(word) => {
                    let word = std::str::from_utf8(word).expect("Value strings are UTF8");
                    let code =
                        tcl_syntax::list::join_list(["TCL", "LOOKUP", "INDEX", "option", word]);
                    crate::command::err_with_code(
                        String::from_utf8(error.option_message().expect("unknown option"))
                            .expect("Value strings are UTF8"),
                        &code,
                    )
                }
                tcl_cmd_core::binary::DecodeArgumentsError::WrongArity => binary_wrong_args(
                    vm,
                    invocation,
                    "decode",
                    Some(codec),
                    tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::Decode,
                ),
                tcl_cmd_core::binary::DecodeArgumentsError::UnresolvedOption => {
                    vm.refuse_host_command("native binary option value is unresolved".to_owned())
                }
            });
        }
    };
    let Some(policy) = vm.native_invocation_dialect().binary_decode_source() else {
        return Err(
            vm.refuse_host_command("native binary decoder protocol is unresolved".to_owned())
        );
    };
    let input = args[layout.data]
        .binary_decode_input(policy)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    Ok((layout.strict, input))
}

/// `invalid <what> "<byte>" (U+XXXXXX) at position N`, code `TCL BINARY DECODE
/// INVALID` — the shared decode-failure form for hex and base64.
fn decode_invalid(
    vm: &mut Vm,
    what: &str,
    input: &tcl_registry::native_binary_value::NativeBinaryDecodeInput,
    error: tcl_cmd_core::binary::DecodeError,
) -> Completion<Value> {
    vm.native_invocation_dialect()
        .binary_decode_error(what, input, error.pos)
        .map_or_else(
            || vm.refuse_host_command("native binary decoder diagnostic is unresolved".to_owned()),
            |message| crate::command::err_with_code(message, "TCL BINARY DECODE INVALID"),
        )
}

/// Width in bytes for an integer specifier.
fn binary_format(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    binary_format_with_invocation(vm, rest, BinaryInvocation::Worker)
}

fn binary_format_with_invocation(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, _numeric| {
        Ok(
            crate::interp::original_authored_operands::with_original_authored_operands(
                vm,
                |vm, original| {
                    let Some((fmt, args)) = rest.split_first() else {
                        return Ok(binary_wrong_args(
                            vm,
                            invocation,
                            "format",
                            None,
                            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::Format,
                        ));
                    };
                    let grammar = original.binary_grammar()?;
                    let conversion = vm
                        .native_invocation_dialect()
                        .binary_data_conversion()
                        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original binary format bytes",
                        ))?;
                    let narrowing = vm
                        .native_invocation_dialect()
                        .binary_format_conversion()
                        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original binary format narrowing",
                        ))?;
                    let syntax = vm.native_invocation_dialect().numbers;
                    let numeric = vm.authored_numeric_context();
                    let fmt_bytes = original.binary_string(vm, fmt)?;
                    let mut ops = OriginalBinaryFormat {
                        format: Some(fmt),
                        vm,
                        original,
                        conversion,
                        narrowing,
                        syntax,
                        numeric,
                    };
                    let reached = tcl_cmd_core::binary::format_values_using(
                        &mut ops,
                        grammar.extent(&fmt_bytes),
                        args,
                        &mut |field| binary_bad_field(grammar, field),
                    );
                    if let Err(error) = &reached {
                        crate::interp::original_authored_operands::OriginalAuthoredOperandScope::retain(ops.vm, error);
                    }
                    original.check(ops.vm)?;
                    Ok(ok(original.binary_result(ops.vm, reached?)?))
                },
            ),
        )
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

fn binary_bad_field(
    grammar: tcl_registry::native_binary_value::NativeBinaryFormatGrammar,
    field: &[u8],
) -> tcl_cmd_core::CmdError {
    let (message, store_none) = grammar.bad_field(field);
    let mut details = tcl_cmd_core::CmdError::new_bytes(message).into_byte_details();
    details.error_code = if store_none {
        tcl_cmd_core::CmdErrorCodeUpdate::Default
    } else {
        tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
    };
    tcl_cmd_core::CmdError::from_byte_details(details)
}

struct OriginalBinaryFormat<'a> {
    format: Option<&'a Value>,
    vm: &'a mut Vm,
    original: &'a crate::interp::original_authored_operands::OriginalAuthoredOperandScope,
    conversion: tcl_registry::native_binary_value::NativeBinaryByteConversion,
    narrowing: tcl_registry::native_binary_value::NativeBinaryByteConversion,
    syntax: tcl_dialect::NumberSyntax,
    numeric: Option<crate::expr::NumericContext<'a>>,
}
impl tcl_cmd_core::binary::FormatValueOps for OriginalBinaryFormat<'_> {
    type Value = Value;
    fn binary_format_current(&mut self) -> Result<(), tcl_cmd_core::CmdError> {
        self.original.check(self.vm)?;
        if let Some(format) = self.format {
            self.original.current_for_value(self.vm, format)?;
        }
        Ok(())
    }

    fn binary_format_bytes(&mut self, value: &Value) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        self.original
            .binary_bytes(self.vm, value, self.conversion, Some(self.narrowing))
            .map(|bytes| bytes.to_vec())
    }
    fn binary_format_integer(&mut self, value: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
        self.original.current_for_value(self.vm, value)?;
        let reached = if let Some(context) = self.numeric {
            let simulation = context
                .simulation
                .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
            let mut bytes = || self.original.raw_bytes(self.vm, value);
            if matches!(
                self.syntax,
                tcl_dialect::NumberSyntax::Tcl85 | tcl_dialect::NumberSyntax::Tcl90
            ) {
                crate::value_ops::logical_integer_magnitude_using(
                    simulation,
                    tcl_syntax::number::Radix::Dec,
                    &mut bytes,
                )
                .map(tcl_cmd_core::binary::integer_from_magnitude)
            } else {
                crate::value_ops::logical_integer_using(context, simulation, &mut bytes)
            }
        } else {
            tcl_cmd_core::binary::integer_value(self.vm, value, self.syntax)
        };
        if let Err(error) = &reached {
            crate::interp::original_authored_operands::OriginalAuthoredOperandScope::retain(
                self.vm, error,
            );
        }
        self.original.current_for_value(self.vm, value)?;
        reached
    }
    fn binary_format_double(&mut self, value: &Value) -> Result<f64, tcl_cmd_core::CmdError> {
        self.original.current_for_value(self.vm, value)?;
        let reached = if let Some(context) = self.numeric {
            let simulation = context
                .simulation
                .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
            crate::value_ops::logical_double_using(context, simulation, &mut || {
                self.original.raw_bytes(self.vm, value)
            })
        } else {
            tcl_syntax::value::ValueOps::as_double(self.vm, value).map_err(Into::into)
        };
        if let Err(error) = &reached {
            crate::interp::original_authored_operands::OriginalAuthoredOperandScope::retain(
                self.vm, error,
            );
        }
        self.original.current_for_value(self.vm, value)?;
        reached
    }
    fn binary_format_elements(
        &mut self,
        value: &Value,
    ) -> Result<Vec<Value>, tcl_cmd_core::CmdError> {
        self.original.list(self.vm, value)
    }
}

impl tcl_cmd_core::binary::FormatValueOps for Vm {
    type Value = Value;

    fn binary_format_bytes(&mut self, value: &Value) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        let dialect = self.native_invocation_dialect();
        let Some(conversion) = dialect.binary_format_conversion() else {
            let _ = self.refuse_host_command(
                "native binary format byte conversion is unresolved".to_owned(),
            );
            return Err(tcl_cmd_core::CmdError::new(""));
        };
        let protocol = dialect.native_string_protocol().ok_or_else(|| {
            tcl_cmd_core::CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native binary format string protocol",
            ))
        })?;
        if dialect.binary_data_conversion()
            == Some(tcl_registry::native_binary_value::NativeBinaryByteConversion::CheckedLatin1)
        {
            return value
                .as_native_byte_array(
                    tcl_registry::native_binary_value::NativeBinaryByteConversion::CheckedLatin1,
                    protocol,
                )
                .map_or_else(
                    |error| match error {
                        crate::value::ByteArrayAccessError::Unicode(error) => {
                            Err(tcl_cmd_core::CmdError::from(error))
                        }
                        crate::value::ByteArrayAccessError::Conversion(_) => {
                            let original = value.native_string_bytes(protocol).map_err(|_| {
                                tcl_cmd_core::CmdError::from(
                                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                        "native binary format string",
                                    ),
                                )
                            })?;
                            conversion
                                .convert_native(
                                    &original,
                                    tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(
                                        protocol.tcl_version().expect("checked C byte conversion"),
                                    ),
                                )
                                .map_err(|error| tcl_cmd_core::CmdError::new(error.message()))
                        }
                        crate::value::ByteArrayAccessError::Unavailable(error) => {
                            Err(tcl_cmd_core::CmdError::from(
                                crate::value::ByteArrayAccessError::Unavailable(error),
                            ))
                        }
                    },
                    |bytes| Ok(bytes.to_vec()),
                );
        }
        value
            .as_native_byte_array(conversion, protocol)
            .map(|bytes| bytes.to_vec())
            .map_err(tcl_cmd_core::CmdError::from)
    }

    fn binary_format_integer(&mut self, value: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
        let syntax = self.native_invocation_dialect().numbers;
        tcl_cmd_core::binary::integer_value(self, value, syntax)
    }

    fn binary_format_double(&mut self, value: &Value) -> Result<f64, tcl_cmd_core::CmdError> {
        Ok(tcl_syntax::value::ValueOps::as_double(self, value)?)
    }

    fn binary_format_elements(
        &mut self,
        value: &Value,
    ) -> Result<Vec<Value>, tcl_cmd_core::CmdError> {
        Ok(tcl_syntax::value::ValueOps::list_elements(self, value)?)
    }
}

fn binary_scan(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    binary_scan_with_invocation(vm, rest, BinaryInvocation::Worker)
}

fn binary_scan_with_invocation(
    vm: &mut Vm,
    rest: &[Value],
    invocation: BinaryInvocation,
) -> Completion<Value> {
    match with_original_numeric_command(vm, TclError::from_execution_failure, |vm, _numeric| {
        Ok(
            crate::interp::original_authored_operands::with_original_authored_operands(
                vm,
                |vm, original| {
                    let [data_v, fmt_v, vars @ ..] = rest else {
                        return Ok(binary_wrong_args(
                            vm,
                            invocation,
                            "scan",
                            None,
                            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::Scan,
                        ));
                    };
                    let grammar = original.binary_grammar()?;
                    let conversion = vm
                        .native_invocation_dialect()
                        .binary_data_conversion()
                        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original binary scan bytes",
                        ))?;
                    let dialect = vm.native_invocation_dialect();
                    // Preserve the actual data getter before the format String getter.
                    let data = original.binary_bytes(vm, data_v, conversion, None)?;
                    let fmt = original.binary_string(vm, fmt_v)?;
                    let vm = std::cell::RefCell::new(vm);
                    let mut assigned = 0;
                    let reached = tcl_cmd_core::binary::scan_values_using(
                        &data,
                        grammar.extent(&fmt),
                        &mut |field| {
                            let vm = &mut vm.borrow_mut();
                            original
                                .current_for_value(vm, data_v)
                                .and_then(|()| original.current_for_value(vm, fmt_v))
                                .map_err(|error| {
                                    crate::command::completion_from_cmd_error(vm, error)
                                })?;
                            let Some(var) = vars.get(assigned) else {
                                return Err(err("not enough arguments for all format specifiers"));
                            };
                            let name = original.binary_string(vm, var).map_err(|error| {
                                crate::command::completion_from_cmd_error(vm, error)
                            })?;
                            let value = match field {
                                tcl_cmd_core::binary::ScanValue::Bytes(bytes) => {
                                    original.binary_result(vm, bytes).map_err(|error| {
                                        crate::command::completion_from_cmd_error(vm, error)
                                    })?
                                }
                                tcl_cmd_core::binary::ScanValue::Double(value) => {
                                    Value::native_double(value, dialect)
                                }
                                tcl_cmd_core::binary::ScanValue::Doubles(values) => Value::list(
                                    values
                                        .into_iter()
                                        .map(|value| Value::native_double(value, dialect))
                                        .collect(),
                                ),
                            };
                            original
                                .current_for_value(vm, data_v)
                                .and_then(|()| original.current_for_value(vm, fmt_v))
                                .map_err(|error| {
                                    crate::command::completion_from_cmd_error(vm, error)
                                })?;
                            let reached = vm.set_var_bytes(&name, value);
                            // The actual variable owner retains operational errors;
                            // its full completion remains intact for Guest propagation.
                            original
                                .current_for_value(vm, data_v)
                                .and_then(|()| original.current_for_value(vm, fmt_v))
                                .map_err(|error| {
                                    crate::command::completion_from_cmd_error(vm, error)
                                })?;
                            reached?;
                            assigned += 1;
                            Ok(())
                        },
                        &mut |field| {
                            crate::command::completion_from_cmd_error(
                                &mut vm.borrow_mut(),
                                binary_bad_field(grammar, field),
                            )
                        },
                    );
                    original.check(&mut vm.borrow_mut())?;
                    Ok(match reached {
                        Ok(count) => ok(Value::int(i64::try_from(count).unwrap_or(i64::MAX))),
                        Err(completion) => completion,
                    })
                },
            ),
        )
    }) {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_tcl_error(vm, error),
    }
}

#[cfg(test)]
mod native_ensemble_tests {
    use super::*;
    use tcl_runtime_api::Code;

    fn values(words: &[&str]) -> Vec<Value> {
        words.iter().map(|word| Value::string(*word)).collect()
    }

    #[test]
    fn binary_bootstrap_retains_real_nested_tokens_and_configuration() {
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            for (head, args) in [
                ("binary", vec!["encode", "hex", "ABC"]),
                ("::tcl::binary::encode", vec!["hex", "ABC"]),
                ("::tcl::binary::encode::hex", vec!["ABC"]),
            ] {
                let result = vm.try_invoke_command(head, &values(&args)).unwrap();
                assert_eq!(result.code, Code::Ok, "{dialect}: {head}");
                assert_eq!(&*result.result.to_str(), "414243", "{dialect}: {head}");
            }
            let abbreviated = vm
                .try_invoke_command("binary", &values(&["encode", "h", "ABC"]))
                .unwrap();
            assert_eq!(abbreviated.code, Code::Error, "{dialect}");
            assert_eq!(
                &*abbreviated.result.to_str(),
                "unknown subcommand \"h\": must be base64, hex, or uuencode",
                "{dialect}",
            );
            vm.register("replacement", |_vm, _args| ok(Value::string("CUSTOM")));
            let configured = vm
                .try_invoke_command(
                    "namespace",
                    &values(&[
                        "ensemble",
                        "configure",
                        "::tcl::binary::encode",
                        "-map",
                        "hex ::replacement",
                    ]),
                )
                .unwrap();
            assert_eq!(configured.code, Code::Ok, "{dialect}");
            let custom = vm
                .try_invoke_command("binary", &values(&["encode", "hex", "ABC"]))
                .unwrap();
            assert_eq!(custom.code, Code::Ok, "{dialect}");
            assert_eq!(&*custom.result.to_str(), "CUSTOM", "{dialect}");
            let private = vm
                .try_invoke_command("::tcl::binary::encode::hex", &values(&["ABC"]))
                .unwrap();
            assert_eq!(&*private.result.to_str(), "414243", "{dialect}");
        }
    }

    #[test]
    fn binary_private_tokens_follow_the_actual_bootstrap_release() {
        for dialect in ["tcl8.4", "tcl8.5", "jim"] {
            let mut vm = Vm::without_native_engine();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
            );
            assert!(
                vm.stock_native_identity("::tcl::binary::encode::hex")
                    .is_none()
            );
            assert!(!vm.namespace_exists("::tcl::binary::encode"), "{dialect}");
        }
    }
}

#[cfg(test)]
mod original_decode_owner_tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;
    use tcl_runtime_api::Code;

    #[test]
    fn original_authored_binary_selector_keeps_guest_word_and_terminal_header_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        assert!(vm.authored_numeric_context().is_some());
        let reached = cmd_binary(&mut vm, &[Value::string("unselected_binary_member")]);
        assert_eq!(reached.code, crate::Code::Error);
        assert!(
            reached
                .result
                .try_to_str()
                .unwrap()
                .contains("unselected_binary_member")
        );
        assert!(vm.execution_refusal.is_none());
        assert!(!vm.current_native_operation_is_entered_for_test());
        let retired = Value::string("unselected_binary_member");
        let original = retired.native_lifetime_lease();
        drop(retired);
        let reached = cmd_binary(&mut vm, std::slice::from_ref(original.value()));
        assert_eq!(reached.code, crate::Code::Error);
        assert!(vm.execution_refusal.is_some());
        assert!(!vm.current_native_operation_is_entered_for_test());
    }

    #[test]
    fn original_binary_decode_selector_keeps_guest_success_and_exact_header_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Real codec adapter and closed VM headers; no extension-object ABI claim.
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"));
        let original = NativeOperationScope::enter(&mut vm).unwrap();
        let completion = binary_decode(
            &mut vm,
            &[Value::string("hex"), Value::string("41")],
            BinaryInvocation::Public,
        );
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion
                .result
                .byte_array_representation()
                .unwrap()
                .as_ref(),
            b"A"
        );
        original.check(&mut vm).unwrap();
        let selector = Value::string("not-a-codec");
        let loan = selector.native_lifetime_lease();
        drop(selector);
        assert!(!loan.value().native_object_is_live());
        let completion = binary_decode(
            &mut vm,
            std::slice::from_ref(loan.value()),
            BinaryInvocation::Public,
        );
        let first = vm.finish_host_execution(completion).unwrap_err();
        assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object header",
                ),
            ),
        );
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(original.check_native(&mut vm).unwrap_err(), first);
        assert!(loan.value().resident_string_bytes().is_some());
    }
}

#[cfg(test)]
mod original_format_scan_tests {
    use super::*;
    use crate::interp::native_operation_currency::NativeOperationScope;
    use tcl_runtime_api::Code;

    #[test]
    fn original_binary_counted_formats_keep_literal_zero_extent_and_reached_scan_assignment() {
        // naming.binary.original-object-format-grammar-extent

        // Actual VM software callers; Native captures are independent evidence.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for format in [b"a1".as_slice(), b"a1\0TAIL", b"a1\xc0\x80TAIL"] {
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(profile),
                );
                assert!(
                    vm.set_var_bytes(b"::errorCode", Value::string("SEEDED"))
                        .is_ok()
                );
                assert!(
                    vm.set_var_bytes(b"assigned", Value::string("SEEDED"))
                        .is_ok()
                );
                let scope = NativeOperationScope::enter(&mut vm).unwrap();
                let original = Value::from_string_bytes(format);
                let format_count = original.native_object_reference_count();
                let completion = binary_format(&mut vm, &[original.clone(), Value::string("AB")]);
                let modified = format.contains(&0xc0);
                assert_eq!(
                    completion.code,
                    if modified { Code::Error } else { Code::Ok },
                    "{profile}"
                );
                if modified {
                    let expected = if profile == "jim" {
                        b"bad field specifier \"\xc0\"".as_slice()
                    } else {
                        b"bad field specifier \"\xc0\x80\"".as_slice()
                    };
                    assert_eq!(
                        completion.result.string_bytes().as_ref(),
                        expected,
                        "{profile}"
                    );
                    assert_eq!(
                        vm.get_var_bytes(b"::errorCode")
                            .unwrap()
                            .string_bytes()
                            .as_ref(),
                        if profile == "jim" {
                            b"SEEDED".as_slice()
                        } else {
                            b"NONE".as_slice()
                        }
                    );
                } else {
                    assert_eq!(
                        value_to_bytes(&mut vm, &completion.result)
                            .unwrap_or_else(|_| panic!("binary result bytes")),
                        b"A"
                    );
                    assert_eq!(
                        vm.get_var_bytes(b"::errorCode")
                            .unwrap()
                            .string_bytes()
                            .as_ref(),
                        b"SEEDED"
                    );
                }
                assert!(
                    vm.set_var_bytes(b"::errorCode", Value::string("SEEDED"))
                        .is_ok()
                );
                let completion = binary_scan(
                    &mut vm,
                    &[
                        Value::string("AB"),
                        original.clone(),
                        Value::string("assigned"),
                    ],
                );
                assert_eq!(
                    completion.code,
                    if modified { Code::Error } else { Code::Ok },
                    "{profile}"
                );
                let assigned = vm.get_var_bytes(b"assigned").unwrap();
                assert_eq!(
                    value_to_bytes(&mut vm, &assigned)
                        .unwrap_or_else(|_| panic!("binary assigned bytes")),
                    b"A"
                );
                if !modified {
                    assert_eq!(completion.result.string_bytes().as_ref(), b"1");
                }
                assert_eq!(original.resident_string_bytes().unwrap().as_ref(), format);
                assert_eq!(original.native_object_reference_count(), format_count);
                scope.check(&mut vm).unwrap();
            }
        }
    }

    #[test]
    fn original_binary_result_recipe_scalar_list_and_cache_issuer_stay_distinct() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.0"));
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let double = Value::native_double(1.25, vm.native_invocation_dialect());
        let formatted = binary_format(
            &mut vm,
            &[
                Value::string("c*d"),
                Value::list(vec![Value::int(65), Value::int(66)]),
                double,
            ],
        );
        assert_eq!(formatted.code, Code::Ok);
        assert_eq!(
            &value_to_bytes(&mut vm, &formatted.result)
                .unwrap_or_else(|_| panic!("binary result bytes"))[..2],
            b"AB"
        );
        let scanned = binary_scan(
            &mut vm,
            &[
                formatted.result.clone(),
                Value::string("a2d"),
                Value::string("letters"),
                Value::string("number"),
            ],
        );
        assert_eq!(scanned.code, Code::Ok);
        assert_eq!(scanned.result.string_bytes().as_ref(), b"2");
        let number = vm.get_var_bytes(b"number").unwrap();
        assert_eq!(
            tcl_syntax::value::ValueOps::as_double(&mut vm, &number),
            Ok(1.25)
        );
        scope.check(&mut vm).unwrap();
        let foreign = Value::from_native_byte_array(
            std::rc::Rc::from(b"AB".as_slice()),
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
        )
        .unwrap();
        let completion = binary_scan(
            &mut vm,
            &[foreign, Value::string("a1"), Value::string("unreached")],
        );
        let first =
            vm.finish_host_execution(completion)
                .map_or_else(core::convert::identity, |value| {
                    drop(value);
                    panic!("original Host refusal")
                });
        assert!(vm.get_var_bytes(b"unreached").is_none());
        assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original binary getter cache issuer"
                )
            )
        );
    }

    #[test]
    fn original_authored_binary_scalar_fronts_keep_none_entry_and_full_header_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        let formatted = binary_format(
            &mut vm,
            &[
                Value::string("c*d"),
                Value::string("65 66"),
                Value::string("1.25"),
            ],
        );
        assert_eq!(formatted.code, Code::Ok);
        let scanned = binary_scan(
            &mut vm,
            &[
                formatted.result,
                Value::string("a2d"),
                Value::string("letters"),
                Value::string("number"),
            ],
        );
        assert_eq!(scanned.code, Code::Ok);
        assert_eq!(scanned.result.string_bytes().as_ref(), b"2");
        assert!(!vm.current_native_operation_is_entered_for_test());
        assert!(vm.execution_refusal.is_none());
        let retired = Value::string("65");
        let held_allocation = retired.native_lifetime_lease();
        drop(retired);
        let original =
            crate::interp::original_authored_operands::OriginalAuthoredOperandScope::capture(
                &mut vm,
            )
            .unwrap();
        let context = vm.authored_numeric_context();
        let conversion = vm
            .native_invocation_dialect()
            .binary_data_conversion()
            .unwrap();
        let narrowing = vm
            .native_invocation_dialect()
            .binary_format_conversion()
            .unwrap();
        let mut ops = OriginalBinaryFormat {
            format: None,
            vm: &mut vm,
            original: &original,
            conversion,
            narrowing,
            syntax: tcl_dialect::NumberSyntax::Tcl84,
            numeric: context,
        };
        let first = tcl_cmd_core::binary::FormatValueOps::binary_format_integer(
            &mut ops,
            held_allocation.value(),
        )
        .unwrap_err();
        assert_eq!(
            first.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert!(!vm.current_native_operation_is_entered_for_test());
        assert!(original.finish(&mut vm).is_err());
    }

    #[test]
    fn original_binary_scan_trace_first_cause_stops_later_assignment() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"));
        vm.register("originalBinaryTrace", |vm, _| {
            vm.refuse_host_command("binary assignment first Host".into())
        });
        assert!(
            vm.add_var_trace_bytes_checked(
                b"first",
                vec!["write".into()],
                &Value::string("originalBinaryTrace"),
                false
            )
            .is_ok()
        );
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let completion = binary_scan(
            &mut vm,
            &[
                Value::string("AB"),
                Value::string("a1a1"),
                Value::string("first"),
                Value::string("later"),
            ],
        );
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(
            vm.finish_host_execution(completion)
                .map_or_else(core::convert::identity, |value| {
                    drop(value);
                    panic!("original Host refusal")
                }),
            first
        );
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert!(vm.get_var_bytes(b"later").is_none());
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
        ));
    }
    #[test]
    fn original_codec_binary_results_keep_recipe_for_format_and_scan_roundtrip() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Actual supported C codec producers and checked consumers, not a Native observation.
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect(profile),
            );
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let protocol = vm.object_materialization().unwrap().protocol();
            let decoded = binary_decode(
                &mut vm,
                &[Value::string("hex"), Value::string("4142")],
                BinaryInvocation::Worker,
            );
            assert_eq!(decoded.code, Code::Ok);
            assert_eq!(decoded.result.native_byte_array_protocol(), Some(protocol));
            assert_eq!(
                decoded.result.byte_array_origin().unwrap().protocol(),
                protocol
            );
            let scanned = binary_scan(
                &mut vm,
                &[
                    decoded.result,
                    Value::string("a1"),
                    Value::string("assigned"),
                ],
            );
            assert_eq!(scanned.code, Code::Ok);
            let assigned = vm.get_var_bytes(b"assigned").unwrap();
            assert_eq!(assigned.byte_array_representation().unwrap().as_ref(), b"A");
            let encoded = binary_encode(
                &mut vm,
                &[Value::string("hex"), Value::string("AB")],
                BinaryInvocation::Worker,
            );
            assert_eq!(encoded.code, Code::Ok);
            assert_eq!(encoded.result.native_byte_array_protocol(), Some(protocol));
            let packed = binary_format(&mut vm, &[Value::string("a*"), encoded.result]);
            assert_eq!(packed.code, Code::Ok);
            assert_eq!(
                packed.result.byte_array_representation().unwrap().as_ref(),
                b"4142"
            );
            assert_eq!(packed.result.native_object_reference_count(), 1);
            assert!(vm.execution_refusal.is_none());
            scope.check(&mut vm).unwrap();
        }
    }
}
