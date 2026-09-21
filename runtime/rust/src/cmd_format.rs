// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `format` — thin runtime adapter onto [`tcl_cmd_core::format`].
//!
//! The shared command core owns Tcl's format grammar and rendering semantics;
//! this module only translates its result into the runtime command ABI.  This
//! keeps the VM and WASM runtime on the same implementation of
//! `Tcl_AppendFormatToObj` (`generic/tclStringObj.c`).

use crate::interp::{Code, Interp};
use crate::obj::TclObj;

/// Register `format`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"format", format_cmd);
}

fn format_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let syntax = interp.runtime_version().number_syntax();
    match tcl_cmd_core::format::format_cmd_with_syntax(interp, &argv[1..], syntax) {
        Ok(value) => {
            interp.set_result(value);
            Code::Ok
        }
        Err(error) => interp.set_error(error.into_message().as_bytes()),
    }
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
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
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
    fn format_conversions_flags_width() {
        leak_free(|i| {
            assert_eq!(
                ok(i, b"format {%d %05d %-5d|} 42 42 42"),
                b"42 00042 42   |"
            );
            assert_eq!(ok(i, b"format {%x %#x %X} 255 255 255"), b"ff 0xff FF");
            assert_eq!(ok(i, b"format {%5.2f} 3.14159"), b" 3.14");
            assert_eq!(ok(i, b"format {%.3s %10s|} hello hi"), b"hel         hi|");
            assert_eq!(ok(i, b"format {%c%c%c} 72 105 33"), b"Hi!");
            assert_eq!(ok(i, b"format {%2$s %1$s} a b"), b"b a");
            assert_eq!(ok(i, b"format {%+d % d %o} 5 5 8"), b"+5  5 10");
        });
    }

    #[test]
    fn alternate_decimal_uses_shared_tcl9_semantics() {
        leak_free(|i| {
            assert_eq!(ok(i, b"format {%#d %#i} 42 42"), b"0d42 0d42");
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                ok(i, b"format {%#d %#i %#o %#X} 42 42 8 12"),
                b"42 42 010 0XC"
            );
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(
                ok(i, b"format {%#d %#i %#o %#X} 42 42 8 12"),
                b"0d42 0d42 0o10 0xC"
            );
        });
    }

    /// #1782: a size modifier and the release pick the integer width, and
    /// the low bits are then read signed for `d`/`i` and unsigned for
    /// `u`/`x`/`o`/`b`. Every expectation here is a transcript from the
    /// matching real tclsh (8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0).
    ///
    /// Three defects, one rule. Before this, `format %hd 5000000000` gave
    /// `5000000000` on every release, `format %u -1` gave `1`, and the
    /// unmodified width was 64-bit even under Tcl 9.
    #[test]
    fn integer_conversions_honour_width_and_signedness_issue_1782() {
        leak_free(|i| {
            // `h` is C `short` — 16 bits on every release and platform.
            for version in [
                tcl_dialect::TclVersion::V8_4,
                tcl_dialect::TclVersion::V8_6,
                tcl_dialect::TclVersion::V9_0,
            ] {
                i.set_runtime_version(version);
                assert_eq!(ok(i, b"format %hd 5000000000"), b"-3584", "{version:?}");
                assert_eq!(ok(i, b"format %hd 32768"), b"-32768", "{version:?}");
                assert_eq!(ok(i, b"format %hd -32769"), b"32767", "{version:?}");
                assert_eq!(ok(i, b"format %hu 5000000000"), b"61952", "{version:?}");
                assert_eq!(ok(i, b"format %hx 5000000000"), b"f200", "{version:?}");
                assert_eq!(ok(i, b"format %ho 5000000000"), b"171000", "{version:?}");
                // `h` does not reach `%c`.
                assert_eq!(ok(i, b"format %hc 65"), b"A", "{version:?}");
            }

            // The unmodified width is the release's: `long` before Tcl 9,
            // `int` from it.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(ok(i, b"format %d 5000000000"), b"5000000000");
            assert_eq!(ok(i, b"format %d 4294967296"), b"4294967296");
            assert_eq!(ok(i, b"format %u -1"), b"18446744073709551615");
            assert_eq!(ok(i, b"format %x -1"), b"ffffffffffffffff");

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(ok(i, b"format %d 5000000000"), b"705032704");
            assert_eq!(ok(i, b"format %d 4294967296"), b"0");
            assert_eq!(ok(i, b"format %u -1"), b"4294967295");
            assert_eq!(ok(i, b"format %x -1"), b"ffffffff");
            // `l` is the wide path, so it keeps all 64 bits under Tcl 9 too.
            assert_eq!(ok(i, b"format %ld 5000000000"), b"5000000000");

            // Width and precision still apply around the truncated value.
            assert_eq!(ok(i, b"format {%-6hd|} 70000"), b"4464  |");
            assert_eq!(ok(i, b"format %5.3d 7"), b"  007");
        });
    }

    #[test]
    fn format_overflow_width_errors_not_panics() {
        leak_free(|i| {
            for width in [b"4294967294".as_slice(), b"18446744073709551614"] {
                let mut source = b"format %".to_vec();
                source.extend_from_slice(width);
                source.extend_from_slice(b"g 0");
                assert_eq!(i.eval_str(&source), Code::Error);
                assert_eq!(i.result_bytes(), b"max size for a Tcl value exceeded");
            }
            assert_eq!(ok(i, b"format %5s hi"), b"   hi");
        });
    }
}
