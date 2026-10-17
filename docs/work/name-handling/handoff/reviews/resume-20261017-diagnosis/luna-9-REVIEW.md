# Luna-9 review: Sol-8 `fix-1-native-lifecycle-next-chain.patch`

**Verdict: HOLD (minor).** Admit once amendment B below is in. Amendment A is strongly recommended in the same change. C is a follow-up in the shared owner.

## How this was checked

- The review was static only.
- Tcl semantics were probed with `nice tclsh8.6` (8.6.18) and `nice tclsh9.0` (9.0.4). The scripts are in `luna-9/probes/p1..p5.tcl` and the output is in `probes/probe-output.txt`.
- I did not run cargo, any test binary or gdb, and I did not write to `/home/user/tcl-lsp`.
- The worktree equals HEAD `bb7277d82` for both patched files, and HEAD includes `60051c3c2`.
- The patch passes `git apply --check`.
- I applied the patch to copies under `luna-9/fmt/`. The result is byte-identical to Sol-8's `b/` tree.
- `rustfmt --edition 2024` leaves both patched files unchanged.

## 1. The diagnosis is confirmed

| Claim | Evidence |
|---|---|
| Test helper analyses as tcl8.6 | `rust/tcl-lsp-core/src/references.rs:4848-4851` |
| tcl8.6 has no lexical advice | `tcl-compiler/src/analyser/types.rs:3119-3127` requires `has_logical_source_name_context`. That is false for any profile with a `native_name_protocol` (`analyser/input.rs:85-95`). The tcl8.6/tcl9.0 assertion is at `command_binding/source_transition_advice/source_class.rs:1002-1004`. |
| `original_declaration::select` proceeds only *without* lexical advice and offers `for_special` rows on the keyword span | `original_declaration.rs:716-718`, `:744-752`, `:1643-1647`, `:372-399` |
| References then go to `readonly_reference_report` | `references.rs:1016-1035` → `original_declaration.rs:1077-1092` → `reference_spans` |
| `Special` returns nothing | `original_declaration.rs:926` |
| The lexical resolver is unreachable for native analyses | `references.rs:1037-1039` returns before `constructor_or_destructor_references` (`references.rs:1176`, `:1245`) |

The diagnosis is correct. Every native lifecycle next-chain reference is empty for the same cause, plain `next` included.

## 2. Semantics against real TclOO (8.6.18 and 9.0.4 behave identically)

### What is right

- **Plain `next` follows the C chain, mixins included.**
  - `tcloo_linearise` puts mixin-path classes first (`tcl-syntax/src/mro.rs:238-312`).
  - The probe agrees: `Mix-ctor → Sub-ctor → Base-ctor`. A superclass's mixin is also hoisted before the subclass: `Mix → Sub2 → Base2`.
  - The patch anchors at `other_q` and scans from `pos+1`, which is correct.
- **Filters never wrap constructors or destructors.** The probe printed `F-ctor` with no filter output, so filters have no effect here.
- **Class-side lifecycle does not exist in TclOO.** `oo::define C self {constructor …}`, `oo::objdefine C {constructor …}` and `self destructor` all fail with `invalid command name "constructor"`/`"destructor"`.
- **The last declaration wins, and an empty constructor body deletes the constructor.** The hierarchy already applies `is_empty_method_body` to constructors (`class_hierarchy.rs:444-472`, `:719-723`). A shadowed or deleted selected constructor yields no spans.
- **Diamond inheritance (`D: B C`, all with constructors).** B's `next` reaches A for `B new` and C for `D new`. The patch reports B→A, which is true for direct instantiation. B→C is missed, which is acceptable because the report only promises positive spans.

### Spans the patch would emit that are not real references

**S1. `nextto X` where X has no own effective implementation**
- The hierarchy falls through to the next provider after X (`class_hierarchy.rs:452-471` and `:485-499`).
- Real Tcl raises `constructor has no non-filter implementation by "X"`, and the same for destructors and for X with an empty body (probe p2 a, b, g, g2).
- The patch attributes the site to a provider it never reaches.
- This is shared with the lexical path (`references.rs:2371-2410`) and with native go-to-definition (`definition.rs:1306-1319`).

**S2. `nextto X` where X precedes the current class in the chain** (for example, the current class's own mixin)
- The hierarchy anchors at X and returns X.
- Real Tcl raises `constructor implementation by "Mix" not reachable from here` (probe p3 `nexttomix`; p2 `self`).
- This is also shared with the lexical path and definition.

**S3. An empty destructor `destructor {}` is a deletion**
- Probe p1 shows `info class destructor B` is `<>` and C's `next` goes straight to A. A destructor with a single space stays real.
- `destructor_next_provider` (`class_hierarchy.rs:474-499`) treats existence as a provider. Its own doc comment says this was unverified, and it is now refuted.
- So a subclass's `next`/`nextto` is attributed to the deleted keyword.
- This is valid real-world code (placeholder destructors).
- It is shared with the lexical path.

**S4. Duplicate class records that share a rendered name — native-specific (the blocker)**
- An example is `Base destroy` followed by `oo::class create Base …` again. A failed re-create has the same effect.
- `original_class_metadata` keeps one record per create (`analyser/oo.rs:726-735`), and `declarations()` issues a `Special` identity for each record (`original_declaration.rs:1643-1647`).
- The patch keys the provider walk by `class.metadata().qualified_name` (patch, `references.rs` +2428/+2457-2463). The same `next` site is therefore reported for *every* record named `::Base`.
- Only one of them is real: probe p5 prints `new-Base`.
- Native go-to-definition resolves to the live `all_classes` member (`definition.rs:1318-1319`), so this also breaks references⇔definition symmetry.
- The lexical path never had this problem: `references.rs:1257-1271` selects the class by `all_classes` body span.

**S5. Class-object-side `Special` rows** — invalid code, low severity
- The analyser can record these for `oo::define C self constructor …` (`original_members.rs:1417-1421` → `:993` → `:1162-1171`).
- The patch filters these rows by side but then consults the instance-side `member_next_provider`, which ignores side.

## 3. Consumers and side effects

`reference_spans` and `readonly_reference_report` have exactly these consumers in the workspace:

| Consumer | Location | Effect of the patch |
|---|---|---|
| Native find-references | `references.rs:1016-1035` | Gains the next-chain spans. This is the intended fix. |
| Native document highlights | `references.rs:4616-4629` | On a lifecycle keyword, also highlights the `next`/`nextto` heads. |
| Native code lens | `code_lens.rs:119-138` | Constructor and destructor lenses now count the chain. |
| Server readonly reports | `tcl-lsp-server/src/readonly_reference_reports.rs:1673-1705` | Adds the spans to references for a native lifecycle identity. |
| `method_symbol.rs:1212` | — | Uses a Method identity only, so it is unaffected. |
| `original_call_hierarchy.rs:865` | — | Test only. |

- **Correction to NOTES:** `original_call_hierarchy::references` is **not** affected, because its `Special` arm adds nothing (`original_call_hierarchy.rs:818-820`).
- **Rename and edits are unaffected:** there are no edit consumers.
- **Lens counts are local only.** Cross-document subclass chains are not counted. The coverage stays `ExistingInventory`, so the lens reads "N references", but that is no worse than the previous 0.
- **Performance:** the native lens now costs O(lifecycle identities × classes) body scans. This is minor.

### Currently passing tests that could flip to failing

None found.

- `constructor_next_chain_reference_skips_non_overriding_subclass` (`references.rs:6098`) and `_not_counted_without_next` (`:6115`) still produce nothing.
- `constructor_lens_reports_zero_when_no_subclass_chains` (`code_lens.rs:928`) is still 0.
- Server `test_property_method_constructor_and_class_all_get_lenses_issue_992` (`tcl-lsp-server/tests/e2e/editor_features.rs:336`, tcl9.0, a single class) is still "0 references".
- Server `test_constructor_lens_counts_and_resolves_subclass_next_chain` (`:372`) uses the default `tcl` dialect, which is logical/lexical, so it is unaffected.
- A scan of every string literal in `tcl-lsp-core`, `tcl-lsp-server`, `tcl-mcp` and `tcl-lsp-db` for an OO source with a lifecycle `next`/`nextto` finds only these tests and two diagnostics or definition e2e tests. Those do not consume reference reports.
- No highlight test covers a lifecycle keyword.

## 4. Tests expected to flip (static prediction; none of these are in the 1085-selector capture)

### FAIL → PASS (with the patch as-is, and still with amendments A and B)

| Test | Location |
|---|---|
| `references::tests::constructor_next_chain_reference_from_direct_subclass` | `references.rs:6077` |
| `references::tests::constructor_next_chain_reference_skips_ancestor_with_no_own_constructor` | `references.rs:6132` |
| `references::tests::constructor_next_chain_reference_via_nextto_explicit_target` | `references.rs:6151` |
| `references::tests::constructor_next_chain_reference_via_braced_nextto_target` | `references.rs:6177` |
| `references::tests::destructor_next_chain_reference_from_direct_subclass` | `references.rs:6206` |
| `references::tests::constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method` | `references.rs:6241` |
| `code_lens::tests::constructor_lens_counts_subclass_next_chain` | `code_lens.rs:885` (qname `::Base::constructor` via `class_constructor_key`, `tcl-compiler/src/analyser/types.rs:826`) |
| `code_lens::tests::destructor_lens_counts_subclass_next_chain` | `code_lens.rs:910` |

These depend on two things I could not observe statically: `declarations()` succeeding for these sources, and `scan_next_dispatch_sites_with_target` finding `next` natively. The second is supported by `jump_to_nextto_named_class_method` being green natively on the same scan.

### Expected to stay FAILING, not addressed by the patch

- `references::tests::constructor_shadowed_by_a_later_redeclaration_has_no_references` (`references.rs:6223`).
- With `include_declaration=true`, the native path always pushes `identity.span()` (`references.rs:1026-1028`), so the result is 1 and not empty.
- This is pre-existing and uncaptured. Track it separately.

### Test quality note

`via_nextto_explicit_target` and `via_braced_nextto_target` place `base_refs` at (4,6), which is the `superclass` word, not `Base`'s constructor (line 5). The "Base gets nothing" half is therefore vacuous.

## 5. Compile, lint and gates

| Check | Result |
|---|---|
| Signatures | Every one used exists with matching visibility at HEAD: `analyser::types::OriginalSourceSpecialMemberMetadata` (re-exported at `types.rs:65-67`); `signature_scan::original_name::SourceDeclarationMetadata::metadata` (`original_name.rs:491`); `analyser::ClassDef`; `class_hierarchy::SourceMemberKind::Special` (`class_hierarchy.rs:50-67`); `AnalysisResult::class_hierarchy() -> &ClassHierarchy` (`types.rs:4020`); `member_next_provider(&str, &SourceMemberKind, &str, Option<&str>, &str) -> Option<&str>` (`class_hierarchy.rs:505`); `original_class_declarations()` (`types.rs:3462`); `crate::definition::canonicalise_class` is `pub(crate)` (`definition.rs:1333`); `scan_next_dispatch_sites_with_target` is `pub(crate)` (`references.rs:3483`); `OriginalDeclarationIdentity::{class_metadata, special_metadata}` are pub (`original_declaration.rs:406`, `:493`). |
| Name collisions | None: `references.rs` did not import either type, and neither new fn name exists elsewhere. |
| Clippy (`pedantic` only, `Cargo.toml [workspace.lints.clippy]`) | Nothing expected. `.filter(..).last()` is over an opaque `impl Iterator` (`original_special_members.rs` `declarations()`), so `double_ended_iterator_last` cannot fire. The fns are `pub(crate)`/private, so `must_use_candidate` does not apply. |
| `#[allow]` | None added. |
| String literals or command names | None. `next`/`nextto` come from `TCLOO_NEXT_CHAIN` plus `ArgRole::Name`, and the kind comes from `DefinitionSpecialMemberKind`. |
| `xtask resolution-drift` | Clean: the patch never mentions `all_procs`/`all_classes` and has no `.name ==` (`rust/xtask/src/resolution_drift.rs:131-183`). |
| `xtask registry-axes` | Clean: `references.rs` ratchet is 1 (`registry_axes.rs:245`), and the patch adds no literal. |
| `xtask owner-resolution` | Clean. `original_declaration.rs` is a `COUNTED_NAME_FACADES` member (`owner_resolution.rs:1411-1425`), but the patch adds no `split`/`find`/`trim_*` delimiter or zero-terminator recipe (`:1499-1567`). No other owner-resolution rule names these files. |
| `retired-api-gate` | No needle matches. |
| `check-name-resolution-proofs` | Manifest evidence pins sha256 for both files (9 records). Those digests are **already stale at HEAD**: `references.rs` recorded `e5fc3101…` vs actual `fa9062c4…`, and `original_declaration.rs` recorded `2fb2df2a…` vs actual `c0cc0ebe…`. This is a pre-existing gate failure, not introduced here, but both digests must be refreshed before the PR gate. |
| Contract note | The patch keys native identity by the rendered `qualified_name` into the reporting-map-derived `class_hierarchy()`. There is precedent in the native definition path (`class_hierarchy.rs:200`, `:227`; `definition.rs:1310-1319`), but that keying is the root of S4. A record-based owner already exists: `class_hierarchy::original_metadata::original_instance_metadata_order` (`original_metadata.rs:32-50`; it abstains on ambiguous joins) and `OriginalLexicalNextCall::provider_start` (`analyser/types/original_member_context.rs:470-497`, which enforces `nextto` reachability). That is the contract-aligned long-term home. |

## Required and recommended amendments

### B (required): make the selected record the one the provider walk names

- Replace or augment the `effective_special_member(class, special) != Some(special)` check:
  ```rust
  if special.side() != MemberSide::Instance
      || hierarchy.classes.get(class_q)
          .and_then(|cd| class_hierarchy::class_member_def(cd, &member))
          .is_none_or(|md| md.name_span != special.keyword_word().span())
  { return Vec::new(); }
  ```
- This mirrors `definition.rs:1318-1319`, so references stay symmetric with definition. It closes S4 and S5.
- It needs a `MemberSide` import in `references.rs`.
- It needs a `MemberSide` import in `references.rs`.
- The `Instance` filter has precedent in `original_member_context.rs:521-526`.
- Optionally apply the same body-span join to each `other` row.
- Validate that the ledger keyword span and the `ClassDef.constructors` `name_span` coincide (both are the keyword word: `original_members.rs:1158`; `references.rs:1269-1271`). The eight flips above are the acceptance check.

### A (recommended, two lines): make `nextto` exact

- In the `Some(target)` arm, skip unless `target == class_q`.
- Also skip unless `target` sits after `other_q` in `hierarchy.mro_map[other_q]`.
- This closes S1 and S2 for native references without touching the shared owner, and both nextto tests still pass: target `::Grandparent` is at MRO position 2, after `::Sub` at 0.

### C (follow-up, separate change in `class_hierarchy.rs`)

- Apply `is_empty_method_body` to destructors in `destructor_next_provider`, now confirmed against 8.6.18/9.0.4.
- Make `nextto` exact and reachable in `constructor_next_provider` and `destructor_next_provider`. No hierarchy test pins the fall-through (`class_hierarchy.rs:1566-1700`).
- This fixes S1–S3 for the lexical references, native references and definition paths together.
