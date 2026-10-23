// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current declaration selections and partial readonly lens occurrences.

use crate::original_declaration::OriginalDeclarationDocument;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Declaration {
    Callable(Box<crate::call_hierarchy::LogicalCallIdentity>),
    Class(Box<crate::type_hierarchy::LogicalHierarchyClass>),
}

/// Authentic Logical declaration selection for a readonly source lens.
/// Neither the label nor the client range can construct or redirect it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalLensIdentity {
    declaration: Declaration,
}
impl LogicalLensIdentity {
    /// Independent original document owner.
    #[must_use]
    pub fn uri(&self) -> &str {
        match &self.declaration {
            Declaration::Callable(row) => row.uri(),
            Declaration::Class(row) => row.uri(),
        }
    }
    /// Reborrow the genuine declaration under current whole source and input.
    #[must_use]
    pub fn is_current(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> bool {
        match &self.declaration {
            Declaration::Callable(row) => row.is_current(uri, source, analysis),
            Declaration::Class(row) => row.is_current(uri, source, analysis),
        }
    }
    /// Validate this selection against an independently retained source inventory.
    #[must_use]
    pub fn is_current_in(&self, documents: &[OriginalDeclarationDocument<'_>]) -> bool {
        let mut uris = std::collections::HashSet::new();
        if documents.iter().any(|document| {
            !uris.insert(document.uri)
                || crate::original_context::CurrentSourceContext::capture(
                    document.source,
                    document.analysis,
                )
                .is_none()
        }) {
            return false;
        }
        match &self.declaration {
            Declaration::Callable(row) => row.is_current_in(documents),
            Declaration::Class(row) => documents
                .iter()
                .any(|document| row.is_current(document.uri, document.source, document.analysis)),
        }
    }
}

/// A declaration-owned lens plus its presentation, before host publication.
#[derive(Debug, Clone)]
pub struct LogicalSourceLens {
    identity: LogicalLensIdentity,
    range: crate::definition::LspRange,
    label: String,
}
impl LogicalSourceLens {
    /// Original selection retained independently of presentation.
    #[must_use]
    pub const fn identity(&self) -> &LogicalLensIdentity {
        &self.identity
    }
    /// Source-owner-issued name geometry; no client range is consumed.
    #[must_use]
    pub const fn range(&self) -> crate::definition::LspRange {
        self.range
    }
    /// A display label only, never an input to selection.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Enumerate genuine source declarations without final report-map selection.
/// Property/lifecycle and other unrepresented declaration roles remain outside
/// this positive catalogue, which proves no complete declaration census.
#[must_use]
pub fn logical_source_lenses(
    uri: &str,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<Vec<LogicalSourceLens>> {
    let document = documents.iter().find(|document| document.uri == uri)?;
    crate::original_context::CurrentSourceContext::capture(document.source, document.analysis)?;
    document
        .analysis
        .allows_retained_logical_declaration_advice()
        .then_some(())?;
    let items = crate::call_hierarchy::declared_logical_source_items(uri, documents)?;
    let mut lenses = items
        .iter()
        .map(|item| LogicalSourceLens {
            identity: LogicalLensIdentity {
                declaration: Declaration::Callable(Box::new(item.identity().clone())),
            },
            range: item.presentation().selection_range,
            label: item.presentation().name.clone(),
        })
        .collect::<Vec<_>>();
    let lines = LineIndex::new_lsp(document.source);
    for class in document.analysis.all_classes.values() {
        let Some(identity) = crate::type_hierarchy::LogicalHierarchyClass::capture(
            uri,
            document.source,
            document.analysis,
            class,
        ) else {
            continue;
        };
        lenses.push(LogicalSourceLens {
            identity: LogicalLensIdentity {
                declaration: Declaration::Class(Box::new(identity)),
            },
            range: crate::definition::span_to_range(document.source, &lines, class.name_span),
            label: class.qualified_name.clone(),
        });
    }
    Some(lenses)
}

#[derive(Debug, Clone)]
struct ClassOccurrence {
    uri: String,
    call: tcl_compiler::command_binding::OriginalSourceConstructorCall,
}

/// Partial positive readonly occurrences and their original source evidence.
/// Possible calls are only a subset of references; unsupported captures,
/// property/lifecycle and foreign class providers remain explicit debt.
#[derive(Debug, Clone)]
pub struct LogicalLensReferenceAdvice {
    identity: LogicalLensIdentity,
    calls: Option<crate::call_hierarchy::LogicalCallAdvice>,
    classes: Vec<ClassOccurrence>,
    targets: Vec<(String, crate::definition::LspRange)>,
}
impl LogicalLensReferenceAdvice {
    /// All results remain partial; zero never proves no references.
    #[must_use]
    pub const fn coverage(&self) -> crate::original_declaration::ReadonlyReferenceCoverage {
        crate::original_declaration::ReadonlyReferenceCoverage::Partial
    }
    /// Possible-positive source locations, projected only inside the host lease.
    #[must_use]
    pub fn possible_source_targets(&self) -> &[(String, crate::definition::LspRange)] {
        &self.targets
    }
    /// Reissue each source occurrence and independently retained declaration.
    #[must_use]
    pub fn is_current(&self, documents: &[OriginalDeclarationDocument<'_>]) -> bool {
        if !self.identity.is_current_in(documents)
            || self
                .calls
                .as_ref()
                .is_some_and(|calls| !calls.is_current(documents))
        {
            return false;
        }
        self.classes.iter().all(|occurrence| {
            let Some(document) = documents
                .iter()
                .find(|document| document.uri == occurrence.uri)
            else {
                return false;
            };
            let Some(head) = occurrence.call.original_words().first() else {
                return false;
            };
            tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                document.source,
                document.analysis,
                head.span().start(),
            )
            .as_ref()
                == Some(&occurrence.call)
        })
    }
}

/// Join retained declarations to genuine possible-call and class-command source
/// occurrences. This does not replace the independent complete edit/reference
/// inventories and does not label the call subset as an exhaustive reference set.
#[must_use]
pub fn logical_lens_reference_advice(
    identity: &LogicalLensIdentity,
    documents: &[OriginalDeclarationDocument<'_>],
) -> Option<LogicalLensReferenceAdvice> {
    identity.is_current_in(documents).then_some(())?;
    let mut report = LogicalLensReferenceAdvice {
        identity: identity.clone(),
        calls: None,
        classes: Vec::new(),
        targets: Vec::new(),
    };
    match &identity.declaration {
        Declaration::Callable(call) => {
            let advice = crate::call_hierarchy::logical_incoming_from_inventory(call, documents)?;
            for (owner, span) in
                crate::call_hierarchy::LogicalCallAdvice::possible_reference_name_targets(&advice)?
            {
                let document = documents.iter().find(|document| document.uri == owner)?;
                report.targets.push((
                    owner,
                    crate::definition::span_to_range(
                        document.source,
                        &LineIndex::new_lsp(document.source),
                        span,
                    ),
                ));
            }
            report.calls = Some(advice);
        }
        Declaration::Class(class) => {
            // The existing constructor/class-command owner reborrows local
            // factories. Equal class bytes in another URI do not donate a target.
            let document = documents
                .iter()
                .find(|document| document.uri == class.uri())?;
            let target = crate::type_hierarchy::LogicalHierarchyClass::class(
                class,
                document.source,
                document.analysis,
            )?;
            let coverage = crate::source_structure::SourceSyntaxCoverage::capture(
                document.source,
                document.analysis,
            )?;
            let walk = crate::refactor::FrameWalk::new(document.source, document.analysis)?;
            let lines = LineIndex::new_lsp(document.source);
            for command in coverage.commands(document.source, document.analysis)? {
                let Some(words) =
                    crate::refactor::FrameWalk::native_words(&walk, document.source, command)
                else {
                    continue;
                };
                let Some(head) = words.first() else {
                    continue;
                };
                let Some(call) =
                    tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                        document.source,
                        document.analysis,
                        head.span().start(),
                    )
                else {
                    continue;
                };
                if call.original_words() != words.as_slice()
                    || !call
                        .class_declaration()
                        .logical_source_class(document.analysis)
                        .is_some_and(|selected| std::ptr::eq(selected, target))
                {
                    continue;
                }
                report.targets.push((
                    document.uri.to_owned(),
                    crate::definition::span_to_range(document.source, &lines, head.span()),
                ));
                report.classes.push(ClassOccurrence {
                    uri: document.uri.to_owned(),
                    call,
                });
            }
        }
    }
    report.targets.sort_by(|a, b| {
        (
            a.0.as_str(),
            a.1.start_line,
            a.1.start_character,
            a.1.end_line,
            a.1.end_character,
        )
            .cmp(&(
                b.0.as_str(),
                b.1.start_line,
                b.1.start_character,
                b.1.end_line,
                b.1.end_character,
            ))
    });
    report.targets.dedup();
    report.is_current(documents).then_some(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn lens_headers_keep_superseded_allocations_and_independent_uri_owners() {
        let source = "proc p {} {}\nproc p {} {return second}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let documents = [OriginalDeclarationDocument {
            uri: "file:///a.tcl",
            source,
            analysis: &analysis,
        }];
        let lenses = logical_source_lenses(documents[0].uri, &documents).unwrap();
        assert_eq!(lenses.len(), 2);
        assert_ne!(lenses[0].identity(), lenses[1].identity());
        assert_ne!(lenses[0].range(), lenses[1].range());
        assert!(
            lenses
                .iter()
                .all(|lens| lens.identity().is_current_in(&documents))
        );
        let foreign = [OriginalDeclarationDocument {
            uri: "file:///b.tcl",
            source,
            analysis: &analysis,
        }];
        assert!(
            lenses
                .iter()
                .all(|lens| !lens.identity().is_current_in(&foreign))
        );
        let duplicate = [documents[0], documents[0]];
        assert!(!lenses[0].identity().is_current_in(&duplicate));
        assert!(logical_source_lenses(documents[0].uri, &duplicate).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        let native_documents = [OriginalDeclarationDocument {
            uri: documents[0].uri,
            source,
            analysis: &native,
        }];
        assert!(logical_source_lenses(documents[0].uri, &native_documents).is_none());
        let other = logical_source_lenses(foreign[0].uri, &foreign).unwrap();
        assert_ne!(lenses[0].identity(), other[0].identity());
        assert!(
            !lenses[0]
                .identity()
                .is_current(documents[0].uri, "proc p {} {}", &analysis)
        );
    }

    #[test]
    fn genuine_call_sites_are_partial_reference_evidence_and_stale_source_refuses() {
        let source = "proc helper {} {}\nproc caller {} {helper}\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let documents = [OriginalDeclarationDocument {
            uri: "file:///calls.tcl",
            source,
            analysis: &analysis,
        }];
        let lenses = logical_source_lenses(documents[0].uri, &documents).unwrap();
        let helper = lenses
            .iter()
            .find(|lens| lens.range().start_line == 0)
            .unwrap();
        let report = logical_lens_reference_advice(helper.identity(), &documents).unwrap();
        assert_eq!(report.possible_source_targets().len(), 1);
        assert_eq!(report.possible_source_targets()[0].0, documents[0].uri);
        assert_eq!(report.possible_source_targets()[0].1.start_line, 1);
        assert_eq!(
            report.coverage(),
            crate::original_declaration::ReadonlyReferenceCoverage::Partial
        );
        assert!(report.is_current(&documents));
        let changed = "proc helper {} {}\nproc caller {} {other}\n";
        let changed_analysis = Analyser::new().analyse(changed, "tcl");
        let current = [OriginalDeclarationDocument {
            uri: documents[0].uri,
            source: changed,
            analysis: &changed_analysis,
        }];
        assert!(!report.is_current(&current));
        assert!(logical_lens_reference_advice(helper.identity(), &current).is_none());
    }
}

#[cfg(test)]
mod class_reference_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn class_call_keeps_actual_local_factory_and_does_not_borrow_equal_foreign_bytes() {
        let source = "oo::class create C {method m {} {}}\nC create item\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let documents = [
            OriginalDeclarationDocument {
                uri: "file:///a.tcl",
                source,
                analysis: &analysis,
            },
            OriginalDeclarationDocument {
                uri: "file:///b.tcl",
                source,
                analysis: &analysis,
            },
        ];
        let lenses = logical_source_lenses(documents[0].uri, &documents).unwrap();
        let class = lenses
            .iter()
            .find(|lens| matches!(&lens.identity.declaration, Declaration::Class(_)))
            .unwrap();
        let report = logical_lens_reference_advice(class.identity(), &documents).unwrap();
        assert!(report.is_current(&documents));
        assert_eq!(
            report.coverage(),
            crate::original_declaration::ReadonlyReferenceCoverage::Partial
        );
        assert_eq!(report.possible_source_targets().len(), 1);
        assert_eq!(report.possible_source_targets()[0].0, documents[0].uri);
        assert_eq!(report.possible_source_targets()[0].1.start_line, 1);
        let only_foreign = [documents[1]];
        assert!(!class.identity().is_current_in(&only_foreign));
        assert!(logical_lens_reference_advice(class.identity(), &only_foreign).is_none());
    }
}
