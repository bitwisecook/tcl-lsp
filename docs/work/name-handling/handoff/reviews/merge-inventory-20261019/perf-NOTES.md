# perf agent notes: Analyser quadratic and SpecTcl load_embedded

Status: the batch is diagnosed, written and partly proven. It is NOT applied to
the tree. Everything needed to resume is in `perf/` beside this file. See
`docs/work/name-handling/outstanding/perf.md` (## Resume) for the step-by-step.

## Root causes (gdb stack sampling, opt-level 1 builds)

### Analyser::analyse, quadratic in top-level commands (all paths new on work)

1. `SourceCommandBindings::original_written_word_observations_at_span`
   (command_binding/original_name_value.rs), about 50% of samples.
   - Every W216 query (`emit_w216_brace_then_paren` -> `retained_value_word`) and every W143 query (`emit_w143_private_tcl_namespace` -> `original_written_head_at_span_in_source`) walks every dispatch offset at or before the span.
   - For each offset it builds a full `attach_invocation_reads(query_dispatch_points(..))` binding, then discards it because its words do not cover the span.
   - That is O(prior commands) bindings per query. The proc-body offset also carries one point per activation, so that one binding is an O(n) join.
2. `declaration_caller_alias_arguments` (command_binding/formal_call_advice.rs), about 35%.
   - It is called per CFG statement from `interprocedural::collect_positioned_call_by_name_reads`.
   - Each call re-scans the callee's declaration sites and rebuilds `declaration_original_tokens_at`, joining all n activation points, even though the result does not depend on the invocation.
3. `CommandBindingRealm::invocation_at_source` (realm.rs) re-joins the n per-activation points on every query.
   - Callers include source_structure, original_definer, interprocedural and lowering.
   - The realm already has a per-offset memo, `retained_original_invocation`, and `bindings.invocation_at_source` ignores `head`.
4. tclIndex shape (`set auto_index(X) [list source [file join $dir f.tcl]]`): `incomplete_body_metadata.rs::incomplete_body_operand_layout_advice`.
   - For every command and every earlier declaration site it calls `header(image, site..end)`, which re-parses the document tail (`native_script_words_in` -> `first_parse_cut`).
   - That is O(commands x sites x doc).

### tcl_spectcl::bundled::load_embedded (65-84 s in the debug test profile, 7.3 s at opt-level 1)

1. tcl-lexer `parse_cut.rs::command_cut` builds `SourceImage::from_bytes(src, channel)` over the whole script once per top-level command. Each build copies, hashes and LineIndexes the whole source, so cost is O(commands x bytes). New on work; origin/rust's `command_cut` has no image.
2. `parsing/spectcl_source/original_word.rs::in_body` re-proves the whole ancestor chain for every word. At each level it compares the full root image: `SourceImage::eq` is a complete byte compare with no shared-allocation shortcut. Called from tcl-spectcl `loader.rs::body_statements` for every word of every block.

## The batch (`perf/perf.patch` = `perf-owned.patch` + `perf-realm-interprocedural.patch`)

Every change is exact by construction: memoise a pure function of immutable inputs, or skip work whose outcome is already established.

- tcl-lexer `parse_cut.rs`: build the script image once per script, not once per command.
- tcl-lexer `source_map.rs`: `SourceImage::eq` short-circuits on `Arc::ptr_eq`, then falls back to the full byte compare.
- `original_word.rs`: `in_body` checks only the new parent link (`matches_parent_link`). The invariant is established at construction: in_document debug-asserts the root, and in_body checks each link.
  - A full-chain or even one-ancestor debug_assert per word costs 6.9-7.0 s vs 1.9 s for load_embedded with debug assertions, so the per-word assert was dropped (coordinator-agreed).
  - New test `every_bound_body_word_satisfies_the_full_chain_proof`: ≥3 nesting levels plus a CRLF pack.
- `original_name_value.rs`: new `OriginalRealmQueryMemo`, owned by the realm.
  - Per-site span-independent geometry: binding, retained config and written spans, computed once.
  - A span query scans a compact Vec and runs the unchanged per-site step only for covering sites. Unmemoised callers keep the old path.
- `formal_call_advice.rs`: per-callee `DeclarationCallerAliasSurvey` (callee + caller-alias formal indices), memoised per (allocation, registry semantic key, metadata owner). Per invocation only `original_formal_argument` runs.
- `incomplete_body_metadata.rs`: `header()` memoised per region in a thread_local.
  - The key uses SourceImage equality (ptr fast path, then bytes) plus config.
  - The memo retains the image clone, so no address reuse can alias.
- `realm.rs` and `interprocedural.rs` (compiler agent's files; hunks in `perf-realm-interprocedural.patch`):
  - realm owns `query_memo`.
  - `original_written_{word,head}_at_span_in_source` pass the memo.
  - `invocation_at_source` uses `retained_original_invocation(offset)`.
  - New `declaration_caller_alias_arguments` on the realm; interprocedural calls it.

## Measurements (opt-level 1; full tables in `perf/evidence/timings.md`)

- `p 1` x n: before n=40/80 = 3.5/14.8 s; after = 1.4/2.9 s; n=160 ~7 s.
- upvar shape n=30: 39.6 → 11.8 s.
- tclIndex: 100 lines 90.7 → 7.9 s; all 253 lines >10 min → 29.3 s.
  - Still mildly super-linear. The residual is per-command whole-document SourceImage/SourceMap/LineIndex rebuilds (~20%) plus re-segmentation per query (~25%), spread over analyser/commands.rs, registry_invocation/source_structure.rs and diagnostics/validity.rs.
  - A further fix would cache one document SourceImage on the Analyser. Those are compiler-names / unowned files; not done.
- load_embedded: 7.3-10.4 s → 1.9-3.1 s.
- origin/rust comparison: not measured. The worktree was set up but never built because of the RAM/disk budget.

## Equality evidence

- Synthetic shapes (calls 10/40/80, procs 41, upvar 10/30) and the tclIndex prefixes 10/40/100/253: in-process old vs new `AnalysisResult ==` holds.
- PackSet from load_embedded: identical sorted `{:#?}` lines old vs new (484,936).
- Corpus run 1 (two-way, old after new): 99/101 `.tcl` equal. The 2 `.tcl` mismatches and all 15 `.irul` mismatches are probe artifacts: the same mismatch appears old-vs-old (`ZZ_SAME=old`) in a warm process, with retained OnceLock/registry fields `<uninit>` vs initialised. Diagnostics were equal in every mismatching case.
  - Outputs: `perf/evidence/corpus_*run1_twoway.out`.
  - The probe now runs old1/new/old2 and compares new against the old runs (`perf/tools/perfprobe_example.rs.txt`). That rerun is NOT done yet.

## Not done (see outstanding/perf.md)

1. Three-way corpus rerun.
2. Crate tests (tcl-lexer lib; tcl-compiler lib filters spectcl_source / original_name_value / formal_call_advice / realm / interprocedural / w216 / w143; tcl-spectcl eval_loader, golden_packs), run with and without `ZZ_OLD_ENV=1`, to show zero pass-to-fail.
3. origin/rust timings.
4. Landing: one batch, after the coordinator's warning.
5. The residual tclIndex constant.
6. The review's section 4 scaling shapes.
