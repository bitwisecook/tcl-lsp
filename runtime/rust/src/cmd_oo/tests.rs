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

use crate::interp::{obj_bytes, Code, Interp, Param};
use crate::{counters, list, obj};

mod class_hierarchy;
mod configurable;
mod construction;
mod definition;
mod destruction;
mod introspection;
mod method_calls;
mod method_context;
mod slots;

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(counters::finalize(), 0, "residual objs/bufs");
    assert_eq!(counters::double_free_count(), 0);
}

fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?} -> {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}

/// Whether `haystack` contains the byte subsequence `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn oo_package_is_provided() {
    leak_free(|i| {
        assert_eq!(ok(i, b"package require tcl::oo"), b"1.3.1");
    });
}

#[test]
fn foundation_abstract_singleton_classes() {
    leak_free(|i| {
        // The three foundation metaclasses exist in a fresh interp.
        assert_eq!(ok(i, b"info object class ::oo::singleton"), b"::oo::class");
        assert_eq!(ok(i, b"info object class ::oo::abstract"), b"::oo::class");
        assert_eq!(
            ok(i, b"info object class ::oo::SingletonInstance"),
            b"::oo::class"
        );
        // singleton/abstract are metaclasses (superclass oo::class).
        assert_eq!(
            ok(i, b"info class superclasses ::oo::singleton"),
            b"::oo::class"
        );
        assert_eq!(
            ok(i, b"info class superclasses ::oo::abstract"),
            b"::oo::class"
        );
        // SingletonInstance is a plain subclass of oo::object.
        assert_eq!(
            ok(i, b"info class superclasses ::oo::SingletonInstance"),
            b"::oo::object"
        );
        assert_eq!(
            ok(i, b"lsort [info class subclasses ::oo::class]"),
            b"::oo::abstract ::oo::configurable ::oo::singleton"
        );
    });
}
