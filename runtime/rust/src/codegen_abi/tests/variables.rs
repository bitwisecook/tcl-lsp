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

//! Compiled slots and named access share one cell: read-modify-write, unset, the trace barrier and array elements.

use super::*;

#[test]
#[cfg(have_tommath)]
fn compiled_slots_and_named_access_share_one_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_local_bind(0, b"b".as_ptr(), 1, owned_str(b"2")),
            0
        );
        assert_eq!(
            tcl_codegen_local_bind(1, b"c".as_ptr(), 1, owned_str(b"4")),
            0
        );
        assert_eq!(tcl_eval_code(box_str(b"set b 5")), 0);

        let sum = tcl_codegen_expr_add(tcl_codegen_local_get(0), tcl_codegen_local_get(1));
        assert_eq!(obj_bytes(sum), b"9");
        tcl_value_release(sum);

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The read-modify-write slot ABI is the runtime's own `incr` / `append` /
/// `lappend` addressed by index: full Tcl semantics, including the bignum
/// promotion `incr` gets from the numeric tower, and a by-name view of the
/// very same cell.
#[cfg(have_tommath)]
#[test]
fn slot_read_modify_write_runs_the_real_commands_on_the_same_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"n".as_ptr(), 1, owned_str(b"1")),
            0
        );
        let mut value: i64 = 0;
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 41, &mut value),
            TCL_VALUE_GET_OK
        );
        assert_eq!(value, 42);
        // The named view sees the indexed write, and vice versa.
        assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
        assert_eq!((*interp).result_bytes(), b"42");
        assert_eq!(tcl_eval_code(box_str(b"incr n")), 0);
        let indexed = tcl_codegen_slot_get(0);
        assert_eq!(obj_bytes(indexed), b"43");
        tcl_value_release(indexed);

        // `incr` promotes past the wide range instead of wrapping, and the
        // i64 read of that new value reports C's overflow error.
        assert_eq!(tcl_eval_code(box_str(b"set n 9223372036854775807")), 0);
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 1, &mut value),
            TCL_VALUE_GET_ERROR
        );
        assert_eq!(
            (*interp).result_bytes(),
            b"integer value too large to represent"
        );
        assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
        assert_eq!((*interp).result_bytes(), b"9223372036854775808");

        // `incr` on a non-integer cell keeps C's message.
        assert_eq!(tcl_eval_code(box_str(b"set n abc")), 0);
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 1, &mut value),
            TCL_VALUE_GET_ERROR
        );
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"abc\""
        );

        // `append` / `lappend` through the slot, seen by name.
        assert_eq!(
            tcl_codegen_slot_bind(1, b"s".as_ptr(), 1, owned_str(b"a")),
            0
        );
        let piece = owned_str(b"bc");
        assert_eq!(tcl_codegen_slot_append(1, piece), 0);
        tcl_value_release(piece);
        assert_eq!(tcl_eval_code(box_str(b"set s")), 0);
        assert_eq!((*interp).result_bytes(), b"abc");

        assert_eq!(
            tcl_codegen_slot_bind(2, b"l".as_ptr(), 1, owned_str(b"x")),
            0
        );
        let item = owned_str(b"y z");
        assert_eq!(tcl_codegen_slot_lappend(2, item), 0);
        tcl_value_release(item);
        assert_eq!(tcl_eval_code(box_str(b"llength $l")), 0);
        assert_eq!((*interp).result_bytes(), b"2");
        assert_eq!(tcl_eval_code(box_str(b"lindex $l 1")), 0);
        assert_eq!((*interp).result_bytes(), b"y z");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// An indexed slot keeps addressing one cell across `unset` and re-creation,
/// and a variable trace, an `upvar` link, and `info locals` all reach that
/// same cell — the indexed path must never become a second variable.
#[cfg(have_tommath)]
#[test]
fn an_indexed_slot_survives_unset_and_stays_the_named_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"first")),
            0
        );
        assert_eq!(tcl_eval_code(box_str(b"unset v")), 0);
        assert!(
            tcl_codegen_slot_get(0).is_null(),
            "an unset cell reads as a missing variable"
        );
        assert_eq!(tcl_eval_code(box_str(b"set v second")), 0);
        let value = tcl_codegen_slot_get(0);
        assert_eq!(
            obj_bytes(value),
            b"second",
            "the re-created variable refills the same cell"
        );
        tcl_value_release(value);

        // A read trace observes the indexed read: the fast path declines
        // whenever anything can see the difference.
        assert_eq!(
            tcl_eval_code(box_str(
                b"set ::hits 0; trace add variable v read {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        let traced = tcl_codegen_slot_get(0);
        assert!(!traced.is_null());
        tcl_value_release(traced);
        assert_eq!(tcl_eval_code(box_str(b"set ::hits")), 0);
        assert_eq!((*interp).result_bytes(), b"1");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Ask the trace barrier about a name. Only the `have_tommath` tests
/// below reach it.
#[cfg(have_tommath)]
unsafe fn var_traced(name: &[u8]) -> i32 {
    // SAFETY: `name` is a valid readable slice.
    unsafe { tcl_codegen_var_traced(name.as_ptr(), name.len() as i32) }
}

/// The per-cell trace bit answers for a name and for a slot, follows an
/// `upvar` link to its target's traces, covers an array's elements, and —
/// the case a hand-maintained bit would get wrong — goes back to `0` when a
/// proc frame's teardown drops its locals' traces.
#[cfg(have_tommath)]
#[test]
fn the_trace_barrier_tracks_adds_removes_links_and_frame_teardown() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        assert_eq!(tcl_eval_code(box_str(b"set g 1; set ::hits 0")), 0);
        assert_eq!(var_traced(b"g"), 0, "an untraced global");

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable g write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"g"), 1);
        assert_eq!(var_traced(b"other"), 0, "the bit is per cell, not global");

        // An array reports traced when any element is: the guard is
        // conservative in the safe direction.
        assert_eq!(tcl_eval_code(box_str(b"set arr(k) 1")), 0);
        assert_eq!(var_traced(b"arr"), 0);
        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable arr(k) write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"arr"), 1);
        assert_eq!(var_traced(b"arr(k)"), 1);

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace remove variable g write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"g"), 0, "removing the last trace clears it");

        // A trace added inside a proc frame is visible through the local
        // name and through an `upvar` alias to it, and both answers go back
        // to untraced when the frame is popped.
        assert_eq!(
            tcl_eval_code(box_str(
                b"proc probe {} {\n\
                        set loc 1\n\
                        upvar 0 loc alias\n\
                        set ::before [tcltrace loc],[tcltrace alias]\n\
                        trace add variable loc write {apply {{a b op} {incr ::hits}}}\n\
                        set ::during [tcltrace loc],[tcltrace alias]\n\
                      }"
            )),
            0
        );
        // A tiny Tcl-visible probe over the same ABI entry point.
        assert_eq!(
            tcl_eval_code(box_str(
                b"proc tcltrace {n} { uplevel 1 [list ::tcl::probe_traced $n] }"
            )),
            0
        );
        (*interp).register_builtin(b"::tcl::probe_traced", |interp, argv| {
            let name = obj_bytes(argv[1]);
            let traced = interp.var_is_traced(&name);
            interp.set_result_bytes(if traced { b"1" } else { b"0" });
            crate::interp::Code::Ok
        });

        assert_eq!(tcl_eval_code(box_str(b"probe")), 0);
        assert_eq!(tcl_eval_code(box_str(b"set ::before")), 0);
        assert_eq!((*interp).result_bytes(), b"0,0");
        assert_eq!(tcl_eval_code(box_str(b"set ::during")), 0);
        assert_eq!(
            (*interp).result_bytes(),
            b"1,1",
            "an upvar alias reports its target's traces"
        );
        // The frame is gone, so its local's trace is too.
        assert_eq!(var_traced(b"loc"), 0);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The slot spelling answers from the cell's own bit, and that bit is
/// re-derived — never trusted — after the trace set changes.
#[cfg(have_tommath)]
#[test]
fn the_slot_trace_bit_is_revalidated_when_the_trace_set_changes() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"1")),
            0
        );
        assert_eq!(tcl_codegen_slot_traced(0), 0);
        // Asked twice: the second answer comes from the cached bit.
        assert_eq!(tcl_codegen_slot_traced(0), 0);

        assert_eq!(tcl_eval_code(box_str(b"set ::hits 0")), 0);
        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable v write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(
            tcl_codegen_slot_traced(0),
            1,
            "the epoch moved, so the cached 0 was recomputed"
        );
        assert_eq!(tcl_codegen_slot_traced(0), 1);

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace remove variable v write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(tcl_codegen_slot_traced(0), 0);
        assert_eq!(tcl_codegen_slot_traced(9), 0, "an unbound slot is untraced");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// `$a(b)` reads the array element, fires its read traces, and hands
/// generated code exactly one releasable reference.
#[test]
fn var_get_element_reads_an_array_element() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(tcl_eval_code(box_str(b"set a(b) element")), 0);

        let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"b".as_ptr(), 1);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"element");
        tcl_obj_release(value);

        // A missing element reports the C-faithful read error, not a scalar
        // miss on a name that happens to contain parentheses.
        assert!(tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"z".as_ptr(), 1).is_null());
        assert_eq!(
            (*interp).result_bytes(),
            b"can't read \"a(z)\": no such element in array"
        );

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn var_get_named_preserves_a_literal_element_key() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            tcl_eval_code(box_str(
                b"set a(k) dynamic; set {a($key)} literal; set key k"
            )),
            0
        );
        let value = tcl_codegen_var_get(b"a($key)".as_ptr(), 7);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"literal");
        tcl_obj_release(value);
        let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"k".as_ptr(), 1);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"dynamic");
        tcl_obj_release(value);

        let value = owned_word(b"updated");
        assert_eq!(tcl_codegen_var_set(b"a($key)".as_ptr(), 7, value), 0);
        assert_eq!(tcl_eval_code(box_str(b"list ${a($key)} $a($key)")), 0);
        assert_eq!((*interp).result_bytes(), b"updated dynamic");

        assert!(tcl_codegen_var_get(b"a(missing)".as_ptr(), 10).is_null());
        assert_eq!(
            (*interp).result_bytes(),
            b"can't read \"a(missing)\": no such element in array"
        );
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn named_element_store_runs_its_observer_before_reporting_the_error() {
    for engine in tcl_test_support::available_tclshs() {
        let trace = if engine.version == TclVersion::V8_4 {
            "trace variable {arr($i)} rw observer"
        } else {
            "trace add variable {arr($i)} {read write} observer"
        };
        let setup = format!(
            "set {{arr($i)}} OLD; set i k; set arr(k) DYNAMIC; set events {{}}; \
                 proc observer {{name key op}} {{lappend ::events [list $name $key $op]; \
                 if {{$op eq \"w\" || $op eq \"write\"}} {{error REJECT}}}}; {trace}"
        );
        let expected = tcl_test_support::run_script(
            &engine.path,
            format!(
                "{setup}; set before ${{arr($i)}}; \
                     set code [catch {{set {{arr($i)}} NEW}} result]; \
                     puts [list $before $code $result ${{arr($i)}} $arr($i) $events]\n"
            )
            .as_bytes(),
        )
        .expect("native named-element observer execution");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{expected:?}"
        );
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            (*interp).set_runtime_version(engine.version);
            tcl_runtime_set_current_interp(interp);
            assert_eq!(tcl_eval_code(box_str(setup.as_bytes())), 0);
            let before = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
            assert!(!before.is_null());
            let before_bytes = obj_bytes(before);
            tcl_obj_release(before);
            let code = tcl_codegen_var_set(b"arr($i)".as_ptr(), 7, owned_word(b"NEW"));
            let message = (*interp).result_bytes();
            let after = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
            assert!(!after.is_null());
            let after_bytes = obj_bytes(after);
            tcl_obj_release(after);
            let sibling = tcl_codegen_var_get_element(b"arr".as_ptr(), 3, b"k".as_ptr(), 1);
            assert!(!sibling.is_null());
            let sibling_bytes = obj_bytes(sibling);
            tcl_obj_release(sibling);
            assert_eq!(tcl_eval_code(box_str(b"set events")), 0);
            let values = [
                before_bytes,
                code.to_string().into_bytes(),
                message,
                after_bytes,
                sibling_bytes,
                (*interp).result_bytes(),
            ];
            let mut observed = Vec::new();
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    observed.push(b' ');
                }
                crate::list::append_list_element(&mut observed, value, index == 0);
            }
            assert_eq!(
                observed,
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{:?}",
                engine.path
            );
            assert!(!(*interp).host_refusal_pending());
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }
}
