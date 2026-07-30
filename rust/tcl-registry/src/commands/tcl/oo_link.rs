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

//! `link` — expose a method as a bareword command in the object's own namespace.
//!
//! Version-gated, not just `TCL86_PLUS`-unconditional (issue #923, Codex
//! review on PR #1020): `oo::Helpers::link` only became a genuine core
//! `TclOO` builtin in Tcl **9.0** (confirmed against tclsh 9.0.4 — bare
//! `link` inside a method body works with no `package require` at all,
//! and `ooutil` is confirmed absent via `package present ooutil`). Under
//! 8.6/8.7 it does not exist in core Tcl at all (confirmed against tclsh
//! 8.6.14 — `invalid command name "link"`); the same bare-inside-a-method
//! spelling is provided there only by the Tcllib `ooutil` package's own
//! `::oo::Helpers::link`, once loaded. Two specs, not one: an
//! unconditional 9.0+ core entry, and an 8.6/8.7 entry gated on
//! `ooutil` — a single `TCL86_PLUS`-wide spec would wrongly treat a
//! bare 8.6 `link` (with no `package require ooutil` anywhere in the
//! file) as a known, resolvable command.
//!
//! Both entries are **method-context-scoped** (issue #1026,
//! [`Traits::TCLOO_METHOD_CONTEXT`]): `link` lives in `::oo::Helpers`, which
//! only a method body's namespace path reaches, so a top-level `link foo`
//! is `invalid command name "link"` and `info commands ::link` is empty
//! (tclsh 9.0.4). That holds under 8.6-with-`ooutil` too — installing
//! `::oo::Helpers::link` does not make the bare word callable outside a
//! method (verified against tclsh 8.6.14 with the package's own
//! `proc ::oo::Helpers::link` shape in place). The qualified spelling
//! itself is registered separately by [`super::oo_helpers`].
//!
//! [`Traits::TCLOO_BINDS_METHOD_ALIAS`] declares what the call *does* — each
//! argument word binds a bareword alias for a method of the current object —
//! so the analyser's class-body walk finds `link` calls through the registry
//! rather than by spelling.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "link linkName ?linkName ...?",
    dialects: None,
}];
const HOVER: HoverSnippet = HoverSnippet {
    summary: "expose a method as a bareword command in the object's own namespace",
    synopsis: &[
        "link linkName ?linkName ...?",
        "link {linkName targetName} ?...?",
    ],
    snippet: "The link command is used within the body of a method, constructor, or destructor to create a command in the object's own private namespace that invokes a method on the current object. Each linkName is either a plain name (aliasing to the method of the same name) or a two-element list {linkName targetName} aliasing linkName to a differently-named method.",
    source: "Tcl man page link.n",
    examples: "",
    return_value: "",
};

/// The core Tcl 9.0+ `oo::Helpers::link` — no package needed.
#[must_use]
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "link",
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::TCLOO_METHOD_CONTEXT)
            .union(Traits::TCLOO_REQUIRES_METHOD_FRAME)
            .union(Traits::TCLOO_BINDS_METHOD_ALIAS),
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HOVER),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// The Tcllib `ooutil` package's own `::oo::Helpers::link`, the only way
/// to get this spelling under Tcl 8.6/8.7 (core `TclOO` gained it only in
/// 9.0 — see the module doc).
#[must_use]
pub fn spec_ooutil_86() -> CommandSpec {
    CommandSpec {
        name: "link",
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::TCLOO_METHOD_CONTEXT)
            .union(Traits::TCLOO_REQUIRES_METHOD_FRAME)
            .union(Traits::TCLOO_BINDS_METHOD_ALIAS),
        dialects: Some(DialectSet::TCL86),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HOVER),
        forms: FORMS,
        tcllib_package: Some("ooutil"),
        required_package: Some("ooutil"),
        ..CommandSpec::DEFAULT
    }
}
