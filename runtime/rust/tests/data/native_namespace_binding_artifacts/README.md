# Native namespace binding artifacts

The five tables describe C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
compiled procedure bodies. `probe.c` retains the original procedure body and
inspects its instruction counts, counted local names and literal array after
calling the procedure. Byte sequences use hexadecimal; literal types are
recorded before their string getters run. `manifest.json` identifies the exact
source, headers, static libraries, commands and output hashes.

The bodies exercise a namespace compiler's accepted prefix followed by a
non-compilable variable tail. The prefix's local declarations and literal
allocations survive the declined attempt. Generic command preparation then
visits the original children again. In Tcl 8.6 and later, the two visits to a
constant `list` manufacture separate private list headers. The runtime tests
compare the complete literal order and local layout, check list identity and
require the declined attempt to contribute no executable namespace bindings.

The shared namespace binding fixtures in
`rust/tcl-registry/tests/data/native_namespace_bindings` cover the complete
20-body instruction and local-layout matrix, including empty names, qualified
tails, expansions and child local declarations.
