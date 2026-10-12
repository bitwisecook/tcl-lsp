# VM lib-test compilation boundaries

Question: which current test-owner/API boundaries prevented the focused VM lib target from compiling, and what is the smallest genuine-input repair?

Evidence: [retained focused build log](../../../../experiments/name-handling/results/vm-c86-mathfunc-bootstrap/focused-tests.log), exit 101, 26 diagnostics, zero tests executed. Historical baseline remains unmeasured; these are errors outside the mathfunc change owners, not established pre-existing failures.

All owner paths below are relative to `rust/tcl-vm/src/`; locations are from that retained build.

| Owner | Count | Exact API mismatch clusters and lines |
| --- | ---: | --- |
| `cmd_oo/native_context/definition_exit/tests.rs` | 1 | L21 imports private `command::ok` rather than its defining owner. |
| `interp/oo_cleanup/active_frame/tests.rs` | 7 | L21 private `command::ok`; L260/324/341/431 use `TclError` where `try_*` produces `NativeExecutionError`; L367 passes `&str` to `refuse_host_command(String)`; L438 reads `activation_depth` from `InterpState`, which has no such field. |
| `value_ops/native_jim_index.rs` | 2 | L1075 passes `&str` to `refuse_host_command(String)`; L1371 passes `&[u8; 3]` to the native byte constructor's `Into<Rc<[u8]>>` parameter. |
| `value_ops/native_jim_index/c84_integer_result_tests.rs` | 2 | L57/76 `Result::unwrap` requires `LsearchError: Debug`, which is unavailable. |
| `cmd_binary.rs` | 5 | L1200/1207/1455/1492 access private `InterpState::native_operation_context`; L1223 has no `Code` in scope. |
| `cmd_list/native_original_group_command_controls.rs` | 5 | L101/234/331/336/337 pass byte-array references to `new_native_string_bytes(impl Into<Rc<[u8]>>)`, rather than byte slices. |
| `cmd_package.rs` | 2 | L2130/2158 pass `&Vm` where `selected_loader` requires `&OriginalPackage<'_>`. |
| `interp/execution_name_policy/original_event_transition_tests.rs` | 1 | L215 calls private `NativeOperationScope::replace_observed_inputs`. |
| `vars/retirement.rs` | 1 | L435 returns `Option<u64>` for a test-helper signature declaring `Option<usize>`. |
| Total | 26 | Nine owners; diagnostic codes: E0603×2, E0308×9, E0277×8, E0616×4, E0433×1, E0624×1, E0609×1. |

Only two representative test owners were inspected: `value_ops/native_jim_index.rs` and `cmd_list/native_original_group_command_controls.rs`. Other clusters are classified from compiler diagnostics, without asserting their intended repair.

The current public [byte constructor](../../../../rust/tcl-vm/src/value.rs#L5288) retains exact native bytes and assigns storage from actual empty/nonempty input; it accepts `impl Into<Rc<[u8]>>`. The current public [host refusal producer](../../../../rust/tcl-vm/src/interp.rs#L23339) takes an owned `String` and retains the actual interpreter/context cause.

Smallest next job after the active source freeze: own only `value_ops/native_jim_index.rs` and change its two argument shapes. At L1075, pass the same refusal text with `.to_owned()` and deliberately discard the returned unwind carrier with `let _ =`, retaining the complete first cause. At L1371, pass `b"a\0b".as_slice()` to the same native constructor; preserve the embedded NUL and existing actual C9.0 core/helper invocation.

Keep both fixture assertions: the authentic earlier host cause and object snapshot/reference count remain unchanged; the parsed empty replacement returns a distinct duplicate retaining exactly `a\0b`. Neither repair manufactures a Native entry, a protocol receipt, or authority from a fixture label. The grouped-sort owner provides the same byte-array mismatch at five sites, but can be a separate bounded follow-up.

Validation must remain explicit: two local fixes alone leave 24 other retained diagnostics unresolved, so they do not establish a green lib target. After remaining owners compile, execute the exact two named controls (`original_jim_index_cache_preserves_the_complete_earlier_host_cause` and `omitted_zero_count_without_in_place_uses_original_duplicate_constructor`) with their genuine native constructors; do not replace them with fake compilation entries.

This review changed documentation only. No Rust edits, builds, cache writes, Git/network actions, or raw F5 reads were performed.
