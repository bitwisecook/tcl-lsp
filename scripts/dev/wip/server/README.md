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
