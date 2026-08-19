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

//! `ValueOps` for the WASM runtime — binds the portable `tcl-cmd-core` command
//! logic to the runtime's 24-byte C-ABI `*mut TclObj` value model.
//!
//! This is the **opposite** value model from the bytecode VM's `Rc<Obj>`:
//! manually refcounted raw objects over the shared linear memory. The same
//! shared command helpers run over it unchanged — that is the entire point of
//! the value seam. Coercion reuses `tcl_syntax::number`, so it is byte-for-byte
//! identical to the VM's `ValueOps`.
//!
//! The copy-on-write asymmetry the contract is designed around is visible here:
//! [`ValueOps::try_append_str_in_place`] performs the runtime's amortised
//! in-place string growth when the object is an unshared plain string (the
//! EXP-STRING decision in `cmd_string.rs`), whereas the VM always copies.
//!
//! Byte-array representation is runtime-only: it has no separate LSP request
//! or VS Code UI surface. Its tests therefore sit beside the `ValueOps` seam
//! and run scripts through the real runtime, while the compiler's registry
//! data separately drives static byte-array diagnostics.

use std::rc::Rc;

use tcl_syntax::number::{self, Number};
use tcl_syntax::value::{string_char_len, ValueError, ValueOps};

use crate::interp::{obj_bytes, Interp};
use crate::list;
use crate::obj::{self, TclObj};

/// Decode raw object bytes as the Unicode string view `tcl-cmd-core`'s shared
/// command logic operates on.
///
/// `binary format` and the binary decoders now create a typed byte-array object
/// whose lazy string representation is valid UTF-8, so ordinary calls take the
/// first branch. The fallback is retained for arbitrary non-UTF-8 strings an
/// embedding passes through the C ABI: byte `b` becomes U+00XX, exactly like a
/// byte-array's string shimmer, rather than being replaced by U+FFFD.
fn bytes_to_str(bytes: &[u8]) -> Rc<str> {
    match core::str::from_utf8(bytes) {
        Ok(s) => Rc::from(s),
        Err(_) => Rc::from(
            bytes
                .iter()
                .copied()
                .map(char::from)
                .collect::<String>()
                .as_str(),
        ),
    }
}

/// The inverse of [`bytes_to_str`]: encode a `tcl-cmd-core` string result back
/// to a Tcl string representation for [`ValueOps::new_str`]/[`ValueOps::new_string`].
///
/// Binary conversion is deliberately not performed here. The result remains a
/// normal Unicode string; the central byte-array conversion in
/// [`Interp::binary_bytes`] later applies the emulated Tcl release's policy.
/// That is why Tcl 8 truncates `Ÿ` to `x`, while Tcl 9 raises instead.
fn str_to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

impl ValueOps for Interp {
    type Value = *mut TclObj;

    fn new_str(&mut self, s: &str) -> *mut TclObj {
        obj::new_string_bytes(&str_to_bytes(s))
    }

    fn new_string(&mut self, s: String) -> *mut TclObj {
        obj::new_string_bytes(&str_to_bytes(&s))
    }

    fn new_int(&mut self, n: i64) -> *mut TclObj {
        obj::new_wide_int_obj(n)
    }

    fn new_double(&mut self, f: f64) -> *mut TclObj {
        obj::new_double_obj(f)
    }

    fn new_bool(&mut self, b: bool) -> *mut TclObj {
        obj::new_boolean_obj(i32::from(b))
    }

    fn new_list(&mut self, items: Vec<*mut TclObj>) -> *mut TclObj {
        list::new_list_obj(&items)
    }

    fn as_str(&mut self, v: &*mut TclObj) -> Rc<str> {
        bytes_to_str(&obj_bytes(*v))
    }

    fn char_len(&mut self, v: &*mut TclObj) -> usize {
        string_char_len(&bytes_to_str(&obj_bytes(*v)), self.runtime_version())
    }

    fn as_int(&mut self, v: &*mut TclObj) -> Result<i64, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n),
            _ => Err(ValueError::NotInteger(s.into_owned())),
        }
    }

    fn string_compare_length(&mut self, v: &*mut TclObj) -> Result<Option<usize>, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(usize::try_from(n).ok()),
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow),
            _ => Err(ValueError::NotInteger(s.into_owned())),
        }
    }

    fn as_double(&mut self, v: &*mut TclObj) -> Result<f64, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n as f64),
            Some(Number::Double(f)) => Ok(f),
            _ => Err(ValueError::NotDouble(s.into_owned())),
        }
    }

    /// `Tcl_GetBooleanFromObj`'s boolean-context acceptor, over the shared
    /// [`tcl_syntax::boolean`] owner — a boolean word by unique prefix
    /// (`tru`, `ye`, `of`), else any number compared against zero. The local
    /// six-spelling table this replaced had no prefix rule (issue #1425).
    fn as_bool(&mut self, v: &*mut TclObj) -> Result<bool, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        tcl_syntax::boolean::truthiness(&s).ok_or_else(|| ValueError::NotBoolean(s.into_owned()))
    }

    /// Bignum-aware integer addition (the `incr` step) — overrides the default's
    /// fixed-`i64` path. Both operands must read as integers (a wide or a bignum;
    /// a float or non-number is the canonical "expected integer" error). The sum
    /// goes over the numeric tower, so it **widens to a bignum on overflow**
    /// instead of erroring — Tcl integers never wrap — and demotes back to a wide
    /// when it fits. A `None` left operand is an unset variable, treated as 0.
    ///
    /// Only when the libtommath FFI is linked (`have_tommath`). A build without it
    /// (the wasm32 target) keeps the trait's fixed-`i64` default — overflow then
    /// errors, exactly like the bytecode VM, since there is no bignum tower.
    #[cfg(have_tommath)]
    fn int_add(
        &mut self,
        a: Option<&*mut TclObj>,
        b: &*mut TclObj,
    ) -> Result<*mut TclObj, ValueError> {
        fn not_int(obj: *mut TclObj) -> ValueError {
            ValueError::NotInteger(String::from_utf8_lossy(&obj_bytes(obj)).into_owned())
        }
        // Coercion order matches C's `TclIncrObj`: the current value first, then
        // the increment (both must read as an integer — a wide or a bignum).
        if let Some(av) = a {
            if !crate::bignum::is_integer(*av) {
                return Err(not_int(*av));
            }
        }
        if !crate::bignum::is_integer(*b) {
            return Err(not_int(*b));
        }
        // Both integers → tower add (never fails for two integer operands except
        // on allocation failure, mapped to the overflow error). The left operand
        // is a transient zero when the variable was unset.
        match a {
            Some(av) => crate::bignum::add(*av, *b).map_err(|_| ValueError::IntegerOverflow),
            None => {
                let zero = obj::new_wide_int_obj(0);
                let sum = crate::bignum::add(zero, *b).map_err(|_| ValueError::IntegerOverflow);
                crate::interp::drop_fresh(zero);
                sum
            }
        }
    }

    fn list_elements(&mut self, v: &*mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
        list::list_elements(*v)
            .map_err(|e| ValueError::BadList(String::from_utf8_lossy(e.message()).into_owned()))
    }

    /// Byte-exact, and simpler than `as_str`'s Unicode round trip — this is why
    /// `append` (which never needs character semantics) routes through the
    /// shared core without any of `bytes_to_str`/`str_to_bytes`'s trade-offs.
    fn as_bytes(&mut self, v: &*mut TclObj) -> Rc<[u8]> {
        Rc::from(obj_bytes(*v).as_slice())
    }

    /// Byte-exact construction (the `obj::new_string_bytes` path).
    fn new_bytes(&mut self, bytes: &[u8]) -> *mut TclObj {
        obj::new_string_bytes(bytes)
    }

    fn try_append_bytes_in_place(&mut self, v: &mut *mut TclObj, bytes: &[u8]) -> bool {
        // Amortised O(1) growth when the object is an unshared plain string —
        // the EXP-STRING in-place path the VM cannot take (it always copies).
        if obj::is_plain_string(*v) && !obj::is_shared(*v) {
            obj::string_append_inplace(*v, bytes);
            true
        } else {
            false
        }
    }

    fn try_list_append_in_place(&mut self, list: &mut *mut TclObj, item: &*mut TclObj) -> bool {
        // Mutate the list's backing vector in place when it is uniquely owned —
        // the COW fast path the runtime's hand-rolled `lappend` used. A shared or
        // non-list value falls back to a rebuild (the caller copies). `list_append`
        // validates the list first, so a malformed value leaves it untouched.
        if obj::is_shared(*list) {
            return false;
        }
        crate::list::list_append(*list, *item).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{bytes_to_str, str_to_bytes};
    use crate::counters;
    use crate::interp::{Code, Interp};
    use tcl_dialect::TclVersion;

    fn run(src: &[u8]) -> (Code, Vec<u8>) {
        run_at(TclVersion::V9_0, src)
    }

    fn run_at(version: TclVersion, src: &[u8]) -> (Code, Vec<u8>) {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            i.set_runtime_version(version);
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(
            counters::finalize(),
            0,
            "leak: {} objs {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
        (code, bytes)
    }
    fn ok(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(c, Code::Ok, "result={:?}", String::from_utf8_lossy(&b));
        b
    }

    // -- bytes_to_str / str_to_bytes unit coverage -------------------------

    /// TP: an arbitrary invalid UTF-8 plain string gets Tcl's U+00XX byte view,
    /// not Rust's replacement character. Runtime-created binary values use the
    /// typed byte-array path, exercised below.
    #[test]
    fn bytes_to_str_falls_back_on_invalid_utf8() {
        let s = bytes_to_str(&[0x41, 0xFF, 0x42]);
        assert_eq!(s.chars().count(), 3);
        assert_eq!(s.chars().nth(1), Some('\u{00ff}'));
    }

    /// TN: genuine valid UTF-8 (including non-ASCII Latin-1-supplement text)
    /// remains its real characters, preserving Unicode string operations.
    #[test]
    fn bytes_to_str_preserves_valid_utf8() {
        let s = bytes_to_str("café".as_bytes());
        assert_eq!(&*s, "café");
        assert_eq!(s.chars().count(), 4);
        assert_eq!(s.chars().nth(3), Some('\u{00e9}'));
    }

    /// String construction remains ordinary UTF-8; byte conversion is deferred
    /// to `Interp::binary_bytes`, where it can consult the Tcl version.
    #[test]
    fn str_to_bytes_keeps_unicode_string_representation() {
        assert_eq!(str_to_bytes("A\u{00ff}B"), "A\u{00ff}B".as_bytes());
    }

    /// FP guard: genuine Latin-1-supplement text (codepoints `<= 0xFF` but
    /// *not* escaped, e.g. 'é') round-trips as real UTF-8, not a raw byte —
    /// the exact case a naive "codepoint fits a byte" heuristic gets wrong.
    #[test]
    fn str_to_bytes_keeps_utf8_for_real_latin1_text() {
        assert_eq!(str_to_bytes("café"), "café".as_bytes());
    }

    /// FP guard: a string with a genuine wide character (e.g. CJK) is encoded
    /// as real UTF-8, not corrupted.
    #[test]
    fn str_to_bytes_keeps_utf8_for_wide_chars() {
        assert_eq!(str_to_bytes("a\u{65e5}b"), "a\u{65e5}b".as_bytes().to_vec());
    }

    // -- end-to-end: byte-array dual ports, driven through `string`/`binary` --

    /// TP: `string index`/`range`/`replace`/`length` on a `binary format` value
    /// preserve binary bytes exactly in both C Tcl 8.6 and 9.0.
    #[test]
    fn string_subcommands_preserve_binary_bytes() {
        let script = br#"
            set b [binary format H* 41ff42]
            list [string length $b] \
                 [binary encode hex [string index $b 1]] \
                 [binary encode hex [string range $b 0 2]] \
                 [binary encode hex [string replace $b 0 0 X]]
        "#;
        assert_eq!(ok(script), b"3 ff 41ff42 58ff42");
    }

    /// TP: every byte-producing command constructs the same typed byte-array
    /// representation, so a later string read followed by `binary encode` sees
    /// the original payload. This covers decode, scan assignment, and zlib's
    /// decompression result rather than only `binary format`.
    #[test]
    fn byte_producing_commands_share_the_dual_port_representation() {
        let script = br#"
            set decoded [binary decode hex 41ff42]
            binary scan [binary format H* 41ff42] a* scanned
            set inflated [zlib decompress [zlib compress [binary format H* 41ff42]]]
            list \
                [binary encode hex [string range $decoded 0 2]] \
                [binary encode hex [string range $scanned 0 2]] \
                [binary encode hex [string range $inflated 0 2]]
        "#;
        assert_eq!(ok(script), b"41ff42 41ff42 41ff42");
    }

    /// TP/FN: a byte-array string shimmer is version-independent, but turning
    /// a changed Unicode string back into bytes is release-defined. These
    /// values are pinned to C Tcl 8.6.18 and 9.0.4.
    #[test]
    fn string_case_mapping_of_a_byte_array_uses_the_tcl_release_byte_policy() {
        let script = br#"binary encode hex [string toupper [binary format H* 41ff42]]"#;
        let (old_code, old_result) = run_at(TclVersion::V8_6, script);
        assert_eq!(old_code, Code::Ok);
        assert_eq!(old_result, b"417842");

        let (modern_code, modern_result) = run_at(TclVersion::V9_0, script);
        assert_eq!(modern_code, Code::Error);
        assert_eq!(
            modern_result,
            b"expected code point values below 0xff but value at byte offset 1 was 0x178"
        );
    }

    /// FN: a byte array has a real Unicode string representation, so its case
    /// mapping is not silently a no-op. Tcl 9 rejects the final conversion to
    /// bytes because `Ÿ` is outside the checked byte domain.
    #[test]
    fn string_toupper_on_binary_uses_the_checked_tcl9_byte_conversion() {
        let (code, message) =
            run(br#"binary encode hex [string toupper [binary format H* 41ff42]]"#);
        assert_eq!(code, Code::Error);
        assert_eq!(
            message,
            b"expected code point values below 0xff but value at byte offset 1 was 0x178"
        );
    }

    /// TN: plain ASCII through the same subcommands is unaffected.
    #[test]
    fn string_subcommands_ascii_unaffected() {
        assert_eq!(ok(b"string range hello 1 3"), b"ell");
        assert_eq!(ok(b"string toupper hello"), b"HELLO");
        assert_eq!(ok(b"string index hello 0"), b"h");
    }

    /// TN: genuine multi-byte Unicode text is still handled by character, not
    /// by byte, through the same subcommands the binary fix touches.
    #[test]
    fn string_subcommands_wide_unicode_unaffected() {
        // U+65E5 U+672C U+8A9E ("nihongo" in kanji) — 3 characters, 9 bytes.
        let script = "list [string length \u{65e5}\u{672c}\u{8a9e}] [string index \u{65e5}\u{672c}\u{8a9e} 1]";
        assert_eq!(ok(script.as_bytes()), "3 \u{672c}".as_bytes());
    }
}
