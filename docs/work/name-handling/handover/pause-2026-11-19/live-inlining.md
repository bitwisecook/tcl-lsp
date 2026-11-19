# Live-inlining handoff, 19 November 2026

The owner approved enabling live reference inlining after the pack-backed proof. `LIVE_REFERENCE_INLINING` was still false at pause. No new campaign or fresh off/on build completed in this session. Keep the November 18 measurements as historical evidence.

Both paused ClausePlan patches were initially reapplied at `b1ee491dc825879cd8f638a396db97cf67a47bc3` as `dd826bb0d4` and `5ccf70d095`. The matched campaign has since moved to the newly published `b528174d13e0ceee1284c9f8a6cf20bbc4230fa2`. Keep the original baseline evidence separate.

| Order | Change | Exact parent | Worker SHA |
|---|---|---|---|
| 1 | [Lower admitted structured control in a reference body](patches/live-inlining-clauseplan-lowering.patch) | `b528174d13e0ceee1284c9f8a6cf20bbc4230fa2` | `9ba7e59b91e0804f86d8bb4b3cac2ebf8d3bfee4` |
| 2 | [Use the admitted compiler dialect and realm](patches/live-inlining-clauseplan-release-surface.patch) | `9ba7e59b91e0804f86d8bb4b3cac2ebf8d3bfee4` | `660b435dffd2b284b45fef24c149b8d6269a7afa` |

Analyser-binding reviewed the fallback and retained conditional approval. Only a pack ReferenceBody with an admitted original native compiler invocation may supply a fallback when logical resolution declines. The word dialect, compiler dialect and compiler realm remain distinct. C 8.5 `try` stays a call. C 8.4 and Jim do not admit native live splices under the current eligibility rules.

## Resume with the matched campaign

1. Prepare one clean checkout on `work` and a second on the same published source, with the two patches atop it. Rebuild Jim and export all five C oracles, latest Jim and the three required flags. Record the revisions, original versions and selected entries. The original working copies were `/tmp/bd-execution` and `/workspace/bd-exec-live`.
2. Set `LIVE_CLEAN_CHECKOUT`, `LIVE_PATCH_CHECKOUT` and `LIVE_EVIDENCE_DIR` to absolute paths. Coordinate the single heavy build slot with DB and compiler owners. Run the [campaign driver](evidence/live-inlining/campaign.sh) with argument `before` for the pristine switch-off reference, then with argument `after` for the two patched configurations. The driver records exit status and full logs for every run and restores tracked files. Inspect every status file and disagreement.
3. The registered `tcl-spectcl` it_5 normal `codegen_stamps::live_differential` fixture installs an actual pack, compiles through a live VM, records splices and fallback recompiles, and compares error and return state with each C interpreter. The [seeded generator](evidence/live-inlining/campaign.py) extends that same fixture with 72 generated caller/input groups, seed 20261119. Computed arguments, errors, structured `if`, variable calls, and the declined loop family are represented. The generated rows are fixtures awaiting execution. Confirm a positive count of actual splices on eligible releases. Confirm the declared refusal on Jim separately. Earlier pack-free fuzzing supplied no coverage here.
4. Diff all five C release results by case, and compare the full compiler, codegen and SpecTcl failing test names between switch positions. Record genuine preexisting failures apart from any new disagreement. The [switch-only patch](patches/live-inlining-switch-on-verification-only.patch) is for verification on the patched tree. Change the guard test per the scrub ruling and land the production switch in a separate checked commit after parity, loaded-pack coverage, clippy and full gates. Notify analyser-binding and the coordinator with the SHA.
5. The deferred 142-cell matrix cut and 13 REWRITE / 4 FIX-EXPECTATION rows remain as recorded in the November 18 handoff. Enumerate any removed cell pairs before collapsing them. The VM `info cmdcount` gap and trace-free argument proof remain separate dependencies.

The clean and patched checkout identities at each checkpoint are stored beside the driver. No agent was compiling these queues at pause, and no PR was opened.
