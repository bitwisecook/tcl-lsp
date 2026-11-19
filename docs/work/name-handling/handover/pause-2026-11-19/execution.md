# Execution handoff, 19 November 2026

Owner paused this queue with published `work` at `b528174d13e0ceee1284c9f8a6cf20bbc4230fa2`. No PR. The coordinator also has local integration work ahead of that SHA. Keep the selected C or Jim execution authority. F5 device questions go in the existing BIG-IP ledger. The physical iRule simulator uses Tcl 9.1.

## Measured and integrated

| Cause | Product entry and evidence |
|---|---|
| C 8.5 command marker | Clean VM compiler missed the `startCommand` at PC 6 for the top-level script `set y 1; return $x`. The selected candidate matches C 8.5.19's marker and continuation. |
| C 8.6 and later condition and unreachable branch | For a compiled proc containing `if {!$a} {return 1}; return 2`, C 8.6.18 and 9.0.4 invert the branch and fill two dead bytes. C 9.1.0 fills five. PC and mnemonic sequences agree in all 52 matched script/procedure controls on C 8.5 through 9.1. The candidate used `b1ee491dc8` codegen sources with the coherent `8d6ae9adde` compiled dependencies. The separate clean-before capture used `a6c66517c7`. The published changes are `423b1d8a03` and `850de4787d`. |
| C 8.4 source error trace | `26d55332ed` adapts the counted file-source entry. The original file-source and direct-eval entries legitimately report different trace context. Keep that distinction for new comparisons. |
| Other areas | Published work contains C 8.5 lrepeat width/usage, selected channel escape handling, Thread 2.8.13 tpool/tsv conversions, Jim dictionary scope self-key/empty-body policy, selected runtime try error causes, source quoting, the `tcl-value-syntax` extraction and the selected `WordValueRules` escape fix. Spec Studio's public package importer subsequently matched all six interpreters for `\x41b`. |

Clean `a6c66517c71145252533d5e35a2c5db200803ecd`, `tcl-codegen` integration `it_3` had 279 tests, 254 passed and 25 failed with all three oracle-required flags set. The complete test names, results, identity, four original disassemblies and 52 selected rows are in [execution evidence](evidence/execution). The full execution-area suite still needs a clean-head run on the new head.

## Filesystem work saved at pause

Apply each missing change in the order below. Coordinator-local `49d6c69a38` and `087c64af94` already carry the first two. Published `b528174d13` did not include them. Identify already-integrated changes before using the archived patches.

| Change | Exact worker parent | Saved worker SHA | Patch |
|---|---|---|---|
| Native switch and `-types` grammar with Unix filesystem permissions | `85a60c277dac0e39af53cadd3bcb12f70314346f` | `b367e881f4fb68012185ea2b0b36614bdee2d4e8` | [Options](patches/execution-glob-options.patch) |
| C Unix section scanner, alternatives, selected matcher and both entry adapters | `b367e881f4fb68012185ea2b0b36614bdee2d4e8` | `ab95c8766f33e7bdb8bb72d16870a9eda9b96fb7` | [C traversal](patches/execution-glob-c-unix.patch) |
| Latest Jim eager file-pattern scanner and walker; retire the old per-engine walkers | `ab95c8766f33e7bdb8bb72d16870a9eda9b96fb7` | `79d4303a3325f8e86b9512f065a9ec053f1c4501` | [Pending Jim integration](patches/execution-glob-jim-unix.patch) |

Directly exercising the new walker through the production NativeHost filesystem matched 145/145 C rows and 48/48 Jim rows. Examples include `{alpha,beta}`, `alp\ha`, duplicate alternatives, hidden files, slash spelling and a malformed suffix beneath an unmatched wildcard. Each row retains the expected and actual result bytes in the evidence directory. The Jim matcher uses its installed library's whole-pattern expansion and its own string matcher. The editor can use `c_glob_head` and `jim_glob_tokens` as Unix syntax views.

The original release-selected VM CLI for every C release and the original Rust runtime ABI on its default C 9.0 both returned list length 0 for `glob {}`. All five C interpreters return length 1 with first element `.`. The latest Jim returns length 1 with a zero-length first element. Strict library clippy passed for `tcl-cmd-core` and `tcl-host-native`. The standalone lockfile check passed at the options increment.

**Next verification**. Compile both normal guest entry points with the final Jim patch. Run the original switch and type fixtures, check the empty list/first-element witness and the matrix under each selected runtime and VM implementation. Recheck all standalone locks on the final head, format and clippy all targets, then run the canonical gates before push. The root applies or publishes the gated stack. The final combined guest binaries and full gates were pending at pause.

The shared filesystem still exposes UTF-8 directory entries and narrow native path APIs. Verify raw Unix names and selected external encoding before making byte-preserving filesystem claims. `-tails -directory DIR {}` gave endpoint-specific outputs and should be proved independently. Windows and Apple filesystem-specific rules also need independent evidence. Jim's `file dirname`, `file tail` and `file join` should reuse one native path owner with the Jim walker.

## Other work parked

- `tcl-vm-wasm` had two actual failures on the clean six-oracle tooling sweep. Compare Document/file decoding and direct counted ABI input separately, then trace the browser entry. The source/channel ruling is in `current-state.md` and `parser-audit.md`.
- Recheck the stale original command-name cache after an engine move (`profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations`), the compile-local frame selector, the original runtime `try_compiles_script_word` fallback and native error-code trap object conversion. The two repeated list reads have separate entries in the evidence.
- 61 direct caller dispositions and 1,092 structural leads are archived. The latter are source triage. They do not establish product equivalence.
- On resume rebuild Jim using `bash scripts/dev/ensure-jim-oracle.sh`. Pin every original, require all three oracle flags, then baseline on clean `origin/work`. Retain exact C 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and current Jim identities with the run. The pause used Jim `0.84-15-g2f8e155`.
