# KCS: WASM silently returned 0 for unset variable reads

> **Audience:** Developer
> **Type:** Issue

## Applies to

all-editors

## Question

Why did `expr {$undefined + 1}` in compiled WASM return `1` instead of
raising `can't read "undefined": no such variable` the way the Python
VM and reference Tcl do?

## Symptoms

- `$x` substitutions / `set x` (one-arg read) / `expr {$x}` operands
  silently produced the empty string / 0 / a default-valued TclObj
  when `x` had never been set in the current scope.
- Loops driven by such a missing read never terminated: `while {$count
  < 10} {incr count}` ran forever because the first read of `$count`
  returned 0, the comparison was true, and `incr` operated on the
  same null TclObj.  This drove a measurable fraction of the
  `wasm-timeout` finding cluster.
- The Python VM (`tooling/tooling/vm/scope.py`) and `vm_opt` correctly raised the
  standard error; only the compiled WASM path was lenient.

## Answer

Issue #263.  The Zig runtime's `local_get` and `global_get` exports
returned `0` (a NULL TclObj handle) when the variable had never been
set, and the WASM codegen's variable-read path emitted a bare
`local.get` of a slot that defaults to 0 — so neither side saw a
"missing" signal.

The fix lives in three places:

1. [`runtime/zig/interp/tcl_catch.zig`](../../runtime/zig/interp/tcl_catch.zig)
   adds `var_unset_error(name_obj)` that builds the standard error
   message and routes it through `tcl_cmd_error` (trap outside catch,
   set `error_flag` inside).
2. [`runtime/zig/interp/tcl_frames.zig`](../../runtime/zig/interp/tcl_frames.zig)
   adds `local_get_or_error` and
   [`runtime/zig/interp/tcl_ns.zig`](../../runtime/zig/interp/tcl_ns.zig)
   adds `global_get_or_error` — strict variants of the existing
   lookups that delegate to the lenient form and raise via
   `var_unset_error` when the lookup resolves to a NULL TclObj.
3. [`compiler/codegen/wasm/_emitter/_variables.py`](../../compiler/codegen/wasm/_emitter/_variables.py)
   `_emit_var_read_obj` now routes every user-visible variable read
   through the strict variants.  The proc-local mirror path emits an
   inline `i32.eqz` check that calls `tcl_var_unset_error` on the
   cold (zero-slot) path.

The lenient `tcl_local_get` / `tcl_global_get` exports stay in use by
paths that legitimately want missing-is-fine behaviour:

- `info exists` / `unset -nocomplain` / `array names` / `array exists`
- The frame readback after an eval-fallback (`_emit_frame_readback`)
- The `global` command's pre-load of a possibly-uninitialised slot
- The compile-time `_emit_value` fast path for bare local-name
  references (which only fires after the slot has been written by a
  prior IR statement)

## Optimisation: elided checks

The WASM-local mirror path elides the inline check only when the
variable name appears in `self._params` — proc parameters are
initialised by the call prologue's param-retain before the body
runs, so the slot is provably bound on every read.

For correctness the emitter does **not** use `self._first_writes_seen`
to skip the check on later reads of a slot.  That set tracks emission
order, not runtime reachability: a write inside an `if` branch flips
the flag at compile time, so a follow-up read would take the elision
fast path even when the branch is not taken at runtime.  The earlier
shape of this fix did make that elision and silently returned 0 for
`if {$flag} {set x 1}; set x` when `$flag` was false.  Every
non-parameter read therefore keeps the inline `i32.eqz` guard and
raises on the cold zero-slot path.

The S5.2 alias-skip peephole (`set x $x` → zero-instruction body) is
still preserved because:

- The dedicated compile-time `_emit_value` fast path emits a bare
  `local.get` for bare-name references resolved through `_local_index`
  (no `i32.eqz` wrap), which is the path the peephole pattern-matches
  on.
- The proc-local mirror path's elision for parameters means the typical
  `set x $x` (where `x` is a param) emits the same straight
  `local.get` shape today as before issue #263.

See [`tests/test_wasm_codegen.py`](../../tests/test_wasm_codegen.py)
`test_s52_alias_skip_self_assign` /
`test_s52_counter_increments` for the coverage.

## Scratch-local reuse

The inline check uses a single per-function scratch i32
(`_var_unset_check_scratch`, lazily allocated on first read site) for
the `local.tee` / `i32.eqz` peek.  An earlier shape of this code
allocated a fresh `_var_check_<n>` slot on every read, which grew the
locals section linearly with the number of variable reads in a proc.
Reusing one slot keeps the locals count constant.

## Regression coverage

[`tooling/fuzzing/tests/test_fuzz_findings.py`](../../tooling/fuzzing/tests/test_fuzz_findings.py)
`TestBatch6WasmIgnoresMissingVar` parametrises the four seed scripts
(`1774200012`, `1774200028`, `1774200037`, `1774200068`) whose
mismatch was directly the missing-variable behaviour and asserts
the WASM run now surfaces a Tcl-level error matching the VM's
return code.

Three other seeds the original issue listed (`1774200067`,
`1774200082`, `1774200094`) surface as independent `wasm-timeout`
bugs once the missing-variable signal stops short-circuiting the
runaway loop.  These are addressed by issues #260, #261, and #262
(bitwise / shift float-domain checks plus strict-integer `incr`),
and the end-to-end coverage lives in `TestBatch6WasmStrictIntOpsCombo`
in the same test file.  All seven seed JSONs flip to
`"fixed": true`.

## Strict-integer follow-up: incr-on-unset preservation

Tightening `incr` to enforce the strict-integer contract initially
regressed the unset-variable case: routing the codegen through the
strict `_emit_var_read_obj` raised `can't read "<name>": no such
variable` for `proc p {} { incr x }`, but Tcl 8.5+ semantics require
that `incr` on an unset scalar initialise it to `0` and return the
increment.  Codex P1 review on PR #288 caught this.

The fix lives in two places:

1. [`runtime/zig/interp/tcl_ns.zig`](../../runtime/zig/interp/tcl_ns.zig)
   `tcl_incr` treats a null `o` (TclObj 0) as the unset case and
   initialises to `0` before adding.  The strict-integer contract
   still applies to non-empty values that fail the integer parse
   (a string `"abc"` or a float `"5.0"` still raises).
2. [`compiler/codegen/wasm/_emitter/_variables.py`](../../compiler/codegen/wasm/_emitter/_variables.py)
   adds `_emit_var_read_obj_lenient` — a lenient counterpart of
   `_emit_var_read_obj` that returns 0 (null TclObj) for an unset
   variable instead of raising.  The three `IRIncr` emit sites
   (`_statements.py`, `_control_flow.py`, `_optimisation.py`) call
   the lenient variant; `tcl_incr`'s null-input branch turns the
   null TclObj into the matching `0 + amount` initialisation.

`TestBatch6WasmStrictIntOpsCombo::test_incr_initialises_unset_scalar`
parametrises five shapes (`proc p {} { incr x }`,
`proc q {} { incr y 5 }`, `proc r {} { incr z -3 }`, `incr top`,
`incr top 7`) and locks in the Tcl 8.5+ behaviour.

## First-error-wins guards on the runtime error helpers

A second Copilot review on PR #288 noted that the int-op error
helpers (`raise_float_in_bitwise` / `raise_float_in_unary_bitwise`
in `tcl_arith.zig`, `raise_expected_integer` in `tcl_ns.zig`, plus
the top-level `tcl_incr` entry) called `tcl_cmd_error`
unconditionally.  Inside a catch scope, that overwrites `error_msg`
even when `error_flag` was already set by an earlier failure in the
same statement (e.g. a missing-variable read on the other operand)
— clobbering the original diagnostic.

The fix adds an early `if (error_flag != 0) return;` guard at the
top of each helper.  Once an error is pending these helpers become
no-ops, so the first error stays the one the catch boundary
surfaces — matching reference Tcl's "first error wins" semantics
for a single command.
