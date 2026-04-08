# v1.5.2

## New Features

- Declarative `arg_role_resolver` framework replaces hardcoded body-index
  functions, making it easier to teach the analyser about new commands whose
  arguments contain Tcl script bodies.
- Profile-based README filtering generates editor-specific documentation for
  the VS Code VSIX and other editor packages.

## Improvements

- Added ~130 missing tcllib/stdlib command definitions (logger, math::statistics,
  json, struct, and others) with full hover, arity, and purity metadata.
- Comprehensive audit of core Tcl, TclOO, and I/O command registry entries
  against official man pages — fixed arity, subcommand trees, synopses, and
  hover documentation across dozens of commands.
- Added `array default` nested subcommands from Tcl 9.0.
- Fixed `switch` synopsis and `try` handler variable roles.

## Bug Fixes

- Fixed `property -set`/`-get` bodies not being highlighted as Tcl scripts.
- Fixed `property` command to match the Tcl 9.0 spec: correct option parsing,
  accessor scope propagation, and multi-name property declarations.
- Fixed Sublime Text build to use `uv pip` instead of `python -m pip`.
- Marked deprecated tcllib command aliases (e.g. `::math::statistics` legacy
  names) so diagnostics can flag them.
