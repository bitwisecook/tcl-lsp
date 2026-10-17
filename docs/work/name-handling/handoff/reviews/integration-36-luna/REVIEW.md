# Independent review of integration-36 (Luna-2)

This review is static only. I ran no cargo, no build and no tests. I used only `git apply` and `rustfmt` (edition 2024, via stdin), on HEAD `2a3928bc6` blobs copied into `mini/`. Line numbers refer to the composed tree, except where marked HEAD.

## Verdict

**HOLD for composition 36 as it stands.**

| Packet | Verdict |
| --- | --- |
| sol-composition/db042-structural-factory-v2 | **ADMIT** (source only). Compiler libtest, DB and Server execution are still owed. |
| sol-core-member-hazard-syntax-purpose-v1 | **HOLD**. It is a semantic no-op, and its new control likely reproduces the failure. |
| sol-core-member-source-census-v1 | **HOLD**. It must never be admitted alone. |
| sol-core-member-source-census-v2 (V1→V2) | **HOLD**. It flips an original passing assertion. |
| root-core-roster-test-setup-v1 (= piece.patch) | **ADMIT** (source), with the conditions below. |

A reduced composition of db042 plus roster is source-clear. It touches six paths and includes only db042's doc section.

## Hash checks: all PASS except the base label

- **Archive and members index match.** The archive `48dddad1…` and the members index `183f51b0…` match `capture.json`. All 20 relevant members match both the scratch copies and their `members.json` rows: integration-36 ×4, packets ×11, pending-peers ×5.
- **packets.json pins pass.** All 10 patch and manifest pins in `packets.json` are correct.
- **The `before_sha256` values match HEAD 2a3928bc6 for all 8 paths.** They do **not** match the declared `"base": f1e36d8d6`. Two paths differ there (name-handling.md and logical_members.rs), and db042 fails to apply at f1e36. The base field is mislabelled.
- **Every per-packet `after_sha256` reproduces.** V2 applies on V1's output (`f4119ffd…` → `6eb79b94…`).
- **The composed name-handling.md hash is `2b376972…` and is unpinned.** It is HEAD, then db042's section, then V1's section appended at EOF. The composed diff is `0802ceb0…`, with 8 files changed (+367/−119).
- **piece.patch matches the roster hunk.** It is byte-identical to the roster packet's completion.rs hunk and applies to HEAD.

## Application and preservation

- **Sequential application succeeds with one manual step.** Census V1's doc hunk collides with db042's addition at the same EOF anchor. The documented "EOF rebase" (V1's Rust hunks, then V1's doc text appended after db042's) reproduces all pins.
- **rustfmt passes.** All five Rust files are rustfmt fixed points, before and after.
- **Old tests are preserved except the roster adapter.** The touched Rust files contain 398 old `#[test]` items. 397 are byte-exact. The one exception is the roster test, which gains one inserted line (`completion.rs:7849`) and keeps every value and assertion. The review JSON says 401/400; this is only a count difference.
- **Other rules hold.** No new `#[allow]` was added. No consumer hardcodes a command name: the census uses `Traits::TCLOO_SELF_DISPATCH`. I found no source-to-Native grant.

## Findings

### db042 (ADMIT)

- **The split preserves full-path behaviour** (`logical_members.rs:366-431`). A class that does not match still clears all four facets.
- **The registry provenance is unchanged** (`state.rs:2304-2310`). `self.registry` is `analysis_context().commands()`, the same store the full path uses.
- **It fixes the real cause.** `project_procedure_roles.rs:72-80` needs the class `source_candidate_coordinate()` in structure-only DB analysis. That matches DB 35r2 test-001.
- **Low: the doc overclaims** (`name-handling.md:3752-3754`). It says the facets do not depend on diagnostics or a CU build. That is true only for the structural path. The full path still issues them inside `produce_object_handle_facts`, after the CU build under `catch_unwind`. The paragraph is also one unwrapped line.
- **Low: the control is weak** (`logical_members.rs:1724`). It never asserts that the full path still issues a body. The `"different"`-image assertion is vacuous.
- **There is no independent pending-peer** for this packet.

### hazard-syntax-purpose-v1 (HOLD)

- **Blocking: the patch changes nothing.** At HEAD, `OriginalRegistryWords::source_script_bodies(ctx)` already returns `source_script_bodies_for(ctx, Syntax)` (`tcl-compiler/src/registry_invocation/source_structure.rs:824-829`, unchanged since 4c717119b). The edits at `member_metadata.rs:141` and `:231` are therefore no-ops, and the stated cause, "iterates default Potential projection", is false. The peer review repeats the same premise.
- **Blocking: the new control will probably fail.** Core test-010 is missing Span 80..81, which is the `export m` inside `if {1} {export m}` nested in the class body. The new control (`member_metadata.rs:460`) asserts exactly that shape.
- **Where to look instead:** the drop is most likely in the nested non-member branch of `collect_region` (`member_metadata.rs:226-240`). Check where `definition_parent_for(&ctx, Some(parent))` returns None (`source_scoped_body.rs:611`), or `source_registry_words_in_definition_body_at` yields nothing.

### census V1 and V2 (HOLD)

- **Blocking: an original passing assertion flips.** The original test `readonly_member_reports_keep_source_spans_out_of_edit_inventories` (35r2 test-096, PASS) asserts `Partial` at `original_declaration.rs:1721` for:
  ```
  oo::class create C {method ping {} {}; method body {} {my ping; my ping}}   (tcl8.6)
  ```
  Its existing inventory is asserted empty, so V2's `all(contains)` gate passes vacuously. V1's own control at `:2282` asserts `CompleteSourceOccurrenceCensus` for the structurally identical:
  ```
  oo::class create C {method target {} {}; method owner {} {my target}}   (tcl8.6)
  ```
  One of the two must be red.
- **Blocking: the original assertions contradict each other.** The code-lens originals (`code_lens.rs:590-613`, `643-661`; 35r2 test-056/058 FAIL) need exact counts on the same tcl8.6 shape, which means Complete. That is a locked same-input contradiction between original assertions, as with Compiler221. It needs a user decision, not a silent behaviour flip.
- **Blocking for V1 alone: it promotes Complete over unaccounted positives.** Without V2, V1 promotes Complete over existing external-receiver positives. Both author and peer acknowledge this; V2 is mandatory.
- **Medium: V2's control is probably non-discriminating** (`:2341`). `C create c` and `c target` have no selected Registry schema, so `source_registry_words(...)?` aborts the census before any V2 veto or gate runs. The control passes on HEAD, V1 and V2 alike.
- **Residual missing-to-empty risk.** A zero-count Complete trusts `source_queries` to capture every non-self positive (callbacks, prefix builders). No control covers that.
- **Low: hand-compacted, non-idiomatic layout** (`:1138-1145`). rustfmt leaves it in place.
- **Low: undocumented narrowing.** Property identities with an `object_owner` lose the census path (`:1122-1127`).
- **Low: process narrative in the contract.** The new section (`name-handling.md:3756+`) contains development-process wording that AGENTS.md requires removing before a PR.
- **Info: the `{source}; mystery` assertion is vacuous** for the census, because it fails on currency first.
- **No static compile risk found.** All APIs used (`SourceSyntaxCoverage`, `definition_member_at`, `class_at_command_head`, `method_at_command`, `source_queries`, `command_span`) exist with matching types, and no dead code results.

### roster test setup (ADMIT, with conditions)

- **The adapter is genuine.** It is the same public `tcl_spectcl::core_surfaces::ensure()` the server's `initialize` calls (`tcl-lsp-server/src/lib.rs:30086`). The fixture and assertions are unchanged.
- **Low: the new dev-dependency cycle needs a rationale comment.** tcl-spectcl depends on tcl-lsp-core, so this creates a cycle. Cargo allows it, and tcl-registry and tcl-compiler already have the same cycle. Their manifests carry a rationale comment; this one has none. `project-layout.md:83` also says the edge "introduces no cycle". There is no `no_mangle` or `links` symbol risk, and the Cargo.lock entry is correct.
- **Medium: process-global seeding inside one test.** `ensure()` changes the tcl_dialect roster store and the tcl_registry core specs. In normal parallel `cargo test -p tcl-lsp-core`, about 30 Jim-dialect test loops become schedule-dependent. The selector-per-process harness hides this.
- **Conditions:** run the full Core binary in default parallel mode, and once with this test first, before claiming no regression.

### Composition metadata

- **Wrong base.** The base should be `2a3928bc6`, not `f1e36d8d6`.
- **Absolute paths.** The applied patch paths are `/workspace/naming-work/...` and need re-rooting on replay.
- **Missing pins.** Pin the composed after-hashes.
- **Test count.** 401/400 versus the 398/397 I counted.

## Environment incident: please check

My `git worktree add` hit **ENOSPC on `/` at about 19:41:27 UTC**. git rolled the worktree back, and `/` now has 8.8G free. The main repo is clean and no worktree metadata remains. **Any frozen-source capture output written around 19:41 UTC should be checked for ENOSPC and treated as suspect** (AGENTS.md rule). After that, I ran every patch check on HEAD blobs extracted to `scratchpad/agents/luna-i36/mini`.
