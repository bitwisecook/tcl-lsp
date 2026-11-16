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
use crate::original_command_rename::{
    original_command_rename_edits_with_conditional_worlds,
    original_command_rename_edits_with_source_views,
};
use crate::original_declaration::{OriginalDeclarationRole, OriginalSourceViewPublication};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::command_binding::{
    OriginalSourceConditionalOutcome, OriginalSourceConditionalRegion,
};

struct Fixture {
    source: &'static str,
    child_source: &'static str,
    prefix: AnalysisResult,
    child: AnalysisResult,
    ordinary_child: AnalysisResult,
    suffix: AnalysisResult,
    region: OriginalSourceConditionalRegion,
    region_count: usize,
    outcome: OriginalSourceConditionalOutcome,
    peer: AnalysisResult,
}
const PEER: &str = "proc host_old {} {return VALUE}; host_old";
impl Fixture {
    fn new(source: &'static str, child_source: &'static str) -> Self {
        let prefix = Analyser::new().analyse(source, "tcl8.6");
        let source_offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source,
                &prefix,
                source_offset,
            )
            .unwrap();
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&prefix, &child)
            .unwrap()
            .source_regions(&prefix, &child)
            .unwrap();
        let region = regions.regions()[0].clone();
        let suffix = Analyser::new()
            .analyse_source_region(&region, &prefix, &child)
            .unwrap();
        Self {
            source,
            child_source,
            ordinary_child: Analyser::new().analyse(child_source, "tcl8.6"),
            outcome: regions.normal_outcome().unwrap(),
            region_count: regions.regions().len(),
            region,
            prefix,
            child,
            suffix,
            peer: Analyser::new().analyse(PEER, "tcl8.6"),
        }
    }

    fn with_suffix_body_layout(
        &self,
        check: impl FnOnce(
            &tcl_compiler::registry_invocation::OriginalSourceCutBodyLayout<'_>,
            &tcl_compiler::ir::CommandTokens,
        ),
    ) {
        self.with_body_layout(None, check);
    }

    fn with_body_layout(
        &self,
        child_command: Option<usize>,
        check: impl FnOnce(
            &tcl_compiler::registry_invocation::OriginalSourceCutBodyLayout<'_>,
            &tcl_compiler::ir::CommandTokens,
        ),
    ) {
        let survey = self
            .outcome
            .single_source_cut_survey(&self.prefix, &[&self.child, &self.suffix])
            .unwrap();
        let (source, analysis) = if child_command.is_some() {
            (self.child_source, &self.child)
        } else {
            (self.source, &self.suffix)
        };
        let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
        let commands = if child_command.is_some() {
            walk.segment_document(source)
        } else {
            Vec::new()
        };
        let command = child_command.map_or(&survey.suffix_commands()[0], |index| &commands[index]);
        let declaration = tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, analysis, command, &survey,
            ).expect("actual cut-owned declaration");
        let span = declaration.body().content_span();
        let layout = declaration.into_body_layout(&survey).unwrap();
        let commands = walk.segment_nested(&source[span.as_range()], span.start());
        let mut tokens = walk.tokens(source, &commands[0]);
        layout
            .stamp_original_tokens(source, analysis, &mut tokens)
            .unwrap();
        check(&layout, &tokens);
    }

    fn documents(&self) -> Vec<OriginalDeclarationDocument<'_>> {
        vec![
            OriginalDeclarationDocument {
                uri: "file:///main.tcl",
                source: self.source,
                analysis: &self.prefix,
            },
            OriginalDeclarationDocument {
                uri: "file:///child.tcl",
                source: self.child_source,
                analysis: &self.ordinary_child,
            },
            OriginalDeclarationDocument {
                uri: "file:///peer.tcl",
                source: PEER,
                analysis: &self.peer,
            },
        ]
    }
    fn identity(&self) -> OriginalDeclarationIdentity {
        crate::original_declaration::declarations("file:///peer.tcl", PEER, &self.peer)
            .unwrap()
            .into_iter()
            .find(|row| row.role() == OriginalDeclarationRole::Procedure)
            .unwrap()
    }
    fn with_views<T>(&self, run: impl FnOnce(&[OriginalSourceViewDocument<'_>]) -> T) -> T {
        let root = OriginalSourceViewPublication::retain(1_u8);
        let child = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let child_ancestors = [root.clone()];
        let suffix_ancestors = [child.clone(), root.clone()];
        run(&[
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source: self.source,
                    analysis: &self.prefix,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &child,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: self.child_source,
                    analysis: &self.child,
                },
                region: None,
                ancestors: &child_ancestors,
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source: self.source,
                    analysis: &self.suffix,
                },
                region: Some(&self.region),
                ancestors: &suffix_ancestors,
            },
        ])
    }
}

fn assert_malformed_source_cut_body_refused() {
    let fixture = Fixture::new("source child.tcl; set done OK", "proc helper {} {[}");
    let documents = fixture.documents();
    let identity = fixture.identity();
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
            let proof = SourceCoverage::capture(&documents, &identity, &worlds, views).unwrap();
            let walk = crate::refactor::FrameWalk::new(fixture.child_source, &fixture.child).unwrap();
            let commands = walk.segment_document(fixture.child_source);
            assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                fixture.child_source, &fixture.child, &commands[0], &proof.cuts[0].survey,
            ).is_none());
            assert!(original_command_rename_edits_with_source_views(
                &documents, &identity, "host_new", &worlds, views,
            ).is_err());
        });
}
fn assert_source_cut_body_layout(
    source: &str,
    analysis: &AnalysisResult,
    ordinary: &AnalysisResult,
    command: &SegmentedCommand,
    survey: &OriginalSingleSourceCutSurvey<'_>,
    receipt: tcl_compiler::registry_invocation::OriginalSourceCutProcedureDeclaration<'_>,
) {
    let content = receipt.body().content_span();
    let layout = receipt
        .into_body_layout(survey)
        .expect("same cut conditional body owner");
    let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
    let commands = walk.segment_nested(&source[content.as_range()], content.start());
    let mut stronger = walk.tokens(source, &commands[0]);
    let retained = stronger.source_binding.as_mut().unwrap();
    retained.may_use_live_dispatch = true;
    retained.evaluated_argument_values = vec![Some("retained argument fact".to_owned())].into();
    let retained = stronger.source_binding.clone();
    assert!(
        layout
            .stamp_original_tokens(source, analysis, &mut stronger)
            .is_some()
    );
    assert_eq!(stronger.source_binding, retained);
    let mut tokens = walk.tokens(source, &commands[0]);
    let before = tokens.source_binding.as_ref().unwrap().clone();
    let had_original_head = before.original_head_name_input(&tokens).is_some();
    assert!(
        layout
            .stamp_original_tokens(source, analysis, &mut tokens)
            .is_some()
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    if had_original_head {
        assert_eq!(
            binding, &before,
            "an existing original head keeps its entire binding"
        );
    } else {
        assert!(binding.unknown);
        assert_eq!(
            binding.targets,
            [] as [tcl_compiler::command_binding::SourceCommandTarget; 0]
        );
        assert!(!binding.unobserved_native_dispatch());
    }
    let input = binding
        .original_head_name_input(&tokens)
        .expect("same complete body vector, no entered invocation");
    assert!(binding.original_command_lookup(&tokens, &input).is_some());
    let mut outside = walk.tokens(source, command);
    assert!(
        layout
            .stamp_original_tokens(source, analysis, &mut outside)
            .is_none()
    );
    let mut stale = tokens.clone();
    assert!(
        layout
            .stamp_original_tokens(&format!("{source}\n# changed"), analysis, &mut stale)
            .is_none()
    );
    let mut foreign = tokens.clone();
    assert!(
        layout
            .stamp_original_tokens(source, ordinary, &mut foreign)
            .is_none()
    );
    let equal_foreign = analysis.clone();
    let mut foreign = tokens;
    assert!(
        layout
            .stamp_original_tokens(source, &equal_foreign, &mut foreign)
            .is_none()
    );
}

fn assert_source_cut_declaration_geometry(
    fixture: &Fixture,
    survey: &OriginalSingleSourceCutSurvey<'_>,
) {
    assert_malformed_source_cut_body_refused();
    let child_walk = crate::refactor::FrameWalk::new(fixture.child_source, &fixture.child).unwrap();
    let child_commands = child_walk.segment_document(fixture.child_source);
    for (source, analysis, command, ordinary) in [
        (
            fixture.child_source,
            &fixture.child,
            &child_commands[0],
            &fixture.ordinary_child,
        ),
        (
            fixture.source,
            &fixture.suffix,
            &survey.suffix_commands()[0],
            &fixture.prefix,
        ),
    ] {
        let receipt = tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, analysis, command, survey,
            ).expect("same actual source-cut declaration");
        assert!(receipt.is_for_analysis(analysis));
        assert!(!receipt.is_for_analysis(ordinary));
        assert_eq!(
            receipt.invocation().written_argument_roles(),
            vec![
                (0, tcl_registry::ArgRole::Name),
                (1, tcl_registry::ArgRole::ParamList),
                (2, tcl_registry::ArgRole::Body),
            ]
        );
        assert_eq!(
            &source[receipt.body().content_span().as_range()],
            "return VALUE"
        );
        let walk = crate::refactor::FrameWalk::new(source, analysis).unwrap();
        assert!(
            walk.procedure_declaration_for_rename_coverage(source, command)
                .is_none()
        );
        assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, ordinary, command, survey,
            ).is_none());
        assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                &format!("# foreign whole source\n{source}"), analysis, command, survey,
            ).is_none());
        let mut foreign = analysis.clone();
        foreign.body_lexer_config.as_mut().unwrap().strict_quoting =
            !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
                source, &foreign, command, survey,
            ).is_none());
        assert_source_cut_body_layout(source, analysis, ordinary, command, survey, receipt);
    }
    let prefix_walk = crate::refactor::FrameWalk::new(fixture.source, &fixture.prefix).unwrap();
    let prefix_commands = prefix_walk.segment_document(fixture.source);
    assert!(
        tcl_compiler::registry_invocation::original_source_cut_procedure_declaration_for_coverage(
            fixture.source,
            &fixture.prefix,
            &prefix_commands[0],
            survey,
        )
        .is_none()
    );
}

fn cut_rename_edits(fixture: &Fixture) -> Result<usize, OriginalCommandRenameRefusal> {
    let documents = fixture.documents();
    let identity = fixture.identity();
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        original_command_rename_edits_with_source_views(
            &documents, &identity, "host_new", &worlds, views,
        )
        .map(|edits| edits.len())
    })
}

/// tclsh 8.6/9.0: each option form changes the completion code or level,
/// `return a b` reads `a b` as an option, and `uplevel`/`eval` move the
/// return into another script; none is modelled for a cut procedure body.
#[test]
fn single_cut_body_return_option_forms_stay_refused() {
    let child = "proc helper {} {return VALUE}";
    for body in [
        "return -code error E",
        "return -code break",
        "return -code continue",
        "return -code return R",
        "return -code 7 N",
        "return -level 2 L",
        "return -options {-code error} O",
        "return -errorcode X E",
        "return a b",
        "uplevel 1 {return U}",
        "eval {return EV}",
    ] {
        let source: &'static str =
            Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
        assert_eq!(
            cut_rename_edits(&Fixture::new(source, child)),
            Err(OriginalCommandRenameRefusal::IncompleteCoverage),
            "{body}"
        );
    }
}

/// tclsh 8.6/9.0: a top-level `return` in the sourced file ends `source`
/// with its value and the issuing script continues; that propagation into
/// the issuing point is unmodelled, so no rename world is issued from it.
#[test]
fn single_cut_child_top_level_return_stays_refused() {
    let source = "source child.tcl; proc run {} {return VALUE}";
    let child_source = "proc helper {} {return VALUE}\nreturn done";
    let prefix = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
    let outcome =
        tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
            source, &prefix, offset,
        )
        .and_then(|load| {
            let point = load.source_point_entry(&prefix)?;
            let child =
                Analyser::new().analyse_with_source_point_entry(child_source, "tcl8.6", &point)?;
            load.conditional_continuation(&prefix, &child)?
                .source_regions(&prefix, &child)?
                .normal_outcome()
        });
    if outcome.is_some() {
        assert_eq!(
            cut_rename_edits(&Fixture::new(source, child_source)),
            Err(OriginalCommandRenameRefusal::IncompleteCoverage)
        );
    }
}

#[test]
fn single_cut_source_coverage_surveys_actual_child_and_issued_suffix() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return VALUE}",
    );
    let documents = fixture.documents();
    let identity = fixture.identity();
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    assert!(
        original_command_rename_edits_with_conditional_worlds(
            &documents, &identity, "host_new", &worlds
        )
        .is_err()
    );
    fixture.with_views(|views| {
        let proof = SourceCoverage::capture(&documents, &identity, &worlds, views).unwrap();
        assert_eq!(proof.cuts.len(), 1);
        assert_eq!(proof.cuts[0].survey.prefix_commands().len(), 1);
        assert_eq!(proof.cuts[0].survey.suffix_commands().len(), 1);
        assert_source_cut_declaration_geometry(&fixture, &proof.cuts[0].survey);
        let edits = original_command_rename_edits_with_source_views(
            &documents, &identity, "host_new", &worlds, views,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{error:?}: {:?}",
                super::super::take_missing_rename_world_trace()
            )
        });
        assert_eq!(edits.len(), 2);
        assert!(
            edits
                .iter()
                .all(|edit| edit.uri == "file:///peer.tcl" && edit.new_text == "host_new")
        );
        assert!(!std::ptr::eq(
            proof.cuts[0].child.analysis,
            documents[1].analysis
        ));
        assert!(std::ptr::eq(documents[2].analysis, &raw const fixture.peer));
    });
}
fn assert_foreign_source_membership_refused(
    fixture: &Fixture,
    documents: &[OriginalDeclarationDocument<'_>],
    identity: &OriginalDeclarationIdentity,
    worlds: &[OriginalConditionalCommandWorld<'_>],
    views: &[OriginalSourceViewDocument<'_>],
) {
    let duplicate = [documents[0], documents[1], documents[1], documents[2]];
    assert!(SourceCoverage::capture(&duplicate, identity, worlds, views).is_err());
    let foreign = Analyser::new().analyse(fixture.source, "tcl8.6");
    let stale = [
        OriginalDeclarationDocument {
            analysis: &foreign,
            ..documents[0]
        },
        documents[1],
        documents[2],
    ];
    assert!(SourceCoverage::capture(&stale, identity, worlds, views).is_err());
    let foreign_input = Analyser::new().analyse(fixture.source, "tcl9.0");
    let wrong_config = [
        OriginalDeclarationDocument {
            analysis: &foreign_input,
            ..documents[0]
        },
        documents[1],
        documents[2],
    ];
    assert!(SourceCoverage::capture(&wrong_config, identity, worlds, views).is_err());
    let changed = [
        documents[0],
        OriginalDeclarationDocument {
            source: "changed",
            ..documents[1]
        },
        documents[2],
    ];
    assert!(SourceCoverage::capture(&changed, identity, worlds, views).is_err());
}
#[test]
fn source_coverage_refuses_missing_child_reference_and_foreign_membership() {
    let fixture = Fixture::new(
        "proc target {} {return VALUE}; source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {target}",
    );
    let documents = fixture.documents();
    let identity = crate::original_declaration::declarations(
        "file:///main.tcl",
        fixture.source,
        &fixture.prefix,
    )
    .unwrap()
    .into_iter()
    .find(|row| row.role() == OriginalDeclarationRole::Procedure)
    .unwrap();
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        assert!(SourceCoverage::capture(&documents, &identity, &worlds, views).is_ok());
        if let Ok(edits) = original_command_rename_edits_with_source_views(
            &documents,
            &identity,
            "target_new",
            &worlds,
            views,
        ) {
            assert!(
                edits
                    .iter()
                    .any(|edit| edit.uri == "file:///child.tcl" && edit.new_text == "target_new")
            );
        }
        for omitted in [0, 1, 2] {
            let retained = views
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != omitted)
                .map(|(_, view)| OriginalSourceViewDocument {
                    publication: view.publication,
                    document: view.document,
                    region: view.region,
                    ancestors: view.ancestors,
                })
                .collect::<Vec<_>>();
            assert!(SourceCoverage::capture(&documents, &identity, &worlds, &retained).is_err());
            assert!(
                original_command_rename_edits_with_source_views(
                    &documents,
                    &identity,
                    "target_new",
                    &worlds,
                    &retained
                )
                .is_err()
            );
        }
        let without_child = [documents[0], documents[2]];
        assert!(SourceCoverage::capture(&without_child, &identity, &worlds, views).is_err());
        assert!(
            original_command_rename_edits_with_source_views(
                &without_child,
                &identity,
                "target_new",
                &worlds,
                views
            )
            .is_err()
        );
        assert_foreign_source_membership_refused(&fixture, &documents, &identity, &worlds, views);
    });
}
#[test]
fn source_coverage_keeps_selected_owner_and_opaque_body_refusals() {
    for child_source in [
        "proc helper {} {return VALUE}",
        "proc helper {} {puts opaque}",
    ] {
        let fixture = Fixture::new("source child.tcl; proc run {} {return VALUE}", child_source);
        let documents = fixture.documents();
        let identity = fixture.identity();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        let child_identity = crate::original_declaration::declarations(
            "file:///child.tcl",
            child_source,
            &fixture.ordinary_child,
        )
        .unwrap()
        .into_iter()
        .find(|row| row.role() == OriginalDeclarationRole::Procedure)
        .unwrap();
        fixture.with_views(|views| {
            assert!(SourceCoverage::capture(&documents, &child_identity, &worlds, views).is_err());
            if child_source.contains("puts") {
                assert!(
                    original_command_rename_edits_with_source_views(
                        &documents, &identity, "host_new", &worlds, views
                    )
                    .is_err()
                );
            }
        });
    }
}

fn assert_logical_source_rename_issuer_unavailable(namespace_source: &str, child_source: &str) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    assert!(input.has_logical_source_name_context());
    for source in [
        namespace_source,
        "source child.tcl; proc run {} {return VALUE}",
    ] {
        let parent = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        assert_eq!(parent.resolved_input.as_ref(), Some(&input));
        let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &parent, offset,
            )
            .expect("actual Logical project-source load card");
        assert!(load.matches_source(&SourceImage::document(source), &input));
        let point = load
            .source_point_entry(&parent)
            .expect("actual Logical source point");
        let child = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse_with_source_point_entry(child_source, profile.name, &point)
            .expect("genuine Logical child under its actual point");
        assert!(point.matches_parent_analysis(&parent));
        assert!(point.matches_child_analysis(&child));
        assert!(
            child
                .resolved_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        assert!(child.original_completed_command_world().is_none());
        assert!(
            load.conditional_continuation(&parent, &child).is_none(),
            "current Logical source issuer has no completed continuation"
        );
    }
}

fn native_namespace_fixture(source: &'static str, child_source: &'static str) -> Fixture {
    let fixture = Fixture::new(source, child_source);
    let offset = u32::try_from(source.find("source child").unwrap()).unwrap();
    let load =
        tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
            source,
            &fixture.prefix,
            offset,
        )
        .unwrap();
    let point = load.source_point_entry(&fixture.prefix).unwrap();
    assert!(point.matches_parent_analysis(&fixture.prefix));
    assert!(point.matches_child_analysis(&fixture.child));
    assert!(fixture.child.original_completed_command_world().is_none());
    assert!(
        !fixture
            .child
            .resolved_input
            .as_ref()
            .unwrap()
            .has_logical_source_name_context()
    );
    fixture
}

#[test]
fn source_coverage_never_omits_extra_cuts_or_namespace_frames() {
    for (source, child_source) in [
        (
            "source child.tcl; proc run {} {return VALUE}",
            "proc helper {} {source hidden.tcl}",
        ),
        (
            "namespace eval :: {source child.tcl}; proc run {} {return VALUE}",
            "proc helper {} {return VALUE}",
        ),
    ] {
        let fixture = if source.starts_with("namespace") {
            assert_logical_source_rename_issuer_unavailable(source, child_source);
            native_namespace_fixture(source, child_source)
        } else {
            Fixture::new(source, child_source)
        };
        if source.starts_with("namespace") {
            assert!(fixture.region_count > 1);
        }
        let documents = fixture.documents();
        let identity = fixture.identity();
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(SourceCoverage::capture(&documents, &identity, &worlds, views).is_err());
            assert!(
                original_command_rename_edits_with_source_views(
                    &documents, &identity, "host_new", &worlds, views
                )
                .is_err()
            );
        });
    }
}

fn inherited_child_identity(fixture: &Fixture) -> OriginalDeclarationIdentity {
    let std::ops::ControlFlow::Break(Some(identity)) = super::super::select_in_documents(
        &[OriginalDeclarationDocument {
            uri: "file:///child.tcl",
            source: fixture.child_source,
            analysis: &fixture.ordinary_child,
        }],
        "file:///child.tcl",
        5,
    ) else {
        panic!("actual physical child declaration")
    };
    identity
}

#[test]
fn inherited_selection_keeps_both_physical_and_child_declaration_guards() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return VALUE}; proc caller {} {helper}",
    );
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        assert!(
            original_command_rename_edits_with_source_views(
                &documents,
                &identity,
                "updated_helper",
                &worlds,
                views,
            )
            .is_err()
        );
        let edits = super::super::original_command_rename_edits_with_selected_source_view(
            &documents,
            &identity,
            "updated_helper",
            "file:///child.tcl",
            5,
            &worlds,
            views,
        )
        .expect("genuine inherited declaration plus independent physical edit agreement");
        assert_eq!(edits.len(), 2);
        assert!(
            edits
                .iter()
                .all(|edit| edit.uri == "file:///child.tcl" && edit.new_text == "updated_helper")
        );
        assert_eq!(
            super::super::original_command_prepare_span_in_with_selected_source_view(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap(),
            identity.span()
        );
        let reference = u32::try_from(fixture.child_source.rfind("helper").unwrap()).unwrap();
        assert!(
            super::super::original_command_rename_edits_with_selected_source_view(
                &documents,
                &identity,
                "updated_helper",
                "file:///child.tcl",
                reference,
                &worlds,
                views,
            )
            .is_err(),
            "reference-origin selection cannot retarget an inherited declaration"
        );
    });
}

#[test]
fn cut_body_result_coverage_keeps_inherited_purpose_and_selects_registered_result() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return ROOT}",
        "proc helper {} {return VALUE}",
    );
    fixture.with_body_layout(Some(0), |layout, tokens| {
            let before = tokens.source_binding.clone();
            assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
                fixture.child_source, &fixture.child, tokens,
            ).is_none());
            let coverage = layout.result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                .expect("same captured child row and actual registered result contract");
            assert!(coverage.is_for_analysis(&fixture.child));
            assert_eq!(coverage.invocation().written_argument_roles(), [(0, tcl_registry::ArgRole::Result)]);
            assert_eq!(coverage.invocation().facts.native_result,
                Some(tcl_registry::native_result::NativeResultContract::ReturnResult));
            assert_eq!(tokens.source_binding, before);
            assert!(!tokens.source_binding.as_ref().unwrap().unobserved_native_dispatch());
            assert!(fixture.child.original_completed_command_world().is_none());
            assert!(tcl_compiler::registry_invocation::original_authored_result_completion_for_source_coverage(
                fixture.child_source, &fixture.child, tokens,
            ).is_none());
            let foreign = fixture.child.clone();
            assert!(layout.result_completion_coverage(fixture.child_source, &foreign, tokens).is_none());
            assert!(layout.result_completion_coverage(&format!("{}\n", fixture.child_source), &fixture.child, tokens).is_none());
        });
}

#[test]
fn cut_body_result_coverage_refuses_options_dynamic_argv_and_replaced_handlers() {
    for body in [
        "return -code error E",
        "return -level 2 X",
        "return $unknown",
        "return [set x VALUE]",
        "return {*}$args",
    ] {
        let child = Box::leak(format!("proc helper {{}} {{{body}}}").into_boxed_str());
        let fixture = Fixture::new("source child.tcl; proc run {} {return ROOT}", child);
        fixture.with_body_layout(Some(0), |layout, tokens| {
            assert!(
                layout
                    .result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                    .is_none(),
                "{body}"
            );
        });
    }
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return ROOT}",
        "proc return args {error custom}; proc helper {} {return VALUE}",
    );
    fixture.with_body_layout(Some(1), |layout, tokens| {
        assert!(
            layout
                .result_completion_coverage(fixture.child_source, &fixture.child, tokens)
                .is_none()
        );
    });
}

#[test]
fn cut_body_reference_is_exact_and_leaves_ordinary_dispatch_unavailable() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {helper}",
        "proc helper {} {return VALUE}",
    );
    fixture.with_suffix_body_layout(|layout, tokens| {
        let binding = tokens.source_binding.as_ref().unwrap();
        let before = binding.clone();
        assert!(binding.original_command_reference(tokens).is_none());
        let reference = layout
            .source_command_reference(fixture.source, &fixture.suffix, tokens)
            .expect("same captured row, complete static argv and actual child allocation");
        assert!(reference.is_for_analysis(&fixture.suffix));
        assert!(crate::original_declaration::reference_matches_declaration(
            reference.source_reference(),
            fixture
                .child
                .original_procedure_declarations()
                .next()
                .unwrap(),
            false,
        ));
        assert_eq!(tokens.source_binding.as_ref(), Some(&before));
        assert!(binding.original_command_reference(tokens).is_none());
        assert!(fixture.child.original_completed_command_world().is_none());
        let foreign = fixture.suffix.clone();
        assert!(
            layout
                .source_command_reference(fixture.source, &foreign, tokens)
                .is_none()
        );
        assert!(
            layout
                .source_command_reference(&format!("{}\n", fixture.source), &fixture.suffix, tokens)
                .is_none()
        );
        let mut uncertain = tokens.clone();
        assert!(!uncertain.source_binding.as_ref().unwrap().may_be_absent);
        uncertain.source_binding.as_mut().unwrap().may_be_absent = true;
        assert_ne!(uncertain.source_binding.as_ref(), Some(&before));
        assert!(
            layout
                .source_command_reference(fixture.source, &fixture.suffix, &uncertain)
                .is_none()
        );
        assert!(uncertain.source_binding.as_ref().unwrap().may_be_absent);
    });
    let foreign = Fixture::new(fixture.source, fixture.child_source);
    fixture.with_suffix_body_layout(|layout, _| {
        foreign.with_suffix_body_layout(|_, tokens| {
            assert!(
                layout
                    .source_command_reference(fixture.source, &fixture.suffix, tokens)
                    .is_none()
            );
        });
    });
}

#[test]
fn cut_body_reference_refuses_dynamic_arguments_under_a_real_layout() {
    for body in ["helper $unknown", "helper {*}$args", "helper [set value X]"] {
        let source =
            Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
        let fixture = Fixture::new(source, "proc helper {} {return VALUE}");
        fixture.with_suffix_body_layout(|layout, tokens| {
            assert!(
                layout
                    .source_command_reference(fixture.source, &fixture.suffix, tokens)
                    .is_none(),
                "{body}"
            );
        });
    }
}

#[test]
fn genuine_single_cut_inherited_declaration_renames_both_physical_files() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {helper}",
        "proc helper {} {return VALUE}",
    );
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        let edits = super::super::original_command_rename_edits_with_selected_source_view(
            &documents,
            &identity,
            "updated_helper",
            "file:///child.tcl",
            5,
            &worlds,
            views,
        )
        .expect("actual whole-root Normal source cut, declaration and complete call coverage");
        assert_eq!(edits.len(), 2);
        assert_eq!(
            edits
                .iter()
                .filter(|edit| edit.uri == "file:///child.tcl")
                .count(),
            1
        );
        assert_eq!(
            edits
                .iter()
                .filter(|edit| edit.uri == "file:///main.tcl")
                .count(),
            1
        );
        assert!(edits.iter().all(|edit| edit.new_text == "updated_helper"));
    });
}

#[test]
fn inherited_cut_keeps_root_child_and_registry_collisions() {
    for (source, child_source, new_tail, child_occupied) in [
        (
            "source child.tcl; proc run {} {helper}; proc updated_helper {} {return ROOT}",
            "proc helper {} {return VALUE}",
            "updated_helper",
            false,
        ),
        (
            "source child.tcl; proc run {} {helper}",
            "proc helper {} {return VALUE}; proc updated_helper {} {return CHILD}",
            "updated_helper",
            true,
        ),
        (
            "source child.tcl; proc run {} {helper}",
            "proc helper {} {return VALUE}",
            "puts",
            true,
        ),
    ] {
        let fixture = Fixture::new(source, child_source);
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            let selected = OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap()
            .expect("genuine cut and unchanged selected allocation");
            let declaration =
                super::super::rename_declaration(&fixture.child, selected.identity()).unwrap();
            let policy = declaration.name().policy();
            let destination =
                super::super::renamed_slot(declaration.name().slot(), policy, new_tail.as_bytes())
                    .unwrap();
            assert_eq!(
                selected
                    .publication()
                    .command_slot_occupied(&destination, policy),
                Some(child_occupied),
            );
            assert_eq!(
                super::super::occupied_with_cut(
                    selected.documents(),
                    &worlds,
                    Some(selected.publication()),
                    &destination,
                    policy,
                ),
                Ok(true),
            );
            assert_eq!(
                super::super::original_command_rename_edits_with_selected_source_view(
                    &documents,
                    &identity,
                    new_tail,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                ),
                Err(OriginalCommandRenameRefusal::Collision),
            );
        });
    }
}

#[test]
fn inherited_header_refuses_uncovered_namespace_eval_in_physical_body() {
    let fixture = Fixture::new(
        "namespace eval ::ns {}; proc ::ns::helper {} {return PREFIX}; source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return CHILD}; proc caller {} {namespace eval ::ns {helper}}",
    );
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        let selected = OriginalInheritedDeclarationSelection::capture(
            &documents,
            &identity,
            "file:///child.tcl",
            5,
            &worlds,
            views,
        )
        .unwrap()
        .expect("actual inherited declaration header");
        assert_eq!(
            super::super::original_command_rename_edits(
                &[selected.physical()],
                &identity,
                "updated_helper",
            ),
            Err(OriginalCommandRenameRefusal::IncompleteCoverage),
            "the standalone namespace eval has no complete source coverage",
        );
        assert_eq!(
            super::super::original_command_rename_edits_with_selected_source_view(
                &documents,
                &identity,
                "updated_helper",
                "file:///child.tcl",
                5,
                &worlds,
                views,
            ),
            Err(OriginalCommandRenameRefusal::IncompleteCoverage),
        );
    });
}

#[test]
fn selected_physical_agreement_refuses_omitted_real_reference_edit() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return VALUE}; proc caller {} {helper}",
    );
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        let selected = OriginalInheritedDeclarationSelection::capture(
            &documents,
            &identity,
            "file:///child.tcl",
            5,
            &worlds,
            views,
        )
        .unwrap()
        .expect("actual inherited declaration header");
        let physical = super::super::original_command_rename_edits(
            &[selected.physical()],
            &identity,
            "updated_helper",
        )
        .expect("complete actual physical plan");
        let coverage =
            SourceCoverage::capture(selected.documents(), selected.identity(), &worlds, views)
                .expect("actual complete cut coverage");
        let mut inherited = super::super::original_command_rename_units_with_source_coverage(
            selected.documents(),
            selected.identity(),
            b"updated_helper",
            &worlds,
            Some(&coverage),
            Some(selected.publication()),
        )
        .expect("complete actual inherited plan");
        assert_eq!(physical.len(), 2);
        assert_eq!(
            super::super::require_selected_physical_edits(
                &physical,
                &inherited,
                "file:///child.tcl"
            ),
            Ok(())
        );
        // Deliberately omit one actual issued edit to test the agreement guard;
        // this does not assert that the actual owners diverged.
        inherited.pop().expect("actual issued inherited edit");
        assert_eq!(
            super::super::require_selected_physical_edits(
                &physical,
                &inherited,
                "file:///child.tcl"
            ),
            Err(OriginalCommandRenameRefusal::IncompleteCoverage)
        );
    });
}

#[test]
fn inherited_cut_reference_coverage_keeps_dynamic_and_unowned_calls_refused() {
    for body in [
        "helper $dynamic",
        "helper {*}$args",
        "helper [set x V]",
        "unknown_helper",
    ] {
        let source =
            Box::leak(format!("source child.tcl; proc run {{}} {{{body}}}").into_boxed_str());
        let fixture = Fixture::new(source, "proc helper {} {return VALUE}");
        let documents = fixture.documents();
        let identity = inherited_child_identity(&fixture);
        let worlds = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &fixture.outcome,
        }];
        fixture.with_views(|views| {
            assert!(
                OriginalInheritedDeclarationSelection::capture(
                    &documents,
                    &identity,
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                )
                .unwrap()
                .is_some(),
                "genuine closed publication before reference coverage: {body}",
            );
            assert_eq!(
                super::super::original_command_rename_edits_with_selected_source_view(
                    &documents,
                    &identity,
                    "updated_helper",
                    "file:///child.tcl",
                    5,
                    &worlds,
                    views,
                ),
                Err(OriginalCommandRenameRefusal::IncompleteCoverage),
                "{body}",
            );
        });
    }
}

#[test]
fn inherited_selection_refuses_foreign_and_duplicate_owners() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return VALUE}",
    );
    let other = Fixture::new(fixture.source, fixture.child_source);
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap()
            .is_some(),
            "non-vacuous genuine source relationship"
        );
        let foreign = [OriginalConditionalCommandWorld {
            uri: "file:///main.tcl",
            outcome: &other.outcome,
        }];
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &foreign,
                views,
            )
            .is_err()
        );
        let duplicate = [
            OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            },
            OriginalConditionalCommandWorld {
                uri: "file:///main.tcl",
                outcome: &fixture.outcome,
            },
        ];
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &duplicate,
                views,
            )
            .is_err()
        );
        let mut duplicate_documents = documents.clone();
        duplicate_documents.push(documents[1]);
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &duplicate_documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .is_err()
        );
    });
}

#[test]
fn inherited_selection_refuses_stale_source_input_and_ancestry() {
    let fixture = Fixture::new(
        "source child.tcl; proc run {} {return VALUE}",
        "proc helper {} {return VALUE}",
    );
    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .unwrap()
            .is_some()
        );
        let changed = format!("{} ", fixture.child_source);
        let mut stale = documents.clone();
        stale[1].source = &changed;
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &stale,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .is_err()
        );
        let changed_input = Analyser::new().analyse(fixture.child_source, "tcl9.0");
        let mut stale_input = documents.clone();
        stale_input[1].analysis = &changed_input;
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &stale_input,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .is_err()
        );
        let missing_ancestry = views
            .iter()
            .map(|view| OriginalSourceViewDocument {
                publication: view.publication,
                document: view.document,
                region: view.region,
                ancestors: &[],
            })
            .collect::<Vec<_>>();
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                &missing_ancestry,
            )
            .is_err()
        );
    });
}

#[test]
fn inherited_selection_never_promotes_a_native_namespace_entry_or_logical_cut() {
    let source = "namespace eval ::N {source child.tcl}; proc run {} {return VALUE}";
    let child_source = "proc helper {} {return VALUE}";
    let fixture = Fixture::new(source, child_source);
    assert!(fixture.region_count > 1);

    let documents = fixture.documents();
    let identity = inherited_child_identity(&fixture);
    let worlds = [OriginalConditionalCommandWorld {
        uri: "file:///main.tcl",
        outcome: &fixture.outcome,
    }];
    fixture.with_views(|views| {
        assert!(
            OriginalInheritedDeclarationSelection::capture(
                &documents,
                &identity,
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .is_err()
        );
        assert!(
            super::super::original_command_rename_edits_with_selected_source_view(
                &documents,
                &identity,
                "updated_helper",
                "file:///child.tcl",
                5,
                &worlds,
                views,
            )
            .is_err()
        );
    });
    assert_logical_source_rename_issuer_unavailable(source, child_source);
}
