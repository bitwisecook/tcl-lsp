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

use super::*;

fn chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

/// A throwaway string-only `ValueOps` for the `first`/`last` index tests.
#[derive(Default)]
struct StrOps;

impl crate::index::NativeIndexOps for StrOps {
    type IndexOperation = ();
    type IndexBinaryReceipt = ();
    type IndexKeyOwner = Self::Value;
    fn index_borrowed_key(&mut self, value: &Self::Value) -> Result<Self::Value, crate::CmdError> {
        Ok(value.clone())
    }
    fn index_created_key(&mut self, value: Self::Value) -> Result<Self::Value, crate::CmdError> {
        Ok(value)
    }
    fn index_key_value(owner: &Self::Value) -> &Self::Value {
        owner
    }
    fn index_hold_key_result(
        &mut self,
        owner: &Self::Value,
    ) -> Result<Self::Value, crate::CmdError> {
        Ok(owner.clone())
    }
    fn index_finish_held_key_result(
        &mut self,
        held: Self::Value,
    ) -> Result<crate::index::IndexResult<Self::Value>, crate::CmdError> {
        Ok(crate::index::IndexResult::Detached(held))
    }
    fn index_key_argument(&mut self, owner: &Self::Value) -> Result<Self::Value, crate::CmdError> {
        Ok(owner.clone())
    }
    fn index_key_result(
        &mut self,
        owner: &Self::Value,
        created: bool,
        ancestor: bool,
    ) -> Result<crate::index::IndexResult<Self::Value>, crate::CmdError> {
        Ok(if created {
            crate::index::IndexResult::Created(owner.clone())
        } else if ancestor {
            crate::index::IndexResult::Detached(owner.clone())
        } else {
            crate::index::IndexResult::Borrowed(owner.clone())
        })
    }
    fn enter_index_operation(&mut self) -> Result<(), crate::CmdError> {
        Ok(())
    }
    fn check_index_operation(&mut self, (): &()) -> Result<(), crate::CmdError> {
        Ok(())
    }
    fn retain_index_failure(&mut self, _error: &crate::CmdError) {
        // Pure neutral adapter has no interpreter first-cause owner.
    }
    fn discard_owned_index_result(&mut self, created: Self::Value) {
        drop(created);
    }
}

impl ValueOps for StrOps {
    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        Some(tcl_dialect::IndexSyntax::for_version(
            tcl_dialect::TclVersion::V9_0,
        ))
    }
    type Value = String;
    fn new_str(&mut self, s: &str) -> String {
        s.to_owned()
    }
    fn new_int(&mut self, n: i64) -> String {
        n.to_string()
    }
    fn new_double(&mut self, f: f64) -> String {
        tcl_syntax::number::format_double(f)
    }
    fn new_bool(&mut self, b: bool) -> String {
        (if b { "1" } else { "0" }).to_owned()
    }
    fn new_list(&mut self, items: Vec<String>) -> String {
        items.join(" ")
    }
    fn as_bytes(&mut self, v: &String) -> std::rc::Rc<[u8]> {
        std::rc::Rc::from(v.as_bytes())
    }
    fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
        self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
    }

    fn as_int(&mut self, v: &String) -> Result<i64, tcl_syntax::value::ValueError> {
        v.parse()
            .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.clone()))
    }
    fn as_double(&mut self, _v: &String) -> Result<f64, tcl_syntax::value::ValueError> {
        Ok(0.0)
    }
    fn as_bool(&mut self, _v: &String) -> Result<bool, tcl_syntax::value::ValueError> {
        Ok(false)
    }
    fn list_elements(&mut self, v: &String) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
        Ok(v.split_whitespace().map(str::to_owned).collect())
    }
}

fn last_of(needle: &str, haystack: &str, last_index: Option<&str>) -> i64 {
    let mut ops = StrOps;
    let n = needle.to_owned();
    let h = haystack.to_owned();
    let li = last_index.map(str::to_owned);
    let r = last(&mut ops, &n, &h, li.as_ref(), NumberSyntax::Tcl90).expect("string last");
    r.parse().unwrap()
}

/// `string <words>` through [`dispatch`] under `numbers`, with the
/// thread's installed grammar set to `ambient` so an answer that read it
/// instead would show.
fn string_under(words: &[&str], numbers: NumberSyntax, ambient: NumberSyntax) -> String {
    use tcl_syntax::number::{runtime_syntax, set_runtime_syntax};
    let restore = runtime_syntax();
    set_runtime_syntax(ambient);
    let args: Vec<String> = words.iter().map(|&w| w.to_owned()).collect();
    let answer = dispatch(&mut StrOps, &args, numbers).expect("a core subcommand");
    set_runtime_syntax(restore);
    answer.unwrap_or_else(|error| format!("error: {}", error.message().expect("guest message")))
}

#[test]
fn index_words_read_under_the_grammar_the_caller_names() {
    // Regression (#2126): the string cores read their index words under
    // the thread's installed grammar, so a caller under a stated profile
    // got whichever release the process last pinned. Measured with
    // LANG=C.UTF-8 on tclsh 8.4.20, 8.5.19 and 8.6.18 (the 8.x column,
    // identical on all three but 8.4's wording of `bad index`) and 9.0.4
    // and 9.1.0 (the 9.x column):
    //   string range abcdefghijkl 010 end           ijkl | kl
    //   string range abcdefghijkl 0 end-010         abcd | ab
    //   string range abcdefghijkl 1_0 end           bad index | kl
    //   string range abcdefghijkl 0d1 end           bad index | bcdefghijkl
    //   string index abcdefghijkl 010               i | k
    //   string index abcdefghijkl end-010           d | b
    //   string wordstart {abcdefgh ijkl mnop} 010   8 | 9
    //   string wordend {abcdefgh ijkl mnop} 010     9 | 13
    //   string wordstart {abcdefgh ijkl mnop} end-010   9 | 0
    //   string first a aaaaaaaaaaaaaa 010           8 | 10
    //   string last a aaaaaaaaaaaaaa 010            8 | 10
    //   string replace abcdefghijkl 010 010 Z       abcdefghZjkl | abcdefghijZl
    //   string toupper abcdefghijkl 010 010         abcdefghIjkl | abcdefghijKl
    let bad = |spec: &str| {
        format!("error: bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    let rows: &[(&[&str], String, String)] = &[
        (
            &["range", "abcdefghijkl", "010", "end"],
            "ijkl".into(),
            "kl".into(),
        ),
        (
            &["range", "abcdefghijkl", "0", "end-010"],
            "abcd".into(),
            "ab".into(),
        ),
        (
            &["range", "abcdefghijkl", "1_0", "end"],
            bad("1_0"),
            "kl".into(),
        ),
        (
            &["range", "abcdefghijkl", "0d1", "end"],
            bad("0d1"),
            "bcdefghijkl".into(),
        ),
        (&["index", "abcdefghijkl", "010"], "i".into(), "k".into()),
        (
            &["index", "abcdefghijkl", "end-010"],
            "d".into(),
            "b".into(),
        ),
        (
            &["wordstart", "abcdefgh ijkl mnop", "010"],
            "8".into(),
            "9".into(),
        ),
        (
            &["wordend", "abcdefgh ijkl mnop", "010"],
            "9".into(),
            "13".into(),
        ),
        (
            &["wordstart", "abcdefgh ijkl mnop", "end-010"],
            "9".into(),
            "0".into(),
        ),
        (
            &["first", "a", "aaaaaaaaaaaaaa", "010"],
            "8".into(),
            "10".into(),
        ),
        (
            &["last", "a", "aaaaaaaaaaaaaa", "010"],
            "8".into(),
            "10".into(),
        ),
        (
            &["replace", "abcdefghijkl", "010", "010", "Z"],
            "abcdefghZjkl".into(),
            "abcdefghijZl".into(),
        ),
        (
            &["toupper", "abcdefghijkl", "010", "010"],
            "abcdefghIjkl".into(),
            "abcdefghijKl".into(),
        ),
    ];
    // 8.4 words its `bad index` for its own grammar.
    let tcl84 = |tcl8: &str| {
        tcl8.replace(
            "must be integer?[+-]integer? or end?[+-]integer?",
            "must be integer or end?-integer?",
        )
    };
    for (words, tcl8, tcl9) in rows {
        assert_eq!(
            string_under(words, NumberSyntax::Tcl84, NumberSyntax::Tcl90),
            tcl84(tcl8),
            "{words:?} under Tcl84"
        );
        assert_eq!(
            &string_under(words, NumberSyntax::Tcl85, NumberSyntax::Tcl90),
            tcl8,
            "{words:?} under Tcl85"
        );
        assert_eq!(
            &string_under(words, NumberSyntax::Tcl90, NumberSyntax::Tcl85),
            tcl9,
            "{words:?} under Tcl90"
        );
    }
    // `string insert` is 9.0's (8.6.18: `unknown or ambiguous subcommand`);
    // 9.0.4 and 9.1.0 read `010` as 10.
    assert_eq!(
        string_under(
            &["insert", "abcdefghijkl", "010", "Z"],
            NumberSyntax::Tcl90,
            NumberSyntax::Tcl85
        ),
        "abcdefghijZkl"
    );
}

#[test]
fn string_last_empty_haystack_with_index_does_not_panic() {
    // `string last a "" 0` must return -1, not panic on an
    // out-of-bounds slice of the empty haystack.
    assert_eq!(last_of("a", "", Some("0")), -1);
    assert_eq!(last_of("a", "", None), -1);
    // Needle longer than haystack with an explicit index.
    assert_eq!(last_of("abc", "ab", Some("1")), -1);
}

#[test]
fn string_last_basic() {
    assert_eq!(last_of("a", "banana", None), 5);
    assert_eq!(last_of("an", "banana", None), 3);
    assert_eq!(last_of("an", "banana", Some("2")), 1);
    assert_eq!(last_of("z", "banana", None), -1);
}

#[test]
fn string_compare_equal_share_option_and_unicode_semantics() {
    let mut ops = StrOps;
    assert_eq!(
        compare(
            &mut ops,
            &[
                "-noc".to_owned(),
                "-len".to_owned(),
                "2".to_owned(),
                "ABxx".to_owned(),
                "abYY".to_owned(),
            ],
            CompareMode::Compare,
        )
        .unwrap(),
        "0"
    );
    // Tcl folds before applying -length: U+0130 lowercases to `i` plus a
    // combining dot, whose first character compares equal to `i`.
    assert_eq!(
        compare(
            &mut ops,
            &[
                "-nocase".to_owned(),
                "-length".to_owned(),
                "1".to_owned(),
                "\u{0130}".to_owned(),
                "i".to_owned(),
            ],
            CompareMode::Equal,
        )
        .unwrap(),
        "1"
    );
    assert_eq!(
        compare(
            &mut ops,
            &["-bogus".to_owned(), "a".to_owned(), "b".to_owned()],
            CompareMode::Equal,
        )
        .unwrap_err()
        .message()
        .unwrap(),
        "bad option \"-bogus\": must be -nocase or -length"
    );
}

#[test]
fn word_start_finds_word_boundaries() {
    // Tcl `string wordstart` semantics. "abc def" = indices a0 b1 c2 ' '3 d4 e5 f6.
    let c = chars("abc def");
    assert_eq!(word_start(&c, 1), 0); // inside "abc"
    assert_eq!(word_start(&c, 5), 4); // inside "def"
    assert_eq!(word_start(&c, 3), 3); // the space is its own word
    assert_eq!(word_start(&c, 0), 0);
    assert_eq!(word_start(&c, 100), 4); // clamps to last char of "def"
    assert_eq!(word_start(&[], 3), 0); // empty
}

#[test]
fn totitle_first_char_uses_titlecase_not_uppercase() {
    let mut ops = StrOps;
    let title = |ops: &mut StrOps, s: &str| {
        case_convert(
            ops,
            &[s.to_owned()],
            CaseMode::Title,
            "u",
            NumberSyntax::Tcl90,
        )
        .unwrap()
    };
    // The Latin dz digraph (U+01F3) titlecases to Dž (U+01F2), not the
    // uppercase DZ (U+01F1) — string-17.7; the rest is lowercased.
    assert_eq!(title(&mut ops, "\u{01F3}BCabc"), "\u{01F2}bcabc");
    assert_eq!(title(&mut ops, "\u{01C9}x"), "\u{01C8}x"); // lj → Lj
    // A non-digraph first char still titlecases to its uppercase.
    assert_eq!(title(&mut ops, "hELLO"), "Hello");
    // `toupper` of the digraph is the uppercase form (U+01F1), not titlecase.
    assert_eq!(
        case_convert(
            &mut ops,
            &["\u{01F3}".to_owned()],
            CaseMode::Upper,
            "u",
            NumberSyntax::Tcl90
        )
        .unwrap(),
        "\u{01F1}"
    );
}

#[test]
fn case_mapping_is_simple_not_full() {
    // C's `Tcl_UtfTo{Upper,Lower,Title}` map one code point to one code
    // point through `Tcl_UniCharTo{Upper,Lower,Title}` (`tclUtf.c:1777-1858`),
    // which hold only Unicode's *simple* mappings — so a Rust *full* mapping
    // that expands has no C counterpart and the character is preserved.
    assert_eq!(simple_upper('\u{00DF}'), '\u{00DF}'); // ß, not SS
    assert_eq!(simple_upper('\u{FB01}'), '\u{FB01}'); // ﬁ, not FI
    // …but an expanding *full* mapping does not imply the character has no
    // simple mapping, and U+0130 is the one place that matters: its full
    // lowercase is `i` + U+0307, while `Tcl_UniCharToLower` maps it to plain
    // `i` (`UnicodeData.txt` field 13 = 0069). Inferring identity from the
    // expansion alone made `string tolower` and `STR_LOWER` disagree with C.
    // It is the only such code point — the only one in Unicode whose full
    // lowercase expands at all.
    assert_eq!(simple_lower('\u{0130}'), 'i');
    // Ordinary 1:1 mappings still apply, including the shrinking ones.
    assert_eq!(simple_upper('a'), 'A');
    assert_eq!(simple_lower('É'), 'é');
    assert_eq!(simple_upper('\u{0131}'), 'I'); // ı (2 bytes) → I (1 byte)
    // C's length guard keeps a mapping that would *grow* the UTF-8 encoding
    // ("only copy the upper case char to dst if its size is <= the original
    // char", `tclUtf.c:1338-1345`): ɐ (U+0250, 2 bytes) does not become
    // Ɐ (U+2C6F, 3 bytes).
    assert_eq!(simple_upper('\u{0250}'), '\u{0250}');
    // Titlecase differs from uppercase only for the Latin digraphs.
    assert_eq!(simple_title('\u{01F3}'), '\u{01F2}'); // dz → Dž
    assert_eq!(simple_title('h'), 'H');
    // `Tcl_UtfToTitle`'s rest-of-string rule lowercases, except the Georgian
    // Mtavruli block it deliberately skips (`tclUtf.c:1452-1456`).
    assert_eq!(simple_title_rest('A'), 'a');
    assert_eq!(simple_title_rest('\u{1C90}'), '\u{1C90}');
    assert_eq!(simple_title_rest('\u{1CC0}'), '\u{1CC0}'); // just past the block
}

#[test]
fn word_end_finds_word_boundaries() {
    // Tcl `string wordend` — end is one past the last word char.
    let c = chars("abc def");
    assert_eq!(word_end(&c, 1), 3); // end of "abc" (exclusive)
    assert_eq!(word_end(&c, 4), 7); // end of "def"
    assert_eq!(word_end(&c, 3), 4); // a non-word char advances exactly one
    assert_eq!(word_end(&c, 100), 7); // past end → length
}

#[test]
fn string_last_empty_haystack_is_minus_one_not_panic() {
    // Regression: `string last a {} 0` clamped `last` to 0 and then sliced
    // an empty haystack, panicking. An empty haystack holds no match → -1.
    assert_eq!(last_of("a", "", Some("0")), -1);
    assert_eq!(last_of("a", "", None), -1);
    assert_eq!(last_of("abc", "", Some("end")), -1);
}

#[test]
fn string_last_finds_last_occurrence() {
    assert_eq!(last_of("a", "banana", None), 5);
    assert_eq!(last_of("an", "banana", None), 3);
    // Bounded search: last "an" ending at or before index 2.
    assert_eq!(last_of("an", "banana", Some("2")), 1);
    // No match → -1.
    assert_eq!(last_of("z", "banana", None), -1);
}

fn repeat_of(s: &str, count: &str) -> Result<String, CmdError> {
    let mut ops = StrOps;
    repeat(&mut ops, &s.to_owned(), &count.to_owned())
}

#[test]
fn string_repeat_bounds_the_result_before_allocating() {
    // Reject the oversized result before allocating its 200 GB payload.
    // Original C9.0.4/9.1b0: string size overflow: unable to alloc 200000000000 bytes
    let err = repeat_of("ab", "100000000000").expect_err("bounded");
    assert_eq!(
        err.message().unwrap(),
        "string size overflow: unable to alloc 200000000000 bytes"
    );
    // The byte count is the *byte* length of the string rep, not the
    // character count: tclsh 9.0.4 reports `string repeat \u00e9
    // 100000000000` as 200000000000 bytes too.
    let err = repeat_of("\u{e9}", "100000000000").expect_err("bounded");
    assert_eq!(
        err.message().unwrap(),
        "string size overflow: unable to alloc 200000000000 bytes"
    );
    // `count * length` is computed in `Tcl_Size` and wraps, and C reports
    // the wrapped value verbatim — tclsh 9.0.4:
    //   % string repeat ab 5000000000000000000
    //   string size overflow: unable to alloc -8446744073709551616 bytes
    let err = repeat_of("ab", "5000000000000000000").expect_err("bounded");
    assert_eq!(
        err.message().unwrap(),
        "string size overflow: unable to alloc -8446744073709551616 bytes"
    );
    // `count > TCL_SIZE_MAX - 1` is a different message — tclsh 9.0.4:
    //   % string repeat ab 9223372036854775807
    //   max size for a Tcl value (9223372036854775807 bytes) exceeded
    let err = repeat_of("ab", "9223372036854775807").expect_err("bounded");
    assert_eq!(
        err.message().unwrap(),
        "max size for a Tcl value (9223372036854775807 bytes) exceeded"
    );
    // …but an empty string short-circuits before either check, so even that
    // count is the empty string and not an error (tclsh 9.0.4:
    // `string repeat {} 9223372036854775807` → {}).
    assert_eq!(repeat_of("", "9223372036854775807").unwrap(), "");
    // Ordinary repeats are untouched.
    assert_eq!(repeat_of("ab", "3").unwrap(), "ababab");
    assert_eq!(repeat_of("ab", "0").unwrap(), "");
    assert_eq!(repeat_of("ab", "-1").unwrap(), "");
}
