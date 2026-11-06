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

use super::{new_string_bytes, retain_script_location, script_location, Owned, SCRIPT_LOCATIONS};
use std::cell::RefCell;
use std::rc::Rc;

struct RetainedObject(Option<Owned>);

impl Drop for RetainedObject {
    fn drop(&mut self) {
        if self.0.is_some() {
            assert!(SCRIPT_LOCATIONS.try_with(|_| ()).is_err());
            assert!(!crate::native_source::context_map_available_for_retirement_test());
        }
        // The real Owned field retires only after the unavailable-map probes.
    }
}

thread_local! {
    // Register this owner first so both metadata maps retire before its object.
    static HOLDER: RefCell<RetainedObject> = const { RefCell::new(RetainedObject(None)) };
}

#[test]
fn original_object_retirement_after_metadata_tls_destruction_does_not_abort() {
    let worker = std::thread::spawn(|| {
        HOLDER.with(|_| ());
        let object = Owned::fresh(new_string_bytes(b"retained original"));
        retain_script_location(object.as_ptr(), None, 7);
        // Initialize the real weak-context TLS after the holder as well.
        crate::native_source::forget_context(object.as_ptr());
        HOLDER.with(|holder| holder.borrow_mut().0 = Some(object));
    });
    worker
        .join()
        .expect("original object retires at thread exit");
}

#[test]
fn original_object_retirement_while_tls_is_live_releases_source_metadata() {
    let object = Owned::fresh(new_string_bytes(b"ordinary retirement"));
    let pointer = object.as_ptr();
    let file: Rc<[u8]> = Rc::from(b"original.tcl".as_slice());
    let weak = Rc::downgrade(&file);
    retain_script_location(pointer, Some(file), 11);
    assert!(weak.upgrade().is_some());
    assert_eq!(script_location(pointer).unwrap().1, 11);
    drop(object);
    assert!(script_location(pointer).is_none());
    assert!(weak.upgrade().is_none(), "live metadata owner was released");
}
