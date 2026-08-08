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

use std::rc::Rc;

use tcl_syntax::number::{self, Number};
use tcl_syntax::value::{ValueError, ValueOps};

use crate::interp::{obj_bytes, Interp};
use crate::list;
use crate::obj::{self, TclObj};

/// Base codepoint for [`bytes_to_str`]'s byte-escape range: Unicode Plane 16,
/// the Supplementary Private Use Area-B (`U+100000..=U+10FFFD`), reserved by
/// the standard for private application use and never assigned to real script
/// text. `BYTE_ESCAPE_BASE + b` (`b` in `0..=0xFF`) is the escaped form of raw
/// byte `b`; the 256-codepoint range starting here is never produced by
/// decoding valid UTF-8, so it cannot collide with genuine text — unlike Tcl's
/// own byte-array string-rep convention (byte `b` ↦ literal `U+00b`, also used
/// by the bytecode VM's `tcl-vm/src/cmd_binary.rs`), which *does* collide: a
/// literal `U+00E9` ('é') is indistinguishable from an escaped raw byte 0xE9,
/// and `runtime/rust` (unlike the VM) has real callers that decode genuine
/// Latin-1-supplement text through this exact seam (`string index héllo 1`
/// must stay `é`, not shimmer to a fallback byte).
const BYTE_ESCAPE_BASE: u32 = 0x10_0000;

/// Decode raw object bytes as the Unicode string view `tcl-cmd-core`'s shared
/// command logic operates on (issue #1309).
///
/// `runtime/rust`'s `TclObj` has one byte buffer doing double duty: it is the
/// object's string rep (always genuine UTF-8 for ordinary Tcl text) *and* the
/// raw payload for a value built by `binary format` / a binary channel read,
/// which is not generally valid UTF-8. Valid UTF-8 decodes losslessly — the
/// common case, ordinary text. Bytes that fail to decode fall back to
/// [`BYTE_ESCAPE_BASE`]: byte `b` becomes the scalar `BYTE_ESCAPE_BASE + b`,
/// so every `string` subcommand sees one character per byte and the value
/// stays round-trippable instead of losing data to a `U+FFFD` blanket
/// substitution. See [`str_to_bytes`] for the paired encode direction, and
/// `docs/kcs/kcs-issue-runtime-string-subcommands-corrupt-binary-values.md`
/// for the trade-off this pair accepts.
fn bytes_to_str(bytes: &[u8]) -> Rc<str> {
    match core::str::from_utf8(bytes) {
        Ok(s) => Rc::from(s),
        Err(_) => Rc::from(
            bytes
                .iter()
                .map(|&b| char::from_u32(BYTE_ESCAPE_BASE + u32::from(b)).unwrap_or('\u{FFFD}'))
                .collect::<String>()
                .as_str(),
        ),
    }
}

/// The inverse of [`bytes_to_str`]: encode a `tcl-cmd-core` string result back
/// to raw object bytes for [`ValueOps::new_str`]/[`ValueOps::new_string`].
///
/// A character in the [`BYTE_ESCAPE_BASE`] range decodes back to the single
/// raw byte it escapes; every other character is real text, encoded as
/// ordinary UTF-8. Because the escape range is never produced by decoding
/// valid UTF-8 (see [`bytes_to_str`]), this is unambiguous per character, so a
/// result can freely mix escaped bytes from one binary operand with genuine
/// text from another (e.g. `string replace $binaryValue 0 0 "café"`) and each
/// half round-trips correctly. This is what makes a `binary format` value
/// survive a `string` subcommand round trip byte for byte (issue #1309):
/// `string replace`/`index`/`range` on such a value read raw invalid-UTF-8
/// bytes as one escaped character per byte via `bytes_to_str`, rearrange them,
/// and this reassembles the original byte values exactly.
///
/// The trade-off: a case-changing subcommand (`string toupper`/`tolower`/
/// `totitle`) on escaped bytes is a no-op — Plane 16 Private Use Area
/// characters have no Unicode case mapping, so `char::to_uppercase` leaves
/// them unchanged, rather than applying the Latin-1 case fold real Tcl's
/// byte-array shimmer would (`0xFF` → `Ÿ` → truncated back to `0x78`). Given
/// the choice between silently case-folding binary data byte-for-byte-wrong
/// and leaving it untouched, leaving it untouched keeps the value intact.
#[allow(clippy::cast_possible_truncation)] // guarded by the BYTE_ESCAPE_BASE range check
fn str_to_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if (BYTE_ESCAPE_BASE..BYTE_ESCAPE_BASE + 0x100).contains(&cp) {
            out.push((cp - BYTE_ESCAPE_BASE) as u8);
        } else {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
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

    fn as_int(&mut self, v: &*mut TclObj) -> Result<i64, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n),
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

    fn as_bool(&mut self, v: &*mut TclObj) -> Result<bool, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        let t = s.trim();
        if let Some(num) = number::parse_whole(t) {
            return Ok(match num {
                Number::Int(n) => n != 0,
                Number::Double(f) => f != 0.0,
                _ => true,
            });
        }
        match t.to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" => Ok(true),
            "false" | "no" | "off" => Ok(false),
            _ => Err(ValueError::NotBoolean(s.into_owned())),
        }
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
    use super::{bytes_to_str, str_to_bytes, BYTE_ESCAPE_BASE};
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn run(src: &[u8]) -> (Code, Vec<u8>) {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
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

    fn escaped(b: u8) -> char {
        char::from_u32(BYTE_ESCAPE_BASE + u32::from(b)).unwrap()
    }

    // -- bytes_to_str / str_to_bytes unit coverage -------------------------

    /// TP: invalid UTF-8 (a lone 0xFF, as `binary format` produces) falls back
    /// to the escape range instead of `from_utf8_lossy`'s U+FFFD.
    #[test]
    fn bytes_to_str_falls_back_on_invalid_utf8() {
        let s = bytes_to_str(&[0x41, 0xFF, 0x42]);
        assert_eq!(s.chars().count(), 3);
        assert_eq!(s.chars().nth(1), Some(escaped(0xFF)));
    }

    /// TN: genuine valid UTF-8 (including non-ASCII Latin-1-supplement text)
    /// decodes losslessly to its real characters, never the escape range —
    /// this is the property that keeps `string index héllo 1` returning a
    /// genuine `é`, not a fallback byte (see `utf8_char_indexing` in
    /// `cmd_string.rs`, which regressed under an earlier, content-sniffing
    /// version of this fix that could not tell the two apart).
    #[test]
    fn bytes_to_str_preserves_valid_utf8() {
        let s = bytes_to_str("café".as_bytes());
        assert_eq!(&*s, "café");
        assert_eq!(s.chars().count(), 4);
        assert!(s.chars().all(|c| (c as u32) < BYTE_ESCAPE_BASE));
    }

    /// TP: encoding an escaped-byte string reverses back to the original raw
    /// bytes.
    #[test]
    fn str_to_bytes_unescapes_the_byte_range() {
        let s: String = [escaped(0x41), escaped(0xFF), escaped(0x42)]
            .into_iter()
            .collect();
        assert_eq!(str_to_bytes(&s), vec![0x41, 0xFF, 0x42]);
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

    /// FN guard: the full round trip (bytes -> str -> bytes) for a value that
    /// came from raw binary data must reproduce the original bytes exactly —
    /// this is the actual invariant issue #1309 depends on, not just the two
    /// halves in isolation.
    #[test]
    fn bytes_str_bytes_round_trip_is_lossless_for_binary_data() {
        let original: &[u8] = &[0x00, 0x41, 0xFF, 0x80, 0xC3, 0x28, 0x42]; // 0xC3 0x28 is invalid UTF-8
        let s = bytes_to_str(original);
        assert_eq!(str_to_bytes(&s), original);
    }

    /// FN guard: a result that mixes an escaped-byte operand with a genuine
    /// text operand (e.g. `string replace $binaryValue 0 0 "café"`) must
    /// round-trip each half correctly rather than picking one encoding for
    /// the whole string.
    #[test]
    fn str_to_bytes_handles_mixed_escaped_and_genuine_text() {
        let mixed: String = "café".chars().chain([escaped(0xFF)]).collect();
        let mut expected = "café".as_bytes().to_vec();
        expected.push(0xFF);
        assert_eq!(str_to_bytes(&mixed), expected);
    }

    // -- end-to-end: the issue #1309 repro, driven through `string`/`binary` --

    /// TP: `string index`/`range`/`replace`/`length` on a `binary format` value
    /// preserve non-UTF-8-safe bytes exactly, matching tclsh 9.0 (verified
    /// against tclsh 8.6 as a stand-in oracle where 9.0 was unavailable).
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

    /// TP: `string toupper`/`tolower` on binary content no longer emit the
    /// U+FFFD replacement-character corruption (`efbfbd`) the lossy `as_str`
    /// produced for every non-ASCII byte.
    #[test]
    fn string_toupper_on_binary_does_not_emit_replacement_char() {
        let out = ok(br#"binary encode hex [string toupper [binary format H* 41ff42]]"#);
        assert!(
            !String::from_utf8_lossy(&out).contains("efbfbd"),
            "still corrupting via U+FFFD: {}",
            String::from_utf8_lossy(&out)
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
