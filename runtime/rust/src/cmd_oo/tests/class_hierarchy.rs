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

//! Precedence: C3 ordering, mixins of mixins and deeply nested mixin chains.

use super::*;

#[test]
fn precedence_ordering_c3_and_mixins() {
    leak_free(|i| {
        // oo-21.2: diamond + mixin precedence. Expected call order
        // `Fmix Emix o D B C A` (shared bases deferred via keep-last).
        ok(
            i,
            b"oo::class create A { method m {} {lappend ::result A} }",
        );
        ok(
            i,
            b"oo::class create B { superclass A; method m {} {lappend ::result B;next} }",
        );
        ok(
            i,
            b"oo::class create C { superclass A; method m {} {lappend ::result C;next} }",
        );
        ok(
            i,
            b"oo::class create D { superclass B C; method m {} {lappend ::result D;next} }",
        );
        ok(
            i,
            b"oo::class create Emix { superclass C; method m {} {lappend ::result Emix;next} }",
        );
        ok(
            i,
            b"oo::class create Fmix { superclass Emix; method m {} {lappend ::result Fmix;next} }",
        );
        ok(i, b"D create o");
        ok(
            i,
            b"oo::objdefine o { method m {} {lappend ::result o;next}; mixin Fmix }",
        );
        ok(i, b"set ::result {}; o m");
        assert_eq!(ok(i, b"set ::result"), b"Fmix Emix o D B C A");
        // A class mixin runs before the class itself (oo-14.8 ordering):
        // the mixin's method runs first and chains via `next` to the class.
        ok(
            i,
            b"oo::class create mix { method t {} {lappend ::r mix; next} }",
        );
        ok(
            i,
            b"oo::class create cls { mixin mix; method t {} {lappend ::r cls; return $::r} }",
        );
        ok(i, b"set ::r {}");
        assert_eq!(ok(i, b"[cls new] t"), b"mix cls");
        // A filter method defined by several providers wraps once per
        // implementation, in precedence order (oo-21.3): F.flt then G.flt.
        ok(
            i,
            b"oo::class create G { method flt {} {lappend ::r G-flt; next} }",
        );
        ok(i, b"oo::class create F { superclass G; method flt {} {lappend ::r F-flt; next}; method go {} {lappend ::r go} }");
        ok(i, b"F create gf");
        ok(i, b"oo::objdefine gf filter flt");
        ok(i, b"set ::r {}; gf go");
        assert_eq!(ok(i, b"set ::r"), b"F-flt G-flt go");
    });
}

#[test]
fn mixin_of_mixin_methods_are_reachable() {
    leak_free(|i| {
        // Bug 1960703 (oo-14.6): a method from a mixin's own mixin is
        // reachable via the call chain.
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"oo::class create A { superclass parent; method egg {} {return chicken} }",
        );
        ok(
            i,
            b"oo::class create B { superclass parent; mixin A; method bar {} {my egg} }",
        );
        ok(
            i,
            b"oo::class create C { superclass parent; mixin B; method foo {} {my bar} }",
        );
        assert_eq!(ok(i, b"[C new] foo"), b"chicken");
    });
}

/// Regression coverage for the native-stack recursion hazard
/// `MAX_MRO_DEPTH`/`MAX_MRO_VISITS` guard against: `linearize_class`
/// (backing `method_chain_faceted` — the hot path for every ordinary
/// method dispatch — and `class_precedence`, `info class call`) and
/// `gather_class_props` (`info class properties -all`) recurse once
/// per mixin/superclass level, with only a same-branch cycle guard
/// bounding a malformed hierarchy — an unguarded deep mixin chain
/// overflows the native stack. Confirmed crash reproduction: a deep
/// `mixin` chain (`oo::class create C$i { mixin C[i-1] }`, no `{*}`
/// needed) SIGABRTs between depth 100-150 on a 256 KiB stack, and still
/// crashes at depth 2000 on a 1 MiB stack (a plain `superclass` chain
/// hits the same recursion but is masked by superclass cycle-check cost, which makes naive *construction* slow
/// before reaching crash depth — mixins avoid that and reproduce
/// cleanly). This builds a 2000-deep mixin chain (matching that
/// confirmed-still-crashing depth) and drives both guarded functions
/// over the whole thing via `info class call` and `info class
/// properties -all`; the assertion is that both complete at all, not
/// what they return — `MAX_MRO_DEPTH` (64) means the reported
/// precedence/property set is legitimately truncated for a hierarchy
/// this deep, the same graceful "just stop descending" degradation the
/// `path`/`seen` cycle guards already apply to a malformed hierarchy,
/// rather than erroring the whole dispatch out or crashing.
#[test]
fn deeply_nested_mixin_chain_survives_linearisation_and_property_gathering() {
    leak_free(|i| {
        const DEPTH: usize = 2000;
        ok(i, b"oo::class create C0");
        for n in 1..DEPTH {
            ok(
                i,
                format!("oo::class create C{n} {{ mixin C{} }}", n - 1).as_bytes(),
            );
        }
        let last = DEPTH - 1;
        let _ = i.eval_str(format!("info class call C{last} foo").as_bytes());
        let _ = i.eval_str(format!("info class properties C{last} -all").as_bytes());
        for n in (0..DEPTH).rev() {
            let _ = i.eval_str(format!("C{n} destroy").as_bytes());
        }
    });
}

/// A moderately nested mixin chain (well under `MAX_MRO_DEPTH`) still
/// linearises completely and correctly — the safety net must not fire,
/// let alone truncate anything, on realistic nesting depths. Checks both
/// fixed functions: method dispatch still resolves through the whole
/// mixin chain to the base class's implementation (`linearize_class` via
/// `method_chain_faceted`), and `-all` property gathering still unions
/// every level's declared properties (`gather_class_props`) — exactly
/// the behaviour before this fix, not merely "does not crash".
#[test]
fn moderately_nested_mixin_chain_still_behaves_identically() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C0 { method foo {} { return from-c0 } }",
        );
        ok(i, b"oo::class create C1 { mixin C0 }");
        ok(i, b"oo::class create C2 { mixin C1 }");
        ok(i, b"oo::class create C3 { mixin C2 }");
        ok(i, b"C3 create obj");
        assert_eq!(ok(i, b"obj foo"), b"from-c0");
        ok(i, b"obj destroy");

        ok(
            i,
            b"oo::configurable create Base { property p0 -kind readable }",
        );
        ok(
            i,
            b"oo::configurable create P1 { mixin Base; property p1 -kind readable }",
        );
        ok(
            i,
            b"oo::configurable create P2 { mixin P1; property p2 -kind readable }",
        );
        assert_eq!(
            ok(i, b"lsort [info class properties P2 -all]"),
            b"-p0 -p1 -p2"
        );

        ok(i, b"C3 destroy; C2 destroy; C1 destroy; C0 destroy");
        ok(i, b"P2 destroy; P1 destroy; Base destroy");
    });
}
