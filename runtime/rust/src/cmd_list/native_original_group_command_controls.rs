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

//! Physical original grouped keys and returned comparator objects, bounded to651.
use crate::{interp::{Code, Interp}, list, obj};
use tcl_cmd_core::index::NativeIndexOps;
use std::cell::Cell;
include!("../../tests/data/native_original_list_group_command651/software_controls.rs");
thread_local! {
    static RETURNED: Cell<*mut obj::TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static CALLBACK_ERROR: Cell<bool> = const { Cell::new(false) };
}
fn make(input: (&str, &str)) -> Option<*mut obj::TclObj> {
    match input.0 {
        "absent" => None,
        "counted-string" => Some(obj::new_string_bytes(&unhex(input.1))),
        "typed-wide" => Some(obj::new_wide_int_obj(input.1.parse().unwrap())),
        "typed-double" => Some(obj::new_double_obj(if input.1 == "NaN" { f64::NAN } else { input.1.parse().unwrap() })),
        _ => unreachable!(),
    }
}
fn kind(original: *mut obj::TclObj) -> &'static str {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() { "none" } else {
        // SAFETY: the actual live header retains this static descriptor.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }.to_str().unwrap()
    }
}
fn callback(interp: &mut Interp, arguments: &[*mut obj::TclObj]) -> Code {
    assert_eq!(arguments.len(), 3);
    // Native651 deliberately gets both actual callback arguments before return.
    for argument in &arguments[1..] { interp.index_operand_bytes(argument).unwrap(); }
    interp.set_result(RETURNED.get());
    if CALLBACK_ERROR.get() { Code::Error } else { Code::Ok }
}
#[test]
fn grouped_sort_and_original_comparator_result_public_predicates_match_native651() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Actual containing List-only key holds and ONE deliberate returned-object
    // observer are retained. Compare only Guest/bytes and public primary
    // category and successful-key residency, plus returned-object residency.
    // Failed-call argv logging is outside this direct command control; no
    // refcount, alias or foreign callback order claim.
    // Jim case12 has no provider output and supplies no expected software result.
    assert_eq!(ROWS.len(), 179);
    for row in ROWS {
        let mut interp = Interp::with_native_core(crate::interp::default_host(),
            crate::environment::profile_for_dialect(row.dialect),
            tcl_registry::special_vars::NativeBootstrapInputs::default()).unwrap();
        let protocol = interp.native_invocation_dialect().native_string_protocol().unwrap();
        let originals = row.input.map(make);
        let members = originals[..row.count].iter().flatten().copied().collect::<Vec<_>>();
        let container = obj::Owned::fresh(list::new_list_obj_native(&members, protocol));
        let returned = make(row.returned).map(obj::Owned::fresh);
        RETURNED.set(returned.as_ref().map_or(std::ptr::null_mut(), obj::Owned::as_ptr));
        CALLBACK_ERROR.set(row.error);
        interp.register_builtin(b"r2286cmp", callback);
        let command_member = obj::new_string_bytes(b"r2286cmp");
        let prefix = obj::Owned::fresh(list::new_list_obj_native(&[command_member], protocol));
        let name = obj::Owned::fresh(obj::new_string_bytes(b"lsort"));
        let mode = obj::Owned::fresh(obj::new_string_bytes(if row.command { b"-command" } else if row.real == "real" { b"-real" } else if row.real == "ascii" { b"-ascii" } else { b"-integer" }));
        let stride = obj::Owned::fresh(obj::new_string_bytes(b"-stride"));
        let two = obj::Owned::fresh(obj::new_string_bytes(b"2"));
        let index = obj::Owned::fresh(obj::new_string_bytes(b"-index"));
        let path = row.index.map(|path| obj::Owned::fresh(obj::new_string_bytes(path.as_bytes())));
        let unique = obj::Owned::fresh(obj::new_string_bytes(b"-unique"));
        let mut arguments = vec![name.as_ptr(), mode.as_ptr()];
        if row.command { arguments.push(prefix.as_ptr()); }
        if row.stride { arguments.extend([stride.as_ptr(), two.as_ptr()]); }
        if let Some(path) = &path { arguments.extend([index.as_ptr(), path.as_ptr()]); }
        if row.unique { arguments.push(unique.as_ptr()); }
        arguments.push(container.as_ptr());
        let code = super::lsort(&mut interp, &arguments);
        let label = format!("{}:{}:{}", row.dialect, row.case, row.stream_sha256);
        assert_eq!(code, if row.code == 0 { Code::Ok } else { Code::Error }, "{label}");
        assert!(interp.native_execution_refusal().is_none(), "{label}");
        for (original, effect) in originals.into_iter().zip(row.effect) {
            if let (Some(original), Some((expected, resident))) = (original, effect) {
                assert_eq!(category(kind(original)), category(expected), "{label}");
                if row.code == 0 {
                    assert_eq!(obj::has_string_rep(original), resident, "{label}");
                }
            }
        }
        if let (Some(returned), Some((expected, resident))) = (&returned, row.return_effect) {
            assert_eq!(category(kind(returned.as_ptr())), category(expected), "{label}");
            assert_eq!(obj::has_string_rep(returned.as_ptr()), resident, "{label}");
        }
        let result = interp.get_obj_result();
        assert_eq!(interp.index_operand_bytes(&result).unwrap().as_ref(), unhex(row.result), "{label}");
        RETURNED.set(std::ptr::null_mut());
    }
}

const SIZE_MESSAGE: &[u8] = b"list size must be a multiple of the stride length";
const INDEX_MESSAGE: &[u8] = b"when used with \"-stride\", the leading \"-index\" value must be within the group";
const OBSERVED_SIZE_ERROR_INFO: &str = "6c6973742073697a65206d7573742062652061206d756c7469706c65206f662074686520737472696465206c656e6774680a202020207768696c6520657865637574696e670a226c736f7274202d696e7465676572202d7374726964652032202d696e6465782030207b32204120317d22";

fn unreached_group_callback(_: &mut Interp, _: &[*mut obj::TclObj]) -> Code {
    panic!("group validation must precede the selected comparison");
}
#[test]
fn original_grouping_failure_metadata_and_pre_frame_boundary_follow_native651_and_source() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Size count3/index0 matches observed651 case7 C86/9 exact code/message.
    // Leading index2 and simultaneous size/index errors are pinned-source
    // software controls only; no failing leading-index native651 output exists.
    // Assert exact private code and bare errorInfo at the direct command boundary;
    // the observed full native errorInfo includes a separate Eval frame.
    // Cold key primaries and unreachable callback are local software predicates;
    // neither callback0 nor these fields grants native getter/qsort chronology.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (count, path, expected_code, expected_message) in [
            (3, "0", b"TCL OPERATION LSORT BADSTRIDE".as_slice(), SIZE_MESSAGE),
            (4, "2", b"TCL OPERATION LSORT BADINDEX".as_slice(), INDEX_MESSAGE),
            (3, "2", b"TCL OPERATION LSORT BADSTRIDE".as_slice(), SIZE_MESSAGE),
        ] {
            for command in [false, true] {
                let mut interp = Interp::with_native_core(crate::interp::default_host(), crate::environment::profile_for_dialect(profile), tcl_registry::special_vars::NativeBootstrapInputs::default()).unwrap();
                let protocol = interp.native_invocation_dialect().native_string_protocol().unwrap();
                let keys: Vec<_> = [b"2".as_slice(), b"A", b"1", b"B"].into_iter()
                    .take(count).map(obj::new_string_bytes).collect();
                let container = obj::Owned::fresh(list::new_list_obj_native(&keys[..count], protocol));
                interp.register_builtin(b"r2286groupcmp", unreached_group_callback);
                let head = obj::new_string_bytes(b"r2286groupcmp");
                let prefix = obj::Owned::fresh(list::new_list_obj_native(&[head], protocol));
                let name = obj::Owned::fresh(obj::new_string_bytes(b"lsort"));
                let mode = obj::Owned::fresh(obj::new_string_bytes(if command { b"-command" } else { b"-integer" }));
                let stride = obj::Owned::fresh(obj::new_string_bytes(b"-stride"));
                let width = obj::Owned::fresh(obj::new_string_bytes(b"2"));
                let index = obj::Owned::fresh(obj::new_string_bytes(b"-index"));
                let path = obj::Owned::fresh(obj::new_string_bytes(path.as_bytes()));
                let mut arguments = vec![name.as_ptr(), mode.as_ptr()];
                if command { arguments.push(prefix.as_ptr()); }
                arguments.extend([stride.as_ptr(), width.as_ptr(), index.as_ptr(), path.as_ptr(), container.as_ptr()]);
                let code = super::lsort(&mut interp, &arguments);
                assert_eq!(code, Code::Error);
                assert!(interp.native_execution_refusal().is_none());
                assert_eq!(category(kind(width.as_ptr())), "integer");
                for &key in &keys[..count] { assert_eq!(category(kind(key)), "string"); }
                let result = interp.get_obj_result();
                assert_eq!(interp.index_operand_bytes(&result).unwrap().as_ref(), expected_message);
                assert_eq!(interp.error_code_bytes_checked().unwrap().as_slice(), expected_code);
                let direct_info = interp.error_info().unwrap();
                assert_eq!(direct_info, expected_message);
                if expected_message == SIZE_MESSAGE {
                    let full_native = unhex(OBSERVED_SIZE_ERROR_INFO);
                    assert!(full_native.starts_with(&direct_info));
                    assert_ne!(full_native, direct_info);
                }
            }
        }
    }
}

include!("../../tests/data/native_original_list_group_cli656/software_controls.rs");
#[test]
fn original_cli_group_fields_match_native656_source_capture() {
    // naming.list.original-cli-group-boundary-results
    // docs/design/analysis/name-resolution-proofs/list-original-cli-group-boundary-results.md
    // Same fixed Tcl capture prefix with separately initialised argv; compare
    // exact script/code/result/errorCode/errorInfo/presence and callback count.
    // Native process startup and private getter/header chronology are excluded.
    // Jim651 case12 is a different input and remains unavailable.
    let probe = include_str!("../../tests/data/native_original_list_group_cli656/probe.tcl");
    let capture = probe.split("puts \"case\\t").next().unwrap();
    assert_eq!(CLI_ROWS.len(), 66);
    for row in CLI_ROWS {
        let mut interp = Interp::with_native_core(crate::interp::default_host(), crate::environment::profile_for_dialect(row.dialect), tcl_registry::special_vars::NativeBootstrapInputs::default()).unwrap();
        let startup = format!("set argv {{{}}}", row.case);
        assert_eq!(interp.eval_str(startup.as_bytes()), Code::Ok);
        let label = format!("{}:{}:{}", row.dialect, row.case, row.stream_sha256);
        assert_eq!(interp.eval_str(capture.as_bytes()), Code::Ok, "{label}");
        assert!(interp.native_execution_refusal().is_none(), "{label}");
        for (name, expected) in CLI_VARIABLES.into_iter().zip(row.fields) {
            let original = interp.var_get(name.as_bytes()).unwrap();
            assert_eq!(interp.index_operand_bytes(&original).unwrap().as_ref(), unhex(expected), "{label}:{name}");
        }
    }
}

#[test]
fn original_guest_compare_failure_preserves_result_and_native651_context() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Native651 case27 records the same returned original, NONE and the exact
    // compare suffix. This direct adapter boundary excludes Eval's outer frame.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(crate::interp::default_host(), crate::environment::profile_for_dialect(profile), tcl_registry::special_vars::NativeBootstrapInputs::default()).unwrap();
        let protocol = interp.native_invocation_dialect().native_string_protocol().unwrap();
        let returned = obj::Owned::fresh(obj::new_string_bytes(b"CALLBACK"));
        RETURNED.set(returned.as_ptr());
        CALLBACK_ERROR.set(true);
        interp.register_builtin(b"r2286cmp", callback);
        let head = obj::new_string_bytes(b"r2286cmp");
        let prefix = obj::Owned::fresh(list::new_list_obj_native(&[head], protocol));
        let left = obj::new_string_bytes(b"B"); let right = obj::new_string_bytes(b"A");
        let container = obj::Owned::fresh(list::new_list_obj_native(&[left, right], protocol));
        let name = obj::Owned::fresh(obj::new_string_bytes(b"lsort"));
        let mode = obj::Owned::fresh(obj::new_string_bytes(b"-command"));
        assert_eq!(super::lsort(&mut interp, &[name.as_ptr(), mode.as_ptr(), prefix.as_ptr(), container.as_ptr()]), Code::Error);
        assert!(interp.native_execution_refusal().is_none());
        assert_eq!(interp.get_obj_result(), returned.as_ptr());
        let info = interp.error_info().unwrap();
        assert!(info.starts_with(b"CALLBACK"));
        assert!(info.ends_with(b"\n    (-compare command)"));
        assert_eq!(interp.error_code_bytes_checked().unwrap().as_slice(), b"NONE");
        RETURNED.set(std::ptr::null_mut());
    }
}
