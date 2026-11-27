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

fn apply(source: &str, edits: &[WorkspaceTextEdit]) -> String {
    let mut result = source.to_owned();
    for edit in edits.iter().rev() {
        result.replace_range(edit.span.as_range(), &edit.new_text);
    }
    result
}

#[test]
fn authored_declaration_queue_uses_actual_frame_walk_and_refuses_foreign_source() {
    let source = "proc outer {} {proc inner {} {return VALUE}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
    let start = source.find("proc inner").unwrap();
    let end = source.rfind('}').unwrap();
    let offset = u32::try_from(start).unwrap();
    let command = walk
        .segment_nested(&source[start..end], offset)
        .into_iter()
        .next()
        .unwrap();
    let document = OriginalDeclarationDocument {
        uri: "file:///authored.tcl",
        source,
        analysis: &analysis,
    };
    let mut pending = Vec::new();
    assert!(walk.structure(source, &command).is_none());
    assert!(queue_declaration(&walk, document, &command, None, 1, &mut pending).unwrap());
    assert_eq!(pending.len(), 1);
    let (body_start, body_end, depth) = pending[0];
    assert_eq!(depth, 2);
    assert_eq!(&source[body_start..body_end], "return VALUE");
    let body = walk.segment_nested(
        &source[body_start..body_end],
        u32::try_from(body_start).unwrap(),
    );
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].texts[0], "return");
    let changed = source.replace("VALUE", "OTHER");
    assert!(crate::refactor::FrameWalk::new(&changed, &analysis).is_none());
    let foreign = Analyser::new().analyse("proc other {} {return VALUE}", "tcl8.6");
    assert!(crate::refactor::FrameWalk::new(source, &foreign).is_none());
    let mut wrong_input = analysis.clone();
    wrong_input.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
    assert!(
        crate::refactor::FrameWalk::new(source, &wrong_input).is_none_or(|other| other
            .procedure_declaration_for_rename_coverage(source, &command)
            .is_none())
    );
}

fn result_completion_command(
    source: &str,
    walk: &crate::refactor::FrameWalk<'_>,
) -> tcl_source::segment::SegmentedCommand {
    let start = source.rfind("return ").unwrap();
    let end = source.rfind('}').unwrap();
    let commands = walk.segment_nested(&source[start..end], u32::try_from(start).unwrap());
    assert_eq!(commands.len(), 1);
    commands.into_iter().next().unwrap()
}

#[test]
fn authored_result_completion_coverage_retains_nested_call_without_normal_grant() {
    let source = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
    let command = result_completion_command(source, &walk);
    let tokens = walk.tokens(source, &command);
    let receipt = walk
        .result_completion_for_rename_coverage(source, &command)
        .expect("same current authored result grammar and original argument closure");
    assert!(receipt.is_for_analysis(&analysis));
    assert_eq!(
        receipt.invocation().effective.origins,
        [
            tcl_compiler::registry_invocation::InvocationWordOrigin::ResolvedHead,
            tcl_compiler::registry_invocation::InvocationWordOrigin::Written(1),
        ]
    );
    assert_eq!(receipt.argument_regions().len(), 1);
    let span = receipt.argument_regions()[0];
    assert_eq!(&source[span.as_range()], "host_old 7");
    let document = OriginalDeclarationDocument {
        uri: "file:///return.tcl",
        source,
        analysis: &analysis,
    };
    let mut pending = Vec::new();
    assert!(
        authored_result_completion_coverage(
            &walk,
            document,
            &command,
            command.argv[0].span.start(),
            1,
            &mut pending,
        )
        .unwrap()
    );
    assert_eq!(
        pending,
        vec![(span.start() as usize, span.end() as usize, 2)]
    );
    let nested = walk.segment_nested(&source[span.as_range()], span.start());
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].texts, ["host_old", "7"]);
    let foreign = Analyser::new().analyse(source, "tcl8.6");
    assert!(!receipt.is_for_analysis(&foreign));
    assert!(
        tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            source, &foreign, &tokens,
        )
        .is_none()
    );
    let mut stale = analysis.clone();
    stale.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
    assert!(
        tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            source, &stale, &tokens,
        )
        .is_none()
    );
    assert!(
        tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
            "proc host_caller {} {return OTHER}",
            &analysis,
            &tokens,
        )
        .is_none()
    );
}

#[test]
fn authored_result_completion_coverage_refuses_unknown_shadowed_error_and_observer_inputs() {
    for source in [
        "proc host_caller {} {return [unknown_operation]}",
        "proc return args {puts OBS}; proc host_caller {} {return VALUE}",
        "proc host_caller {} {return -code error BAD}",
        "proc observer args {puts OBS}; proc host_caller {} {set value VALUE; trace add variable value read observer; return $value}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
        let command = result_completion_command(source, &walk);
        assert!(
            walk.result_completion_for_rename_coverage(source, &command)
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn authored_result_completion_never_skips_nested_name_write_or_opaque_channel() {
    for nested in ["rename host_old moved", "puts VALUE"] {
        let source = format!(
            "proc host_old {{value}} {{return $value}}\nproc host_caller {{}} {{return [{nested}]}}\n"
        );
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let walk = crate::refactor::FrameWalk::new(&source, &analysis).unwrap();
        let command = result_completion_command(&source, &walk);
        if let Some(receipt) = walk.result_completion_for_rename_coverage(&source, &command) {
            assert_eq!(receipt.argument_regions().len(), 1);
            assert_eq!(&source[receipt.argument_regions()[0].as_range()], nested);
            // A conditional Return card never changes the generic link guard.
            let expected = reject_link_obligations(receipt.invocation());
            let document = OriginalDeclarationDocument {
                uri: "file:///nested.tcl",
                source: &source,
                analysis: &analysis,
            };
            let mut pending = Vec::new();
            let result = authored_result_completion_coverage(
                &walk,
                document,
                &command,
                command.argv[0].span.start(),
                0,
                &mut pending,
            );
            match expected {
                Ok(()) => {
                    assert_eq!(result, Ok(true));
                    assert_eq!(pending.len(), 1);
                }
                Err(refusal) => {
                    assert_eq!(result, Err(refusal));
                    assert_eq!(pending, [] as [(usize, usize, u32); 0]);
                }
            }
        }
        // The nested command is judged exactly as when it is the whole
        // body: `rename host_old moved` refuses in both places, and
        // `puts VALUE`, which names no command, is accepted in both.
        let refused = |source: &str| {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let identity = OriginalDeclarationIdentity::for_procedure(
                "file:///nested.tcl",
                source,
                &analysis,
                analysis.original_procedure_declarations().next().unwrap(),
            )
            .unwrap();
            original_command_rename_edits(
                &[OriginalDeclarationDocument {
                    uri: "file:///nested.tcl",
                    source,
                    analysis: &analysis,
                }],
                &identity,
                "changed",
            )
            .is_err()
        };
        let whole_body = format!(
            "proc host_old {{value}} {{return $value}}\nproc host_caller {{}} {{{nested}}}\n"
        );
        assert_eq!(
            refused(&source),
            refused(&whole_body),
            "nested operation must not disappear from coverage: {nested}"
        );
        assert_eq!(refused(&source), nested.starts_with("rename"), "{nested}");
    }
}

#[test]
fn quiet_store_coverage_requires_actual_points_and_refuses_traced_foreign_source() {
    let source = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let walk = crate::refactor::FrameWalk::new(source, &analysis).unwrap();
    let command = walk.segment_document(source).into_iter().nth(1).unwrap();
    let tokens = walk.tokens(source, &command);
    let selected = walk
        .quiet_store_for_rename_coverage(source, &command)
        .expect("actual same-owner quiet store and complete handler");
    assert_eq!(
        selected.invocation().written_argument_roles(),
        vec![(0, tcl_registry::ArgRole::VarWrite)]
    );
    assert!(matches!(
        selected.invocation().facts.state_transitions,
        tcl_registry::StateTransitionKnowledge::UnknownInvocation
    ));
    // A plain store is the generic invocation the completed world closes
    // over, so its registry facts carry the same empty link inventory.
    assert!(reject_link_obligations(selected.invocation()).is_ok());
    let document = OriginalDeclarationDocument {
        uri: "file:///quiet.tcl",
        source,
        analysis: &analysis,
    };
    assert!(quiet_store_coverage(&walk, document, &command, 40).unwrap());
    let foreign = Analyser::new().analyse(source, "tcl8.6");
    assert!(matches!(
        reject_command_link_coverage(CommandLinkCoverage::QuietScalarStore {
            receipt: &selected,
            analysis: &foreign,
        }),
        Err(OriginalCommandRenameRefusal::StaleSource)
    ));
    assert!(
        tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
            source, &foreign, &tokens
        )
        .is_none()
    );
    assert!(
        tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
            "set host_marker OTHER",
            &analysis,
            &tokens
        )
        .is_none()
    );
    let mut wrong_input = analysis.clone();
    wrong_input.resolved_input = Analyser::new().analyse(source, "tcl9.0").resolved_input;
    assert!(
        tcl_compiler::registry_invocation::original_authored_quiet_store_for_source_coverage(
            source,
            &wrong_input,
            &tokens
        )
        .is_none()
    );
    for refused_source in [
        "proc set args {return VALUE}; set host_marker VALUE",
        "proc observer args {puts OBS}; trace add variable host_marker write observer; set host_marker VALUE",
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        "set ::auto_index(unknown_op) {source lib.tcl}; unknown_op; set host_marker VALUE",
        "puts VALUE",
    ] {
        let refused = Analyser::new().analyse(refused_source, "tcl8.6");
        let other = crate::refactor::FrameWalk::new(refused_source, &refused).unwrap();
        let last = other.segment_document(refused_source).pop().unwrap();
        assert!(
            other
                .quiet_store_for_rename_coverage(refused_source, &last)
                .is_none(),
            "{refused_source}"
        );
    }
}

#[test]
fn conditional_collision_world_requires_exact_prefix_and_unique_publication() {
    let source = "source child.tcl; proc unrelated {} {return VALUE}";
    let parent = Analyser::new().analyse(source, "tcl8.6");
    let load =
        tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap();
    let point = load.source_point_entry(&parent).unwrap();
    let child = Analyser::new()
        .analyse_with_source_point_entry("proc helper {} {return VALUE}", "tcl8.6", &point)
        .unwrap();
    let regions = load
        .conditional_continuation(&parent, &child)
        .unwrap()
        .source_regions(&parent, &child)
        .unwrap();
    let outcome = regions.normal_outcome().unwrap();
    let probe_source = "proc host_old {} {return VALUE}";
    let probe = Analyser::new().analyse(probe_source, "tcl8.6");
    let world = probe.original_completed_command_world().unwrap();
    let publication = world.declarations().next().unwrap();
    let slot = publication.slot();
    let policy = publication.policy();
    let document = OriginalDeclarationDocument {
        uri: "file:///main.tcl",
        source,
        analysis: &parent,
    };
    assert_eq!(
        occupied_with_cut(&[document], &[], None, slot, policy),
        Err(OriginalCommandRenameRefusal::UnknownWorld)
    );
    let entry = OriginalConditionalCommandWorld {
        uri: document.uri,
        outcome: &outcome,
    };
    assert_eq!(
        occupied_with_cut(&[document], &[entry], None, slot, policy),
        Ok(false)
    );
    let wrong = OriginalConditionalCommandWorld {
        uri: "file:///other.tcl",
        outcome: &outcome,
    };
    assert_eq!(
        occupied_with_cut(&[document], &[wrong], None, slot, policy),
        Err(OriginalCommandRenameRefusal::UnknownWorld)
    );
    let duplicate = [
        OriginalConditionalCommandWorld {
            uri: document.uri,
            outcome: &outcome,
        },
        OriginalConditionalCommandWorld {
            uri: document.uri,
            outcome: &outcome,
        },
    ];
    assert_eq!(
        occupied_with_cut(&[document], &duplicate, None, slot, policy),
        Err(OriginalCommandRenameRefusal::UnknownWorld)
    );
    let foreign = Analyser::new().analyse(source, "tcl8.6");
    let foreign_document = OriginalDeclarationDocument {
        analysis: &foreign,
        ..document
    };
    let entry = OriginalConditionalCommandWorld {
        uri: document.uri,
        outcome: &outcome,
    };
    assert_eq!(
        occupied_with_cut(&[foreign_document], &[entry], None, slot, policy),
        Err(OriginalCommandRenameRefusal::UnknownWorld)
    );
}

#[test]
fn original_command_rename_keeps_opaque_words_and_ignores_reporting_maps() {
    // Implementation contract: naming.source.original-jim-text-evaluation
    // docs/design/analysis/name-resolution-proofs/original-jim-text-evaluation.md
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for name in ["helper", r"p\uD800"] {
            let source = format!("proc {name} {{argument}} {{return $argument}}\n{name} value\n");
            let mut analysis = Analyser::new().analyse(&source, dialect);
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.all_classes.clear();
            let row = analysis.original_procedure_declarations().next().unwrap();
            let identity = OriginalDeclarationIdentity::for_procedure(
                "file:///owner.tcl",
                &source,
                &analysis,
                row,
            )
            .unwrap();
            let documents = [OriginalDeclarationDocument {
                uri: "file:///owner.tcl",
                source: &source,
                analysis: &analysis,
            }];
            let edits = original_command_rename_edits(&documents, &identity, "changed")
                .unwrap_or_else(|refusal| panic!("{dialect}: {name}: {refusal:?}"));
            assert_eq!(edits.len(), 2, "{dialect}: {name}");
            assert_eq!(
                apply(&source, &edits),
                "proc changed {argument} {return $argument}\nchanged value\n"
            );
            assert!(analysis.all_procs.is_empty());
        }
    }
}

#[test]
fn original_command_rename_declines_stale_duplicate_collision_and_readonly_links() {
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    let source = "proc helper {argument} {return $argument}\nhelper value\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let identity = OriginalDeclarationIdentity::for_procedure(
        "file:///owner.tcl",
        source,
        &analysis,
        analysis.original_procedure_declarations().next().unwrap(),
    )
    .unwrap();
    let owner = OriginalDeclarationDocument {
        uri: "file:///owner.tcl",
        source,
        analysis: &analysis,
    };
    assert_eq!(
        original_command_rename_edits(&[owner, owner], &identity, "changed"),
        Err(OriginalCommandRenameRefusal::AmbiguousOwner)
    );
    let equal_source_owner = OriginalDeclarationDocument {
        uri: "file:///independent.tcl",
        source,
        analysis: &analysis,
    };
    assert_eq!(
        original_command_rename_edits(&[owner, equal_source_owner], &identity, "changed"),
        Err(OriginalCommandRenameRefusal::AmbiguousOwner)
    );
    let displaced = format!("# displaced\n{source}");
    assert_eq!(
        original_command_rename_edits(
            &[OriginalDeclarationDocument {
                source: &displaced,
                ..owner
            }],
            &identity,
            "changed"
        ),
        Err(OriginalCommandRenameRefusal::StaleSource)
    );
    assert_eq!(
        original_command_rename_edits(&[owner], &identity, "return"),
        Err(OriginalCommandRenameRefusal::Collision)
    );
    assert_eq!(
        original_command_rename_edits(&[owner], &identity, "N::changed"),
        Err(OriginalCommandRenameRefusal::InvalidTail)
    );
    for source in [
        "proc helper {argument} {return $argument}\ninterp alias {} link {} helper\nlink value\n",
        "proc helper {argument} {return $argument}\nset selected helper\n$selected value\n",
        "proc helper {argument} {return $argument}\nafter idle {helper value}\n",
        "proc helper {argument} {return $argument}\ninfo body helper\n",
        "proc ::tcl::mathfunc::helper {argument} {return $argument}\nexpr {helper(1)}\n",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let identity = OriginalDeclarationIdentity::for_procedure(
            "file:///owner.tcl",
            source,
            &analysis,
            analysis.original_procedure_declarations().next().unwrap(),
        )
        .unwrap();
        let result = original_command_rename_edits(
            &[OriginalDeclarationDocument {
                uri: "file:///owner.tcl",
                source,
                analysis: &analysis,
            }],
            &identity,
            "changed",
        );
        assert!(
            result.is_err(),
            "unclosed name links must not emit a partial edit: {source}"
        );
    }
}
