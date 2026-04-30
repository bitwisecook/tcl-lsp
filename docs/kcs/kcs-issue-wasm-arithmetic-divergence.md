# KCS: WASM arithmetic returns 0 instead of raising divide-by-zero

> **Audience:** Developer
> **Type:** Issue
> **Status:** Resolved (both arithmetic and clock divergences fixed)

## Applies to

all-editors

## Question

Why did `expr {1 / 0}` in compiled WASM return `0` instead of raising
`divide by zero` the way tclsh does?  And why did `clock format` /
`clock scan` / `clock add` return `"0"` / `0` instead of computing
the real value?

## Resolution

**Both divergences are fixed.**  The runtime now matches tclsh on:

- **`expr {1 / 0}`** — raises `divide by zero` (PR #237).  The
  silent-zero workaround the tcllib `counter::init -timehist` path
  used to depend on is gone; that test bundle has its own
  initialisation-order fix downstream.  See
  [`runtime/zig/valtypes/tcl_arith.zig`](../../runtime/zig/valtypes/tcl_arith.zig)
  for the implementation.
- **`clock format` / `clock scan` / `clock add`** — real
  implementations backed by a TZif (RFC 8536) parser, a host
  filesystem probe, and a comptime-embedded tzdata bundle.  See
  the
  [Clock + timezone resolution](../design/compiler/wasm-runtime-primitives.md#clock--timezone-resolution)
  section of the runtime-primitives doc for the architecture.

## Historical symptoms (pre-fix)

Kept here for reference so anyone bisecting older builds knows
what the old behaviour looked like:

- `expr {$a / $b}` silently returned `0` when `$b == 0` in compiled
  code, instead of surfacing `divide by zero` as a Tcl error.
- `expr {$a % $b}` likewise returned `0` on a zero divisor.
- The numeric `clock format` / `clock scan` / `clock add` paths
  returned `"0"` / `0` instead of producing real values.
- The interpreter path was unaffected; the divergence was specific
  to compiled WASM.

## How the fix works

### Divide-by-zero

The arithmetic helpers (`tcl_arith_div`, `tcl_arith_mod`) now call
`stubs.raise("divide by zero")` when the divisor is zero — matching
Tcl's `ARITH DIVZERO {divide by zero}` error code.  The header of
[`tcl_arith.zig`](../../runtime/zig/valtypes/tcl_arith.zig)
documents the rationale: silent zero made every legitimate
divide-by-zero in user code produce `0`, which is exactly the
porting hazard a runtime should not introduce.

### Clock + timezone

The resolver in
[`runtime/zig/io/tcl_tz.zig`](../../runtime/zig/io/tcl_tz.zig) walks
this lookup order:

1. Synthetic UTC — `-gmt 1` always works, no I/O.
2. Host tzdata via wasi-libc `open()`:
   `/usr/share/zoneinfo/<zone>` and friends.  Fresh tzdata
   reaches scripts without re-shipping the wasm binary.
3. Comptime-embedded bundle (`runtime/zig/data/tzdata.bin`,
   ~115 zones, ~133 KB).  Covers sandboxed environments that
   can't preopen anything.
4. Last-ditch synthetic UTC with a non-zero `last_error`.

Format / scan / add live in
[`runtime/zig/io/tcl_clock.zig`](../../runtime/zig/io/tcl_clock.zig)
and the dispatcher in
[`runtime/zig/cmds/stubs.zig`](../../runtime/zig/cmds/stubs.zig).
The free-form scan grammar covers `now` / `today` / `yesterday` /
`tomorrow` / `+N unit` / `Month Day, Year` / `MM/DD/YYYY` plus
the ISO date / RFC 3339 / integer-epoch forms.  Tests in
[`tests/test_wasm_clock.py`](../../tests/test_wasm_clock.py)
pin the contract.

## Remaining follow-ups (none blocking)

These are documented for completeness but not required for any
known caller:

1. **DST-aware month math.**  `clock_add_pair` for `months` /
   `years` does the calendar math in UTC; a calling script that
   spans a DST transition gets the same wall-clock-in-UTC answer
   tclsh produces unless its `-timezone` argument explicitly
   crosses the transition.  Edge case — wait for a real bug
   report before fixing.
2. **`clock scan "next thursday"` and weekday relativisers.**
   The full `library/clock.tcl::GetDate.y` grammar (~3 KSLOC of
   yacc) is not ported.  Tcl's free-form weekday-relative inputs
   are rarely used; the ISO + relative-units + month-name forms
   we ship cover the common cases.
3. **Tzdata bundle trim.**  The bundle today ships untrimmed TZif
   blobs (~3-4 KB each).  A "decade ± 5 years" trimmer would
   shrink ~70 % off the 133 KB total — see
   [`docs/design/compiler/wasm-runtime-primitives.md`](../design/compiler/wasm-runtime-primitives.md#bundled-trimmed-tzdata-fallback-deferred).
   Worth doing once the wasm binary's size budget tightens.
