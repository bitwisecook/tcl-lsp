# C8.6 mathfunc constructor roster and procedure birth

Question: does the actual C8.6 constructor roster permit the library's genuine
mathfunc procedure births while preserving compiler currency, and does matching
real initialisation reach the retained startup marker?

The shared bootstrap protocol now supplies C8.6's complete 29-name
`BuiltinFuncTable` (`tclBasic.c:343–374`, registration at916–930). The runtime
installer consumes that roster. `min` and `max` are absent before library init;
no hook descriptor, procedure-header rule or Unknown refusal changes.
Other release installers retain their previous path; their constructor rosters
are unmeasured by this repair. The original C source is retained byte-exact in
[tclBasic.c.gz](tclBasic.c.gz) and [tclProc.c.gz](tclProc.c.gz).

[records.json](captured/records.json) retains ten fresh-process executions,
exact argv/input/binary/source hashes, exits and stream hashes. Every execution
exited0 without timeout. The C API probe creates an interpreter before any init;
a stock already-initialised shell cannot answer the constructor question.

| Control | Actual C8.6 | Current native C8.6 |
| --- | --- | --- |
| Sorted constructor inventory | 29 names, min/max absent | Identical sorted output |
| Empty bareargs min birth | Present hook; epochs17/0 unchanged | Real generation `ProcedureNoOp`; relative epochs unchanged |
| Authentic nonempty max definition | NULL hook; epochs17/0 unchanged | Real generation `Absent`; relative epochs unchanged |
| Replace actual empty min | Retired Present; compiler17→18, resolver0 | New generation `Absent`; compiler+1, resolver unchanged |
| Matching real init | `Tcl_Init`0; min/max NULL; epochs17/0 | `--init`0; exact `REACHED\n`, empty stderr |

The runtime result is attributable to the fresh native runner's real
`Interp::with_native_core`/`NativeHost` path, exact8.6 profile and
`init_library_completion`: it reads the hash-pinned original
`TCL_LIBRARY/init.tcl`, checks currency, then executes the unchanged startup
input. [build-records.json](build-records.json) pins the runner and source anchors.
This startup control succeeded; no tcltest or general conformance verdict follows.

[fixture-origin.json](fixture-origin.json) pins original init lines103–118.
[actual-init-max-definition.tcl](actual-init-max-definition.tcl) preserves tabs
and every original byte; the test wrapper adds only the actual namespace and
marker. Each birth shape uses a fresh native interpreter. The focused test
matched1/passed1, with zero residual objects/buffers, double-frees and OOM.
The first test compilation failed on a moved expected value; its exact source
and log remain retained, without a semantic pass claim.

Before/frozen/after owner snapshots and inventories are retained here.
The 4178 selected dependency source hashes matched frozen→after; the broad4939
Rust inventory changed only `rust/tcl-compiler/src/script_binds.rs`, which is
outside this runner's dependency graph. No whole-workspace byte identity claim.
[receipt-validation.json](receipt-validation.json) verifies the captured hashes,
fixture, inventory comparison, marker and replay parser checks.

Replay into a required NEW destination (never overwrite retained results):

```sh
python experiments/name-handling/results/native-c86-mathfunc-bootstrap/capture.py \
  --output-dir /tmp/name-mathfunc-bootstrap-replay-NEW \
  --runner /tmp/name-native-c86-mathfunc-bootstrap-runner
```

The replay builds an output-specific C probe. Its original captured binary was
preserved unchanged under `/tmp`; its executed argv and hash remain pinned.
Previous [mathfunc failure captures](../../clock-bootstrap-epoch/registration-repair/mathfunc-bootstrap/README.md)
and [foreach failure captures](../../clock-bootstrap-epoch/registration-repair/compiled-foreach/README.md)
remain unchanged. Clock deferred-map order, physical worker registration/hash
order, seven missing clock support workers, and VM's all-functions installer
remain open. Command-backing/rust-check/prep-pr and broader tests were not run
against this checkpoint; no waiver is invented.
