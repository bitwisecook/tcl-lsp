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

//! Installing, replacing and clearing this thread's hook host, and the evaluator generation that names it.

use super::dispatch::clear_cache;
use super::{
    FROM_PLAN, GENERATION, HOST, HookSlot, PRIVATE_EVALUATORS_DENIED, PackHookHost, SPENT,
};
use crate::value_transfer::EvaluatorGeneration;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};

/// `true` while any thread has a host installed — the one check an
/// unaccelerated process pays.
pub(super) static ANY_HOST: AtomicBool = AtomicBool::new(false);

/// Builds and installs this thread's host on demand, registered by the layer
/// that owns the hook host (`tcl-spectcl`).
///
/// The registry cannot build a host — that needs the loader and a VM, both of
/// which sit above it — so the capability is registered downwards once and
/// called from [`dispatch`].
pub(super) static INSTALLER: OnceLock<fn()> = OnceLock::new();

/// Deny external pack evaluators on an owned private worker before preparation.
/// A present receiver or nonempty evaluator generation refuses; nothing is
/// cleared or replaced to manufacture absence. This restriction never restores
/// before thread teardown and grants no Native operation or source authority.
#[must_use]
pub fn deny_external_evaluators_on_private_thread() -> bool {
    if !private_thread_has_no_evaluator() {
        return false;
    }
    PRIVATE_EVALUATORS_DENIED
        .try_with(|denied| denied.set(true))
        .is_ok()
}

/// Observe the actual denied/absent service without invoking its installer.
#[must_use]
pub fn private_thread_evaluators_are_closed() -> bool {
    PRIVATE_EVALUATORS_DENIED
        .try_with(Cell::get)
        .unwrap_or(false)
        && private_thread_has_no_evaluator()
}

fn private_thread_has_no_evaluator() -> bool {
    HOST.try_with(|slot| slot.try_borrow().is_ok_and(|host| host.is_none()))
        .unwrap_or(false)
        && GENERATION
            .try_with(Cell::get)
            .is_ok_and(|generation| generation == EvaluatorGeneration::NO_HOST)
}

pub(super) fn external_evaluators_blocked() -> bool {
    PRIVATE_EVALUATORS_DENIED
        .try_with(Cell::get)
        .unwrap_or(true)
}

/// Register the thread-host installer. Idempotent; the first registration wins.
///
/// # Why this exists
///
/// A host is per **thread** (it owns VMs, which are not `Send`), so every
/// thread that might dispatch a hook has to build one. Relying on each worker
/// closure to do that by hand made hook availability depend on which Tokio
/// worker happened to run the task: the analysis and optimisation closures
/// installed a host, but the semantic-token and minify ones did not, so the
/// same document could resolve a pack's commands or not depending on
/// scheduling. Installing lazily at the one point that actually needs a host —
/// the dispatch — makes that impossible to get wrong by omission.
pub fn set_installer(installer: fn()) {
    if external_evaluators_blocked() {
        return;
    }
    let _ = INSTALLER.set(installer);
}

/// What the installer would do on this thread if it ran now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingHost {
    /// The thread's host is current: nothing to do.
    Current,
    /// The published plan is empty: the installer would clear the host.
    Cleared,
    /// The installer would build a host for published plan `0`.
    Plan(u64),
}

static PENDING: OnceLock<fn() -> PendingHost> = OnceLock::new();

/// Register how to ask what the installer would do, without doing it.
/// Idempotent; the first registration wins.
///
/// [`evaluator_generation`] names the evaluators an analysis starting now
/// runs with. Building a host to learn that costs a VM bootstrap per pack per
/// thread, paid by every analysis whether or not it ever reaches a hook; the
/// generation a host built from a plan would carry is already known.
pub fn set_pending_host(pending: fn() -> PendingHost) {
    if external_evaluators_blocked() {
        return;
    }
    let _ = PENDING.set(pending);
}

/// The next generation no state has had. Process-wide, so a generation
/// minted on one thread is never minted again on another.
static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);

/// A generation no other state shares.
fn fresh_generation() -> EvaluatorGeneration {
    // Zero is the host-absent state; a wrapped counter skips it.
    EvaluatorGeneration(NEXT_GENERATION.fetch_add(1, Ordering::Relaxed).max(1))
}

/// The generation of a host built from published plan `plan`: one per plan,
/// shared by every thread whose host was built from it.
fn plan_generation(plan: u64) -> EvaluatorGeneration {
    static PLANS: LazyLock<Mutex<HashMap<u64, EvaluatorGeneration>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut plans = match PLANS.lock() {
        Ok(plans) => plans,
        Err(poisoned) => poisoned.into_inner(),
    };
    *plans.entry(plan).or_insert_with(fresh_generation)
}

/// Install `host` as this thread's pack-hook host, replacing any previous one.
///
/// Every pack-declared hook on this thread abstains until this is called, so
/// a worker thread that never installs a host behaves exactly like a build
/// with no packs loaded. The host gets a generation of its own; a host built
/// from a published plan is installed with [`install_plan_host`] instead, so
/// every worker serving that plan shares its memoised answers.
pub fn install_host(host: Rc<dyn PackHookHost>) {
    if external_evaluators_blocked() {
        return;
    }
    install(host, fresh_generation(), false);
}

/// Install `host`, built from the published plan `plan`, as this thread's
/// host: every thread that installs a host for one plan is at one
/// generation.
pub fn install_plan_host(host: Rc<dyn PackHookHost>, plan: u64) {
    if external_evaluators_blocked() {
        return;
    }
    install(host, plan_generation(plan), true);
}

pub(super) fn install(
    host: Rc<dyn PackHookHost>,
    generation: EvaluatorGeneration,
    from_plan: bool,
) {
    if external_evaluators_blocked() {
        return;
    }
    ANY_HOST.store(true, Ordering::Relaxed);
    clear_cache();
    HOST.with(|slot| {
        slot.borrow_mut().replace(host);
    });
    GENERATION.with(|current| current.set(generation));
    FROM_PLAN.with(|current| current.set(from_plan));
}

/// Remove this thread's host; every pack hook abstains again, and the
/// thread is at [`EvaluatorGeneration::NO_HOST`].
pub fn clear_host() {
    clear_cache();
    HOST.with(|slot| {
        slot.borrow_mut().take();
    });
    GENERATION.with(|current| current.set(EvaluatorGeneration::NO_HOST));
    FROM_PLAN.with(|current| current.set(false));
}

/// Record that the host's engine dispatched `commands` answering the call
/// in flight: a declared implementation's evaluation charges them to its
/// budget one-to-one (`docs/design/compiler/value-evaluation.md` § *Units
/// and charges*).
pub fn record_commands_spent(commands: u64) {
    SPENT.with(|spent| spent.set(spent.get().saturating_add(commands)));
}

/// The commands recorded since the last take, which starts the count
/// again.
#[must_use]
pub fn take_commands_spent() -> u64 {
    SPENT.with(|spent| spent.replace(0))
}

/// Record that this thread's host quarantined a hook or poisoned a pack:
/// the cached answers go, the thread takes a generation no other state
/// has, since what its host can still run is its own, and the process's
/// [`evaluator_epoch`] moves.
pub fn note_quarantine() {
    clear_cache();
    GENERATION.with(|current| current.set(fresh_generation()));
    advance_evaluator_epoch();
}

/// The process's evaluator epoch: it moves whenever the evaluators some
/// thread serves change in a way a memo shared between threads must see —
/// a hook plan published ([`advance_evaluator_epoch`], which the plan's
/// owner calls), or a hook quarantined ([`note_quarantine`]).
static EVALUATOR_EPOCH: AtomicU64 = AtomicU64::new(0);

/// The process's evaluator epoch. A thread's own
/// [`evaluator_generation`] keys what it computes; this is the one number a
/// memo shared by every thread — the language server's query database —
/// takes as an input, so a plan reload or a quarantine anywhere re-keys it.
#[must_use]
pub fn evaluator_epoch() -> u64 {
    EVALUATOR_EPOCH.load(Ordering::Relaxed)
}

/// Move the process's [`evaluator_epoch`]: the evaluators some thread
/// serves have changed.
pub fn advance_evaluator_epoch() {
    EVALUATOR_EPOCH.fetch_add(1, Ordering::Relaxed);
}

/// This thread's evaluator generation, building its host first as
/// [`dispatch`] would, so the generation names the evaluators an analysis
/// that starts now runs with.
#[must_use]
pub fn evaluator_generation() -> EvaluatorGeneration {
    if ANY_HOST.load(Ordering::Relaxed) || INSTALLER.get().is_some() {
        let direct = HOST.with(|slot| slot.borrow().is_some()) && !FROM_PLAN.with(Cell::get);
        if !direct && !external_evaluators_blocked() {
            match PENDING.get().map(|pending| pending()) {
                Some(PendingHost::Current) => {}
                Some(PendingHost::Cleared) => return EvaluatorGeneration::NO_HOST,
                Some(PendingHost::Plan(plan)) => return plan_generation(plan),
                None => {
                    ensure_host();
                }
            }
        }
    }
    GENERATION.with(Cell::get)
}

/// This thread's host, first brought up to the published plan by the
/// registered installer.
///
/// Unless the thread's host was installed directly ([`install_host`]), the
/// installer runs before the host is read, every time: it builds a host for
/// a thread that has none — abstaining there would answer "no packs" on a
/// thread that simply had not been initialised — and rebuilds one whose
/// plan was superseded. A thread that kept whatever plan host it last had
/// would serve the old plan: a pool thread last used under plan N, reached
/// by a query no worker closure re-synced, would compute a lattice through
/// plan N's host and memoise it under plan N+1's pack key and evaluator
/// epoch. When nothing moved the installer returns after one atomic load
/// and one thread-local read.
pub(super) fn ensure_host() -> Option<Rc<dyn PackHookHost>> {
    ensure_host_with_installer(|| INSTALLER.get().copied())
}

pub(super) fn ensure_host_with_installer(
    installer: impl FnOnce() -> Option<fn()>,
) -> Option<Rc<dyn PackHookHost>> {
    if external_evaluators_blocked() {
        return None;
    }
    let direct = HOST
        .with(|slot| slot.borrow().clone())
        .filter(|_| !FROM_PLAN.with(Cell::get));
    if direct.is_some() {
        return direct;
    }
    if let Some(installer) = installer() {
        installer();
    }
    HOST.with(|slot| slot.borrow().clone())
}

/// Compare the actual current thread-local receiver without invoking the
/// global lazy installer, a callback or a Native operation. Identity alone
/// grants no closed-consumer permission and does not validate hook inputs.
#[must_use]
pub fn current_host_is(expected: &Rc<dyn PackHookHost>) -> bool {
    HOST.try_with(|slot| {
        slot.try_borrow().is_ok_and(|held| {
            held.as_ref()
                .is_some_and(|actual| Rc::ptr_eq(actual, expected))
        })
    })
    .unwrap_or(false)
}

/// Retire only this exact current receiver. A changed/borrowed TLS slot is left
/// untouched. The removed Rc is dropped after the TLS borrow ends.
#[must_use]
pub fn clear_current_host_if(expected: &Rc<dyn PackHookHost>) -> bool {
    let removed = HOST
        .try_with(|slot| {
            let mut held = slot.try_borrow_mut().ok()?;
            if !held
                .as_ref()
                .is_some_and(|actual| Rc::ptr_eq(actual, expected))
            {
                return None;
            }
            held.take()
        })
        .ok()
        .flatten();
    let Some(removed) = removed else {
        return false;
    };
    clear_cache();
    GENERATION.with(|current| current.set(EvaluatorGeneration::NO_HOST));
    FROM_PLAN.with(|current| current.set(false));
    drop(removed);
    true
}

/// Whether this thread has a host installed.
#[must_use]
pub fn has_host() -> bool {
    ANY_HOST.load(Ordering::Relaxed) && HOST.with(|slot| slot.borrow().is_some())
}

/// Whether this thread's host can run `slot`'s hook now, building the host
/// first as [`dispatch`] would. `false` — no host, or the hook quarantined
/// or its pack poisoned — is transient: it says nothing about any call's
/// inputs, so an answer that rests on it must not be kept as a verdict.
#[must_use]
pub fn slot_available(slot: HookSlot) -> bool {
    if external_evaluators_blocked() {
        return false;
    }
    if !ANY_HOST.load(Ordering::Relaxed) && INSTALLER.get().is_none() {
        return false;
    }
    ensure_host().is_some_and(|host| host.is_available(slot))
}
