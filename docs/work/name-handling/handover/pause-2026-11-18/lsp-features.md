# lsp-features: pause handover, 18 November 2026

Area: `rust/tcl-lsp-core` (including semantic tokens) and the navigation family of the `tcl-lsp-server` e2e tests. The working tree was clean at pause: no patches, no uncommitted work, no background runs. Everything below is on `origin/work`.

## (a) Landed today

Product:

- `8b2c7618` An incoming call inside a method body is attributed to that method in the call hierarchy (it was `<top-level>`, a wrong answer).
- `f43045a1` Completion at a character past the line end completes at the line end (LSP rule).
- Earlier the same day: `command_settlement.rs` `settle_original_invocation` takes the wildcard import index (linked view only). That fixed 7 rows.

Tests and ledger, FIX-EXPECTATION work, oldest first:

| Commit | What |
|---|---|
| `5b225d25` | Partial-census lens titles ("at least N references"). |
| `ddb361a2` | Strict variable rename tests run under tcl8.6; `café` must refuse (tclsh: `$café` reads `caf`). Cause `lenient-tcl-strict-rename-refuses`, 5 repaired. |
| `ac859842` | Two pedantic lints (doc_markdown in `logical_call_target_tests.rs:336`, assert-empty in `definition/tests.rs`). |
| `33b93473` | brace-expr tests expect the `missing-expression-evaluation-equivalence` refusal, 4 repaired. |
| `4d9fa469`, `c30b8580` | Inline tests: a free proc variable refuses; prefix-sharing is checked with a bound second parameter. 3 repaired. |
| `151b13d5` | Wildcard-import rename tests expect a refusal, not a declaration-only edit. 2 repaired. |
| `740756a0` | Property rename tests use `oo::configurable` and `configure -color`, and expect a refusal. 2 repaired. |
| `f2fe0504` | Dir-view rename tests: the plain `${dir}view` brace check passes; a computed head (`$w ${dir}view`) must refuse. Red: wrong answer, routed. 2 exposed. |
| `c59e6548` | iRule helper and extract-proc tests expect no direct-proc-call action (IRULE5005). 3 repaired. |
| `92818d58` | Property lens test on `oo::configurable`; `my size` is no reference. Red: wrong compiler fact, routed to class-facts. |

Also today: about 25 fixture commits (FIX-FIXTURE causes, now 0 open), Logical internals restated as editor answers (`0b30c309`, `994c18dd`, `d2ce7b90`, `82636bee`), and classmethod order tests (`4bf356c2`, `80a7cdb9`). See `git log --grep=session_017u5g3MRrNmuqWZxa9Lcp4F`.

## (b) In flight

Nothing is half done. The next cause had not been started. A successor starts from a clean tree.

## (c) Queue, in order

1. **Measure the area at a clean `origin/work` head.** The last full-area counts are older than today's work, and the integration targets were never measured by this session; see (f). The coordinator's pause message mentions core `it_2` to `it_5`. This session has no partial run of those targets: none was started.
2. **Import-order product work.** It waits on analyser-binding's fact `head_blocked_only_by_glob_imports() -> Option<Vec<(source_ns, pattern, import_site)>>`, at their queue position 12. I may write it with their review.
3. **Remaining FIX-EXPECTATION rows** (45 open in the ledger). Do the singles first; their evidence is in the ledger. Of these:
   - Mechanical: `call-hierarchy-proc-label-short-name`, `build-counter-exact-equality`, `proc-offered-at-method-position`, `completion-detail-wording`, `refusal-shape-none-vs-disabled`, `logical-obligation-cleared-expected`, `superseded-declaration-cursor-selects-itself`, `test-expects-literal-dollar-colon-rename`, `unknown-head-world-ends`, `member-rename-future-callback-terminal`, `unbound-read-references-itself`, `test-accepts-rename-leaving-map-list-stale`, `test-expects-unproved-rename-past-dynamic-eval`, `test-accepts-partial-duplicate-decl-rename`, `unavailable-lens-not-clickable`.
   - Need a ruling first: `if-to-switch-trace-gate-incomplete-visit` (doc contradiction), `forced-import-shadow-inline-proof-tier` (inline from an unsourced sibling's export under `tcl`).
   - Ruled, not yet done: `native-t101-hardening-permission` (offer the action where the inserted `string` is the proved builtin; add a declining rebound-`string` row; amend `original-diagnostic-source-actions.md`), `pinned-release-unsourced-shadow-of-builtin`, the classmethod rows (blocked on class-facts).
   - Then `refusal-reason-wording` (12) and `switch-to-dict-edits-expected` (9; see (e)).
4. RETIRE/MERGE (7), then REWRITE (35).
5. Semantic tokens' remaining rows.

Reason for the order: measurement first, because the counts in (f) predate today's commits and the integration targets have never been measured. Then the coordinator's standing order: product work, then FIX-EXPECTATION, RETIRE/MERGE, REWRITE.

## (d) Waiting on others, and others on me

Waiting on:

- **Coordinator:**
  - Name the owner of the computed-head rename barrier (sent 18 Nov). Under tcl8.6, `proc m {w dir} { $w ${dir}view }` renames `dir`, and `[set w] $dir` does too. `$w x; return $dir` refuses. tclsh with `proc peek {args} {upvar 1 dir d; return $d}`: before the rename `y`, after it `can't read "d"`. The fact is tcl-compiler `AnalysisResult::original_variable_rename_is_complete(_for)`. Exposed by core `rename::tests::variables::rename_var_applying_the_dir_view_idiom_edit_produces_valid_tcl` and e2e `rename::rename_var_applying_the_dir_view_idiom_reference_edit_does_not_corrupt_the_source`.
  - The switch_to_dict ruling, in (e).
- **class-facts** (session_017u5g3Kgho1C9zZ3yZYrK8P):
  - Sent 18 Nov: `original_source_self_property_references` (`source_method_reference.rs`, `capture_property`) matches `my <name>` to property `<name>`. tclsh 9.0.4/9.1.0: `my size` raises unknown method; the accessors are `<ReadProp-size>`/`<WriteProp-size>`. It feeds the property lens, member rename and member census. Witness: core `code_lens::tests::lens_counts_property_my_dispatch_within_class_body`.
  - Earlier routings, all accepted: the classmethod instance-slot order; Logical class drop after an unknown inside `namespace eval`; list-built `[self]` callback receivers under `tcl`; inner `C` in `::a`.
- **analyser-binding:**
  - The glob-import head fact.
  - Alias templates for a computed element (FocusGrab).
  - The stale binding of `C` inside `catch` after `rename C M`.
- **server-e2e-db:** the pack-extension file that never enters the index (`spec_packs/reload.rs:616` skips the rescan at Startup). It is theirs.

Others waiting on me: none known.

## (e) Coordinator rulings not yet carried out

- **switch_to_dict (9 rows).** The ruling asked for:
  - a restatement as a disabled action with the explicit no-edit obligation reason;
  - the matching amendment to `refactor-original-source-rewrite-permissions.md` ("All earlier tests and assertions are retained…");
  - a positive control with a default arm.
  Not started.
- **native T101 hardening permission:** see (c).
- **Pinned-release unsourced shadow of a builtin:** the three external-occupant hover cases become FIX-EXPECTATION (builtin hover stands). Not started.

## (f) Last measured counts

- **Server navigation family**, under the required oracles, on 18 Nov before today's FIX-EXPECTATION commits (head not recorded): 260 run, 77 failing, all ledgered, 0 new. Command:

  ```
  TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 \
    cargo nextest run -p tcl-lsp-server --test e2e --no-fail-fast \
    -E 'test(/^(navigation|tcloo_navigation|references|definition|hover|call_hierarchy|type_hierarchy|implementation|rename)::/)'
  ```

- **Core lib, whole area:** the last full list is `docs/work/name-handling/lists/lsp-features-failing-2026-11-17.txt` (808 lines). Same environment, then:

  ```
  cargo nextest run -p tcl-lsp-core --lib --no-fail-fast -E 'not test(=irules_context::tests::deeply_nested_bodies_survive_when_scanning)'
  ```

  It has not been re-run since today's commits.
- **Ledger** (`docs/work/test-scrub/ledger/lsp-features.tsv`) at pause:

  | Status | Rows |
  |---|---|
  | open | 473 |
  | fixed | 52 |
  | exposed | 54 |
  | routed | 33 |
  | blocked | 32 |
  | repaired | 29 |
  | deleted | 10 |

  Open by verdict:

  | Verdict | Rows |
  |---|---|
  | KEEP-ORACLE | 224 |
  | KEEP-CONTRACT | 157 |
  | FIX-EXPECTATION | 45 |
  | REWRITE | 35 |
  | KEEP-F5 | 5 |
  | MERGE | 4 |
  | RETIRE | 3 |

- **Clippy:** `cargo clippy -p tcl-lsp-core --tests` gives 9 pedantic warnings, all in other areas' files (`original_context.rs`, `vfs/tests.rs`, `document_symbols/tests.rs`). `make xtask-gates` and `--all-targets` were not run today.

## (g) Things a successor would get wrong

- The conditional variable rename (`conditional_variable_rename_edits`) is narrow on purpose: only top-level, root-namespace commands whose roles are all VarRead/VarWrite (`tcl_compiler::analyser::logical_variable_alpha`). Proc locals, `namespace eval`, `upvar`, `global` and `trace` all refuse there. A ledger note saying "re-point at the conditional route" can be wrong; prove such tests under a pinned release with the strict route instead.
- Under a pinned release, an unknown bare command (`foo`) does not block a local rename: in the pinned world it raises. A computed head (`$w`, `[set w]`) should block it, and today it does only when the read comes after the call. That is the routed wrong answer.
- `$café` in Tcl reads `caf`. Non-ASCII letters end a `$name` substitution.
- `property` exists only in `oo::configurable` (Tcl 9). `my <prop>` is an unknown method. Many older fixtures use `oo::class create` with `property`; they are invalid.
- An inline refusal reason for a free variable reads "a nested command's original frame or operand grammar is unavailable", not a free-cell reason. Assert `is_err`, not the wording.
- The disk allowance fills with `target/debug/build` (19 GB seen). Keep the newest two hash directories per crate (`ls -1t | tail -n +3 | xargs rm -rf`), and remove `target/debug/incremental`.
- `cargo fmt -p tcl-compiler` reformats other areas' files. Revert anything you did not mean to touch.
- Test names are keys in `docs/work/test-scrub/data/inventory.tsv` and the ledger. Keep names when correcting expectations; fix the comments instead.

## Addendum: messages received after the handover commit

These were queued before the pause and reached this session only after `f499969f`. They are recorded here and not acted on.

- **Computed-head rename barrier: the owner is diagnostics** (session_017u5g3HXzMVEcpUS6bxGcmA; `rust/tcl-compiler/src/analyser/types/variable_renames.rs`).
  - Not yet sent. Send them the fixtures, the `peek` transcripts on 8.6.18 and 9.0.4, and the three controls (`$w x; return $dir`, `eval $w`, `uplevel 0 $w`).
  - The rule the fix must satisfy: a strict local rename is complete only if no command that could reach the local by name runs while the local is in scope. The barrier is the unproved call, not a later read. A proved, non-linking target does not block.
  - Ask diagnostics to state why the controls refuse while `${dir}view` does not. Ask for one positive witness: a body whose every call is proved and non-linking still renames.
  - The two tests stay red, exposed, blocked on that fix.
- **switch_to_dict ruling (coordinator, 18 Nov).** It replaces the summary in (e).
  1. Each of the nine rows expects a disabled action naming its reason (no default arm, or the fresh-store obligation), with no edits. Each row carries a tclsh 8.5.19 to 9.1.0 transcript for a matched and an unmatched subject.
  2. In the same commit, amend `refactor-original-source-rewrite-permissions.md`. Such tests are restated to the explicit no-edit obligation with the oracle transcript; the earlier expectation is recorded in the ledger row.
  3. Product fix: `set_pattern` returning `None` becomes the same disabled action with a reason. `dictionary_conversion_does_not_unwrap_literal_command_name_data_twice` then expects the disabled refusal, unless its row says why no action is the designed answer.
  4. A mandatory positive control: an enabled edit where equivalence is proved (a default arm assigning the same variable, `-exact`, literal arms, no fall-through, no other effects). Run it on tclsh before and after, for a matched and an unmatched subject, with identical output. It stays red (KEEP-ORACLE) if the product cannot prove it.
  5. Edges:
     - tcl8.4 has no `dict`: not offered, or disabled naming the release. Say which.
     - `-glob`, `-regexp`, fall-through arms, duplicate patterns (first wins in `switch`, last wins in `dict create`; show the transcript) and arms with other effects: one row per refusal reason.
- **Integration targets (coordinator).** `rust/tcl-lsp-core/tests/it_2` to `it_5` (about 1,254 tests) were never measured by this area.
  - Run them from a clean head with the oracle flags.
  - Push the failing list, and send run/failed per target before triage.
  - Then triage into the ledger. The 24 `it_5` `command_prefix_integration` call-graph callback-edge failures are one cause until shown otherwise.
  - Also list any `tcl-lsp-server` test target besides `e2e`, `lib` and `preview_tickets_e2e`, and say whether it was ever run.
  - This is queue item 1 in (c).
- **Clippy.** The `doc_markdown` warning at `logical_call_target_tests.rs:336` was already fixed in `ac859842`. Standing rule: clippy `--all-targets` must finish clean before any push.
- **Disk.** Run `rm -rf /tmp/tcl-spectcl-cache-test-*` before touching `target/`.
- **analyser-binding:** the stale `C` binding after `rename C M` was a probe offset fault. Probe offset 54 (the `catch` head, which is unresolved), not 42 (the `rename` operand). If `original_oo::class_for_invocation` at 54 still answers a class, that is a core lookup bypassing the invocation's empty reference; re-check it.
