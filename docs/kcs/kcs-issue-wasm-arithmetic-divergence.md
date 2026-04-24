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
- The numeric `clock format` / `clock scan` / `clock add` paths return
  `"0"` / `0` instead of raising `unsupported command`.
- In the interpreter path (not compiled), `expr` still behaves the way
  tclsh does — the divergence is specific to compiled WASM.

## Answer

The divergence is deliberate and documented in the header of
[`runtime/zig/tcl_arith.zig`](../../runtime/zig/tcl_arith.zig) and
[`runtime/zig/tcl_time_stubs.zig`](../../runtime/zig/tcl_time_stubs.zig).

Rationale:

- The tcllib `counter::init -timehist` code path hits a transient
  zero-divisor on first-bucket initialisation. tclsh gates this with
  surrounding state our compiled runtime does not yet reconstruct;
  trapping on the divide would abort the counter test bundle on every
  run.
- `clock format` / `clock scan` / `clock add` require a full timezone
  database we do not ship in the WASM module.

Workarounds for user code:

- If you suspect a silent divide-by-zero, guard with `if {$d == 0}
  {error "denominator is zero"}` before the divide so you surface the
  problem yourself.
- If you depend on real `clock format` semantics (timezone-aware date
  formatting), use the Python VM path rather than the compiled WASM
  path.

Follow-up work (track in a separate KCS issue if the follow-up spawns
its own sub-problems):

1. Raise `divide by zero` from `tcl_arith_div` / `tcl_arith_mod` once
   the CFG-level guards and counter::init code path are reliable
   enough to avoid regression.
2. Wire a real TZ database (or a minimal Olson subset) behind the
   `clock` stubs so `clock format` / `clock scan` produce correct
   values.
