# v1.7.1

## New Features
- WASM compiler: compile pure Tcl to standalone WebAssembly with an embedded Zig interpreter, supporting procs, namespaces, arrays, upvar/uplevel, real regexp via Tcl's Spencer engine, and WASI-backed file/clock operations
- Runtime namespace tree: full hierarchical namespace resolution with `namespace eval`, `namespace import/export/forget`, `namespace path`, and per-namespace command/variable tables
- Workspace signature scan: background indexing descends into if/catch/try bodies for broader symbol coverage
- Runtime rename and interp alias support with dispatch trampolines and invalidation
- Parse cache: sidecar storage keyed on body pointer/length, consulted during eval for faster proc lookups
- Option-shape factory call-site specialisation in the compiler pass
- Compile-time `subst -nocommands` evaluator and `proc $var body` resolution via lowering const-map

## Improvements
- LRU cache for `proc_lookup` and namespace-import call resolution at compile time
- Resolve proc-locals into the call frame for interpreter visibility; pre-eval-sync replaces per-write frame-writeback
- CodeLens uses `editor.action.showReferences` and walks the workspace index for peek locations
- Folding: serve ranges immediately after `didOpen`; request `workspace/foldingRange/refresh` after analysis; keep if/else sibling folds disjoint
- Zig runtime rebuilt with ReleaseFast and Zig 0.16 (`callconv(.C)` → `.c`)
- AI diagnostics extended with W230–W242 warnings
- VS Code extension test coverage for `tcl-lsp.showReferences` adapter

## Bug Fixes
- Fix IEEE 754 edge cases: `string is double`, `scan %f`, Inf literals, integer overflow, sign of `-0.0`, division by `±0.0`
- Fix `&&`/`||` identity rewrites to preserve Tcl's boolean result
- Fix `$arr(key)` / `[set arr(key)]` reads in value and interpolation contexts
- Fix `subst_flagged` output-buffer overflow
- Fix code folding seen-set type to match lsprotocol kind declaration
- Suppress W002 dialect warning when a user proc shadows the command
- Suppress namespace import/export in dead if branches
- Fix CodeLens JSON-to-VS-Code type conversion for showReferences
- Fix KCS diagnostic filename casing for W130–W134
- Fix npm audit: pin lodash to ^4.18.1
