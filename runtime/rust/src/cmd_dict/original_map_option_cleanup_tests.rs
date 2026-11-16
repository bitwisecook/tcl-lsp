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
use crate::interp::default_host;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::WeakInterp;
use tcl_cmd_core::return_options::ReturnOptionPair;

extern "C" fn retire(value: *mut TclObj) {
    let weak = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut WeakInterp) };
    if let Some(mut interpreter) = weak.upgrade() {
        interpreter.refuse_host_command("original dict map option retirement");
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalDictMapOption".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_dict_map_option_refusal_retires_its_allocated_accumulator() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software accumulator cleanup under a member free, not a Native map chronology claim.
    crate::counters::reset();
    {
        let environment =
            tcl_registry::model::ingress::resolve_known_environment("tcl8.6").unwrap();
        let mut interpreter = Interp::with_native_core(
            default_host(),
            environment.unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let value = obj::Owned::fresh(obj::alloc_typed(
            &ORIGINAL,
            Box::into_raw(Box::new(interpreter.downgrade())) as usize as u64,
        ));
        interpreter
            .set_return_option_objects_checked(vec![ReturnOptionPair {
                key: obj::Owned::fresh(obj::new_string_bytes(b"-original")),
                value,
                key_bytes: b"-original".to_vec(),
            }])
            .unwrap();
        let words: Vec<_> = [
            b"dict".as_slice(),
            b"map",
            b"key value",
            b"a A b B",
            b"set ::original_map_effect REACHED",
        ]
        .into_iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
        let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
        let scope = NativeOperationScope::enter(&interpreter).unwrap();
        assert_eq!(map(&mut interpreter, &argv), Code::Error);
        let first = interpreter.native_execution_refusal().unwrap();
        let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(payload) = &first else {
            panic!("actual option free must retain its full Host cause");
        };
        assert_eq!(payload.reason, "original dict map option retirement");
        assert_eq!(
            scope.currency().ensure_current_or_refuse().err(),
            Some(first)
        );
        assert_eq!(
            crate::vars::get_at(
                &interpreter.frames.borrow(),
                &interpreter.namespaces(),
                b"::original_map_effect",
                0
            ),
            None
        );
        drop(scope);
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
