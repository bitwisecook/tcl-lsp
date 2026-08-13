# Legacy duplication audit — August 2026

> **TEMPORARY — DELETE THIS DIRECTORY WHEN EVERY ISSUE LISTED BELOW IS CLOSED.**
>
> These are working notes, not documentation. They exist only to carry detail
> that did not fit into the issues they back — verified-clean lists, rejected
> candidates, and the full reasoning behind each finding. Once the issues are
> closed the notes are stale by definition: they describe code that no longer
> exists, and a future reader cannot tell which parts were fixed. Delete the
> whole directory, including this README, rather than trying to prune it.
>
> Tracking issue: **#1400**. When #1400 closes, this directory goes with it.

## What this is

Six read-only audits run in parallel after PR #1371, which found the
optimiser's redundancy family answering one semantic question — "can Tcl's
mutable dispatch machinery observe this call site?" — in two places: a legacy
command-string classifier and a modern typed per-site proof. GVN was migrated
to the proof; PRE and LICM were left on the legacy classifier, which was
unsound.

Each audit hunted the same shape in a different subsystem: a shared authority
exists, most consumers adopt it, one or two do not, and nothing detects the
stragglers.

Every finding cites `file.rs:line` on both the modern and the legacy side.
Four of the six audits additionally reproduced their findings against a
locally built `tcl` binary; those reports quote the command and its output
inline.

## Reports and the issues they back

| Report | Scope | Issues |
|---|---|---|
| [`optimiser.md`](optimiser.md) | Optimiser passes and the analyses feeding them | #1374, #1377, #1385, #1392 |
| [`analyser.md`](analyser.md) | Analyser, semantic model, registry compliance | #1378, #1379, #1381, #1388, #1389, #1390, #1391 |
| [`lowering.md`](lowering.md) | Parsing, CST, segmenter, IR, CFG, SSA, codegen | #1375, #1376, #1380, #1393 |
| [`lsp.md`](lsp.md) | LSP feature providers | #1386 |
| [`runtime.md`](runtime.md) | `runtime/rust`, `tcl-vm`, `tcl-cmd-core`, parity gate | #1382, #1383, #1384 |
| [`tooling.md`](tooling.md) | CLIs, MCP, xtask generators, editor integrations | #1387, #1394 |

Finding IDs inside each report (`F1`, `F2`, …) are local to that report and
are referenced from the issues.

Issues **#1395**, **#1396**, and **#1397** are carried forward from PR #890
rather than found by these audits, and **#1398** and **#1399** come from
#1371's own "known limitations" — none of those five is backed by a report
here.

## What these notes contain that the issues do not

- **Verified-clean lists.** Each report records the subsystems checked with
  nothing to report, and why. That is the part most expensive to reproduce
  and most useful to a future auditor deciding where to look.
- **Rejected candidates.** Findings considered and dismissed, with the reason
  — including AGENTS.md's documented carve-outs (the lowering
  fallback-to-generic-call pattern; the irreducible analyser-local TclOO
  semantics) and debt already tracked at its call site, such as
  `alias.rs:201-223`.
- **Confidence and scale estimates** per finding.

## Caveats

These are agent-produced audit notes reviewed for evidence quality, not
maintained documentation. They were accurate against the tree at
`0f97d98` and are not updated as the code moves. Where a report and the code
disagree, the code is right.

One finding is explicitly scoped as a hazard rather than a live defect:
`first_positional_index`'s five hand-rolled copies (#1395) were verified to
exist, but no input was found where they currently produce a wrong answer.
The live version of that fault is #1378.
