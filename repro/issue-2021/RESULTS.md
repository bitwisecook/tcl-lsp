# Issue #2021 reproduction — orphaned `tcl-lsp-server` after the client goes away

Harness: `orphan_repro.py` (stdlib-only Python 3, Content-Length JSON-RPC over stdio).
Driver: `run_all.sh`. Raw per-run records: `runs.jsonl`, `runs-long.jsonl`.
Full console transcripts: `driver.out` / `runs.log`, `long.out`, `tail.out`.

## Environment

| item | value |
|---|---|
| binary | `/home/user/tcl-lsp/target/release/tcl-lsp-server` (release, built this session) |
| host | 4 × Intel Xeon @ 2.80 GHz, 16 GiB RAM, Linux 6.18.44 |
| primary workspace | `/home/user/tcl-lsp/tmp/tcllib-2.0` — 883 `.tcl` files |
| second workspace | `/home/user/tcl-lsp/tmp/tcl8.6.18/library` — 23 `.tcl` files |
| document opened | tcllib: `modules/practcl/practcl.tcl` (8463 lines, 263 816 B); tcl8.6: `clock.tcl` (4568 lines, 129 006 B) |
| per run | `initialize` → `initialized` → `didOpen` → 2 × `didChange` → `semanticTokens/full` → teardown |

4 cores means **400 % is the ceiling**; the issue's reported ~401 % is exactly "all cores saturated",
which is what every orphan here shows.

The harness answers server-to-client requests (`workspace/configuration`,
`client/registerCapability`, `workspace/*/refresh`) like a real client **until teardown**, and stops
answering at teardown in every scenario — so the only variable `exit-stdout-open` changes versus
`clean` is whether the server's stdout write end is broken.

## Results table

`midscan` = teardown 1.5 s after `initialized`. `settled` = teardown after the
`[timing] workspace_folders_scan` log line + 3 s, bounded by `--scan-timeout 300`.

| # | scenario | at | workspace | scan line seen at teardown | verdict | CPU (avg over window) | RSS at end | threads |
|---|---|---|---|---|---|---|---|---|
| 1 | `clean` | midscan | tcllib-2.0 | no | **ORPHAN alive after 45 s** | 385 % | 962 MiB | 14 |
| 2 | `eof` | midscan | tcllib-2.0 | no | **ORPHAN alive after 45 s** | 388 % | 977 MiB | 14 |
| 3 | `shutdown-eof` | midscan | tcllib-2.0 | no | **ORPHAN alive after 45 s** | 390 % | 990 MiB | 15 |
| 4 | `exit-stdout-open` | midscan | tcllib-2.0 | no | **ORPHAN alive after 45 s** | 392 % | 982 MiB | 14 |
| 5 | `clean` | settled | tcllib-2.0 | no (300 s cap hit) | EXITED after 19.1 s | 100 % | 987 MiB | 8 → 1 |
| 6 | `eof` | settled | tcllib-2.0 | no (300 s cap hit) | EXITED after 27.1 s | 100 % | 1212 MiB | 7 → 6 |
| 7 | `shutdown-eof` | settled | tcllib-2.0 | no (300 s cap hit) | EXITED after 23.1 s | 100 % | 1225 MiB | 8 → 6 |
| 8 | `exit-stdout-open` | settled | tcllib-2.0 | no (300 s cap hit) | EXITED after 8.0 s | 100 % | 1124 MiB | 8 |
| 9 | `eof` | settled | tcl8.6.18/library | **yes** (`4250 ms, roots=1, files=23`) | EXITED after 1.0 s | – | – | – |

Supplementary runs (same harness, longer observation windows):

| # | scenario | at | workspace | observe | verdict |
|---|---|---|---|---|---|
| L1 | `eof` | midscan | tcllib-2.0 | 900 s | **EXITED after 318.3 s** (self-terminated) |
| L2 | `eof` | midscan | tcllib-2.0 | 150 s | **ORPHAN alive after 150 s** (cpu avg 190 %, 1176 MiB, 6 threads) — gdb sample taken, then SIGKILLed |

Nothing was OOM-killed: every server stderr log is empty (0 bytes) and `dmesg` shows no OOM events.
All exits were the process terminating on its own.

## What the numbers show

**Every one of the four teardown styles orphans identically when the client goes away during the
workspace scan.** Runs 1–4 are indistinguishable: ~390 % CPU from the first second, RSS climbing
monotonically 223–255 MiB → 962–990 MiB over 45 s, 14–15 threads, process state `S`/`R`.

That rules out three candidate causes:

* **Not EPIPE-driven.** `exit-stdout-open` (run 4) keeps the read end open for the whole window —
  the server never gets EPIPE — and behaves exactly like `clean` (run 1), which does get EPIPE.
* **Not "the client failed to say shutdown/exit".** `clean` (run 1) got a `shutdown` **response**
  back within the 2 s budget and then sent `exit`; the server still orphaned. `eof` (run 2) sent
  neither. Same outcome.
* **Not the `exit` notification being lost.** `shutdown-eof` (run 3, what vscode-languageclient does
  when `stop()` times out) and `clean` (run 1, `exit` delivered) are the same.

RSS trajectory for run 1 (representative, `t` seconds after teardown):

```
 1s 223M   5s 422M  10s 568M  15s 622M  20s 725M  25s 742M
30s 820M  35s 921M  40s 922M  45s 962M
```

Growth continues for as long as the scan runs. Run L1 shows the ceiling on this corpus: RSS peaked
around 1194 MiB and the process exited at 318.3 s.

**The orphan's lifetime here is exactly "however long the workspace scan still has to run".**
Run L1 (900 s window) is decisive: after `eof` at 1.5 s the server ran ~40 s with all 4 cores
saturated, dropped to a single-threaded 100 % phase for another ~4.5 min, and then **exited by
itself at 318.3 s**. Runs 5–8 are the same fact from the other end: their teardown at +300 s landed
after the parallel batch phase had drained (100 % CPU, 6–8 threads at teardown), and the leftover
work finished in 8–27 s.

Run 9 is the control: on a 23-file workspace the scan completed (4250 ms), `initialized` ran to
completion (a second `workspace/foldingRange/refresh` was observed and answered), and a bare stdin
EOF made the server exit in **1.0 s**.

**Caveat, stated plainly:** on this corpus the orphan is bounded, not permanent — it terminated on
its own at 318 s. This reproduction therefore reproduces the *state* the issue describes (surviving
process, all cores saturated, RSS climbing, thread count in the teens, immune to `shutdown`/`exit`)
but not its *18-hour duration*. Whether the reporter's 3217-file Quartus corpus simply extends the
same bounded scan far enough to look permanent, or whether something else keeps it alive
indefinitely, is not answered by these runs.

## gdb: what the orphan's threads are doing

Saved samples: `clean-midscan.gdb.txt`, `eof-midscan.gdb.txt`, `shutdown-eof-midscan.gdb.txt`,
`exit-stdout-open-midscan.gdb.txt` (all at t+45 s), `eof-midscan-tail.gdb.txt` (at t+150 s).

All four midscan samples agree exactly. Counting the poll frames in each file:

```
4 × BlockingTask<Backend::analyse_and_merge_scanned_files::{closure#0}::{closure#0}::{closure#0}>
4 × BlockingTask<multi_thread::worker::Launch::launch::{closure#0}>      (idle runtime workers)
```

**Busy threads — exactly 4, i.e. one per core, all inside the workspace scan.**
Bottom frames are identical in every busy thread:

```
#N  tokio::runtime::task::raw::poll::<BlockingTask<
        <tcl_lsp_server::Backend>::analyse_and_merge_scanned_files::{closure#0}::{closure#0}::{closure#0}
    >, BlockingSchedule>
```

Top frames from `clean-midscan.gdb.txt`:

* Thread 9 — registry lookup during dispatch-site diagnostics
  ```
  #0  <core::fmt::Formatter>::pad
  #2  alloc::fmt::format::format_inner
  #3  <tcl_dialect::model::authored_surface::SpecSurface>::covers
  #4  tcl_dialect::model::authored_surface::surface_admits
  #5  <tcl_registry::registry::CommandRegistry>::spec_visible
  #6  <tcl_registry::registry::CommandRegistry>::get_for_surface
  #7  <tcl_registry::registry::CommandRegistry>::resolve_structured_invocation
  #8  <Analyser>::emit_dispatch_site_diagnostics
  #9  <Analyser>::process_command  → #10 analyse_body → #13 handle_proc_command → #16 <Analyser>::analyse
  ```
* Thread 6 — interprocedural taint/SSA
  ```
  #0  tcl_compiler::taint::local_instance_classes_with_initial
  #1  tcl_compiler::ssa::enrich_instance_option_defs_with_initial
  #2  tcl_compiler::interprocedural::direct_instance_option_writes
  #3  <tcl_compiler::compilation_unit::CompilationUnit>::build_with
  #4  <Analyser>::emit_cfg_ssa_diagnostics → #5 run_diagnostic_emitters → #6 <Analyser>::analyse
  ```
* Thread 4 — subcommand-table resolution inside the same taint pass
  ```
  #0  __memcmp_evex_movbe
  #1  tcl_dialect::version::satisfies_internal
  #2  <SpecSurface>::covers → #3 surface_admits
  #4  <tcl_registry::spec::CommandSpec>::subcommand_table
  #5  <CommandRegistry>::resolve_structured_invocation
  #6  <CommandRegistry>::command_binding_transitions
  #7  tcl_compiler::alias::command_table_transitions
  #8  taint::transfer_instance_lifecycle → #9 transfer_instance_statement
  #10 taint::local_instance_classes_with_initial → #11 instance_classes_for_function
  #12 <FunctionUnit>::build_full → #14 <CompilationUnit>::build_with
  #15 <Analyser>::emit_cfg_ssa_diagnostics → #17 <Analyser>::analyse
  ```
* Thread 2 — lexing for noqa suppressions
  ```
  #0  <tcl_lexer::lexer::Lexer as Iterator>::next
  #1  <tcl_lexer::lexer::Lexer>::tokenise_all
  #2  <tcl_compiler::analyser::utils::CommentLineWalker>::visit
  #4  tcl_compiler::analyser::utils::script_comment_facts
  #5  tcl_compiler::analyser::utils::parse_noqa_line_suppressions_for_dialect
  #6  <Analyser>::analyse
  ```

Other midscan samples show the same call graph at different points, e.g.
`eof-midscan.gdb.txt` (`<LineIndex>::new` ← `scan_invocation_words` ← `check_simple_arity`;
`drop_glue::<tcl_compiler::ir::Script>`) and `shutdown-eof-midscan.gdb.txt`
(`variable_write_effects_from_commands` ← `<CfgBuilder>::condition_out_vars` ←
`lower_script_statement`).

**Parked threads.** The main thread is parked in the runtime, i.e. `Server::serve` has not returned:

```
Thread 1 "tcl-lsp-server":
#0  syscall
#1  <tokio::runtime::park::Inner>::park
#2  tcl_lsp_server::main
#3  std::sys::backtrace::__rust_begin_short_backtrace::<fn(), ()>
#5  std::rt::lang_start_internal
#6  main
```

The remaining threads are idle Tokio machinery, in two shapes — scheduler workers with no work
(`worker::Context::park_internal` → `worker::run`, one of them additionally in
`<tokio::runtime::time::Driver>::park_internal`) and free blocking-pool threads waiting on the pool
condvar (`<std::sys::sync::condvar::futex::Condvar>::wait_optional_timeout` ←
`blocking::pool::Spawner::spawn_thread`). **No thread anywhere in these samples is parked on a
server-to-client request, a client socket send, or a diagnostics publish.**

**The single-threaded 100 % tail** (`eof-midscan-tail.gdb.txt`, t+150 s) is one straggler file still
in the same scan task — 5 idle threads plus:

```
Thread 2 (the only busy one):
#0  __memcpy_evex_unaligned_erms
#1  <tcl_registry::registry::CommandRegistry>::command_binding_transitions
#2  tcl_compiler::alias::command_table_transitions
#3  tcl_compiler::taint::transfer_instance_lifecycle
#4  tcl_compiler::taint::transfer_instance_statement
#5  tcl_compiler::taint::local_instance_classes_with_initial
#6  tcl_compiler::ssa::enrich_instance_option_defs_with_initial
#7  tcl_compiler::interprocedural::direct_instance_option_writes
#8  tcl_compiler::interprocedural::build_interprocedural_analysis_inner
#9  <tcl_compiler::compilation_unit::CompilationUnit>::with_interprocedural
#10 <Analyser>::emit_cfg_ssa_diagnostics
#11 <Analyser>::run_diagnostic_emitters
#12 <Analyser>::analyse
#13 tokio::runtime::task::raw::poll::<BlockingTask<
        <Backend>::analyse_and_merge_scanned_files::{closure#0}::{closure#0}::{closure#0}>, …>
```

## Mechanism as evidenced

1. `initialized` runs `scan_workspace_folders`, whose stage 2
   (`analyse_and_merge_scanned_files`) fans the corpus across `spawn_blocking` workers.
2. The client goes away. `Server::serve` cannot return until the `initialized` notification's future
   completes, so the runtime keeps running.
3. Nothing cancels the scan on stdin EOF, on `shutdown`, on `exit`, or on EPIPE. It runs to
   completion at full parallelism, and the process only exits afterwards.
4. `workspace/foldingRange/refresh` at the end of `initialized` is *not* what pins these runs: it
   was never reached in runs 1–4 (only the earlier config-time refreshes appear in
   `server_requests_all`), and run L1 exited without it having to be answered.

## Reproducing a single scenario

```bash
python3 /tmp/claude-0/-home-user-tcl-lsp/e2b9cc4f-a1cb-5ec8-b576-3c86047f6744/scratchpad/repro2021/orphan_repro.py \
  --binary   /home/user/tcl-lsp/target/release/tcl-lsp-server \
  --workspace /home/user/tcl-lsp/tmp/tcllib-2.0 \
  --scenario eof --at midscan --observe 45
```

`--scenario {clean,eof,shutdown-eof,exit-stdout-open}`, `--at {midscan,settled}`,
`--observe SECONDS`, `--scan-timeout SECONDS`, `--tag NAME` (output-file suffix),
`--json FILE` (append the run record), `--keep` (leave a surviving server alive and print its PID).
A surviving server is always SIGKILLed at the end of a run unless `--keep` is given.

The full 9-run sweep is `bash run_all.sh` (runs are strictly sequential so CPU numbers stay clean).
