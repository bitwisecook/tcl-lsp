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
    vm.register("binary", cmd_binary);
}

/// Encode a byte slice as a Tcl byte-array string (one scalar per byte).
fn bytes_to_value(bytes: &[u8]) -> Value {
    Value::string(bytes.iter().map(|&b| char::from(b)).collect::<String>())
}

/// Decode a string back to bytes, taking the low 8 bits of each scalar.
#[allow(clippy::cast_possible_truncation)] // Keeping the low 8 bits (`as u8`) is the intended byte decode.
fn value_to_bytes(v: &Value) -> Vec<u8> {
    v.to_str().chars().map(|c| c as u32 as u8).collect()
}

/// `binary`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
const BINARY_SUBS: &[&str] = &["decode", "encode", "format", "scan"];

/// `binary encode`/`binary decode`'s format set. These two are ensembles with
/// **`-prefixes` off** (`TclMakeEnsemble` with `TCL_ENSEMBLE_PREFIX` cleared),
/// so nothing abbreviates and the miss is worded `unknown subcommand`, never
/// `unknown or ambiguous` (tclsh: `binary encode h a` → `unknown subcommand
/// "h": must be base64, hex, or uuencode`).
const BINARY_FORMATS: &[&str] = &["base64", "hex", "uuencode"];

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
        return err("wrong # args: should be \"binary subcommand ?arg ...?\"");
    };
    // `encode`/`decode` (TIP 317) arrive in 8.6: under an earlier pin `binary
    // d` must not resolve to `decode`.
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "binary",
        BINARY_SUBS,
    );
    let canon = match resolve_binary_sub(subs, &sub.to_str(), true, b"::tcl::binary") {
        Ok(name) => name,
        Err(m) => return err(m),
    };
    match canon {
        "format" => binary_format(rest),
        "scan" => binary_scan(vm, rest),
        "encode" => binary_encode(rest),
        // Unreachable: `BINARY_SUBS` has exactly the four arms here.
        _ => binary_decode(rest),
    }
}

/// `binary encode hex|base64|uuencode ?options? data`. The byte codecs live in
/// the shared `tcl_cmd_core::binary`; bytes cross the VM's value boundary via the
/// byte-array convention (`value_to_bytes`/`bytes_to_value`).
fn binary_encode(rest: &[Value]) -> Completion<Value> {
    let Some((fmt, args)) = rest.split_first() else {
        return err("wrong # args: should be \"binary encode format ?options? data\"");
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
                return err("wrong # args: should be \"binary encode hex data\"");
            };
            let out = tcl_cmd_core::binary::hex_encode(&value_to_bytes(data));
            ok(bytes_to_value(&out))
        }
        "base64" => binary_encode_wrapped(args, false),
        // Unreachable: `BINARY_FORMATS` has exactly the three arms here.
        _ => binary_encode_wrapped(args, true),
    }
}

/// `binary encode base64|uuencode ?-maxlen n? ?-wrapchar c? data`.
fn binary_encode_wrapped(args: &[Value], uu: bool) -> Completion<Value> {
    let mut maxlen: usize = if uu { 61 } else { 0 };
    let mut wrapchar: Vec<u8> = b"\n".to_vec();
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
                wrapchar = value_to_bytes(&args[i + 1]);
                i += 2;
            }
            _ => return err("wrong # args: should be \"binary encode format ?options? data\""),
        }
    }
    let [data] = &args[i..] else {
        return err("wrong # args: should be \"binary encode format ?options? data\"");
    };
    let bytes = value_to_bytes(data);
    let out = if uu {
        tcl_cmd_core::binary::uu_encode(&bytes, maxlen, &wrapchar)
    } else {
        tcl_cmd_core::binary::base64_encode(&bytes, maxlen, &wrapchar)
    };
    ok(bytes_to_value(&out))
}

/// `binary decode hex|base64|uuencode ?-strict? data`.
fn binary_decode(rest: &[Value]) -> Completion<Value> {
    let Some((fmt, args)) = rest.split_first() else {
        return err("wrong # args: should be \"binary decode format ?options? data\"");
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
    match canon {
        "hex" => {
            let [data] = args else {
                return err("wrong # args: should be \"binary decode hex data\"");
            };
            match tcl_cmd_core::binary::hex_decode(&value_to_bytes(data)) {
                Ok(out) => ok(bytes_to_value(&out)),
                Err(e) => decode_invalid("hexadecimal digit", e),
            }
        }
        "base64" => match decode_args(args) {
            Ok((strict, bytes)) => match tcl_cmd_core::binary::base64_decode(&bytes, strict) {
                Ok(out) => ok(bytes_to_value(&out)),
                Err(e) => decode_invalid("base64 character", e),
            },
            Err(c) => c,
        },
        // Unreachable: `BINARY_FORMATS` has exactly the three arms here.
        _ => match decode_args(args) {
            Ok((_strict, bytes)) => ok(bytes_to_value(&tcl_cmd_core::binary::uu_decode(&bytes))),
            Err(c) => c,
        },
    }
}

/// Parse a decode subcommand's `?-strict? data` → `(strict, bytes)`.
fn decode_args(args: &[Value]) -> Result<(bool, Vec<u8>), Completion<Value>> {
    let mut strict = false;
    let mut i = 0;
    while i + 1 < args.len() {
        match &*args[i].to_str() {
            "-strict" => {
                strict = true;
                i += 1;
            }
            _ => {
                return Err(err(
                    "wrong # args: should be \"binary decode format ?options? data\"",
                ));
            }
        }
    }
    let [data] = &args[i..] else {
        return Err(err(
            "wrong # args: should be \"binary decode format ?options? data\"",
        ));
    };
    Ok((strict, value_to_bytes(data)))
}

/// `invalid <what> "<byte>" (U+XXXXXX) at position N`, code `TCL BINARY DECODE
/// INVALID` — the shared decode-failure form for hex and base64.
fn decode_invalid(what: &str, e: tcl_cmd_core::binary::DecodeError) -> Completion<Value> {
    let msg = format!(
        "invalid {what} \"{}\" (U+{:06X}) at position {}",
        e.byte as char,
        u32::from(e.byte),
        e.pos
    );
    crate::command::err_with_code(msg, "TCL BINARY DECODE INVALID")
}

/// Width in bytes for an integer specifier.
fn binary_format(rest: &[Value]) -> Completion<Value> {
    let Some((fmt, args)) = rest.split_first() else {
        return err("wrong # args: should be \"binary format formatString ?arg ...?\"");
    };
    let fmt_bytes = value_to_bytes(fmt);
    let arg_bytes: Vec<Vec<u8>> = args.iter().map(value_to_bytes).collect();
    let refs: Vec<&[u8]> = arg_bytes.iter().map(Vec::as_slice).collect();
    match tcl_cmd_core::binary::format(&fmt_bytes, &refs) {
        Ok(out) => ok(bytes_to_value(&out)),
        Err(e) => err(e.message().to_string()),
    }
}

fn binary_scan(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [data_v, fmt_v, vars @ ..] = rest else {
        return err("wrong # args: should be \"binary scan string formatString ?varName ...?\"");
    };
    let data = value_to_bytes(data_v);
    let fmt = value_to_bytes(fmt_v);
    // The unpack grammar is shared; the variable assignment stays here.
    let values = match tcl_cmd_core::binary::scan(&data, &fmt) {
        Ok(v) => v,
        Err(e) => return err(e.message().to_string()),
    };
    for (k, val) in values.iter().enumerate() {
        let Some(var) = vars.get(k) else {
            return err("not enough arguments for all format specifiers");
        };
        let name = var.to_str();
        if let Err(c) = vm.var_set(&name, bytes_to_value(val)) {
            return c;
        }
    }
    ok(Value::int(i64::try_from(values.len()).unwrap_or(i64::MAX)))
}
