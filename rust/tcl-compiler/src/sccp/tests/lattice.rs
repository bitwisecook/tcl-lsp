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

//! The lattice join, `set_value`, predecessors and the block order.

use super::*;

// Join.

#[test]
fn join_unknown_absorbs() {
    let c = LatticeValue::Const(ConstValue::int(7));
    assert_eq!(join(&LatticeValue::Unknown, &c), c);
    assert_eq!(join(&c, &LatticeValue::Unknown), c);
}

#[test]
fn join_overdefined_is_absorbing() {
    let c = LatticeValue::Const(ConstValue::int(7));
    assert_eq!(
        join(&LatticeValue::Overdefined, &c),
        LatticeValue::Overdefined
    );
    assert_eq!(
        join(&c, &LatticeValue::Overdefined),
        LatticeValue::Overdefined
    );
}

#[test]
fn join_identical_const_stays_const() {
    let c = LatticeValue::Const(ConstValue::int(7));
    assert_eq!(join(&c, &c), c);
}

#[test]
fn repeated_nan_does_not_change_the_lattice() {
    let candidate = LatticeValue::Const(ConstValue::float(f64::NAN));
    let mut values = HashMap::new();
    let key = (Symbol(0), 1);

    assert_eq!(join(&candidate, &candidate), candidate);
    assert!(set_value(&mut values, key, &candidate));
    assert!(!set_value(&mut values, key, &candidate));
}

#[test]
fn joining_signed_zeros_preserves_both_members() {
    let positive = ConstValue::float(0.0);
    let negative = ConstValue::float(-0.0);
    let expected = LatticeValue::ConstSet(vec![positive.clone(), negative.clone()]);
    let positive = LatticeValue::Const(positive);
    let negative = LatticeValue::Const(negative);

    assert_eq!(join(&positive, &negative), expected);
    assert_eq!(join(&negative, &positive), expected);
}

#[test]
fn joining_integers_and_booleans_keeps_both_kinds() {
    let integer = ConstValue::int(1);
    let boolean = ConstValue::boolean(true);
    let expected = LatticeValue::ConstSet(vec![integer.clone(), boolean.clone()]);

    assert_eq!(
        join(&LatticeValue::Const(integer), &LatticeValue::Const(boolean)),
        expected,
    );
}

#[test]
fn join_distinct_consts_widens_to_set() {
    let a = LatticeValue::Const(ConstValue::int(1));
    let b = LatticeValue::Const(ConstValue::int(2));
    let merged = join(&a, &b);
    match merged {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn join_existing_set_absorbs_member() {
    let a = LatticeValue::ConstSet(vec![ConstValue::int(1), ConstValue::int(2)]);
    let b = LatticeValue::Const(ConstValue::int(1));
    // Adding a member already in the set returns `old` unchanged.
    assert_eq!(join(&a, &b), a);
}

#[test]
fn join_widens_large_sets_to_overdefined() {
    let big: Vec<ConstValue> = (0..MAX_CONSTSET_SIZE)
        .map(|i| ConstValue::int(i64::try_from(i).unwrap()))
        .collect();
    let a = LatticeValue::ConstSet(big);
    let b = LatticeValue::Const(ConstValue::int(999));
    assert_eq!(join(&a, &b), LatticeValue::Overdefined);
}

#[test]
fn set_value_tracks_change() {
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let key: ValueKey = (Symbol(0), 1);
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::int(1))
    ));
    assert!(!set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::int(1))
    ));
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::int(2))
    ));
}

// Predecessors + cfg_order.

#[test]
fn predecessors_simple_chain() {
    let mut f = Function::new("::top", "a");
    let a = f.entry;
    let b = block(&mut f, "b");
    f.blocks.get_mut(&a).unwrap().terminator = Some(goto(b));
    f.blocks.get_mut(&b).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let p = compute_predecessors(&f);
    assert!(p.get(&b).unwrap().contains(&a));
    assert!(p.get(&a).is_none_or(HashSet::is_empty));
}

#[test]
fn cfg_order_starts_at_entry() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    let join = block(&mut f, "join");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&e).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert_eq!(order[0], entry);
    // join must appear after both branches.
    let join_pos = order.iter().position(|b| *b == join).unwrap();
    let t_pos = order.iter().position(|b| *b == t).unwrap();
    let e_pos = order.iter().position(|b| *b == e).unwrap();
    assert!(join_pos > t_pos);
    assert!(join_pos > e_pos);
}

#[test]
fn cfg_order_appends_unreachable_blocks() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let dead = block(&mut f, "dead");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert!(order.contains(&entry));
    assert!(order.contains(&dead));
}
