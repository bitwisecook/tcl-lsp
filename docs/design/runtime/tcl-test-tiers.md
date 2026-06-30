# Tcl test tiers — the capability ladder to C parity

The goal is for the Rust interpreters (`tcl-vm` bytecode VM, `runtime/rust`
tree-walk runtime) to match C Tcl 9's pass / fail / skip on the upstream
tcltest suite. This document orders that work as a **capability ladder**: a
lower tier is a prerequisite for the tiers above it, so fixing tiers
bottom-up is the highest-leverage order. A bug in parsing corrupts every test
above; a missing socket only affects I/O.

The live per-stem scoreboard (C P/S/F vs VM P/S/F, MATCH / gap / CRASH) lives
in [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md). This document is the
*semantic* grouping behind it — what each tier means and why its order matters.
The two are complementary: the scoreboard says *where we are*, the ladder says
*what to fix next and why*.

## The ladder

### Tier 1 — Parsing

The lexer, word splitter, brace/quote/bracket nesting, backslash and
command/variable substitution, and the expression tokeniser. Everything else
is built on getting bytes → words → commands right.

> Stems: `parse`, `parseOld`, `parseExpr`, `word`, `subst`

### Tier 2 — Interpretation

Turning parsed commands into execution: the bytecode compiler, the VM's
execution loop, `eval`/`uplevel` re-entry, and the object/representation model.
A parser that is correct but an executor that mis-dispatches fails here.

> Stems: `basic`, `compile`, `execute`, `eval`, `assemble`, `obj`, `nre`,
> `appendComp`, `lsetComp`, `regexpComp`, `compExpr`, `compExpr-old`, `misc`

### Tier 3 — Fundamental Tcl machinery

The state model every command relies on. Within the tier the order is roughly
variables → namespaces → traces → aliases/introspection, but all of it gates
Tier 4.

- **Variable storage & management** — scalars, arrays, `upvar`/`uplevel`
  linkage, qualified vs local resolution.
  > `var`, `set`, `set-old`, `append`, `incr`, `incr-old`, `upvar`, `uplevel`, `get`
- **Namespaces** — creation, qualified resolution, `import`/`forget`/`export`,
  ensembles, the namespace a command (incl. a renamed proc) executes in.
  > `namespace`, `namespace-old`
- **Traces** — read / write / unset / array variable traces and command traces,
  including firing from `info exists` and the `set` read path.
  > `trace`
- **Aliasing, rename & introspection** — `rename` (incl. across namespaces),
  `interp alias`, and the `info` surface tests read to verify the above.
  > `rename`, `info`, `cmdInfo`, `indexObj`, `assocd`

#### Tier 3.5 — Pure-compute data types

Lists, dicts, strings, and the numeric/format/scan commands. They depend only
on Tiers 1–3 and are themselves prerequisites for most higher-tier test
bodies, so they sit between the machinery and control flow. Pure computation —
they run identically on every backend.

> `list`, `lindex`, `linsert`, `llength`, `lrange`, `lrepeat`, `lreplace`,
> `lsearch`, `lset`, `lmap`, `lpop`, `lseq`, `listObj`, `listRep`,
> `abstractlist`, `dict`, `string`, `stringObj`, `format`, `scan`, `split`,
> `join`, `concat`, `regexp`, `cmdIL`, `cmdAH`, `binary`

### Tier 4 — Control flow & evaluation

Conditionals, loops, exceptions, procedures, and expressions — the constructs
that drive a test body. Built on Tiers 1–3.5.

> Stems: `if`, `if-old`, `for`, `for-old`, `foreach`, `while`, `while-old`,
> `switch`, `error`, `result`, `cmdMZ` (catch/try/throw), `expr`, `expr-old`,
> `mathop`, `proc`, `proc-old`, `apply`, `unknown`, `tailcall`, `coroutine`,
> `oo` (TclOO)

### Tier 5 — I/O

Channels, files, globbing, sockets, and the event loop. The first tier whose
capabilities a backend can *lack*: WASI has no sockets, eBPF has no I/O at all.
This is where the [backend-constraint overlay](backend-constraints.md) starts
skipping tests.

> Stems: `io`, `chan`, `chanio`, `iocmd`, `iogt`, `iortrans`, `socket`,
> `fileevent`, `file`, `glob`, `fCmd`, `fileName`

### Tier 6 — Platform-specific features

Capabilities tied to the host OS or a specific build: clocks/timezones,
encodings, message catalogues, subprocesses, dynamic loading, threads, and the
safe-interpreter machinery. Heavily backend-gated — most of these are skipped
on wasm / WASI / eBPF.

> Stems: `clock`, `encoding`, `msgcat`, `utf`, `safe`, `safe-stock`,
> `safe-stock86`, `safe-zipfs`, `exec`, `load`, `thread`, `registry`, `env`,
> `pid`

## How to use the ladder

- **Fix bottom-up.** A Tier 1 parser bug or a Tier 3 trace bug shows up as
  scattered failures across many higher-tier stems; fixing it lifts all of
  them at once (e.g. firing read traces from `info exists` unblocked lazy
  tcltest constraints across the whole suite). Chasing an individual Tier 4
  failure while a Tier 3 fundamental is broken is usually wasted work.
- **Use the scoreboard for "where", the ladder for "why".** A CRASH on a low
  tier is the highest-leverage fix — it zeroes a whole file. Within a tier,
  prefer the stem whose failures share a single root cause.
- **Tier 5–6 parity is per-backend.** Match C on native; on wasm / WASI / eBPF
  the honest target is "skip what you can't do", which the
  [backend-constraint overlay](backend-constraints.md) handles, so the
  pass/fail/skip line still matches what the backend can run.

## Cross-references

- [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) — live per-stem scoreboard.
- [`backend-constraints.md`](backend-constraints.md) — per-backend skip overlay
  and `tcl_platform` introspection.
- [`tcltest-bringup.md`](tcltest-bringup.md) — how the real `init.tcl` /
  `tcltest.tcl` are brought up on the runtime, and the no-edit hard rules.
