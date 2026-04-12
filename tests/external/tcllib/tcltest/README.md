# Vendored tcltest (Tcl core 8.6.15 library)

Source: [tcltk/tcl library/tcltest](https://github.com/tcltk/tcl/tree/core-8-6-15/library/tcltest)

`tcltest` is shipped with Tcl itself (not tcllib — tcllib's tests merely
`package require` it).  These files are vendored verbatim so we can
concatenate the whole harness into a single compilation unit and run it
through our WASM codegen. No modifications — we want to validate that
real upstream tcltest survives the normal init pipeline.

## Files

- `tcltest.tcl` — the harness (3588 lines, version 2.5.8)
- `pkgIndex.tcl` — minimal package metadata (`package ifneeded tcltest 2.5.8`)
- `license.terms` — Tcl / BSD-style license

## Updating

To resync with upstream (pinned to `core-8-6-15`):

```
cd tests/external/tcllib/tcltest
curl -sL -o tcltest.tcl \
  https://raw.githubusercontent.com/tcltk/tcl/core-8-6-15/library/tcltest/tcltest.tcl
curl -sL -o license.terms \
  https://raw.githubusercontent.com/tcltk/tcl/core-8-6-15/license.terms
```
