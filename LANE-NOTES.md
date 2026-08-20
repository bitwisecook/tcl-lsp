# Lane F6 — #1600 / #1651 working notes

Scratch file for continuity across container rollbacks. **Delete before the PR
is opened**; its content becomes the PR body.

## Status

| item | state |
|---|---|
| #1651 root cause | found, fixed, unit-test pinned, mutation-verified |
| #1600 root cause | found, fixed, unit-test pinned |
| full ext-host suite under xvfb | **not yet run** — the remaining gate |
| pre-push gates (fmt/clippy/check/e2e) | not yet run |

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

## Next steps

1. `make test-ext` under xvfb, looped, on the fixed tree (and confirm the
   baseline failure on stock `origin/rust` first if a spare build slot exists).
2. Pre-push gates: `cargo fmt --all --check`, clippy on `tcl-lsp-server`
   `--all-targets`, `cargo check --workspace`, `lsp-e2e`, full ext-host suite.
3. Adversary pass, then open the PR (`Fixes #1600`, `Fixes #1651`), delete this
   file.

## Open questions

* Whether the `#1600` comment's "marker and publish are different channels"
  reading has any residual truth for *other* `waitForDeepDiagnostics` +
  `getDiagnostics` pairs. Evidence so far says no (server-side ordering is
  publish-then-marker over one FIFO), but the suite run will tell.
