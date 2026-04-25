# v1.7.2

## New Features
- Command introspection and manipulation: `info level`, `info script`, and
  related `info` callers now match tclsh error strings; `interp hide` /
  `interp expose` / `interp invokehidden` dispatch through a real
  hidden-command table, and `namespace which` is aware of hidden commands.
- Compiler explorer: structured WASM disassembly view, extended to the Tcl ASM
  tab, with unified orthogonal-edge rendering, line hover, diff-side arrows,
  and jump-table multi-edge support.

## Improvements
- `expr` now supports the `double()` cast.
- WASM codegen: substantial IR improvements and expanded execution-test
  coverage.
- Compiler explorer toolbar and tab bar in VS Code are now reactive.
- ASM output escapes all control characters; tabs renamed to "Tcl ASM" with
  consistent "(opt)" casing.
- Explorer serialisation discovers the wheel filename from `build_info.json`
  rather than hard-coding it.
- Namespace-tree walker is now shared across `hide` and `info` introspection
  callers.
- Procs grow an `OFF_EXPORT_NAME_BUCKET` sidecar for uniform rename handling.

## Bug Fixes
- Codegen: `proc_index` is now invalidated on `hide`, `rename`, and `expose`,
  and invalidation is qualified by the enclosing namespace so unrelated procs
  are no longer evicted.
- Runtime: `info` no longer double-scans the root namespace; `interp hidden`
  arity is correct.
- W210: reads of globals written by setter procs are suppressed; `unset` is
  excluded from that suppression, and the suppression is scoped to W210 only.
- VS Code folding: `folding.markers` is scoped to `#region` comments so
  unrelated comments no longer fold unexpectedly.
- Compiler explorer: CDN build loads without errors; the micropip 0.8.0
  `deps=False` race is worked around by bypassing micropip for the initial
  load.
- Terminator source ranges in structured disassembly are now correct.
