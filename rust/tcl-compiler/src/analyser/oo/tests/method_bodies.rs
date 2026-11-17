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

//! The bodies the definition walk enters: `initialise`, `classvariable` declarations and property accessors.

use super::*;

// OO body-walks: `initialise` body, `property -get/-set` accessor bodies,
// and the `new` / `createWithNamespace` class-command variants.

#[test]
fn oo_initialise_body_is_walked() {
    // A `variable` read inside `initialise { … }` must not false-fire
    // W210 read-before-set — the class-level init body is walked with the
    // class's instance variables visible.
    let src = "oo::class create Foo {\n\
                   variable cache\n\
                   initialise { set cache [dict create] }\n\
                   method get {k} { return [dict get $cache $k] }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
        "initialise body should be walked cleanly: {:?}",
        r.diagnostics
    );
}

/// A class-shared variable set up by the class-level `initialize` script
/// and reached from a method through `classvariable` must not draw a
/// read-before-set.
///
/// Oracled on tclsh 9.0.4: this exact class answers `5e-6`. The
/// `initialize` body runs in the class's own object namespace, so the
/// write and the read live in different frames — which is precisely why
/// `classvariable` has to be modelled as a declaration rather than as a
/// plain local.
#[test]
fn classvariable_reaches_a_variable_initialise_declared() {
    let src = "oo::class create Foo {\n\
                   initialize {\n\
                   variable Pitch\n\
                   const Pitch 5e-6\n\
                   }\n\
                   method m {} { classvariable Pitch; return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        !r.diagnostics.iter().any(|d| d.code.as_str() == "W210"),
        "classvariable reaches the initialise-declared cell: {:?}",
        r.diagnostics
    );
}

/// Near variants of the same shape, each **run** on tclsh
/// 9.0.4: a plain `set` instead of `const`, a value-bearing
/// `variable`, the British `initialise` spelling, the `oo::define`
/// form, two names in one `classvariable` call (both must bind — a
/// `VarWrite` role on index 0 alone leaves the second unbound), and a
/// write/read split across two methods.
///
/// The `variable Pitch` before the `const` is load-bearing,
/// not noise: an `initialize` script runs in a call frame, so a bare
/// `const Pitch 5e-6` there makes a *frame-local* constant that no
/// method can reach (tclsh 9.0.4: `can't read "Pitch": no such
/// variable`). Only the `variable` declaration aims the write at the
/// class-shared cell — which is why that shape is absent from this
/// clean-list.
#[test]
fn classvariable_variants_stay_clean() {
    for (label, src) in [
        (
            "set instead of const",
            "oo::class create Foo {\n\
                 initialize { variable Pitch; set Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "value-bearing variable",
            "oo::class create Foo {\n\
                 initialize { variable Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "initialise spelling",
            "oo::class create Foo {\n\
                 initialise { variable Pitch; const Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "oo::define form",
            "oo::class create Foo {}\n\
                 oo::define Foo {\n\
                 initialize { variable Pitch; const Pitch 5e-6 }\n\
                 method m {} { classvariable Pitch; return $Pitch }\n\
                 }",
        ),
        (
            "two names in one call",
            "oo::class create Foo {\n\
                 initialize { variable a 1; variable b 2 }\n\
                 method m {} { classvariable a b; return [list $a $b] }\n\
                 }",
        ),
        (
            "written in one method, read in another",
            "oo::class create Foo {\n\
                 method w {} { classvariable Count; set Count 1 }\n\
                 method r {} { classvariable Count; return $Count }\n\
                 }",
        ),
        (
            // The `classvariable(n)` manual page's own example.
            "manual-page Counted example",
            "oo::class create Counted {\n\
                 initialise { variable count 0 }\n\
                 variable number\n\
                 constructor {} { classvariable count; set number [incr count] }\n\
                 method report {} { classvariable count; puts \"$number of $count\" }\n\
                 }",
        ),
    ] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl9.0");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W210" | "W211")),
            "{label}: {:?}",
            r.diagnostics
        );
    }
}

/// The other half: the classvariable handling must not silence a genuine
/// unbound read. Without `classvariable`, a method's `$Pitch` is an
/// *instance* variable that the class-level `initialize` never wrote —
/// tclsh 9.0.4 raises `can't read "Pitch": no such variable` — so W210
/// is correct and has to stay.
#[test]
fn a_method_read_without_classvariable_still_warns() {
    let src = "oo::class create Foo {\n\
                   initialize { variable Pitch; const Pitch 5e-6 }\n\
                   method m {} { ::return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.diagnostics.iter().any(|d| d.code.as_str() == "W210"),
        "an instance-side read of a class-scoped cell is a real bug: {:?}",
        r.diagnostics
    );
}

/// The same snippet under 8.6 reports what 8.6 actually lacks and never
/// W210. `initialize` is a 9.0 member, so tclsh 8.6 stops at it with
/// `invalid command name "initialize"` and the `const` inside is never
/// reached. `classvariable` is reachable there only through Tcllib's
/// `oo::util`, which the document requires.
#[test]
fn the_reporter_snippet_under_86_reports_the_missing_commands() {
    let src = "package require oo::util\n\
                   oo::class create Foo {\n\
                   initialize { variable Pitch; const Pitch 5e-6 }\n\
                   method m {} { classvariable Pitch; return $Pitch }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"W210"),
        "8.6 must not add a read-before-set: {:?}",
        r.diagnostics
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W002" && d.message.contains("'initialize'")),
        "8.6 has no `initialize` in oo::define (it is 9.0): {:?}",
        r.diagnostics
    );
}

/// `classvariable` binds only literal, unqualified names: a
/// substitution names no cell the analyser can see, so it stays the
/// W212 it already was rather than silently declaring `$dyn`.
#[test]
fn a_dynamic_classvariable_name_is_not_a_declaration() {
    let src = "oo::class create Foo { method m {} { classvariable $dyn; return $Pitch } }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(
        r.diagnostics.iter().any(|d| d.code.as_str() == "W212"),
        "a substituted name is still reported: {:?}",
        r.diagnostics
    );
}

#[test]
fn oo_property_accessor_bodies_are_walked() {
    // `-get`/`-set` accessor bodies are walked with the instance variable
    // `val` and the implicit `value` visible — no false W210 / W307.
    // `property` is a 9.0 member, so the vector runs under a 9.0 dialect.
    let src = "oo::configurable create Bar {\n\
                   variable val\n\
                   property color -get { return $val } -set { set val $value }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    assert!(r.all_classes.contains_key("::Bar"));
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W210" | "W307")),
        "property accessor bodies should be walked cleanly: {:?}",
        r.diagnostics
    );
}

#[test]
fn oo_class_create_with_namespace_is_not_exported() {
    // C Tcl 9.0.4 and 8.6 both reject this ordinary command dispatch:
    // createWithNamespace exists internally but is not exported on
    // oo::class. A rejected call cannot introduce a class.
    let mut a = Analyser::new();
    let r = a.analyse("oo::class createWithNamespace MyCls ::ns { }", "tcl8.6");
    assert!(
        !r.all_classes.keys().any(|k| k.contains("MyCls")),
        "an unexported manufacturer must not record a class: {:?}",
        r.all_classes.keys().collect::<Vec<_>>()
    );
}
