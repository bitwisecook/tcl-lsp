# phi_can_undef exponential-path reproducer (issue #2021)

`gen.py <variant> <N> [--no-read] [--no-init] [--proc]` emits a Tcl script with
N sibling conditional writes to one variable followed by a read. On
`tcl diag` the W210 phi-from-undef trace
(`rust/tcl-compiler/src/analyser/diagnostics/helpers.rs::phi_can_undef`)
enumerates every simple path through the phi graph (`seen` is a path set, no
memo), so `nested_if` grows ~2^N, `nested_if3` ~3^N, `switch_empty5` ~4^N.
`switch_empty5_n12.tcl` (137 lines) takes ~60 s; N=11 takes ~10 s.

The blowup needs the read (dead writes are dropped before SSA) and a
dominating initial definition: when the variable really can be undefined the
first undef path short-circuits, so the analysis is fast on the buggy input
and exponential on the correct one.

`shape.py` counts, per proc, the largest run of sibling conditional writes to
one variable; the Quartus `alt_xcvr/.../nf_hssi_*_pld_pcs_interface_*.tcl`
files reach 105 (`legal_values`), i.e. ~2^105 paths.

`fix_sketch.rs` sketches the fix: memoise path-independent answers (only when
no cycle cut was used below the node) and hoist the per-name startup facts
out of the recursion.
