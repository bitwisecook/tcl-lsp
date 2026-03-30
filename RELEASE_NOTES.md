# v1.3.0

## New Features

- **TclOO support**: full TclOO runtime in the Tcl VM — classes, objects,
  method dispatch, MRO computation, mixins, filters, properties, private
  variables (TIP 500), and constructor/destructor lifecycle.  85% native
  test conformance against Tcl 9.0.3 `oo.test` / `ooNext2.test` suites.
- **Type hierarchy**: supertypes/subtypes visualisation for class
  hierarchies via the standard LSP type-hierarchy protocol.
- **Per-file dialect directives**: `# tcl-dialect: <dialect>` comment in
  the first five lines pins a specific dialect for that file, overriding
  global settings.
- **Platform-native config paths**: configuration file now follows
  platform conventions (`~/.config/tcl-lsp/config.ini` on Linux,
  `~/Library/Application Support/tcl-lsp/config.ini` on macOS,
  `%APPDATA%\tcl-lsp\config.ini` on Windows).
- **O127 optimisation**: inline single-use variable assignments
  (store-to-load forwarding).
- **New commands**: `tailcall`, `array for`, `array default`, `const`,
  `namespace ensemble`, `info frame` with OO method context, and full
  slot operations for mixin/filter commands.

## Improvements

- W201 (path concatenation) migrated to the taint system using Rendered
  Value Properties for more precise escape-sequence detection.
- W210 (variable used before definition) no longer false-positives inside
  `tcltest::test` script arguments.
- W214 (unused parameter) fixed for variables appearing in
  `return [expr {...}]` substitutions.
- E204/E205 no longer false-positive on valid backslash-newline
  continuation after closing braces.
- Immediate dominator computation optimised from O(n²) to O(n);
  side-effect cache pre-computed and reused across passes.
- Semantic token pre-computation eliminates redundant work for faster
  highlighting.
- iRules dialect auto-detection now always triggers for `.irul`/`.irule`
  files, even when the editor reports its default `tcl8.6` setting.

## Bug Fixes

- Fixed compiler re-inserting proc definitions with dynamic params/body.
- Fixed namespace-relative class resolution in `oo::define`/`oo::objdefine`.
- Fixed MRO cycle detection for pure superclass cycles.
- Fixed method/filter call chain ordering to match C Tcl.
- Fixed `next`/`nextto` dispatch, private-method visibility, and
  cross-object private access.
- Fixed instance variable scoping per defining-class and variable slot
  operations.
- Fixed `oo::copy` namespace handling and private call chains.
- Fixed stale trace cleanup and auto-removal of traces with deleted
  commands.
- Fixed line-ending handling for `\r` and `\r\n` in
  backslash-continuation guards.
- Over 120 additional bug fixes for OO semantics, variable scoping,
  namespace resolution, and error reporting.
