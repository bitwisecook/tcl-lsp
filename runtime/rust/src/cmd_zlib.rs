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

//! `zlib` — checksums (`crc32`/`adler32`) and DEFLATE (de)compression with the
//! zlib (RFC 1950), gzip (RFC 1952), and raw-deflate (RFC 1951) framings.
//!
//! The DEFLATE codec is [`flate2`] on its pure-Rust `miniz_oxide` backend (no C
//! `libz`), so this builds for the `wasm32` cdylib as well as native. The
//! checksums are computed here directly (they double as the `crc32`/`adler32`
//! subcommands and are tiny). Streaming (`zlib stream`) and channel stacking
//! (`zlib push`) are *not* supported under the tree-walking / WASM runtime:
//! `stream` builds a stateful command object and `push` stacks a transform on a
//! real channel, neither of which the runtime models — they report so clearly.
//!
//! Semantics verified against tclsh 9.0 (`Tcl_ZlibObjCmd`, `tclZlib.c`).

use crate::ensemble::{must_be, resolve_subcommand};
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;
use std::io::{Read, Write};

/// Register the `zlib` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"zlib", zlib_cmd);
}

// -- checksums -------------------------------------------------------------

/// The reflected CRC-32 table (polynomial `0xEDB88320`), built at compile time.
const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

/// zlib's `crc32(init, data)` — `init` is the running CRC to continue from
/// (`0` to start). Matches `zlib crc32 data ?startValue?`.
fn crc32(init: u32, data: &[u8]) -> u32 {
    let mut c = init ^ 0xffff_ffff;
    for &b in data {
        c = CRC_TABLE[((c ^ u32::from(b)) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

/// zlib's `adler32(init, data)` — `init` is the running checksum (`1` to start).
/// Matches `zlib adler32 data ?startValue?`.
fn adler32(init: u32, data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a = init & 0xffff;
    let mut b = (init >> 16) & 0xffff;
    for &byte in data {
        a = (a + u32::from(byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

// -- codec (flate2 / miniz_oxide) ------------------------------------------

/// zlib's `zError` string for a decompression failure — corrupt or truncated
/// input surfaces as `Z_DATA_ERROR`, whose message is `data error` (`tclZlib.c`
/// `ConvertError`).
const DATA_ERROR: &[u8] = b"data error";

fn compress_zlib(data: &[u8], level: Compression) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), level);
    // Writing into a `Vec` sink cannot fail (barring OOM, which aborts).
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

fn compress_raw(data: &[u8], level: Compression) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), level);
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

fn compress_gzip(data: &[u8], level: Compression) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), level);
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

fn decompress_zlib(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    ZlibDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

fn decompress_raw(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    DeflateDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    GzDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

// -- argument helpers ------------------------------------------------------

/// Parse an optional `level` (0..=9) argument for `compress`/`deflate`; the
/// default (no argument) is zlib's `Z_DEFAULT_COMPRESSION` (level 6).
fn parse_level(interp: &mut Interp, arg: Option<&[u8]>) -> Result<Compression, Code> {
    match arg {
        None => Ok(Compression::default()),
        Some(bytes) => match tcl_cmd_core::sort::parse_wide(bytes) {
            Some(n) if (0..=9).contains(&n) => Ok(Compression::new(n as u32)),
            Some(_) => Err(interp.set_error(b"level must be 0 to 9")),
            None => {
                let mut m = b"expected integer but got \"".to_vec();
                m.extend_from_slice(bytes);
                m.push(b'"');
                Err(interp.set_error(&m))
            }
        },
    }
}

// -- the ensemble ----------------------------------------------------------

fn zlib_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const SUBS: &[&[u8]] = &[
        b"adler32",
        b"compress",
        b"crc32",
        b"decompress",
        b"deflate",
        b"gunzip",
        b"gzip",
        b"inflate",
        b"push",
        b"stream",
    ];
    if argv.len() < 2 {
        return interp.wrong_args(b"zlib subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    let names: Vec<Vec<u8>> = SUBS.iter().map(|s| s.to_vec()).collect();
    let Some(idx) = resolve_subcommand(&names, &sub, true) else {
        let mut m = b"unknown or ambiguous subcommand \"".to_vec();
        m.extend_from_slice(&sub);
        m.extend_from_slice(b"\": must be ");
        m.extend_from_slice(&must_be(&names));
        return interp.set_error(&m);
    };
    match SUBS[idx] {
        b"crc32" => checksum(interp, argv, false),
        b"adler32" => checksum(interp, argv, true),
        b"compress" => compress(interp, argv, Format::Zlib),
        b"deflate" => compress(interp, argv, Format::Raw),
        b"gzip" => gzip(interp, argv),
        b"decompress" => decompress(interp, argv, Format::Zlib),
        b"inflate" => decompress(interp, argv, Format::Raw),
        b"gunzip" => gunzip(interp, argv),
        // `stream` yields a stateful command object; `push` stacks a transform
        // on a channel. Neither is modelled by the tree-walking runtime.
        other => {
            let mut m = b"zlib ".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b" is not supported under the WASM runtime");
            interp.set_error(&m)
        }
    }
}

enum Format {
    Zlib,
    Raw,
}

/// `zlib crc32|adler32 data ?startValue?`.
fn checksum(interp: &mut Interp, argv: &[*mut TclObj], adler: bool) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        let usage: &[u8] = if adler {
            b"zlib adler32 data ?startValue?"
        } else {
            b"zlib crc32 data ?startValue?"
        };
        return interp.wrong_args(usage);
    }
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    // The start value defaults to the format's identity (adler starts at 1, crc
    // at 0) and is otherwise the running checksum to continue from.
    let init = if let Some(&a) = argv.get(3) {
        let bytes = obj_bytes(a);
        match tcl_cmd_core::sort::parse_wide(&bytes) {
            Some(n) => n as u32,
            None => {
                let mut m = b"expected integer but got \"".to_vec();
                m.extend_from_slice(&bytes);
                m.push(b'"');
                return interp.set_error(&m);
            }
        }
    } else if adler {
        1
    } else {
        0
    };
    let sum = if adler {
        adler32(init, &data)
    } else {
        crc32(init, &data)
    };
    interp.set_result_bytes(sum.to_string().as_bytes());
    Code::Ok
}

/// `zlib compress|deflate data ?level?`.
fn compress(interp: &mut Interp, argv: &[*mut TclObj], format: Format) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        let usage: &[u8] = match format {
            Format::Zlib => b"zlib compress data ?level?",
            Format::Raw => b"zlib deflate data ?level?",
        };
        return interp.wrong_args(usage);
    }
    let level = match parse_level(interp, argv.get(3).map(|&a| obj_bytes(a)).as_deref()) {
        Ok(l) => l,
        Err(code) => return code,
    };
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let out = match format {
        Format::Zlib => compress_zlib(&data, level),
        Format::Raw => compress_raw(&data, level),
    };
    interp.set_result_byte_array(&out);
    Code::Ok
}

/// `zlib decompress|inflate data ?bufferSize?` (the buffer-size hint is accepted
/// and ignored — the output vector grows as needed).
fn decompress(interp: &mut Interp, argv: &[*mut TclObj], format: Format) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        let usage: &[u8] = match format {
            Format::Zlib => b"zlib decompress data ?bufferSize?",
            Format::Raw => b"zlib inflate data ?bufferSize?",
        };
        return interp.wrong_args(usage);
    }
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let out = match format {
        Format::Zlib => decompress_zlib(&data),
        Format::Raw => decompress_raw(&data),
    };
    match out {
        Some(bytes) => {
            interp.set_result_byte_array(&bytes);
            Code::Ok
        }
        None => interp.set_error(DATA_ERROR),
    }
}

/// `zlib gzip data ?-level level? ?-header header?` — the `-header` dict (custom
/// gzip metadata) is not modelled here.
fn gzip(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"zlib gzip data ?-level level? ?-header header?");
    }
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let mut level = Compression::default();
    let mut i = 3;
    while i < argv.len() {
        let opt = obj_bytes(argv[i]);
        match opt.as_slice() {
            b"-level" => {
                let Some(&val) = argv.get(i + 1) else {
                    return interp.wrong_args(b"zlib gzip data ?-level level? ?-header header?");
                };
                level = match parse_level(interp, Some(&obj_bytes(val))) {
                    Ok(l) => l,
                    Err(code) => return code,
                };
                i += 2;
            }
            b"-header" => {
                return interp.set_error(
                    b"the -header option to zlib gzip is not supported under the WASM runtime",
                );
            }
            _ => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(&opt);
                m.extend_from_slice(b"\": must be -level or -header");
                return interp.set_error(&m);
            }
        }
    }
    let out = compress_gzip(&data, level);
    interp.set_result_byte_array(&out);
    Code::Ok
}

/// `zlib gunzip data ?-headerVar varName?` — the header-var readback is not
/// modelled here.
fn gunzip(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"zlib gunzip data ?-headerVar varName?");
    }
    if argv.len() > 3 {
        let opt = obj_bytes(argv[3]);
        if opt == b"-headerVar" {
            return interp.set_error(
                b"the -headerVar option to zlib gunzip is not supported under the WASM runtime",
            );
        }
        let mut m = b"bad option \"".to_vec();
        m.extend_from_slice(&opt);
        m.extend_from_slice(b"\": must be -headerVar");
        return interp.set_error(&m);
    }
    let data = match interp.binary_bytes(argv[2]) {
        Ok(data) => data,
        Err(code) => return code,
    };
    match decompress_gzip(&data) {
        Some(bytes) => {
            interp.set_result_byte_array(&bytes);
            Code::Ok
        }
        None => interp.set_error(DATA_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32};
    use crate::interp::{Code, Interp};

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
    fn checksums_match_zlib_vectors() {
        // Direct unit checks against tclsh 9.0 output.
        assert_eq!(crc32(0, b""), 0);
        assert_eq!(adler32(1, b""), 1);
        assert_eq!(crc32(0, b"abc"), 891_568_578);
        assert_eq!(adler32(1, b"abc"), 38_600_999);
        assert_eq!(crc32(0, b"hello, world"), 4_289_425_978);
        assert_eq!(adler32(1, b"hello, world"), 492_045_449);
        // Continued (non-default start value).
        assert_eq!(crc32(5, b"abc"), 871_334_697);
    }

    #[test]
    fn checksum_commands() {
        let mut i = Interp::new();
        assert_eq!(ok(&mut i, b"zlib crc32 abc"), b"891568578");
        assert_eq!(ok(&mut i, b"zlib adler32 abc"), b"38600999");
        assert_eq!(ok(&mut i, b"zlib crc32 abc 5"), b"871334697");
        // Arity.
        assert_eq!(i.eval_str(b"zlib crc32 a b c"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"zlib crc32 data ?startValue?\""
        );
    }

    #[test]
    fn compression_round_trips_all_framings() {
        let mut i = Interp::new();
        for form in [
            b"compress".as_slice(),
            b"gzip".as_slice(),
            b"deflate".as_slice(),
        ] {
            let (comp, decomp): (&[u8], &[u8]) = match form {
                b"compress" => (b"compress", b"decompress"),
                b"gzip" => (b"gzip", b"gunzip"),
                _ => (b"deflate", b"inflate"),
            };
            let mut script = b"zlib ".to_vec();
            script.extend_from_slice(decomp);
            script.extend_from_slice(b" [zlib ");
            script.extend_from_slice(comp);
            script.extend_from_slice(b" \"The quick brown fox jumps over the lazy dog\"]");
            assert_eq!(
                ok(&mut i, &script),
                b"The quick brown fox jumps over the lazy dog"
            );
        }
        // A larger, compressible payload exercises back-references.
        assert_eq!(
            ok(
                &mut i,
                b"zlib decompress [zlib compress [string repeat {abcABC123 } 500]]"
            ),
            b"abcABC123 ".repeat(500)
        );
    }

    #[test]
    fn decodes_real_tclsh_vectors() {
        // Raw-deflate / zlib / gzip streams produced by tclsh 9.0 — proves the
        // decoder is interoperable, not just self-consistent.
        let mut i = Interp::new();
        // deflate "hi" -> cbc80400 (fixed Huffman).
        assert_eq!(
            ok(&mut i, b"zlib inflate [binary decode hex cbc80400]"),
            b"hi"
        );
        // zlib compress "hello, world".
        assert_eq!(
            ok(
                &mut i,
                b"zlib decompress [binary decode hex 789ccb48cdc9c9d75128cf2fca4901001d540489]"
            ),
            b"hello, world"
        );
        // gzip "hello, world".
        assert_eq!(
            ok(
                &mut i,
                b"zlib gunzip [binary decode hex 1f8b0800000000000003cb48cdc9c9d75128cf2fca4901003a72abff0c000000]"
            ),
            b"hello, world"
        );
    }

    #[test]
    fn errors() {
        let mut i = Interp::new();
        // Unknown subcommand.
        assert_eq!(i.eval_str(b"zlib bogus x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown or ambiguous subcommand \"bogus\": must be adler32, compress, crc32, decompress, deflate, gunzip, gzip, inflate, push, or stream"
        );
        // Level out of range.
        assert_eq!(i.eval_str(b"zlib compress x 99"), Code::Error);
        assert_eq!(i.result_bytes(), b"level must be 0 to 9");
        // Corrupt data.
        assert_eq!(i.eval_str(b"zlib decompress {not zlib data}"), Code::Error);
        assert_eq!(i.result_bytes(), b"data error");
        // Streaming / channel forms report unsupported.
        assert_eq!(i.eval_str(b"zlib stream deflate"), Code::Error);
        assert!(i
            .result_bytes()
            .ends_with(b"is not supported under the WASM runtime"));
    }
}
