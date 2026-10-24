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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software registration incarnation and bounded owner exhaustion.
use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_syntax::value::ValueError;
fn key() -> NativeLiteralKey {
    NativeLiteralKey {
        protocol: NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        namespace: None,
        original: b"registered".to_vec(),
    }
}
fn register(world: &Rc<RefCell<NativeLiteralWorld<Rc<Vec<u8>>>>>) -> (usize, Rc<Vec<u8>>) {
    world
        .borrow_mut()
        .register(
            key(),
            |value| Ok::<_, ValueError>(value.as_ref().clone()),
            || Ok(Rc::new(b"registered".to_vec())),
        )
        .unwrap()
}
#[test]
fn original_registration_episode_refuses_getter_and_factory_mutation_before_acquisition() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Pure Rust incarnation controls grant no native constructor or callback policy.
    for factory in [false, true] {
        let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
        let (index, local) = register(&world);
        let birth = Rc::clone(&world.borrow().entries[index].as_ref().unwrap().birth);
        let calls = Cell::new(0);
        let reached = NativeLiteralWorld::register_using(
            &world,
            key(),
            Rc::clone,
            |value, current| {
                calls.set(calls.get() + 1);
                assert!(world.try_borrow_mut().is_ok());
                if factory {
                    Ok(b"changed resident spelling".to_vec())
                } else {
                    let extra = world.borrow_mut().acquire_registration(index).unwrap();
                    let retired = world.borrow_mut().release(index);
                    drop(retired);
                    drop(extra);
                    current()?;
                    Ok(value.as_ref().clone())
                }
            },
            || {
                let extra = world.borrow_mut().acquire_registration(index).unwrap();
                let retired = world.borrow_mut().release(index);
                drop(retired);
                drop(extra);
                Ok(Rc::new(b"registered".to_vec()))
            },
            &NativeLiteralCallbacks {
                check: &|| Ok(()),
                refuse: &std::convert::identity,
                retain: &std::convert::identity,
            },
        );
        assert!(matches!(
            reached,
            Err(ValueError::CommandProtocolUnavailable(
                "changed original literal registration episode"
            ))
        ));
        assert_eq!(calls.get(), 1);
        assert_eq!(world.borrow().registration_owners(index), Some(1));
        assert!(Rc::ptr_eq(
            &world.borrow().entries[index].as_ref().unwrap().birth,
            &birth
        ));
        assert!(Rc::ptr_eq(
            world.borrow().registration_value(index).unwrap(),
            &local
        ));
        let retired = world.borrow_mut().release(index);
        drop(retired);
        drop(local);
    }
}
#[test]
fn original_registration_exhaustion_refuses_without_clone_or_slot_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let world = Rc::new(RefCell::new(NativeLiteralWorld::default()));
    let (index, local) = register(&world);
    world.borrow_mut().entries[index]
        .as_mut()
        .unwrap()
        .local_arrays = usize::MAX;
    let before = Rc::strong_count(&local);
    let reached = NativeLiteralWorld::register_using(
        &world,
        key(),
        Rc::clone,
        |value, current| {
            current()?;
            Ok(value.as_ref().clone())
        },
        || panic!("matching original cannot call factory"),
        &NativeLiteralCallbacks {
            check: &|| Ok(()),
            refuse: &std::convert::identity,
            retain: &std::convert::identity,
        },
    );
    assert!(matches!(
        reached,
        Err(ValueError::CommandProtocolUnavailable(
            "literal registration owners exhausted"
        ))
    ));
    assert_eq!(Rc::strong_count(&local), before);
    assert_eq!(world.borrow().registration_owners(index), Some(usize::MAX));
    world.borrow_mut().entries[index]
        .as_mut()
        .unwrap()
        .local_arrays = 1;
    let retired = world.borrow_mut().release(index);
    drop(retired);
    drop(local);
}
