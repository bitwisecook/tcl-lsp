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

//! Dispatching one hook call: a tabulated answer first, then the host, behind the shape-keyed answer cache.

use super::host_installation::{
    ANY_HOST, INSTALLER, current_host_is, ensure_host, evaluator_generation,
    external_evaluators_blocked,
};
use super::tabulation::{TableObservation, tabulated};
#[cfg(test)]
use super::tests;
use super::{CacheMode, GENERATION, HookAnswer, HookCall, HookSlot, SHAPE_CACHE, cache_mode};
use crate::invocation_words::InvocationWordKind;
use crate::value_transfer::{ContextDependency, ImplementationBudget};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tcl_dialect::TclVersion;

/// The shape key: everything a shape-cacheable hook may read, packed.
///
/// `kinds` is two bits per word, so a call with more than 64 words has no
/// shape key and is answered by the host every time (uncached, still
/// correct).
///
/// `content` is `0` for a [`CacheMode::Shape`] slot and a hash of the call's
/// literal content for a [`CacheMode::Content`] one — which is what lets a
/// hook that genuinely reads values still be answered from the cache when
/// nothing at *its* call site changed (E-R14, principle P-B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ShapeKey {
    slot: HookSlot,
    nwords: u16,
    kinds: u128,
    /// `tcl-version` as a stable discriminant — `TclVersion` is `Ord` but not
    /// `Hash`, and only its identity matters here.
    version: Option<&'static str>,
    /// The profile the call is analysed under. A hook body may not read it
    /// and stay cacheable, but a release-pinned body runs on an engine
    /// pinned to it, so one release's answer is never served under another.
    dialect: Option<&'static str>,
    in_event_body: bool,
    content: u64,
}

/// Everything a content-keyed answer rests on beyond its [`ShapeKey`]: the
/// words' values, the `constraints` family's invocation view, and the
/// `evaluate` family's declared targets, budget and dependencies. Kept with
/// the answer and compared on every hit, because the key's `content` is a
/// hash — the bucket, never the proof. The profile the target semantics
/// derive from is the key's `dialect`, compared exactly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct CallContent {
    pub(super) words: Vec<String>,
    pub(super) constraints: Option<ConstraintContent>,
    targets: Vec<usize>,
    budget: ImplementationBudget,
    depends: Vec<ContextDependency>,
}

/// An owned copy of a [`ConstraintCallCtx`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConstraintContent {
    pub(super) options: Vec<(&'static str, Option<String>)>,
    pub(super) positionals: Vec<Option<String>>,
    pub(super) complete: bool,
}

impl CallContent {
    /// The content of `call` a `mode` slot's answer may depend on: all of
    /// it for a content-keyed slot, nothing for a shape-keyed one — whose
    /// key already holds everything its body is given.
    pub(super) fn of(call: &HookCall<'_>, mode: CacheMode) -> Self {
        if mode != CacheMode::Content {
            return Self::default();
        }
        Self {
            words: call
                .words
                .iter()
                .map(|word| word.value.to_owned())
                .collect(),
            constraints: call.constraints.map(|view| ConstraintContent {
                options: view
                    .options
                    .iter()
                    .map(|(name, value)| (*name, value.map(str::to_owned)))
                    .collect(),
                positionals: view
                    .positionals
                    .iter()
                    .map(|word| word.map(str::to_owned))
                    .collect(),
                complete: view.complete,
            }),
            targets: call.targets.to_vec(),
            budget: call.budget,
            depends: call.depends.to_vec(),
        }
    }

    /// Whether `call` has exactly this content, compared without copying.
    pub(super) fn matches(&self, call: &HookCall<'_>, mode: CacheMode) -> bool {
        if mode != CacheMode::Content {
            return true;
        }
        let constraints_match = match (&self.constraints, call.constraints) {
            (None, None) => true,
            (Some(kept), Some(view)) => {
                kept.complete == view.complete
                    && kept.options.len() == view.options.len()
                    && kept.options.iter().zip(view.options).all(
                        |((name, value), (other, other_value))| {
                            name == other && value.as_deref() == *other_value
                        },
                    )
                    && kept.positionals.len() == view.positionals.len()
                    && kept
                        .positionals
                        .iter()
                        .zip(view.positionals)
                        .all(|(kept, word)| kept.as_deref() == *word)
            }
            _ => false,
        };
        constraints_match
            && self.words.len() == call.words.len()
            && self
                .words
                .iter()
                .zip(call.words)
                .all(|(kept, word)| kept == word.value)
            && self.targets == call.targets
            && self.budget == call.budget
            && self.depends == call.depends
    }
}

/// One cached answer, with the content it was computed for.
pub(super) struct CacheEntry {
    pub(super) content: CallContent,
    pub(super) answer: HookAnswer,
}

#[derive(Default)]
pub(super) struct ShapeCache {
    pub(super) entries: HashMap<ShapeKey, CacheEntry>,
    pub(super) hits: u64,
    pub(super) misses: u64,
}

/// Hit / miss / entry counts for this thread's shape cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    /// Invocations answered from the cache.
    pub hits: u64,
    /// Invocations that reached the host.
    pub misses: u64,
    /// Distinct shapes resident.
    pub entries: usize,
}

/// This thread's shape-cache counters.
#[must_use]
pub fn cache_stats() -> CacheStats {
    SHAPE_CACHE.with(|cache| {
        let cache = cache.borrow();
        CacheStats {
            hits: cache.hits,
            misses: cache.misses,
            entries: cache.entries.len(),
        }
    })
}

/// Drop every cached answer and zero the counters. Called on host install /
/// removal, and by the host when a hook is quarantined.
pub fn clear_cache() {
    SHAPE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.entries.clear();
        cache.hits = 0;
        cache.misses = 0;
    });
}

/// A stable hash of everything a [`CacheMode::Content`] slot may read beyond
/// the shape: the words' literal values and the `constraints` family's
/// structured invocation view. The bucket a content-keyed answer is found
/// in; [`CallContent::matches`] is what proves the hit.
fn content_hash(call: &HookCall<'_>) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    #[cfg(test)]
    if let Some(forced) = tests::FORCED_CONTENT_HASH.with(Cell::get) {
        return forced;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for word in call.words {
        word.value.hash(&mut hasher);
    }
    if let Some(constraints) = call.constraints {
        for (name, value) in constraints.options {
            name.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        constraints.positionals.hash(&mut hasher);
        constraints.complete.hash(&mut hasher);
    }
    hasher.finish()
}

pub(super) fn shape_key(slot: HookSlot, call: &HookCall<'_>, mode: CacheMode) -> Option<ShapeKey> {
    if call.words.len() > 64 {
        return None;
    }
    let mut kinds: u128 = 0;
    for (position, word) in call.words.iter().enumerate() {
        let bits: u128 = match word.kind {
            // The legacy two-bit cache vocabulary has no native byte value.
            // Decline caching instead of colliding with a logical literal.
            InvocationWordKind::KnownBytes => return None,
            InvocationWordKind::Literal => 0,
            InvocationWordKind::Dynamic => 1,
            InvocationWordKind::Expanded => 2,
            InvocationWordKind::Opaque => 3,
        };
        kinds |= bits << (position * 2);
    }
    Some(ShapeKey {
        slot,
        nwords: u16::try_from(call.words.len()).ok()?,
        kinds,
        version: call.version.map(TclVersion::version_string),
        dialect: call.dialect,
        in_event_body: call.in_event_body,
        content: match mode {
            CacheMode::Content => content_hash(call),
            CacheMode::None | CacheMode::Shape => 0,
        },
    })
}

/// Resident-entry ceiling for the shape cache.
///
/// A content-keyed entry is per *distinct call content*, so a very large
/// workspace could otherwise grow the table without bound. At the ceiling the
/// table is dropped wholesale rather than evicted one by one: the cache is a
/// pure memo, so losing it costs a re-run and never an answer.
const MAX_CACHE_ENTRIES: usize = 8192;

/// Invoke a pack hook after synchronizing its actual published plan receiver.
/// A tabulated answer needs no receiver. The current receiver's cache is used
/// only when declared inputs permit it; unavailable hosts retain the family's
/// silence.
#[must_use]
pub fn dispatch(slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
    if external_evaluators_blocked() {
        return HookAnswer::Abstain;
    }
    let Some(publication) = TableObservation::capture() else {
        return HookAnswer::Abstain;
    };
    if let Some(answer) = tabulated(slot, call) {
        return if publication.is_current() {
            answer
        } else {
            HookAnswer::Abstain
        };
    }
    n17_live_trace!("dispatch-entry slot={slot:?}");
    if !ANY_HOST.load(Ordering::Relaxed) && INSTALLER.get().is_none() {
        n17_live_trace!("decline=dispatch-no-host slot={slot:?}");
        return HookAnswer::Abstain;
    }
    // Synchronize the actual installed receiver/plan before deriving cache
    // mode or admitting a prior answer. Installation clears stale TLS entries.
    let Some(host) = ensure_host() else {
        n17_live_trace!("decline=dispatch-no-receiver slot={slot:?}");
        return HookAnswer::Abstain;
    };
    // An owning receiver admits or refuses each dispatch itself, so it is
    // shown the ones it will refuse too.
    if host.requires_uncached_dispatch() {
        let answer = host.invoke(slot, call);
        return if publication.is_current() {
            answer
        } else {
            HookAnswer::Abstain
        };
    }
    if !host.is_available(slot) {
        return HookAnswer::Abstain;
    }
    let generation = GENERATION.with(Cell::get);
    let mode = cache_mode(slot);
    let key = (mode != CacheMode::None)
        .then(|| shape_key(slot, call, mode))
        .flatten();
    if let Some(key) = key
        && let Some(hit) = SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let hit = cache
                .entries
                .get(&key)
                .filter(|entry| entry.content.matches(call, mode))
                .map(|entry| entry.answer.clone());
            if hit.is_some() {
                cache.hits += 1;
            }
            hit
        })
    {
        n17_live_trace!("dispatch-cache-hit slot={slot:?} answer={hit:?}");
        return if publication.is_current() && host.is_available(slot) {
            hit
        } else {
            HookAnswer::Abstain
        };
    }
    n17_live_trace!("dispatch-host-call slot={slot:?}");
    let answer = host.invoke(slot, call);
    n17_live_trace!("dispatch-host-return slot={slot:?} answer={answer:?}");
    // A callback may replace this receiver. A concurrent plan publication
    // is visible through the pending generation without installing a host.
    if !publication.is_current() || !host.is_available(slot) {
        return HookAnswer::Abstain;
    }
    if !current_host_is(&host) || evaluator_generation() != generation {
        return answer;
    }
    if let Some(key) = key {
        SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.misses += 1;
            if cache.entries.len() >= MAX_CACHE_ENTRIES {
                cache.entries.clear();
            }
            // A colliding bucket holds the latest content's answer.
            cache.entries.insert(
                key,
                CacheEntry {
                    content: CallContent::of(call, mode),
                    answer: answer.clone(),
                },
            );
        });
    }
    answer
}
