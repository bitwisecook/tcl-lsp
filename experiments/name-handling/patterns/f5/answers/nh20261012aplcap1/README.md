# Measured APL presentation capabilities: nh20261012aplcap1

The retained browser form selects the owned template and renders 17 APL
`choice ... tcl` callbacks. Its implementation action is empty. The screenshot,
DOM, [browser-options.json](browser-options.json) and invocation metadata
retain the actual presentation action. These are APL results, independently
of iApp implementation execution.

Each callback returned captured rows as equal label/value pairs. The browser
stores that callback return as a Tcl list inside its option value. Independent
Tcl `splitlist` parsing of each option value, then each two-element equal pair,
reproduces [invoke.stdout](iapp-apl-0/invoke.stdout) exactly: 80 distinct NHO
rows across all 17 controls, with no missing or duplicate captured chunks.
The projection preserves encoded bytes and supplies no extra runtime result.

[ANSWERS.md](ANSWERS.md), [records.json](records.json) and
[reviewed-proofs.json](reviewed-proofs.json) retain 17 reached observations:
10 completion 0 and seven completion 1. The version control returns
`8.4.6 8.4 ::script`; [interpreter-facts.json](interpreter-facts.json) records
that control and its raw-source digest. The seven errors remain actual
operation errors, including the tcltest load's missing `debug` variable;
none is automatically classified as semantic unsupported behaviour.

Each callback records setup completion 0 and runtime cleanup completion 0
with result hex `30207b7d`. Ownership is proven by the absent-before receipt;
the template's post-delete receipt finds the same exact object absent. Source,
actual input, wrapper and metadata hashes and materialised callback source
were checked. The retained Python capability and APL wrapper sources are
authored template provenance, not portable Tcl originals. No result is
inferred for other execution contexts or appliances.
