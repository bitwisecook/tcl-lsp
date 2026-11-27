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

use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

fn analyse(source: &str) -> tcl_compiler::analyser::AnalysisResult {
    use tcl_dialect::{
        DialectProfile,
        model::{DialectPoint, Release},
    };
    let profile = DialectProfile::projected_from_point(
        "jim-original-conditional-body-roles",
        &[],
        "Original conditional Jim body roles",
        DialectPoint::canonical(Release::JIM_0_79),
    )
    .intern();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn selected_logical_if_body_retains_original_jim_class_metadata() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let source = "if {1} {class Point {x 0}; class Derived Point {z 0}}";
    let result = analyse(source);
    let point = result
        .all_classes
        .values()
        .find(|class| class.name == "Point")
        .unwrap();
    let derived = result
        .all_classes
        .values()
        .find(|class| class.name == "Derived")
        .unwrap();
    assert_eq!(point.variables, ["x"]);
    assert_eq!(derived.variables, ["x", "z"]);
    assert!(point.source_name.is_none());
    assert!(derived.source_name.is_none());
}

#[test]
fn replaced_logical_handler_and_inert_data_do_not_donate_body_roles() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    for source in [
        "proc if {args} {}; if {1} {class Hidden {x 0}}",
        "set text {class Hidden {x 0}}",
    ] {
        let result = analyse(source);
        assert!(
            result
                .all_classes
                .values()
                .all(|class| class.name != "Hidden")
        );
    }
}
