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

use crate::registry_invocation::source_structure::OriginalPackageIndexSourceSurvey;

/// Whether the Logical source survey still advertises the
/// `package ifneeded q5` body command. The Native (`tcl8.6`) body-region
/// path is not used here: it issues no definite source advice for an
/// entered body command at all, so a shadow assertion on it is vacuous.
fn advertised(source: &str) -> bool {
    let config = tcl_lexer::LexerConfig::default();
    let survey = OriginalPackageIndexSourceSurvey::capture(
        source,
        &tcl_registry::CommandRegistry::build_default(),
        config,
    );
    let command = "package ifneeded q5 1.0 {}";
    let start = u32::try_from(source.find(command).unwrap()).unwrap();
    let end = start + u32::try_from(command.len()).unwrap();
    let plan = tcl_lexer::native_script_words_in(
        survey.source_image().clone(),
        tcl_lexer::Span::new(start, end),
        config,
    )
    .unwrap();
    let [body_command] = plan.commands.as_slice() else {
        panic!("one original body command: {source}")
    };
    survey.advertisement(body_command).is_some()
}

#[test]
fn c_source_lookup_refuses_fall_through_past_condition_created_command() {
    // tclsh 8.6 / 9.0: the condition defines `::ns5::package`, so the later
    // body's `package` is the shadow and nothing is registered.
    assert!(!advertised(
        "if {[namespace eval ::ns5 {proc package args {return shadow}}; package vsatisfies 8.6 8.5]} {}\nnamespace eval ::ns5 {package ifneeded q5 1.0 {}}"
    ));
    // The same shadow created by the invocation's own condition stays
    // ahead of that invocation's body catalogue (tclsh 8.6 / 9.0 register
    // nothing).
    assert!(!advertised(
        "if {[namespace eval ::ns5 {proc package args {}}; list 1]} {namespace eval ::ns5 {package ifneeded q5 1.0 {}}}"
    ));
    // Positive control: without the shadow the same body command keeps
    // its selected source advertisement (tclsh registers `q5 1.0`).
    assert!(advertised(
        "if {[package vsatisfies 8.6 8.5]} {}\nnamespace eval ::ns5 {package ifneeded q5 1.0 {}}"
    ));
}
