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

use super::*;
use tcl_compiler::analyser::Analyser;

fn environment(
    source: &str,
    analysis: &AnalysisResult,
    cursor: usize,
) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let position = index.position_at_utf16(u32::try_from(cursor).unwrap(), source);
    scoped_env_at(
        analysis,
        source,
        position.line,
        position.character.get(),
        &index,
    )
}

#[test]
fn logical_scoped_completion_keeps_actual_parent_body_after_reporting_region_erasure() {
    let source = "# 😀\r\n::report::defstyle st {} {\r\n    to\r\n}\r\n";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    analysis.scoped_command_regions.clear();
    let cursor = source.find("    to").unwrap() + 6;
    let selected = environment(source, &analysis, cursor).unwrap();
    assert!(selected.command("top").is_some());
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let position = index.position_at_utf16(u32::try_from(cursor).unwrap(), source);
    let items = completions(
        source,
        position.line,
        position.character.get(),
        &analysis,
        None,
        None,
        analysis.resolved_profile().unwrap(),
    );
    assert!(items.iter().any(|item| item.label == "top"));
    assert!(environment(&source.replace("st", "zz"), &analysis, cursor).is_none());
}

#[test]
fn logical_scoped_vocabulary_follows_genuine_alias_parent_and_known_shadow_barriers() {
    let source = "interp alias {} style {} ::report::defstyle\nstyle st {} {\n to\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(
        environment(source, &analysis, source.find(" to").unwrap() + 3)
            .unwrap()
            .command("top")
            .is_some()
    );
    for prefix in [
        "namespace eval ::report {}; proc ::report::defstyle args {}\n",
        "rename ::report::defstyle {}\n",
    ] {
        let source = format!("{prefix}::report::defstyle st {{}} {{\n to\n}}\n");
        let mut analysis = Analyser::new().analyse(&source, "tcl");
        analysis
            .scoped_command_regions
            .push(tcl_compiler::analyser::types::ScopedBodyRegion {
                span: tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                env: &tcl_registry::scoped::REPORT_DEFSTYLE_ENV,
            });
        assert!(environment(&source, &analysis, source.find(" to").unwrap() + 3).is_none());
    }
}

#[test]
fn scoped_vocabulary_inherits_same_source_frame_and_stops_at_genuine_procedure_body() {
    for (source, expected) in [
        ("::report::defstyle st {} {if {1} {\n to\n}}", true),
        ("::report::defstyle st {} {proc p {} {\n to\n}}", false),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        assert_eq!(
            environment(source, &analysis, source.find(" to").unwrap() + 3).is_some(),
            expected
        );
    }
    let source = "puts to";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    analysis
        .scoped_command_regions
        .push(tcl_compiler::analyser::types::ScopedBodyRegion {
            span: tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            env: &tcl_registry::scoped::REPORT_DEFSTYLE_ENV,
        });
    assert!(environment(source, &analysis, source.len()).is_none());
}
