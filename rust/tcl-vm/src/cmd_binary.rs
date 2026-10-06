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

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    if let Some(scripted) = vm.native_invocation_dialect().binary_scripted_ingress() {
        // These are actual extension callbacks, not C private worker identities.
        vm.register(scripted.members[0], binary_format);
        vm.register(scripted.members[1], binary_scan);
        let _ = vm.register_bootstrap_procedure(scripted.name, scripted.parameters, scripted.body);
        return;
    }
    let Some(namespace) = vm
        .native_invocation_dialect()
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
            if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
                vm.remove_registered_command(target.trim_start_matches("::"));
            }
        }
    }
    register(vm);
    if vm
        .native_invocation_dialect()
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
        .map(|word| vm.native_name_operand_bytes(word))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
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
    word: &str,
    prefixes: bool,
    ns: &[u8],
) -> Result<&'static str, String> {
    match tcl_cmd_core::ensemble::resolve_subcommand(subs, word.as_bytes(), prefixes) {
        Some(index) => Ok(subs[index]),
        None => Err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                word.as_bytes(),
                prefixes,
                ns,
            ))
            .into_owned(),
        ),
    }
}

fn cmd_binary(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
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
        match vm.native_static_option_index(sub, subs, false, "option") {
            Ok(index) => subs[index],
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    } else {
        match resolve_binary_sub(subs, &sub.to_str(), true, b"::tcl::binary") {
            Ok(name) => name,
            Err(m) => return err(m),
        }
    };
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
    let Some((fmt, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"binary encode format ?options? data\"",
        );
    };
    let canon = match resolve_binary_sub(
        BINARY_FORMATS,
        &fmt.to_str(),
        false,
        b"::tcl::binary::encode",
    ) {
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
    let mut maxlen: usize = if uu { 61 } else { 0 };
    let mut wrapchar: Vec<u8> = b"\n".to_vec();
    let mut string_wrap = false;
    let mut i = 0;
    while i + 1 < args.len() {
        match &*args[i].to_str() {
            "-maxlen" => match args[i + 1].as_int() {
                Ok(n) if n >= 0 => {
                    maxlen = usize::try_from(n).unwrap_or(usize::MAX);
                    i += 2;
                }
                _ => return err("line length out of range"),
            },
            "-wrapchar" => {
                let Some(policy) = vm.native_invocation_dialect().binary_decode_source() else {
                    return vm.refuse_host_command(
                        "native binary wrap conversion is unresolved".to_owned(),
                    );
                };
                let input = match args[i + 1].binary_decode_input(policy) {
                    Ok(input) => input,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                };
                string_wrap = matches!(
                    input,
                    tcl_registry::native_binary_value::NativeBinaryDecodeInput::String(_)
                );
                wrapchar = input.bytes().to_vec();
                i += 2;
            }
            _ => {
                return binary_wrong_args(
                    vm,
                    invocation,
                    "encode",
                    Some(codec),
                    tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::EncodeWrapped,
                );
            }
        }
    }
    let [data] = &args[i..] else {
        return binary_wrong_args(
            vm,
            invocation,
            "encode",
            Some(codec),
            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::EncodeWrapped,
        );
    };
    let bytes = match value_to_bytes(vm, data) {
        Ok(bytes) => bytes,
        Err(error) => return error,
    };
    let out = if uu {
        tcl_cmd_core::binary::uu_encode(&bytes, maxlen, &wrapchar)
    } else {
        tcl_cmd_core::binary::base64_encode(&bytes, maxlen, &wrapchar)
    };
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
    }
}

/// `binary decode hex|base64|uuencode ?-strict? data`.
fn binary_decode(vm: &mut Vm, rest: &[Value], invocation: BinaryInvocation) -> Completion<Value> {
    use tcl_cmd_core::binary::UuDecodeError;
    let Some((fmt, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"binary decode format ?options? data\"",
        );
    };
    let canon = match resolve_binary_sub(
        BINARY_FORMATS,
        &fmt.to_str(),
        false,
        b"::tcl::binary::decode",
    ) {
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
    let Some((fmt, args)) = rest.split_first() else {
        return binary_wrong_args(
            vm,
            invocation,
            "format",
            None,
            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::Format,
        );
    };
    let fmt_bytes = fmt.to_str();
    match tcl_cmd_core::binary::format_values(vm, fmt_bytes.as_bytes(), args) {
        Ok(out) => ok(bytes_to_value(vm, &out)),
        Err(e) => crate::command::completion_from_cmd_error(vm, e),
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
    let [data_v, fmt_v, vars @ ..] = rest else {
        return binary_wrong_args(
            vm,
            invocation,
            "scan",
            None,
            tcl_registry::native_binary_usage::NativeBinaryArgumentUsage::Scan,
        );
    };
    let data = match value_to_bytes(vm, data_v) {
        Ok(bytes) => bytes,
        Err(error) => return error,
    };
    let fmt = fmt_v.to_str();
    // The unpack grammar is shared; the variable assignment stays here.
    let values = match tcl_cmd_core::binary::scan_values(&data, fmt.as_bytes()) {
        Ok(v) => v,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    for (k, val) in values.iter().enumerate() {
        let Some(var) = vars.get(k) else {
            return err("not enough arguments for all format specifiers");
        };
        let name = var.to_str();
        let value = match val {
            tcl_cmd_core::binary::ScanValue::Bytes(bytes) => bytes_to_value(vm, bytes),
            tcl_cmd_core::binary::ScanValue::Double(value) => {
                Value::native_double(*value, vm.native_invocation_dialect())
            }
            tcl_cmd_core::binary::ScanValue::Doubles(values) => Value::list(
                values
                    .iter()
                    .map(|value| Value::native_double(*value, vm.native_invocation_dialect()))
                    .collect(),
            ),
        };
        if let Err(c) = vm.var_set(&name, value) {
            return c;
        }
    }
    ok(Value::int(i64::try_from(values.len()).unwrap_or(i64::MAX)))
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
            let mut vm = Vm::new();
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
