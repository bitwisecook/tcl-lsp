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

#[test]
fn shared_minifier_call_sites_keep_same_names_in_their_actual_namespace() {
    let source = "namespace eval A {proc longName {} {}; longName}\nnamespace eval B {proc longName {} {}; longName}\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let selected = analysis
        .all_procs
        .get("::A::longName")
        .expect("actual A declaration");
    let called = source.find("; longName").unwrap() + 2;
    assert_eq!(
        current_proc_call_sites(source, selected, &analysis),
        vec![Span::new(
            u32::try_from(called).unwrap(),
            u32::try_from(called + "longName".len()).unwrap(),
        )]
    );
    let detached = selected.clone();
    assert_eq!(
        current_proc_call_sites(source, &detached, &analysis),
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        current_proc_call_sites(&format!("{source} "), selected, &analysis),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn constructed_minifier_call_sites_keep_original_indirect_and_naming_guards() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // naming.minifier.original-naming-permission
    // docs/design/analysis/name-resolution-proofs/minifier-original-naming-permission.md
    // Descriptive compatibility rows do not authorise public source rewriting.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::signature_scan::types::SignatureCommandInvocation;
    let source = ":箱::méthode\n箱::méthode\nother\n:箱::méthode\n$cmd\n";
    let written = |name: &str, start: usize| {
        SignatureCommandInvocation::written(
            name.to_owned(),
            Span::new(
                u32::try_from(start).unwrap(),
                u32::try_from(start + name.len()).unwrap(),
            ),
            Some(0),
        )
    };
    let selected = written(":箱::méthode", 0);
    let wrong = written("箱::méthode", source.find("\n箱").unwrap() + 1);
    let mut resolved = written("other", source.find("other").unwrap());
    resolved.resolved_qualified_name = Some(":::箱::méthode".to_owned());
    let mut foreign = written(":箱::méthode", source.rfind(":箱::méthode").unwrap());
    foreign.resolved_qualified_name = Some("::箱::méthode".to_owned());
    let mut indirect = written("$cmd", source.find("$cmd").unwrap());
    indirect.resolved_qualified_name = Some(":::箱::méthode".to_owned());
    indirect.indirect = true;
    let expected = vec![selected.range, resolved.range];
    let other = vec![wrong.range, foreign.range];
    let indirect_span = indirect.range;
    let mut analysis = Analyser::new().analyse("", "tcl");
    analysis.command_invocations = vec![
        selected.clone(),
        selected,
        wrong,
        resolved,
        foreign,
        indirect,
    ];
    assert_eq!(
        find_proc_call_sites("méthode", ":::箱::méthode", &analysis),
        expected
    );
    assert_eq!(
        find_proc_call_sites("méthode", "::箱::méthode", &analysis),
        other
    );
    assert!(!find_proc_call_sites("méthode", ":::箱::méthode", &analysis).contains(&indirect_span));

    let source = "proc publicName {} {return VALUE}\npublicName\n";
    let original = Analyser::new().analyse(source, "tcl8.6");
    assert!(!original.allows_lexical_declaration_advice());
    let result = minify_with_analysis(
        source,
        &original,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated: true,
            abbreviations: false,
        },
    );
    assert!(
        result
            .refusals
            .contains(&MinifyRefusal::MissingAlphaRenameContract)
    );
    assert_eq!(result.symbol_map, SymbolMap::default());
    assert!(result.source.contains("proc publicName"));
    assert!(result.source.contains("publicName"));
}
