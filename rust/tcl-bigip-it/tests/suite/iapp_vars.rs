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

use tcl_bigip::apl::iapp_vars::*;
use tcl_lexer::LexerConfig;

#[test]
fn extracts_braced_and_unbraced() {
    crate::install();
    let src = "set a $::basic__addr\nset b ${::ssl__members__port}";
    let refs = extract_iapp_var_refs(src);
    let names: Vec<&str> = refs.iter().map(|r| r.apl_name.as_str()).collect();
    assert_eq!(names, vec!["basic.addr", "ssl.members.port"]);
    assert_eq!(refs[0].tcl_name, "::basic__addr");
}

#[test]
fn drops_plain_non_iapp_vars() {
    crate::install();
    // No `__` separator — not an iApp reference.
    assert_eq!(
        extract_iapp_var_refs("set x $::tcl_version"),
        [] as [tcl_bigip::apl::iapp_vars::IappVarRef; 0]
    );
}

#[test]
fn ignores_array_index() {
    crate::install();
    let refs = extract_iapp_var_refs("set v $::pool__members(0)");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].apl_name, "pool.members");
}

fn configured(source: &str) -> IappSourceReferences {
    let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
    extract_iapp_var_refs_with_context(
        source,
        environment.analyser_profile(),
        environment.default_context_registry(),
        LexerConfig::from_grammar(environment.grammar()),
    )
}

#[test]
fn original_source_components_exclude_comments_and_inert_data() {
    crate::install();
    let source = r#"# $::unseen__comment
set a {$::unseen__literal}
set b \$::unseen__escaped
set c "$::basic__addr"
"#;
    let refs = configured(source);
    assert_eq!(
        refs.references()
            .iter()
            .map(|r| r.apl_name.as_str())
            .collect::<Vec<_>>(),
        ["basic.addr"]
    );
    assert!(refs.complete_references().is_some());
    let span = refs.references()[0].range;
    assert_eq!(
        &source[span.start.offset as usize..=span.end.offset as usize],
        "$::basic__addr"
    );
}

#[test]
fn original_root_geometry_preserves_literal_colons_and_array_forms() {
    crate::install();
    let source = r"set a $:::basic__addr
set b ${:basic__addr}
set c $::ns::basic__addr
set d $::pool__members(0)
set e ${::pool__members(0)}";
    let refs = configured(source);
    assert_eq!(
        refs.references()
            .iter()
            .map(|r| r.apl_name.as_str())
            .collect::<Vec<_>>(),
        ["basic.addr", "pool.members", "pool.members"]
    );
    assert_eq!(refs.references()[0].tcl_name, ":::basic__addr");
    assert!(refs.complete_references().is_none());
}

#[test]
fn selected_body_roles_keep_positive_candidates_without_absence_coverage() {
    crate::install();
    let source = "proc helper {} {set a $::basic__addr}\nunknown_helper {$::unseen__field}";
    let refs = configured(source);
    assert_eq!(
        refs.references()
            .iter()
            .map(|r| r.apl_name.as_str())
            .collect::<Vec<_>>(),
        ["basic.addr"]
    );
    assert!(refs.complete_references().is_none());
}

#[test]
fn expression_and_command_source_keep_their_original_substitutions() {
    crate::install();
    let source = "set a [list $::basic__addr]\nexpr {$::ssl__members__port eq \"443\"}";
    let refs = configured(source);
    assert_eq!(
        refs.references()
            .iter()
            .map(|r| r.apl_name.as_str())
            .collect::<Vec<_>>(),
        ["basic.addr", "ssl.members.port"]
    );
    assert!(refs.complete_references().is_none());
}

#[test]
fn relative_convention_labels_cannot_supply_a_global_reference() {
    crate::install();
    let refs = configured("set a $local__field");
    assert_eq!(refs.references(), []);
    assert!(refs.complete_references().is_none());
}
