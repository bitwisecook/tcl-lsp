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

//! The command references `superclass`, `mixin`, `forward` and itcl `inherit` record.

use super::*;

/// A command reference the analyser records for a class-body member
/// argument (`superclass`/`mixin`/`inherit`/`forward` target).  `written`
/// is the token text; `resolved` the qualified command it denotes.
fn has_cmd_ref(r: &crate::analyser::types::AnalysisResult, written: &str, resolved: &str) -> bool {
    r.command_invocations
        .iter()
        .any(|inv| inv.name == written && inv.resolved_qualified_name.as_deref() == Some(resolved))
}

// TP: a `superclass ::ns::Base` names the base class — recorded as a
// command reference so references / go-to-definition / rename reach it.
#[test]
fn superclass_records_a_command_reference_to_the_base_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass ::ns::Base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "superclass must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TP: `mixin ::ns::Role` — the mixed-in class is a command reference.
#[test]
fn mixin_records_a_command_reference_to_the_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Role {}\n  \
                 oo::class create C { mixin ::ns::Role }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "::ns::Role", "::ns::Role"));
}

// The `forward` target resolves through the member grammar's
// `ArgRole::CommandName` and records the delegated command.
#[test]
fn forward_target_still_records_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  proc helper {} {}\n  \
                 oo::class create C { forward f ::ns::helper }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "::ns::helper", "::ns::helper"));
}

// TP: a bare `superclass Base` (same namespace) resolves to the enclosing
// namespace's class — the one-hop call-site resolution, not an ancestor.
#[test]
fn bare_superclass_resolves_in_the_class_namespace() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub { superclass Base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(has_cmd_ref(&r, "Base", "::ns::Base"));
}

// TP (itcl): `inherit ::ns::Base` names a base class the same way.
#[test]
fn itcl_inherit_records_a_command_reference_to_the_base_class() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "itcl::class ::ns::Base {}\n\
                 itcl::class ::ns::Sub { inherit ::ns::Base }\n",
            "tcl8.6",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "itcl inherit must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TN: a dynamic base name (`superclass $base`) names no static command —
// no reference is recorded (nothing to navigate or rename).
#[test]
fn dynamic_superclass_records_no_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  \
                 oo::class create Sub { superclass $base }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        !r.command_invocations
            .iter()
            .any(|inv| inv.name.contains('$')),
        "a dynamic superclass name must not be recorded as a reference"
    );
}

// FP guard: `mixin -append ::ns::M` — the `-append` flag is not a class,
// only `::ns::M` is a reference.
#[test]
fn mixin_append_flag_is_not_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create M {}\n  \
                 oo::class create C { mixin -append ::ns::M }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::M", "::ns::M"),
        "the class is a reference"
    );
    assert!(
        !r.command_invocations
            .iter()
            .any(|inv| inv.name == "-append"),
        "the -append flag must not be recorded as a command reference"
    );
}

// TP: the inline `oo::define Sub superclass Base` form (no `{body}` block)
// records the base-class reference too — the same as the braced body form.
// (The inline path is separate from the body walk.)
#[test]
fn inline_oo_define_superclass_records_a_command_reference() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create Base {}\n  \
                 oo::class create Sub {}\n  oo::define Sub superclass ::ns::Base\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        has_cmd_ref(&r, "::ns::Base", "::ns::Base"),
        "inline `oo::define … superclass` must record a command reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}

// TP: a `forward` target may legally begin with `-` (a command named
// `-foo`).  The `-`-flag filter applies only to class-list members
// (`mixin -append`), never to the forward command name.
#[test]
fn forward_target_beginning_with_dash_is_recorded() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "namespace eval ::ns {\n  oo::class create C { forward f -foo }\n}\n",
            "tcl9.0",
        )
        .clone();
    assert!(
        r.command_invocations.iter().any(|inv| inv.name == "-foo"),
        "a hyphen-prefixed forward target must be recorded: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>()
    );
}
