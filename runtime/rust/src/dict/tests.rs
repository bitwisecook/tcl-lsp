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

#[test]
fn actual_c84_updater_retains_original_long_and_wide_minimum_cache() {
    use tcl_platform::{NativeIntegerFormatter, NativeIntegerKind};
    use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let formatter =
        tcl_test_support::native_integer_formatter::load_pinned_c84_integer_formatter(&root)
            .expect("explicit pinned native formatter");
    let dialect = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
    );
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
    for (kind, cache) in [
        (
            NativeIntegerKind::Long,
            NativeScalarCache::Tcl84Long(i64::MIN),
        ),
        (
            NativeIntegerKind::Wide,
            NativeScalarCache::Number(Number::Int(i64::MIN)),
        ),
    ] {
        let original = obj::Owned::fresh(obj::new_wide_int_obj(i64::MIN));
        obj::adopt_native_scalar_cache(
            original.as_ptr(),
            cache.clone(),
            dialect.native_scalar_getter_protocol().unwrap(),
        )
        .unwrap();
        assert!(native_object_bytes(original.as_ptr(), protocol).is_err());
        assert!(!obj::has_string_rep(original.as_ptr()));
        let expected = formatter.format(kind, i64::MIN).unwrap();
        let bytes = native_object_bytes_with_integer_formatter(
            original.as_ptr(),
            protocol,
            Some(&formatter),
        )
        .unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(
            obj::native_scalar_cache(original.as_ptr()).unwrap(),
            Some(cache)
        );
        assert!(obj::has_string_rep(original.as_ptr()));
    }
}

fn test_dict(pairs: &[(*mut TclObj, *mut TclObj)]) -> *mut TclObj {
    let value = super::new_dict_obj(pairs);
    super::seal_string_protocol(
        value,
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
    )
    .unwrap();
    value
}
use super::*;
use crate::counters;
use crate::obj::new_string_bytes;

fn leak_free(body: impl FnOnce()) {
    counters::reset();
    body();
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

fn s(b: &[u8]) -> *mut TclObj {
    new_string_bytes(b)
}

fn bytes(obj: *mut TclObj) -> Vec<u8> {
    obj::bytes_of(obj)
}

#[test]
fn build_get_size() {
    leak_free(|| {
        let d = test_dict(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2"))]);
        unsafe { obj::incr_ref_count(d) };
        assert_eq!(dict_size(d).unwrap(), 2);
        assert_eq!(bytes(dict_get(d, b"a").unwrap().unwrap()), b"1");
        assert!(dict_get(d, b"missing").unwrap().is_none());
        assert!(dict_exists(d, b"b").unwrap());
        unsafe { obj::decr_ref_count(d) }; // frees dict + its keys + values
    });
}

#[test]
fn set_overwrites_in_place_preserving_order() {
    leak_free(|| {
        let d = test_dict(&[(s(b"x"), s(b"1"))]);
        unsafe { obj::incr_ref_count(d) };
        dict_set(d, s(b"y"), s(b"2")).unwrap(); // new key: both retained
                                                // Overwrite keeps the existing key object and does NOT retain the
                                                // passed key (Tcl_DictObjPut's contract), so the caller owns the
                                                // throwaway overwrite key — manage it here.
        let kx = s(b"x");
        unsafe { obj::incr_ref_count(kx) };
        dict_set(d, kx, s(b"9")).unwrap(); // overwrite x, keep its position
        unsafe { obj::decr_ref_count(kx) }; // overwrite didn't keep our key
        let keys: Vec<Vec<u8>> = dict_keys(d).unwrap().into_iter().map(bytes).collect();
        assert_eq!(keys, vec![b"x".to_vec(), b"y".to_vec()]);
        assert_eq!(bytes(dict_get(d, b"x").unwrap().unwrap()), b"9");
        unsafe { obj::decr_ref_count(d) };
    });
}

#[test]
fn insertion_order_in_string_rep() {
    leak_free(|| {
        let d = test_dict(&[(s(b"z"), s(b"1")), (s(b"a"), s(b"2")), (s(b"m"), s(b"3"))]);
        unsafe { obj::incr_ref_count(d) };
        // NOT sorted — insertion order (z a m), not alphabetical
        assert_eq!(bytes(d), b"z 1 a 2 m 3");
        unsafe { obj::decr_ref_count(d) };
    });
}

#[test]
fn unset_preserves_order_and_frees() {
    leak_free(|| {
        let d = test_dict(&[(s(b"a"), s(b"1")), (s(b"b"), s(b"2")), (s(b"c"), s(b"3"))]);
        unsafe { obj::incr_ref_count(d) };
        assert!(dict_unset(d, b"b").unwrap());
        let keys: Vec<Vec<u8>> = dict_keys(d).unwrap().into_iter().map(bytes).collect();
        assert_eq!(keys, vec![b"a".to_vec(), b"c".to_vec()]);
        assert_eq!(bytes(dict_get(d, b"c").unwrap().unwrap()), b"3"); // index fixed up
        assert!(!dict_unset(d, b"b").unwrap()); // already gone
        unsafe { obj::decr_ref_count(d) };
    });
}

#[test]
fn string_to_dict_shimmer() {
    leak_free(|| {
        let v = new_string_bytes(b"name tcl {ver 9} 0");
        unsafe { obj::incr_ref_count(v) };
        assert_eq!(dict_size(v).unwrap(), 2);
        assert_eq!(bytes(dict_get(v, b"name").unwrap().unwrap()), b"tcl");
        assert_eq!(bytes(dict_get(v, b"ver 9").unwrap().unwrap()), b"0");
        unsafe { obj::decr_ref_count(v) };
    });
}

/// This runtime's native dict rep is the one binding of the
/// canonicalisation rule that is *not* a call to the shared owner
/// [`tcl_syntax::value::canonical_dict_slots`]: it keeps a live key index
/// across mutation, so it canonicalises incrementally (one
/// `Tcl_DictObjPut` per insert) rather than in one walk over the elements.
/// That makes it exactly the shape the owner cannot enforce by
/// construction, so it is pinned by agreement instead — the same corpus
/// the cross-crate gate `rust/tcl-vm/tests/it_5/dict_canonicalisation_parity.rs`
/// drives through the other bindings.
#[test]
fn duplicate_keys_canonicalise_like_the_shared_owner() {
    leak_free(|| {
        for source in [
            "a 1 a 2",
            "a 1 b 2 a 3",
            "x 1 x 2 y 3",
            "a 1 a 2 a 3",
            "{k k} 1 {k k} 2",
            "a b b a a c",
            "1 one 01 oh-one 1 uno",
        ] {
            // The owner's answer, from the same decoded elements.
            let elements = tcl_syntax::list::split_list_lenient(source);
            let keys: Vec<&str> = elements.iter().step_by(2).map(AsRef::as_ref).collect();
            let want: Vec<(&str, &str)> =
                tcl_syntax::value::canonical_dict_slots(keys.iter().copied())
                    .into_iter()
                    .map(|(key_slot, value_slot)| {
                        (
                            elements[key_slot * 2].as_ref(),
                            elements[value_slot * 2 + 1].as_ref(),
                        )
                    })
                    .collect();

            let v = new_string_bytes(source.as_bytes());
            unsafe { obj::incr_ref_count(v) };
            let got: Vec<(Vec<u8>, Vec<u8>)> = dict_pairs(v)
                .unwrap()
                .into_iter()
                .map(|(k, val)| (bytes(k), bytes(val)))
                .collect();
            let want: Vec<(Vec<u8>, Vec<u8>)> = want
                .into_iter()
                .map(|(k, val)| (k.as_bytes().to_vec(), val.as_bytes().to_vec()))
                .collect();
            assert_eq!(
                got, want,
                "native dict rep diverges from the owner on {source:?}"
            );
            unsafe { obj::decr_ref_count(v) };
        }
    });
}

#[test]
fn odd_length_string_is_an_error() {
    leak_free(|| {
        let v = new_string_bytes(b"a 1 b"); // missing value for b
        unsafe { obj::incr_ref_count(v) };
        assert_eq!(dict_size(v), Err(DictError::MissingValue));
        unsafe { obj::decr_ref_count(v) };
    });
}

/// `\<newline>` is the line-continuation escape:
/// `TclParseBackslash` (tclParse.c(9.0.4):884-890) collapses the backslash,
/// the newline **and the run of spaces/tabs after it** into a single space,
/// so that whitespace is *data* inside the element and must not terminate
/// it. The dict scan is the shared `tcl_syntax::list` codec rather than a
/// separate `FindElement` port: a port whose backslash arm only skips two
/// bytes would split `a\<LF> b c` into three elements where the list
/// codec (and tclsh) see two — and the mutating dict subcommands, which
/// reach the dict through this scan rather than through the canonical
/// codec, would then disagree with `llength` and with `dict size`.
#[test]
fn backslash_newline_absorbs_the_following_space_run() {
    leak_free(|| {
        // Two elements per tclsh 9.0.4: `a b` and `c` ⇒ one dict pair.
        let v = new_string_bytes(b"a\\\n b c");
        unsafe { obj::incr_ref_count(v) };
        assert_eq!(dict_size(v).unwrap(), 1);
        assert_eq!(bytes(dict_get(v, b"a b").unwrap().unwrap()), b"c");
        // The string rep re-renders the collapsed key with list quoting.
        let mut pairs = dict_pairs(v).unwrap();
        assert_eq!(pairs.len(), 1);
        let (k, val) = pairs.pop().unwrap();
        assert_eq!(bytes(k), b"a b");
        assert_eq!(bytes(val), b"c");
        unsafe { obj::decr_ref_count(v) };

        // The odd-length mirror: three elements ⇒ `missing value to go
        // with key`, exactly where tclsh errors.
        let w = new_string_bytes(b"a\\\n b c d");
        unsafe { obj::incr_ref_count(w) };
        assert_eq!(dict_size(w), Err(DictError::MissingValue));
        unsafe { obj::decr_ref_count(w) };
    });
}

/// The dict-worded delimiter errors still carry the offending fragment
/// after the scan moved to the shared list codec.
#[test]
fn delimiter_errors_keep_their_dict_wording_and_fragment() {
    leak_free(|| {
        let v = new_string_bytes(b"a 1 {b}c d");
        unsafe { obj::incr_ref_count(v) };
        assert_eq!(
            dict_size(v),
            Err(DictError::Parse {
                error: tcl_syntax::list::ListError::BraceFollowedByJunk,
                source: b"a 1 {b}c d".to_vec()
            })
        );
        unsafe { obj::decr_ref_count(v) };

        let q = new_string_bytes(b"a 1 \"b\"c d");
        unsafe { obj::incr_ref_count(q) };
        assert_eq!(
            dict_size(q),
            Err(DictError::Parse {
                error: tcl_syntax::list::ListError::QuoteFollowedByJunk,
                source: b"a 1 \"b\"c d".to_vec()
            })
        );
        unsafe { obj::decr_ref_count(q) };

        let ub = new_string_bytes(b"a 1 {b");
        unsafe { obj::incr_ref_count(ub) };
        assert_eq!(
            dict_size(ub),
            Err(DictError::Parse {
                error: tcl_syntax::list::ListError::UnmatchedBrace,
                source: b"a 1 {b".to_vec()
            })
        );
        unsafe { obj::decr_ref_count(ub) };

        let uq = new_string_bytes(b"a 1 \"b");
        unsafe { obj::incr_ref_count(uq) };
        assert_eq!(
            dict_size(uq),
            Err(DictError::Parse {
                error: tcl_syntax::list::ListError::UnmatchedQuote,
                source: b"a 1 \"b".to_vec()
            })
        );
        unsafe { obj::decr_ref_count(uq) };
    });
}
