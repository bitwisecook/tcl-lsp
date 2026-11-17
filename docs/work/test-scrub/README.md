# Test scrub

Started 17 November 2026 at the owner's request: find what the tests hold, where they overlap, and
which of the failing ones are worth making pass. Value is counted in behaviours, not in tests.

## Inventory

`tools/scrub_inventory.py` reads every `#[test]` under `rust/` and `runtime/rust/` without compiling
and writes `data/` (about 20 seconds):

```bash
python3 docs/work/test-scrub/tools/scrub_inventory.py --root . --failing docs/work/name-handling/lists/*-2026-11-17.txt --out docs/work/test-scrub/data
```

- `data/summary.md`: counts by class, crate and family.
- `data/inventory.tsv` (not committed, 8 MB): one row per test with its file and line, whether the
  lists mark it failing, its class, the codes it names, how many Tcl sources it runs, and the groups
  it falls in.
- `data/clusters_exact.tsv`: tests whose bodies are the same text.
- `data/clusters_overlap.tsv`: tests that run the same Tcl sources through the same helpers against
  the same codes. A lower bound: the same check written through a different helper is not caught.
- `data/fixtures_shared.tsv`: Tcl sources used from more than one file, the wider net.

Classes are read from the test's text, so they sort the work; they are not verdicts. **A** runs Tcl
source and cites tclsh, Jim or a release. **B** runs Tcl source and cites an issue, a contract or a
KCS page. **C** runs Tcl source and cites nothing. **D** runs no Tcl source: it holds an internal
structure or API. A differential test that runs tclsh itself and says so nowhere reads as C.

Its own tests: `cd docs/work/test-scrub/tools && uv run --with pytest python -m pytest -q`.

## First reading, 17 November 2026

37,567 tests; 3,743 marked failing by the morning's lists (about 150 have been fixed since).

| Class | Tests | Failing |
| --- | ---: | ---: |
| A, cites an oracle | 4,045 | 564 |
| B, cites an issue or contract | 2,962 | 497 |
| C, runs Tcl, cites nothing | 18,182 | 2,333 |
| D, internal structure | 12,378 | 349 |

- Six failing tests in ten state an expectation about Tcl source with nothing cited behind it. Each
  is either a behaviour to confirm on tclsh, a contract to cite, or an expectation nobody can defend.
- 179 tests in 88 groups are word-for-word copies of another test.
- 844 tests in 385 groups repeat another test's sources, helpers and codes; 157 groups span files
  (the same check in a unit test and again in `tests/`).
- 4,313 Tcl sources are used from more than one file.
- 12 overlap groups have members that pass and members that fail on the same input.
- The families with the most failures: `tcl-compiler` `analyser` (408 in the library, 112 in `it_4`),
  `command_binding` (283), `optimiser` (147), `it_2/value_transfer_witnesses` (109).

## Triage

Each area session writes a ledger for the failing tests of its families before fixing anything else,
`ledger/<area>.tsv`, tab-separated:

```
crate	target	path	verdict	cause	evidence	survivor
```

`cause` is a short slug shared by every test that fails for the same reason; `evidence` is the tclsh
or Jim release and output, the contract or KCS path, the ruling, or the commit; `survivor` is the
test a merged one folds into.

| Verdict | Meaning | What happens to the test |
| --- | --- | --- |
| `KEEP-ORACLE` | It holds behaviour tclsh or Jim shows, confirmed on the releases named | Stays; the product is fixed |
| `KEEP-F5` | It holds iRules or iApp behaviour no Tcl core can show | Stays; a row goes in the BIG-IP check document |
| `KEEP-CONTRACT` | It holds product behaviour a contract, KCS page or ruling states | Stays; the product is fixed |
| `FIX-EXPECTATION` | The oracle or the contract contradicts what it expects | The expectation is corrected, with the evidence |
| `FIX-FIXTURE` | The source is not what the test says it is | The source is corrected |
| `MERGE` | It holds the same observable on the same input as the survivor | Deleted once the survivor asserts at least as much |
| `REWRITE` | The behaviour is real but held through an internal detail | Re-pointed at the observable |
| `RETIRE` | It holds an internal detail or a superseded design, and nothing observable or stated stands behind it | Deleted, with the reason |
| `UNDECIDED` | The session cannot tell | The coordinator rules |

Rules:

- No test is edited or deleted during triage. The ledger comes first.
- "It fails and is hard to fix" is never a reason to retire. A retirement says what the test asserted
  and why nothing observable or stated depends on it, or names the ruling that superseded it.
- A merge needs a named survivor that asserts at least what the merged test asserts.
- A class C test that is kept gets its citation: the tclsh rows or the contract path go into the test.
- The coordinator samples every ledger before anything is deleted. `RETIRE` and `MERGE` then land in
  commits of their own that name the ledger rows, never mixed with a fix.
- Each area ends with `<area>.md`: counts by verdict, the causes ranked by tests blocked, and the list
  of distinct behaviours that are failing. That list is what is worth fixing.

The second pass, after the failing tests, takes the passing ones through the exact and overlap groups.

## Families by session

`tcl-compiler` is split by family (crate, target, first modules); a family not listed goes to the
session whose area note names it, and analyser-binding decides the rest.

| Session | Families |
| --- | --- |
| diagnostics | `tcl-compiler` `analyser::diagnostics`, `analyser::handlers`, `analyser::param_traits`, `analyser::state`, `analyser::types`, `analyser::bounds_checks`, `analyser::scope`, `compiler_checks`, `it_4/analyser`, `it_5/checks`, `it_3/fp_depth`, `it_2/core_analyses` |
| analyser-binding | `tcl-compiler` `command_binding`, `unit_scope`, `compilation_unit`, `inline_uplevel`, `dynamic_names`, `deferred_writes`, `word_subst`, `common_aot_plan`, `native_lowering`, `cfg_builder::upvar_info` |
| class-facts | `tcl-compiler` `registry_invocation`, `lowering`, `analyser::oo`, `analyser::original_members`, `object_types` |
| dataflow-optimiser | `tcl-compiler` `optimiser`, `sccp`, `ssa`, `cfg_builder::tests`, `cfg_builder::execution_regions`, `interprocedural`, `native_integer_proof`, `it_3/optimiser`, `it_3/optimiser_depth`, `it_3/side_effects_binding`, `it_5/optimiser_coverage`, `it_5/pipeline_coverage` |
| values | `tcl-compiler` `it_2/value_transfer_witnesses`, `cfg_builder::global_write_info`, `shimmer`, `type_infer`, `value_transfer`, `it_2/var_escape_cfg`, `it_5/var_escape_cfg_catch` |
| taint | `tcl-compiler` `taint`, `taint_interproc`, `irules_checks`, `analyser::irules_event_checks`, `uri_split`, `connection_scope`, `it_3/taint`, `it_4/taint_depth` |
| analysis-performance | `tcl-compiler` `analyser::per_item`, `per_item_corpus`, the performance tests |
| lsp-features | `tcl-lsp-core`; the navigation groups of `tcl-lsp-server` |
| server-e2e-db | `tcl-lsp-server` (end to end and library), `tcl-lsp-db` |
| execution | `tcl-codegen`, `tcl-vm`, `tcl-runtime`, the engines, `tcl-cshim`, `tcl-spec-hooks` |
| irules-registry-tooling | `tcl-cli`, `tcl-spectcl`, `tcl-diagram`, `tcl-explorer`, `tcl-mcp`, `tcl-irules`, `tcl-spec-studio`, the F5 crates |
| tk-oo | the tests its issues name |
| live-inlining | `tcl-spectcl` `codegen_stamps` |
