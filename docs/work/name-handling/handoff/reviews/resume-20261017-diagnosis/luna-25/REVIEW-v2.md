# Luna-25: delta review of Sol-22 `db921-merged-v2.patch`

**Scope.** I ran no cargo, made no repo writes and created no worktrees.

**Apply check** (`git apply --check --cached` under a temporary `GIT_INDEX_FILE`):

| Target | Result |
|---|---|
| Base `19b8d7cc7` | OK |
| Current HEAD `31740e1e2` | OK |

The working tree's foreign edit to `tcl-lsp-core/.../instance_completion.rs` is untouched and outside the patch.
- `p2/a` matches HEAD for all 37 existing files.
- The 38th file, `DB921-TEST-CORRECTION.md`, is new.
- `diff -ruN p2/a p2/b` reproduces the patch.

**Interdiff method.** The v1→v2 interdiff compares `p/b` against `p2/b`. `p/a` and `p2/a` differ only by the newly included implementer guide.

**Verdict: ADMIT-TO-BUILD.**

D1, D2 and D3 are fixed correctly, with results identical to v1 apart from D1's intended extra refusal. Two small items should be fixed before landing (E1, E2). Neither affects compiling or measuring.

## 1. D1: sole-entry guard. Correct, and the test exercises it.

**The guard** is at `relate` `:164-178`, and runs in `Pairs` mode only.
- `at_site` counts every value of `module.installed_procedure_body_units`, whether its lowering was admitted, refused or missing from the stores.
- A pair survives only if its Installed allocation site has a count of exactly 1.
- `CommandAllocationSite` derives `Ord` and `Eq`. The manual impls on `SourceOriginId` and `SourceImage` are mutually consistent, so map keying agrees with `==`.
- `Complete` (`capture`) is unchanged.
- The two-proc coverage fixture and the single-proc DB fixture each have one entry per site, so they still carry.

**The new test genuinely exercises the guard.** It re-tags `installed[1]` with `installed[0]`'s full allocation.
- `installed[1]` is then refused: `matches_installed_row` fails because its `declaration_site` is not row 0's site.
- `installed[0]` still classifies fully: one row for its allocation, `installed_procedure_cfg_for_body` passes on membership alone, and it covers A0.
- It is not `shared`, because only one Installed lowering was admitted.
- **Without the guard, `installed[0]` would be carried into A0. With it, `source_for(installed[0])` is `None`.** No other check blocks it, so the test is not vacuous.
- The guard is keyed by site, so it also covers a refused entry with a different allocation at the same site.
- tclsh confirms the scenario is real: sol-18's `probes/db921.out` records "one site, two executed bodies".

**Optional.** Also count `original_declaration_body_units` at the same site. This needs no change for the decision.

## 2. D2: precomputed `RelationInputs`. Results are identical.

**Original-body set.**
- `original` is `original_source_procedure_bodies().map(id).collect()` over the same immutable Module. Membership is identical to the old per-call `.any(|(body, _)| *body == id)`.
- `original_procedure_cfg_in` and `original_procedure_cfg_for_body` now both end in `prepared_original_cfg(id)`. That is the old tail moved verbatim: `procedures.contains_key` gives `cfg_module.procedures`, otherwise `source_body_cfgs`.
- `original_procedure_cfg_for_body` keeps its own membership derivation, so its public behaviour is unchanged.

**Rows by allocation.**
- `rows` is a `BTreeMap<&CommandAllocation, Vec<&row>>`, and `let [record] = …as_slice() else { return None }` requires exactly one row.
- That is equivalent to the old filter, `next()?` and `next().is_some()` refusal. `CommandAllocation` derives `Ord`, consistent with `Eq`.
- `get(allocation)` resolves through `Borrow<T> for &T`. `&&row` deref-coerces at the call sites.

**Authored cards by site.**
- `authored` groups card IDs by `card.site()` in ID order, which reproduces v1's site-filtered scan. `covers_installed_analysis_source` already requires site equality.
- Candidate uniqueness is unchanged.

**Shared detection.** `!sources.insert(source)` matches the old `installed_sources.values().any(..)` test before insertion.

**Cost.** `matches_procedure` now runs O(n) times per relation: once per card for the set, once per authored classification, and once per same-site candidate. The quadratic term is gone.

## 3. D3: lazy merge. Ordered, with no drops and no duplicates.

- Both sources are ascending by `ProcedureId`: `ProcedureInventory` and `CarriedInstalledLowerings` are both `BTreeMap`s.
- The merge emits a carried element first only when it is strictly smaller. A tie emits the unit element first and then the carried one.
- That is exactly what v1's stable `sort_by` produced over the chain, which put units before carried.
- Each `next()` advances exactly one side, and `None` comes only when both are exhausted. Nothing is dropped, and an ID present in both stores (an invariant break) is emitted twice, as in v1.
- Lifetimes: the `move` closure holds peekables borrowing `&self`. The comparison is between two `&&ProcedureId` values. It compiles.

## 4. D4: doc text and `DB921-TEST-CORRECTION.md`

**Implementer guide `:3904-3914`: accurate.**
- It states the post-build move, the `ReadonlyAuthoredCoverage` / `covers_installed_analysis_source` source of pairs, and the sole-Installed-at-site condition.
- It lists the retained identities, the unit count, and the three accessors that reach both lowerings.
- It sits right after the Installed-tagging paragraph, which it qualifies.

**`architecture.md`:** the paragraph is reflowed correctly.

**`DB921-TEST-CORRECTION.md`:** the assertion table, the unchanged and added lists, and the DB-test note all match the code. The relative links `../luna-20/REVIEW.md` and `../user-decisions-20261018T1600Z.json` resolve.

**Defects found while checking D4:**

| ID | Severity | Item |
|---|---|---|
| E1 | fix before landing (trivial) | `shared-utility-contracts-rust.md`: the `ReadonlyAuthoredCoverage::pairs` row is added **twice**, at patch lines 66-67, and `p2/b` has 2 copies. `owner-resolution` collects rows into a set, so the gate likely won't fail, but the duplicate is a copy error. Delete one. |
| E2 | fix before landing (minor) | `DB921-TEST-CORRECTION.md` says "The probe is recorded in `sol-18-NOTES-v2.md` in this directory (tclsh 8.4–9.1)". The notes only cite `probes/db921.out` and record no per-version results. The probe itself lives in the scratchpad (`agents/sol-18/probes/db921.{tcl,out}`), where it does show code 0 and one command on 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Either copy the probe and its output into `integration-44/` (and `SHA256SUMS` if that convention applies), or reword the line as a citation. The phrase "Luna-25's review of v1" has no path, because that review is scratchpad-only. |

## 5. Remaining from v1

**Still owed:** D5, the measured runs, including `xtask owner-resolution`, a large standalone performance fixture and `make rust-check`. The Explorer coverage gaps that already exist at HEAD are recorded correctly in the correction note.

**Static compile read of the new code:** no likely errors.
- `RelationInputs<'a>` borrows the local `rows` and `retained`.
- `classify_lowering` has the new signature.
- The slice pattern is valid.
- No new `#[allow]`.
