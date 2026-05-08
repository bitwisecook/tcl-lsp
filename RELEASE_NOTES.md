# v1.8.1

## New Features

- WASM runtime now ships an optional `tcltest` extension, enabling Tcl 9 core
  test slices to run inside the embedded VM.
- Implemented `upvar` support for array elements, allowing aliases such as
  `upvar 1 arr(key) localName` to work correctly.
- Added a Tcl 9 core test slice harness with regression baselines for both the
  Python VM and the Zig WASM runtime, tracked under `tests/baselines/`.
- New `impossible_in_wasm_wasi` and `impossible_in_wasm_browser` capability
  buckets so platform / host-capability tests are correctly classified per
  target.

## Improvements

- Variable completion now preserves the leading `$` and any partial token via
  a precise `TextEdit`, eliminating duplicated dollar signs and
  truncated-prefix glitches in editors that apply completions verbatim
  (issue #362).
- Lazy materialisation of `BUILTIN` namespaces speeds up `namespace import` of
  rarely used namespaces and reduces start-up cost for large workspaces.
- Pre-populated `::tcl::*` and `::oo::*` implementation namespaces so
  reflection commands (`namespace children`, `info commands`) match Tcl 9
  behaviour out of the box.
- Non-finite floats now render as `Inf` / `-Inf` / `NaN` to match Tcl 9.0.3,
  and subnormal floats use `Tcl_PrintDouble`-style formatting for
  byte-for-byte parity with the reference interpreter.
- Centralised the frame / no-frame compiler architecture, simplifying codegen
  for commands that switch between framed and non-framed execution and
  extending the embedded C Tcl reference data.
- Implemented Tcl 9 expression semantics across the compiler and VM, bringing
  arithmetic, comparison, and bitwise operator behaviour in line with Tcl
  9.0.3.

## Bug Fixes

- Fix qualified array reads (`$ns::arr(key)`) inside nested command
  substitutions, which previously failed to resolve the namespace prefix
  (issue #370).
- Fix integer overflow in `lseq` when called with large floating-point
  arguments — the runtime now produces the same error as Tcl 9 instead of
  wrapping around (issue #369).
- Fix `regexp` / `regsub` error handling so failures propagate correctly, and
  fix coroutine semantics around yielded results.
- Fix i32 negation overflow in the WASM runtime (negating `INT32_MIN` no
  longer traps).
- Fix array name resolution for global aliases and the root namespace, so
  `::arr(key)` and global aliases pointing at array elements resolve
  consistently.
- Fix variable resolution and trace dispatch inside namespace contexts so
  read / write traces fire reliably for namespace-qualified variables.
