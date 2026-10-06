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

//! `binary format` / `binary scan` (C ref `tclBinary.c`).
//!
//! Converts between Tcl values and binary byte strings via a format string of
//! `<type><count>` fields. Implemented type codes (both directions):
//!
//! - `a`/`A` — bytes, null-/space-padded
//! - `b`/`B` — binary-digit string (low-to-high / high-to-low within a byte)
//! - `h`/`H` — hex-digit string (low / high nibble first)
//! - `c` — 8-bit ints; `s`/`S`/`t`, `i`/`I`/`n`, `w`/`W`/`m` — 16/32/64-bit
//!   ints (little / big / native endian); `f`/`r`/`R` 32-bit, `d`/`q`/`Q`
//!   64-bit floats
//! - `x` skip/zero, `X` back up, `@` absolute position
//!
//! `count` is a number or `*` (all); native endian is little (the wasm/x86
//! target). Also: the `u` unsigned scan modifier, and `binary encode`/`decode`
//! (`hex`/`base64`/`uuencode`). Verified against tclsh 9.0.

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use tcl_registry::native_binary_usage::NativeBinaryArgumentUsage;

/// Register `binary`.
pub fn install(interp: &mut Interp) {
    const NAMES: &[&[u8]] = &[
        b"decode".as_slice(),
        b"encode".as_slice(),
        b"format".as_slice(),
        b"scan".as_slice(),
    ];
    let admitted = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "binary",
        NAMES,
    );
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
        b"binary",
        binary_cmd,
        STOCK_MEMBERS,
        admitted,
    );
    if interp
        .native_invocation_dialect()
        .ensemble_implementation_namespace(
            tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
        )
        .is_some()
    {
        interp.register_stock_nested_ensemble(
            tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
            b"::tcl::binary::encode",
            stock_encode,
            &[
                (b"hex", stock_encode_hex),
                (b"base64", stock_encode_base64),
                (b"uuencode", stock_encode_uuencode),
            ],
        );
        interp.register_stock_nested_ensemble(
            tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
            b"::tcl::binary::decode",
            stock_decode,
            &[
                (b"hex", stock_decode_hex),
                (b"base64", stock_decode_base64),
                (b"uuencode", stock_decode_uuencode),
            ],
        );
    }
    if let Some(ingress) = interp.native_invocation_dialect().binary_scripted_ingress() {
        interp.install_stock_scripted_binary(ingress, &[stock_format, stock_scan]);
    }
}

const STOCK_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
    (b"decode", stock_decode),
    (b"encode", stock_encode),
    (b"format", stock_format),
    (b"scan", stock_scan),
];

fn stock_decode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"decode"], binary_cmd)
}

fn stock_encode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"encode"], binary_cmd)
}

fn stock_format(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"format"], binary_cmd)
}

fn stock_scan(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"scan"], binary_cmd)
}

fn stock_encode_hex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"encode", b"hex"], binary_cmd)
}

fn stock_encode_base64(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"encode", b"base64"], binary_cmd)
}

fn stock_encode_uuencode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"encode", b"uuencode"], binary_cmd)
}

fn stock_decode_hex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"decode", b"hex"], binary_cmd)
}

fn stock_decode_base64(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"decode", b"base64"], binary_cmd)
}

fn stock_decode_uuencode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"binary", b"decode", b"uuencode"], binary_cmd)
}

fn binary_wrong_args(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    prefix_words: usize,
    operation: NativeBinaryArgumentUsage,
) -> Code {
    let suffix = interp
        .native_invocation_dialect()
        .binary_argument_usage(operation)
        .unwrap_or(operation.compatibility_usage());
    interp.wrong_args_for_prefix(argv, prefix_words, suffix.as_bytes())
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

/// `binary`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// `encode`/`decode` (TIP 317) arrive in 8.6, so the table is filtered to the
/// emulated release before the scan — under an 8.5 pin `binary d` is not
/// `decode`, it is a miss over `format, or scan`.
const BINARY_SUBS: &[&[u8]] = &[b"decode", b"encode", b"format", b"scan"];

/// `binary encode`/`binary decode`'s format set. These two are ensembles with
/// **`-prefixes` off**, so nothing abbreviates and the miss is worded
/// `unknown subcommand`, never `unknown or ambiguous` (tclsh: `binary encode
/// h a` → `unknown subcommand "h": must be base64, hex, or uuencode`).
const BINARY_FORMATS: &[&[u8]] = &[b"base64", b"hex", b"uuencode"];

fn binary_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args_for_invocation(argv, b"subcommand ?arg ...?");
    }
    let word = obj_bytes(argv[1]);
    let subs = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "binary",
        BINARY_SUBS,
    );
    let Some(protocol) = interp.native_invocation_dialect().binary_root_dispatch() else {
        return interp.error(b"native binary root protocol required");
    };
    let index = match protocol {
        tcl_registry::native_binary_usage::NativeBinaryRootDispatch::Ensemble => {
            let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, &word, true) else {
                return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                    subs,
                    &word,
                    true,
                    b"::tcl::binary",
                ));
            };
            index
        }
        tcl_registry::native_binary_usage::NativeBinaryRootDispatch::Indexed
        | tcl_registry::native_binary_usage::NativeBinaryRootDispatch::Scripted => {
            // Scripted ingress reaches only its separately registered helpers;
            // their immutable parser prefix selects this internal parse view.
            match interp.native_static_option_index(argv[1], subs, false, "option") {
                Ok(index) => index,
                Err(error) => return interp.report_cmd_error(error),
            }
        }
    };
    match subs[index] {
        b"format" => binary_format(interp, argv),
        b"scan" => binary_scan(interp, argv),
        b"encode" => binary_encode(interp, argv),
        // Unreachable: `BINARY_SUBS` has exactly the four arms here.
        _ => binary_decode(interp, argv),
    }
}

/// `binary encode hex|base64|uuencode ?options? data` (`BinaryEncodeHex`/
/// `BinaryEncode64`/`BinaryEncodeUu`). The byte codecs are shared with the VM in
/// `tcl_cmd_core::binary`; this adapter handles option parsing + result/error.
fn binary_encode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"subcommand ?arg ...?");
    }
    let fmt = obj_bytes(argv[2]);
    match fmt.as_slice() {
        b"hex" => {
            if argv.len() != 4 {
                return binary_wrong_args(interp, argv, 3, NativeBinaryArgumentUsage::EncodeHex);
            }
            let data = match interp.binary_bytes(argv[3]) {
                Ok(data) => data,
                Err(code) => return code,
            };
            let out = tcl_cmd_core::binary::hex_encode(&data);
            interp.set_result_bytes(&out);
            Code::Ok
        }
        b"base64" => binary_encode_wrapped(interp, argv, false),
        b"uuencode" => binary_encode_wrapped(interp, argv, true),
        _ => binary_encode_bad(interp, &fmt),
    }
}

fn binary_encode_bad(interp: &mut Interp, fmt: &[u8]) -> Code {
    interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
        BINARY_FORMATS,
        fmt,
        false,
        b"::tcl::binary::encode",
    ))
}

/// `binary encode base64|uuencode ?-maxlen n? ?-wrapchar c? data`.
fn binary_encode_wrapped(interp: &mut Interp, argv: &[*mut TclObj], uu: bool) -> Code {
    let mut maxlen: usize = if uu { 61 } else { 0 };
    let mut wrapchar: Vec<u8> = b"\n".to_vec();
    let mut i = 3;
    while i < argv.len() - 1 {
        match obj_bytes(argv[i]).as_slice() {
            b"-maxlen" if i + 1 < argv.len() - 1 => {
                match crate::cmd_list::index_spec(interp, &obj_bytes(argv[i + 1]), 0) {
                    Some(n) if n >= 0 => maxlen = n as usize,
                    _ => return interp.set_error(b"line length out of range"),
                }
                i += 2;
            }
            b"-wrapchar" if i + 1 < argv.len() - 1 => {
                wrapchar = obj_bytes(argv[i + 1]);
                i += 2;
            }
            _ => {
                return binary_wrong_args(
                    interp,
                    argv,
                    3,
                    NativeBinaryArgumentUsage::EncodeWrapped,
                );
            }
        }
    }
    if argv.len() - i != 1 {
        return binary_wrong_args(interp, argv, 3, NativeBinaryArgumentUsage::EncodeWrapped);
    }
    let data = match interp.binary_bytes(argv[i]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let out = if uu {
        tcl_cmd_core::binary::uu_encode(&data, maxlen, &wrapchar)
    } else {
        tcl_cmd_core::binary::base64_encode(&data, maxlen, &wrapchar)
    };
    interp.set_result_bytes(&out);
    Code::Ok
}

/// `binary decode hex|base64|uuencode ?options? string`. The byte codecs are
/// shared in `tcl_cmd_core::binary`; this adapter handles options + errors.
fn binary_decode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"subcommand ?arg ...?");
    }
    let codec = obj_bytes(argv[2]);
    if !BINARY_FORMATS.contains(&codec.as_slice()) {
        return binary_encode_bad(interp, &codec);
    }
    let operands = &argv[3..];
    let option = (operands.len() == 2).then(|| obj_bytes(operands[0]));
    let layout =
        match tcl_cmd_core::binary::decode_argument_layout(operands.len(), option.as_deref()) {
            Ok(layout) => layout,
            Err(tcl_cmd_core::binary::DecodeArgumentsError::UnknownOption(word)) => {
                let failure =
                    tcl_cmd_core::binary::DecodeArgumentsError::UnknownOption(word.clone());
                let message = failure.option_message().expect("actual unknown switch");
                let code = crate::interp::error_code_list(&[
                    b"TCL", b"LOOKUP", b"INDEX", b"option", &word,
                ]);
                return interp.error_with_code(&message, &code);
            }
            Err(_) => return binary_wrong_args(interp, argv, 3, NativeBinaryArgumentUsage::Decode),
        };
    let Some(policy) = interp.native_invocation_dialect().binary_decode_source() else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "binary decoder source",
            ),
        );
    };
    let recipe = match interp.byte_array_string_recipe() {
        Ok(recipe) => recipe,
        Err(code) => return code,
    };
    let input = crate::bytearray::native_decode_input(operands[layout.data], policy, recipe);
    let decoded = match codec.as_slice() {
        b"hex" => tcl_cmd_core::binary::hex_decode_with_strict(input.bytes(), layout.strict),
        b"base64" => tcl_cmd_core::binary::base64_decode(input.bytes(), layout.strict),
        _ => match tcl_cmd_core::binary::uu_decode_with_strict(input.bytes(), layout.strict) {
            Ok(bytes) => Ok(bytes),
            Err(tcl_cmd_core::binary::UuDecodeError::Invalid(error)) => Err(error),
            Err(tcl_cmd_core::binary::UuDecodeError::Short) => {
                return interp.error_with_code(b"short uuencode data", b"TCL BINARY DECODE SHORT");
            }
        },
    };
    match decoded {
        Ok(bytes) => interp.set_result_byte_array(&bytes),
        Err(error) => {
            let what = match codec.as_slice() {
                b"hex" => "hexadecimal digit",
                b"base64" => "base64 character",
                _ => "uuencode character",
            };
            let Some(message) = interp
                .native_invocation_dialect()
                .binary_decode_error(what, &input, error.pos)
            else {
                return interp.error(b"native binary decoder presentation required");
            };
            interp.error_with_code(message.as_bytes(), b"TCL BINARY DECODE INVALID")
        }
    }
}

fn binary_format(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return binary_wrong_args(interp, argv, 2, NativeBinaryArgumentUsage::Format);
    }
    let fmt = obj_bytes(argv[2]);
    match tcl_cmd_core::binary::format_values(interp, &fmt, &argv[3..]) {
        Ok(out) => {
            if interp
                .native_invocation_dialect()
                .binary_format_conversion()
                == Some(tcl_registry::native_binary_value::NativeBinaryByteConversion::Utf8)
            {
                // Jim's compound format procedure returns raw string bytes;
                // C's byte-array Unicode stringification is a separate type.
                interp.set_result_bytes(&out);
            } else {
                return interp.set_result_byte_array(&out);
            }
            Code::Ok
        }
        Err(e) => interp.report_cmd_error(e),
    }
}

impl tcl_cmd_core::binary::FormatValueOps for Interp {
    type Value = *mut TclObj;

    fn binary_format_bytes(
        &mut self,
        value: &Self::Value,
    ) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
        let policy = self
            .native_invocation_dialect()
            .binary_format_conversion()
            .ok_or_else(|| {
                tcl_cmd_core::CmdError::from(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "binary format conversion",
                    ),
                )
            })?;
        // Tcl9 format narrows a copy; encode/scan instead perform checked conversion.
        let cache = self.native_invocation_dialect().binary_data_conversion() == Some(policy);
        self.native_binary_bytes_with(*value, policy, cache)
            .map_err(|_| {
                if let Some(refusal) = self.native_access_refusal() {
                    return tcl_cmd_core::CmdError::from(refusal);
                }
                tcl_cmd_core::CmdError::with_error_code_bytes(
                    self.result_bytes(),
                    self.error_code(),
                )
            })
    }

    fn binary_format_integer(
        &mut self,
        value: &Self::Value,
    ) -> Result<i64, tcl_cmd_core::CmdError> {
        let syntax = self.native_invocation_dialect().numbers;
        tcl_cmd_core::binary::integer_value(self, value, syntax)
    }

    fn binary_format_double(&mut self, value: &Self::Value) -> Result<f64, tcl_cmd_core::CmdError> {
        Ok(tcl_syntax::value::ValueOps::as_double(self, value)?)
    }

    fn binary_format_elements(
        &mut self,
        value: &Self::Value,
    ) -> Result<Vec<Self::Value>, tcl_cmd_core::CmdError> {
        Ok(tcl_syntax::value::ValueOps::list_elements(self, value)?)
    }
}

// scan

fn binary_scan(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return binary_wrong_args(interp, argv, 2, NativeBinaryArgumentUsage::Scan);
    }
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let fmt = obj_bytes(argv[3]);
    let vars = &argv[4..];
    // The unpack grammar is shared; the variable assignment (Family-B) stays here.
    let values = match tcl_cmd_core::binary::scan_values(&data, &fmt) {
        Ok(v) => v,
        Err(e) => return interp.report_cmd_error(e),
    };
    for (k, val) in values.iter().enumerate() {
        let Some(&var) = vars.get(k) else {
            return err(interp, b"not enough arguments for all format specifiers");
        };
        let name = obj_bytes(var);
        // `arr(a)` writes the array *element*, not a literal scalar named
        // `arr(a)` — the same `split_array_ref` +
        // `var_set`/`var_set_elem` routing `set` uses, so this doesn't
        // hand-roll a second name parser.
        let (base, elem) = crate::frame::split_array_ref(&name);
        let o = match val {
            tcl_cmd_core::binary::ScanValue::Bytes(bytes) => {
                if interp.native_invocation_dialect().binary_data_conversion()
                    == Some(tcl_registry::native_binary_value::NativeBinaryByteConversion::Utf8)
                {
                    obj::new_string_bytes(bytes)
                } else {
                    match interp.new_native_byte_array(bytes) {
                        Ok(value) => value,
                        Err(code) => return code,
                    }
                }
            }
            tcl_cmd_core::binary::ScanValue::Double(value) => obj::new_double_obj(*value),
            tcl_cmd_core::binary::ScanValue::Doubles(values) => {
                let elements: Vec<_> = values
                    .iter()
                    .map(|value| obj::new_double_obj(*value))
                    .collect();
                interp.new_list_object(&elements)
            }
        };
        let stored = match &elem {
            Some(k) => interp.var_set_elem(&base, k, o),
            None => interp.var_set(&base, o),
        };
        if let Err(e) = stored {
            crate::interp::drop_fresh(o);
            return crate::builtins::var_error(interp, &name, e);
        }
    }
    interp.set_result(obj::new_wide_int_obj(
        i64::try_from(values.len()).unwrap_or(i64::MAX),
    ));
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(counters::finalize(), 0, "residual objs/bufs");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} -> {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    fn compare_native_binary(path: &std::path::Path, dialect: &str, sources: &[&str]) {
        for source in sources {
            let oracle =
                tcl_test_support::run_script(path, format!("puts [{source}]\n").as_bytes())
                    .unwrap();
            assert!(
                oracle.success() && oracle.stderr.is_empty(),
                "{dialect}: {source}: {:?}",
                oracle.stderr
            );
            let mut interpreter = Interp::new();
            interpreter.set_dialect_profile(crate::environment::profile_for_dialect(dialect));
            assert_eq!(
                interpreter.eval_str(source.as_bytes()),
                Code::Ok,
                "{dialect}: {source}"
            );
            let mut actual = interpreter.result_bytes();
            actual.push(b'\n');
            assert_eq!(actual, oracle.stdout, "{dialect}: {source}");
        }
    }

    #[test]
    fn binary_objects_and_decoders_follow_actual_native_input_policies() {
        use tcl_test_support::binary_values::{BINARY_DECODER_SCRIPTS, BINARY_VALUE_SCRIPTS};
        for reference in tcl_test_support::available_tclshs() {
            let dialect = format!("tcl{}", reference.version.version_string());
            compare_native_binary(&reference.path, &dialect, BINARY_VALUE_SCRIPTS);
            if tcl_registry::InvocationDialect::for_version(reference.version)
                .binary_root_dispatch()
                == Some(tcl_registry::native_binary_usage::NativeBinaryRootDispatch::Indexed)
            {
                for source in BINARY_VALUE_SCRIPTS.iter().skip(2).take(6) {
                    let with_code = format!("{source}; list $c $r $::errorCode");
                    compare_native_binary(&reference.path, &dialect, &[with_code.as_str()]);
                }
            }
            if reference.version >= tcl_dialect::TclVersion::V8_6 {
                compare_native_binary(&reference.path, &dialect, BINARY_DECODER_SCRIPTS);
            }
        }
        if let Some(reference) = tcl_test_support::locate_jimsh().expect("validated Jim override") {
            compare_native_binary(&reference.path, "jim", BINARY_VALUE_SCRIPTS);
        }
    }

    #[test]
    fn binary_usage_retains_actual_names_and_native_prefix_quoting() {
        use tcl_dialect::TclVersion;

        for (version, expected) in [
            (
                TclVersion::V8_4,
                b"binary name format formatString ?arg arg ...?".as_slice(),
            ),
            (
                TclVersion::V8_5,
                b"binary name format formatString ?arg arg ...?".as_slice(),
            ),
            (
                TclVersion::V8_6,
                b"binary name format formatString ?arg ...?".as_slice(),
            ),
            (
                TclVersion::V9_0,
                b"{binary name} format formatString ?arg ...?".as_slice(),
            ),
            (
                TclVersion::V9_1,
                b"{binary name} format formatString ?arg ...?".as_slice(),
            ),
        ] {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                assert_eq!(interp.eval_str(b"rename binary {binary name}"), Code::Ok);
                assert_eq!(interp.eval_str(b"{binary name} format"), Code::Error);
                let mut message = b"wrong # args: should be \"".to_vec();
                message.extend_from_slice(expected);
                message.push(b'"');
                assert_eq!(interp.result_bytes(), message, "{version:?}");
            });
        }
    }

    #[test]
    fn actual_usage_rewrite_retains_byte_values_and_requires_the_full_prefix() {
        leak_free(|interp| {
            interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"private"));
            let member = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"codec"));
            let arguments = [head.as_ptr(), member.as_ptr()];
            assert!(interp.begin_ensemble_rewrite(
                vec![
                    crate::obj::Owned::fresh(crate::interp::new_string(&[0xff, b' '])),
                    crate::obj::Owned::fresh(crate::interp::new_string(b"hex"))
                ],
                2,
                2
            ));
            assert_eq!(
                interp.wrong_args_for_prefix(&arguments, 2, b"data"),
                Code::Error
            );
            assert_eq!(
                interp.result_bytes(),
                b"wrong # args: should be \"{\xff } hex data\""
            );
            assert_eq!(
                interp.wrong_args_for_prefix(&arguments, 1, b"data"),
                Code::Error
            );
            assert_eq!(
                interp.result_bytes(),
                b"wrong # args: should be \"private data\""
            );
            interp.clear_ensemble_rewrite();
        });
    }

    /// `binary` is a `TclMakeEnsemble` command, while
    /// `binary encode`/`binary decode` are ensembles with **`-prefixes` off**
    /// — nothing abbreviates there and the miss is worded `unknown
    /// subcommand`, never `unknown or ambiguous`. All three read their scan
    /// and miss sentence from `tcl_cmd_core::ensemble` rather than matching
    /// exactly and spelling the sentence by hand.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   binary e hex a       -> 61
    ///   binary {}            -> unknown or ambiguous subcommand "": must be
    ///                           decode, encode, format, or scan
    ///   binary encode h a    -> unknown subcommand "h": must be base64, hex,
    ///                           or uuencode
    ///   binary decode b YQ== -> unknown subcommand "b": must be <same>
    #[test]
    fn binary_ensembles_resolve_like_tclsh() {
        const FORMATS: &str = "must be base64, hex, or uuencode";
        leak_free(|i| {
            let err_of = |i: &mut Interp, src: &[u8]| {
                assert_eq!(i.eval_str(src), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(ok(i, b"binary e hex a"), b"61");
            assert_eq!(ok(i, b"binary en hex a"), b"61");
            assert_eq!(
                err_of(i, b"binary {}"),
                "unknown or ambiguous subcommand \"\": must be decode, encode, format, or scan"
            );
            assert_eq!(
                err_of(i, b"binary encode h a"),
                format!("unknown subcommand \"h\": {FORMATS}")
            );
            assert_eq!(
                err_of(i, b"binary encode {} a"),
                format!("unknown subcommand \"\": {FORMATS}")
            );
            assert_eq!(
                err_of(i, b"binary decode b YQ=="),
                format!("unknown subcommand \"b\": {FORMATS}")
            );
        });
    }

    #[test]
    fn format_and_scan_roundtrip() {
        // Hex-dump helper round-trips through `binary scan H*`.
        leak_free(|i| {
            // format → hex (verified vs tclsh 9.0)
            i.eval_str(b"binary scan [binary format a5 foo] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"666f6f0000");
            i.eval_str(b"binary scan [binary format A5 foo] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"666f6f2020");
            i.eval_str(b"binary scan [binary format B8 01001101] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"4d");
            i.eval_str(b"binary scan [binary format b8 01001101] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"b2");
            i.eval_str(b"binary scan [binary format H2 4d] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"4d");
            i.eval_str(b"binary scan [binary format s 258] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"0201");
            i.eval_str(b"binary scan [binary format I 258] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"00000102");
            i.eval_str(b"binary scan [binary format c3 {1 2 3}] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"010203");
            i.eval_str(b"unset ::h");
        });
    }

    #[test]
    fn encode_decode_and_unsigned_scan() {
        leak_free(|i| {
            assert_eq!(ok(i, b"binary encode hex Hello"), b"48656c6c6f");
            assert_eq!(ok(i, b"binary decode hex 48656c6c6f"), b"Hello");
            assert_eq!(
                ok(i, b"binary encode base64 {Hello, World!}"),
                b"SGVsbG8sIFdvcmxkIQ=="
            );
            assert_eq!(
                ok(i, b"binary decode base64 SGVsbG8sIFdvcmxkIQ=="),
                b"Hello, World!"
            );
            assert_eq!(
                ok(i, b"binary decode uuencode [binary encode uuencode Cat]"),
                b"Cat"
            );
            // unsigned scan modifier.
            assert_eq!(
                ok(i, b"binary scan [binary format c -128] cu v; set v"),
                b"128"
            );
            assert_eq!(
                ok(i, b"binary scan [binary format s -1] su v; set v"),
                b"65535"
            );
        });
    }

    #[test]
    fn scan_values_and_count() {
        leak_free(|i| {
            // single int (no count) → the value directly
            assert_eq!(
                ok(i, b"binary scan [binary format i 258] i v; set v"),
                b"258"
            );
            // count → a list
            assert_eq!(
                ok(i, b"binary scan [binary format c3 {1 2 3}] c3 v; set v"),
                b"1 2 3"
            );
            // signed 8-bit: 0xff → -1
            assert_eq!(
                ok(i, b"binary scan [binary format c 255] c v; set v"),
                b"-1"
            );
            // return value = number of conversions
            assert_eq!(ok(i, b"binary scan abc a2a1 x y"), b"2");
            assert_eq!(ok(i, b"set x"), b"ab");
            // `a*` takes the rest
            assert_eq!(ok(i, b"binary scan abcdef a* v; set v"), b"abcdef");
            i.eval_str(b"unset -nocomplain v x y");
        });
    }

    /// C Tcl 8.6 and 9.0 agree that a byte-array has a Unicode string view,
    /// but they intentionally differ when that view is converted back to bytes
    /// after case mapping. This is the TP/FP/TN/FN matrix for the dual-port
    /// representation, pinned against both installed C oracles.
    #[test]
    fn byte_array_string_shimmer_uses_the_release_selected_byte_policy() {
        use tcl_dialect::TclVersion;

        // TP: Tcl 8's legacy conversion keeps the low byte of U+0178 (Ÿ).
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(
                ok(
                    i,
                    br#"binary encode hex [string toupper [binary format H* 41ff42]]"#,
                ),
                b"417842"
            );
            // TN: ASCII never reaches the version-specific boundary.
            assert_eq!(
                ok(
                    i,
                    br#"binary encode hex [string toupper [binary format H* 4162]]"#
                ),
                b"4142"
            );
        });

        // FN: Tcl 9 must not silently preserve or truncate the wide character.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(
                i.eval_str(br#"binary encode hex [string toupper [binary format H* 41ff42]]"#),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"expected code point values below 0xff but value at byte offset 1 was 0x178"
            );
            assert_eq!(i.eval_str(b"set ::errorCode"), Code::Ok);
            assert_eq!(i.result_bytes(), b"TCL VALUE BYTES");

            // FP: a real Unicode character inside the byte domain remains a
            // normal string and converts correctly; it is not confused with a
            // raw byte solely because both spellings look Latin-1-like.
            assert_eq!(
                ok(i, "binary encode hex [string toupper café]".as_bytes()),
                b"434146c9"
            );
            // `tolower` stays within U+00FF for the byte-array case and must
            // succeed on Tcl 9 as well.
            assert_eq!(
                ok(
                    i,
                    br#"binary encode hex [string tolower [binary format H* 41ff42]]"#,
                ),
                b"61ff62"
            );
        });
    }
}
