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

//! `callback` / `mymethod` — build a command prefix that re-enters a method
//! of the current object.
//!
//! One man page (`callback.n`, "callback, mymethod — Generate callbacks to
//! methods") documents both spellings, so both live here and share its hover
//! text: `mymethod` exists purely "for compatibility with the ooutil and snit
//! packages of Tcllib".
//!
//! **Version gating.** Both are genuine core `::oo::Helpers` members from the
//! 8.7-line `TclOO` onwards, which shipped as Tcl **9.0** (the 8.7 branch was
//! never cut as a stable release, so this registry's dialect axis reaches them
//! first at 9.0 — the same reasoning [`super::oo_classvariable`] records).
//! tclsh 9.0.4 vs tclsh 8.6.14, verbatim:
//!
//! ```text
//! 9.0.4: info commands ::oo::Helpers::*
//!        -> ::oo::Helpers::callback ::oo::Helpers::classvariable
//!           ::oo::Helpers::link ::oo::Helpers::mymethod ::oo::Helpers::next
//!           ::oo::Helpers::nextto ::oo::Helpers::self
//! 8.6.14: info commands ::oo::Helpers::*
//!        -> ::oo::Helpers::next ::oo::Helpers::nextto ::oo::Helpers::self
//!           ;# and a bare `callback Foo` in a method body is
//!           ;# `invalid command name "callback"`
//! ```
//!
//! `mymethod` therefore gets the **two** entries `link` gets: a 9.0+ core one
//! and an 8.6/8.7 one gated on Tcllib's `ooutil`, which installs a real
//! `proc ::oo::Helpers::mymethod` (tcllib-2-0 `modules/ooutil/ooutil.tcl:18`).
//! `callback` gets only the core entry — `ooutil` does not provide that
//! spelling, and under 8.6 the name reaches only a hand-rolled "`TclOO` Tricks"
//! wiki helper, i.e. an ordinary user `proc` the workspace index resolves on
//! its own (issue #923 audit, `ticklecharts` idx 51).
//!
//! **Scope.** Both are [`Traits::TCLOO_METHOD_CONTEXT`] *and*
//! [`Traits::TCLOO_REQUIRES_METHOD_FRAME`], on the same evidence as `link` /
//! `self` / `classvariable` — tclsh 9.0.4:
//!
//! ```text
//! top level:                  invalid command name "callback"
//! ::oo::Helpers::callback Foo: ::oo::Helpers::callback may only be called
//!                              from inside a method
//! class `initialise` body:    namespace which -command callback
//!                               -> ::oo::Helpers::callback   (resolves)
//!                             callback Foo
//!                               -> callback may only be called from inside
//!                                  a method                  (but cannot run)
//! method body:                callback Foo 1 2
//!                               -> ::oo::Obj22::my Foo 1 2
//! ```
//!
//! Neither carries a dispatch trait. `callback`/`mymethod` **build** a command
//! prefix and return it; nothing is invoked at the call site, and the method
//! named may well run in a completely different frame later (that is the whole
//! point of the command). Claiming [`Traits::TCLOO_SELF_DISPATCH`] would make
//! every consumer that walks dispatch sites treat the word as a call — see
//! that trait's docs for why `link`, which likewise only *prepares* a call,
//! is kept out of the family too.
use crate::prelude::*;

const CALLBACK_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "callback methodName ?arg ...?",
    ..FormSpec::DEFAULT
}];

const MYMETHOD_FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "mymethod methodName ?arg ...?",
    ..FormSpec::DEFAULT
}];

const SNIPPET: &str = "Used within the body of a method, constructor, or destructor to generate a script fragment (a proper list) that will invoke methodName on the current object — as reported by self — when it is later executed. Any additional arguments are supplied as leading arguments to that callback. mymethod is an alias of callback, named for compatibility with Tcllib's ooutil and snit packages. The generated prefix keeps working from outside any method (that is its purpose: file events, traces, after scripts), and it keeps naming the right object even if the object is renamed. methodName may be any exported or unexported method, but not a private one; if no such method exists when the callback fires, the object's unknown method handler is called.";

const EXAMPLES: &str = "oo::class create Echo {\n    variable chan\n    constructor {c} {\n        set chan $c\n        fileevent $chan readable [callback Readable]\n    }\n    method Readable {} {\n        if {[gets $chan line] >= 0} {\n            puts $chan $line\n        }\n    }\n}";

/// Hover for one spelling — same page, same prose, only the synopsis line
/// differs, so the two cannot drift apart.
const fn hover(synopsis: &'static [&'static str]) -> HoverSnippet {
    HoverSnippet {
        summary: "build a command prefix that invokes a method of the current object",
        synopsis,
        snippet: SNIPPET,
        source: "Tcl callback(n)",
        examples: EXAMPLES,
        return_value: "A command prefix (a proper list) that invokes methodName on the current object.",
    }
}

const CALLBACK_SYNOPSIS: &[&str] = &["callback methodName ?arg ...?"];
const MYMETHOD_SYNOPSIS: &[&str] = &["mymethod methodName ?arg ...?"];

/// Traits shared by every spelling and every dialect entry below.
///
/// See the module docs for the tclsh evidence behind each one.
const TRAITS: Traits = Traits::LANGUAGE_KEYWORD
    .union(Traits::TCLOO_METHOD_CONTEXT)
    .union(Traits::TCLOO_REQUIRES_METHOD_FRAME);

/// The core Tcl 9.0+ `oo::Helpers::callback`.
#[must_use]
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "callback",
        traits: TRAITS,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        hover: Some(hover(CALLBACK_SYNOPSIS)),
        forms: CALLBACK_FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// The core Tcl 9.0+ `oo::Helpers::mymethod` — `callback` under its
/// Tcllib-compatibility name.
#[must_use]
pub fn mymethod_spec() -> CommandSpec {
    CommandSpec {
        name: "mymethod",
        traits: TRAITS,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        hover: Some(hover(MYMETHOD_SYNOPSIS)),
        forms: MYMETHOD_FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// The Tcllib `ooutil` package's own `::oo::Helpers::mymethod`, the only way
/// to get this spelling under Tcl 8.6/8.7 (core `TclOO` gained it with the
/// 8.7-line work that shipped as 9.0 — see the module doc).
#[must_use]
pub fn mymethod_spec_ooutil_86() -> CommandSpec {
    CommandSpec {
        name: "mymethod",
        traits: TRAITS,
        dialects: Some(DialectSet::TCL86),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        hover: Some(hover(MYMETHOD_SYNOPSIS)),
        forms: MYMETHOD_FORMS,
        tcllib_package: Some("oo::util"),
        required_package: Some("oo::util"),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The version dimension of issue #923's `ticklecharts` idx 51: a bare
    /// `callback` in a method body is a genuine core command on 9.0+ and a
    /// genuinely unknown one on 8.6 (where `ticklecharts` supplies its own
    /// `proc ::oo::Helpers::callback` behind a `package vcompare` guard).
    /// Both facts are dialect data, not a branch in a consumer.
    #[test]
    fn callback_is_core_only_from_90() {
        assert_eq!(spec().dialects, Some(DialectSet::TCL90_PLUS));
        assert_eq!(mymethod_spec().dialects, Some(DialectSet::TCL90_PLUS));
        // No package gate on the core entries — 9.0 needs no `package
        // require` (tclsh 9.0.4: `info commands ::oo::Helpers::callback`
        // answers straight out of a bare interpreter).
        assert_eq!(spec().required_package, None);
        assert_eq!(mymethod_spec().required_package, None);
    }

    /// `mymethod` — and only `mymethod` — has an 8.6 route: Tcllib's
    /// `ooutil` defines `proc ::oo::Helpers::mymethod` (tcllib-2-0
    /// `modules/ooutil/ooutil.tcl:18`). `callback` is not among the procs
    /// that file installs, so no second entry may exist for it.
    #[test]
    fn only_mymethod_has_an_ooutil_86_route() {
        let ooutil = mymethod_spec_ooutil_86();
        assert_eq!(ooutil.dialects, Some(DialectSet::TCL86));
        assert_eq!(ooutil.required_package, Some("oo::util"));
        assert_eq!(ooutil.tcllib_package, Some("oo::util"));
    }

    /// Scope, not dispatch: both spellings resolve only where `::oo::Helpers`
    /// is on the namespace path *and* need a real method frame, and neither
    /// may claim a dispatch trait — they build a command prefix, they do not
    /// call anything (module doc has the tclsh 9.0.4 transcript).
    #[test]
    fn both_spellings_are_method_frame_scoped_and_never_dispatch() {
        for s in [spec(), mymethod_spec(), mymethod_spec_ooutil_86()] {
            assert!(
                s.traits.contains(Traits::TCLOO_METHOD_CONTEXT),
                "{}",
                s.name
            );
            assert!(
                s.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME),
                "{}",
                s.name
            );
            assert!(
                !s.traits.intersects(
                    Traits::TCLOO_SELF_DISPATCH
                        .union(Traits::TCLOO_NEXT_CHAIN)
                        .union(Traits::TCLOO_INTROSPECTION)
                        .union(Traits::TCLOO_BINDS_METHOD_ALIAS)
                ),
                "{} prepares a call, it does not make one",
                s.name
            );
        }
    }
}
