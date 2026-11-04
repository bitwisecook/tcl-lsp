# Exact combined #2441 source review

This is a failing production baseline, not a product fix or a CI assertion.
The original report is retained in `issue.json`. Its complete script is:

```tcl
proc p {} {
    foreach x {} {}
    if {[info exists x]} { puts yes }
    puts $x
}
foreach y {} {}
puts $y
```

The fresh six-profile public analyser probe maintained by the registry lane
reports only top-level `y` under Tcl 8.4–9.1, and neither read under Jim. The
earlier isolated procedure witness did not verify the complete issue input.

This independent six-case Tcl 8.6 probe uses the same emitted production
library: source `f5b2f7d05ea1a322ef8f3f018e1af04394b36477` plus reviewed
joint patch `da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`;
compiler rlib SHA-256
`5d1be13ab0851b1f85a83172acaa6d1ac0465a1bd51d8db594d670546c32759c`.
The patch contains the verified index-subject and query-fixture corrections;
there is no #2441 production change. `manifest.json` hashes the retained
source, output and owner trace. The probe uses typed W210 codes and spans.

```sh
source /tmp/lsp-release-env.sh
python3 /tmp/lsp-2441-source-probe-link.py
TCL_LSP_TRACE_RBS_SOURCE_OWNER=1 /tmp/lsp-2441-source-probe \
  > /tmp/lsp-2441-source-probe.log 2> /tmp/lsp-2441-source-probe.trace
```

| Case | W210 | Procedure source |
| --- | --- | --- |
| Exact original script | `y`, span 105..107; missing `x` at 79..81 | Conditional |
| Procedure only | `x`, span 66..68 | Reached after modelled successful load |
| Top-level missing read, no guard | `y`, span 60..62; missing `x` at 33..35 | Conditional |
| Safe top-level suffix | `x`, span 66..68 | Reached after modelled successful load |
| Explicit top-level error | Missing `x` at 66..68 | Conditional |
| Actual procedure call before top-level loop | `x`, span 66..68 | Reached |

Every procedure variant retains the authentic original `$x` access, undefined
contents presence, and a semantically executable read block. The conditional
variants make the positioned opaque-callee name collector return `x`, whereas
the normal-load variants do not. The analyser includes those names in
`extra_known_defined`, which suppresses both physical and declared W210 reads.
The no-guard conditional declaration graph retains a missing-read candidate;
the extra-known-defined check still suppresses it.

The exact guarded conditional declaration graph also stops at its expression
command substitution: `expression_events` currently maps `ExprNode::Command`
to an unknown event. This makes its warning-prefix inventory unavailable. The
guard does not dominate the later read, and removing it does not restore W210.

The retained probes grant no new Native entry, source point, Normal result,
physical cell or executable admission. Repairs must preserve that separation
and must retain unknown-callee, alias and mutation controls. The declaration
graph is currently owned by the S102 work; this receipt changes no production
code. These Rust API observations are implementation evidence, not Tcl
execution evidence.
