# Tcltest extension — port status

> **Audience:** Maintainer, contributor.
> **Type:** Triage matrix + porting roadmap.
> **Companion to:** [`wasm-extensions.md`](wasm-extensions.md).

This is the working triage of upstream Tcl 9 `tcltest` C-tier
`test*` commands as we port them to `runtime/zig/tcltest/`.  It's
both a roadmap (what's left) and a contract (which sub-commands
can never be portable, and why).

Source files (Tcl 9.0.3, under `tmp/tcl9.0.3/generic/`):

| File                       | Lines | Init function           | Commands |
|----------------------------|-------|--------------------------|----------|
| `tclTest.c`                | 8980  | `Tcltest_Init`           | ~94 `test*` |
| `tclTestObj.c`             | 1865  | `TclObjTest_Init`        | 9 obj-type test cmds |
| `tclTestProcBodyObj.c`     | 354   | `Procbodytest_Init`      | 2 (`proc`, `check`) under `tcl::procbodytest` |
| `tclTestABSList.c`         | 1257  | `Tcl_ABSListTest_Init`   | 2 (`lstring`, `lgen`) abstract-list demo |

## Classification

Each command (and where relevant, each sub-command) gets one of:

| Class              | Meaning                                                                            |
|--------------------|------------------------------------------------------------------------------------|
| **PORTABLE**       | Tests observable Tcl-level behaviour we already implement.  Goes into Zig.         |
| **PARTIAL**        | Some sub-commands portable, others probe internal C struct layout — those trap.    |
| **NOT-PORTABLE**   | Tests C-only state we deliberately don't replicate (refcount of an internal\_rep, native fork/threads/sockets, hardware FS probes).  Command absent from the Zig port. |

NOT-PORTABLE ≠ "we don't care" — it's a deliberate decision that the
test-suite rows depending on those commands need an alternate
strategy (skip on WASM, port to a Tcl-level equivalent, or rely
on the Zig unit tests for the same invariant).

## Status by command

### `tclTestObj.c` — object-type tests

| Command              | Class       | Status (PR-1) | Zig file              | Notes |
|----------------------|-------------|---------------|------------------------|-------|
| `testintobj`         | PORTABLE    | partial       | `tcltest/cmd_obj.zig` | `set` / `set2` / `get` / `mult10` / `div10` ported.  Remaining sub-commands (`setint`, `setmax`, `ismax`, `ismin`, `bug3598580`) follow in PR-2. |
| `testbooleanobj`     | PORTABLE    | partial       | `tcltest/cmd_obj.zig` | `set` / `get` / `not` ported. |
| `testdoubleobj`      | PORTABLE    | partial       | `tcltest/cmd_obj.zig` | `set` / `get` / `mult10` ported. |
| `testbignumobj`      | PORTABLE    | not started   | —                      | Pure-compute; portable in PR-2. |
| `testindexobj`       | PORTABLE    | not started   | —                      | Drives `Tcl_GetIndexFromObj` — straight port. |
| `testlistobj`        | PARTIAL     | not started   | —                      | List ops portable; sub-commands probing the C `List *` internal\_rep are NOT-PORTABLE. |
| `testdictobj`        | PORTABLE    | not started   | —                      | Pure-compute. |
| `teststringobj`      | PARTIAL     | not started   | —                      | `set` / `get` / `length` / `append` / `appendstrings` portable; `getunicode` and the `String *` internal-struct probes are NOT-PORTABLE. |
| `testbigdata`        | NOT-PORTABLE | n/a          | —                      | Allocates >2 GiB; impossible on wasm32. |

### `tclTest.c` — interpreter / parser / runtime tests

PR-1 ports none of these yet; the cells below are the planned
classification we'll fill in PR-2 onward.

| Command (representative slice) | Class | Reason |
|-------------------------------|-------|--------|
| `testparser`, `testexprparser`, `testparsevarname`, `testparsevar`, `testsubparse` | PORTABLE | Drive our parser; emit upstream's flat-list token form. |
| `testevalex`, `testevalobjv`, `testreturn`, `testseterr`, `testwrongnumargs`, `testsetobjerrorcode` | PORTABLE | Pure interpreter behaviour. |
| `testencoding`, `testtranslatefilename`, `testdstring`, `testbytestring`, `testpurebytesobj` | PORTABLE | Value-tier round-trips. |
| `testdcall`, `testdel`, `testlink`, `testlinkarray`, `testpanic`, `testlongsize`, `testdoubledigits`, `testcmdtoken`, `testcmdtrace` | PORTABLE | Pure C state machinery without OS dependence. |
| `testchannel`, `testchannelevent`, `testopenfilechannel` | PARTIAL | Value formatting + permission ops portable; native-FS probes NOT-PORTABLE. |
| `testfilesystem`, `testfile`, `testfilelink` | PARTIAL | Same shape as channel — most subops portable, link traversal / mount-point checks NOT-PORTABLE. |
| `testasync`, `testevent`, `testservicemode`, `testinterpdelete`, `testinterpresolver` | PARTIAL | Synchronous variants portable; threaded / async branches NOT-PORTABLE. |
| `testsocket`, `testthread`, `testmutex`, `testmainloop`, `testexithandler`, `testexithandler-thread`, `testNREfornonNREcmd`, `testfork`, `testbumperror` | NOT-PORTABLE | Threading, OS sockets, fork — none available under WASI. |

### `tclTestProcBodyObj.c` + `tclTestABSList.c`

| Command                            | Class    | Status |
|------------------------------------|----------|--------|
| `tcl::procbodytest::proc`          | PORTABLE | not started |
| `tcl::procbodytest::check`         | PORTABLE | not started |
| `lstring`                          | PORTABLE | not started |
| `lgen`                             | PORTABLE | not started |

## File layout under `runtime/zig/tcltest/`

PR-1 establishes the layout; subsequent PRs add cmd files in
priority order.

```
runtime/zig/tcltest/
├── slots.zig            # per-extension Tcl_Obj* slot table (refcount-safe)
├── cmd_obj.zig          # testintobj / testbooleanobj / testdoubleobj — landed PR-1
├── cmd_listobj.zig      # testlistobj, lstring, lgen — PR-N
├── cmd_dictobj.zig      # testdictobj — PR-N
├── cmd_stringobj.zig    # teststringobj — PR-N
├── cmd_parser.zig       # all parser-test cmds — PR-N
├── cmd_dstring.zig      # testdstring + helpers — PR-N
├── cmd_encoding.zig     # testencoding + testtranslatefilename — PR-N
├── cmd_eval.zig         # testevalex / testseterr / testreturn / testwrongnumargs — PR-N
├── cmd_proc.zig         # tcl::procbodytest — PR-N
├── cmd_chan.zig         # testchannel + friends (PARTIAL) — PR-N
├── cmd_fs.zig           # testfilesystem / testfile / testfilelink (PARTIAL) — PR-N
├── cmd_async.zig        # testasync / testevent (PARTIAL) — PR-N
├── cmd_link.zig         # testlink / testlinkarray — PR-N
└── cmd_misc.zig         # everything else uncategorised — PR-N
```

Each file follows the same shape as `runtime/zig/cmds/*.zig`:
exported `pub const registrations: [N]reg.CmdEntry`, internal
helpers private to the file.

## Suggested PR sequencing

1. **PR-1 (this branch)** — link infrastructure + scaffolding
   + minimal `cmd_obj.zig`.  Done.
2. **PR-2** — finish `cmd_obj.zig` (full sub-command coverage of
   testintobj/testbooleanobj/testdoubleobj/testbignumobj/testindexobj).
3. **PR-3** — `cmd_listobj.zig` + abstract-list (`lstring`, `lgen`).
4. **PR-4** — `cmd_stringobj.zig` + `cmd_dictobj.zig`.
5. **PR-5** — `cmd_parser.zig`.
6. **PR-6** — `cmd_eval.zig` + `cmd_misc.zig` (the small interpreter-
   plumbing commands).
7. **PR-7** — `cmd_encoding.zig` + `cmd_dstring.zig`.
8. **PR-8** — `cmd_proc.zig`.
9. **PR-9** — `cmd_chan.zig` + `cmd_fs.zig` (PARTIAL).
10. **PR-10** — `cmd_async.zig` + `cmd_link.zig` (PARTIAL).
11. **Final** — switch the upstream tcltest test corpus harness on
    by default in `make test-slow`.
