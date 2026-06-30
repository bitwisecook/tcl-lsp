# Tcl test tiers — the capability ladder to C parity

The goal is for the Rust interpreters (`tcl-vm` bytecode VM, `runtime/rust`
tree-walk runtime — both native and the wasm/WASI build) to match C Tcl 9's
pass / fail / skip on the upstream tcltest suite. This document orders that
work as a **capability ladder**: a lower tier is a prerequisite for the tiers
above it, so fixing bottom-up is the highest-leverage order. A bug in parsing
corrupts every test above it; a missing socket only affects I/O.

The live per-stem scoreboard (C P/S/F vs VM P/S/F, MATCH / gap / CRASH) lives
in [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md). This document is the
*semantic* grouping behind it — what each tier means, which **exact upstream
`.test` files** belong to it, and why the order matters.

## How to read the file lists

- Every file is `tmp/tcl9.0.3/tests/<name>.test`.
- A file that spans tiers is listed under **each** tier it touches, tagged with
  the *command group* that puts it there. The three big "commands A–H / I–L /
  M–Z" files are the main offenders:
  - **`cmdAH`** — `after array append break case catch cd clock close concat
    continue encoding eof error eval exec exit expr fblocked fconfigure file
    fileevent flush for foreach format gets glob global` → spans Tiers 3.5, 4,
    6, and 7.
  - **`cmdIL`** — `info lappend lassign lindex linsert list llength lrange
    lreplace lreverse lsearch lset lsort` → spans Tiers 3 and 3.5.
  - **`cmdMZ`** — `regexp regsub return scan set split string subst switch time
    trace unset` → spans Tiers 1, 3, 3.5, and 4.
- Platform-native files (`win*`, `mac*`/`macOSX*`, `unix*`) are listed in
  Tier 7 and are out of scope for the portable Rust runtimes except where the
  generic command underneath is tested.

## The ladder

### Tier 1 — Parsing

Lexer, word splitter, brace/quote/bracket nesting, backslash and
command/variable substitution, and the expression tokeniser. Everything above
depends on bytes → words → commands being right.

> `parse`, `parseOld`, `parseExpr`, `word`, `subst`
> · `cmdMZ` (subst group)

### Tier 2 — Interpretation

Turning parsed commands into execution: the bytecode compiler, the VM
execution loop, `eval`/`uplevel` re-entry, the non-recursive engine (NRE), and
the object/representation model.

> `basic`, `compile`, `execute`, `eval`, `assemble`, `obj`, `nre`,
> `appendComp`, `lsetComp`, `regexpComp`, `compExpr`, `compExpr-old`, `misc`

### Tier 3 — Fundamental Tcl machinery

The state model every command relies on. Roughly ordered variables →
namespaces → traces → aliases/introspection; all of it gates Tier 4.

- **Variable storage & management** — scalars, arrays, `upvar`/`uplevel`
  linkage, qualified vs local resolution.
  > `var`, `set`, `set-old`, `append`, `incr`, `incr-old`, `upvar`, `uplevel`,
  > `get`, `link` · `cmdMZ` (set/unset group)
- **Namespaces** — creation, qualified resolution,
  `import`/`forget`/`export`, ensembles, the namespace a command (incl. a
  renamed proc) runs in, custom command/variable resolvers.
  > `namespace`, `namespace-old`, `resolver`
- **Traces** — read / write / unset / array variable traces, command and
  execution traces (incl. firing from `info exists` and the `set` read path).
  > `trace` · `cmdMZ` (trace group)
- **Aliasing, rename & introspection** — `rename` (incl. across namespaces),
  and the `info` surface tests read to verify the machinery above.
  > `rename`, `info`, `cmdInfo` · `cmdIL` (info group)

### Tier 3.5 — Pure-compute data types

Lists, dicts, strings, numeric/format/scan, regexp, and binary. They depend
only on Tiers 1–3 and are prerequisites for most higher-tier test bodies, so
they sit between the machinery and control flow. Pure computation — identical
on every backend.

> Lists: `list`, `lindex`, `linsert`, `llength`, `lrange`, `lrepeat`,
> `lreplace`, `lsearch`, `lset`, `lsetComp`, `lmap`, `lpop`, `lseq`, `lreverse`,
> `listObj`, `listRep`, `abstractlist`, `range`
> · Dicts: `dict`
> · Strings: `string`, `stringObj`, `format`, `scan`, `split`, `join`, `concat`
> · Regexp: `regexp`, `regexpComp`, `reg`
> · Binary / misc: `binary`, `indexObj`, `util`, `stack`, `assocd`
> · `cmdIL` (list-ops group), `cmdMZ` (scan/string/split group),
> `cmdAH` (format group)

### Tier 4 — Control flow & procedures

Conditionals, loops, exceptions, procedures, expressions, coroutines, and the
object system — the constructs that drive a test body. Built on Tiers 1–3.5.

> Flow: `if`, `if-old`, `for`, `for-old`, `foreach`, `while`, `while-old`,
> `switch`, `error`, `result`
> · Expr: `expr`, `expr-old`, `mathop`
> · Procs: `proc`, `proc-old`, `apply`, `unknown`
> · Coroutines / tailcall: `coroutine`, `tailcall`, `dcall`
> · TclOO: `oo`, `ooNext2`, `ooProp`, `ooUtil`
> · `cmdMZ` (return/switch group),
> `cmdAH` (break/case/catch/continue/error/eval/for/foreach group)

### Tier 5 — Interpreters (child & safe)

Creating, evaluating in, and tearing down **child interpreters**, the
cross-interpreter **alias** and **hidden-command** machinery, and **safe
interpreters** (`interp create -safe`, `marktrusted`, the Safe Base). Placed
above control flow because a child interp re-enters the whole evaluator, and
below I/O because a *safe* interp is precisely the mechanism that gates a
child's access to Tiers 6–7. Depends on namespaces and aliases (Tier 3) and
`eval`/control flow (Tiers 2, 4).

- **Child interpreters** — `interp create`/`eval`/`delete`/`exists`/`children`,
  child-as-command dispatch, recursion limits, `interp target`/`share`/
  `transfer`, `interp alias`/`aliases` across paths.
  > `interp`
- **Safe interpreters & the Safe Base** — `interp create -safe`, `issafe`,
  `marktrusted`, `hide`/`expose`/`hidden`/`invokehidden`, and the
  `safe.tcl` re-aliasing of `source`/`load`/`file`/`encoding`.
  > `safe`, `safe-stock`, `safe-stock86`, `safe-zipfs`, `security`
  > · `opt` (the `::tcl::OptProc` option parser the Safe Base builds on)

### Tier 6 — I/O

Channels, files, globbing, sockets, and the event loop. The first tier whose
capabilities a backend can *lack* — WASI has no sockets, eBPF has no I/O at
all — so this is where the [backend-constraint overlay](backend-constraints.md)
starts skipping tests.

> Channels: `io`, `ioCmd`, `ioTrans`, `iogt`, `chan`, `chanio`
> · Sockets: `socket`
> · Event loop: `event`, `async`, `timer`, `notify`, `fileevent`
> · Files / FS: `file`? (see `cmdAH`), `fCmd`, `fileName`, `fileSystem`, `pwd`,
> `link`, `source`
> · `cmdAH` (after/close/eof/fblocked/fconfigure/file/fileevent/flush/gets/glob
> group)

### Tier 7 — Platform-specific features

Capabilities tied to the host OS or a specific build: clocks/timezones,
encodings, message catalogues, subprocesses, dynamic loading, threads,
packages/modules, compression/archives, networking protocols, and the
platform bootstrap. Heavily backend-gated — most are skipped on
wasm / WASI / eBPF.

> Time: `clock`, `clock-ivm`
> · Encodings / Unicode: `encoding`, `utf`, `utfext`, `icu`,
> `fileSystemEncoding`
> · Localisation: `msgcat`
> · Process / OS: `exec`, `process`, `pid`, `env`
> · Dynamic code: `load`, `unload`
> · Threads: `thread`, `mutex`
> · Packages / modules: `package`, `pkgMkIndex`, `autoMkindex`, `tm`,
> `config`
> · Archives / compression: `zipfs`, `zlib`
> · Networking protocols: `http`, `http11`, `httpPipeline`, `httpProxy`,
> `httpcookie`
> · Registry / bootstrap: `registry`, `platform`, `init`, `main`, `history`,
> `aaa_exit`
> · Big data / misc: `bigdata`, `brodnik`
> · Native-only (out of scope for the portable runtimes): `unixFCmd`,
> `unixFile`, `unixForkEvent`, `unixInit`, `unixNotfy`, `winConsole`,
> `winDde`, `winFCmd`, `winFile`, `winNotify`, `winPipe`, `winTime`,
> `macOSXFCmd`, `macOSXLoad`
> · `cmdAH` (cd/clock/encoding/exec/exit group)

## Feature checklist — areas that are easy to forget

Each capability area below has at least one representative file; use it to
confirm a feature is actually being exercised, not silently absent.

| Area | Files | Tier |
|---|---|---|
| Coroutines / NRE | `coroutine`, `dcall`, `nre` | 2, 4 |
| TclOO | `oo`, `ooNext2`, `ooProp`, `ooUtil` | 4 |
| Namespace ensembles | `namespace` (ensemble group) | 3 |
| Custom resolvers | `resolver` | 3 |
| Child / safe interpreters | `interp`, `safe`, `safe-*`, `security` | 5 |
| Option parser (Safe Base) | `opt` | 5 |
| Encodings / Unicode | `encoding`, `utf`, `utfext`, `icu` | 7 |
| Channel transforms | `iogt`, `ioTrans` | 6 |
| Event loop / async | `event`, `async`, `timer`, `notify`, `fileevent` | 6 |
| Packages / modules | `package`, `pkgMkIndex`, `autoMkindex`, `tm` | 7 |
| Compression / archives | `zlib`, `zipfs` | 7 |
| HTTP protocol | `http`, `http11`, `httpPipeline`, `httpProxy`, `httpcookie` | 7 |
| Abstract / big lists | `abstractlist`, `range`, `bigdata` | 3.5, 7 |

## How to use the ladder

- **Fix bottom-up.** A Tier 1 parser bug or a Tier 3 trace bug shows up as
  scattered failures across many higher-tier stems; fixing it lifts all of
  them at once (firing read traces from `info exists` unblocked lazy tcltest
  constraints across the whole suite). Chasing a Tier 4 failure while a Tier 3
  fundamental is broken is usually wasted work.
- **A CRASH on a low tier is the highest-leverage fix** — it zeroes a whole
  file (e.g. `cmdAH`'s 16 820 C-passing tests are gated behind `interp create`,
  a Tier 5 feature the bytecode VM still lacks).
- **Use the scoreboard for "where", the ladder for "why".** Within a tier,
  prefer the stem whose failures share a single root cause.
- **Tier 6–7 parity is per-backend.** On native, match C exactly; on
  wasm / WASI / eBPF the honest target is "skip what you can't do", which the
  [backend-constraint overlay](backend-constraints.md) handles, so the
  pass / fail / skip line still matches what the backend can run. Tier 5 safe
  interpreters are the in-language counterpart: a safe child legitimately
  *cannot* reach Tier 6–7, so those tests are expected to fail closed.

## Cross-references

- [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) — live per-stem scoreboard.
- [`backend-constraints.md`](backend-constraints.md) — per-backend skip overlay
  and `tcl_platform` introspection.
- [`child-interp.md`](child-interp.md) — child-interpreter design (Tier 5).
- [`tcltest-bringup.md`](tcltest-bringup.md) — how the real `init.tcl` /
  `tcltest.tcl` are brought up, and the no-edit hard rules.
