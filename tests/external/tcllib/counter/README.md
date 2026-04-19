# Vendored tcllib counter module

Source: [tcllib/modules/counter](https://github.com/tcltk/tcllib/tree/master/modules/counter)

These files are vendored verbatim from Tcllib for end-to-end WASM
execution testing (`tests/test_wasm_real_tcl.py::TestExternalTcllibCounter`).
No modifications — we want to validate that a real-world pure Tcl
module compiles and runs in our WASM VM, not a shimmed copy.

## Files

- `counter.tcl` — implementation (1263 lines, 14 procs)
- `counter.test` — upstream tests (not currently executed; awaits real
  `tcltest` compatibility in our VM)
- `pkgIndex.tcl` — package metadata
- `license.terms` — Ajuba Solutions / tcllib license (permissive, BSD-like)

## Updating

To resync with upstream:

```
cd tests/external/tcllib/counter
for f in counter.tcl counter.test pkgIndex.tcl license.terms; do
  curl -sLO "https://raw.githubusercontent.com/tcltk/tcllib/master/modules/counter/$f" \
    || curl -sLO "https://raw.githubusercontent.com/tcltk/tcllib/master/$f"
done
```
