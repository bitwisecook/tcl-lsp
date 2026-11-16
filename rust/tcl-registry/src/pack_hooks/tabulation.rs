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

//! Answers tabulated ahead of time for one published plan.

use super::{HookAnswer, HookCall, HookSlot};
use std::sync::atomic::{AtomicBool, Ordering};

/// An exact publication's table ownership, independent of source hashes.
/// Cloning retains that publication; a later publication cannot reuse it.
/// This is memo ownership only and grants no execution or source authority.
#[derive(Clone)]
pub struct TabulationPlan(std::sync::Arc<()>);

struct Tabulations {
    plan: Option<TabulationPlan>,
    active: bool,
    answers: Option<rustc_hash::FxHashMap<HookSlot, std::sync::Arc<[HookAnswer]>>>,
    quarantined: std::collections::BTreeSet<HookSlot>,
}

static TABULATED: std::sync::RwLock<Tabulations> = std::sync::RwLock::new(Tabulations {
    plan: None,
    active: false,
    answers: None,
    quarantined: std::collections::BTreeSet::new(),
});

static ANY_TABULATED: AtomicBool = AtomicBool::new(false);

/// Retire the previous publication's tables and issue the new plan's owner.
/// A delayed host can still finish computing, but cannot publish old answers.
#[must_use]
pub fn begin_tabulation_plan() -> TabulationPlan {
    let plan = TabulationPlan(std::sync::Arc::new(()));
    let mut tables = TABULATED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    tables.plan = Some(plan.clone());
    tables.active = false;
    tables.answers = None;
    tables.quarantined.clear();
    ANY_TABULATED.store(false, Ordering::Release);
    plan
}

/// Expose staged tables only after their plan and evaluator epoch commit.
#[must_use]
pub fn activate_tabulation_plan(plan: &TabulationPlan) -> bool {
    let mut tables = TABULATED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !tables
        .plan
        .as_ref()
        .is_some_and(|current| std::sync::Arc::ptr_eq(&current.0, &plan.0))
    {
        return false;
    }
    tables.active = true;
    true
}

/// Record count-only answers only while their exact published plan is current.
/// Returns false when another plan retired this host's table ownership.
#[must_use]
pub fn tabulate(plan: &TabulationPlan, slot: HookSlot, answers: Vec<HookAnswer>) -> bool {
    let mut tables = TABULATED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !tables
        .plan
        .as_ref()
        .is_some_and(|current| std::sync::Arc::ptr_eq(&current.0, &plan.0))
        || tables.quarantined.contains(&slot)
    {
        return false;
    }
    tables
        .answers
        .get_or_insert_with(rustc_hash::FxHashMap::default)
        .insert(slot, std::sync::Arc::from(answers));
    ANY_TABULATED.store(true, Ordering::Release);
    true
}

/// Retire unavailable slots only for the host's own current publication.
/// Late probes on that same owner cannot restore a quarantined table.
#[must_use]
pub fn quarantine_tabulations(plan: &TabulationPlan, slots: &[HookSlot]) -> bool {
    let mut tables = TABULATED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !tables
        .plan
        .as_ref()
        .is_some_and(|current| std::sync::Arc::ptr_eq(&current.0, &plan.0))
    {
        return false;
    }
    tables.quarantined.extend(slots.iter().copied());
    if let Some(answers) = &mut tables.answers {
        for slot in slots {
            answers.remove(slot);
        }
    }
    true
}

pub(super) fn tabulated(slot: HookSlot, call: &HookCall<'_>) -> Option<HookAnswer> {
    if !ANY_TABULATED.load(Ordering::Acquire) || call.option.is_some() || call.constraints.is_some()
    {
        return None;
    }
    let tables = TABULATED
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !tables.active {
        return None;
    }
    tables
        .answers
        .as_ref()?
        .get(&slot)?
        .get(call.words.len())
        .cloned()
}

pub(super) struct TableObservation(Option<TabulationPlan>);

impl TableObservation {
    pub(super) fn capture() -> Option<Self> {
        let tables = TABULATED
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if tables.plan.is_some() && !tables.active {
            return None;
        }
        Some(Self(tables.plan.clone()))
    }

    pub(super) fn is_current(&self) -> bool {
        let tables = TABULATED
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match (&self.0, &tables.plan) {
            (None, None) => true,
            (Some(observed), Some(current)) => {
                tables.active && std::sync::Arc::ptr_eq(&observed.0, &current.0)
            }
            _ => false,
        }
    }
}
