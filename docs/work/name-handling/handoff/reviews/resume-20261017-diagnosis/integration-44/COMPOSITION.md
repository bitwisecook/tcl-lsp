# Integration44 candidate (not built; not admitted)

Base: 6f01dc10c (integration43 candidate 21a199ad8 plus docs). Pieces, applied in order (hashes in SHA256SUMS):

| Piece | Review | Verdict |
|---|---|---|
| `i43-fix.patch` (Sol-17: Logical survey walks own conditions before body catalogues; Logical positive control) | `../luna-24/REVIEW.md` | ADMIT-WITH-CONDITIONS |
| `compiler221-v2.patch` (assertion corrected to tclsh 8.6/9.0/9.1; proof doc updated) | `../luna-20/`, `../luna-22/` | ADMIT-WITH-CONDITIONS |
| `vsatisfies-84-v2.patch` (8.4 multi-requirement arity raises; three-state arity from registry forms) | `../luna-20/`, `../luna-22/` | ADMIT-WITH-CONDITIONS |
| `forced-import-assertion-v2.patch` (assertion corrected to tclsh 8.4-9.1; query offset moved onto the call) | `../luna-20/`, `../luna-22/` | ADMIT-WITH-CONDITIONS |
| `lone-cr-document-channel-v3.patch` (C Tcl `source -translation auto` lone CR; `SourceLineEndings` dialect axis, Jim `Preserve`) | `../luna-21/`, `../luna-23/`; v2->v3 delta checked by the coordinator against Luna-23's six conditions | ADMIT-WITH-CONDITIONS |

`cargo fmt --all --check` passes on the composed tree.

## Recorded corrections

- Assertions corrected to tclsh (rule of 2026-10-17 / 2026-10-18T11:55Z): Compiler221 `is_none` -> `is_some`; forced-import `hit.is_none()`; lone-CR: `cr_is_separator_not_eol` (Document expectation added, native-value expectation kept), completion `("\r", None)`, `original_oo` `("\t\r", false)`.
- Test-input correction (separate from the tclsh corrections): the forced-import test's query offset moved from the closing `}` (selected nothing; vacuous) onto the `p` head word.
- Rewritten test: `c_source_lookup_refuses_fall_through_past_condition_created_command` (introduced by 21a199ad8, never admitted) now checks the Logical survey with three tclsh-checked assertions.

## Obligations carried (from reviews)

- V2-2 (Native C-source fall-through refusal past a possible occupant) is **uncovered**; the renamed test above is not C-source coverage (Luna-24 C1).
- Follow-ups present at HEAD, not caused here: per-arm `if` conditions (an earlier arm's body runs before a later elseif condition); `for` start-script and loop-carried effects not threaded into body catalogues (Luna-24 C2).
- Raw-byte scanners still read lone CR as whitespace on unnormalised text; `^Z` EOF; per-dialect issuer normalisation; Jim backslash-CR gating (lone-CR v3 contract text).
- `arity_accepts_under` ignores per-form `literal_argument_prefix`/`options` (none on vsatisfies forms) (Luna-22 recommendation).
- The `resolve_called_proc` tail after the lexical-advice branches is dead code (Luna-22 (a)); sibling forced-import offsets are decided (user 16:00Z) and drafted separately.
- Owed validation: the never-run later Compiler221 assertions, `make check-name-resolution-proofs`, tcl-dialect/tcl-lexer/tcl-spectcl/registry SpecTcl tests, reachability/package_/package_target_survey tests, nested-wrapper timing, `make rust-check`, `prep-pr` (clippy deferred to PR stage per 2026-10-18T07:30Z).

## Amendment after targeted run 45a (build45)

Targeted 45a: 1218/1338; against integration43 binaries 117 of the 120 failures pre-exist. Two PASS->FAIL (lone-CR corrected rows in completion argument slots and OO selector gaps) are fixed by `sol-24-completion-gap.patch`: `instance_completion::completion_gap` reads bytes through `SourceChannel::script_byte` with the grammar's `source_line_endings` (Jim `Preserve` unchanged). Coordinator review: reads through the shared owner, continuation logic unchanged, no names hardcoded. Sol-24's runs: zero PASS->FAIL against 45a across lexer, registry, compiler survey/source-structure and core completion/oo/signature/call-hierarchy/package_resolver.

The new test `a_multi_requirement_vsatisfies_raises_on_tcl_8_4` still fails (Conditional): the package-index survey's Logical realm never enters `if` conditions, so no guard receipt exists (the same pre-existing cause as ~15 reachability failures). With Sol-23's survey v1 applied temporarily it passes, as does the compiler receipt test; that v1 also overflows `pathological_nesting_terminates` on a 2 MiB stack, so survey v2 is required. Returning Unavailable without the receipt would be absence-as-completeness.
