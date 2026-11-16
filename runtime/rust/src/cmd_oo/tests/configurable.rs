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

//! TIP 558: `configure`, property accessors and kinds, property storage and the option diagnostics.

use super::*;

/// The `property` half of the option-table prefix matching, which needs
/// a configurable class.
#[test]
fn configure_property_word_abbreviates() {
    const MUST: &str = "must be -yellow or -zed";
    leak_free(|i| {
        let err_of = |i: &mut Interp, src: &[u8]| {
            assert_eq!(i.eval_str(src), Code::Error, "expected an error");
            String::from_utf8_lossy(&i.result_bytes()).into_owned()
        };
        ok(
            i,
            b"oo::configurable create Q { property yellow zed\n\
                   constructor {} { my configure -yellow 0 -zed 0 } }",
        );
        ok(i, b"Q create q");
        assert_eq!(
            err_of(i, b"q configure -zz"),
            format!("bad property \"-zz\": {MUST}")
        );
        assert_eq!(
            err_of(i, b"q configure {}"),
            format!("ambiguous property \"\": {MUST}")
        );
        assert_eq!(
            err_of(i, b"q configure -"),
            format!("ambiguous property \"-\": {MUST}")
        );
        // A unique prefix resolves — and writes the *canonical* property.
        ok(i, b"q configure -ye 1");
        assert_eq!(ok(i, b"q configure -yellow"), b"1");
        assert_eq!(ok(i, b"q configure -ye"), b"1");
        i.eval_str(b"Q destroy");
    });
}

#[test]
fn tip558_configurable_configure() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"oo::configurable create Point { superclass parent; property x y; \
                  constructor args { my configure -x 0 -y 0 {*}$args } }",
        );
        ok(i, b"set pt [Point new -x 3]");
        assert_eq!(ok(i, b"$pt configure -x"), b"3");
        assert_eq!(ok(i, b"$pt configure -y"), b"0");
        ok(i, b"$pt configure -y 4");
        assert_eq!(ok(i, b"$pt configure"), b"-x 3 -y 4");
        assert_eq!(i.eval_str(b"$pt configure gorp"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad property \"gorp\": must be -x or -y");
        assert_eq!(i.eval_str(b"$pt configure -x 1 -y"), Code::Error);
        // per-object property declaration (oo-2.3).
        ok(i, b"oo::objdefine $pt property z");
        ok(i, b"$pt configure -z 5");
        assert_eq!(ok(i, b"$pt configure -z"), b"5");
        // -kind controls direction.
        ok(
            i,
            b"oo::configurable create RO { superclass parent; property r -kind readable; \
                  constructor {} { my variable r; set r 7 } }",
        );
        ok(i, b"RO create ro");
        assert_eq!(ok(i, b"ro configure -r"), b"7");
        assert_eq!(i.eval_str(b"ro configure -r 9"), Code::Error);
        assert_eq!(i.result_bytes(), b"property \"-r\" is read only");
        // Error codes (ooProp-4.x).
        ok(i, b"oo::configurable create E { superclass parent }");
        assert_eq!(i.eval_str(b"oo::define E {property -x}"), Code::Error);
        assert_eq!(ok(i, b"set ::errorCode"), b"TCL OO PROPERTY_FORMAT");
        assert_eq!(
            i.eval_str(b"oo::define E {property q -kind gorp}"),
            Code::Error
        );
        assert_eq!(ok(i, b"set ::errorCode"), b"TCL LOOKUP INDEX kind gorp");
    });
}

#[test]
fn tip558_property_custom_accessors_and_kinds() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // Custom -get/-set bodies; option/kind prefix abbreviation.
        ok(
            i,
            b"oo::configurable create Point { superclass parent; variable xyz; \
                  property x -g {return [list $xyz $xyz]} -s {set xyz $value} }",
        );
        ok(i, b"Point create p");
        ok(i, b"p configure -x 5");
        assert_eq!(ok(i, b"p configure -x"), b"5 5");
        // A getter doing break/continue is reported, not leaked as a loop error.
        ok(
                i,
                b"oo::configurable create B { superclass parent; property y -get {return -code break} }",
            );
        ok(i, b"B create b");
        assert_eq!(i.eval_str(b"b configure -y"), Code::Error);
        assert_eq!(i.result_bytes(), b"property getter for -y did a break");
        // A second declaration with a different -kind replaces membership.
        ok(
                i,
                b"oo::configurable create R { superclass parent; variable z; \
                  property z -kind readable -get {return $z}; property z -kind writable -set {set z $value} }",
            );
        ok(i, b"R create r");
        ok(i, b"r configure -z ok");
        assert_eq!(i.eval_str(b"r configure -z"), Code::Error);
        assert_eq!(i.result_bytes(), b"property \"-z\" is write only");
    });
}

#[test]
fn tip558_property_and_info_options_match_tcl_prefix_diagnostics() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        assert_eq!(
            i.eval_str(b"oo::configurable create P {property x -}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"-\": must be -get, -kind, or -set"
        );
        assert_eq!(
            i.eval_str(b"oo::configurable create Q {property x -kind r}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"ambiguous kind \"r\": must be readable, readwrite, or writable"
        );

        ok(
            i,
            b"oo::configurable create C {superclass parent; property x}",
        );
        ok(i, b"C create o");
        assert_eq!(ok(i, b"info class properties C -a"), b"-x");
        assert_eq!(ok(i, b"info object properties o -a"), b"-x");
        assert_eq!(i.eval_str(b"info class properties C {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"\": must be -all, -readable, or -writable"
        );
        assert_eq!(i.eval_str(b"info object properties o -"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"-\": must be -all, -readable, or -writable"
        );
    });
}

#[test]
fn tip558_property_storage_and_introspection() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        ok(i, b"oo::class create c {superclass parent}");
        // Empty to start; the slot stores uniqued, info sorts (oo-1.1).
        assert_eq!(ok(i, b"info class properties c"), b"");
        ok(
            i,
            b"oo::define c ::oo::configuresupport::readableproperties -set f e d",
        );
        assert_eq!(ok(i, b"info class properties c"), b"d e f");
        assert_eq!(ok(i, b"info class properties c -writable"), b"");
        ok(
            i,
            b"oo::define c ::oo::configuresupport::readableproperties -set a a a",
        );
        assert_eq!(ok(i, b"info class properties c"), b"a");
        // Writable slot is independent.
        ok(
            i,
            b"oo::define c ::oo::configuresupport::writableproperties -set w2 w1",
        );
        assert_eq!(ok(i, b"info class properties c -writable"), b"w1 w2");
        // -all merges the superclass chain (oo-1.5).
        ok(i, b"oo::class create d {superclass c}");
        ok(
            i,
            b"oo::define d ::oo::configuresupport::readableproperties -set x y z",
        );
        assert_eq!(ok(i, b"info class properties d -all"), b"a x y z");
        // Object properties + -all merges object, mixins, class chain (oo-1.9).
        ok(i, b"d create o");
        ok(
            i,
            b"oo::objdefine o ::oo::configuresupport::objreadableproperties -set m n",
        );
        assert_eq!(ok(i, b"info object properties o"), b"m n");
        assert_eq!(ok(i, b"info object properties o -all"), b"a m n x y z");
    });
}
