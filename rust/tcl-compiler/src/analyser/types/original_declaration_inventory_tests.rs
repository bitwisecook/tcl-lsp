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
fn original_procedure_byte_slots_survive_display_collision_and_ui_removal() {
    let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
    let mut result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let declarations = result.original_procedure_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 2);
    assert_ne!(declarations[0].name().slot(), declarations[1].name().slot());
    assert_eq!(declarations[0].name_input().bytes(), b"p\xed\xa0\x80");
    assert_eq!(declarations[1].name_input().bytes(), b"p\xed\xa0\x81");
    let first = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declarations[0].name_input().clone(),
    );
    let second = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declarations[1].name_input().clone(),
    );
    result.all_procs.clear();
    result.superseded_procs.clear();
    let scope = result.original_namespace_scope_at(0).unwrap();
    let first = result.procedures_for_original_name(&first, scope);
    let second = result.procedures_for_original_name(&second, scope);
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_ne!(first[0].name_span, second[0].name_span);
    let call_offsets = [
        source.rfind(r"p\uD800").unwrap(),
        source.rfind(r"p\uD801").unwrap(),
    ];
    let calls = result
        .command_invocations
        .iter()
        .filter(|invocation| call_offsets.contains(&(invocation.range.start() as usize)))
        .filter_map(|invocation| invocation.original_name_input.as_ref())
        .collect::<Vec<_>>();
    assert!(calls.iter().any(|input| input.bytes() == b"p\xed\xa0\x80"));
    assert!(calls.iter().any(|input| input.bytes() == b"p\xed\xa0\x81"));
}

#[test]
fn original_colon_collision_keeps_both_slots_and_allocation_joins() {
    // N01: C Tcl 8.6/9.0/9.1 keep a:/p and a/:p distinct despite
    // their colliding full-name reports. Original slots own this join.
    let source = "namespace eval {a:} {proc p {} {return LEFT}}\n\
namespace eval a {proc :p {} {return RIGHT}}\n\
namespace eval {a:} {p}\n\
namespace eval a {:p}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut result = crate::analyser::Analyser::new().analyse(source, dialect);
        result.all_procs.clear();
        result.superseded_procs.clear();
        let declarations = result.original_procedure_declarations().collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2, "{dialect}");
        let slots =
            [(b"a:".as_slice(), b"p".as_slice()), (b"a", b":p")].map(|(namespace, simple)| {
                tcl_core_types::ByteCommandSlot {
                    namespace: tcl_core_types::ByteNamespacePath::from_segments([namespace]),
                    simple: tcl_core_types::NameBytes::from(simple),
                }
            });
        let offsets = [
            source.rfind("{p}").unwrap() + 1,
            source.rfind("{:p}").unwrap() + 1,
        ];
        for (index, offset) in offsets.into_iter().enumerate() {
            let call = result
                .command_invocations
                .iter()
                .find(|call| call.range.start() == u32::try_from(offset).unwrap())
                .expect("original authored call");
            assert!(call.original_lookup.is_some(), "{dialect}: {offset}");
            for (target, slot) in slots.iter().enumerate() {
                let declaration = declarations
                    .iter()
                    .find(|declaration| declaration.name().slot() == slot)
                    .expect("independently retained typed slot");
                assert_eq!(
                    crate::source_graph::invocation_targets_declaration_in(
                        source,
                        &result,
                        source,
                        &result,
                        call,
                        declaration,
                        false
                    ),
                    index == target,
                    "{dialect}: call {index}, target {target}"
                );
            }
        }
    }
}

#[test]
fn original_scope_lookup_uses_native_namespace_and_local_priority() {
    let source = r"proc p\u0000tail {} {return ROOT}
namespace eval N {proc p\u0000tail {} {return LOCAL}; p\u0000tail}";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let call = result
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.range.start()
                == u32::try_from(source.rfind(r"p\u0000tail").unwrap()).unwrap()
        })
        .unwrap();
    let input = call.original_name_input.as_ref().unwrap();
    assert_eq!(input.bytes(), b"p\xc0\x80tail");
    let matches = result.procedures_for_original_name(
        input,
        result
            .original_namespace_scope_at(call.range.start())
            .unwrap(),
    );
    assert_eq!(matches.len(), 1);
    let local_declaration = result
        .original_procedure_declarations()
        .find(|declaration| {
            declaration.declaration_site().offset
                == u32::try_from(source.rfind(r"proc p\u0000tail").unwrap()).unwrap()
        })
        .unwrap();
    assert_eq!(matches[0].name_span, local_declaration.metadata().name_span);
}

#[test]
fn invocation_transports_opaque_original_lookup_without_reporting_name() {
    let source = r"missing\uD800";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let invocation = result
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() == 0)
        .unwrap();
    let input = invocation.original_name_input.as_ref().unwrap();
    let lookup = invocation.original_lookup.as_ref().unwrap();
    assert_eq!(input.bytes(), b"missing\xed\xa0\x80");
    assert_eq!(lookup.name_input(), input);
    assert_eq!(lookup.site().offset, 0);
    assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), input.bytes());
    assert!(invocation.resolved_definition.is_none());
}

#[test]
fn computed_head_retains_readonly_input_and_positioned_lookup() {
    let source = "proc p {} {}; set cmd p; $cmd";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.rfind("$cmd").unwrap()).unwrap();
    let invocation = result
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() == offset && invocation.indirect)
        .unwrap();
    let input = invocation.original_name_input.as_ref().unwrap();
    assert!(input.original_word_key().is_none());
    assert_eq!(input.bytes(), b"p");
    let lookup = invocation.original_lookup.as_ref().unwrap();
    assert_eq!(lookup.name_input(), input);
    assert_eq!(lookup.site().offset, offset);
    assert_eq!(lookup.candidates()[0][0].simple.as_bytes(), b"p");
}

#[test]
fn substituted_heads_and_consumed_names_retain_original_lookup() {
    // naming.source.original-command-lookup
    // docs/design/analysis/name-resolution-proofs/source-original-command-lookup.md
    // tclsh 8.6.18 / 9.0.4: each `greet` below runs (or, for `info body`,
    // names) the procedure declared at offset 0.
    let declaration = "proc greet {} { return hi }\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        for (use_site, count) in [
            ("greet\ngreet\n", 2),
            ("puts [greet]\nputs [greet]\n", 2),
            ("set a [greet]\n", 1),
            ("puts [list [greet] x]\n", 1),
            ("set a \"<[greet]>\"\n", 1),
            ("info body greet\n", 1),
            ("puts [info body greet]\n", 1),
        ] {
            let source = format!("{declaration}{use_site}");
            let result = crate::analyser::Analyser::new().analyse(&source, dialect);
            let target = result
                .original_procedure_declarations()
                .find(|declaration| declaration.declaration_site().offset == 0)
                .unwrap();
            let rows = result
                .command_invocations
                .iter()
                .filter(|invocation| {
                    invocation.name == "greet"
                        && invocation.range.start() >= u32::try_from(declaration.len()).unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rows.iter()
                    .filter(
                        |row| crate::source_graph::invocation_targets_declaration_in(
                            &source, &result, &source, &result, row, target, false
                        )
                    )
                    .count(),
                count,
                "{dialect}: {use_site:?}"
            );
        }
    }
}

#[test]
fn root_unknown_command_mutation_reports_only_unknown_table_changes() {
    let mut mismatches = Vec::new();
    for (source, expected) in [
        ("eval [gets stdin]\n", true),
        ("set script [gets stdin]\neval $script\n", true),
        ("set n [gets stdin]\neval \"proc $n {} {}\"\n", true),
        ("puts hello\n", false),
        ("set x $missing\n", false),
        // `package require` runs package scripts, which can define commands
        // (tclsh 8.6.18: 101 commands before `package require http`, 104
        // after). A bare undefined head with no autoload entry raises
        // `invalid command name` under the stock `unknown` and does nothing
        // else (ruling #2427); with an `auto_index` entry it may source a
        // file that defines anything.
        ("package require http\n", true),
        ("catch {unknowncmd}\n", false),
        ("unknowncmd arg\n", false),
        (
            "set ::auto_index(unknowncmd) {source lib.tcl}\ncatch {unknowncmd}\n",
            true,
        ),
        ("rename unknown {}\ncatch {unknowncmd}\n", false),
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        if result.root_unknown_command_mutation() != Some(expected) {
            mismatches.push((source, result.root_unknown_command_mutation()));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn completed_world_follows_the_normal_completion_path_and_entered_bodies_keep_lookups() {
    // naming.source.original-command-lookup
    // docs/design/analysis/name-resolution-proofs/source-original-command-lookup.md
    // The completed command world is the world after the document runs
    // to normal completion: a command that may raise (`puts`, `list`
    // arity) only adds a path that never reaches it. `exit` or an
    // unknown command keeps it withheld.
    let declaration = "proc greet {} { return hi }\n";
    let mut mismatches = Vec::new();
    for dialect in ["tcl8.6", "tcl9.0"] {
        for (use_site, completed) in [
            ("", true),
            ("greet\ngreet\n", true),
            ("puts hi\n", true),
            ("puts [greet]\nputs [greet]\n", true),
            ("set a [list [greet]]\n", true),
            ("set a [string length x]\n", true),
            ("missing_command\n", false),
            ("puts [missing_command]\n", false),
            ("puts [undefined_cmd]\n", false),
            ("exit 0\n", false),
            // A procedure whose analysed body closes the world, and one
            // whose body runs an unknown command.
            ("proc say {} { return [greet] }\nsay\n", true),
            ("proc say {} { puts hi }\nsay\n", true),
            ("proc bad {} { missing_command }\nbad\n", false),
            // Incomplete descriptors: a re-entrant callback, a subcommand
            // the registry cannot select, and a dynamically chosen script.
            ("lsort -command greet {b a}\n", false),
            ("string $op x\n", false),
            ("eval $script\n", false),
            // The closed-builtin shortcut rests on the fresh-interpreter
            // premise: nothing lets a builtin re-enter Tcl. A program
            // that installs such a hook withholds the completed world.
            ("proc xf args {}\nchan push stdout ::xf\nputs hi\n", false),
            (
                "proc rc args {}\nset c [chan create write ::rc]\nputs $c hi\n",
                false,
            ),
            (
                "proc cb args {}\ntrace add variable x write ::cb\nset x 1\n",
                false,
            ),
            (
                "proc cb args {}\ntrace add execution puts enter ::cb\nputs hi\n",
                false,
            ),
            ("proc cb args {}\nnamespace unknown ::cb\nputs hi\n", false),
            // Retained scripts include empty timer scripts and concatenated
            // multiword forms. Their single-body ordinals are not complete
            // registration inventories. Getter/removal precision belongs to
            // the selected registry contract, not this conservative guard.
            ("after 0 {greet}\nputs hi\n", false),
            ("after 0 {}\nputs hi\n", false),
            ("after 0 greet extra\nputs hi\n", false),
            ("after idle greet extra\nputs hi\n", false),
            ("fileevent stdin readable {greet}\nputs hi\n", false),
            ("chan event stdout writable {greet}\nputs hi\n", false),
            ("set callback greet\nafter 0 $callback\nputs hi\n", false),
            ("proc unknown args {}\nputs hi\n", false),
        ] {
            let source = format!("{declaration}{use_site}");
            let result = crate::analyser::Analyser::new().analyse(&source, dialect);
            if result
                .retained_command_realm()
                .and_then(crate::realm::CommandBindingRealm::original_completed_command_world)
                .is_some()
                != completed
            {
                mismatches.push(format!(
                    "{dialect}: {use_site:?}: world expected {completed}"
                ));
            }
            // Every command of the body keeps its own original lookup
            // however often, and from wherever, the body is entered.
            let body_returns = result
                .command_invocations
                .iter()
                .filter(|invocation| {
                    invocation.name == "return"
                        && invocation.range.start() < u32::try_from(declaration.len()).unwrap()
                })
                .collect::<Vec<_>>();
            if body_returns.is_empty()
                || !body_returns
                    .iter()
                    .all(|invocation| invocation.original_lookup.is_some())
            {
                mismatches.push(format!(
                    "{dialect}: {use_site:?}: body lookup missing or lost"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn unknown_substitution_with_known_suffix_withholds_both_completed_world_views() {
    let declaration = "proc host_old {value} { return $value }\n";
    let opaque = "proc host_old {value} { return $value }\nputs [nop \"🧪\"]; puts [host_old 7]\n";
    assert_eq!(opaque.len(), 77);
    let cases = [
        ("puts [host_old 7]\n", true),
        ("puts [host_old \"🧪\"]\n", true),
        ("puts [nop ascii]\n", false),
        ("puts [nop \"🧪\"]\n", false),
        ("puts [nop ascii]; puts [host_old 7]\n", false),
        ("puts [nop \"🧪\"]; puts [host_old 7]\n", false),
        ("puts [host_old 7]; puts [nop ascii]\n", false),
        ("puts [host_old 7]; puts [nop \"🧪\"]\n", false),
    ];
    for dialect in ["tcl8.6", "tcl9.0"] {
        for (tail, completed) in cases {
            let source = format!("{declaration}{tail}");
            if tail == "puts [nop \"🧪\"]; puts [host_old 7]\n" {
                assert_eq!(source, opaque);
            }
            let result = crate::analyser::Analyser::new().analyse(&source, dialect);
            assert_eq!(
                result.original_completed_command_world().is_some(),
                completed,
                "{dialect}: {tail:?}: analysis completed world"
            );
            let realm = result
                .retained_command_realm()
                .expect("retained original realm");
            assert_eq!(
                realm.original_completed_command_world().is_some(),
                completed,
                "{dialect}: {tail:?}: realm completed world"
            );
        }
    }
}

#[test]
fn crlf_class_method_bodies_analyse_without_lexer_panics() {
    // A Windows document: every line ends in CR LF, and the method body
    // reads `$label` inside a quoted word.
    let source = concat!(
        "oo::class create Widget {\n",
        "    superclass WidgetBase\n",
        "    variable label\n",
        "    constructor {text} {\n",
        "        set label $text\n",
        "    }\n",
        "    method render {} {\n",
        "        puts \"widget $label\"\n",
        "    }\n",
        "}",
    )
    .replace('\n', "\r\n");
    for dialect in ["tcl8.6", "tcl9.0"] {
        for text in [source.clone(), format!("{source}\r\n")] {
            let result = crate::analyser::Analyser::new().analyse(&text, dialect);
            assert!(result.all_classes.contains_key("::Widget"), "{dialect}");
        }
    }
}

#[test]
fn advisory_invocation_and_unknown_body_scope_cannot_issue_source_names() {
    let invocation = SignatureCommandInvocation::written("p".to_owned(), Span::new(0, 1), Some(0));
    assert!(invocation.original_name_input.is_none());
    let mut result = crate::analyser::Analyser::new().analyse("set x 1", "tcl8.6");
    let mut child = Scope::new(ScopeKind::Method, "opaque".to_owned());
    child.body_span = Some(Span::new(0, 7));
    result.global_scope.children.push(child);
    assert!(result.original_namespace_scope_at(2).is_none());
}
