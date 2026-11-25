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

//! Registry fold adapters: policy and command evidence projected to shared value kernels.

/// Canonical list serialisation for registry callbacks and `SpecTcl` `foldlist`.
/// Materialised output shares the constant-evaluation byte budget.
#[must_use]
pub fn fold_list(args: &[&str]) -> Option<String> {
    let value = tcl_cmd_core::literal_value::fold_list(args);
    crate::value_transfer::Budget::evaluation()
        .charge_result(value.len() as u64)
        .ok()?;
    Some(value)
}
pub(crate) use tcl_cmd_core::literal_value::{
    NativeIndexSelection, clamp_range, dictionary_argument_pairs, fold_concat, fold_dict_create,
    fold_dict_exists, fold_dict_get, fold_dict_keys, fold_dict_merge, fold_dict_merge_versioned,
    fold_dict_size, fold_dict_values, fold_join, fold_lindex, fold_llength, fold_lrepeat,
    fold_lreverse, fold_lsort, parse_element_index, parse_index, parse_range, repetition_count,
    split_list,
};

fn context(dialect: crate::InvocationDialect) -> tcl_cmd_core::literal_value::LiteralContext {
    tcl_cmd_core::literal_value::LiteralContext {
        numbers: dialect.numbers,
        lists: dialect.word_values.list,
        escapes: dialect.lexer_grammar.escapes,
        protocol: dialect.native_string_protocol(),
    }
}

pub(crate) fn fold_lreverse_in(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
    tcl_cmd_core::literal_value::fold_lreverse_in(args, dialect.map(context))
}

pub(crate) fn fold_lsort_in(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
) -> Option<String> {
    tcl_cmd_core::literal_value::fold_lsort_in(args, dialect.map(context))
}

pub(crate) fn fold_lrepeat_in(args: &[&str], dialect: crate::InvocationDialect) -> Option<String> {
    tcl_cmd_core::literal_value::fold_lrepeat_in(args, context(dialect))
}

pub(crate) fn fold_dict_members(
    args: &[&str],
    dialect: Option<crate::InvocationDialect>,
    command: crate::native_dictionary::NativeDictionaryCommand,
) -> Option<String> {
    use crate::native_dictionary::NativeDictionaryCommand;
    use tcl_cmd_core::literal_value::DictionaryMember;
    let member = match command {
        NativeDictionaryCommand::Keys => DictionaryMember::Keys,
        NativeDictionaryCommand::Values => DictionaryMember::Values,
        _ => return None,
    };
    tcl_cmd_core::literal_value::fold_dict_members(args, dialect.map(context), member)
}

pub(crate) fn fold_split(args: &[&str]) -> Option<String> {
    fold_split_versioned(args, None)
}

pub(crate) fn fold_split_versioned(
    args: &[&str],
    version: Option<tcl_dialect::TclVersion>,
) -> Option<String> {
    crate::value_transfer::evaluate_literal(
        &crate::value_transfer::builtins::SPLIT,
        "split",
        None,
        args,
        version,
    )
}

pub(crate) fn fold_lrange(args: &[&str]) -> Option<String> {
    let [source, first, last] = args else {
        return None;
    };
    let length = split_list(source)?.len();
    parse_range(first, last, length)?;

    let contract = crate::native_result::NativeResultContract::ListRange {
        list_at: 0,
        first_at: 1,
        last_at: 2,
    };
    let dialects = tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(crate::InvocationDialect::for_version)
        .chain([crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )]);
    let mut result = None;
    for dialect in dialects {
        let arguments = crate::InvocationArguments::literals(args).with_dialect(dialect);
        let value = contract
            .select(arguments, 0)
            .constant_range_literal_result(arguments)?;
        if result.as_ref().is_some_and(|previous| previous != &value) {
            return None;
        }
        result = Some(value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_cmd_core::literal_value::list_join;

    /// A minimal string-backed `ValueOps` so these folds can be cross-checked
    /// against the **real** canonicalisation owner rather than against a second
    /// copy of my own reasoning. Values are `Rc<str>`; lists are the elements
    /// this module's own `split_list` finds, so both sides see identical input.
    #[derive(Default)]
    struct Strs;

    impl tcl_cmd_core::index::NativeIndexOps for Strs {
        type IndexOperation = ();
        type IndexBinaryReceipt = ();
        type IndexKeyOwner = Self::Value;
        fn index_borrowed_key(
            &mut self,
            value: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(value.clone())
        }
        fn index_created_key(
            &mut self,
            value: Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(value)
        }
        fn index_key_value(owner: &Self::Value) -> &Self::Value {
            owner
        }
        fn index_hold_key_result(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(owner.clone())
        }
        fn index_finish_held_key_result(
            &mut self,
            held: Self::Value,
        ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
            Ok(tcl_cmd_core::index::IndexResult::Detached(held))
        }
        fn index_key_argument(
            &mut self,
            owner: &Self::Value,
        ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
            Ok(owner.clone())
        }
        fn index_key_result(
            &mut self,
            owner: &Self::Value,
            created: bool,
            ancestor: bool,
        ) -> Result<tcl_cmd_core::index::IndexResult<Self::Value>, tcl_cmd_core::CmdError> {
            Ok(if created {
                tcl_cmd_core::index::IndexResult::Created(owner.clone())
            } else if ancestor {
                tcl_cmd_core::index::IndexResult::Detached(owner.clone())
            } else {
                tcl_cmd_core::index::IndexResult::Borrowed(owner.clone())
            })
        }
        fn enter_index_operation(&mut self) -> Result<(), tcl_cmd_core::CmdError> {
            Ok(())
        }
        fn check_index_operation(&mut self, (): &()) -> Result<(), tcl_cmd_core::CmdError> {
            Ok(())
        }
        fn retain_index_failure(&mut self, _error: &tcl_cmd_core::CmdError) {
            // Pure neutral adapter has no interpreter first-cause owner.
        }
        fn discard_owned_index_result(&mut self, created: Self::Value) {
            drop(created);
        }
    }

    impl tcl_syntax::value::ValueOps for Strs {
        type Value = std::rc::Rc<str>;

        fn new_str(&mut self, s: &str) -> Self::Value {
            std::rc::Rc::from(s)
        }
        fn new_int(&mut self, n: i64) -> Self::Value {
            std::rc::Rc::from(n.to_string().as_str())
        }
        fn new_double(&mut self, f: f64) -> Self::Value {
            std::rc::Rc::from(f.to_string().as_str())
        }
        fn new_bool(&mut self, b: bool) -> Self::Value {
            std::rc::Rc::from(if b { "1" } else { "0" })
        }
        fn new_list(&mut self, items: Vec<Self::Value>) -> Self::Value {
            std::rc::Rc::from(
                list_join(
                    &items
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>(),
                )
                .as_str(),
            )
        }
        fn as_bytes(&mut self, v: &Self::Value) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
            self.new_str(std::str::from_utf8(bytes).expect("Unicode-only fixture input"))
        }

        fn as_int(&mut self, v: &Self::Value) -> Result<i64, tcl_syntax::value::ValueError> {
            v.parse::<i64>()
                .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.to_string()))
        }
        fn as_double(&mut self, v: &Self::Value) -> Result<f64, tcl_syntax::value::ValueError> {
            v.parse::<f64>()
                .map_err(|_| tcl_syntax::value::ValueError::NotDouble(v.to_string()))
        }
        fn as_bool(&mut self, v: &Self::Value) -> Result<bool, tcl_syntax::value::ValueError> {
            Ok(!matches!(v.as_ref(), "0" | "false" | "no" | "off" | ""))
        }
        fn list_elements(
            &mut self,
            v: &Self::Value,
        ) -> Result<Vec<Self::Value>, tcl_syntax::value::ValueError> {
            match split_list(v) {
                Some(elems) => Ok(elems
                    .into_iter()
                    .map(|e| std::rc::Rc::from(e.as_str()))
                    .collect()),
                None => Err(tcl_syntax::value::ValueError::BadList(
                    "unparsable list".to_string(),
                )),
            }
        }
    }

    /// These registry const-folds decode a dict
    /// **string** and must canonicalise duplicate keys exactly as the runtime
    /// does, or the O129 folds change program results.
    ///
    /// Each expectation is the byte-exact output of `tclsh9.0.4` (and
    /// `tclsh8.6.16`, which agrees on every row), and `parse_dict` is
    /// additionally cross-checked pair-for-pair against
    /// [`tcl_syntax::value::ValueOps::dict_pairs`] — the owner — on the same
    /// inputs, so the two cannot drift apart silently.
    #[test]
    fn dict_folds_canonicalise_duplicate_keys_like_the_owner() {
        use tcl_syntax::value::ValueOps as _;

        // Cross-check against the owner on the same inputs.
        for src in [
            "a 1 a 2",
            "x 1 x 2 y 3",
            "a 1 b 2 a 3",
            "a 1 b 2",
            "",
            "k {v 1 v 2}",
        ] {
            let mine = tcl_cmd_core::literal_value::parse_dict(src).expect("parses");
            let mut ops = Strs;
            let v: std::rc::Rc<str> = std::rc::Rc::from(src);
            let theirs = ops.dict_pairs(&v).expect("owner parses");
            let theirs: Vec<(String, String)> = theirs
                .into_iter()
                .map(|(k, val)| (k.to_string(), val.to_string()))
                .collect();
            assert_eq!(
                mine, theirs,
                "parse_dict disagrees with the owner on {src:?}"
            );
        }

        // Oracle rows for the folds the duplicate bug actually reached.
        assert_eq!(fold_dict_get(&["a 1 a 2", "a"]).unwrap(), "2");
        assert_eq!(fold_dict_get(&["x 1 x 2 y 3", "x"]).unwrap(), "2");
        assert_eq!(fold_dict_size(&["a 1 a 2"]).unwrap(), "1");
        assert_eq!(fold_dict_size(&["x 1 x 2 y 3"]).unwrap(), "2");
        assert_eq!(fold_dict_keys(&["a 1 a 2"]).unwrap(), "a");
        assert_eq!(fold_dict_keys(&["x 1 x 2 y 3"]).unwrap(), "x y");
        assert_eq!(fold_dict_values(&["a 1 a 2"]).unwrap(), "2");
        assert_eq!(fold_dict_values(&["x 1 x 2 y 3"]).unwrap(), "2 3");
        assert_eq!(fold_dict_exists(&["a 1 a 2", "a"]).unwrap(), "1");
        // First-occurrence position survives a later duplicate.
        assert_eq!(fold_dict_keys(&["a 1 b 2 a 3"]).unwrap(), "a b");
        assert_eq!(fold_dict_values(&["a 1 b 2 a 3"]).unwrap(), "3 2");
        // `dict get` returns a nested value verbatim — it is not re-canonicalised.
        assert_eq!(fold_dict_get(&["o {a 1 a 2}", "o"]).unwrap(), "a 1 a 2");

        // `dict merge` only canonicalises when a pair is actually merged in.
        assert_eq!(fold_dict_merge(&[]).unwrap(), "");
        assert_eq!(fold_dict_merge(&["a 1 a 2"]).unwrap(), "a 1 a 2");
        assert_eq!(fold_dict_merge(&["x 1 x 2 y 3"]).unwrap(), "x 1 x 2 y 3");
        assert_eq!(fold_dict_merge(&["a 1 a 2", ""]).unwrap(), "a 1 a 2");
        assert_eq!(fold_dict_merge(&["a 1 a 2", "b 9"]).unwrap(), "a 2 b 9");
        assert_eq!(fold_dict_merge(&["a 1", "a 2 a 3"]).unwrap(), "a 3");
        assert_eq!(fold_dict_merge(&["", "a 1 a 2"]).unwrap(), "a 2");

        // `dict create` already had the rule; it must stay agreeing.
        assert_eq!(fold_dict_create(&["a", "1", "a", "2"]).unwrap(), "a 2");
    }

    #[test]
    fn split_list_handles_braces_quotes_bare() {
        assert_eq!(split_list("a b c").unwrap(), ["a", "b", "c"]);
        assert_eq!(split_list("{a b} c").unwrap(), ["a b", "c"]);
        assert_eq!(split_list("{a {b c}} d").unwrap(), ["a {b c}", "d"]);
        assert_eq!(split_list("\"a b\" c").unwrap(), ["a b", "c"]);
        assert_eq!(split_list("").unwrap(), Vec::<String>::new());
        // Malformed → None.
        assert_eq!(split_list("{a b"), None, "unbalanced brace");
        assert_eq!(split_list("\"a b"), None, "unterminated quote");
        assert_eq!(split_list("a\\ b"), None, "backslash bails");
        assert_eq!(split_list("{a}b"), None, "junk after brace");
    }

    #[test]
    fn list_element_quotes_like_tcl() {
        use tcl_syntax::list::list_element;
        assert_eq!(list_element("foo"), "foo");
        assert_eq!(list_element(""), "{}");
        assert_eq!(list_element("a b"), "{a b}");
        assert_eq!(list_element("a$b"), "{a$b}");
    }

    /// A leading `#` is comment-unsafe only in **list position 0**
    /// (`Tcl_Merge` passes `TCL_DONT_QUOTE_HASH` for every later element), so
    /// the folds that re-render a list must not quote it everywhere. Each row
    /// is the byte-exact output of `tclsh8.6.16` and `tclsh9.0.4`, which agree.
    #[test]
    fn lsort_folds_only_byte_ordered_ascii_sorts() {
        assert_eq!(fold_lsort(&["c b a"]).as_deref(), Some("a b c"));
        assert_eq!(
            fold_lsort(&["-ascii", "-increasing", "B a b"]).as_deref(),
            Some("B a b")
        );
        assert_eq!(
            fold_lsort(&["-decreasing", "-unique", "b a b"]).as_deref(),
            Some("b a")
        );
        assert_eq!(fold_lsort(&["{b c} a"]).as_deref(), Some("a {b c}"));
        for option in [
            "-integer",
            "-real",
            "-dictionary",
            "-nocase",
            "-command",
            "-index",
        ] {
            assert_eq!(fold_lsort(&[option, "b a"]), None, "{option}");
        }
        assert_eq!(fold_lsort(&[]), None);
    }

    #[test]
    fn short_list_results_keep_receiver_spelling_and_dictionary_members_use_its_writer() {
        use crate::native_dictionary::NativeDictionaryCommand::{Keys, Values};
        use tcl_dialect::model::{DialectPoint, Release};
        let jim = crate::InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let c86 = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let c91 = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        for (dialect, reversed, sorted, members) in [
            (jim, "abc", " {abc} ", r"a\]\ b"),
            (c86, "abc", "abc", "{a] b}"),
            (c91, " {abc} ", "abc", "{a] b}"),
        ] {
            assert_eq!(
                fold_lreverse_in(&[" {abc} "], Some(dialect)).as_deref(),
                Some(reversed)
            );
            assert_eq!(
                fold_lsort_in(&[" {abc} "], Some(dialect)).as_deref(),
                Some(sorted)
            );
            assert_eq!(
                fold_dict_members(&["{a] b} x"], Some(dialect), Keys).as_deref(),
                Some(members)
            );
            assert_eq!(
                fold_dict_members(&["x {a] b}"], Some(dialect), Values).as_deref(),
                Some(members)
            );
        }
        assert_eq!(fold_lreverse(&[" {abc} "]), None);
        assert_eq!(fold_lsort(&[" {abc} "]), None);
    }

    #[test]
    fn folded_lists_quote_a_leading_hash_only_in_position_zero() {
        assert_eq!(fold_list(&["a", "#"]).as_deref(), Some("a #"));
        assert_eq!(fold_list(&["#", "a"]).as_deref(), Some("{#} a"));
        assert_eq!(fold_lreverse(&["# a"]).as_deref(), Some("a #"));
        assert_eq!(fold_lsort(&["a #"]), None);
        assert_eq!(fold_lsort(&["-decreasing", "a #"]).as_deref(), Some("a #"));
        assert_eq!(fold_lrepeat(&["2", "#"]).as_deref(), Some("{#} #"));
        // A version-neutral fold cannot choose between native C84's bare
        // leading hash and the protected C85+/Jim list-object bytes.
        assert_eq!(fold_lrange(&["x # y", "1", "2"]), None);
        assert_eq!(fold_split(&["a-#", "-"]).as_deref(), Some("a #"));
        assert_eq!(fold_concat(&["a", "#"]), "a #");
        assert_eq!(fold_dict_keys(&["a 1 # 2"]).as_deref(), Some("a #"));
        assert_eq!(fold_dict_values(&["a # b 1"]).as_deref(), Some("{#} 1"));
        assert_eq!(
            fold_dict_create(&["a", "1", "#", "2"]).as_deref(),
            Some("a 1 # 2")
        );
    }

    #[test]
    fn dictionary_kernels_keep_unknown_parses_distinct_from_missing_keys() {
        assert_eq!(fold_dict_exists(&[r"a\ b 1", "a b"]), None);
        assert_eq!(fold_dict_get(&["a 1 b"]), None);
        for release in tcl_dialect::TclVersion::ALL {
            if release == tcl_dialect::TclVersion::V8_4 {
                continue;
            }
            let Some(interpreter) = tcl_test_support::witness_tclsh(release) else {
                continue;
            };
            let script = br"puts [dict exists {a\ b 1} {a b}]
puts [catch {dict get {a 1 b}}]
";
            let result = tcl_test_support::run_script(&interpreter.path, script)
                .unwrap()
                .strict_text()
                .unwrap();
            assert_eq!(result, "1\n1", "{release:?}");
        }
    }

    #[test]
    fn list_folds_match_tcl() {
        assert_eq!(fold_concat(&["a", " b ", "c"]), "a b c");
        assert_eq!(fold_concat(&["{a b}", "c"]), "{a b} c");
        assert_eq!(fold_concat(&["a", "b\\ "]), "a b\\ ");
        assert_eq!(fold_list(&["a", "b c"]).as_deref(), Some("a {b c}"));
        assert_eq!(fold_llength(&["a b c"]).as_deref(), Some("3"));
        assert_eq!(fold_llength(&["{a b} c"]).as_deref(), Some("2"));
        assert_eq!(fold_lreverse(&["a b c"]).as_deref(), Some("c b a"));
        assert_eq!(fold_lreverse(&["{a b} c"]).as_deref(), Some("c {a b}"));
        assert_eq!(fold_join(&["a b c", "-"]).as_deref(), Some("a-b-c"));
        assert_eq!(fold_join(&["{a b} c"]).as_deref(), Some("a b c"));
        assert_eq!(fold_split(&["a,b,c", ","]).as_deref(), Some("a b c"));
        assert_eq!(fold_split(&["a b,c", ","]).as_deref(), Some("{a b} c"));
        // the default split set includes `\r`. `split "a\r\nb"`
        // → tclsh `a {} b` (empty element between \r and \n).
        assert_eq!(fold_split(&["a\r\nb"]).as_deref(), Some("a {} b"));
        assert_eq!(fold_split(&["a b\tc"]).as_deref(), Some("a b c"));
        assert_eq!(fold_lrepeat(&["3", "x"]).as_deref(), Some("x x x"));
        assert_eq!(fold_lrepeat(&["2", "a", "b"]).as_deref(), Some("a b a b"));
        assert_eq!(fold_lindex(&["a b c", "1"]).as_deref(), Some("b"));
        assert_eq!(fold_lindex(&["{a b} c", "0"]).as_deref(), Some("a b"));
        assert_eq!(fold_lindex(&["a b c", "9"]).as_deref(), Some(""));
        assert_eq!(fold_lrange(&["a b c d", "1", "2"]).as_deref(), Some("b c"));
        assert_eq!(
            fold_lrange(&["{a b} c d", "0", "1"]).as_deref(),
            Some("{a b} c")
        );
        // Malformed list arg → no fold.
        assert_eq!(fold_llength(&["{a b"]), None);
    }

    #[test]
    fn range_folding_uses_actual_native_serialization_and_portable_agreement() {
        let args = ["x #value end", "1", "1"];
        assert_eq!(fold_lrange(&args), None);
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let supplied = crate::model::ingress::static_context_for(profile);
            let registry = supplied.commands();
            let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
            let resolved = registry
                .resolve_call("lrange", &args, dialect.authoring_query())
                .unwrap();
            assert_eq!(
                resolved.spec.run_const_fold(&args, dialect.tcl_version),
                None
            );
            assert_eq!(
                resolved.spec.run_const_fold_in(&args, dialect).as_deref(),
                Some(if profile == "tcl8.4" {
                    "#value"
                } else {
                    "{#value}"
                }),
                "{profile}"
            );
        }
        assert_eq!(fold_lrange(&["x a b", "1", "2"]).as_deref(), Some("a b"));
        assert_eq!(fold_lrange(&["x a b", "9", "end"]).as_deref(), Some(""));
        assert_eq!(fold_lrange(&["x a b", "invalid", "end"]), None);
    }

    #[test]
    fn index_folds_match_tclsh_oracle() {
        // The unversioned callback preserves only the native grammar
        // intersection. C8.4 rejects these arithmetic forms, so it must decline
        // even though a selected modern grammar can resolve both to index 2.
        assert_eq!(fold_lindex(&["a b c d e", "1+1"]), None);
        assert_eq!(fold_lindex(&["a b c d e", "3-1"]), None);
        let selected = tcl_dialect::IndexSyntax::for_version(tcl_dialect::TclVersion::V8_6);
        for index in ["1+1", "3-1"] {
            assert_eq!(
                tcl_cmd_core::index::resolve_opt_in(index, 5, selected),
                Some(2)
            );
        }
        assert_eq!(fold_lindex(&["a b c d e", "0x2"]).as_deref(), Some("c"));
        assert_eq!(fold_lindex(&["a b c d e", "end-1"]).as_deref(), Some("d"));
        // C end+1 and Jim's encoded positive-end sentinel both return empty.
        // Raw index equality remains unknown; container result equality holds.
        assert_eq!(parse_index("end--1", 5), None);
        assert_eq!(fold_lindex(&["a b c d e", "end--1"]).as_deref(), Some(""));
        assert_eq!(
            fold_lrange(&["a b c d e", "0", "end--1"]).as_deref(),
            Some("a b c d e")
        );
        assert_eq!(
            fold_lrange(&["a b c d e", "end--1", "end"]).as_deref(),
            Some("")
        );
        assert_eq!(
            fold_lrange(&["a b c d e", "2", "end--1"]).as_deref(),
            Some("c d e")
        );
        assert_eq!(fold_lrange(&["a b c d e", "1+1", "end"]), None);
        assert_eq!(
            fold_lrange(&["a b c d e", "2", "end"]).as_deref(),
            Some("c d e")
        );
        // Still declines genuinely bad specs.
        assert_eq!(fold_lindex(&["a b c d e", "1.0"]), None);
        assert_eq!(fold_lindex(&["a b c d e", "foo"]), None);
    }

    // The callback accepts empty and non-empty lists within its result budget.
    #[test]
    fn fold_list_preserves_empty_and_quoted_values() {
        assert_eq!(fold_list(&["a", "b c"]).as_deref(), Some("a {b c}"));
        assert_eq!(fold_list(&[]).as_deref(), Some(""));
        assert!(fold_list(&["x"]).is_some());
    }

    #[test]
    fn dict_folds_match_tcl() {
        assert_eq!(fold_dict_get(&["a 1 b 2", "b"]).as_deref(), Some("2"));
        assert_eq!(fold_dict_get(&["a 1 b 2", "z"]), None, "missing key");
        assert_eq!(fold_dict_exists(&["a 1 b 2", "b"]).as_deref(), Some("1"));
        assert_eq!(fold_dict_exists(&["a 1 b 2", "z"]).as_deref(), Some("0"));
        assert_eq!(fold_dict_size(&["a 1 b 2"]).as_deref(), Some("2"));
        assert_eq!(fold_dict_keys(&["a 1 b 2"]).as_deref(), Some("a b"));
        assert_eq!(fold_dict_values(&["a 1 b 2"]).as_deref(), Some("1 2"));
        // dict create de-dups, last value wins, position preserved.
        assert_eq!(
            fold_dict_create(&["a", "X", "b", "Y", "a", "Z"]).as_deref(),
            Some("a Z b Y")
        );
        assert_eq!(
            fold_dict_merge(&["a 1 b 2", "b 9 c 3"]).as_deref(),
            Some("a 1 b 9 c 3")
        );
        // Odd dict → no fold.
        assert_eq!(fold_dict_size(&["a 1 b"]), None);
    }

    /// A plain `ConstFoldFn` carries no release, so an index whose value depends
    /// on the release must not fold. Declining costs an optimisation; folding
    /// under the wrong grammar would bake a wrong constant into the program.
    #[test]
    fn index_folds_decline_when_the_releases_disagree() {
        // `010` is index 8 up to 8.6 and 10 from 9.0 — no single answer.
        assert_eq!(parse_index("010", 12), None);
        assert_eq!(parse_index("end-010", 12), None);
        // `1_0` and `0d1` are 9.0-only spellings: valid there, `bad index` before.
        assert_eq!(parse_index("1_0", 12), None);
        assert_eq!(parse_index("0d1", 12), None);
        // Unanimous spellings still fold.
        assert_eq!(parse_index("1", 12), Some(1));
        assert_eq!(parse_index("0x1", 12), Some(1));
        assert_eq!(parse_index("007", 12), Some(7));
        assert_eq!(parse_index("end", 12), Some(11));
        assert_eq!(parse_index("end-2", 12), Some(9));
        // Still nothing at all for a genuinely bad spec.
        assert_eq!(parse_index("nope", 12), None);
        // Past 32 bits 8.4 to 8.6 wrap or raise where 9.0 reads the wide, and
        // a 64-bit `long` decides the rest.
        for spec in [
            "2147483648",
            "4294967295",
            "4294967296",
            "-4294967295",
            "end-4294967295",
            "1+2147483647",
            "18446744073709551615",
            "9223372036854775808",
        ] {
            assert_eq!(parse_index(spec, 12), None, "{spec}");
        }
        assert_eq!(parse_index("2147483647", 12), Some(2_147_483_647));
        // Every grammar reads this one before the first element, but 8.6
        // encodes the literal after the end.
        assert_eq!(parse_index("end-2147483649", 12), None);
    }

    /// Fold only unanimous container outcomes, including clamped ranges whose
    /// raw native indices differ. A genuinely differing result still declines.
    #[test]
    fn lindex_folds_only_unanimous_indices() {
        assert_eq!(
            fold_lindex(&["a b c d e f g h i j k l", "1"]).as_deref(),
            Some("b")
        );
        assert_eq!(fold_lindex(&["a b c d e f g h i j k l", "010"]), None);
        assert_eq!(fold_lrange(&["a b c d", "1", "2"]).as_deref(), Some("b c"));
        assert_eq!(
            fold_lrange(&["a b c d", "1", "010"]).as_deref(),
            Some("b c d")
        );
        assert_eq!(fold_lrange(&["a b c d e f g h i j k l", "1", "010"]), None);
        assert_eq!(
            fold_lindex(&["a b c d e f g h i j k l", "4294967295"]),
            None
        );
        assert_eq!(fold_lrange(&["a b c d", "0", "4294967296"]), None);
        assert_eq!(fold_lrange(&["a b c d", "0", "end-2147483649"]), None);
    }
}
