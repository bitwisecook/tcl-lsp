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
for exact semantics) **and the former Zig runtime** (`runtime/zig/`, the retired
behavioural oracle — see the [discoveries appendix](#appendix--zig-runtime-discoveries-to-honour))
to inform it, implement it as a Rust builtin, and re-drive. Empirical loop:
`source` the file → hit the first wall → port that command → repeat. (The Zig
runtime has since been deleted; its oracle discoveries are preserved in the
appendix and its citations point into git history.)

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
- **M2 — VFS + channels (L2) — ✅ done.** Host file/VFS layer via `std::fs`
  (native / `wasm32-wasip1`; a non-WASI shim can swap in): `source`/`file`/
  `glob`/`pwd`/`cd` (`cmd_fs.rs`) + channels `open`/`close`/`read`/`gets`/`puts`/
  `flush`/`eof`/`seek`/`tell`/`fconfigure`/`fblocked` (`cmd_chan.rs`). Plus the
  `Tcl_Init` bootstrap (`Interp::init_library`: `$TCL_LIBRARY` → startup globals
  + `auto_path` → `source init.tcl`), the `unknown` auto-load hook in `dispatch`,
  `package require` via `ifneeded`/`pkgIndex`, and `return -code/-options`. Gate:
  the **unmodified Tcl 9 `init.tcl` loads cleanly**, and `package require tcltest`
  drives the real auto-load chain (finds `tcltest/pkgIndex.tcl`, runs `ifneeded`,
  sources `tcltest.tcl`) — reaching `regsub` (the regex/L3 boundary, M3).
- **M3 — `package require tcltest` — ✅ done.** The package-unknown →
  `pkgIndex.tcl` search → `source tcltest.tcl` chain now runs to completion:
  `package require tcltest` returns `2.5.10` and `test`/`cleanupTests` run real
  test bodies with byte-identical pass/fail reporting vs `tclsh9.0`. The wall was
  the **regex engine** plus a layer of L1/L3 commands the library assumes; what
  landed:
  - **`regexp`/`regsub`** on the **real Tcl 9 Henry-Spencer ARE engine** — the
    same `regcomp.c`/`regexec.c` (+`#include`d `regc_*.c`/`rege_dfa.c`) `tclsh`
    uses, compiled to a static archive by `build.rs` (gated `have_regex`, like
    `have_tommath`) and FFI'd via `src/regex.rs`. The C host hooks (heap, ASCII
    char-class predicates, `Tcl_UniChar*` case/encode + `Tcl_DString`) live in
    `regex_shim/` (`regcustom.h`/`tclInt.h`/`regex_shim.c`), adapted from the
    former Zig runtime's `regex_include/` (the oracle) and retargeted from
    wasm32 to the native host. The Spencer workspace `struct vars` is
    `_Thread_local` (vs the former Zig file-static) so the multi-threaded
    `cargo test` runner is race-free.
    `cmd_regex.rs` mirrors `tclCmdMZ.c` (`-all`/`-inline`/`-indices`/`-nocase`/
    `-line`/`-start`/`--`; `&`/`\N` substitution; UTF-8↔codepoint offset
    mapping). ASCII-only Unicode classes for now (a mechanical follow-up).
  - **`try`/`throw`** (TIP 329: `on code`/`trap pattern`/`finally`),
    **`subst`** (`-no*` flags, error-propagating), **`trace add|remove|info
    variable`** (read/write/unset firing from the var chokepoints; a re-entrancy
    guard; matches by name).
  - `string match`/`map`/`is` (the shared `tcl_syntax::glob` engine);
    `namespace code`/`origin`; `info level N`/`info complete`/`info
    sharedlibextension`; `unset -nocomplain`/`--`; `source -nopkg`; `file`
    ensemble unambiguous-prefix resolution (`file isdir`→`isdirectory`).
  - **Frame-model fix:** `namespace eval`/`inscope` now push a **namespace
    frame** (`Frame.is_proc=false`), so unqualified `set`/`variable`/`upvar 0`
    inside a `namespace eval` nested in a proc target the namespace, not the
    enclosing proc's locals; and relative `upvar 0` at namespace scope aliases a
    namespace var/element (tcltest's option/accessor machinery — e.g.
    `upvar 0 Option(-debug) debug`).
- **M4 — run real compute-only `*.test` files — ✅ first suites green.** The
  unmodified C-Tcl-9 list/dict suites run on the runtime with high pass rates:
  `list.test` **78/78**, `split.test` **18/18**, `linsert` **28/28**,
  `llength`/`concat` 100%, `lrange` 1759/1766, `dict.test` **272/373** (was
  136), `lindex` 42/84 (+37 skipped for the C-only `testevalex`), `join` 9/10.
  What landed for M4:
  - **`scan`** (`cmd_scan.rs`) — `%d`/`%i`/`%u`/`%o`/`%x`/`%b`/`%c`/`%s`/`%e`/
    `%f`/`%g`/`%[...]`/`%n`/`%%` with `*`/width/size-modifiers, inline + var
    modes, codepoint-based (`tclScan.c`).
  - **`format`** (`cmd_format.rs`) — the `sprintf` analogue: flags
    (`-`/`+`/space/`0`/`#`), width + precision (literal or `*`), positional
    `%N$`, all numeric/string/float conversions (`tclStringObj.c`).
  - **List-element quoting** rewritten to the faithful COMPAT
    `TclScanElement`/`TclConvertElement` four-mode algorithm (none / brace /
    mask / escape), fixing `[`/`$`/`;`/`]`/`"`/leading-`#` cases — the single
    biggest pass-rate lift (`list.test` 65→78).
  - **`split`** made codepoint-based (was byte-based — broke multi-byte
    separators / empty-split on non-ASCII); **`lindex`** extended to a full
    index *path* (`lindex $l 1 2` / `lindex $l {1 0}` nested indexing).
  - **`dict`** expanded with `replace`/`remove`/`getdef`/`filter`/`map`/
    `update`/`with`, multi-key `get`/`exists` paths, and glob-filtered
    `keys`/`values` (`dict.test` 136→272).
  - **Index arithmetic** in `index_spec` — the full `TclGetIntForIndex`
    grammar (`0-1`, `-2+1`, `end--1`); `linsert` resolves `end` to the list
    length (insertion point).
  - **`expr` array-index substitution** (`expr {$a($k)}` now substitutes `$k`)
    — also unblocked tcltest's constraint evaluation.
  - Generic errors now stamp `::errorInfo` (the message) + `::errorCode`
    (`TCL WRONGARGS` for wrong-args, else `NONE`).
  - **Frame model:** logical call level decoupled from the stack index, so a
    proc invoked under `uplevel` (tcltest's `uplevel 1 [list Eval …]`) gets the
    right level — fixed `$errorCode`-style resolution across the test harness.
  - **Known remaining gaps** (deferred): the full `-errorcode` taxonomy (only
    `TCL WRONGARGS` so far), the incremental `errorInfo` source-trace, the
    Tcl-9 lone-surrogate (`\uD83D`) encoding (`append.test`), and C-only test/
    introspection commands (`testevalex`, `tcl::unsupported::representation`,
    `ledit`).
- **M5 — basic-suite fidelity pass.** Driving the unmodified `*.test` files
  through the interpreter directly (`cargo run --example run_script -- --init
  <file>`, `TCL_LIBRARY` → real `init.tcl`/`tcltest.tcl`) and fixing the first
  walls on the core suites:
  - **No more crashes on deep recursion.** The recursive tree-walker uses native
    stack per Tcl call level, so the 1000-deep `interp recursionlimit` (a
    *catchable* error) could not fire on the default 8 MiB main stack — deep
    recursion overflowed and aborted the process, zeroing a whole file's results
    (`error-1.8`, the intentional infinite-recursion case). The dev driver now
    runs eval on a 512 MiB-stack worker thread (the interp is `Rc`, single-thread)
    like `tclsh`; deep recursion is now caught (`too many nested evaluations`).
    `error.test` 0 (crash) → **309/309**.
  - **`if` grammar** rewritten to C's `Tcl_IfObjCmd`: validate the whole
    `?then?`/`elseif`/`else` structure before executing the matched branch, with
    faithful `no expression after`/`no script following`/`extra words after
    "else" clause` errors. `if.test` 59 → **69/73**.
  - **`array set a {}`** materialises an empty array; an existing scalar accessed
    with an index reports `variable isn't array`; `trace add|remove|info`
    resolves its type word by unambiguous prefix (`var`→`variable`). `set.test`
    53 → **59/64**.
  - **`while`/`for` errorInfo frames + for-step break.** The interpreted
    `Tcl_WhileObjCmd`/`Tcl_ForObjCmd` append `("while" body line N)` /
    `("for" body line N)` on a body error (outside a proc, like `foreach`), plus
    the no-line `("for" initial command)` / `("for" loop-end command)` frames for
    the init/next scripts (new `Interp::append_frame_noline`); a `break` in the
    for **step** clause now ends the loop cleanly (C `case TCL_BREAK: result =
    TCL_OK`). `while.test` 45 → **46/46**, `for.test` 52 → **63/88**.
  - Other basic suites run with high pass rates on this path: `list` 78/78,
    `eval` 12/12, `unknown` 7/7, `error` 309/309, `while` 46/46, `switch`
    112/113, `string` 681/705, `foreach` 39/43, `incr` 61/69, `var` 175/219. The
    dominant remaining failure mode across files is the deferred incremental
    `errorInfo` source-trace on non-control-flow paths and the trace/upvar detail
    suites; next is the list-element-quoting / `"word"junk`
    (`extra characters after close-quote`) parse-error fidelity in the runtime's
    parser.

## Appendix — Zig-runtime discoveries to honour

Mined from the **former Zig runtime** (`runtime/zig/`, the retired behavioural
oracle — now deleted; the `*.zig` line citations below point into its git
history). Each is a concrete, testable semantic the Rust port must match;
**bold** = a guard against a real bug the Zig runtime hit. Rust status noted
where already handled.

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
- C Tcl 9 source: `tmp/tcl9.0.3/generic/*.c` (exact semantics); the former Zig
  runtime `runtime/zig/` remains the historical oracle (git history only);
  `make runtime-rust-test` / the Tcl 9 tcltest sweep (no regression of a file
  the runtime already passes).
