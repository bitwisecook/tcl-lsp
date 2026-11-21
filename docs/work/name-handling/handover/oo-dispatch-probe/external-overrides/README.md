# overrides package fixtures

`b*.tcl` exercise the package itself: run with `TCLLIBPATH=<clone>/src tclsh9.0 b1-global-from-ns.tcl` against a clone of `Dechcaudron/overrides` at `fe18ee3`. `o1`/`o2` are usage files for the editor-feature probe (`;# CALL word` marks the queried word). `w214-reproducer*.tcl` are the false-W214 reproducer and its two non-reproducing variants. `analyser.out` holds the probe output.
