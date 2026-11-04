# LSP release recovery and external issue report

Snapshot: 4 November 2026, after the execution environment became unavailable.
The last remotely inspected work tip was `45c90293f70389a1c3dfa261948123d9d09cbd61`.
This document is saved through the connected GitHub API because shell execution,
builds and local pull/push integration are unavailable. No release-readiness
claim follows from this checkpoint.

## Active instructions

Work continues on `work`, integrating at least every 15 minutes while execution
is available. Scope is LSP and compiler dependencies, excluding separate VM,
BIG-IP, f5tcl and unrelated tooling work. Use shared naming owners and existing
child Tcl parsers/lexers. Oracles are Tcl 8.4–9.1 and latest JimTcl.
The user revoked the previously supplied AGENTS.md instructions when the
environment became unavailable.

The user made reopened #940 the highest priority and authorised evidence comments
and closure only for verified fixed issues authored by bitwisecook. All other
reporters' issues stay open and require a status report in the chat.

## Highest priority: #940

The new report is the conditional-append case:

```tcl
set ys {}
set pts {}
foreach pt {{1 2} {3 4} {5 6}} {
    set y [lindex $pt 1]
    if {$y > 2} {
        lappend ys $y
        lappend pts $pt
    }
}
```

Native execution agrees on all five Tcl releases and Jim: ys is `4 6` and
pts is `{3 4} {5 6}`. Tcl 8.6's representation trace shows initially pure
strings becoming lists at the first taken branch and remaining lists thereafter;
Tcl 9.0/9.1 show lists throughout.

**Not verified fixed.** An initial compiler harness lacked retained source
metadata and was invalid. The corrected-input probe has ShimmerContext and
returns no S102 for the report, but also misses three genuine positive controls:
header SSA types become Overdefined. Investigate that before claiming resolution.
The original braced-fontSizes and list-command variants are both quiet too.

Values' temporary production-only probe avoids unrelated dev-dependency Core
compilation. One Core E0599 is attributable to inconsistent source/artifacts:
current remote work has both the caller and the compiler owner method
`OriginalSourceCutBodyLayout::source_command_reference`. The method landed in
`1fa97847b1bb`; the caller changed in `63179cc6a9d7`. Compare exact local
sources and rebuild rather than adding another name/parser implementation.

## All open issues from other reporters

The GitHub query `is:issue is:open -author:bitwisecook` returned these 20 issues.
All remain open. Existing test names alone do not count as executed validation.

| Issue | Reporter | Last defensible status |
| --- | --- | --- |
| #940 | nico-robert | Highest priority; inconclusive. Negative repro quiet, but S102 positive controls also quiet. |
| #2021 | yarko18 | Two real-process EOF and shutdown/exit-during-scan regressions pass at 9d9140a60. Original extension-update/orphan incident not reproduced. |
| #2181 | torbenmu | Nested-body and association/reload source coverage exists; fresh protocol/editor execution pending. |
| #2242 | Ammar-Kasem | Exact custom-pack arity publication E2E exists; fresh execution pending. |
| #2265 | torbenmu | Fresh exact nested-expression semantic-token-span regression passes. |
| #2286 | georgtree | All four reported tcltest fixture shapes are clean of W210 in fresh compiler baseline 961f17a32; later focused audit also passes these controls. |
| #2288 | georgtree | Additive bind callback case awaiting fresh focused verification. |
| #2289 | georgtree | Existing procedure-created child-interpreter witness passes; final full linked-file assessment pending. |
| #2290 | georgtree | Latest linked tclmeasure file is clean of W210 in the fresh compiler baseline; unrelated W103 remains. |
| #2497 | nico-robert | Fresh inherited-my one-reference Code Lens and References agreement test passes. |
| #2498 | nico-robert | Numeric lindex value: source coverage exists; valid fresh per-report execution pending. |
| #2499 | nico-robert | Dynamic variable-name read: source coverage exists; fresh execution pending. |
| #2500 | nico-robert | Negative infinity: source coverage exists; valid fresh representation check pending. |
| #2501 | nico-robert | Exact regexp/lassign-loop report is clean of W210 in fresh focused validation. |
| #2502 | nico-robert | Exact dict-with-variable report is clean of W210 in fresh focused validation. |
| #2503 | nico-robert | Nested expr in quoted result: source coverage exists; fresh execution pending. |
| #2504 | nico-robert | Numeric foreach binder: source coverage exists; valid fresh representation check pending. |
| #2505 | nico-robert | Multiline list callback: source coverage exists; fresh execution pending. |
| #2506 | nico-robert | Boolean-result taint: source coverage exists; fresh execution pending. |
| #2507 | nico-robert | Hex conversion scan recommendation: source coverage exists; fresh execution pending. |

## Verified closures and other results

Evidence comments were posted and bitwisecook-authored issues #2254, #2255,
#2334, #2351 and #2434 were closed. No other-reporter issue was closed.

- Lexer library: 586/586 pass after expanding the array-index-mask regression to
  every supported Tcl release and Jim. Native mask witness agrees on all six
  interpreters. Production lexer Clippy passes; test-target Clippy had a
  pre-existing assert_is_empty finding.
- Optimiser: corrected initial 44/44 fixed-input differential witnesses pass on
  applicable Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 at 1c15d4ff5.
  Expanded 59-test acceptance suite is committed but remains unverified.
- #2440 W121 enclosing-body fix cb6f0e2c3 passes its focused integration test
  before the final rebase; docs follow-up dbbf93907 is pushed.
- #1657 admission fix be314fd8d passes its focused regression, but it mitigates
  one wedge mechanism and does not prove every underlying wedge fixed.
- #2295 originally reproduces: first symbol query returns [] after scan budget
  expires during system package indexing. Reviewed cancellable readiness/failure/
  shutdown fix is pushed in 6ae635398; formatting passes, tests remain unrun.
- Fresh binding audit: 13/17 pass. #2376 fix passes locally. Residuals #2246,
  #2371, #2323 and #2294 remain; do not close them.
- #2299/#2270 registry changes are pushed; security-floor test began compilation
  but execution disconnected before a result. Treat as pending.
- KCS corrections for #2446–#2449 and #2372 are pushed; diff checks pass.
- Full locked Cargo metadata succeeds for LSP Server, Spec Studio and Explorer
  WASM hosts. This is not a WASM binary or editor-package build.
- Core hover suite: focused new production controls and #2497/#2265 pass;
  broader suite has 150 pass/30 existing failures. Full release gates are not green.

Tcl oracle binaries/sources are under
`/workspace/tcl-lsp/tmp/lsp-oracles`. Latest Jim commit is
`82fb4d0caf90240d94352a31fcbd16adf52658a7` (reports 0.84).
Exact Tcllib 2.0 corpus is
`/workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0`.

## Local recovery inventory

Inspect every checkout before mutation; preserve stashes, untracked tests and
existing rebase state. Do not reset/clean blindly. Targets are isolated per
checkout; `/tmp/lsp-release-env.sh` holds oracle exports and one-job defaults.
Future heavy builds should use `python3 /tmp/lsp-heavy-build.py cargo ...`
to limit concurrent compilers without sharing Cargo targets.

- Coordinator: `/workspace/tcl-lsp/tmp/lsp-release-work` was detached during
  an interrupted rebase at 6ae635398. Modified `docs/LSP_RELEASE_STATUS.md`
  and untracked `docs/LSP_RELEASE_EXTERNAL.md` remain. A docs-only binding
  checkpoint was adopted locally but publication did not complete; its exact
  patch is `/tmp/lsp-bindings-checkpoint.patch` (original commit 764b2c06c).
  Inspect `git status` and rebase state before continuing.
- Values: `lsp-values` has #2436 in stash `wip nested syntax diagnostic 2436`.
  Two lexer tests pass; compiler integration pending. #940 candidate is only
  `/tmp/lsp-940-candidate.patch`; untracked
  `rust/tcl-compiler/tests/lsp_shimmer_940.rs` contains the probe/controls.
  Logs: `/tmp/lsp-values-940-probe-controls.log`,
  `/tmp/lsp-values-940-ssa.log`. Avoid full Debug of recursively retained
  metadata: a trace produced excessive output before transport loss. Whether a
  probe remains running is unknown; inspect `lsp_values_release_probe` on recovery.
  Latest status commit was local after GitHub 500/race failures.
- Bindings: `lsp-bindings`, HEAD 764b2c06c over 738e7717d. Verified #2376
  changes in `command_binding.rs`; unfinished #2371 changes in
  `dataflow.rs`/`helpers.rs`. Known tuple destructuring at helpers.rs:697
  still needs the third field. Untracked `lsp_binding_release.rs` includes
  failing/debug cases; separate those before committing a verified fix.
- Optimiser: `lsp-optimiser` has uncommitted #2373 CLI dialect fallback,
  input/CLI tests and KCS updates; manager applied-index/report-order follow-up;
  Tcllib test-path fallback. Last pushed cohort e6259d3e9; expanded tests
  0ef58558b unverified. Evidence JSON: `/tmp/lsp-optimiser-evidence.json`.
- Packs: `lsp-packs` has unfinished #2259 VarCell work stashed. Last security
  test compiled registry and began compiler before disconnect; no pass recorded.
- Providers: `lsp-providers` has local docs-only 677833737; prior pushes through
  fc1cb1e4b. #2143 Rust integrations remain unrun. #1956/#1655 require a genuine
  Clay grammar/completion/argument-binding design and remain open; no name-specific
  shortcut was introduced.
- Server: #2295 changes pushed through 6ae635398; delayed readiness, failure,
  cancellation and shutdown tests require execution. Consider virtual/paused time
  or another deterministic control that actually detects the former 10-second
  timeout; a short 20ms delay alone does not.

The execution environment must be restored before remaining fixes can be
validated. Repository metadata reporting running did not restore shell access;
exec-server rejected the lost session.
