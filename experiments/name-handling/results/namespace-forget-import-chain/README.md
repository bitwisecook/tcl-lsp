# Original import-chain namespace-forget controls

The same fixed input completes setup and all five operations on pinned C Tcl
8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Qualified patterns remove the final
import when they match the ultimate source or the immediate first source.
Matching another middle hop preserves the final import. When ultimate and
immediate sources share a namespace but have different tails, the ultimate
tail controls the match: the immediate tail does not trigger a fallback.
An unqualified pattern removes a renamed imported binding by its current own
name. Source and other imported commands survive the selected deletions.

Pinned Jim accepts the unchanged setup, then each original forget operation
reports that the subcommand is unavailable. This is an operational refusal,
not a successful forget control; no API is substituted. Each provider ran in
a fresh bounded process, exited 0, and retained caught guest errors separately.
Binary hashes were checked against maintained provider receipts. No Tcltest
harness was loaded. Native F5 remains unexecuted.

The Runtime source uses shared purpose-selected pattern parts, actual context
components, direct namespace tokens and authentic imported generation slots.
The qualified source selection follows the ultimate namespace first, then the
immediate source only on a namespace mismatch. No display name is reparsed.
This native capture does not measure Rust consumer execution. The separate
[Runtime validation](../namespace-forget-global-origin/runtime-validation-01/README.md)
passed the fixed Native C8.6 controls and retained #2300 witness/adverse controls. Broader replacement and teardown lifetimes (#1778)
remain unproved.

[Question](question.md) · [Fixed original input](probe.tcl) · [Runner](run.py)
· [Actual native results](result.json) · [Inspected source](source.json)
