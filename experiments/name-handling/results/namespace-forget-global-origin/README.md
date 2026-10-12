# Original global-origin namespace-forget witness

The exact #2300 setup/operation removes `n::g` on C Tcl 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0: setup and forget return code 0 and the final command
inventory is empty. The fixed adverse returns an unknown-namespace error,
keeps the unrelated imported `::n::h`, keeps real `::n::local`, and retains the
global source `::g`. A qualified forget of `::n::*` does not remove the real
local command or the foreign-origin import. C8.4 reports errorCode `NONE`;
the four newer engines report `TCL LOOKUP NAMESPACE ::missing::g`.

Pinned Jim accepts the unchanged namespace export/import setup but refuses the
actual operation because `namespace forget` is unavailable. Its adverse and
local-forget attempts reach the same unsupported-command boundary; the remaining
commands' presence is not a successful Jim forget control. No APIs were adapted.
Each provider ran once in a fresh process with a 30-second bound; all processes
exited 0, with guest errors captured separately. Binary hashes match maintained
native-provider receipts. No current Rust consumer or new source-to-build
relationship was measured, and native F5 was not executed.

Current source inspection still identifies the Runtime defect: `ns_forget`
appends `::` to storage's global qualified name `::` before appending `g`.
VM's existing forget worker instead filters the ultimate origin namespace token
and tail; that does not prove all VM lifetime behavior. Shared core has import
pattern parsing, while syntax's purpose-selected `namespace_pattern_parts`
provides the correct source-component/tail boundary.

The [question and delegation design](question.md) identifies the bounded typed
storage work needed. Runtime's `imported_in` publishes display bytes, but actual
imports retain immediate source generations and the existing native slot accessor
checks current generation membership. A direct namespace-component resolver and
an authentic ultimate-origin projection should replace display assembly; no
consumer-local root special case or display reparse is proposed. Unavailable
owners must refuse, and #1778 remains an independent replacement/lifetime gap.
No production patch, build, issue update or canonical proof edit was made.

[Original issue input](issue-witness.tcl) · [Probe](probe.tcl) · [Runner](run.py)
· [Actual measurements](result.json) · [Source inspection scope](source.json)
