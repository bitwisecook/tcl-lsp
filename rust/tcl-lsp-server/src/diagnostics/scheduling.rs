// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The debounced per-document worker and the scheduler that feeds it.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::Uri;

use crate::Backend;
use crate::documents::{DocumentPathAdmission, DocumentSourceResult};
use crate::protocol::bounded_client_request;
use crate::tracked_locks::TrackedMutex;
use crate::workspace::{EvidenceHandles, refresh_cross_file_evidence};

use super::admission::{Admitted, AnalysisAdmission};
use super::run::run_diagnostics_core;
use super::{DiagInputs, DiagSlot};

/// Debounce interval before a scheduled diagnostics run starts work.  A burst
/// of keystrokes within this window collapses to a single analysis (the
/// generation check supersedes earlier scheduled runs).  Small enough that
/// diagnostics still feel immediate after typing stops.
pub(in super::super) const DIAGNOSTICS_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(50);

/// How many times [`Backend::schedule_diagnostics_impl`] may re-read
/// `diag_inputs` when the configuration moves while it is being read.
///
/// Bounded so the loop can never spin: each attempt costs only the dozen mutex
/// acquisitions `diag_inputs` makes, so exhausting this needs several config
/// applies inside a fraction of a millisecond — which the reload's own
/// coalescing already rules out. Exhausting it is safe rather than merely
/// unlikely: the resolve is discarded, the slot keeps its older stamp, and the
/// next schedule therefore re-resolves.
const DIAG_INPUTS_RESOLVE_ATTEMPTS: u32 = 4;

/// Run the analyser + diagnostic lifts for one document and publish the result,
/// off the LSP event loop.
///
/// The base analysis runs through the cancellable salsa
/// `file_analysis_incremental` query: the per-item walk is memoised and a
/// concurrent edit's `set_text` cancels an in-flight read at a per-item query
/// boundary, so the diagnostics path needs no uncancellable direct-`analyse`
/// detour to keep write contention from stalling it.
/// Deliver a freshly-computed diagnostic set to the client.
///
/// The pull cache is always updated by the caller *before* this runs, so a
/// `textDocument/diagnostic` request reflects the latest set either way. How
/// the client is *notified* then depends on what it supports:
///
/// - **Pull-capable client** (`client_supports_pull`): don't push. Ask the
///   client to re-pull via `workspace/diagnostic/refresh`; it then issues a
///   `textDocument/diagnostic` and reads the cache we just primed. Pushing
///   *and* pulling the same set makes such clients show every diagnostic
///   twice; a refresh also covers cross-file (`crossFileResolution`)
///   updates the client would otherwise not know to re-pull.
/// - **Push-only client**: publish as before, the only channel it has.
///
/// This await runs only in [`DiagnosticPublisher`]'s single persistent
/// consumer. The transport's backpressure is retained, but it can park only
/// diagnostics delivery — never a global document/index guard or the request
/// admission path. Do not detach an independent send per result: the mailbox's
/// single-consumer order and latest-wins coalescing are the delivery invariant.
pub(in super::super) async fn deliver_diagnostics(
    client: &Client,
    uri: &Uri,
    diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    version: Option<i32>,
    client_supports_pull: bool,
) {
    if client_supports_pull {
        // Best-effort: a client that advertised pull support is expected to
        // honour the refresh, but a transport error here must not abort the
        // worker (the primed cache still serves the next manual pull).
        let _ = bounded_client_request(client.workspace_diagnostic_refresh()).await;
    } else {
        client
            .publish_diagnostics(uri.clone(), diags, version)
            .await;
    }
}

/// Release the worker's claim on `uri`'s slot.
///
/// A slot whose `latest_inputs` were cleared belongs to a document that is no
/// longer open ([`Backend::evict_diag_slot`] clears them on close), so the slot
/// itself is dropped; an open document keeps its slot, with `running` cleared so
/// the next edit starts a fresh worker.
///
/// Dropping the slot is what bounds closed-file state: keeping one for every
/// URI ever scheduled would retain a [`DiagInputs`] — per-URI clones of the
/// disabled / extra-command / severity-override / entry-point sets — for the
/// process's life, so browsing a workspace would grow `diag_slots` without
/// limit.  Retaining it buys nothing: `did_open` re-resolves the inputs
/// unconditionally (`reschedule_diagnostics`), so a reopened document never
/// reads them.
pub(in super::super) fn retire_slot(slots: &mut HashMap<Uri, DiagSlot>, uri: &Uri) {
    let Some(slot) = slots.get_mut(uri) else {
        return;
    };
    if slot.latest_inputs.is_none() {
        slots.remove(uri);
    } else {
        slot.running = false;
    }
}

/// Wait until `uri`'s scheduled diagnostics generation has stayed unchanged
/// for one full debounce window.  Returns that stable generation, or `None`
/// when the slot disappeared or no longer has work.
///
/// When no work remains, this retires the worker's slot while still holding
/// the slot-map lock.  That makes observing a clean slot and releasing its
/// worker claim one transition: an edit that follows either sees a removed /
/// non-running slot and starts a new worker, or is folded into the still-live
/// worker before it makes that decision.
pub(in super::super) async fn stable_diagnostics_generation(
    slots: &Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    uri: &Uri,
) -> Option<u64> {
    loop {
        let generation = {
            let mut guard = slots.lock().await;
            let (dirty, generation) = guard.get(uri).map(|slot| (slot.dirty, slot.generation))?;
            if !dirty {
                retire_slot(&mut guard, uri);
                return None;
            }
            generation
        };
        crate::rt::sleep(DIAGNOSTICS_DEBOUNCE).await;
        let mut guard = slots.lock().await;
        let (dirty, current_generation) =
            guard.get(uri).map(|slot| (slot.dirty, slot.generation))?;
        if !dirty {
            retire_slot(&mut guard, uri);
            return None;
        }
        if current_generation == generation {
            return Some(generation);
        }
    }
}

fn spawn_diagnostics_worker(slots: Arc<Mutex<HashMap<Uri, DiagSlot>>>, uri: Uri) {
    crate::rt::spawn(async move {
        let Some(admission) = slots.lock().await.get(&uri).map(|slot| {
            slot.admission
                .clone()
                .unwrap_or_else(AnalysisAdmission::shared)
        }) else {
            return;
        };
        // Held from admission until this document's run (and its evidence
        // refresh) is done, including across a re-debounce, so a document
        // still being edited keeps its turn instead of rejoining the queue.
        let mut admitted: Option<Admitted> = None;
        loop {
            // A true trailing-edge debounce. Starting analysis every fixed
            // interval *during* a long typing burst makes each salsa write
            // cancel fresh reads; a reader waiting on a shared query can then
            // hold `set_text` for seconds and pin the global edit-order barrier.
            let Some(generation) = stable_diagnostics_generation(&slots, &uri).await else {
                return;
            };
            if admitted.is_none() {
                // Waiting here costs no thread. The inputs are taken after
                // admission, so a document that waited analyses its latest
                // state, never the one it had when it joined the queue.
                admitted = Some(admission.admit().await);
            }
            let inputs = {
                let mut guard = slots.lock().await;
                let Some(slot) = guard.get_mut(&uri) else {
                    return;
                };
                if slot.generation != generation {
                    continue;
                } else if slot.dirty {
                    slot.dirty = false;
                    slot.latest_inputs.clone()
                } else {
                    retire_slot(&mut guard, &uri);
                    return;
                }
            };
            // `latest_inputs` is set alongside `dirty`, so this is `Some` unless
            // `Backend::evict_diag_slot` cleared it while we were draining;
            // either way there is nothing to analyse, so retire.
            let Some(mut inputs) = inputs else {
                retire_slot(&mut *slots.lock().await, &uri);
                return;
            };
            // A document that has never had cross-file evidence set gets it
            // *before* its first analysis, so its first published diagnostics
            // are already correct — publishing an "always true" fold and
            // retracting it a moment later is precisely the flicker this
            // avoids. Subsequent passes skip this and refresh after
            // publishing instead, keeping the project-wide scan off the
            // critical path for every later edit (and out from in front of the
            // semantic-token enrichment tier on a large document).
            // Handles for the post-publish evidence refresh below
            // (`run_diagnostics_core` consumes `inputs`).
            let evidence_handles = EvidenceHandles::from_inputs(&inputs);
            // Capture + analyse the document's current state.  If it is gone
            // (closed) there is nothing to publish — retire, unless a `did_open`
            // re-armed the slot in the meantime, in which case keep draining
            // rather than orphaning its dirty bit with no worker behind it.
            let Some(job) = inputs.capture_job(&uri).await else {
                let mut guard = slots.lock().await;
                match guard.get(&uri) {
                    Some(slot) if slot.dirty => continue,
                    Some(_) => retire_slot(&mut guard, &uri),
                    None => {}
                }
                return;
            };
            if inputs.registry_at_drain
                && let Some(registry) = job
                    .diagnostic_pass
                    .as_ref()
                    .and_then(|pass| pass.checked_registry())
            {
                inputs.registry = registry;
            }
            let settled = run_diagnostics_core(inputs, &uri, job).await;
            if !settled {
                // Cancelled mid-flight — re-mark dirty so we retry the latest
                // state next loop. There was no successful publish to enrich,
                // and the newer edit has already invalidated the project view,
                // so do not launch the whole-project evidence/factory refresh
                // for this stale run. Under a typing storm those doomed
                // refreshes otherwise compete with every queued `didChange`
                // for the salsa database and can pin the edit-order barrier.
                if let Some(slot) = slots.lock().await.get_mut(&uri) {
                    slot.mark_dirty();
                }
                admitted = None;
                continue;
            }
            // `run_diagnostics_core` can also settle by declining to publish a
            // revision that a newer edit already superseded. In that case the
            // slot is dirty again and the only useful next step is to debounce
            // and analyse its latest state. Refreshing project-wide evidence
            // here would derive it from an obsolete run while contending with
            // the queued document writes that made the run obsolete.
            if slots.lock().await.get(&uri).is_some_and(|slot| slot.dirty) {
                admitted = None;
                continue;
            }
            // Refresh the project's cross-file call-site evidence *after*
            // publishing. Off the edit handler, as it must be — but also
            // behind this document's own result rather than in front of it:
            // the cold path lowers every project file, and on a large document
            // that would delay both the diagnostics and the semantic-token
            // enrichment tier gated behind them.
            //
            // The cost of publishing first is that a file's *first* run uses
            // whatever evidence it already had. When the refresh then changes
            // this file's own evidence we re-mark it dirty, so the loop
            // immediately re-analyses under the corrected view — the same
            // publish-then-enrich shape the semantic-token tiers use. Salsa
            // memoisation makes the second pass cheap apart from what the
            // evidence genuinely invalidated.
            refresh_cross_file_evidence(&evidence_handles, &slots, &uri).await;
            admitted = None;
        }
    });
}

/// Mark each peer dirty and start its worker if one is not already draining
/// it. See [`spawn_diagnostics_worker`] for why this exists.
pub(in super::super) async fn reschedule_peers(
    slots: &Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    self_uri: &Uri,
    peers: Vec<Uri>,
) {
    let mut to_spawn = Vec::new();
    {
        let mut guard = slots.lock().await;
        for peer in peers {
            if &peer == self_uri {
                continue;
            }
            let Some(slot) = guard.get_mut(&peer) else {
                continue;
            };
            if slot.latest_inputs.is_none() {
                continue;
            }
            slot.mark_dirty();
            if !slot.running {
                slot.running = true;
                to_spawn.push(peer);
            }
        }
    }
    for peer in to_spawn {
        spawn_diagnostics_worker(Arc::clone(slots), peer);
    }
}

/// Whether the cross-file refresh gave `uri` something that can actually
/// change its own result.
///
/// Two independent facts qualify, and both distinguish "the project has
/// something to say about this file" from "the project was enumerated and had
/// nothing to add" — the latter cannot change the file's analysis, so it must
/// not trigger a second, identical publish:
///
/// * its **call-site evidence** names a caller — what retracts an
///   interprocedural fold;
/// * the workspace's **class-factory index** is non-empty — a
///   `Meta create Name …` in this file is recorded or abstained on depending
///   on it, so a file that publishes before the index arrives publishes
///   without the classes it manufactures.
///
/// Lock order is `db` → `db_files`.
pub(in super::super) async fn refresh_can_change_result(
    db: &TrackedMutex<tcl_lsp_db::TclDatabase>,
    db_files: &TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>,
    uri: &Uri,
) -> bool {
    let db = db.lock().await;
    let files = db_files.lock().await;
    files.get(uri).is_some_and(|f| {
        f.external_call_sites(&*db)
            .as_ref()
            .is_some_and(|e| !e.is_empty())
            || f.workspace_class_factories(&*db).is_some()
    })
}

impl Backend {
    /// Release `uri`'s diagnostics slot when its document closes.
    ///
    /// With no worker draining it the slot is dropped outright.  With one
    /// in flight the slot must stay — it is the worker's own liveness record —
    /// so only the heavy [`DiagInputs`] are released and the dirty bit cleared;
    /// [`retire_slot`] then drops the slot as the worker retires (and a
    /// `did_open` racing in front of that re-arms it before the worker looks,
    /// so nothing is orphaned).
    pub(in super::super) async fn evict_diag_slot(
        &self,
        uri: &Uri,
        admission: &DocumentPathAdmission,
    ) -> DocumentSourceResult<()> {
        let mut slots = self.diag_slots.lock().await;
        let mut completion = self.diagnostic_publisher.completion.lock().await;
        admission.check_uri(uri)?;
        DocumentPathAdmission::with_current(&[admission], self.store.as_ref(), || {
            completion.remove(uri);
            if let Some(slot) = slots.get_mut(uri) {
                if slot.running {
                    slot.latest_inputs = None;
                    slot.dirty = false;
                } else {
                    slots.remove(uri);
                }
            }
        })?;
        Ok(())
    }

    /// Re-run diagnostics for every open document — used after a change to
    /// shared workspace-wide state (config, folder set, or the on-disk
    /// `workspace_index` / `package_resolver` domain) that did **not** originate
    /// from an open document's own edit, so push-diagnostic clients refresh
    /// results that depend on that state (cross-file `crossFileResolution`, and
    /// the always-on W120/W123 workspace refinement) instead of showing stale
    /// diagnostics until the caller is next touched. Unconditional — the W120/W123
    /// refinement runs for every document regardless of the `crossFileResolution`
    /// toggle.
    pub(in super::super) async fn reschedule_all_open_documents(&self) {
        let snapshot: Vec<(Uri, String)> = {
            let docs = self.documents.lock("reschedule_all_open_documents").await;
            docs.iter()
                .map(|(uri, doc)| (uri.clone(), doc.dialect.clone()))
                .collect()
        };
        for (uri, dialect) in snapshot {
            self.reschedule_diagnostics(uri, dialect).await;
        }
    }

    /// Schedule a debounced, detached diagnostics run for `uri`.  Returns
    /// immediately (the analyser work runs on a `tokio::spawn`ed task), so
    /// `did_open` / `did_change` never block the message loop on analysis.
    ///
    /// Coalescing per-URI worker: the edit marks the document dirty and, if no
    /// worker is already draining `uri`, starts one.  The worker debounces, then
    /// captures and analyses the document's **current** state (so a burst — even
    /// one whose `didChange` notifications are processed out of order — settles
    /// on the latest committed version), retrying if a run is cancelled
    /// mid-flight by a concurrent `set_text`.  This guarantees the final
    /// version's diagnostics are always published, even under a heavy edit burst
    /// on a loaded machine, and never runs two analyses for one document
    /// concurrently (so an edit's write is never blocked behind a stale read).
    pub(in super::super) async fn schedule_diagnostics(&self, uri: Uri, dialect: String) {
        // Edit path: reuse the slot's cached inputs *if the configuration they
        // were read from is still current* (the worker still reads the
        // document's own content at drain time regardless).  Currency is the
        // epoch check inside the impl, not an assumption — see
        // [`Self::invalidate_diag_inputs`].
        self.schedule_diagnostics_impl(uri, dialect, false).await;
    }

    /// Like [`Self::schedule_diagnostics`] but forces a re-resolve of the
    /// config-sensitive inputs, so an already-running worker drains under the
    /// *new* toggles.  Used by the config-change paths (`optimiser.enabled`,
    /// `features.diagnostics`, dialect switch) where the cached inputs are stale
    /// even though the document text has not changed.
    pub(in super::super) async fn reschedule_diagnostics(&self, uri: Uri, dialect: String) {
        self.schedule_diagnostics_impl(uri, dialect, true).await;
    }

    pub(in super::super) async fn schedule_diagnostics_impl(
        &self,
        uri: Uri,
        dialect: String,
        force_refresh: bool,
    ) {
        let profile = tcl_lsp_core::profile_for_dialect(&dialect);
        // Resolve under an epoch that did not move while we were reading it.
        //
        // `diag_inputs` is a long chain of awaits over a dozen separate config
        // mutexes, so an apply landing in the middle of it yields a snapshot
        // that is half pre-change and half post-change — and stamping that with
        // *either* epoch is a lie. Reading the epoch on both sides and keeping
        // only a snapshot that saw the same value on both is what makes the
        // stamp mean what it says.
        //
        // Discarding rather than stamping-conservatively matters because the
        // stamp is not the only consumer: the worker takes `latest_inputs` at
        // drain time and never re-checks it, so a mixed snapshot committed here
        // is a mixed snapshot published — the same window, just narrower.
        let mut resolved: Option<(DiagInputs, u64)> = None;
        for _ in 0..DIAG_INPUTS_RESOLVE_ATTEMPTS {
            let epoch = self.diag_inputs_epoch();
            // Only (re)resolve the relatively expensive `diag_inputs` when the
            // worker has none yet, a config change forces it, or the cached ones
            // were read from a configuration that has since moved
            // ([`Self::invalidate_diag_inputs`]) — an edit under an unchanged
            // configuration reuses them.  Peek that decision under the lock
            // without mutating. Re-peeked on every attempt, so a concurrent
            // scheduler that installed current inputs while we were reading
            // leaves us nothing to do.
            let need_inputs = {
                let mut slots = self.diag_slots.lock().await;
                let slot = slots.entry(uri.clone()).or_default();
                force_refresh
                    || slot
                        .latest_inputs
                        .as_ref()
                        .is_none_or(|inputs| inputs.inputs_epoch != Some(epoch))
                    || slot.inputs_epoch != epoch
            };
            if !need_inputs {
                break;
            }
            // Resolve the fresh inputs *before* marking the slot dirty. Marking
            // dirty first and storing `latest_inputs` only after the `await`
            // would let a running worker drain the dirty flag with the *stale*
            // inputs in that window, silently dropping a config change (e.g.
            // squiggles the user just disabled persisting until the next
            // keystroke).
            // Resolving first means `dirty` and `latest_inputs` are published
            // together, atomically, so the worker never observes one without the
            // other.
            let inputs = self.diag_inputs_with(&uri, profile, true).await;
            if self.diag_inputs_epoch() == epoch && inputs.inputs_epoch == Some(epoch) {
                resolved = Some((inputs, epoch));
                break;
            }
            // The configuration moved while we read it. Drop the snapshot and
            // read again under the epoch that now applies.
        }
        // Commit `latest_inputs` (if freshly resolved) and mark dirty in a
        // single locked section — no `await` between them — so a storm of rapid
        // edits still coalesces predictably.
        let start_worker = {
            let mut slots = self.diag_slots.lock().await;
            let slot = slots.entry(uri.clone()).or_default();
            // Forwards only. Two schedules for one URI can be in flight at once,
            // and nothing makes them finish in the order they started: a resolve
            // that read epoch N can land *after* one that read N+1 and has
            // already committed. Overwriting there would replace a current
            // snapshot with an older one and stamp the slot backwards, so the
            // reuse check would then call the older one current. An equal epoch
            // may commit — the two snapshots saw the same configuration.
            if let Some((inputs, epoch)) = resolved
                && epoch >= slot.inputs_epoch
            {
                slot.latest_inputs = Some(inputs);
                slot.inputs_epoch = epoch;
            }
            slot.admission = Some(Arc::clone(&self.diagnostics_admission));
            slot.mark_dirty();
            if slot.running {
                false
            } else {
                slot.running = true;
                true
            }
        };
        if !start_worker {
            return;
        }
        let slots = Arc::clone(&self.diag_slots);
        spawn_diagnostics_worker(slots, uri);
    }
}
