# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T12:13Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- Run 4, a single command that builds, runs and deletes, started at 12:15Z:
  - the seam probes;
  - the scaling rows, re-sized to P = 25–200, because run 3 measured 248 s just to analyse a 250-proc file;
  - the existing `pack_source_e2e::a_pack_file_folds_a_call_site_in_the_optimiser` test, to check whether live const-fold hooks fold at all.

## Outstanding (in priority order)

1. Interpret run 4 and settle the N17 live hook, the seams and section 4.1.
2. Finish `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md`. The summary table is drafted; the details table is still to be written.
3. Delete `rust/tcl-spectcl/tests/zz_review_probe.rs` (in snapshot 2fe1d48d97) and the scratch files.

## Blocked (on whom, and what is needed)

- Nothing. Run 3 was SIGKILLed during `scaling_rows` when the usage limit stopped the session; its other tests had already reported.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Run 3 (11:28Z–12:06Z) re-ran rename with a registry and a program export view, as the server's `rename_in_program` call is made:
  - **N04 and N05:** REFUSED ("unknown-completed-command-world").
  - **N06:** class, method and proc→class renames are REFUSED with specific collision reasons. The local→sibling rename is REFUSED (incomplete coverage), and the workspace rename paths are disabled (`allows_lexical_rename_advice=false`).
  - **N07:** REFUSED ("unreferenced-edit").
  - **Controls:** `proc foo; puts [foo]` and `set x [foo]` renames are REFUSED (incomplete coverage). Only a bare `foo` call renames, and all renames are refused under Jim. Rename is therefore broadly refusing supported static cases.
  - **N11:** the `if {0}` provider is classified `Conditional`, not `Unavailable`, so p2 is still selected alongside p1.
  - **N17 control:** the shipped-style `mylib::strlen` hook also emits no O129, so live const-fold hooks do not fold via `optimise_raw` at all.

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
