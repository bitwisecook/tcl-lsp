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

use crate::counters;
use crate::interp::{Code, Interp};

#[test]
fn selected_dictionary_completion_and_scripted_worker_lookup() {
    let cases: &[(&str, &[u8], &[u8])] = &[
            ("tcl8.6", b"set d {k A};set c [catch {dict update d k v {set v NEW;return BODY}} r];list $c $r $d", b"2 BODY {k NEW}"),
            ("tcl8.6", b"set d {k A};set c [catch {dict update d k v {set d BROKEN}} r];list $c $r $d", b"1 {missing value to go with key} BROKEN"),
            ("jim", b"set d {k A};set c [catch {dict update d k v {set v NEW;return BODY}} r];list $c $r $d", b"0 BODY {k NEW}"),
            ("jim", b"set d {k A};set c [catch {dict with d {set k NEW;return BODY}} r];list $c $r $d", b"2 BODY {k A}"),
            ("jim", b"set d {k A};set c [catch {dict with d {set d BROKEN}} r];list $c $r $d", b"0 {missing value to go with key} BROKEN"),
            ("jim", b"proc {dict update} {args} {return CUSTOM};set d {k A};dict update d k v {}", b"CUSTOM"),
            ("jim", b"namespace eval N {proc {dict update} {args} {return LOCAL};set d {k A};dict update d k v {}}", b"LOCAL"),
            ("jim", b"proc catch {args} {return CUSTOM};set d {k A};catch {dict update d k v {}}", b"CUSTOM"),
        ];
    for &(dialect, source, expected) in cases {
        counters::reset();
        {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(dialect));
            assert_eq!(
                interp.eval_str(source),
                Code::Ok,
                "{dialect}: {} => {}",
                String::from_utf8_lossy(source),
                String::from_utf8_lossy(&interp.result_bytes())
            );
            assert_eq!(
                interp.result_bytes(),
                expected,
                "{dialect}: {}",
                String::from_utf8_lossy(source)
            );
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }
}

// Native proof: naming.variable.dictionary-missing-receiver-and-array-default-phases
// docs/design/analysis/name-resolution-proofs/variable.dictionary-missing-receiver-and-array-default-phases.md
#[test]
fn dictionary_receiver_and_defaults_match_original_native_scripts() {
    fn unhex(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut measured = 0;
    for row in include_str!("../../tests/data/native_dictionary_receiver/controls.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let version = match fields[0] {
            "8.5.19" => tcl_dialect::TclVersion::V8_5,
            "8.6.18" => tcl_dialect::TclVersion::V8_6,
            "9.0.4" => tcl_dialect::TclVersion::V9_0,
            "9.1.0" => tcl_dialect::TclVersion::V9_1,
            _ => panic!("unselected native fixture"),
        };
        counters::reset();
        {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(
                interp.eval_str(&unhex(fields[2])),
                Code::Ok,
                "{} {}: {:?}",
                fields[0],
                fields[1],
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[3]),
                "{} {}",
                fields[0],
                fields[1]
            );
        }
        assert_eq!(counters::finalize(), 0, "{} {}", fields[0], fields[1]);
        assert_eq!(counters::double_free_count(), 0);
        measured += 1;
    }
    assert_eq!(measured, 18);
}

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

/// `dict` is a `TclMakeEnsemble` command, so its exact-then-unique-prefix
/// scan and its whole miss sentence belong to `tcl_cmd_core::ensemble`
/// rather than a hand-rolled 22-entry literal. `dict filter`'s type word
/// is a `Tcl_GetIndexFromObj(…, "filterType", 0)` table in the shared
/// core.
///
/// tclsh 9.0.4:
///   dict k {a 1}          -> a       ;  dict si {a 1} -> 1
///   dict g {a 1} a        -> unknown or ambiguous subcommand "g": must be
///                            append, create, exists, filter, for, get,
///                            getdef, getwithdefault, incr, info, keys,
///                            lappend, map, merge, remove, replace, set,
///                            size, unset, update, values, or with
///   dict {} {a 1}         -> unknown or ambiguous subcommand "": must be <same>
///   dict filter {a 1} k * -> a 1
///   dict filter {a 1} {} *-> ambiguous filterType "": must be key, script, or value
#[test]
fn dict_ensemble_and_filter_type_resolve_like_tclsh() {
    const MUST: &str = "must be append, create, exists, filter, for, get, getdef, \
                            getwithdefault, incr, info, keys, lappend, map, merge, remove, \
                            replace, set, size, unset, update, values, or with";
    let err_of = |src: &[u8]| {
        let (c, b) = run(src);
        assert_eq!(c, Code::Error, "expected an error");
        String::from_utf8_lossy(&b).into_owned()
    };
    assert_eq!(ok(b"dict k {a 1}"), b"a");
    assert_eq!(ok(b"dict si {a 1}"), b"1");
    assert_eq!(
        err_of(b"dict g {a 1} a"),
        format!("unknown or ambiguous subcommand \"g\": {MUST}")
    );
    assert_eq!(
        err_of(b"dict {} {a 1}"),
        format!("unknown or ambiguous subcommand \"\": {MUST}")
    );
    assert_eq!(ok(b"dict filter {a 1} k *"), b"a 1");
    assert_eq!(
        err_of(b"dict filter {a 1} {} *"),
        "ambiguous filterType \"\": must be key, script, or value"
    );
}

#[test]
fn create_get_size() {
    assert_eq!(ok(b"dict create a 1 b 2"), b"a 1 b 2");
    assert_eq!(ok(b"dict get {a 1 b 2} b"), b"2");
    assert_eq!(ok(b"dict size {a 1 b 2 c 3}"), b"3");
    assert_eq!(ok(b"dict exists {a 1 b 2} b"), b"1");
    assert_eq!(ok(b"dict exists {a 1 b 2} z"), b"0");
}

/// The runtime adapter uses the same Tcl-hash layout and formatter as the
/// native VM. Exact Tcl 9.0.4 result for this freshly parsed dictionary.
#[test]
fn info_uses_the_shared_hash_statistics_owner() {
    assert_eq!(
        ok(b"dict info {a 1 b 2}"),
        b"2 entries in table, 4 buckets\n\
              number of buckets with 0 entries: 2\n\
              number of buckets with 1 entries: 2\n\
              number of buckets with 2 entries: 0\n\
              number of buckets with 3 entries: 0\n\
              number of buckets with 4 entries: 0\n\
              number of buckets with 5 entries: 0\n\
              number of buckets with 6 entries: 0\n\
              number of buckets with 7 entries: 0\n\
              number of buckets with 8 entries: 0\n\
              number of buckets with 9 entries: 0\n\
              number of buckets with 10 or more entries: 0\n\
              average search distance for entry: 1.0"
    );

    let (code, message) = run(b"rename dict d; d info");
    assert_eq!(code, Code::Error);
    assert_eq!(message, b"wrong # args: should be \"d info dictionary\"");

    use std::fmt::Write as _;

    let mut retained_script = "set d {};".to_owned();
    for index in 0..13 {
        write!(retained_script, "dict set d k{index} {index};")
            .expect("writing to a String cannot fail");
    }
    for index in 0..12 {
        write!(retained_script, "dict unset d k{index};").expect("writing to a String cannot fail");
    }
    retained_script.push_str("dict info $d");
    assert_eq!(
        ok(retained_script.as_bytes()),
        b"1 entries in table, 16 buckets\n\
              number of buckets with 0 entries: 15\n\
              number of buckets with 1 entries: 1\n\
              number of buckets with 2 entries: 0\n\
              number of buckets with 3 entries: 0\n\
              number of buckets with 4 entries: 0\n\
              number of buckets with 5 entries: 0\n\
              number of buckets with 6 entries: 0\n\
              number of buckets with 7 entries: 0\n\
              number of buckets with 8 entries: 0\n\
              number of buckets with 9 entries: 0\n\
              number of buckets with 10 or more entries: 0\n\
              average search distance for entry: 1.0"
    );

    let mut cow_script = "set d {};".to_owned();
    for index in 0..13 {
        write!(cow_script, "dict set d k{index} {index};")
            .expect("writing to a String cannot fail");
    }
    for index in 0..12 {
        write!(cow_script, "dict unset d k{index};").expect("writing to a String cannot fail");
    }
    cow_script.push_str(
        "set e $d;dict set d x 1;set f $e;dict unset e k12;\
             list \
                 [lindex [split [dict info $d] \\n] 0] \
                 [lindex [split [dict info $e] \\n] 0] \
                 [lindex [split [dict info $f] \\n] 0]",
    );
    assert_eq!(
        ok(cow_script.as_bytes()),
        b"{2 entries in table, 4 buckets} \
              {0 entries in table, 4 buckets} \
              {1 entries in table, 16 buckets}"
    );
}

#[test]
fn keys_values_insertion_order() {
    assert_eq!(ok(b"dict keys {z 1 a 2 m 3}"), b"z a m"); // not sorted
    assert_eq!(ok(b"dict values {z 1 a 2 m 3}"), b"1 2 3");
}

#[test]
fn set_and_unset_variable_cow() {
    assert_eq!(ok(b"dict set d a 1; dict set d b 2"), b"a 1 b 2");
    assert_eq!(ok(b"set d {a 1 b 2}; dict unset d a"), b"b 2");
    // overwrite keeps key position
    assert_eq!(
        ok(b"dict set d x 1; dict set d y 2; dict set d x 9"),
        b"x 9 y 2"
    );
}

/// A moderately nested `dict set`/`dict unset` key path (well under any
/// depth that would have stressed the original recursion) still produces
/// exactly the nested structure the recursive version did — the
/// iterative rewrite must not change ordinary behaviour.
#[test]
fn moderately_nested_dict_path_set_unset_unaffected() {
    assert_eq!(ok(b"dict set d a b c 1"), b"a {b {c 1}}");
    assert_eq!(
        ok(b"set d {a {b {c 1 d 2}}}; dict unset d a b c; set d"),
        b"a {b {d 2}}"
    );
    // A missing intermediate segment still errors the same way.
    let (c, b) = run(b"set d {a {}}; dict unset d a b c");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"key \"b\" not known in dictionary");
}

/// Deep dictionary paths use iterative mutation and deepest-first string
/// generation. This fixed depth exercises both operations and successful
/// completion without comparing the complete nested string.
#[test]
fn deeply_nested_dict_path_set_and_unset_survive() {
    const DEPTH: usize = 3800;
    let set_src = format!("dict set d {{*}}[lrepeat {DEPTH} k] v");
    ok(set_src.as_bytes());
    let unset_src =
        format!("dict set d {{*}}[lrepeat {DEPTH} k] v; dict unset d {{*}}[lrepeat {DEPTH} k]");
    ok(unset_src.as_bytes());
}

#[test]
fn merge_later_wins_first_position_kept() {
    assert_eq!(ok(b"dict merge {a 1 b 2} {b 9 c 3}"), b"a 1 b 9 c 3");
}

#[test]
fn replace_and_remove() {
    // `replace` upserts (existing key keeps position; new key appends).
    assert_eq!(ok(b"dict replace {a 1 b 2} b 3 c 4"), b"a 1 b 3 c 4");
    assert_eq!(ok(b"dict replace {a 1 b 2}"), b"a 1 b 2");
    // `remove` drops the named keys; a missing key is not an error.
    assert_eq!(ok(b"dict remove {a 1 b 2 c 3} b d"), b"a 1 c 3");
    assert_eq!(ok(b"dict remove {a 1 b 2}"), b"a 1 b 2");
    // An odd key/value count to `replace` is a wrong-# args error.
    let (c, _) = run(b"dict replace {a 1} b");
    assert_eq!(c, Code::Error);
}

#[test]
fn getdef_returns_default_on_miss() {
    assert_eq!(ok(b"dict getdef {a 1 b 2} a X"), b"1");
    assert_eq!(ok(b"dict getdef {a 1 b 2} z X"), b"X");
    // Nested key path.
    assert_eq!(ok(b"dict getdef {a {b 1}} a b X"), b"1");
    assert_eq!(ok(b"dict getdef {a {b 1}} a z X"), b"X");
    // The alias behaves identically and echoes its own name on misuse.
    assert_eq!(ok(b"dict getwithdefault {a 1} z D"), b"D");
    let (c, b) = run(b"dict getwithdefault {a 1} z");
    assert_eq!(c, Code::Error);
    assert!(b.starts_with(b"wrong # args"));
    assert!(b.windows(14).any(|w| w == b"getwithdefault"));
}

#[test]
fn dict_for_iterates_in_order() {
    assert_eq!(
        ok(b"set out {}; dict for {k v} {a 1 b 2 c 3} { lappend out $k=$v }; set out"),
        b"a=1 b=2 c=3"
    );
}

#[test]
fn get_missing_key_errors() {
    let (c, b) = run(b"dict get {a 1} z");
    assert_eq!(c, Code::Error);
    assert_eq!(b, b"key \"z\" not known in dictionary");
}

// Needs the numeric tower: the filter scripts test via `expr`.
#[cfg(have_tommath)]
#[test]
fn filter_key_value_via_shared_core() {
    // `key`/`value` globs now come from `tcl_cmd_core::dict`.
    assert_eq!(ok(b"dict filter {a 1 b 2 aa 3} key a*"), b"a 1 aa 3");
    assert_eq!(ok(b"dict filter {a 1 b 2 aa 3} value 2"), b"b 2");
    assert_eq!(ok(b"dict filter {a 1 b 2} key"), b""); // no patterns → empty
                                                       // The filterType is validated *before* the dict is parsed (was a bug:
                                                       // a bad dict + bogus type reported the dict error first).
    let (c, b) = run(b"dict filter {a b c} bogus");
    assert_eq!(c, Code::Error);
    assert_eq!(
        b,
        b"bad filterType \"bogus\": must be key, script, or value"
    );
    // `script` stays in the runtime adapter (Family-B).
    assert_eq!(
        ok(b"dict filter {a 1 b 2 c 3} script {k v} {expr {$v > 1}}"),
        b"b 2 c 3"
    );
}
/// The **read-only** dict path reports value-parse failures
/// with the dict noun, the junk fragment, and the dict `errorCode`.
///
/// `size`/`info`/`get`/`keys`/`values`/`merge`/`filter`/`replace`/`remove`/
/// `getdef` reach C's parser only through `dispatch_canon`, which decodes
/// with the shared **list** codec. Two things have to be corrected on the
/// way out: the shared core's message arrives list-worded and needs
/// translating (otherwise `dict size` would say `list element in braces
/// …` with `errorCode` `NONE`), and `ValueOps::list_elements` must report
/// the full `TclFindElement` sentence rather than `ListError::message`'s
/// fragment-less prefix.
///
/// The mutating path was already correct via `bad_dict`; it is included
/// here as a regression net. Byte-checked against `tclsh9.0.4`.
#[test]
fn read_only_dict_path_uses_the_dict_noun_fragment_and_error_code() {
    // `{b}c` — a brace-delimited element followed by junk.
    const BAD: &[u8] = b"set d [binary format H* 612031207b627d632064]; ";
    const JUNK: &[u8] = b"dict element in braces followed by \"c\" instead of space";
    for sub in [
        &b"dict size $d"[..],
        &b"dict info $d"[..],
        &b"dict get $d a"[..],
        &b"dict keys $d"[..],
        &b"dict values $d"[..],
        &b"dict merge $d"[..],
        &b"dict filter $d key *"[..],
        &b"dict replace $d q 1"[..],
        &b"dict remove $d q"[..],
        &b"dict getdef $d q Z"[..],
    ] {
        let (c, b) = run(&[BAD, sub].concat());
        assert_eq!(c, Code::Error, "{}", String::from_utf8_lossy(sub));
        assert_eq!(b, JUNK, "{}", String::from_utf8_lossy(sub));
        let (_, ec) = run(&[BAD, b"catch {", sub, b"}; set ::errorCode"].concat());
        assert_eq!(
            ec,
            b"TCL VALUE DICTIONARY JUNK",
            "{} errorCode",
            String::from_utf8_lossy(sub)
        );
    }
    // The other three parse failures, on the read path.
    for (hexset, msg, code) in [
        (
            &b"set d [binary format H* 61203120226222632064]; "[..],
            &b"dict element in quotes followed by \"c\" instead of space"[..],
            &b"TCL VALUE DICTIONARY JUNK"[..],
        ),
        (
            &b"set d [binary format H* 612031207b62]; "[..],
            &b"unmatched open brace in dict"[..],
            &b"TCL VALUE DICTIONARY BRACE"[..],
        ),
        (
            &b"set d [binary format H* 6120312062]; "[..],
            &b"missing value to go with key"[..],
            &b"TCL VALUE DICTIONARY"[..],
        ),
    ] {
        let (c, b) = run(&[hexset, b"dict size $d"].concat());
        assert_eq!(c, Code::Error);
        assert_eq!(b, msg);
        let (_, ec) = run(&[hexset, b"catch {dict size $d}; set ::errorCode"].concat());
        assert_eq!(ec, code);
    }
    // The *list* noun is untouched — the two must stay distinguishable.
    let (c, b) = run(&[BAD, b"llength $d"].concat());
    assert_eq!(c, Code::Error);
    assert_eq!(
        b,
        b"list element in braces followed by \"c\" instead of space"
    );
}

/// `dict lappend` onto a non-dict does not silently succeed, and this
/// pins why.
///
/// The three shapes a value can relate to `dict`, each matching C Tcl
/// 9.0.4 *and* 8.6.16 byte-for-byte in message text and `errorCode`:
///
/// | value     | outcome |
/// |-----------|---------|
/// | `"a b"`   | accepted — it *is* a valid one-entry dict |
/// | `"a b c"` | `missing value to go with key` / `TCL VALUE DICTIONARY` |
/// | `"{"`     | `unmatched open brace in dict` / `TCL VALUE DICTIONARY BRACE` |
///
/// The even-length accept is correct and is the shape most easily
/// mistaken for the reported bug.  What the fuzzer actually found (seed
/// 90022) was `set b 5; namespace eval n2 { dict incr b qux 1 }`: under
/// the 8.6 oracle the relative `b` fell back to the global `5`, which is
/// not a dict, so C raised — under 9.0 it binds a fresh `::n2::b` and
/// succeeds.  That is finding (1)'s resolution rule seen through a `dict`
/// command, not a dict-validation defect.
#[test]
fn dict_lappend_validation_matches_c_tcl_for_every_non_dict_shape() {
    // A valid (even-length) dict is accepted — not a bug.
    assert_eq!(ok(b"set d {a b}; dict lappend d k v"), b"a b k v");
    // Odd length and unbalanced braces both raise, with C's exact text.
    for (script, msg, code) in [
        (
            &b"set d {a b c}; dict lappend d k v"[..],
            &b"missing value to go with key"[..],
            &b"TCL VALUE DICTIONARY"[..],
        ),
        (
            b"set d \\{; dict lappend d k v",
            b"unmatched open brace in dict",
            b"TCL VALUE DICTIONARY BRACE",
        ),
        (
            b"set d {{a}x b}; dict lappend d k v",
            b"dict element in braces followed by \"x\" instead of space",
            b"TCL VALUE DICTIONARY JUNK",
        ),
    ] {
        let (c, b) = run(script);
        assert_eq!(c, Code::Error, "{}", String::from_utf8_lossy(script));
        assert_eq!(b, msg, "message text");
        // `errorCode` is part of the contract, not just the raise bit.
        // Wrapped in `catch` so evaluation reaches the read.
        let (_, ec) = run(&[b"catch {", script, b"}; set ::errorCode"].concat());
        assert_eq!(ec, code, "errorCode");
    }
}

/// The dict-validation answer must come from the *value*, never from
/// whichever internal representation it happens to be carrying — a
/// shimmered list and an identical pure string must agree.  All six
/// pinned against C Tcl 9.0.4 and 8.6.16.
#[test]
fn dict_lappend_validation_is_unaffected_by_shimmering() {
    // Built as a 3-element list, so it arrives with a list intrep.
    let (c, b) = run(b"set v {}; lappend v a; lappend v b; lappend v c; dict lappend v k x");
    assert_eq!(c, Code::Error, "a shimmered odd-length list still raises");
    assert_eq!(b, b"missing value to go with key");
    // The same odd-length value as a pure string agrees.
    let (c2, b2) = run(b"set v {a b c}; string length $v; dict lappend v k x");
    assert_eq!(c2, Code::Error);
    assert_eq!(b2, b, "string rep and list rep must give the same answer");
    // An even-length shimmered list is accepted, as it is a valid dict.
    assert_eq!(
        ok(b"set v {}; lappend v a; lappend v b; dict lappend v k x"),
        b"a b k x"
    );
    // dict -> string -> list -> dict round trip: appending " c" makes it
    // odd, and `llength` forces a list intrep before `dict lappend` sees it.
    let (c3, b3) =
        run(b"set v [dict create a 1 b 2]; append v { c}; llength $v; dict lappend v k x");
    assert_eq!(c3, Code::Error);
    assert_eq!(b3, b"missing value to go with key");
    // A value that already has a dict intrep stays valid after `llength`
    // forces a list rep over it.
    assert_eq!(
        ok(b"set v [dict create a 1]; llength $v; dict lappend v k x"),
        b"a 1 k x"
    );
}
