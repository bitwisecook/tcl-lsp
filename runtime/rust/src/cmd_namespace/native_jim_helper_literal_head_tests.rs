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

use crate::interp::{Code, Interp};
use crate::obj::{self, Owned, TclObj};
use tcl_syntax::value::ValueOps;

// Original Jim capture: experiments/name-handling/results/namespace-helper-original-selector.
const EVIDENCE: &str =
    include_str!("../../tests/data/native_jim_helper_literal_head/prefix-selection-20261015.tsv");
fn row(kind: &str, phase: &str) -> Vec<&'static str> {
    EVIDENCE
        .lines()
        .find_map(|line| {
            let fields: Vec<_> = line.split('|').collect();
            (fields[0] == kind && fields[1] == phase).then(|| fields[2..].to_vec())
        })
        .unwrap_or_else(|| panic!("missing genuine row {kind}/{phase}"))
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for byte in bytes {
        write!(&mut output, "{byte:02x}").unwrap();
    }
    output
}
fn check_header(kind: &str, phase: &str, original: *mut TclObj) {
    let descriptor = obj::obj_type_ptr(original);
    let name = if descriptor.is_null() {
        "none"
    } else {
        // SAFETY: the retained original's static descriptor has a terminated name.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
    };
    assert_eq!(
        [
            u8::from(obj::has_string_rep(original)).to_string(),
            name.to_owned()
        ],
        row(kind, phase),
        "{kind}/{phase}"
    );
}
fn phase(interp: &mut Interp, label: &str, reuse: bool, originals: [&Owned; 3]) {
    let [caller, head, member] = originals;
    let phase = format!("{label}_{}", if reuse { "REUSE" } else { "COLD" });
    if reuse {
        obj::invalidate_string(caller.as_ptr());
    }
    let context = interp.native_jim_object_context().unwrap();
    let argument = Owned::fresh(obj::new_wide_int_obj(84));
    crate::native_source::bind_context(argument.as_ptr(), &context).unwrap();
    let vector =
        Owned::fresh(interp.new_list_object(&[head.as_ptr(), member.as_ptr(), argument.as_ptr()]));
    check_header("CALLER_BEFORE", &phase, caller.as_ptr());
    check_header("ARGUMENT_BEFORE", &phase, argument.as_ptr());
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(Owned::retain(caller.as_ptr()));
    let code = interp.ns_eval_in_token(
        namespace,
        &[],
        None,
        false,
        Some((
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
            vector.as_ptr(),
        )),
        None,
    );
    check_header("CALLER_AFTER", &phase, caller.as_ptr());
    check_header("ARGUMENT_AFTER", &phase, argument.as_ptr());
    if code == Code::Ok {
        let result = interp.result_obj();
        let elements = interp.list_elements(&result).unwrap();
        assert_eq!(
            u8::from(elements.len() == 2 && elements[1] == argument.as_ptr()).to_string(),
            row("RESULT_ARGUMENT_IS_ORIGINAL", &phase)[0],
            "{phase}"
        );
    }
    let expected = row("RESULT", &phase);
    assert_eq!(code.as_int().to_string(), expected[0], "{phase}");
    assert_eq!(hex(&interp.result_bytes()), expected[1], "{phase}");
    assert!(interp.native_execution_refusal().is_none(), "{phase}");
}
#[test]
fn original_jim_public_helper_lookup_retains_the_recognised_original_selector() {
    assert_eq!(EVIDENCE.lines().count(), 200);
    // Public arity rendering, counted-prefix rejection and subcmd-lookup header
    // ownership are independently recorded stages, outside this helper lookup.
    for (label, selector, change) in [
        ("EXACT", b"upvar".as_slice(), 0),
        ("ABBREVIATED", b"upv", 0),
        ("EXACT_NUL", b"upvar\0TAIL", 0),
        ("ABBREVIATED_CANONICAL_REPLACED", b"upv", 1),
        ("ABBREVIATED_SHORT_REPLACED", b"upv", 2),
        ("EXACT_SHORT_REPLACED", b"upvar", 2),
        ("EXACT_BOTH_REPLACED", b"upvar", 3),
        ("ABBREVIATED_BOTH_REPLACED", b"upv", 3),
        ("EXACT_NUL_BOTH_REPLACED", b"upvar\0TAIL", 3),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        if change & 1 != 0 {
            assert_eq!(
                interp.eval_str(b"proc {namespace upvar} {args} {return [list FULL {*}$args]}"),
                Code::Ok
            );
        }
        if change & 2 != 0 {
            assert_eq!(
                interp.eval_str(b"proc {namespace upv} {args} {return [list SHORT {*}$args]}"),
                Code::Ok
            );
        }
        let context = interp.native_jim_object_context().unwrap();
        let caller = Owned::fresh(obj::new_wide_int_obj(42));
        let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
        let member = Owned::fresh(obj::new_string_bytes(selector));
        for original in [&caller, &head, &member] {
            crate::native_source::bind_context(original.as_ptr(), &context).unwrap();
        }
        for reuse in [false, true] {
            phase(&mut interp, label, reuse, [&caller, &head, &member]);
        }
        interp.set_current_ns(crate::namespace::GLOBAL);
    }
}
