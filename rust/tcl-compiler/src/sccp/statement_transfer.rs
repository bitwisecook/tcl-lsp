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

//! Lattice-only statement transfers shared by the SCCP consumers.

use super::set_value;
use crate::analyses::LatticeValue;
use crate::ssa::{Symbol, ValueKey, Version};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) enum BarrierHistory {
    /// Positioned source reads retain immutable earlier value evidence.
    Retain,
    /// A non-positioned consumer loses values other than entry seeds.
    WidenNonEntry,
}

pub(super) fn widen_barrier_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    defs: &HashMap<Symbol, Version>,
    history: BarrierHistory,
) -> bool {
    let mut changed = false;
    if matches!(history, BarrierHistory::WidenNonEntry) {
        changed |= widen_held_values(values, |(_, version)| version != 0);
    }
    // The barrier can itself define names. Its fresh keys must reach a
    // downstream phi as Overdefined even when they were absent until now.
    changed | widen_def_values(values, defs)
}

pub(super) fn widen_held_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    selected: impl Fn(ValueKey) -> bool,
) -> bool {
    let mut changed = false;
    for (&key, old) in values {
        // Overdefined is absorbing, so joining in place preserves the
        // lattice update without another lookup or key allocation.
        if *old != LatticeValue::Overdefined && selected(key) {
            *old = LatticeValue::Overdefined;
            changed = true;
        }
    }
    changed
}

fn widen_def_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    defs: &HashMap<Symbol, Version>,
) -> bool {
    let mut changed = false;
    for (&symbol, &version) in defs {
        changed |= set_value(values, (symbol, version), &LatticeValue::Overdefined);
    }
    changed
}

#[derive(Clone, Copy)]
pub(super) enum FreshClobber {
    Reached { protected_parameters: bool },
    Unreached,
}

/// Apply the fresh SSA versions an opaque call leaves. Consumers can project
/// folded tags from the retained prior value independently of this transfer.
pub(super) fn transfer_fresh_clobbers(
    values: &mut HashMap<ValueKey, LatticeValue>,
    marker: Option<&HashMap<Symbol, (Version, Version)>>,
    policy: FreshClobber,
    mut record: impl FnMut(ValueKey, Option<ValueKey>),
) -> bool {
    let mut changed = false;
    for (&symbol, &(prior, fresh)) in marker.into_iter().flatten() {
        let kept = match policy {
            FreshClobber::Reached {
                protected_parameters,
            } => {
                protected_parameters
                    && matches!(values.get(&(symbol, 0)), Some(LatticeValue::Const(_)))
            }
            FreshClobber::Unreached => true,
        }
        .then_some((symbol, prior));
        let value = kept.map_or(LatticeValue::Overdefined, |prior| held_value(values, prior));
        record((symbol, fresh), kept);
        changed |= set_value(values, (symbol, fresh), &value);
    }
    changed
}

/// The value read by a phi or preserved definition. A missing non-entry
/// definition stays Unknown and an unseeded entry is Overdefined.
pub(super) fn held_value<S: std::hash::BuildHasher>(
    values: &HashMap<ValueKey, LatticeValue, S>,
    key: ValueKey,
) -> LatticeValue {
    values.get(&key).cloned().unwrap_or(if key.1 == 0 {
        LatticeValue::Overdefined
    } else {
        LatticeValue::Unknown
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::ConstValue;

    #[test]
    fn barrier_history_keeps_positioned_reads_and_widens_unpositioned_reads() {
        let x = Symbol(0);
        let out = Symbol(1);
        let seed = LatticeValue::Const(ConstValue::Int(1));
        let prior = LatticeValue::Const(ConstValue::Int(2));
        let mut values = HashMap::from([((x, 0), seed.clone()), ((x, 1), prior.clone())]);
        let defs = HashMap::from([(out, 1)]);
        assert!(widen_barrier_values(
            &mut values,
            &defs,
            BarrierHistory::Retain
        ));
        assert_eq!(values[&(x, 1)], prior);
        assert_eq!(values[&(out, 1)], LatticeValue::Overdefined);
        assert!(widen_barrier_values(
            &mut values,
            &defs,
            BarrierHistory::WidenNonEntry
        ));
        assert_eq!(values[&(x, 0)], seed);
        assert_eq!(values[&(x, 1)], LatticeValue::Overdefined);
        assert!(!widen_barrier_values(
            &mut values,
            &defs,
            BarrierHistory::WidenNonEntry
        ));
    }

    #[test]
    fn fresh_clobbers_preserve_only_the_selected_prior_versions() {
        let parameter = Symbol(0);
        let other = Symbol(1);
        let value = LatticeValue::Const(ConstValue::Int(42));
        let marker = HashMap::from([(parameter, (1, 2)), (other, (1, 2))]);
        let initial = HashMap::from([
            ((parameter, 0), value.clone()),
            ((parameter, 1), value.clone()),
            ((other, 1), value.clone()),
        ]);
        let mut reached = initial.clone();
        let mut preserved = HashMap::new();
        transfer_fresh_clobbers(
            &mut reached,
            Some(&marker),
            FreshClobber::Reached {
                protected_parameters: true,
            },
            |key, prior| {
                preserved.insert(key, prior);
            },
        );
        assert_eq!(reached[&(parameter, 2)], value);
        assert_eq!(reached[&(other, 1)], value);
        assert_eq!(reached[&(other, 2)], LatticeValue::Overdefined);
        assert_eq!(preserved[&(parameter, 2)], Some((parameter, 1)));
        assert_eq!(preserved[&(other, 2)], None);

        let mut unreached = initial;
        transfer_fresh_clobbers(
            &mut unreached,
            Some(&marker),
            FreshClobber::Unreached,
            |_, _| {},
        );
        assert_eq!(unreached[&(parameter, 2)], value);
        assert_eq!(unreached[&(other, 2)], value);
    }
}
