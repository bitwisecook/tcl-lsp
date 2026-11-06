# LSP release issue sweep

Updated 4 November 2026. Branch: `work`. The sweep stopped after verified closure of #2131. The LSP is not yet release-ready.

The sweep stops after verified closure of [#2131](https://github.com/bitwisecook/tcl-lsp/issues/2131#issuecomment-6643420031). Shared math-result ownership and its focused compiler consumer pass; public nested-expression inference remains unverified. The user explicitly authorised pushing all current code, including unfinished development checkpoints, and corrected the delivery target to `work`. The superseded Rust-target PR #2521 was closed without merging. [The final handoff](LSP_RELEASE_STOP_HANDOFF_2026-11-04.md) distinguishes those checkpoints from verified fixes and records issue updates and pending gates.

User priority update: the conditional-list S102 follow-up in #940 takes priority.
The user permits evidence comments and closure only for verified fixed issues
authored by bitwisecook. The seventeen issues #2131, #2218, #2254, #2255, #2295, #2334, #2351,
#2365, #2372, #2376, #2434, #2436, #2440 and #2446–#2449
were verified, commented and closed. All 20 other-reporter issues remain open;
their current assessment is in [LSP_RELEASE_EXTERNAL.md](LSP_RELEASE_EXTERNAL.md).

This sweep follows the user’s LSP/compiler release scope. Unrelated VM, BIG-IP, f5tcl and ancillary feature development is deferred; actual LSP-linked Engine/VM execution and hook confinement remain necessary naming-migration scope. Existing naming-migration acceptance work and its failing gates continue independently; this issue sweep does not certify those gates.

The coordinator attempts normal pull/rebase/push every ten minutes while its checkout is clean, with bounded race retries and no force. Source-frozen edits and tests pause this job; manual integration resumes it. Other issue-sweep lanes retain their authorised at-least-15-minute development cadence and pull with rebase before pushing reviewed fixes and status evidence. Development integration may proceed before green tests; required release gates still need to pass on the final source.

Six isolated shallow checkouts use separate Cargo targets. Models: Terra High/Extra High, Sol High/Extra High and Luna High. Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 are built from the repository’s pinned references. Latest JimTcl was cloned at `82fb4d0caf90240d94352a31fcbd16adf52658a7` (reports `0.84-10-g82fb4d0`). Reference provisioning alone certifies no product test.

All naming changes use the shared naming owners. Tcl parsing must use the existing child lexer/parser/segmenter. Original checkout code and pre-existing binaries are stale relative to `work`; only fresh source-bound results count.

| Lane | Status document | Assigned issues |
| --- | --- | --- |
| SERVER | `LSP_RELEASE_SERVER.md` | #1657, #2021, #2295 |
| PROVIDERS | `LSP_RELEASE_PROVIDERS.md` | #1956, #2143, #2254, #2265, #2497 |
| VALUES | `LSP_RELEASE_VALUES.md` | #2131, #2288, #2387, #2427, #2436, #2437, #2438, #2440, #2451, #2452, #2453, #2454, #2498, #2499, #2500, #2503, #2504, #2505, #2506, #2507 |
| BINDINGS | `LSP_RELEASE_BINDINGS.md` | #1778, #1796, #2243, #2246, #2247, #2268, #2286, #2289, #2290, #2294, #2296, #2297, #2305, #2306, #2323, #2324, #2325, #2332, #2367, #2370, #2371, #2374, #2376, #2379, #2407, #2408, #2410, #2412, #2414, #2415, #2441, #2455, #2501, #2502 |
| OPTIMISER | `LSP_RELEASE_OPTIMISER.md` | #1797, #2093, #2216, #2217, #2218, #2231, #2233, #2234, #2248, #2258, #2260, #2261, #2262, #2263, #2264, #2267, #2307, #2308, #2327, #2334, #2339, #2340, #2341, #2347, #2348, #2350, #2351, #2352, #2358, #2373, #2382, #2383, #2384, #2385, #2386, #2390, #2391, #2394, #2396, #2397, #2398, #2399, #2400, #2402, #2403, #2404, #2406, #2411, #2417, #2419, #2423, #2424, #2430, #2431, #2433, #2434, #2439, #2442 |
| PACKS | `LSP_RELEASE_PACKS.md` | #2130, #2137, #2138, #2181, #2242, #2259, #2270, #2299, #2303, #2309, #2365, #2366, #2372, #2422, #2446, #2447, #2448, #2449 |
| COORDINATOR | This document | #1655, #1710, #1712, #1714, #2129, #2183, #2255, #2301, #2426 |

## Current evidence and next actions

Historical naming snapshot, 4 November 2026 at 17:42 UTC: **not release-ready**. The fresh 21-control Compiler diagnostic passes 16/21. Adopted Named binding, card-fixture and prefix transport repairs remain untested; Logical namespace/channel drafts are private. Core V13 builds, then refuses before tests on one unsealed Tcl input; its separate post-build diagnostic passes 21/23 plus observation with unchanged pins, but complete prebuild custody, current-Main and parent acceptance remain false. RMW v19 starts fresh builds. New host run 41877790468 is queued after integrated Explorer Arc repair `33cf8caf26`; the previous b0ff run fails before runtime tests. Old b067 Browser 40/41 and WASI 32/33 remain source-bound failures. Host V7 private controls, lint and TypeScript pass; real-host acceptance is pending. Tcl evidence checks 321 files and passes 11 tests; full release gates and performance remain open. Shallow `work` continues reviewed five-minute syncs without force. [Current naming state](work/name-handling/current-state.md) records exact receipts, earlier failures and the restored 13-fixture cleanup incident.

- Full-suite sweep, failure clusters and fix log: [LSP_RELEASE_TEST_SWEEP.md](LSP_RELEASE_TEST_SWEEP.md).

- The initial GitHub inventory contains 203 open issues. Assignment does not mean reproduced or fixed; each lane must record its verdict and exact checks.
- The source is a shallow clone. Its current sparse checkout excludes exactly 114 Git-backed historical handoff archives; all 128,890 other tracked paths remain present, including source and regression fixtures. Any later gate needing an excluded archive must rehydrate that exact file.
- The first development sync completed at 09:13 UTC, incorporating `961f17a32` and pushing the current head. No product test result is inferred.
- Coordinator #2255: `variable_component` already enables every array-index substitution independently of the enclosing mask. Expanded its existing regression to include Tcl 8.4 and 8.5. All 586 lexer unit tests pass. A fixed native witness agrees on all five Tcl releases and Jim: command and escape substitutions inside the index remain enabled while disabled outside; disabling variables retains the whole literal operand.
- Lexer Clippy is blocked by a pre-existing `assert_is_empty` finding in `source_map.rs:917`; no lint suppression was added. Full repository gates remain pending in the separate gate-fixing work.
- Coordinator #2183: current lexer retains exact inner parse-error terms, and its nested-term/rebasing tests pass in the 586-test run. VM error-info delivery is outside this sweep.
- Coordinator #2436: the analyser now reports E204/E205 for rejected words inside command substitutions through the shared parse-cut owner. Two lexer tests and four production-analyser/reference tests pass, including nested brackets, array indices, multiple commands, duplicate suppression and valid braced-data controls. The reference tests required all five pinned Tcl releases and current JimTcl; source hashes matched before and after. The analyser tests ran through a minimal production-dependency harness containing the committed test file; the full workspace gates remain pending.
- The shared Jim reference is deliberately refreshed to verified upstream HEAD `82fb4d0caf90240d94352a31fcbd16adf52658a7` (3 November 2026), now built with revision-bearing patchlevel `0.84-10-g82fb4d0`. The earlier release-only patchlevel could not satisfy the shared reference validator. The official provisioning helper and the nested parse-cut oracle tests pass with the refreshed build.
- #2436 was closed after its [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2436#issuecomment-6630162759) was posted and fetched back. The [bounded receipt](LSP_RELEASE_PARSE_CUTS_RECEIPT_2026-11-04.json) records the exact harness, results and six source hashes.
- Targeted production Clippy for `tcl-lexer` and `tcl-compiler` passes with `-D warnings` at `c80aa092d`; this also includes the pushed #2218 optimiser guard. Full workspace gates remain separate and pending.
- #2218 was fixed in `c47d83bda`: retained O115 return/argument normalisation now obeys the shared module command-trust owner. Three focused tests pass, with eight positive and eight negative cases plus equivalent original/rewritten execution on all five Tcl releases and Jim `82fb4d0`. The [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2218#issuecomment-6630388414) was fetched back before closure.
- #2365 was fixed in `89694c328` and `d58e2952e`: valid stub directives outside a block receive W153 while W123 remains on the unknown call. Clean analyser and Core policy tests, generated-catalogue drift checks and targeted compiler/Core Clippy pass. The new KCS page/index are included; its full index gate remains blocked by 390 unrelated sparse-checkout paths. The [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2365#issuecomment-6631444932) was fetched back before closure.
- #2295 was closed after its [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2295#issuecomment-6634160183) was posted and fetched back. Four clean deterministic readiness controls pass, including a paused-clock wait beyond the former deadlines and admission/failure/shutdown checks; the real first-query regression passed on the earlier clean server in 39.57 seconds. The [receipt](LSP_RELEASE_WORKSPACE_READINESS_RECEIPT_2026-11-04.json) retains exact sources, hashes, logs and the separate unresolved source-view, lifecycle and pack failures.
- Coordinator #2129: the registry already has a `SOURCE_ENCODING` admissibility axis and declines non-ASCII text where the target’s source reader is unknown. A fresh 11-case native check confirms the Tcl 8.4–8.6 system-encoding distinction, Tcl 9.0–9.1 UTF-8 source reading and current Jim UTF-8 reading; the [receipt](LSP_RELEASE_SOURCE_ENCODING_RECEIPT_2026-11-04.json) retains exact source bytes and launchers. The design example now matches the actual `source_utf8` field and explicitly identifies the missing encoding-profile/decode provenance path. This is scoped reference evidence; #2129 remains open.
- Documentation issues #2372 and #2446–#2449 were closed after source-owner review and exact proof-comment readback. The [receipt](LSP_RELEASE_DOCUMENTATION_RECEIPT_2026-11-04.json) distinguishes documentation verification from runtime execution and records the correction commits, source hashes and comment URLs.
- #2426 typed index identity in `5738b1a10` replaces the written/proven duplicate check by IndexBounds subjects from the same original invocation owner. Five source-value tests pass, including wording/advisory-value changes and absent/independent-subject refusals. The [receipt](../rust/tcl-compiler/tests/data/index-bounds-subject-identity/README.md) records the frozen joint build and production Clippy; full test-target Clippy remains unrun. The broad issue stays open.
- Coordinator #2426: VS Code desktop and browser now select the generate-docstring action by `source.generateDocstring`, independent of its display title; disabled actions are excluded. Focused Core and TypeScript tests pass on `c7261836d`. The wider envelope/dedup work remains independently assessed; #2426 stays open.
- Values #2131: `MathFuncSpec` owns result classes and the compiler's focused classifier test passes against the frozen binary; shared API controls and Tcl/Jim observations are preserved in the [receipt](../evidence/compiler/mathfunc-result-class-2131/README.md). The public nested-expression `CompilationUnit` probe did not yield a type, so no public inference coverage is claimed. The issue awaits root's closure review.
- Central gate snapshot: `make check-tcl-evidence` passes (1,119 files; 11 tests). On source `33cf8caf26e45b222ddae40fea54df68af027e42`, `make NPROC=1 rust-check` exited 2 at the initial rustfmt check (`source_graph.rs:947`), before Clippy/xtask. The wrap and `cargo fmt --all --check` are now fixed/passing in `fe7e3f2e980db5601c2ca96d288bc40a8c38bf41` (patch SHA-256 `95684342b64d077581dd6e966628ce7dc9676e0a2d15a44d2e0b6778ad040e56`); corrected full rust-check remains unrun. `make NPROC=1 prep-pr` first failed three times with uv cache/tool-dir/network setup errors (exit 2 each), then was interrupted (exit 130) during `gen-ai-diagnostics --check`, after codegen, TypeScript typecheck/bundles and earlier editor checks passed. Smoke and remaining prep-pr checks were not reached. No JetBrains test suite was run. Full logs and SHA-256 manifest are retained under `/tmp/lsp-packs-*`; the concise receipt is `/tmp/lsp-packs-final-gate-receipt.md` (SHA-256 `fdf79b2b7e3a91e4d786aaf280110614689dbe77c961440d6b1f6a1b965697fb`). Next: rerun rust-check on `fe7e3f2e980db5601c2ca96d288bc40a8c38bf41`, then complete prep-pr after the S102 priority work; neither gate is green.
- Large design/audit issues #1655, #1710, #1712, #1714 and #2426 require explicit assessment; no blanket completion is claimed.
- #940 seed controls: fresh runs on all five C Tcl releases and current Jim reject the first increment of literal `abc`; this cannot establish a repeated conversion. Starting at `9223372036854775807` completes on all six with different displayed results, so that magnitude is not an error control. The [raw receipt](LSP_RELEASE_SHIMMER_SEED_RECEIPT_2026-11-04.json) and standalone [invalid-seed](../evidence/tcl/lsp-shimmer-incr-list-invalid-seed.md) / [large-seed](../evidence/tcl/lsp-shimmer-incr-list-wide-max-seed.md) records preserve exact scripts and streams. These are reference outcomes, not a product warning result.
- #940 native recurrence control: `set x 0; foreach i {1 2 3} {incr x; lindex $x 0}` returns 3 on all five Tcl releases and current Jim. Tcl 8.6, 9.0 and 9.1 additionally expose three integer-to-list conversions on the same object through their representation observer; older Tcl and Jim rows verify output only. The [receipt](LSP_RELEASE_SHIMMER_CYCLE_RECEIPT_2026-11-04.json) retains the script, hash and raw observations. The bounded source-only S102 projection is still pending; this is not a product fix.
- #940 authenticated native LSP follow-up: top-level S100 delivers exactly one warning for the dynamic length/list-conversion control; literal-list and both original font examples have completed empty reports. Whole-source/version/dialect/input-epoch/result identities were reborrowed as current. Suppression and severity controls pass. The [protocol receipt](LSP_RELEASE_S100_PROTOCOL_RECEIPT.json) records the frozen source and disabled local trace patch. The original external proof comment was updated and fetched back; conditional S102 and named-procedure support remain unresolved.
- #2376 was closed after its [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2376#issuecomment-6638347447) was posted and fetched back. Read-local guards in `07a1bd0ac` pass 185 integration rows and 14 guard cases. `62553e86f` corrects two fixtures to genuinely conditional children without changing their refusal assertions; all nine query tests pass. The frozen joint unit build, production compiler Clippy and normal library build pass at `f5b2f7d05` plus patch `da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`. The [receipt](../rust/tcl-compiler/tests/data/w210-query-purpose-review/verified-fixtures/README.md) records source custody and the after-load/actual-entry distinction. Full workspace gates remain unfinished.
- Next: run fresh focused regressions, fix residual defects, preserve positive and refusal cases, update lane evidence, and continue scheduled integration.
- The corrected optimiser differential cohort passes 44/44 on the applicable pinned Tcl releases at `1c15d4ff5`; expanded controls and newer changes require their own results.
- Full locked Cargo metadata succeeds for the LSP server, Spec Studio and Explorer WASM hosts. This verifies those lockfiles, not a WASM binary or editor package build.


## External comment validation completed

All 19 requested external issue comments were posted and verified on GitHub;
#2021 was excluded. [Comment index and outcomes](LSP_RELEASE_EXTERNAL.md).
The comments distinguish passing scoped checks from genuine-control failures.
The source-owner/shimmer gap remains unresolved for #940 and related reports;
no speculative authority or parser workaround was committed.


Naming status at 22:57 UTC: **paused at user request; not release-ready**. The current autoload fix is complete:9/9 focused controls and three consecutive actual public definitions pass; cursor6/7 public definitions and hovers pass. Browser44+11 and WASI27 Wasmtime units+35 host checks pass on captured CI source; newer run41919112254 Browser/WASI jobs also succeed, while Native remains pending at the last read. SpecTcl/Registry17/19 retains the CR-value and mixed-emitter failures. Core45 ran zero tests after ENOSPC. Counted methods, pattern occupancy, O127, VM11, full gates/packages and release performance remain open. [Current state](work/name-handling/current-state.md) and its final validation receipt record exact limits. All other work is stopped; source/status checkpoints are published to work and the periodic publisher is stopped.

Baseline compiler lint repair, 19:36 UTC: the two oversized Logical source
walkers are split into their existing static body/target selection and typed
effect/transition checks. Guards, state transitions and Native/Normal boundaries
are preserved, including the concurrent delegated-script effect selection;
no lint suppression is added. The reviewed patch SHA-256 is
`19dc230cd86eb4164244ea44c55e57f73ebae76322189d60310987ed91281256`.
Standalone rustfmt and diff checks pass. Compilation, production Clippy and
runtime regression checks are pending the priority #940 joint build; this is
not a claim that either repository gate is green.

The first joint compiler build after this extraction exited 101 before tests
or Clippy: the namespace helper declared `String` for the structured
`NamespaceTransitionTarget`. The return signature is corrected to retain that
shared type. The priority #940 joint build must rerun before any compile or
lint success is claimed.

#940 phase8, 19:53 UTC: the corrected compiler unit/integration no-run build
passes at `6c97a1cdc54ab089f6a4c9e7086177c7106ec531` plus private patch
`89d75e8f0100e49c055525d6dd12d7ca025851e61752f285ed438cde5c8ddd5a`.
The genuine fixed-count increment/list cycle reports exactly one conditional
S102 through both the direct projection and `run_all_checks`, with no action
authority, across Tcl 8.4–9.1. Provenance/refusal runtimes are still running.
Production Clippy fails three style findings: the candidate's 102-line
`representation_plan`, unchanged `retain_logical_callback_candidates`, and
an unchanged nonminimal boolean in `registry_invocation.rs`. Corrections are
in progress. The original conditional-lappend report remains unsupported;
native LSP S102 delivery and full release gates are unverified. This is a
scoped positive control, not a #940 fix claim.

The two unchanged phase8 lint blockers have a reviewed mechanical correction:
callback candidate setup retains its original interpreter/role and declared-scope
checks in small helpers, and the declaration-name predicate uses its equivalent
De Morgan form. Per-operand assembly, refusal behavior and shared word-owner
checks are unchanged. Patch SHA-256
`584d46440cc809c1aceffb4dd34b198cf1a7464b142542bd89fc3bca933a6f1f`;
standalone rustfmt and diff checks pass. Production Clippy is pending a fresh
run; no full gate result is inferred.

The four upstream phase10 size findings have a mechanical correction: trace
formatting and source-index preparation use small existing-state helpers;
diagnostic checkpoints keep their original order; supplied-frame updates and
the discarded-root opacity predicate retain the same typed inputs and guards.
No lint suppression or parsing behavior is added. Standalone rustfmt and diff
checks pass; fresh production Clippy and interruption/runtime checks are pending.

The scoped #940 recurrence foundation is published in `cab14d1a4`. Its clean
native server build and eleven authenticated Tcl 8.6 protocol cases pass:
the genuine cycle delivers S102 at the original `$x` range, with matching
suppression, shimmer-off and severity controls. The generic versioned noqa
action remains available; no source-context compiler rewrite is granted.
The [protocol receipt](LSP_RELEASE_S102_PROTOCOL_RECEIPT.json) records exact
source/binary hashes and publication identities. The existing external proof
comment was updated and read back exactly. The original conditional-append
example remains coverage-inconclusive, and #940 stays open. Clean foundation
production compiler/registry Clippy subsequently passes at `987e6a73a`.

The adjacent taint seed-owner repair is published in `f2b927c5c`: incoming
external globals are selected by retained cell/entry identity, and independent
taint provenance survives conservative global-write seeding. Production Clippy,
compiler test build and twelve focused seed/interruption controls pass. The
exact IMAP4 diagnostic-read coverage blocker remains; #2243 is open. Full
repository gates remain unfinished.

## Issue-sweep lane (bitwisecook issues, easiest first), 6 November 2026

Closed after verification (proof comment on each): #2082, #2216, #2266,
#2272, #2291, #2293, #2301, #2302, #2312, #2338, #2343, #2349, #2378, #2402,
#2430, #2439, #2445.

### Blocking regression: one proven-dead body empties constant propagation

`set n 0; if {$n} {puts a} else {puts b}` gives `n#1 = overdefined`, so
I230/W240/O100/O102 never fire. The chain, traced at HEAD:

1. The source walk decides the `if` (or proves `$x` raises) and never
   enters the dead arm, so nothing in it receives a source binding.
2. Lowering finds no namespace for that body. In authored simulation (no
   native entry, no Logical name context), `original_body_namespace_context`
   requires one of those and declines, so the arm becomes the barrier
   `original body namespace context is unavailable`. Giving authored input
   the parent's own closed namespace there was tried: the body then lowers,
   but each call in it still has no binding.
3. `cfg_builder::source_upvar_effects` gets `None` from
   `procedure_summary_owners.calls` for an unbound call and emits
   `<cmd> upvar-aliases a dynamic caller variable`.
4. `sccp::escaping_names` takes `var_observability`'s flow-insensitive union
   over every block, dead ones included, so every variable escapes and every
   typed assignment is overdefined. (`typed_assignment_trusted` stays true;
   `set` trust is not the cause.)

A dead arm anywhere poisons the whole module, even inside a procedure that
is never called (`proc p {} {if {$x} {puts a}}`). First-parent bisect:
good at `84ae94c7f5`, bad at `e803f53f1e`; between them is the unbuildable
merge `c49ef6fd5d` (490 side-branch commits), whose side tip does not build
either. The fix belongs to the naming lane: the walk should record bindings
for an arm it proves dead under the arm's entry state without merging that
state, or the escape union should exclude blocks proven unentered.

Consequence: the `release_optimiser_safety` witnesses (62/66 pass on real
tclsh 8.4–9.1) only show preservation while nothing folds, so the miscompile
issues they cover stay open until this is fixed and they rerun. Residual
witness failures: #2248, the `after 0 …; update` variant of #2308, and the
I230/W240 controls (`issue_2341`, `issue_2384_computed_false_loop_is_reported`).
#2341's message fix is verified on `if {0}` and waits for its control. The
#2375 classifier change (I230/I231 by the branch's own targets) is local and
unverifiable until constants flow again.

### Other open findings

- #2309: the false positive is gone, but `socket -server accept` (no port)
  draws no E002 and its own regression test fails. Selecting the spelled
  server form in `pick_form` gives a false E003 on the valid form, so the
  server form's option counting is the defect.
- `tclvm --tcl-version 8.4` does not start (`no library has been specified
  for Tcl` from `info library`); per-release VM e2e harnesses fail
  `NativePreflightRequired` (sweep cluster 13).
- BPF: `while` now reaches `OutOfSubset` before loop detection
  (`bpf-tcl/tests/loops.rs::native_while_still_rejected` fails).
- #2253: `cargo xtask dialect-drift` reports 51 sites; wiring the gate needs
  each site threaded first.
