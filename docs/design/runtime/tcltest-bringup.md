# Running the real Tcl library — `tcltest` bring-up plan

**Goal.** Run the **unmodified** pure-Tcl standard library (`tmp/tcl9.0.3/library/init.tcl`,
`tcltest/tcltest.tcl`, the `pkgIndex.tcl`/`tm` machinery) and then **real C Tcl 9
`*.test` files** on the Rust runtime interpreter (`runtime/rust/`). We do **not**
re-port any pure-Tcl library code to Rust — we port the **C command surface** the
library assumes (the commands `tclsh` registers in C before sourcing `init.tcl`),
so the Tcl-language library runs on top. This is the north-star "the AOT compiler
is always half of a pair — a complete runtime parser+evaluator" made concrete.

**Method (the one the work follows).** Reason over portions of `init.tcl` /
`tcltest.tcl`, find the missing bit, **reference C Tcl** (`tmp/tcl9.0.3/generic/*.c`
for exact semantics) **and the Zig runtime** (`runtime/zig/`, the behavioural
oracle — see the [discoveries appendix](#appendix--zig-runtime-discoveries-to-honour))
to inform it, implement it as a Rust builtin, and re-drive. Empirical loop:
`source` the file → hit the first wall → port that command → repeat.

## What "C command" means here (the surface to port)

C Tcl registers its built-ins in C (`tclBasic.c` `BuiltInCmds[]` + the per-subsystem
`Tcl*InitObjCmd`s). Everything else in the library is pure Tcl that will just run.
Cross-referencing `BuiltInCmds[]` + the subsystem registrations against what the
runtime has today (`set expr if while for foreach proc puts incr append return
unset break continue list lindex llength lrange lreverse lassign concat join split
string dict namespace rename interp upvar variable global` + `::tcl::mathfunc/mathop`
+ ensembles), the **missing C commands** group into dependency layers:

| Layer | Missing C commands | C Tcl ref | Needs host/OS? |
|---|---|---|---|
| **L0 — fixes to existing** | (no new cmds — correctness fixes from the Zig oracle) | — | no |
| **L1 — eval + exception + introspection core** | `eval` `uplevel` `apply` `subst`(cmd) `catch` `error` `throw` `try` `return -options` `switch` `info` `array` `package` `lsort` `lsearch` `linsert` `lreplace` `lrepeat` `lset` `lmap` `lpop` `lremove` `lseq` | `tclBasic.c`, `tclProc.c`, `tclResult.c`, `tclCmdAH.c` (`catch`/`error`/`for`/`foreach`), `tclCmdIL.c` (`info`/`lsort`/`lsearch`/`linsert`/…), `tclCmdMZ.c` (`switch`/`subst`/`try`/`throw`/`trace`), `tclVar.c` (`array`), `tclPkg.c` (`package`), `tclNamesp.c` (`apply` is `tclProc.c`) | **no** (pure compute) |
| **L2 — filesystem + channels** | `source` `file` `glob` `pwd` `cd` `open` `close` `read` `gets` `flush` `seek` `tell` `eof` `fconfigure` `fblocked` `fcopy` | `tclIOUtil.c`/`tclFileName.c`/`tclFCmd.c` (file/glob), `tclIO.c`/`tclIOCmd.c` (channels) | **yes** — a VFS/host layer (WASI preview1) |
| **L3 — misc / host** | `clock` `encoding` `format` `scan` `regexp` `regsub` `exec` `after` `update` `vwait` `time` `exit` `pid` `const` `fpclassify` `load` `unload` `socket` `fileevent` `timerate` | `tclClock.c`/`tclEncoding.c`/`tclStringObj.c`(format)/`tclScan.c`/`tclRegexp.c`/`tclExecCmd.c`/… | **mostly yes** |

`info`, `array`, `string`, `dict`, `namespace`, `clock`, `file`, `encoding`,
`binary` are **not** in `BuiltInCmds[]` — they are registered by their own
subsystem init (e.g. `TclInitInfoCmd`, `TclInitArrayCmd`) and several are
**ensembles** (`string`/`dict`/`array`/`file`/`clock`/`namespace`); our ensemble
machinery (T1.5) can host the ensemble ones, but each subcommand is still C work.

## Reasoning over the actual library code

### `init.tcl` top level (lines 18–160) — what it executes at load
`package require -exact tcl 9.0.3`; `info exists`; `interp issafe`; `apply`;
`lmap`; `catch`; `file tildeexpand`/`dirname`/`join`; `namespace eval`/`variable`/
`foreach`/`lappend`; `info nameofexecutable`; `encoding dirs`; `unset`;
`package unknown {…}`; `$tcl_platform(os|platform)` (array global); `dict
create/set`; `namespace inscope`; `namespace ensemble create`;
`::tcl::unsupported::clock::configure -init-complete`; `namespace which -command`;
`proc`; `puts stderr`. **So even *sourcing* `init.tcl` needs L1 (`apply`/`lmap`/
`info`/`catch`/`package`) + L2 (`file`) + L3 (`encoding`/`clock`).** The bulk
below line 160 is `proc` definitions (`unknown`, `auto_load*`, `auto_qualify`,
`auto_import`, `tcl::Pkg::source`, `CopyDirectory`, …) that only run when called.

### `tcltest.tcl` top level (lines 19–60, 3340–3588)
`namespace eval tcltest`; `package require Tcl 8.5-`; `package vsatisfies
[package provide Tcl] 9.0-`; `variable`; a long `Configure`/`Option` option
system; `namespace import`/`forget`/`origin`; `package provide`. Running a
**test** then drives the deep surface: `catch`/`uplevel`/`info`/`string`/`file`/
`glob`/`regexp`/`format`/`clock`/`array`/`switch`/`error`/`return -options`/`open`/
`read` (output capture). So L1 is needed to *load* tcltest and L1+L2+L3 to *run* a
test.

## Milestone ladder (each gated; bottom of the loop is a real `.test` file)

- **M0 — L0 correctness fixes** (done as found): `break`/`continue` escaping a
  proc → `invoked "break"/"continue" outside of a loop` (Zig discovery 4/9) — **done**.
- **M1 — eval/exception/introspection core (L1)**: Gate: leak-checked unit
  tests per command + the cross-scope cases vs tclsh. PC-3 + PC-4 of
  [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md) plus the
  introspection/list/package builtins.
  - **done:** `catch`/`error` (+ the `-options` dict, `::errorInfo`/`::errorCode`
    stamping); `eval`/`uplevel` (+ active-level/varFramePtr + per-frame ns
    restore)/`apply` (shared `run_proc`); `info` (`exists`[arr(key)]/`commands`/
    `procs`/`vars`/`globals`/`locals`/`level`/`tclversion`/`patchlevel`/`body`/
    `args`/`default`); `array` (`set`/`get`/`names`/`exists`/`size`/`unset`).
  - **also done:** `switch` (-exact/-glob/-nocase/--, default, `-` fall-through);
    list ops `lrepeat`/`linsert`/`lreplace`/`lsearch`/`lsort`; `package`
    (provide/require/present/ifneeded/unknown/names/versions/vsatisfies/vcompare,
    with the core `tcl`/`Tcl` pre-provided + TIP-268 version reqs).
  - **next (closing L1):** `subst` (cmd); `lset`/`lmap`/`lpop`/`lremove`/`lseq`;
    `-regexp` modes (with the regex engine); the incremental `errorInfo`
    source-trace (`return -options`/`try`/`throw` + PC-1's `CmdFrame` stack).
    Then **L2** (VFS + `source`/`file`/`glob`/channels) to load `init.tcl`.
- **M2 — VFS + channels (L2)**: a host file/VFS layer (WASI preview1 native; a
  shim under test) behind `source`/`file`/`glob`/`open`/`read`/`gets`/`close`/
  `fconfigure`. Gate: `source init.tcl` loads cleanly; `info script` correct.
- **M3 — `package require tcltest`**: the package-unknown → `pkgIndex.tcl` search
  (needs `glob`+`file`+`source`) → `source tcltest.tcl`. Gate: tcltest's
  namespace + `test`/`Configure` defined; `test` with a trivial body runs.
- **M4 — run a real `*.test` file**: pick a compute-only suite first
  (e.g. `expr.test` / `string.test` slices), add L3 commands (`format`/`scan`/
  `regexp`/`clock`/`encoding`) as the chosen suites demand, gated by the
  pass-delta vs the Zig baseline (no regression of a file the Zig runtime passes).

## Appendix — Zig-runtime discoveries to honour

Mined from `runtime/zig/` (the behavioural oracle). Each is a concrete,
testable semantic the Rust port must match; **bold** = a guard against a real bug
the Zig runtime hit. Rust status noted where already handled.

**Procs / call:**
- **Body-level `break`/`continue` that escapes a proc → error** (`tcl_interp.zig`
  ~1342) — **done** (`call_proc`). `return` → Ok (done). 
- `args` tail collects excess args as a **properly list-quoted** Tcl list, empty
  list when none — done (Rust `new_list_obj`, which quotes via the list string rep).
- Param/​body objects must **own** their bytes (Zig `ensure_owned` + retain,
  `tcl_procs.zig` 271–368) — N/A in Rust: `ProcDef` stores owned `Vec<u8>`.
- `info level 0` must report the **invoked** word for renamed/imported/aliased
  procs (`pending_argv0`, `tcl_interp.zig` 1173) — to honour when `info level`
  lands (M1): carry `argv` on the call frame.

**Control flow:**
- Loop result **leak avoidance** (release prev iteration result before
  overwriting; issue #303, `tcl_interp.zig` 594) — N/A in Rust (`set_result`
  retains/releases; each iteration's result object is balanced).
- `foreach` decodes **unbraced** elements' backslashes, copies braced verbatim —
  Rust splits via `parse::split_list` (the shared `TclFindElement`), which
  already applies the list-element rules; verify against `foreach x "a\\nb"`.

**eval / uplevel / upvar:**
- **`uplevel` must restore the caller's namespace AND frame depth together**
  (`tcl_frames.zig` 486–530; the frame records the *caller's* ns) — design `eval`/
  `uplevel` (M1) so the upleveled body runs in the target frame's `(level, ns)`;
  tcltest's `RunTest` uplevels test bodies and reads `$::tcltest::…` arrays.
- `upvar` level forms: `#N` absolute (0 = global), `N` relative (default 1),
  relative 0 = current frame — **done** in `cmd_var::upvar` (`#N`/`N`); verify
  the relative-0 alias.
- The local name may differ from the target name; alias resolves **by name at
  access time** — Rust `Var::Link{home,name,elem}` already does this.

**catch / error / return:**
- **`catch` must read the body's completion code BEFORE clearing it** and write
  the result var with an **empty string (never an unset sentinel)** on a 0/empty
  result (`tcl_catch.zig` 98–126; issues #280) — bake into the `catch` impl (M1):
  snapshot code, then write `result`/`options` vars.
- **Every error stamps `::errorInfo` + `::errorCode`** (`"NONE"` default), in and
  out of `catch` (`tcl_catch.zig` 215–226) — implement with the `ExceptionState`
  (PC-4); `catch -options` reads them back.
- Nested `catch` auto-absorbs the inner error (depth counter) — the Rust model
  is structural (a `catch` builtin consumes the `Code::Error` of its body), so
  this falls out; verify catch-in-catch.

**variables:**
- **`info exists arr(key)` / array-element names must split `arr(key)`** before
  lookup (`tcl_frames.zig` 994; tcltest's `testConstraints(unix)`) — Rust
  `split_array_ref` exists; wire it through `info exists` + `array` (M1).
- Unqualified resolution order (frame → current-ns → global) and `::`-absolute
  bypassing the frame — **done** (the `vars.rs` classifier).

**dispatch:**
- Alias/child-interp/coroutine command kinds route **before** the generic proc
  path — Rust models these as distinct `Command` variants, so the `match` in
  `invoke` handles it structurally (done for `Alias`/`Imported`/`Ensemble`).

## Cross-references
- [`rust-runtime-port.md`](rust-runtime-port.md) — Track 1 status + gates.
- [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md) — PC-3 (eval/
  uplevel) + PC-4 (catch/error/return-options/errorInfo) drive M1.
- C Tcl 9 source: `tmp/tcl9.0.3/generic/*.c` (exact semantics); the Zig runtime
  `runtime/zig/` (oracle); `make check-wasm-parity` / the Tcl 9 tcltest sweep
  (no regression of a file the Zig baseline passes).
