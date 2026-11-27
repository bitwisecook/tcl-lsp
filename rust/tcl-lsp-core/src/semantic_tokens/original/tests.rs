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
fn original_property_roles_refuse_foreign_producer_input() {
    let source = "oo::configurable create C {property p\\uD800}";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let offset = u32::try_from(source.find("p\\uD800").unwrap()).unwrap();
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    assert!(matches!(
        roles.names.get(&offset),
        Some((_, ArgOverride::MemberName))
    ));
    let mut foreign = analysis.clone();
    foreign.original_class_metadata = Analyser::new()
        .analyse(source, "tcl9.1")
        .original_class_metadata;
    let roles = OriginalTokenRoles::capture(source, Some(&foreign));
    assert!(!roles.names.contains_key(&offset));
}

#[test]
fn counted_initial_method_roles_keep_exact_bytes_sides_and_source_context() {
    let source = "oo::class create C\\uD800 {method m\\uD800 {} {}; method m\\uD800 {} {}; method m\\uD801 {} {}; self method make {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let [class] = analysis.original_counted_class_metadata.as_slice() else {
        panic!("counted class")
    };
    assert_eq!(
        class
            .metadata()
            .initial_method_declarations()
            .unwrap()
            .len(),
        4
    );
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    let profile = crate::environment_for_dialect("tcl8.6").analyser_profile();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        profile,
        crate::registry_for_dialect("tcl8.6"),
        None,
        Some(&analysis),
    ));
    for name in ["m\\uD800", "m\\uD801", "make"] {
        for (offset, _) in source.match_indices(name) {
            assert!(
                matches!(
                    roles.names.get(&u32::try_from(offset).unwrap()),
                    Some((_, ArgOverride::MemberName))
                ),
                "{name}"
            );
            assert!(
                kind_at(source, &tokens, offset, TokenKind::Method, MOD_DEFINITION),
                "{name}"
            );
        }
    }
    assert_eq!(source.match_indices("m\\uD800").count(), 2);
    let stale = OriginalTokenRoles::capture(&format!("#{source}"), Some(&analysis));
    assert!(stale.names.is_empty());
    let mut foreign = analysis.clone();
    foreign.original_counted_class_metadata = Analyser::new()
        .analyse(source, "tcl9.0")
        .original_counted_class_metadata;
    let roles = OriginalTokenRoles::capture(source, Some(&foreign));
    for name in ["m\\uD800", "m\\uD801", "make"] {
        let offset = u32::try_from(source.find(name).unwrap()).unwrap();
        assert!(!roles.names.contains_key(&offset), "{name}");
    }
}

#[test]
fn counted_initial_method_roles_refuse_unknown_computed_and_malformed_bodies() {
    for body in [
        "{unknown_member; method m {} {}}",
        "{error nope; method m {} {}}",
        "{return; method m {} {}}",
        "{method $name {} {}}",
        "{foreach name {a b} {method $name {} {}}}",
        "{method m {} \"unterminated}",
    ] {
        let source = format!("oo::class create C\\uD800 {body}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let [class] = analysis.original_counted_class_metadata.as_slice() else {
            panic!("{source}")
        };
        assert!(
            class.metadata().initial_method_declarations().is_none(),
            "{source}"
        );
        let roles = OriginalTokenRoles::capture(&source, Some(&analysis));
        if let Some(name) = source.find("method m ") {
            let offset = u32::try_from(name + "method ".len()).unwrap();
            assert!(!roles.names.contains_key(&offset), "{source}");
        }
    }
}

#[test]
fn counted_nonempty_class_token_roles_colour_source_method_names() {
    let source = "oo::class create C\\uD800 {method m\\uD801 {} {return body}}\nC\\uD800 new\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(analysis.original_counted_class_metadata.len(), 1);
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    let head = u32::try_from(source.rfind("C\\uD800 new").unwrap()).unwrap();
    assert!(roles.is_class_head(head));
    let member = u32::try_from(source.find("m\\uD801").unwrap()).unwrap();
    assert!(matches!(
        roles.names.get(&member),
        Some((_, ArgOverride::MemberName))
    ));
    let profile = crate::environment_for_dialect("tcl8.6").analyser_profile();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        profile,
        crate::registry_for_dialect("tcl8.6"),
        None,
        Some(&analysis),
    ));
    assert!(kind_at(
        source,
        &tokens,
        usize::try_from(head).unwrap(),
        TokenKind::Class,
        0
    ));
    assert!(kind_at(
        source,
        &tokens,
        usize::try_from(member).unwrap(),
        TokenKind::Method,
        MOD_DEFINITION
    ));
}

#[test]
fn counted_class_head_rejects_swapped_reference_and_duplicate_record() {
    let source =
        "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let first_offset = u32::try_from(source.find("C\\uD800 new").unwrap()).unwrap();
    let second_offset = u32::try_from(source.find("C\\uD801 new").unwrap()).unwrap();
    let mut swapped = analysis.clone();
    let second_reference = swapped
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.range.start() == second_offset && invocation.lookup.is_execution_site()
        })
        .unwrap()
        .resolved_command_reference
        .clone();
    let first = swapped
        .command_invocations
        .iter_mut()
        .find(|invocation| {
            invocation.range.start() == first_offset && invocation.lookup.is_execution_site()
        })
        .unwrap();
    first.resolved_command_reference = second_reference;
    assert!(!OriginalTokenRoles::capture(source, Some(&swapped)).is_class_head(first_offset));
    let mut duplicate = analysis.clone();
    duplicate
        .original_counted_class_metadata
        .push(duplicate.original_counted_class_metadata[0].clone());
    assert!(!OriginalTokenRoles::capture(source, Some(&duplicate)).is_class_head(first_offset));
    let mut foreign = analysis.clone();
    foreign.original_counted_class_metadata = Analyser::new()
        .analyse(source, "tcl9.0")
        .original_counted_class_metadata;
    let roles = OriginalTokenRoles::capture(source, Some(&foreign));
    assert!(!roles.is_class_head(first_offset));
    let name = u32::try_from(source.find("C\\uD800 {}").unwrap()).unwrap();
    assert!(!roles.names.contains_key(&name));
}

#[test]
fn counted_empty_classes_colour_only_current_authenticated_heads() {
    let source =
        "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800 new\nC\\uD801 new\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(analysis.original_counted_class_metadata.len(), 2);
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    let profile = crate::environment_for_dialect("tcl8.6").analyser_profile();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        profile,
        crate::registry_for_dialect("tcl8.6"),
        None,
        Some(&analysis),
    ));
    for (name, call) in [
        ("C\\uD800 {}", "C\\uD800 new"),
        ("C\\uD801 {}", "C\\uD801 new"),
    ] {
        let declaration = u32::try_from(source.find(name).unwrap()).unwrap();
        let head = u32::try_from(source.find(call).unwrap()).unwrap();
        assert!(matches!(
            roles.names.get(&declaration),
            Some((_, ArgOverride::ClassNameDef))
        ));
        assert!(roles.is_class_head(head));
        assert!(kind_at(
            source,
            &tokens,
            usize::try_from(declaration).unwrap(),
            TokenKind::Class,
            MOD_DEFINITION
        ));
        assert!(kind_at(
            source,
            &tokens,
            usize::try_from(head).unwrap(),
            TokenKind::Class,
            0
        ));
    }
    let stale = OriginalTokenRoles::capture(&format!("#{source}"), Some(&analysis));
    assert!(!stale.current);
    assert!(stale.class_heads.is_empty());
    let foreign = Analyser::new().analyse(source, "jim");
    assert_eq!(
        foreign.original_counted_class_metadata,
        [] as [tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::types::OriginalCountedClass,
        >; 0]
    );
}

#[test]
fn counted_empty_class_head_refuses_shadow_and_unresolved_observer() {
    for source in [
        "oo::class create C\\uD800 {}; rename C\\uD800 {}; C\\uD800 new",
        "oo::class create C\\uD800 {}; rename C\\uD800 {}; proc C\\uD800 args {}; C\\uD800 new",
        "oo::class create C\\uD800 {}; proc change args {rename C\\uD800 {}}; trace add execution C\\uD800 enter change; C\\uD800 new",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            analysis.original_counted_class_metadata.len(),
            1,
            "{source}"
        );
        let head = u32::try_from(source.rfind("C\\uD800 new").unwrap()).unwrap();
        let roles = OriginalTokenRoles::capture(source, Some(&analysis));
        assert!(!roles.is_class_head(head), "{source}");
    }
}
use crate::semantic_tokens::full_with_cu_and_analysis;
use crate::semantic_tokens::legend::MOD_DEFINITION;
use crate::semantic_tokens::tests::decode_semantic;
use tcl_compiler::analyser::Analyser;

fn kind_at(
    source: &str,
    tokens: &[(u32, u32, u32, u32, u32)],
    at: usize,
    wanted: TokenKind,
    modifier: u32,
) -> bool {
    let position =
        tcl_lexer::LineIndex::new(source).position_at_utf16(u32::try_from(at).unwrap(), source);
    tokens.iter().any(|&(line, character, _, kind, modifiers)| {
        line == position.line
            && character == position.character.get()
            && kind == wanted as u32
            && modifiers == modifier
    })
}

#[test]
fn original_vendor_token_roles_require_supported_header_units_and_current_source() {
    // Implementation contract: naming.vendor.original-source-declaration-consumers
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
    let source = "proc helper {} {}\nproc p\\uD800 {} {}\nwhen HTTP_REQUEST {}\n";
    let mut analysis = Analyser::new().analyse(source, "f5-irules");
    analysis.all_procs.clear();
    analysis.all_defined_symbols.clear();
    analysis.global_scope.procs.clear();
    analysis.global_scope.defined_symbols.clear();
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    assert!(roles.current);
    let helper = u32::try_from(source.find("helper").unwrap()).unwrap();
    assert!(matches!(
        roles.names.get(&helper),
        Some((_, ArgOverride::ProcNameDef))
    ));
    let event = u32::try_from(source.find("HTTP_REQUEST").unwrap()).unwrap();
    assert!(matches!(
        roles.names.get(&event),
        Some((_, ArgOverride::Kind(TokenKind::Event)))
    ));
    let unsupported = u32::try_from(source.find(r"p\uD800").unwrap()).unwrap();
    assert!(!roles.names.contains_key(&unsupported));
    assert!(
        analysis
            .original_vendor_procedure_declarations()
            .any(|row| row.name_input().span().start() == unsupported
                && row.name_input().literal_units(row.purpose()).is_none())
    );
    let stale = OriginalTokenRoles::capture(&format!("# displaced\n{source}"), Some(&analysis));
    assert!(!stale.current && stale.names.is_empty());
}

#[test]
fn original_oo_roles_keep_opaque_declarations_and_actual_calls_without_ui_maps() {
    let source = "oo::class create C\\uD800 {method m\\uD800 {} {return A}}\noo::class create C\\uD801 {method m\\uD801 {} {return B}}\nC\\uD800 create a\nC\\uD801 create b\na m\\uD800\nb m\\uD801\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(analysis.original_class_declarations().count(), 2);
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    analysis.instance_classes.clear();
    analysis.created_instance_commands.clear();
    let profile = crate::environment_for_dialect("tcl8.6").analyser_profile();
    let tokens = decode_semantic(&full_with_cu_and_analysis(
        source,
        profile,
        crate::registry_for_dialect("tcl8.6"),
        None,
        Some(&analysis),
    ));
    for written in ["C\\uD800", "C\\uD801"] {
        assert!(kind_at(
            source,
            &tokens,
            source.find(written).unwrap(),
            TokenKind::Class,
            MOD_DEFINITION
        ));
    }
    for written in ["m\\uD800", "m\\uD801"] {
        assert!(kind_at(
            source,
            &tokens,
            source.find(written).unwrap(),
            TokenKind::Method,
            MOD_DEFINITION
        ));
        assert!(kind_at(
            source,
            &tokens,
            source.rfind(written).unwrap(),
            TokenKind::Method,
            0
        ));
    }
}

#[test]
fn original_oo_roles_do_not_use_final_members_for_an_earlier_or_stale_call() {
    let source = "oo::class create C {}; C create object\nobject later\noo::define C {method later {} {return OK}}\nobject later\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    assert!(roles.current);
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    );
    let first = commands
        .iter()
        .find(|command| command.name() == "object")
        .unwrap();
    assert!(crate::receiver_identity::method_at_command(&analysis, source, first).is_none());
    analysis.all_classes.clear();
    let stale = OriginalTokenRoles::capture(&format!("{source} "), Some(&analysis));
    assert!(!stale.current);
    assert!(stale.names.is_empty());
    assert!(stale.class_heads.is_empty());
}

#[test]
fn original_property_roles_keep_counted_declarations_separate_from_accessors() {
    let source =
        r"oo::configurable create C {property p\uD800 -kind readable p\uD801 -kind writable}";
    let mut analysis = Analyser::new().analyse(source, "tcl9.1");
    assert_eq!(
        analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .original_properties
            .declarations()
            .count(),
        2
    );
    analysis.all_classes.clear();
    analysis.global_scope.classes.clear();
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    assert!(roles.current);
    for written in [r"p\uD800", r"p\uD801"] {
        let start = u32::try_from(source.find(written).unwrap()).unwrap();
        let (span, kind) = roles.names.get(&start).unwrap();
        assert_eq!(
            span,
            &Span::new(start, start + u32::try_from(written.len()).unwrap())
        );
        assert!(matches!(kind, ArgOverride::MemberName));
    }
    let stale = OriginalTokenRoles::capture(&format!("{source} "), Some(&analysis));
    assert!(!stale.current);
    assert!(stale.names.is_empty());
}
