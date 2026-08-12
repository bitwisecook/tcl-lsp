# Tcl test tiers — the capability ladder

The Rust interpreters (`tcl-vm` bytecode VM, `runtime/rust` tree-walk runtime,
both native and wasm/WASI) are held to C Tcl 9's pass / fail / skip on the
upstream tcltest suite. This document is the **capability ladder** that
organises that suite: which upstream `.test` files belong to each tier, what
each tier means, and why one tier depends on another.

It is the *semantic* grouping behind the per-stem scoreboard in
[`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) — that file says *where* the
gaps are, this one says *why* they matter and in what order they are worth
fixing.

## Why the ordering matters

A lower tier is a prerequisite for every tier above it, so a defect low down
manifests as scattered failures higher up. A bug in parsing corrupts every test
above it; a missing socket only affects advanced I/O. Fixing the fundamentals
once — parsing, traces, encodings, channels — lifts everything that depends on
them, instead of the same root cause being patched per-symptom across ten
higher-tier files.

Parity is protected by a pass-only ratchet in the committed baselines
(`tests/baselines/tcl9-tcltest-vm/summary.json` and the WASM / runtime
equivalents): the regression gate only ever lets the pass count rise, so a
change that regresses a tier already at parity fails the gate.

## Core language vs optional features

The single most useful distinction the ladder encodes: a **core language**
versus a long tail of **optional features**.

- **Core language — Tiers 1–8.** What makes the thing Tcl: parsing,
  interpretation, the fundamental machinery (variables, namespaces, traces,
  aliases, option/ensemble dispatch, encodings, core channels, the `env`
  array), data types, control flow, the object system, package loading, and
  interpreters. Every backend must match C here, exactly — there is no "skip it"
  escape hatch, because a program (or the standard library that bootstraps it)
  cannot run without it.
- **Optional features — Tiers 9–11.** Capabilities layered on top. Some are
  **host/OS-dependent** (sockets, the event loop, subprocesses, dynamic
  loading, threads, clocks) and are legitimately absent on a sandboxed backend.
  Others are **"just a feature"** — a self-contained subsystem that could ship
  as a package and that no core program needs: compression (`zlib`/`zipfs`),
  HTTP (`http*`), message catalogues (`msgcat`), the option parser (`opt`), the
  registry. Parity on these matters for completeness but never blocks the core.

Rule of thumb when triaging: a failure in Tiers 1–8 gets fixed; a failure in
Tiers 9–11 first gets the question "can the running backend even do this?" — and
if not, the [backend-constraint overlay](backend-constraints.md) skips it.

## How to read the file lists

- Every file is `tmp/tcl9.0.3/tests/<name>.test`.
- A file (or a command group within it) that spans tiers is listed under
  **each** tier it touches, tagged with the group that puts it there. The three
  big "commands A–H / I–L / M–Z" files are the main offenders:
  - **`cmdAH`** — `after array append break case catch cd clock close concat
    continue encoding eof error eval exec exit expr fblocked fconfigure file
    fileevent flush for foreach format gets glob global` → spans fundamentals,
    data types, control flow, I/O, and platform.
  - **`cmdIL`** — `info lappend lassign lindex linsert list llength lrange
    lreplace lreverse lsearch lset lsort` → fundamentals (info) + data types.
  - **`cmdMZ`** — `regexp regsub return scan set split string subst switch time
    trace unset` → parsing (subst), fundamentals (set/unset, trace), data types
    (scan/string/split/regexp), control flow (return/switch).
- The same overlap rule applies to whole subsystems: the **channel** and
  **encoding** files exercise both their fundamental core (Tier 3) and their
  advanced surface (Tiers 9/11), so they appear in both.
- Platform-native files (`win*`, `mac*`/`macOSX*`, `unix*`) are listed in
  Tier 11 and are out of scope for the portable Rust runtimes except where the
  generic command underneath is tested.

## The ladder

### Tier 1 — Parsing  *(core)*

Lexer, word splitter, brace/quote/bracket nesting, backslash and
command/variable substitution, and the expression tokeniser. Everything above
depends on bytes → words → commands being right.

> `parse`, `parseOld`, `parseExpr`, `word`, `subst` · `cmdMZ` (subst group)

### Tier 2 — Interpretation  *(core)*

Turning parsed commands into execution: the bytecode compiler, the VM
execution loop, `eval`/`uplevel` re-entry, the non-recursive engine (NRE)
basics, and the object/representation model.

> `basic`, `compile`, `execute`, `eval`, `assemble`, `obj`, `nre`,
> `appendComp`, `lsetComp`, `regexpComp`, `compExpr`, `compExpr-old`, `misc`

### Tier 3 — Fundamental Tcl machinery  *(core)*

The state model and core services every command relies on — including three
subsystems that are *fundamental*, not platform extras: **option/ensemble
dispatch**, **encodings**, and the **core channel abstraction**.

- **Variable storage & management** — scalars, arrays, `upvar`/`uplevel`
  linkage, qualified vs local resolution, and the predefined globals
  (`tcl_platform`, **`env`**) the bootstrap installs.
  > `var`, `set`, `set-old`, `append`, `incr`, `incr-old`, `upvar`, `uplevel`,
  > `get`, `link`, `env` · `cmdMZ` (set/unset group)
- **Namespaces** — creation, qualified resolution,
  `import`/`forget`/`export`, ensembles, the namespace a command (incl. a
  renamed proc) runs in, custom resolvers.
  > `namespace`, `namespace-old`, `resolver`
- **Traces** — read / write / unset / array variable traces, command and
  execution traces (incl. firing from `info exists` and the `set` read path).
  > `trace` · `cmdMZ` (trace group)
- **Aliasing, rename & introspection** — `rename` (incl. across namespaces),
  and the `info` surface used to verify the machinery above.
  > `rename`, `info`, `cmdInfo` · `cmdIL` (info group)
- **Option & ensemble dispatch** — the built-in `Tcl_GetIndexFromObj`
  subcommand / option resolution (unambiguous-prefix matching, the
  `bad option …: must be …` wording) every ensemble relies on. (The pure-Tcl
  `::tcl::OptProc` *package* — `opt` — is a library feature, Tier 11.)
  > `indexObj` · every ensemble (`string`/`dict`/`namespace`/`chan`/`file`/
  > `clock`/`interp`) depends on this.
- **Encodings** — the Unicode core: `encoding convertfrom`/`convertto`, the
  internal UTF representation, and `\u`/`\U` handling that string and channel
  code assume.
  > `encoding`, `utf`, `utfext` · advanced/external encodings → Tier 11
- **Core channels** — the channel abstraction itself: `puts`/`gets`/`read`/
  `open`/`close`/`flush`/`eof` on the standard and in-memory channels — what
  `puts stdout` and the test harness's output capture rely on.
  > `chan`, `io` (core groups) · non-blocking / transforms / sockets / events →
  > Tier 9 · `cmdAH` (close/eof/flush/gets group)

### Tier 4 — Data types  *(core)*

Lists, dicts, strings, numeric/format/scan, regexp, and binary. Depend only on
Tiers 1–3 and are prerequisites for most higher-tier test bodies. Pure
computation — identical on every backend.

> Lists: `list`, `lindex`, `linsert`, `llength`, `lrange`, `lrepeat`,
> `lreplace`, `lsearch`, `lset`, `lsetComp`, `lmap`, `lpop`, `lseq`, `lreverse`,
> `listObj`, `listRep`, `abstractlist`, `range`
> · Dicts: `dict`
> · Strings: `string`, `stringObj`, `format`, `scan`, `split`, `join`, `concat`
> · Regexp: `regexp`, `regexpComp`, `reg`
> · Binary / misc: `binary`, `util`, `stack`, `assocd`, `dstring`
> · `cmdIL` (list-ops group), `cmdMZ` (scan/string/split/regexp group),
> `cmdAH` (format group)

### Tier 5 — Control flow & procedures  *(core)*

Conditionals, loops, exceptions, procedures, and expressions — the constructs
that drive a test body. Built on Tiers 1–4.

> Flow: `if`, `if-old`, `for`, `for-old`, `foreach`, `while`, `while-old`,
> `switch`, `error`, `result`
> · Expr: `expr`, `expr-old`, `mathop`
> · Procs: `proc`, `proc-old`, `apply`, `unknown`
> · `cmdMZ` (return/switch group),
> `cmdAH` (break/case/catch/continue/error/eval/for/foreach group)

### Tier 6 — Object system (TclOO)  *(core, mid)*

The standard object system — classes, objects, methods, `next`, mixins,
filters, and properties. Mid-tier: core Tcl (8.6+), built squarely on
procedures, namespaces, and ensemble dispatch (Tiers 3 & 5), but nothing below
it depends on it.

> `oo`, `ooNext2`, `ooProp`, `ooUtil`

### Tier 7 — Packages & code loading  *(core)*

Loading code at run time: `source`, the `package` database
(`provide`/`require`/`ifneeded`/`vsatisfies`/`vcompare`), Tcl modules (`tm`),
and the auto-load / index machinery. Core because the standard library — and
tcltest itself — bootstraps through it. The version/dependency logic is
pure computation; the auto-load path layers on `source`, which needs the
filesystem on backends that have one.

> `package`, `pkgMkIndex`, `autoMkindex`, `tm`, `config` · `source`
> (the loading primitive; FS surface shared with Tier 9)

### Tier 8 — Interpreters (child & safe)  *(core)*

Creating, evaluating in, and tearing down **child interpreters**, the
cross-interpreter **alias** and **hidden-command** machinery, and **safe
interpreters** (`interp create -safe`, `marktrusted`, the Safe Base). A child
interp re-enters the whole evaluator; a *safe* interp is precisely the
mechanism that gates a child's access to the optional-feature tiers above.
Depends on namespaces/aliases (Tier 3), control flow (Tier 5), and package
loading (Tier 7 — the Safe Base is loaded as a package).

- **Child interpreters** — `interp create`/`eval`/`delete`/`exists`/`children`,
  child-as-command dispatch, `recursionlimit`, `interp target`/`share`/
  `transfer`, `interp alias`/`aliases` across paths.
  > `interp`
- **Safe interpreters & the Safe Base** — `interp create -safe`, `issafe`,
  `marktrusted`, `hide`/`expose`/`hidden`/`invokehidden`, and the
  `safe.tcl` re-aliasing of `source`/`load`/`file`/`encoding`.
  > `safe`, `safe-stock`, `safe-stock86`, `safe-zipfs`, `security`

### Tier 9 — Advanced I/O & events  *(feature, host-dependent)*

The capabilities beyond the Tier 3 channel core: sockets, the event loop and
fileevents, channel transforms and stacking, the filesystem command surface,
and globbing. The first tier whose capabilities a backend can *lack* — WASI has
no sockets, eBPF has no I/O at all — so this is where the
[backend-constraint overlay](backend-constraints.md) starts skipping tests.

> Advanced channels: `ioCmd`, `ioTrans`, `iogt`, `chanio` · advanced groups of
> `io`/`chan`
> · Sockets: `socket`
> · Event loop: `event`, `async`, `timer`, `notify`, `fileevent`
> · Files / FS: `fCmd`, `fileName`, `fileSystem`, `fileSystemEncoding`, `pwd`,
> `link`
> · `cmdAH` (after/fblocked/fconfigure/file/fileevent/glob group)

### Tier 10 — Concurrency & advanced evaluation  *(feature, late)*

Constructs that suspend, resume, or parallelise execution. Late because they
sit on top of a correct evaluator, the event loop, and (for threads) the host.

> Coroutines: `coroutine`, `dcall` · deep NRE re-entry beyond Tier 2
> · Tailcall: `tailcall`
> · Threads: `thread`, `mutex`

### Tier 11 — Platform & library features  *(feature, late)*

The long tail. **Host/OS-dependent**: subprocesses, dynamic loading,
clocks/timezones, the advanced/external encoding surface. **Pure library
features** (each could be a package, none is needed by a core program): the
option parser, message catalogues, compression, networking protocols, the
registry. Heavily backend-gated — most are skipped on wasm / WASI / eBPF.

> Host/OS: `exec`, `process`, `pid`, `load`, `unload`, `clock`, `clock-ivm`,
> `icu`
> · Library features: `opt`, `msgcat`, `zipfs`, `zlib`, `http`, `http11`,
> `httpPipeline`, `httpProxy`, `httpcookie`, `registry`
> · Bootstrap / misc: `platform`, `init`, `main`, `history`, `aaa_exit`,
> `bigdata`, `brodnik`
> · Native-only (out of scope for the portable runtimes): `unixFCmd`,
> `unixFile`, `unixForkEvent`, `unixInit`, `unixNotfy`, `winConsole`,
> `winDde`, `winFCmd`, `winFile`, `winNotify`, `winPipe`, `winTime`,
> `macOSXFCmd`, `macOSXLoad`
> · `cmdAH` (cd/clock/encoding-system/exec/exit group)

## Feature checklist — areas that are easy to forget

Each capability area has at least one representative file; use it to confirm a
feature is exercised, not silently absent. The **kind** column is the
core-vs-feature lens.

| Area | Files | Tier | Kind |
|---|---|---|---|
| Built-in option / ensemble dispatch | `indexObj` | 3 | core |
| Encodings / Unicode core | `encoding`, `utf`, `utfext` | 3 | core |
| Core channels | `chan`, `io` (core) | 3 | core |
| `env` array | `env` | 3 | core |
| Namespace ensembles | `namespace` (ensemble group) | 3 | core |
| Custom resolvers | `resolver` | 3 | core |
| TclOO | `oo`, `ooNext2`, `ooProp`, `ooUtil` | 6 | core (mid) |
| Packages / modules / loading | `package`, `pkgMkIndex`, `autoMkindex`, `tm`, `source` | 7 | core |
| Child / safe interpreters | `interp`, `safe`, `safe-*`, `security` | 8 | core |
| Channel transforms / stacking | `iogt`, `ioTrans` | 9 | feature (host) |
| Event loop / async | `event`, `async`, `timer`, `notify`, `fileevent` | 9 | feature (host) |
| Coroutines / NRE | `coroutine`, `dcall`, `nre` | 10 (2 for NRE basics) | feature |
| Threads | `thread`, `mutex` | 10 | feature (host) |
| Subprocess / dynamic load | `exec`, `process`, `load`, `unload` | 11 | feature (host) |
| Option parser package | `opt` | 11 | feature (library) |
| Compression / archives | `zlib`, `zipfs` | 11 | feature (library) |
| HTTP protocol | `http`, `http11`, `httpPipeline`, `httpProxy`, `httpcookie` | 11 | feature (library) |
| Message catalogues | `msgcat` | 11 | feature (library) |
| Abstract / big lists | `abstractlist`, `range`, `bigdata` | 4, 11 | core / feature |

## How to use the ladder

- **Fix bottom-up.** A Tier 1 parser bug or a Tier 3 trace/encoding/channel bug
  shows up as scattered failures across many higher-tier stems; fixing it lifts
  all of them at once (firing read traces from `info exists` unblocked lazy
  tcltest constraints across the whole suite). Chasing a Tier 5 failure while a
  Tier 3 fundamental is broken is usually wasted work.
- **A CRASH on a low tier is the highest-leverage fix** — it zeroes a whole
  file, so one fix converts a `0` into whatever the file would otherwise
  score. The scoreboard marks these `CRASH`; the multi-tier `cmd*` files are
  the ones to watch, because a single abort there costs coverage across
  fundamentals, data types, control flow, and platform at once.
- **Use the scoreboard for "where", the ladder for "why".** Within a tier,
  prefer the stem whose failures share a single root cause.
- **Tier 9–11 parity is per-backend.** On native, match C exactly; on
  wasm / WASI / eBPF the honest target is "skip what you can't do", which the
  [backend-constraint overlay](backend-constraints.md) handles. Tier 8 safe
  interpreters are the in-language counterpart: a safe child legitimately
  *cannot* reach Tiers 9–11, so those tests are expected to fail closed.

## Cross-references

- [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) — live per-stem scoreboard.
- [`backend-constraints.md`](backend-constraints.md) — per-backend skip overlay
  and `tcl_platform` introspection.
- [`child-interp.md`](child-interp.md) — child-interpreter design (Tier 8).
