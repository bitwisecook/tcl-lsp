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

//! `classvariable` — link a local variable to a class-shared variable.
//!
//! **8.6/8.7 route.** Like `link` and `mymethod`, Tcllib's `ooutil` installs
//! its own `proc ::oo::Helpers::classvariable` (tcllib-2-0
//! `modules/ooutil/ooutil.tcl:27`), so a document that says
//! `package require ooutil` may legitimately write the bare word under
//! 8.6/8.7 even though core `TclOO` there has no such member. Confirmed
//! live: `tclsh8.6` with `ooutil` loaded resolves and runs a
//! `classvariable count` inside a method body exactly as 9.0 core does;
//! without the `package require`, the same script fails with
//! `invalid command name "classvariable"`. Two specs, not one — the 9.0+
//! core entry below plus [`spec_ooutil_86`] — mirroring `mymethod`'s
//! [`super::oo_callback::mymethod_spec_ooutil_86`].
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "classvariable variableName ?...?",
    dialects: None,
}];

const HOVER: HoverSnippet = HoverSnippet {
    summary: "Link local variables to variables shared by every instance of a class.",
    synopsis: &["classvariable variableName ?...?"],
    snippet: "Valid only inside a method, constructor, or destructor body. Each variableName must be an unqualified scalar name — no :: namespace separator and no array-element syntax — and classvariable links it, in the calling scope, to the like-named variable in the namespace of the class that defined the running method, so its value is shared by every instance of that class. This is the class-level counterpart to the variable command, and is equivalent to the typevariable command that the snit package (tcllib) provides for approximately the same purpose. In a method defined directly on an object (e.g. through oo::objdefine), linking a name this way behaves like namespace upvar [namespace current] $var $var for each name listed.",
    source: "Tcl classvariable(n)",
    examples: "oo::class create Counted {\n    initialise {\n        variable count 0\n    }\n    variable number\n    constructor {} {\n        classvariable count\n        set number [incr count]\n    }\n    method report {} {\n        classvariable count\n        puts \"This is instance $number of $count\"\n    }\n}\nset a [Counted new]\nset b [Counted new]\n$a report                 ;# This is instance 1 of 2\nset c [Counted new]\n$b report                 ;# This is instance 2 of 3\n$c report                 ;# This is instance 3 of 3",
    return_value: "The empty string.",
};

/// Traits shared by both the core and `ooutil` entries.
const TRAITS: Traits = Traits::LANGUAGE_KEYWORD
    .union(Traits::TCLOO_METHOD_CONTEXT)
    .union(Traits::TCLOO_REQUIRES_METHOD_FRAME);

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// The core Tcl 9.0+ `oo::Helpers::classvariable` — no package needed.
///
/// TIP 478 ("Add Expected Class Level Behaviors to oo::class") introduced
/// `classvariable`; its `Tcl-Version` metadata targeted 8.7, a branch that
/// was never cut as a stable release, and the feature landed when 8.7's
/// work was folded into 9.0. Per-version fetch of classvariable.n: absent
/// (404, both the .html and .htm extensions) under 8.4, 8.5, and 8.6 — and
/// 8.6's own define.n manual has no mention of "classvariable" anywhere in
/// its text — present, word-for-word identical, under 9.0 and 9.1.
#[must_use]
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "classvariable",
        traits: TRAITS,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HOVER),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// The Tcllib `ooutil` package's own `::oo::Helpers::classvariable`, the
/// only way to get this spelling under Tcl 8.6/8.7 (core `TclOO` gained it
/// only in 9.0 — see the module doc).
#[must_use]
pub fn spec_ooutil_86() -> CommandSpec {
    CommandSpec {
        name: "classvariable",
        traits: TRAITS,
        dialects: Some(DialectSet::TCL86),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HOVER),
        forms: FORMS,
        tcllib_package: Some("ooutil"),
        required_package: Some("ooutil"),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The version dimension: a bare `classvariable` in a method body is a
    /// genuine core command on 9.0+ with no `package require` needed
    /// (tclsh 9.0.4: `info commands ::oo::Helpers::classvariable` answers
    /// straight out of a bare interpreter).
    #[test]
    fn classvariable_is_core_only_from_90() {
        assert_eq!(spec().dialects, Some(DialectSet::TCL90_PLUS));
        assert_eq!(spec().required_package, None);
    }

    /// Tcllib's `ooutil` defines `proc ::oo::Helpers::classvariable`
    /// (tcllib-2-0 `modules/ooutil/ooutil.tcl:27`), so the 8.6/8.7 route
    /// must carry the package gate that makes a bare `classvariable`
    /// resolve only once the document says `package require ooutil`.
    #[test]
    fn classvariable_has_an_ooutil_86_route() {
        let ooutil = spec_ooutil_86();
        assert_eq!(ooutil.dialects, Some(DialectSet::TCL86));
        assert_eq!(ooutil.required_package, Some("ooutil"));
        assert_eq!(ooutil.tcllib_package, Some("ooutil"));
    }

    /// Scope, not dispatch: both entries resolve only where `::oo::Helpers`
    /// is on the namespace path *and* need a real method frame.
    #[test]
    fn both_entries_are_method_frame_scoped() {
        for s in [spec(), spec_ooutil_86()] {
            assert!(s.traits.contains(Traits::TCLOO_METHOD_CONTEXT), "{}", s.name);
            assert!(
                s.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME),
                "{}",
                s.name
            );
        }
    }
}
