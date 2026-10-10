// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual missing-command completion uses the ordinary interpreter result owner.

use crate::{Code, Value};
use tcl_syntax::native_object::NativeObjectCacheSnapshot;

#[test]
fn original_missing_lmap_lookup_publishes_the_same_native_result_header() {
    // Native proof: naming.each-loop.original-lmap-collection
    // docs/design/analysis/name-resolution-proofs/each-loop-original-lmap-collection.md
    // Original native_each_loop case4 records the result String window after
    // this absent command; no loop body is entered on these two providers.
    for engine in ["tcl8.4", "tcl8.5"] {
        let profile = crate::environment::profile_for_dialect(engine);
        let mut vm = crate::native_fixture::interpreter(profile);
        let strings = vm
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let arguments = [
            Value::native_list_constructor(
                vec![Value::new_native_string_bytes(b"v".as_slice())],
                strings,
            ),
            Value::native_list_constructor(Vec::new(), strings),
            Value::new_native_string_bytes(b"{".as_slice()),
        ];
        let completion = vm.try_invoke_command("lmap", &arguments).unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}");
        vm.with_native_interp_result(|original| {
            assert!(original.is_same_object(&completion.result), "{engine}");
            assert!(
                matches!(
                    original.native_object_snapshot().cache,
                    NativeObjectCacheSnapshot::String { .. }
                ),
                "{engine}"
            );
        })
        .unwrap();
        assert_eq!(
            vm.native_name_operand_bytes(&completion.result)
                .unwrap()
                .as_ref(),
            b"invalid command name \"lmap\"",
            "{engine}"
        );
    }
}
