# Server issue session: unfinished measurement tools (#1657, #2543)

Not finished or tested to the standard of `scripts/dev/`; kept so a successor can repeat the
measurements. The hand-over is `docs/work/issue-sessions/handover/server.md` on `work`.

| File | What it is |
| --- | --- |
| `debug-memory.patch` + `debug_memory.rs` | Temporary `tcl-lsp.debugMemory` command: salsa struct/query counts (`memory_usage()`, needs the `salsa_unstable` feature the patch turns on), epochs. Apply with `git apply`, copy the `.rs` to `rust/tcl-lsp-server/src/commands/debug_memory.rs`. Never commit. |
| `cfg_log_instrument.py` | Temporary per-execution log of `lowered_document_module` and fallback `CfgContext` field hashes, to `$TCL_LSP_DEBUG_CFG`. Found the interned-GC reclaim behind 8febb23f. |
| `massif-server.sh` | valgrind massif wrapper usable as the driver's `--server-bin`; set `OUT` and `SERVER`. |
| `snap_at.py LOG PREFIX ROUND...` | Watches the driver's JSONL log and takes a massif `detailed_snapshot` via `vgdb` at each named round's `rss` event. |
| `msdiff.py A B` | Inclusive-bytes diff of two massif snapshots by function. |
| `msroots.py A B` | The same diff grouped by the owning salsa query frame. |
| `summ.py STACKS [N]` | Groups a gdb `thread apply all bt` dump by thread name and top frames. |
| `id_carrier_scan.py` | Text scan for types that transitively carry `ProcedureId` (run from `rust/`). |
| `data/ms8-{100,200}.ms` | Massif snapshots, 10 documents, rounds 100 and 200, on 8febb23f (depth 30). |
| `data/ms8-50.ms`, `data/ms9-100.ms` | Massif snapshot at round 50 (depth 30), and at round 100 with depth 150, from the run killed by the pause (its round-200 snapshot was never taken). |
| `data/runs/*.jsonl` | Every driver log from this session, including those quoted on #2543. 0f09b12e era: `final-0f09b12e`, `final-latency` (183 documents), `sets-40` (40-document counts), `attr-40`, `scale-*`. ac289446: `memo-40`, `memo-40-trim`. 43d017e0: `b2-40-trim`. 8febb23f: `b3-40-trim`, `b3-183`, `b3-10-trim`, `m8`/`m9` (under massif). |
| `data/runs/cfg*.log` | `cfg_log_instrument.py` output; `cfg2.log` is the 30-round run that showed the key-slot recycling on 43d017e0. |
