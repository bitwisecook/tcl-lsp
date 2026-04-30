# KCS: WASM arithmetic returns 0 instead of raising divide-by-zero

> **Audience:** Developer
> **Type:** Issue

## Applies to

all-editors

## Question

Why does `expr {1 / 0}` in compiled WASM return `0` instead of raising
`divide by zero` the way tclsh does?

## Symptoms

- `expr {$a / $b}` silently returns `0` when `$b == 0` in compiled
  code, instead of surfacing `divide by zero` as a Tcl error.
- `expr {$a % $b}` likewise returns `0` on a zero divisor instead of
  raising.
- ~~The numeric `clock format` / `clock scan` / `clock add` paths
  return `"0"` / `0` instead of raising `unsupported command`.~~
  **Fixed** — see the
  [Clock + timezone resolution](../design/compiler/wasm-runtime-primitives.md#clock--timezone-resolution)
  section of the runtime-primitives doc.  `clock format` /
  `clock scan` / `clock add` are now real implementations: the TZ
  resolver probes host tzdata under `/usr/share/zoneinfo` /
  `/etc/zoneinfo` / `/etc/localtime` (capability-gated by the
  embedder's WASI preopens) and falls back to a synthetic UTC zone
  when the host has nothing exposed.
- In the interpreter path (not compiled), `expr` still behaves the way
  tclsh does — the divergence is specific to compiled WASM.

## Answer

The divergence is deliberate and documented in the header of
[`runtime/zig/valtypes/tcl_arith.zig`](../../runtime/zig/valtypes/tcl_arith.zig) and
[`runtime/zig/stubs/tcl_time_stubs.zig`](../../runtime/zig/stubs/tcl_time_stubs.zig).

Rationale:

- The tcllib `counter::init -timehist` code path hits a transient
  zero-divisor on first-bucket initialisation. tclsh gates this with
  surrounding state our compiled runtime does not yet reconstruct;
  trapping on the divide would abort the counter test bundle on every
  run.
- ~~`clock format` / `clock scan` / `clock add` require a full
  timezone database we do not ship in the WASM module.~~  Resolved
  by the TZ-resolver work — see the runtime-primitives doc.

Workarounds for user code:

- If you suspect a silent divide-by-zero, guard with `if {$d == 0}
  {error "denominator is zero"}` before the divide so you surface the
  problem yourself.
- For `clock format` with a non-UTC timezone, the embedder must
  preopen `/usr/share/zoneinfo` (or another tzdata directory) into
  the WASI sandbox.  Without a preopen the resolver falls back to
  synthetic UTC — `-gmt 1` always works, named zones quietly
  degrade to UTC instead of trapping.

Follow-up work (track in a separate KCS issue if the follow-up spawns
its own sub-problems):

1. Raise `divide by zero` from `tcl_arith_div` / `tcl_arith_mod` once
   the CFG-level guards and counter::init code path are reliable
   enough to avoid regression.  PR #237 raised the divide-by-zero
   path; once the counter::init regression is gone the workaround
   note above can be retired entirely.
2. Bundle a trimmed Olson tzdata blob into the wasm binary so hosts
   without tzdata preopens still produce real local-time output.
   The trimmer design is sketched in
   [`docs/design/compiler/wasm-runtime-primitives.md`](../design/compiler/wasm-runtime-primitives.md#bundled-trimmed-tzdata-fallback-deferred).
3. Port the free-form date grammar from `library/clock.tcl::GetDate`
   so `clock scan "next thursday"` and similar relative-date inputs
   work in compiled WASM.  The current parser handles ISO + RFC 3339
   + integer-epoch only.
