# Native namespace alias settlement

The counted source exercises a traced dynamic local alias, an array-shaped
declaration over a scalar root, and a write callback that replaces the
namespace target before the local link is installed. The output files contain
actual Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 results.

`probe.c` evaluates `source.tcl` in a fresh native interpreter using
`Tcl_EvalEx`. The Runtime control uses the same source and an authentic
per-version native constructor; its host captures standard output.
