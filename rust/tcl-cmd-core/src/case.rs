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

//! `case` clause selection (Tcl 8.x), the one owner of that decision.
//!
//! `case` is `switch`'s obsolete ancestor, present in 8.4 to 8.6 and
//! removed in 9.0. Like [`crate::switch`], only its **decision** lives
//! here: which clause's body runs for a subject. The caller reads the
//! subject, skips the optional `in` word, splits the one-word clause list,
//! and runs the chosen body itself; today the registry's `case` selection
//! contract is that caller.
//!
//! Mirrors C's `Tcl_CaseObjCmd` (`tclCmdAH.c`, the same loop in 8.4.20,
//! 8.5.19 and 8.6.18): every clause compares with `Tcl_StringMatch`'s
//! case-sensitive glob; a pattern word holding whitespace or a backslash is
//! a list of patterns, any of which selects; a single `default` pattern is
//! the fallback wherever it stands and is still matched as a pattern there;
//! the first clause that matches wins; a pattern with no body is an error
//! only once the scan reaches it; and there is no fall-through body — a body
//! spelled `-` is a command like any other.

use tcl_syntax::glob::string_case_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// The spelling of the fallback pattern.
const DEFAULT: &str = "default";

/// Whether `case` reads the pattern word `pattern` as a list of patterns:
/// it holds whitespace or a backslash. The whitespace is the six bytes
/// 8.4's `isspace` in the C locale and 8.5's and 8.6's `TclIsSpaceProc`
/// agree on; a word holding none of them, and no backslash, is one
/// pattern.
#[must_use]
pub fn splits_as_list(pattern: &str) -> bool {
    pattern
        .bytes()
        .any(|byte| matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b'\\'))
}

/// The clause whose body `case` runs for `value`, as its index among the
/// pattern/body pairs of `clauses` — the words after the subject and any
/// `in`, the one-word form already split into its elements — or `None`
/// when nothing matches and no `default` stands.
///
/// # Errors
/// `extra case pattern with no body` when the scan reaches a final pattern
/// with no body, and a pattern list's own list error.
pub fn select<O, V>(ops: &mut O, value: &V, clauses: &[V]) -> Result<Option<usize>, CmdError>
where
    O: ValueOps<Value = V>,
{
    let subject = ops.as_str(value);
    let mut fallback = None;
    for (index, pattern) in clauses.iter().enumerate().step_by(2) {
        if index + 1 == clauses.len() {
            return Err(CmdError::new("extra case pattern with no body"));
        }
        let arm = index / 2;
        let text = ops.as_str(pattern);
        if !splits_as_list(&text) {
            if &*text == DEFAULT {
                fallback = Some(arm);
            }
            if string_case_match(&text, &subject, false) {
                return Ok(Some(arm));
            }
            continue;
        }
        for element in ops.list_elements(pattern)? {
            let element = ops.as_str(&element);
            if string_case_match(&element, &subject, false) {
                return Ok(Some(arm));
            }
        }
    }
    Ok(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A string-only `ValueOps`, as the switch core's tests use: patterns
    /// and subjects as text, lists split on whitespace with braces kept
    /// literal — enough for the list patterns below.
    #[derive(Default)]
    struct StrOps;

    impl ValueOps for StrOps {
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
        fn as_str(&mut self, v: &String) -> std::rc::Rc<str> {
            std::rc::Rc::from(v.as_str())
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
        fn list_elements(
            &mut self,
            v: &String,
        ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
            tcl_syntax::list::split_list(v)
                .map(|elements| elements.iter().map(ToString::to_string).collect())
                .map_err(|error| tcl_syntax::value::ValueError::BadList(error.message().to_owned()))
        }
    }

    /// A pattern word is a list when it holds one of the six whitespace
    /// bytes or a backslash, as `Tcl_CaseObjCmd`'s scan reads it.
    #[test]
    fn a_pattern_holding_whitespace_or_a_backslash_is_a_list() {
        for list in [
            "x a*", "a\\*", "a\tb", "a\nb", "a\u{0b}b", "a\u{0c}b", "a\rb", " ",
        ] {
            assert!(splits_as_list(list), "{list:?}");
        }
        for single in ["a*", "", "default", "[a-c]*", "a\u{a0}b"] {
            assert!(!splits_as_list(single), "{single:?}");
        }
    }

    fn chosen(value: &str, clauses: &[&str]) -> Result<Option<usize>, String> {
        let clauses: Vec<String> = clauses.iter().map(|&word| word.to_owned()).collect();
        select(&mut StrOps, &value.to_owned(), &clauses).map_err(|error| error.message().to_owned())
    }

    /// Each rule against what tclsh 8.4.20, 8.5.19 and 8.6.18 run: glob
    /// patterns, a pattern list, an escaped pattern read as a list, the
    /// fallback wherever it stands and its literal match, the first match
    /// winning, and the missing body raised only when the scan reaches it.
    #[test]
    fn case_selects_as_tcl_case_obj_cmd_does() {
        assert_eq!(chosen("abc", &["a*", "Y", "default", "N"]), Ok(Some(0)));
        assert_eq!(chosen("zzz", &["a*", "Y", "default", "N"]), Ok(Some(1)));
        assert_eq!(chosen("abc", &["x a*", "L", "default", "N"]), Ok(Some(0)));
        assert_eq!(chosen("abc", &["a\\*", "E", "default", "N"]), Ok(Some(0)));
        assert_eq!(chosen("zzz", &["default", "D", "a", "A"]), Ok(Some(0)));
        assert_eq!(
            chosen("default", &["default", "D", "def*", "X"]),
            Ok(Some(0))
        );
        assert_eq!(chosen("abc", &["abc", "X"]), Ok(Some(0)));
        assert_eq!(chosen("abc", &["-", "D", "default", "N"]), Ok(Some(1)));
        assert_eq!(chosen("abc", &["q", "Q"]), Ok(None));
        // The odd final pattern is raised only once the scan reaches it.
        assert_eq!(chosen("abc", &["a*", "Y", "b*"]), Ok(Some(0)));
        assert_eq!(
            chosen("zzz", &["a*", "Y", "b*"]),
            Err("extra case pattern with no body".to_owned())
        );
    }
}
