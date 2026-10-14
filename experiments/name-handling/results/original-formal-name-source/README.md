# Original formal-name source forms

[probe.tcl](probe.tcl) defines the unchanged four parameter-list forms in fresh original C Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and latest checked Jim processes. [results.json](results.json) preserves exact source/executable hashes, original C init.tcl identities, provider identity queries/Jim source revision, argv, stdout, stderr and exit status. No Tcltest or replacement init.tcl is used.

The actual `info args` reports differ by provider:

| Input form | C8.4–9.1 report | Jim report |
| --- | --- | --- |
| `first {"as" ""} {third value}` |`first as third`|`first {"as" ""} {third value}`|
| Quoted parent `other {last default}` |`other last`|`other {last default}`|
| Escaped parent `first\x20second` |`first second`|`first second`|
| Quoted marker `"as" value extra` |`as value extra`|`"as" value extra`|

C's reports expose decoded names; Jim's reports retain formal specifier/quote spellings in these cases. An `info args` presentation is not a generic stored-cell-name API. Preserve the distinct raw answers and the independently selected formal grammar rather than equating provider reports.

Repeated definitions remain separate original source sites in the source-geometry controls; the final live p does not expose earlier declarations. These finite observations establish decoded argument names only. W214's quoted-keyword suppression is a reporting convention, not interpreter keyword semantics.

Compiler conformance controls retain exact declaration sites/current input and shared formal/list rules. Native metadata uses existing original formal topology. Logical metadata owns an independently selected authored source schema and original whole parameter word; it does not fabricate Native topology. Tight source extents are readonly, and escaped/unmapped parent values withhold coordinates. No actual runtime frame/cell, Native compilation, edit authority or F5 behavior follows from these observations. The [F5 packet](../../f5/questions/original-formal-name-source.md) is unanswered.
