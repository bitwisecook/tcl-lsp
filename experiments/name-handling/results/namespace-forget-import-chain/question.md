# Qualified forget: ultimate and immediate imported sources

Does the original namespace-forget operation select the ultimate source's tail
when its namespace matches, otherwise the immediate first source's tail, while
leaving other intermediate hops unmatched? Does an unqualified pattern select
the imported binding's current own name after rename?

The fixed input creates three independent four-hop chains, one same-namespace
ultimate/immediate tail mismatch, and one renamed destination import. It runs
unchanged on each pinned C Tcl 8.4–9.1 and Jim shell. Guest setup/operation
refusals remain results; no alternative Jim API or adapted input is used.
The original #2300 evidence remains unchanged. Native F5 remains unexecuted;
no Tcltest harness is loaded. These controls do not establish broader import
replacement or command/namespace teardown lifetime conformance (#1778).
