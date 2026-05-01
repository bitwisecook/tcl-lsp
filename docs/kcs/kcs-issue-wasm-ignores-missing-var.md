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
- The Python VM (`vm/scope.py`) and `vm_opt` correctly raised the
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
3. [`core/compiler/codegen/wasm/_emitter/_variables.py`](../../core/compiler/codegen/wasm/_emitter/_variables.py)
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

[`fuzzing/tests/test_fuzz_findings.py`](../../fuzzing/tests/test_fuzz_findings.py)
`TestBatch6WasmIgnoresMissingVar` parametrises the four seed scripts
(`1774200012`, `1774200028`, `1774200037`, `1774200068`) whose
mismatch was directly the missing-variable behaviour and asserts
the WASM run now surfaces a Tcl-level error matching the VM's
return code.

Three other seeds the original issue listed (`1774200067`,
`1774200082`, `1774200094`) turn out to surface independent
`wasm-timeout` bugs once the missing-variable signal stops
short-circuiting the runaway loop; those need their own follow-up
fixes and stay `"fixed": false` for now.
