# KCS: VM `set.test` core slice — array-set, braced-key, and errorInfo fixes

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI, compiler, runtime, test-suite

## Question

Why did the Tcl 9 `set.test` core slice report six failures through the
Python VM (`make test-tcl9-vm-core`), and what made each one pass so the
stem now matches C Tcl 9 exactly (63 passed, 1 skipped, 0 failed)?

## Symptoms

`tmp/tcl9.0.3/tests/set.test` through `vm.interp.TclInterp` failed:

- `set-1.15`, `set-1.26` — array-element assignments produced the wrong
  value or raised.
- `set-1.25` — a *masked* failure: it only passed because the
  `set-1.15` bug leaked a stray scalar `foo`.
- `set-2.1`, `set-4.1` — `$::errorInfo` lacked the `while executing`
  frame, or rendered a reconstructed command (`${z} "foo"`) instead of
  the original source (`$z {"foo}`).
- `set-2.4`, `set-4.4` — a write-trace rejection collapsed the
  traceback to one frame instead of the `readonly` → trace → `set`
  chain the `-match glob` result expects.

## Answer

Five independent codegen / runtime contracts were wrong:

1. **Nested array-element `set`.** `set x [set a(foo) 11]` unrolled to a
   flat `storeStk` chain in `compiler/codegen/bytecode/_statements.py`,
   so the inner array element used the scalar store. The unroll now
   emits `storeArrayStk` for an array-element target (matching tclsh
   9.0's disassembly).

2. **Brace-suppressed array key.** `set {arr($foo)} 5` must store under
   the *literal* element `$foo`. Lowering normalises a *live* index to
   `${foo}` and leaves a brace-suppressed one bare, so
   `_push_array_key` (`_values.py`) now pushes a bare simple `$foo`
   *store-target* key with the raw (no-substitution) marker, while a
   live read index (`$testConstraints($constraint)`) still resolves.

3. **Array read inside a word template.** `$be($a,hej)` embedded in a
   larger key/value was parsed as the whole array `be`;
   `_parse_subst_template` (`_helpers.py`) now captures the
   parenthesised index so the element resolves.

4. **Backslash + live substitution in a generic invoke.** `b\e($a)`
   collapsed `\e` then pushed the result raw, dropping the live `$a`
   index. `_emit_generic_cmd_subst` (`_cmd_subst.py`) now routes a word
   carrying both an escape and a live `$`/`[` through `_emit_value`.

5. **`while executing` errorInfo frames.** The VM compiles commands to
   bytecode, so a failing *compiled* `set`/`incr` op never logged a
   command frame. `eval()` / `_call_proc()` now append a
   `while executing "<cmd>"` frame for the failing instruction —
   restricted to command-terminal ops (`_FRAME_WORTHY_OPS` in
   `machine.py`) so a half-built outer command (`incr x [set]`) is not
   blamed — using the instruction's source span for the exact original
   text. A write-trace rejection (`fire_traces` in
   `vm/commands/trace_cmds.py`) preserves the callback's errorInfo
   chain and flags `invoke()` to append the triggering command's frame.

## Verification

```
make test-tcl9-vm-core        # whole-slice regression gate
uv run pytest tests/test_vm_set_codegen.py -q
```

`set` now reports `63 / 1 / 0` (pass / skip / fail), matching the
`c-tclsh.ndjson` reference. The same changes also improved `list`
(20→4 fails), `linsert`, `for`, `if`, and `parse` with no stem
regressing.

## Scope

This note covers the **Python VM** (`vm.interp`). The Zig WASM runtime
is a separate target; its `set.test` parity is tracked under
`tests/baselines/tcl9-tcltest-wasm/` and is not addressed here.
