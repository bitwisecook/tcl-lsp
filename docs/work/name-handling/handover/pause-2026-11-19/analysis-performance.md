# Analysis performance, 19 November 2026

The owner paused work before either clean DB timings or a new #2543 churn run completed. No server, debugger or profiler job remains running from this session.

## #2543

The historical 2.7 MB/round is the 40-document 100→200 figure at `8febb23f`, with trimmed RSS moving from 2.18 to 2.45 GB. A new measurement needs a confirmed allocator invocation. The stress harness previously reported untrimmed RSS as trimmed when gdb was absent or attach failed. `032e84d0b6` (combined `1974b7f4c1`) makes failed trim samples null with `trim_error` and records the allocator result for successful calls. The formatter follow-up `ca48b18add` is combined `c278c59eeb`, with a portable copy at `patches/performance-confirmed-trim-format.patch`. The 24 existing Python tests and four direct success/failure controls passed. Debian gdb 16.3 was provisioned at `/workspace/bd-build/debug-tools/bin/gdb`; an isolated attached allocator returned `malloc_trim_result=1`. A fresh host can use its own gdb. Check a successful attach before collecting trimmed samples.

For comparable counts, build an isolated clean-head debug server with the historical instrumentation from `issues/server-2` at `72bb19cac9`. The portable pieces are `patches/performance-2543-debug-memory.patch` (Cargo feature and routing) and `measurement/debug_memory.rs.txt` (copy to `rust/tcl-lsp-server/src/commands/debug_memory.rs`). Only apply these to a measurement checkout. They applied against a6c66517c7 during preparation. These are local measurement hooks. Reconcile the patch with current source before changing HEAD. The comparable builds use development profile opt-level 2, a single build job and no incremental artifacts.

```bash
CARGO_PROFILE_DEV_OPT_LEVEL=2 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo build --locked -p tcl-lsp-server
# BIN is the copied instrumented executable. Run the corrected driver separately.
python3 scripts/dev/pack_churn_stress.py --server-bin "$BIN" --workdir "$RUN40" --log "$RUN40/run.jsonl" --rounds 200 --max-documents 40 --rss-every 25 --malloc-trim --memory-command tcl-lsp.debugMemory
python3 scripts/dev/pack_churn_stress.py --server-bin "$BIN" --workdir "$RUN183" --log "$RUN183/run.jsonl" --rounds 250 --rss-every 10 --slow-probe 2 --gdb
```

Run the 40-document trace alone, identify the dominant heap owner over rounds 100→200, and repeat after a targeted fix. Record the full 183-document latency and RSS trace independently. For historical Massif scripts and the original command matrix see the [server issue handoff](../../../issue-sessions/handover/server.md). The 183-document fixture normally needs 6–9 GB. This session collected zero fresh #2543 rounds.

## Compiler order dependence and per-body memo

The known witnesses are `command_binding::source_callable_static_tests::static_formals_update_one_retained_slot_across_actual_calls` and `command_binding::source_command_world::tests::original_opaque_jim_callable_name_does_not_gate_ascii_static_capture`. The 18 November evidence at 2a026629 showed each passes on its own but both fail inside the shared `command_binding::` one-thread process. Nextest’s fresh process per case cannot show the offending neighbour. Use the clean compiler libtest via the exact cargo manifest and rerun the subset within one process using one thread. Then bisect by skipping individual prior test names while keeping the target in that process. The a6c66517c7 baseline binary was at `/workspace/bd-build/lsp/debug/build/tcl-compiler/d067023007bb1ffa/out/tcl_compiler-d067023007bb1ffa`. Verify its head and features before timing. No new same-process bisect was run. Separate nextest baselines belong to diagnostics.

The held wrapper is still in `../../patches/handover-analysis-performance-body-wrapper-realm.patch`; use the 18 November handoff for the two remaining security parity controls and per-body rollout. The clean DB case timings remain with server-e2e-db until it supplies a named case. Keep the tickets and evidence separate.
