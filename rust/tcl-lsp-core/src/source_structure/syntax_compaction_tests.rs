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

fn has_script(structure: &SourceStructure, source: &str, content: &str) -> bool {
    structure
        .scripts
        .iter()
        .any(|(span, _)| &source[span.as_range()] == content)
}

#[test]
fn original_syntax_regions_keep_quiet_selected_bodies_and_source_currency() {
    // Implementation contract: naming.source.closed-syntax-script-regions
    // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
    let source = "proc p {} {if {1} {  puts [list quiet]  }}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    analysis.global_scope.children.clear();
    let structure =
        SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
    assert!(has_script(
        &structure,
        source,
        "if {1} {  puts [list quiet]  }"
    ));
    assert!(has_script(&structure, source, "  puts [list quiet]  "));
    assert!(has_script(&structure, source, "list quiet"));
    assert!(
        SourceStructure::capture_for_syntax_compaction(
            &source.replace("quiet", "other"),
            &analysis,
            config,
        )
        .is_none()
    );
    assert!(
        SourceStructure::capture_for_syntax_compaction(
            source,
            &analysis,
            LexerConfig {
                expand_syntax: !config.expand_syntax,
                ..config
            },
        )
        .is_none()
    );
}

#[test]
fn original_syntax_regions_do_not_promote_conditional_roles_or_declarations() {
    // Implementation contract: naming.source.closed-syntax-script-regions
    // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let source = "set ::auto_index(unavailable) {source lib.tcl}; unavailable\nif {1} {  puts advisory  }\nputs [list lexical]\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let readonly = SourceStructure::capture(source, Some(&analysis), config).unwrap();
    assert!(has_script(&readonly, source, "  puts advisory  "));
    let closed = SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
    assert!(!has_script(&closed, source, "  puts advisory  "));
    assert!(has_script(&closed, source, "list lexical"));

    let source = "proc if {args} {}\nif {1} {  puts data  }\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let closed = SourceStructure::capture_for_syntax_compaction(
        source,
        &analysis,
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    assert!(!has_script(&closed, source, "  puts data  "));

    let source = "proc p {} \"puts \\n [list decoded]\"\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let closed = SourceStructure::capture_for_syntax_compaction(
        source,
        &analysis,
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    assert!(!has_script(&closed, source, "puts \\n [list decoded]"));
}

#[test]
fn original_syntax_regions_reject_body_roles_when_arguments_replace_the_head() {
    // Implementation contract: naming.source.closed-syntax-script-regions
    // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
    // Native proof: naming.source.argument-replacement-body-data
    // docs/design/analysis/name-resolution-proofs/argument-replacement-body-data.md
    let source = concat!(
        "if [eval {rename if saved; proc if {condition body} ",
        "{set ::custom_ran CUSTOM; set ::custom_body $body; return $body}; list 1}] ",
        "{  set ::builtin_ran BUILTIN  ; list {  VALUE  }  }",
    );
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let readonly = SourceStructure::capture(source, Some(&analysis), config).unwrap();
    assert!(!has_script(
        &readonly,
        source,
        "  set ::builtin_ran BUILTIN  ; list {  VALUE  }  "
    ));
    let closed = SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
    assert!(!has_script(
        &closed,
        source,
        "  set ::builtin_ran BUILTIN  ; list {  VALUE  }  "
    ));
    assert!(has_script(
        &closed,
        source,
        concat!(
            "eval {rename if saved; proc if {condition body} ",
            "{set ::custom_ran CUSTOM; set ::custom_body $body; return $body}; list 1}",
        )
    ));
}
