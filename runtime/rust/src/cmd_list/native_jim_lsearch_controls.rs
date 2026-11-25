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
thread_local! { static CALLBACKS: std::cell::RefCell<Vec<Vec<(String, usize)>>> = const { std::cell::RefCell::new(Vec::new()) }; }
fn kind(value: *mut TclObj) -> String {
    let descriptor = obj::obj_type_ptr(value);
    if descriptor.is_null() {
        "none".to_owned()
    } else {
        // SAFETY: the observed live original owns this static type descriptor.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
            .to_owned()
    }
}
fn twice(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().push(
            argv.iter()
                .map(|&value| {
                    // SAFETY: actual ObjVector retains every original during this callback.
                    (
                        kind(value),
                        usize::try_from(unsafe { (*value).ref_count }).unwrap(),
                    )
                })
                .collect(),
        )
    });
    interp.set_result(obj::new_wide_int_obj(2));
    Code::Ok
}
fn returns(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    interp.set_result(obj::new_string_bytes(b"ORIGINAL_RETURN"));
    Code::Return
}
#[test]
fn jim_lsearch_preserves_all_24_original_native_option_and_callback_controls() {
    let rows = include_str!("../../../../rust/tcl-cmd-core/tests/data/native_jim_lsearch/rows.txt");
    let hex = |text: &str| {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut count = 0;
    for record in rows.split("case\t").skip(1) {
        let lines: Vec<_> = record.lines().collect();
        let case: usize = lines[0].parse().unwrap();
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        interp.register_builtin(b"two", twice);
        interp.register_builtin(b"returns", returns);
        if case == 8 {
            interp.register_builtin(b"regexp", twice);
        }
        CALLBACKS.with(|callbacks| callbacks.borrow_mut().clear());
        let originals: Vec<_> = tcl_test_support::fixture::tsv_fields(lines[1])
            .skip(1)
            .map(|word| obj::Owned::fresh(obj::new_string_bytes(&hex(word))))
            .collect();
        let head = obj::Owned::fresh(obj::new_string_bytes(b"lsearch"));
        let argv: Vec<_> = std::iter::once(head.as_ptr())
            .chain(originals.iter().map(obj::Owned::as_ptr))
            .collect();
        let code = interp.eval_original_object_vector(&argv);
        let expected_code: i32 = lines
            .iter()
            .find_map(|line| line.strip_prefix("code\t"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(code.as_int(), i64::from(expected_code), "case {case}");
        let value = interp.get_obj_result();
        let expected: Vec<_> = tcl_test_support::fixture::tsv_fields(
            lines
                .iter()
                .find_map(|line| line.strip_prefix("result\t"))
                .unwrap(),
        )
        .collect();
        assert_eq!(kind(value), expected[0], "case {case}");
        assert_eq!(
            obj::has_string_rep(value),
            expected[1] == "1",
            "case {case}"
        );
        // SAFETY: the actual interpreter result owns the observed live header.
        assert_eq!(
            unsafe { (*value).ref_count },
            expected[2].parse::<isize>().unwrap(),
            "case {case} original result refs"
        );
        for line in lines
            .iter()
            .filter_map(|line| line.strip_prefix("argument\t"))
        {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
            let value = originals[fields[0].parse::<usize>().unwrap() - 1].as_ptr();
            assert_eq!(kind(value), fields[1], "case {case} argument {}", fields[0]);
            assert_eq!(
                obj::has_string_rep(value),
                fields[2] == "1",
                "case {case} argument {}",
                fields[0]
            );
            // SAFETY: the external original owner remains live throughout capture.
            assert_eq!(
                unsafe { (*value).ref_count },
                fields[3].parse::<isize>().unwrap(),
                "case {case} argument {}",
                fields[0]
            );
        }
        let native_callbacks: Vec<_> = lines
            .iter()
            .filter_map(|line| line.strip_prefix("callback\t"))
            .map(|line| {
                tcl_test_support::fixture::tsv_fields(line)
                    .skip(1)
                    .map(|field| {
                        let (kind, refs) = field.split_once(':').unwrap();
                        (kind.to_owned(), refs.parse::<usize>().unwrap())
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        CALLBACKS.with(|callbacks| {
            assert_eq!(
                *callbacks.borrow(),
                native_callbacks,
                "case {case} original callback argv"
            )
        });
        let expected_bytes = hex(lines
            .iter()
            .find_map(|line| line.strip_prefix("bytes\t"))
            .unwrap());
        assert_eq!(interp.result_bytes(), expected_bytes, "case {case}");
        count += 1;
    }
    assert_eq!(count, 24);
}
