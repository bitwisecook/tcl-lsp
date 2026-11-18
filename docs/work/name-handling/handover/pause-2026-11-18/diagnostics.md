# Diagnostics: pause handoff, 18 November 2026

Session `session_017u5g3HXzMVEcpUS6bxGcmA` (area: diagnostics). Paused by the owner's instruction. Tree clean; nothing in flight on disk; no patch files.

## (a) Landed today (all on `origin/work`)

Test scrub, in the ledger's order:

- Fixture causes: `ee88fdc70b` (fixture-raises-before-subject), `beb6a56c22` (unknown-command-body-never-runs), then one commit per cause for FIX-FIXTURE/FIX-EXPECTATION: `09e42ccd55` … `0685177273` (groups 1-2), `45b0532f40` … `9ebee01fe0` (groups 3-5), `e2e80e6788` … `2ef11646bc` (group 6, incl. the two passing-wrong `namespace unknown` tests now expecting W123).
- RETIRE/MERGE: `0b76c8293a` (five regex-pattern-record tests), `81b6e5b715` (glob-switch unknown), `321b128b6c` (18 exact-cluster merges; the two `phi_undef_memo` ones left to analysis-performance), `d5d0b443f9` (p(key) formal retired, Jim witness added), `20ee3d3571` (quoting-child merged into a pack-declared helper test).
- REWRITE: `dbb5237818` … `381241d612`, `c70f40a071` … `ba924c47ec`; `b11d859ae6` routes the 11 hover/navigation rows to lsp-features.
- Rulings carried out: `a6efd0db9c` (dict update harvest, SCCP-only twin, caller-only silence; gap in `outstanding/diagnostics-gaps.md`), `114a8ea820` (+ notes re-pointed to class-facts), `549d1c54c5`, `40b94d4ebc` (eight rows pass on the full run).
- Product: `d1fe753820` (unresolved computed interp path gets its own `@dynns@` site domain). `36941b155e`: a captured `interp alias` slot's call error stays at its producer word, once per producer, no fix, message "(reached through the call at line N)"; W104 style advice declines on a captured slot; contract `diagnostic-registry-source-ownership.md` amended (Conclusion + Scope) with provider answers citing `evidence/tcl/naming-diagnostic-registry-source-ownership-captured-alias-call-errors.md`. (`8eb1f62864` was the first attempt, reverted by `bbab922db4` after review.)
- Clippy/fmt fixes of others' pushes: `f2d00d58b6`, `f280552546`, `501c862751`.

## (b) In flight

Nothing on disk. I had started reading the W216 family (six rows, causes `w216-literal-replacement-missing`, `w216-braced-array-value-read`, `w216-brace-dollar-index`, `w216-fix-qualified-set`, `w216-name-position-indirect-array`); a test run was killed before it reported. tclsh facts gathered (8.6.18/9.0.4): `set foo bar; puts ${arr($foo)}` → can't read "arr($foo)" (index not substituted); with `set tok ::http::1`, `unset ${tok}(socketcoro)` → can't unset "::http::1(socketcoro)"; `set arr v; puts ${arr}(a\)b)` → `v(a)b)`.

## (c) Queue, in order

Wrong answers first, then KEEP causes by tests blocked (diagnostics-owned):

1. Wrong answer routed by lsp-features: strict local rename past a computed head. I have not seen that message's detail in this session; ask lsp-features for the witness.
2. Wrong answer routed by values: I230 "always true" after a body-role stub call (`wrong-i230-body-role-stub`, witness it_2 `unseen_code::a_stub_that_states_its_frame_effect_is_a_named_head`). Diagnosis so far: the emitter `analyser/diagnostics/dataflow/constant_conditions.rs::emit_constant_branch_diagnostics` chains the advisory `fu.diagnostic_value_facts()` (SemanticValueProjection, `sccp/original_value_facts`, dataflow-optimiser's family) before strict `fu.sccp`; strict is right per values, so the advisory solve misses the barrier a `script:body` role forces. Routed to dataflow-optimiser (inferred, not probed). Keep the emitter ungated unless they ask: gating on strict would drop sound advisory-only I230s.
3. W216 family (6 rows, above): fixture repairs for the two exposed rows (bind `arr`/`tok` first), then the missing literal-replacement fix, then KCS-vs-product `set` vs `::set` fallback (`w216_funny_name_falls_back_to_set_indirection`; the test holds the KCS).
4. Remaining diagnostics-owned KEEP causes from `docs/work/test-scrub/diagnostics.md` ("Causes ranked"): `const-subst-head-fold` (5), `interp-eval-literal-domain` (5), `uncalled-body-generic-entry` (4), `vendor-profile-w123-silent` (4), `safe-interp-w129-reach` (4), `s100-shimmer-silent` (4), `manifest-env-not-applied` (3), `materialized-footprint-empty` (3), `irules-disabled-w002-missing` (3), `bounds-switch-arm-write-scan` (3), `logical-class-family-context` (3), `w210-narrowing-false-arm` (3), then the 1-2 row causes. Order = tests blocked, per the scrub README.

## (d) Waiting on / waited on

Waiting on:
- class-facts: `registry_invocation` role advice empty for `if {1} $body`, `uplevel $lvl $script`, `eval {*}$p` (3 param-traits rows, routed; class-facts acknowledged and queued).
- analyser-binding: loop first-entry binding (`loop-first-entry-binding`) for `eval $b` in a `while` body (row `deep_pass_descends_through_multiple_levels`, blocked).
- dataflow-optimiser: the I230 advisory-solve barrier (queue item 2).
- taint: `w201-detector-silent` (7 rows).
- tk-oo: index entry fact for `w210_pkgindex_reports_other_reads_inside_command_substitutions` (ruling #832).
- lsp-features: 11 REWRITE rows (hover/navigation observables).
- Owner: the contract amendment in `36941b155e` is flagged by the coordinator for the owner to check.

Others waiting on me:
- server-e2e-db: W216 quick-fix e2e test depends on the compiler W216 rows (queue item 3). W201 is taint's.

## (e) Coordinator rulings not yet carried out

- None outstanding beyond the queue. (The 18 Nov contract-amending ruling is carried out in `36941b155e`.)

## (f) Last measured counts

At `2678398306`-era tree (my last full run, just before `36941b155e` landed on top of `1a3a7cd6c1`): `cargo nextest run -p tcl-compiler --no-fail-fast` with `TCL_LSP_JIMSH=tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`, `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1` after `source scripts/dev/agent-build-env.sh`: 13158 run, 11277 passed, 1881 failed, 6 skipped (whole crate, all areas). Required-oracle vs no-jimsh runs give the identical failing set (0 new under required oracles). Ledger (`python3 docs/work/test-scrub/tools/ledger_status.py docs/work/test-scrub/ledger`): diagnostics open 232, repaired 62, exposed 64, fixed 13, deleted 19, blocked 1, routed 9, rows 400.

## (g) Things a successor would get wrong

- The deletion helper I used (scratch, not committed) once cut a test at a Tcl `}` in column 0 inside a string literal; check every deletion compiles.
- Never `git stash` around a fixup/autosquash with `&&` chains: a failed step dropped a stash once (recovered from a dangling commit).
- A test-only change still needs clippy: the toolchain flags `assert!(x.is_empty())` (`assert_is_empty`) and argument-less `format!`.
- Captured alias diagnostics: the product anchors on the producer word in the alias definition; refusing the subject wholesale breaks W127/W302/pattern/path advice, which legitimately keep producer geometry. The contract now says: call error kept at producer, no fix, call named; style advice declines.
- `cargo fmt`, clippy and `make xtask-gates` before every push; quote fixed counts only against a clean `origin/work` baseline.
- `rm -rf /tmp/tcl-spectcl-cache-test-*` if disk runs out.
