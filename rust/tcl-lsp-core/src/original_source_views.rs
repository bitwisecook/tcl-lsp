// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional procedure relationships retaining exact interpretation owners.
//! The host supplies current physical publication chains and rechecks them at
//! rendering. These positive relationships grant no execution or edit proof.

use crate::original_declaration::{
    OriginalDeclarationDocument, OriginalDeclarationIdentity, OriginalSourceViewPublication,
};
use std::fmt::Write as _;
use tcl_compiler::command_binding::OriginalSourceProcedureCall;
use tcl_lexer::Span;

#[derive(Default)]
struct PrepareHeldTrace {
    detail: String,
    heads_seen: usize,
}

thread_local! {
    static PREPARE_REFUSAL_TRACE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    static PREPARE_HELD_TRACE: std::cell::RefCell<PrepareHeldTrace> = const { std::cell::RefCell::new(PrepareHeldTrace { detail: String::new(), heads_seen: 0 }) };
}

fn trace_prepare_held(record: impl FnOnce(&mut PrepareHeldTrace)) {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        PREPARE_HELD_TRACE.with(|trace| record(&mut trace.borrow_mut()));
    }
}

fn trace_prepare_inventory(owner: &OriginalSourceViewDocument<'_>, offset: u32, calls: usize) {
    trace_prepare_held(|trace| {
        let _ = write!(
            trace.detail,
            " offset={offset} calls={calls} selected_analysis={:p} selected_region={}",
            owner.document.analysis,
            owner.region.is_some(),
        );
    });
}

fn trace_prepare_head(span: Option<Span>, owned: bool, matches: bool) {
    trace_prepare_held(|trace| {
        trace.heads_seen += 1;
        if trace.heads_seen <= 16 {
            let _ = write!(
                trace.detail,
                " head[{}]={span:?}:owned={owned}:matches={matches}",
                trace.heads_seen,
            );
        } else if trace.heads_seen == 17 {
            trace.detail.push_str(" head_rows_truncated=true");
        }
    });
}

fn trace_prepare_uncertainty(operations: &[std::sync::Arc<[tcl_lexer::NativeWord]>]) {
    trace_prepare_held(|trace| {
        let _ = write!(trace.detail, " uncertainty_count={}", operations.len());
        for (ordinal, operation) in operations.iter().take(16).enumerate() {
            let first = operation.first();
            let _ = write!(
                trace.detail,
                " debt[{ordinal}]=words:{}:head:{:?}:last:{:?}:image_bytes:{:?}",
                operation.len(),
                first.map(tcl_lexer::NativeWord::span),
                operation.last().map(tcl_lexer::NativeWord::span),
                first.map(|word| word.image().bytes().len()),
            );
        }
        if operations.len() > 16 {
            trace.detail.push_str(" debt_rows_truncated=true");
        }
    });
}

fn trace_prepare_refusal(stage: &'static str) {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        PREPARE_REFUSAL_TRACE.with(|trace| {
            if trace.get().is_none() {
                trace.set(Some(stage));
            }
        });
    }
}

fn prepare_required<T>(value: Option<T>, stage: &'static str) -> Option<T> {
    if value.is_none() {
        trace_prepare_refusal(stage);
    }
    value
}

/// Drain the first refusal from the existing synchronous prepare computation.
/// This observes no additional semantic query or currentness decision.
#[must_use]
pub fn take_prepare_refusal_trace() -> Option<String> {
    if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        let stage = PREPARE_REFUSAL_TRACE.with(|trace| trace.take());
        let held = PREPARE_HELD_TRACE.with(|trace| std::mem::take(&mut *trace.borrow_mut()));
        stage.map(|stage| format!("{stage}{}", held.detail))
    } else {
        None
    }
}

/// One exact retained interpretation and its genuine physical ancestor chain.
pub struct OriginalSourceViewDocument<'a> {
    /// Typed host publication locator, independently checked by the host.
    pub publication: &'a OriginalSourceViewPublication,
    /// The actual producer's source and analysis, never a URI reconstruction.
    pub document: OriginalDeclarationDocument<'a>,
    /// Exact issuer-owned partial view, separately selected from a whole input.
    /// Its command inventory never widens whole-document/CU completion.
    pub region: Option<&'a tcl_compiler::command_binding::OriginalSourceConditionalRegion>,
    /// Current retained ancestor locators in nearest-parent-first order.
    pub ancestors: &'a [OriginalSourceViewPublication],
}

impl OriginalSourceViewDocument<'_> {
    fn current(&self) -> Option<()> {
        let analysis = self.document.analysis;
        let input = analysis.resolved_input.as_ref()?;
        if analysis.analysis_context_unavailable.is_some()
            || analysis.body_lexer_config != Some(input.lexer_config())
            || !analysis
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        match self.region {
            None => crate::original_context::CurrentSourceContext::capture(
                self.document.source,
                self.document.analysis,
            )
            .map(|_| ()),
            Some(region) => {
                let analysis = self.document.analysis;
                let config = analysis.body_lexer_config?;
                (region.matches_region_analysis(analysis)
                    && analysis.original_source_region() == Some(region)
                    && analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(self.document.source),
                        config,
                    ))
                .then_some(())
            }
        }
    }

    fn procedure_calls(&self) -> Option<Vec<OriginalSourceProcedureCall>> {
        self.current()?;
        match self.region {
            Some(region) => region.original_procedure_calls(self.document.analysis),
            None => {
                tcl_compiler::registry_invocation::source_structure::source_original_procedure_calls(
                    self.document.source,
                    self.document.analysis,
                )
            }
        }
    }

    fn owns_command(&self, offset: u32) -> bool {
        self.region
            .is_none_or(|region| region.owns_original_command_at(self.document.analysis, offset))
    }

    fn owns_procedure_call(&self, call: &OriginalSourceProcedureCall) -> bool {
        self.region
            .is_none_or(|region| region.owns_original_procedure_call(self.document.analysis, call))
    }

    fn owns_declaration(&self, identity: &OriginalDeclarationIdentity) -> bool {
        identity
            .procedure_metadata(self.document.analysis)
            .is_some_and(|row| self.owns_command(row.declaration_site().offset))
    }
}

/// A selected declaration retaining the view used for metadata and lifting.
#[derive(Clone)]
pub struct OriginalSourceViewDeclaration {
    /// Same typed publication belonging to the selected producer.
    pub publication: OriginalSourceViewPublication,
    /// Genuine compiler-selected original declaration identity.
    pub identity: OriginalDeclarationIdentity,
}

/// One positive conditional call relationship, with separate actual owners.
pub struct OriginalSourceViewCallEdge {
    /// Actual calling declaration and interpretation.
    pub caller: OriginalSourceViewDeclaration,
    /// Actual declaring interpretation selected by the retained call.
    pub target: OriginalSourceViewDeclaration,
    /// Original written call head in the caller's complete physical image.
    pub span: Span,
}

fn target(
    call: &OriginalSourceProcedureCall,
    consumer: &OriginalSourceViewDocument<'_>,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<OriginalSourceViewDeclaration> {
    let mut selected = documents.iter().filter_map(|owner| {
        let record = if owner.publication == consumer.publication {
            call.declaration(consumer.document.analysis)
        } else {
            consumer
                .ancestors
                .contains(owner.publication)
                .then_some(())?;
            call.declaration_in_parent(consumer.document.analysis, owner.document.analysis)
                .or_else(|| {
                    consumer.region?;
                    call.declaration_in_region_dependency(
                        consumer.document.analysis,
                        owner.document.analysis,
                    )
                })
        }?;
        owner.current()?;
        owner
            .owns_command(record.declaration_site().offset)
            .then_some(())?;
        let identity = OriginalDeclarationIdentity::for_procedure(
            owner.document.uri,
            owner.document.source,
            owner.document.analysis,
            record,
        )?;
        Some(OriginalSourceViewDeclaration {
            publication: owner.publication.clone(),
            identity,
        })
    });
    let first = selected.next()?;
    selected.next().is_none().then_some(first)
}

fn region_owns_cursor(owner: &OriginalSourceViewDocument<'_>, offset: u32) -> Option<bool> {
    owner.current()?;
    let region = owner.region?;
    let analysis = owner.document.analysis;
    let contains = |span: Span| span.start() <= offset && offset < span.end();
    if analysis.original_procedure_declarations().any(|row| {
        contains(row.name_input().span())
            && region.owns_original_command_at(analysis, row.declaration_site().offset)
    }) {
        return Some(true);
    }
    if owner.procedure_calls()?.iter().any(|call| {
        call.original_words()
            .first()
            .is_some_and(|head| contains(head.span()) && owner.owns_procedure_call(call))
    }) {
        return Some(true);
    }
    let image = tcl_lexer::SourceImage::document(owner.document.source);
    let config = analysis.body_lexer_config?;
    Some(analysis.command_invocations.iter().any(|call| {
        contains(call.range)
            && region.owns_original_command_at(analysis, call.range.start())
            && matches!(call.original_name_input.as_ref(),
                Some(tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(word))
                    if word.source_image() == &image
                        && word.lexer_config() == config
                        && word.span() == call.range)
    }))
}

/// Exact command/declaration ownership chooses appended suffix views. A region
/// can replace only its own retained prefix's answer at an actually owned
/// cursor; unrelated physical interpretations remain independently required.
/// This projection supplies no target or lookup permission.
#[must_use]
pub fn cursor_publications(
    uri: &str,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<Vec<OriginalSourceViewPublication>> {
    let owners = documents
        .iter()
        .filter(|row| row.document.uri == uri)
        .collect::<Vec<_>>();
    let mut claims = Vec::new();
    for owner in &owners {
        owner.current()?;
        if owner.region.is_some() && region_owns_cursor(owner, offset)? {
            claims.push(*owner);
        }
    }
    Some(
        owners
            .into_iter()
            .filter(|owner| {
                (owner.region.is_none()
                    || claims
                        .iter()
                        .any(|claim| claim.publication == owner.publication))
                    && !claims.iter().any(|claim| {
                        claim.publication != owner.publication
                            && claim.ancestors.contains(owner.publication)
                    })
            })
            .map(|owner| owner.publication.clone())
            .collect(),
    )
}

/// Select a local declaration or exact conditional procedure target. The
/// selected publication is explicit; equal URI/text cannot pick another view.
#[must_use]
pub fn prepare(
    selected: &OriginalSourceViewPublication,
    offset: u32,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Option<OriginalSourceViewDeclaration> {
    let _ = take_prepare_refusal_trace();
    let mut owners = documents.iter().filter(|row| row.publication == selected);
    let owner = prepare_required(owners.next(), "prepare.selected-owner")?;
    prepare_required(
        owners.next().is_none().then_some(()),
        "prepare.unique-owner",
    )?;
    let doc = &owner.document;
    prepare_required(owner.current(), "prepare.current-owner")?;
    let rows = prepare_required(
        crate::original_declaration::declarations(doc.uri, doc.source, doc.analysis),
        "prepare.declaration-inventory",
    )?;
    let mut local = rows.into_iter().filter(|row| {
        owner.owns_declaration(row) && row.span().start() <= offset && offset < row.span().end()
    });
    if let Some(identity) = local.next() {
        return prepare_required(
            local
                .next()
                .is_none()
                .then(|| OriginalSourceViewDeclaration {
                    publication: selected.clone(),
                    identity,
                }),
            "prepare.unique-local-declaration",
        );
    }
    let calls = prepare_required(owner.procedure_calls(), "prepare.procedure-call-inventory")?;
    trace_prepare_inventory(owner, offset, calls.len());
    let mut matching = calls.iter().filter(|call| {
        let head = call.original_words().first();
        if head.is_none() {
            trace_prepare_head(None, false, false);
        }
        head.is_some_and(|head| {
            let span = head.span();
            let owned = owner.owns_procedure_call(call);
            let matches = owned && span.start() <= offset && offset < span.end();
            trace_prepare_head(Some(span), owned, matches);
            matches
        })
    });
    let call = prepare_required(matching.next(), "prepare.matching-call")?;
    prepare_required(
        matching.all(|other| other == call).then_some(()),
        "prepare.unique-call",
    )?;
    let uncertainty = call.uncertain_operations();
    if !uncertainty.is_empty() {
        trace_prepare_uncertainty(uncertainty);
        trace_prepare_refusal("prepare.call-uncertainty");
        return None;
    }
    if call.has_lookup_operand_effects() {
        trace_prepare_refusal("prepare.lookup-operand-effects");
        return None;
    }
    prepare_required(target(call, owner, documents), "prepare.target-owner")
}

/// Positive conditional procedure edges. Missing coverage is unavailable; an
/// empty result cannot certify absence of calls or edit coverage.
#[must_use]
pub fn procedure_edges(
    documents: &[OriginalSourceViewDocument<'_>],
) -> Vec<OriginalSourceViewCallEdge> {
    let mut result = Vec::new();
    for consumer in documents {
        let doc = &consumer.document;
        if consumer.current().is_none() {
            continue;
        }
        let Some(calls) = consumer.procedure_calls() else {
            continue;
        };
        for call in calls {
            if call.original_words().is_empty() || !consumer.owns_procedure_call(&call) {
                continue;
            }
            if !call.uncertain_operations().is_empty() || call.has_lookup_operand_effects() {
                continue;
            }
            let Some(caller) = call.caller_declaration(doc.analysis).and_then(|record| {
                OriginalDeclarationIdentity::for_procedure(
                    doc.uri,
                    doc.source,
                    doc.analysis,
                    record,
                )
            }) else {
                continue;
            };
            if !consumer.owns_declaration(&caller) {
                continue;
            }
            let Some(target) = target(&call, consumer, documents) else {
                continue;
            };
            let Some(head) = call.original_words().first() else {
                continue;
            };
            result.push(OriginalSourceViewCallEdge {
                caller: OriginalSourceViewDeclaration {
                    publication: consumer.publication.clone(),
                    identity: caller,
                },
                target,
                span: head.span(),
            });
        }
    }
    result
}

/// A positive exact source reference, including top-level calls without a
/// procedure caller. It supplies no complete absence or edit-coverage proof.
pub struct OriginalSourceViewReference {
    /// Actual physical/source publication containing this call.
    pub publication: OriginalSourceViewPublication,
    /// Genuine original written call-head geometry.
    pub span: Span,
}

/// Positive calls to one actual declaration across the supplied current views.
/// Call hierarchy's caller-definition requirement cannot remove file-root
/// references from this separate inventory.
#[must_use]
pub fn references(
    selected: &OriginalSourceViewDeclaration,
    documents: &[OriginalSourceViewDocument<'_>],
) -> Vec<OriginalSourceViewReference> {
    let mut result = Vec::<OriginalSourceViewReference>::new();
    for consumer in documents {
        let Some(calls) = consumer.procedure_calls() else {
            continue;
        };
        for call in calls {
            let Some(head) = call.original_words().first() else {
                continue;
            };
            if !consumer.owns_procedure_call(&call)
                || !call.uncertain_operations().is_empty()
                || call.has_lookup_operand_effects()
            {
                continue;
            }
            let Some(target) = target(&call, consumer, documents) else {
                continue;
            };
            if target.publication == selected.publication
                && target.identity == selected.identity
                && !result
                    .iter()
                    .any(|old| &old.publication == consumer.publication && old.span == head.span())
            {
                result.push(OriginalSourceViewReference {
                    publication: consumer.publication.clone(),
                    span: head.span(),
                });
            }
        }
    }
    result
}

#[cfg(test)]
mod nested_procedure_navigation_tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult};

    fn select(
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<OriginalSourceViewDeclaration> {
        let publication = OriginalSourceViewPublication::retain(1_u8);
        let inventory = [OriginalSourceViewDocument {
            publication: &publication,
            document: OriginalDeclarationDocument {
                uri: "file:///host-naming.tcl",
                source,
                analysis,
            },
            region: None,
            ancestors: &[],
        }];
        prepare(&publication, offset, &inventory)
    }

    #[test]
    fn nested_procedure_navigation_keeps_its_original_operand_horizon() {
        for dialect in ["tcl8.6", "tcl9.0"] {
            for source in [
                "proc host_old {value} { return $value }\nputs \"🧪 café\"; puts [host_old 7]\n",
                "proc host_old {value} {return $value}\nputs [list [host_old 7]]\n",
                "proc host_old {value} {return $value}\nputs [host_old 7] $later\n",
            ] {
                let analysis = Analyser::new().analyse(source, dialect);
                let offset = u32::try_from(source.find("host_old 7").unwrap()).unwrap();
                let selected = select(source, &analysis, offset).expect(source);
                assert_eq!(&source[selected.identity.span().as_range()], "host_old");
                assert_eq!(selected.identity.span().start(), 5);
                let call = tcl_compiler::registry_invocation::source_structure::source_original_procedure_call_at(
                    source, &analysis, offset,
                ).unwrap();
                assert!(call.uncertain_operations().is_empty());
                assert!(!call.has_lookup_operand_effects());
                let independent = Analyser::new().analyse(source, dialect);
                assert!(call.declaration(&independent).is_none());
                let changed = source.replace("host_old 7", "host_old 8");
                assert!(select(&changed, &analysis, offset).is_none());
            }
        }
    }

    #[test]
    fn nested_procedure_navigation_retains_earlier_operand_and_parent_debts() {
        for source in [
            "proc host_old {value} {return $value}\nputs $earlier [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs $earlier(index) [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [opaque] [host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [rename host_old removed; host_old 7]\n",
            "proc host_old {value} {return $value}\nputs [host_old [opaque]]\n",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("host_old ").unwrap()).unwrap();
            assert!(select(source, &analysis, offset).is_none(), "{source}");
        }
        let source = "proc host_old {value} {return $value}\nputs [host_old 7]; host_old 8\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("host_old 8").unwrap()).unwrap();
        assert!(select(source, &analysis, offset).is_none());
    }
}

#[cfg(test)]
mod region_call_cursor_tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult};
    use tcl_compiler::command_binding::OriginalSourceConditionalRegion;

    const CHILD: &str = "proc helper {} {}";

    fn fixture(
        source: &str,
    ) -> (
        AnalysisResult,
        AnalysisResult,
        OriginalSourceConditionalRegion,
        AnalysisResult,
    ) {
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load =
            tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(
                source, &parent, 0,
            )
            .unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
            .unwrap();
        let continuation = load.conditional_continuation(&parent, &child).unwrap();
        let regions = continuation.source_regions(&parent, &child).unwrap();
        let region = regions.regions()[0].clone();
        let caller = Analyser::new()
            .analyse_source_region(&region, &parent, &child)
            .unwrap();
        (parent, child, region, caller)
    }

    #[test]
    fn region_call_cursor_uses_retained_call_after_reporting_rows_clear() {
        let source = "source child.tcl; helper";
        let (parent, child, region, mut caller) = fixture(source);
        caller.command_invocations.clear();
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let inventory = [
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        assert_eq!(
            cursor_publications("file:///main.tcl", offset, &inventory),
            Some(vec![suffix.clone()])
        );
        let selected = prepare(&suffix, offset, &inventory).unwrap();
        assert_eq!(selected.publication, loaded);
        assert_eq!(&CHILD[selected.identity.span().as_range()], "helper");
        assert_eq!(
            cursor_publications("file:///main.tcl", 0, &inventory),
            Some(vec![root])
        );

        let changed = "source other.tcl; helper";
        let stale = [OriginalSourceViewDocument {
            publication: &suffix,
            document: OriginalDeclarationDocument {
                uri: "file:///main.tcl",
                source: changed,
                analysis: &caller,
            },
            region: Some(&region),
            ancestors: &ancestors,
        }];
        assert!(cursor_publications("file:///main.tcl", offset, &stale).is_none());
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        let foreign = [OriginalSourceViewDocument {
            publication: &suffix,
            document: OriginalDeclarationDocument {
                uri: "file:///main.tcl",
                source,
                analysis: &foreign,
            },
            region: Some(&region),
            ancestors: &ancestors,
        }];
        assert!(cursor_publications("file:///main.tcl", offset, &foreign).is_none());
    }

    #[test]
    fn region_cursor_claim_cannot_donate_target_or_clear_earlier_debt() {
        for source in [
            "source child.tcl; helper",
            "source child.tcl; opaque; helper",
        ] {
            let (parent, child, region, mut caller) = fixture(source);
            caller.command_invocations.clear();
            let root = OriginalSourceViewPublication::retain(1_u8);
            let loaded = OriginalSourceViewPublication::retain(2_u8);
            let suffix = OriginalSourceViewPublication::retain(3_u8);
            let ancestors = [loaded.clone(), root];
            let load = tcl_compiler::registry_invocation::source_structure::original_project_source_load_at(source, &parent, 0).unwrap();
            let point = load.source_point_entry(&parent).unwrap();
            let foreign = Analyser::new()
                .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
                .unwrap();
            let inventory = [
                OriginalSourceViewDocument {
                    publication: &loaded,
                    document: OriginalDeclarationDocument {
                        uri: "file:///child.tcl",
                        source: CHILD,
                        analysis: &foreign,
                    },
                    region: None,
                    ancestors: &[],
                },
                OriginalSourceViewDocument {
                    publication: &suffix,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source,
                        analysis: &caller,
                    },
                    region: Some(&region),
                    ancestors: &ancestors,
                },
            ];
            let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
            assert!(prepare(&suffix, offset, &inventory).is_none());
            if source.contains("opaque") {
                let inventory = [
                    OriginalSourceViewDocument {
                        publication: &loaded,
                        document: OriginalDeclarationDocument {
                            uri: "file:///child.tcl",
                            source: CHILD,
                            analysis: &child,
                        },
                        region: None,
                        ancestors: &[],
                    },
                    OriginalSourceViewDocument {
                        publication: &suffix,
                        document: OriginalDeclarationDocument {
                            uri: "file:///main.tcl",
                            source,
                            analysis: &caller,
                        },
                        region: Some(&region),
                        ancestors: &ancestors,
                    },
                ];
                assert!(prepare(&suffix, offset, &inventory).is_none());
            }
        }
    }

    #[test]
    fn deferred_region_body_call_retains_its_selected_caller_owner() {
        let source = "source child.tcl; proc run {} {puts [helper]}";
        let (parent, child, region, mut caller) = fixture(source);
        caller.command_invocations.clear();
        let root = OriginalSourceViewPublication::retain(1_u8);
        let loaded = OriginalSourceViewPublication::retain(2_u8);
        let suffix = OriginalSourceViewPublication::retain(3_u8);
        let ancestors = [loaded.clone(), root.clone()];
        let inventory = [
            OriginalSourceViewDocument {
                publication: &root,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &parent,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &loaded,
                document: OriginalDeclarationDocument {
                    uri: "file:///child.tcl",
                    source: CHILD,
                    analysis: &child,
                },
                region: None,
                ancestors: &[],
            },
            OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            },
        ];
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        assert!(!region.owns_original_command_at(&caller, offset));
        let calls = region.original_procedure_calls(&caller).unwrap();
        let call = calls
            .iter()
            .find(|call| call.original_words()[0].span().start() == offset)
            .unwrap();
        assert!(call.is_future_body());
        assert!(region.owns_original_procedure_call(&caller, call));
        assert_eq!(
            cursor_publications("file:///main.tcl", offset, &inventory),
            Some(vec![suffix.clone()])
        );
        let selected = prepare(&suffix, offset, &inventory).unwrap();
        assert_eq!(selected.publication, loaded);
        assert_eq!(&CHILD[selected.identity.span().as_range()], "helper");
        let found = references(&selected, &inventory);
        assert!(
            found
                .iter()
                .any(|row| row.publication == suffix && row.span.start() == offset)
        );
        let (_, foreign_child, foreign_region, foreign_caller) = fixture(source);
        assert!(!foreign_region.owns_original_procedure_call(&foreign_caller, call));
        assert!(!region.owns_original_procedure_call(&foreign_caller, call));
        assert!(
            call.declaration_in_region_dependency(&caller, &foreign_child)
                .is_none()
        );
    }

    #[test]
    fn deferred_region_body_call_keeps_debt_and_data_exclusion() {
        for source in [
            "source child.tcl; proc run {} {opaque; puts [helper]}",
            "source child.tcl; proc run {} {set payload {helper}}",
        ] {
            let (_, child, region, caller) = fixture(source);
            let loaded = OriginalSourceViewPublication::retain(2_u8);
            let suffix = OriginalSourceViewPublication::retain(3_u8);
            let ancestors = [loaded.clone()];
            let inventory = [
                OriginalSourceViewDocument {
                    publication: &loaded,
                    document: OriginalDeclarationDocument {
                        uri: "file:///child.tcl",
                        source: CHILD,
                        analysis: &child,
                    },
                    region: None,
                    ancestors: &[],
                },
                OriginalSourceViewDocument {
                    publication: &suffix,
                    document: OriginalDeclarationDocument {
                        uri: "file:///main.tcl",
                        source,
                        analysis: &caller,
                    },
                    region: Some(&region),
                    ancestors: &ancestors,
                },
            ];
            let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
            assert!(prepare(&suffix, offset, &inventory).is_none(), "{source}");
            let changed = source.replace("helper", "other_");
            let stale = [OriginalSourceViewDocument {
                publication: &suffix,
                document: OriginalDeclarationDocument {
                    uri: "file:///main.tcl",
                    source: &changed,
                    analysis: &caller,
                },
                region: Some(&region),
                ancestors: &ancestors,
            }];
            assert!(cursor_publications("file:///main.tcl", offset, &stale).is_none());
        }
    }
}
