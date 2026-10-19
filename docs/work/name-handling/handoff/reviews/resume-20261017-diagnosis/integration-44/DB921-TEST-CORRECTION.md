# DB921 test correction (decision-forced; not admitted)

**Patch.** `db921-merged-v2.patch` (Sol-22), on base `19b8d7cc7`.

**Reviews.** `../luna-20/REVIEW.md` §2 and the Luna-25 review.

**Status.** Not built and not admitted.

## Decision and C behaviour

**User decision (2026-10-18T16:00Z,** `../user-decisions-20261018T1600Z.json`): DB921 uses the merged-unit design. There is one analysis unit per evaluated `proc`, and it carries both the readonly authored lowering and the Installed lowering.

**C Tcl behaviour.** One evaluated `proc` creates one command. The probe is [`probes/db921.tcl`](probes/db921.tcl), and its output on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 is [`probes/db921.out`](probes/db921.out). On every release the fixture adds exactly `::sample` (code 0), and one site that executes two bodies adds one command, `::p`, whose final body is the second one.

## What changes in the tests

The three `readonly_authored_coverage` tests in `rust/tcl-compiler/src/body_analysis_capsule/readonly_authored_coverage.rs` were written for the separate-unit layout. They use the fixture `proc kept {x} {…}; proc other {} {…}` under tcl8.6.

**Old expectations:**
- `installed_procedure_body_units.len() == 2`
- `original_authored_declaration_body_units.len() == 2`
- `body_units.len() == 4`, on both the Module and the CompilationUnit
- each Installed function stored in `body_units`

The merged design necessarily changes the unit count. The correction is the narrowest one possible:

| Assertion | Before | After | Why |
|---|---|---|---|
| `ir_module.body_units.len()` | 4 | **2** | One unit per evaluated `proc` (decision-forced). |
| `unit.body_units.len()` | 4 | **2** | Same reason. |
| Lowering count | (implicit in the 4) | `module.body_lowerings().count() == 4` and `unit.body_lowering_functions().count() == 4` | The four lowerings are kept and still asserted. |
| Carried stores | — | `carried_installed_lowerings.len() == 2` on both the Module and the CU | Added. The Installed lowerings stay present. |
| Installed lookup | `body_units[&installed]` | `body_lowering(&installed)` and `carried_installed_lowerings[&installed]` | The Installed lowering now lives in the carried store. Its `executed_source.is_some()` is still asserted. |
| Mutation targets in refusal tests | `body_units` | `carried_installed_lowerings` | The same mutations, retargeted to the store that now holds the Installed lowering. |

**Unchanged:**
- both inventories of 2;
- `executed_source` is `None` on the authored lowering and `Some` on the Installed lowering;
- the authored and Installed IDs are distinct;
- every refusal: duplicated owner, three config axes, foreign input, changed source, missing authored, swapped allocation, foreign `params_raw`, `body_id`, foreign registry.

**Added:**
- A refusal for a carried lowering re-pointed at the wrong authored unit.
- `carried_lowerings_are_exactly_the_readonly_authored_pairs`. It checks four things:
  - un-carrying restores the old 4-unit coverage;
  - `pairs` re-derives the recorded carriers;
  - a second Installed body at the same site, even a refused one, blocks the carry (fail-closed);
  - carrying again reproduces the built unit.

**The DB test is not changed.** `tcl-lsp-db/src/lib.rs` `procedures + body_units == 1` (with `checked == 1`) is expected to pass once the single `proc` is carried. Its lexer config is `from_grammar`, whereas the coverage fixture uses `for_file_grammar`, so it must be measured.

## Owed measurements

The decision requires measured runs of:
- coverage
- `interprocedural`
- `object_types`
- `source_graph`
- SSA
- the DB test

Also owed: the Explorer coverage test (pre-existing red rows at HEAD, not caused by DB921), `tcl-diagram`, `tcl-lsp-core` graphs, `tcl-spec-studio` infer, `cargo xtask owner-resolution` (new `ReadonlyAuthoredCoverage::pairs` row), a large standalone performance fixture, and `make rust-check`.
