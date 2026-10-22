# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T11:35Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- Probe run 3 is queued. Its build waits until `df` shows at least 2.4 GB free (it was at 1.2 GB at 11:35Z). It covers:
  - Rename probes re-run through `rename_in_program` with the registry and the program export view, as the server calls it. Run 2 passed no registry, so its rename refusals may come from that omission.
  - Plain-rename controls with and without `puts`.
  - The N17 live-hook control: the shipped `mylib::strlen` fold. No O129 appeared even for `fold good`, so it is unclear whether hook folding works at all.
  - N11 per-provider availability.
  - The seams: nested passthrough lowering for iRules, extract/inline, linked editing.
  - The section 4.1 scaling rows.

## Outstanding (in priority order)

1. Run 3: interpret it and settle N04, N05, N06 and the N17 live hook.
2. Write `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md`.
3. Delete the probe file `rust/tcl-spectcl/tests/zz_review_probe.rs` and the scratch files.

## Blocked (on whom, and what is needed)

- Shared disk is at 1.2 GB. The next build waits for at least 2.4 GB, per the coordinator's instruction to wait for more than 2.3 GB.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Run 2 (11:20Z) of `zz_review_probe` executed 14 tests. It was stopped during `scaling_rows`, which was too slow with per-file analysis of 1,000 documents. The probe has since been simplified. Settled statuses:
  - **N01:** REPRODUCES. The two C Tcl 8.6+ slots collapse into one `::a:::p` entry; no false diagnostic now. The empty-tail alias target `::n::` is preserved (FIXED).
  - **N02:** CHANGED. Jim keeps `::a::p` and `::a:::p` distinct, but definition abstains from both calls.
  - **N03:** FIXED. C Tcl definition, references and rename select `caf`, and the edit runs `Aé` on C Tcl. Jim selects `café`. Nested braces resolve to `a{b` on 8.6 and `a{b}` on 9.0. A cursor on a literal suffix returns nothing.
  - **N08:** FIXED. Compilation-unit SCCP for `::n::c` contains neither `GLOBAL` nor `LOCAL` constants.
  - **N09:** constructor typing FIXED: it gives `Object(::n::Foo)`, and a nearer proc `::n::Foo` blocks the global class. The call graph is CHANGED: `::n::c` has no interprocedural summary, while the analyser resolves `::lib::p`.
  - **N10:** REPRODUCES for path-resolved `C new`: no hover, type hierarchy or definition. FIXED for the relation site (`::A::Base`) and for the missing qualified class (no supertype).
  - **N11:** `auto_qualify` Unicode FIXED. `resolve_require` still returns both p1 and the `if {0}` p2; per-provider availability is pending in run 3.
  - **N12:** FIXED. The replaced index answers `true`, the same as a fresh index; the oracle prints 0 then 1.
  - **N13:** REPRODUCES. The seeded child resolves `helper` to `::app::helper`, while the oracle gives `good` via `::lib::helper`.
  - **N14:** semantic tokens FIXED: `plain` is a string token.
  - **N16:** all four rows FIXED: signature help, alias hover, type definition, completion.
