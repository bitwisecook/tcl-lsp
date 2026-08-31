# TclVM conformance-foundation lane

## Goal

Give every TclVM issue lane one shared, release-aware C Tcl oracle and upstream
source discovery layer, plus a focused Tcltest runner that can select exact
test IDs without rewriting the committed full-suite baseline.

## Decisions

- Tcl 9.0 remains the gold standard and full scoreboard runs remain pinned to
  `TclVersion::V9_0.patchlevel()`.
- A focused run may use another 9.0 patchlevel only when its interpreter and
  source suite match exactly.
- Oracle output is byte-valued at the shared boundary. Text conversion is an
  explicit strict projection for consumers whose contracts are text.
- Discovery belongs in a low-dependency `tcl-test-support` crate. Semantic
  assertions and backend-specific execution remain in their owning crates.
- `--stem` is repeatable and `--match` is forwarded identically to Tcltest in
  C Tcl and the VM runner.

## Site inventory

- [x] Add `tcl-test-support` and the workspace dependency.
- [x] Replace the syntax conformance suite's local interpreter discovery.
- [x] Replace xtask's hardcoded Tcl 9 tree/interpreter discovery.
- [x] Add exact-patch validation and focused/full artifact rules.
- [x] Add match filtering to xtask and the VM Tcltest runner.
- [x] Document the owner, architecture, and contributor workflow.
- [x] Run focused Tcl 9 oracle/VM checks and the workspace compile gate.
- [ ] Remove this lane file when the PR is finalized.

## Behavioural deltas

- `cargo xtask tcltest-sweep --stem` may be repeated.
- `--match` selects Tcltest IDs and requires a focused run.
- `--tcl-root` selects an explicit upstream tree.
- Full baselines cannot be regenerated from a different 9.0 patch release.
- An explicitly configured missing or mismatched interpreter/source now fails
  instead of silently falling through.

The focused Tcl 9.0.4 `parse-18.*` probe selected the same 271-test Tcltest
universe on both backends: C passed all 30 selected tests and skipped 241; the
VM exposed the existing parse/substitution failures (25 passed, 241 skipped,
5 failed). This validates selection plumbing without treating an existing VM
conformance gap as a harness failure.

## Open uncertainties

- None. Backend-specific normalization must be added beside the relevant
  backend rather than broadening the raw shared oracle boundary.
