# Lane F6 — #1600 / #1651 working notes

Scratch file for continuity across container rollbacks. **Delete before the PR
is opened**; its content becomes the PR body.

## Status

| item | state |
|---|---|
| #1651 root cause | found, fixed, unit-test pinned, mutation-verified |
| #1600 root cause | found, fixed, unit-test pinned |
| ext-host suite, fixed tree | **920 passing, 0 failing, 1 pending** (single-root) + **14/14** (multi-root) |
| ext-host suite, baseline (epoch check removed only) | **651 passing, 8 failing, 241 pending** — #1651 reproduced verbatim (`got [O111,O120]`) *and* the run truncated |
| pre-push gates (fmt/clippy/check/e2e) | fmt clean both trees; clippy/check/e2e outstanding |

### A/B evidence

Same tree, same host, same binary path, same harness; the only difference is
whether `|| slot.inputs_epoch != epoch` is present in
`schedule_diagnostics_impl`.

| arm | runs | optimiser-test failures | runs truncated by the wedge latch | clean 920/0/1 |
|---|---|---|---|---|
| baseline | 5 | **1** (`got [O111,O120]`, verbatim) | **1** (241 skipped) | 3 |
| fixed | 4 (so far) | 0 | 0 | 4 |

Honest caveat: on a *quiet* box the baseline is green too — the failure needs
the pack-discovery walk to be slow (cold page cache and/or concurrent load),
which is exactly what #1600's own comment reports ("0-in-4 when run alone",
"2 in 4" under four concurrent build lanes). So the ext-suite numbers show the
fix does not regress and do reproduce the reported failure at baseline; the
*deterministic* proof of the mechanism is the mutation-verified Rust unit test,
which does not depend on timing at all.

Also of note: the two failing baseline runs each carried a batch of
`SpecTcl pack torture` failures, and the loop's three quiet baseline runs did
not — so those are load artifacts of the same kind, not caused by the probe
change (the same probe change was in place for every green run in both arms).

The 241 pending in the baseline run is the wedge latch truncating the rest
(`index.ts:124` `skipWhenServerWedged` is the only such skip site in the suite),
so **both** halves of #1600 are reachable from load on this host.

**Baseline run 2** (full capture, `scratchpad/baseline-full.log`) is the more
informative one and directly supports the #1600 fix:

* the `optimiser.enabled` test **passed** — so the #1651 failure is
  load-dependent even at baseline, matching #1600's own "2 in 4". Single runs
  prove nothing; rates do. Loops are running.
* seven `SpecTcl pack torture` tests failed, and the failing probe is
  `a hover on an undriven document`, timing out at **57803ms** with
  `PROBE: could not confirm starvation` — while `getEffectiveConfig("")`, asked
  immediately before it in the same helper, **answered**.
* `SERVER WEDGED` appears **zero** times in that log, and the run continued
  past the pack suite.

That is the #1600 fix working, observed live: transport alive + document
pipeline stuck is now reported (and tolerated) as what it is. With the old
probe, the same situation would have had `getEffectiveConfig(docUri)` blocked on
the very same stalled pipeline, all three probes failing, `SERVER WEDGED`
latching, and the remainder of the run skipped.

Open: whether the pack-torture stall itself is pre-existing (likely; it is the
condition that suite exists to catch) or somehow arm-dependent. The fixed arm's
single run had it green. Rates needed — see the loops.

## #1651 (and #1600 item 1) — the optimiser-disable test

**Not** a client-side marker-versus-publish race (the hypothesis in #1600's
comments). The server emits `[timing] deep diagnostics` *after*
`cache_and_deliver`, over the same single-writer `stdio_pump` FIFO, so the
publish provably precedes the marker on the wire. The set the test reads really
does contain O111/O120 — it was computed with the optimiser still on.

Mechanism — a stale config cache in the diagnostics scheduler:

* `schedule_diagnostics` (edit path) passes `force_refresh: false`, and
  `schedule_diagnostics_impl` only re-resolved `DiagInputs` when
  `slot.latest_inputs.is_none()`. So an ordinary keystroke reuses whatever
  config was resolved at the *previous* schedule.
* `run_config_reload` leaves a window where that is wrong:
  1. `pull_and_apply_config` → `apply_global_toggles` sets
     `*self.optimiser_enabled = false`. **From this instant
     `tcl-lsp.getEffectiveConfig` reports `optimiser_enabled: false`** (it reads
     that very mutex, `lib.rs` ~11364).
  2. `reload_spec_packs(Config)` — a filesystem discovery walk.
  3. `reschedule_all_open_documents()` — the only thing that refreshes
     `slot.latest_inputs`.
* The test's barrier is `waitForEffectiveConfig(optimiser_enabled === false)`,
  satisfied at step 1. Its post-toggle edit therefore lands inside 1→3 and
  schedules analysis from pre-apply inputs → O-codes republished → assertion
  fails. Step 3 lands moments later and republishes clean, which is why the
  residue is always "the old set", never a wrong or partial one.

**Local vs CI** is step 2: the pack-discovery walk is longer on the loaded
local box under xvfb than on the CI runner, so the window is wide enough to
catch the edit almost every time locally and rarely on CI. Nothing about the
test is local-only.

Not just a test bug — a real user hits it: disable the optimiser, type a
character inside the reload window, and the O-codes come back for that publish.

**Fix** (`rust/tcl-lsp-server/src/lib.rs`): `Backend::diag_inputs_epoch`
(`AtomicU64`) + `DiagSlot::inputs_epoch`. `schedule_diagnostics_impl`
re-resolves when the stamp is stale, whether or not `force_refresh` was asked
for. `invalidate_diag_inputs()` is called from every path that writes state
`diag_inputs` reads: `apply_global_config`, `apply_folder_configs`,
`did_change_configuration`'s inline writes, and a `reload_spec_packs` that
changed the set.

Pinned by `an_edit_during_a_config_apply_re_resolves_its_diagnostics_inputs`.
Mutation-verified: dropping `|| slot.inputs_epoch != epoch` fails it.

## #1600 item 2 — the SERVER WEDGED liveness verdict

Two independent defects, both in the harness.

**(a) The "document-free" probe was not document-free.**
`serverLivenessDiagnostic` passed `stalledUri` to
`tcl-lsp.getEffectiveConfig`, while `classifyLiveness` reports that probe as
"a request that touches no document". Lock-acquisition table for
`get_effective_config_command`:

| step | `parsed_uri = Some(doc)` | `parsed_uri = None` (`""`) |
|---|---|---|
| `read_document` → `edits_settled()` | **global `EditOrder` barrier**, unbounded | not called |
| `read_document` → `documents.lock()` | yes | no |
| `resolved_db_config(uri)` + `db.lock()` | **salsa db mutex** (held by `set_text` while it waits out open reads) | no — `analyser_config()`, two leaf mutexes |
| `resolved_docstring_style(uri)` | per-URI folder fold | direct settings lock |
| everything else | leaf mutexes | leaf mutexes |

Every lock in the `Some` column can be held by exactly the condition the
diagnostic is trying to classify, so "the server answers nothing" was reachable
from a server answering everything else. The in-repo precedent for the right
form is `serverProbe.ts`'s `probeServer`, whose docstring makes the same claim
and passes `""` — and the extension's own pack sync also passes `""`.

**(b) The latch drew a terminal conclusion from contradicted evidence.**
`latchFromOutcomes` armed on `!transport.answered` alone. The latch makes
`index.ts`'s root `beforeEach` skip *every remaining test* — that is the "can
truncate whole runs" of #1600's title (687/899, then 212 skipped; a
byte-identical re-run passed 899/899). But an answer from either hover probe is
a reply that crossed the whole client → server → client path, which contradicts
"the server answers nothing" outright. Now: `classifyLiveness` returns
`DOCUMENT-FREE REQUEST SLOW, TRANSPORT ALIVE` for that combination, and the
latch requires all three probes to have failed.

**Is a genuine wedge path still open?** The remaining channel is
`DeferredConcurrency` (`transport_liveness.rs`), which admits only
`DEFAULT_HANDLER_CONCURRENCY = 4` ordinary requests; a handler parked in
`edits_settled()` holds its permit. That is a latency amplifier, not a
deadlock: `edits_settled` waits only on document-sync *notifications*, which
bypass the admission limit by construction, so the pool always drains. And if
it were ever starved, all three probes fail together — which is exactly the
`SERVER WEDGED` branch that still latches. So the terminal verdict keeps its
meaning and loses only its false positives.

## Gate results

| gate | result |
|---|---|
| `cargo fmt --all --check` (`rust/`, `runtime/rust/`) | clean |
| `cargo clippy -p tcl-lsp-server --all-targets` | clean (one `too_many_lines` I introduced in `reload_spec_packs` was fixed by moving the invalidation inside the existing `if changed` block) |
| `cargo check --workspace` | clean |
| `cargo test -p tcl-lsp-server` (lib) | 471 passed |
| `cargo test -p tcl-lsp-server` (e2e) | 1504 passed, 1 failed under concurrent load — `semantic_tokens_reference_client::large_file_range_semantic_tokens_converges_via_refresh`, which passes in isolation and touches nothing this branch changes (semantic tokens do not go through `DiagInputs`). Re-run on a quiet box pending. |
| ext-host single-root | 920 passing / 0 failing / 1 pending, ×4 |
| ext-host multi-root | 14 passing ×1 |
| `prettier --check` / `eslint` on touched TS | clean |

## Next steps

1. Finish the fixed-arm loop; re-run the full server test suite on a quiet box
   to confirm the semantic-tokens e2e failure is load-only.
2. Adversary pass, then open the PR (`Fixes #1600`, `Fixes #1651`), delete this
   file.

## Resolved questions

* *"Is the `#1600` comment's marker-versus-publish reading right?"* — No. The
  server emits the marker after `cache_and_deliver` over `stdio_pump`'s single
  FIFO, so the publish precedes it on the wire; the set the test reads really
  did contain O1xx. The 23 other `waitForDeepDiagnostics` call sites are
  therefore not suspect on those grounds.

## Out of scope, worth filing separately

Under load the `SpecTcl pack torture` suite hits a real document-pipeline
stall: `a hover on an undriven document` timing out at **57803ms** with
`PROBE: could not confirm starvation`, and — later in the same run — the
heartbeat reporting `getEffectiveConfig` unresponsive after 5000ms at
`loadFactor 1`. Evidence in `scratchpad/baseline-full.log`. No open issue
covers it (searched). Not #1600 (that is the *verdict*, now correct) and not
#1651.
