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

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register `binary`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"binary", binary_cmd);
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

fn binary_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"binary subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"format" => binary_format(interp, argv),
        b"scan" => binary_scan(interp, argv),
        b"encode" => binary_encode(interp, argv),
        b"decode" => binary_decode(interp, argv),
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be decode, encode, format, or scan");
            interp.set_error(&m)
        }
    }
}

/// `binary encode hex|base64|uuencode ?options? data` (`BinaryEncodeHex`/
/// `BinaryEncode64`/`BinaryEncodeUu`). The byte codecs are shared with the VM in
/// `tcl_cmd_core::binary`; this adapter handles option parsing + result/error.
fn binary_encode(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"binary encode format ?options? data");
    }
    let fmt = obj_bytes(argv[2]);
    match fmt.as_slice() {
        b"hex" => {
            if argv.len() != 4 {
                return interp.wrong_args(b"binary encode hex data");
            }
            let out = tcl_cmd_core::binary::hex_encode(&obj_bytes(argv[3]));
            interp.set_result_bytes(&out);
            Code::Ok
        }
        b"base64" => binary_encode_wrapped(interp, argv, false),
        b"uuencode" => binary_encode_wrapped(interp, argv, true),
        _ => binary_encode_bad(interp, &fmt),
    }
}

fn binary_encode_bad(interp: &mut Interp, fmt: &[u8]) -> Code {
    let mut m = b"unknown subcommand \"".to_vec();
    m.extend_from_slice(fmt);
    m.extend_from_slice(b"\": must be base64, hex, or uuencode");
    interp.set_error(&m)
}

/// `binary encode base64|uuencode ?-maxlen n? ?-wrapchar c? data`.
fn binary_encode_wrapped(interp: &mut Interp, argv: &[*mut TclObj], uu: bool) -> Code {
    let mut maxlen: usize = if uu { 61 } else { 0 };
    let mut wrapchar: Vec<u8> = b"\n".to_vec();
    let mut i = 3;
    while i < argv.len() - 1 {
        match obj_bytes(argv[i]).as_slice() {
            b"-maxlen" if i + 1 < argv.len() - 1 => {
                match crate::cmd_list::index_spec(&obj_bytes(argv[i + 1]), 0) {
                    Some(n) if n >= 0 => maxlen = n as usize,
                    _ => return interp.set_error(b"line length out of range"),
                }
                i += 2;
            }
            b"-wrapchar" if i + 1 < argv.len() - 1 => {
                wrapchar = obj_bytes(argv[i + 1]);
                i += 2;
            }
            _ => return interp.wrong_args(b"binary encode format ?options? data"),
        }
    }
    if argv.len() - i != 1 {
        return interp.wrong_args(b"binary encode format ?options? data");
    }
    let data = obj_bytes(argv[i]);
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
        return interp.wrong_args(b"binary decode format ?options? data");
    }
    let fmt = obj_bytes(argv[2]);
    match fmt.as_slice() {
        b"hex" => {
            if argv.len() != 4 {
                return interp.wrong_args(b"binary decode hex data");
            }
            match tcl_cmd_core::binary::hex_decode(&obj_bytes(argv[3])) {
                Ok(out) => {
                    interp.set_result_bytes(&out);
                    Code::Ok
                }
                Err(e) => decode_invalid(interp, b"hexadecimal digit", e),
            }
        }
        b"base64" => binary_decode_b64(interp, argv),
        b"uuencode" => binary_decode_uu(interp, argv),
        _ => binary_encode_bad(interp, &fmt),
    }
}

/// `invalid <what> "<byte>" (U+XXXXXX) at position N`, code `TCL BINARY DECODE
/// INVALID` — the decode-failure message shared by `hex` and `base64`.
fn decode_invalid(interp: &mut Interp, what: &[u8], e: tcl_cmd_core::binary::DecodeError) -> Code {
    let mut m = b"invalid ".to_vec();
    m.extend_from_slice(what);
    m.extend_from_slice(b" \"");
    m.push(e.byte);
    m.extend_from_slice(
        format!("\" (U+{:06X}) at position {}", u32::from(e.byte), e.pos).as_bytes(),
    );
    interp.error_with_code(&m, b"TCL BINARY DECODE INVALID")
}

fn binary_decode_b64(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut strict = false;
    let mut i = 3;
    while i < argv.len() - 1 {
        match obj_bytes(argv[i]).as_slice() {
            b"-strict" => {
                strict = true;
                i += 1;
            }
            _ => return interp.wrong_args(b"binary decode format ?options? data"),
        }
    }
    if argv.len() - i != 1 {
        return interp.wrong_args(b"binary decode format ?options? data");
    }
    match tcl_cmd_core::binary::base64_decode(&obj_bytes(argv[i]), strict) {
        Ok(out) => {
            interp.set_result_bytes(&out);
            Code::Ok
        }
        Err(e) => decode_invalid(interp, b"base64 character", e),
    }
}

fn binary_decode_uu(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut i = 3;
    while i < argv.len() - 1 {
        match obj_bytes(argv[i]).as_slice() {
            b"-strict" => i += 1,
            _ => return interp.wrong_args(b"binary decode format ?options? data"),
        }
    }
    if argv.len() - i != 1 {
        return interp.wrong_args(b"binary decode format ?options? data");
    }
    let out = tcl_cmd_core::binary::uu_decode(&obj_bytes(argv[i]));
    interp.set_result_bytes(&out);
    Code::Ok
}

fn binary_format(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"binary format formatString ?arg ...?");
    }
    let fmt = obj_bytes(argv[2]);
    let args: Vec<Vec<u8>> = argv[3..].iter().map(|&a| obj_bytes(a)).collect();
    let refs: Vec<&[u8]> = args.iter().map(Vec::as_slice).collect();
    match tcl_cmd_core::binary::format(&fmt, &refs) {
        Ok(out) => {
            interp.set_result_bytes(&out);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

// -- scan ------------------------------------------------------------------

fn binary_scan(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"binary scan string formatString ?varName ...?");
    }
    let data = obj_bytes(argv[2]);
    let fmt = obj_bytes(argv[3]);
    let vars = &argv[4..];
    // The unpack grammar is shared; the variable assignment (Family-B) stays here.
    let values = match tcl_cmd_core::binary::scan(&data, &fmt) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    for (k, val) in values.iter().enumerate() {
        let Some(&var) = vars.get(k) else {
            return err(interp, b"not enough arguments for all format specifiers");
        };
        let name = obj_bytes(var);
        let o = new_string(val);
        if let Err(e) = interp.var_set(&name, o) {
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
}
