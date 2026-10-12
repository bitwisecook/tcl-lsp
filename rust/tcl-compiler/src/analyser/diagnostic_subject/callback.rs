// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Callback signature advice from retained source prefix and declaration headers.

use crate::analyser::{ItemKind, SourceDeclarationSignature};
use crate::command_binding::{OriginalCallbackPrefix, OriginalSourceCallbackProcedureTarget};
use crate::signature_scan::formal_count::SourceFormalCountOrigin;
use std::sync::Arc;
use tcl_core_types::DiagCode;
use tcl_lexer::{SourceChannel, Span};

/// Complete suffix-count contract plus baked prefix words. These are source
/// descriptor counts, not an actual callback argv or entered frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCallbackArgumentCounts {
    /// Every finite alternative must fit at least one source declaration.
    Finite(Vec<usize>),
    /// The receiver's suffix has no known upper bound.
    AtLeast(usize),
}

/// Exact source-signature incompatibility selected by the emitting owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCallbackArityIssue {
    /// Every retained source signature requires more arguments.
    TooFew {
        /// Effective source argument count that cannot fit a retained header.
        supplied: usize,
        /// Smallest lower bound across the retained source headers.
        expected_minimum: usize,
    },
    /// Every retained source signature accepts fewer arguments.
    TooMany {
        /// Effective source argument count that cannot fit a retained header.
        supplied: usize,
        /// Greatest finite upper bound across the retained source headers.
        expected_maximum: usize,
    },
    /// A finite suffix count lies between disjoint signature alternatives.
    NoCompatibleSignature {
        /// Effective source argument count between retained signature alternatives.
        supplied: usize,
    },
}
impl SourceCallbackArityIssue {
    /// Stable diagnostic classification, independent of message presentation.
    #[must_use]
    pub const fn code(self) -> DiagCode {
        match self {
            Self::TooFew { .. } => DiagCode::E002,
            Self::TooMany { .. } => DiagCode::E003,
            Self::NoCompatibleSignature { .. } => DiagCode::E005,
        }
    }
}

/// Body-free local signature and the genuine registration-horizon lookup.
/// Refusals remain explicit so external headers cannot bypass source barriers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCallbackSignatureLookup {
    prefix: Arc<OriginalCallbackPrefix>,
    original: Arc<crate::command_binding::OriginalSourceCallbackProcedureLookup>,
    declaration: Option<SourceDeclarationSignature>,
}
impl SourceCallbackSignatureLookup {
    /// Join the authentic target to its own canonical original declaration.
    /// Neither current UI maps nor cross-file name/offset pairs enter this join.
    #[must_use]
    pub fn from_original_lookup(
        analysis: &crate::analyser::AnalysisResult,
        prefix: Arc<OriginalCallbackPrefix>,
        original: Arc<crate::command_binding::OriginalSourceCallbackProcedureLookup>,
    ) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let container = prefix.name_input().original_static_list_container()?;
        let parent = container.parent_word();
        if parent.config() != input.lexer_config()
            || !analysis.matches_original_source_image(parent.image(), input.lexer_config())
            || !original.matches_source_context(
                parent.image(),
                input.lexer_config(),
                &input.context_registry(),
            )
        {
            return None;
        }
        let retained = original.prefix();
        if prefix
            .source_registration()
            .is_some_and(|registration| registration != original.registration())
            || retained.name_input() != prefix.name_input()
            || retained.scope() != prefix.scope()
            || retained.appended_arity() != prefix.appended_arity()
            || retained.baked_argument_count() != prefix.baked_argument_count()
            || prefix
                .lookup()
                .is_some_and(|lookup| lookup.site() != original.registration().site())
        {
            return None;
        }
        let declaration = original
            .target()
            .and_then(|target| target.source_procedure(analysis))
            .map(SourceDeclarationSignature::from_original_procedure);
        Some(Self {
            prefix,
            original,
            declaration,
        })
    }
    /// Captured alias count under the shared argv owner, independently of the
    /// callback's baked words and appended suffix. Expansion remains uncertain.
    #[must_use]
    pub fn captured_argument_count(&self) -> Option<tcl_registry::InvocationArgumentCount> {
        let arguments = self.original.target().map_or_else(
            Vec::new,
            OriginalSourceCallbackProcedureTarget::captured_arguments,
        );
        let words = arguments
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        tcl_registry::resolved_invocation::count_invocation_argv(
            tcl_registry::InvocationArguments::structured(&words),
            0,
        )
    }
    /// Positioned registration-horizon source target or structured refusal.
    #[must_use]
    pub fn original(&self) -> &crate::command_binding::OriginalSourceCallbackProcedureLookup {
        &self.original
    }
    /// Genuine original readonly prefix used by the receiving invocation.
    #[must_use]
    pub fn prefix(&self) -> &OriginalCallbackPrefix {
        &self.prefix
    }
    /// Local header projected from the target's own original declaration.
    #[must_use]
    pub const fn declaration(&self) -> Option<&SourceDeclarationSignature> {
        self.declaration.as_ref()
    }
}

/// Conditional callback signature advice. Current source declarations and
/// callback lookup purpose cannot prove registration, future command occupancy,
/// native argument activation, callback entry or normal completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCallbackAritySubject {
    prefix: Arc<OriginalCallbackPrefix>,
    declarations: Vec<SourceDeclarationSignature>,
    source_lookup: Option<Arc<SourceCallbackSignatureLookup>>,
    counts: SourceCallbackArgumentCounts,
    issue: SourceCallbackArityIssue,
    span: Span,
    source_channel: SourceChannel,
}
impl SourceCallbackAritySubject {
    /// Select exact current source headers with the original prefix's ordered
    /// lookup. Unknown counts, missing geometry and non-procedure candidates
    /// refuse assistance; display labels and nominal tails never enter the join.
    #[must_use]
    pub fn from_source_signatures(
        prefix: Arc<OriginalCallbackPrefix>,
        headers: &[SourceDeclarationSignature],
    ) -> Option<Self> {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        let lookup = prefix.lookup()?;
        if lookup.name_input() != prefix.name_input()
            || lookup.callback_lookup_scope() != prefix.scope()
        {
            return None;
        }
        let declarations = lookup
            .matching_publications(headers.iter().map(|header| (header.name(), header.clone())))?;
        Self::from_selected_signatures(prefix, declarations, 0, false, None)
    }
    /// Assess the proper conditional alias target before external header advice.
    /// Known source barriers and unavailable callback frames remain terminal.
    #[must_use]
    pub fn from_source_lookup(
        selection: Arc<SourceCallbackSignatureLookup>,
        external_headers: &[SourceDeclarationSignature],
    ) -> Option<Self> {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        use crate::command_binding::OriginalSourceCallbackProcedureTargetKind as Kind;
        let original = selection.original();
        let (declarations, captured, indeterminate) = if let Some(target) = original.target() {
            let declarations = match target.kind() {
                Kind::LocalProcedure => vec![selection.declaration()?.clone()],
                Kind::ExternalSourceName => {
                    let policy = target.target_input().native_input()?.policy();
                    crate::signature_scan::scope::first_matching_byte_publications(
                        policy,
                        std::slice::from_ref(target.source_slot()),
                        external_headers
                            .iter()
                            .map(|header| (header.name(), header.clone())),
                    )
                }
            };
            let count = selection.captured_argument_count()?;
            (
                declarations,
                usize::from(count.minimum),
                count.indeterminate,
            )
        } else if original.permits_external_signature_lookup() {
            let lookup = selection.prefix().lookup()?;
            let declarations = lookup.matching_publications(
                external_headers
                    .iter()
                    .map(|header| (header.name(), header.clone())),
            )?;
            (declarations, 0, false)
        } else {
            return None;
        };
        Self::from_selected_signatures(
            Arc::clone(&selection.prefix),
            declarations,
            captured,
            indeterminate,
            Some(selection),
        )
    }
    fn from_selected_signatures(
        prefix: Arc<OriginalCallbackPrefix>,
        declarations: Vec<SourceDeclarationSignature>,
        captured: usize,
        indeterminate: bool,
        source_lookup: Option<Arc<SourceCallbackSignatureLookup>>,
    ) -> Option<Self> {
        if declarations.is_empty()
            || declarations.iter().any(|header| {
                let formal = header.formal_count_projection();
                header.kind() != ItemKind::Proc
                    || formal.origin() != SourceFormalCountOrigin::OriginalSource
                    || formal.parameter_grammar().is_none()
            })
        {
            return None;
        }
        let appended = prefix.appended_arity()?;
        if !appended.is_checkable() {
            return None;
        }
        let fixed = prefix.baked_argument_count().checked_add(captured)?;
        let counts = if !indeterminate && let Some(exact) = appended.exact_counts() {
            SourceCallbackArgumentCounts::Finite(
                exact
                    .map(|count| fixed.checked_add(usize::from(count)))
                    .collect::<Option<Vec<_>>>()?,
            )
        } else {
            SourceCallbackArgumentCounts::AtLeast(fixed.checked_add(usize::from(appended.min()))?)
        };
        let issue = signature_issue(&counts, &declarations)?;
        let input = prefix.name_input();
        let container = input.original_static_list_container()?;
        let span = input
            .original_static_value_source_extent(0..input.bytes().len())
            .unwrap_or_else(|| container.parent_word().span());
        let source_channel = container.parent_word().image().channel();
        Some(Self {
            prefix,
            declarations,
            source_lookup,
            counts,
            issue,
            span,
            source_channel,
        })
    }
    /// Genuine readonly prefix; it grants no writable complete child word.
    #[must_use]
    pub fn prefix(&self) -> &OriginalCallbackPrefix {
        &self.prefix
    }
    /// Body-free original declarations selected by the retained source lookup.
    #[must_use]
    pub fn declarations(&self) -> &[SourceDeclarationSignature] {
        &self.declarations
    }
    /// Retained registration-horizon alias target, capture and source obligations.
    /// Its absence denotes independently selected direct source-header advice.
    #[must_use]
    pub fn source_lookup(&self) -> Option<&SourceCallbackSignatureLookup> {
        self.source_lookup.as_deref()
    }
    /// Effective source descriptor counts including all baked words.
    #[must_use]
    pub const fn argument_counts(&self) -> &SourceCallbackArgumentCounts {
        &self.counts
    }
    /// Exact mismatch computed before presentation is constructed.
    #[must_use]
    pub const fn issue(&self) -> SourceCallbackArityIssue {
        self.issue
    }
    /// Original readonly head extent, or its whole authentic parent operand
    /// when decoding prevents a child substring. This is not an edit span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Original source channel, independently of the message or output protocol.
    #[must_use]
    pub const fn source_channel(&self) -> SourceChannel {
        self.source_channel
    }
}
fn signature_issue(
    counts: &SourceCallbackArgumentCounts,
    declarations: &[SourceDeclarationSignature],
) -> Option<SourceCallbackArityIssue> {
    let minimum = declarations
        .iter()
        .map(|header| usize::from(header.formal_count_projection().arity().min))
        .min()?;
    let maximum = declarations
        .iter()
        .map(|header| {
            let arity = header.formal_count_projection().arity();
            if arity.is_unlimited() {
                usize::MAX
            } else {
                usize::from(arity.max)
            }
        })
        .max()?;
    match counts {
        SourceCallbackArgumentCounts::Finite(counts) => {
            let supplied = counts.iter().copied().find(|&count| {
                !declarations.iter().any(|header| {
                    let arity = header.formal_count_projection().arity();
                    usize::from(arity.min) <= count
                        && (arity.is_unlimited() || count <= usize::from(arity.max))
                })
            })?;
            Some(if supplied < minimum {
                SourceCallbackArityIssue::TooFew {
                    supplied,
                    expected_minimum: minimum,
                }
            } else if supplied > maximum {
                SourceCallbackArityIssue::TooMany {
                    supplied,
                    expected_maximum: maximum,
                }
            } else {
                SourceCallbackArityIssue::NoCompatibleSignature { supplied }
            })
        }
        SourceCallbackArgumentCounts::AtLeast(supplied) if *supplied > maximum => {
            Some(SourceCallbackArityIssue::TooMany {
                supplied: *supplied,
                expected_maximum: maximum,
            })
        }
        SourceCallbackArgumentCounts::AtLeast(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, ItemTree};

    #[test]
    fn original_callback_subject_retains_prefix_counts_headers_and_source_purpose() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        // Source assistance only: lsort/trace are not executed here.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let suffix = "lsort -command {cb fixed} {3 1 2}";
            let source = format!("proc cb {{a b}} {{return 0}}\nproc target {{}} {{}}\n{suffix}");
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(&source, dialect);
            let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
                .sigs()
                .into_iter()
                .filter_map(|sig| sig.original_declaration)
                .collect::<Vec<_>>();
            let invocation = analysis
                .command_invocations
                .iter()
                .find(|inv| {
                    inv.original_callback_prefix
                        .as_ref()
                        .is_some_and(|prefix| prefix.name_input().bytes() == b"cb")
                })
                .expect(dialect);
            let prefix = Arc::clone(invocation.original_callback_prefix.as_ref().unwrap());
            let subject = SourceCallbackAritySubject::from_source_signatures(prefix, &headers)
                .expect(dialect);
            assert_eq!(
                subject.argument_counts(),
                &SourceCallbackArgumentCounts::Finite(vec![3])
            );
            assert_eq!(subject.issue().code(), DiagCode::E003);
            assert_eq!(subject.declarations().len(), 1);
            assert!(subject.prefix().name_input().original_word_key().is_none());
            assert_eq!(&source[subject.span().as_range()], "cb");
            let mut changed = invocation.clone();
            changed.name = "different display".into();
            changed.range = Span::new(0, 1);
            changed.callback_baked_args = usize::MAX;
            changed.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
            let retained = SourceCallbackAritySubject::from_source_signatures(
                Arc::clone(changed.original_callback_prefix.as_ref().unwrap()),
                &headers,
            )
            .unwrap();
            assert_eq!(
                retained, subject,
                "mutable reporting fields cannot issue source facts"
            );
        }
    }
    #[test]
    fn original_absolute_trace_subject_retains_source_signature_without_trigger_frame() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        // Qualification supplies target geometry only. No triggering frame,
        // installed trace, future command occupancy or actual argv is observed.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "proc ::cb {a b c} {}\ntrace add execution target {enter leave} ::cb";
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, dialect);
            let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
                .sigs()
                .into_iter()
                .filter_map(|sig| sig.original_declaration)
                .collect::<Vec<_>>();
            let prefix = analysis
                .command_invocations
                .iter()
                .find_map(|inv| inv.original_callback_prefix.as_ref())
                .expect(dialect);
            assert_eq!(
                prefix.scope(),
                Some(tcl_registry::ScriptLookupScope::TriggerFrame)
            );
            assert!(prefix.lookup().unwrap().original_naming_scope().is_none());
            let subject =
                SourceCallbackAritySubject::from_source_signatures(Arc::clone(prefix), &headers)
                    .unwrap();
            assert_eq!(
                subject.argument_counts(),
                &SourceCallbackArgumentCounts::Finite(vec![2, 4])
            );
            assert_eq!(
                subject.issue(),
                SourceCallbackArityIssue::TooFew {
                    supplied: 2,
                    expected_minimum: 3
                }
            );
        }
    }

    #[test]
    fn original_callback_subject_withdraws_unknown_counts_and_missing_frame() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        for source in [
            "proc cb $params {}; lsort -command cb {1 2}",
            "oo::class create cb {}; lsort -command cb {1 2}",
            "proc cb {a b} {}; trace add variable x write cb",
            "proc cb {a b} {}; trace add execution target {enter leave} cb",
            "proc cb {a b} {}; lsort -command cb {1 2}",
            "proc cb {a b c} {}; fcopy $in $out -command cb",
        ] {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, "tcl9.0");
            let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
                .sigs()
                .into_iter()
                .filter_map(|sig| sig.original_declaration)
                .collect::<Vec<_>>();
            assert!(
                analysis
                    .command_invocations
                    .iter()
                    .filter_map(|inv| inv.original_callback_prefix.as_ref())
                    .all(|prefix| SourceCallbackAritySubject::from_source_signatures(
                        Arc::clone(prefix),
                        &headers
                    )
                    .is_none()),
                "{source}"
            );
        }
    }
    fn sorting_signature_lookup(
        analysis: &crate::analyser::AnalysisResult,
    ) -> &Arc<SourceCallbackSignatureLookup> {
        analysis
            .command_invocations
            .iter()
            .find_map(|row| {
                row.original_callback_signature_lookup
                    .as_ref()
                    .filter(|selection| {
                        selection.prefix().appended_arity()
                            == Some(tcl_registry::AppendedArity::Exactly(2))
                    })
            })
            .expect("authentic sorting callback source lookup")
    }

    #[test]
    fn original_callback_alias_signature_composes_separate_capture_counts() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        // Source signature control; native public args/order are independently recorded.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let alias = if dialect == "jimtcl" {
                "alias cb target FIXED"
            } else {
                "interp alias {} cb {} target FIXED"
            };
            let source =
                format!("proc target {{a b c d}} {{}}\n{alias}\nlsort -command cb {{2 1}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let selection = sorting_signature_lookup(&analysis);
            let subject =
                SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
            assert_eq!(
                subject.argument_counts(),
                &SourceCallbackArgumentCounts::Finite(vec![3]),
                "{dialect}"
            );
            assert_eq!(subject.prefix().baked_argument_count(), 0);
            assert_eq!(
                subject.issue(),
                SourceCallbackArityIssue::TooFew {
                    supplied: 3,
                    expected_minimum: 4
                }
            );
            assert_eq!(selection.captured_argument_count().unwrap().minimum, 1);
            assert_eq!(
                subject.declarations()[0].name().slot().simple.as_bytes(),
                b"target"
            );
            assert_eq!(
                selection
                    .original()
                    .target()
                    .unwrap()
                    .argument_input(0)
                    .unwrap()
                    .bytes(),
                b"FIXED"
            );
        }
    }

    #[test]
    fn original_callback_alias_signature_retains_nested_and_baked_premises() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let aliases = if dialect == "jimtcl" {
                "alias inner target INNER\nalias outer inner OUTER"
            } else {
                "interp alias {} inner {} target INNER\ninterp alias {} outer {} inner OUTER"
            };
            let source = format!(
                "proc target {{a b c d}} {{}}\n{aliases}\nlsort -command {{outer BAKED}} {{2 1}}"
            );
            let analysis = Analyser::new().analyse(&source, dialect);
            let selection = sorting_signature_lookup(&analysis);
            let subject =
                SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
            assert_eq!(subject.prefix().baked_argument_count(), 1);
            assert_eq!(
                subject.prefix().appended_arity(),
                Some(tcl_registry::AppendedArity::Exactly(2))
            );
            assert_eq!(selection.captured_argument_count().unwrap().minimum, 2);
            assert_eq!(
                subject.argument_counts(),
                &SourceCallbackArgumentCounts::Finite(vec![5])
            );
            let target = selection.original().target().unwrap();
            assert_eq!(target.argument_input(0).unwrap().bytes(), b"INNER");
            assert_eq!(target.argument_input(1).unwrap().bytes(), b"OUTER");
            assert_eq!(target.lineage().len(), 2);
            assert!(target.obligations().contains(&crate::command_binding::SourceCommandTransitionObligation::OriginalCallbackTargetApplicability));
        }
    }

    #[test]
    fn original_callback_alias_signature_keeps_alias_move_and_target_replacement() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for tail in [
            "rename cb moved_cb\nlsort -command moved_cb {2 1}",
            "rename target moved_target\nproc target {a b c d} {}\nlsort -command cb {2 1}",
        ] {
            let source = format!(
                "proc target {{a b c d}} {{}}\ninterp alias {{}} cb {{}} target FIXED\n{tail}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let selection = sorting_signature_lookup(&analysis);
            let subject =
                SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &[]).unwrap();
            assert_eq!(
                subject.argument_counts(),
                &SourceCallbackArgumentCounts::Finite(vec![3])
            );
            let expected = if tail.starts_with("rename target") {
                source.rfind("proc target").unwrap()
            } else {
                0
            };
            assert_eq!(
                subject.declarations()[0].declaration_offset(),
                u32::try_from(expected).unwrap()
            );
            assert_eq!(
                subject.declarations()[0].name().slot().simple.as_bytes(),
                b"target"
            );
        }
    }

    #[test]
    fn original_callback_source_barriers_cannot_resurrect_project_headers() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for (prefix, tail) in [
            (
                "proc cb {a b c d} {}",
                "rename cb {}\nlsort -command cb {2 1}",
            ),
            (
                "proc target {a b c d} {}\ninterp alias {} cb {} target FIXED",
                "rename target moved_target\nlsort -command cb {2 1}",
            ),
            (
                "proc target {a b c d} {}\ninterp alias {} cb {} target FIXED",
                "rename target {}\nlsort -command cb {2 1}",
            ),
            (
                "proc target {a b c d} {}\ninterp alias {} cb {} target FIXED",
                "rename target {}\ninterp alias {} target {} cb\nlsort -command cb {2 1}",
            ),
        ] {
            let source = format!("{prefix}\n{tail}");
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(&source, "tcl9.0");
            let headers = ItemTree::from_analysis(&analysis, &analyser.ensemble_namespaces)
                .sigs()
                .into_iter()
                .filter_map(|sig| sig.original_declaration)
                .collect::<Vec<_>>();
            let selection = sorting_signature_lookup(&analysis);
            assert_eq!(selection.original().refusal(), Some(crate::command_binding::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier), "{source}");
            assert!(!selection.original().permits_external_signature_lookup());
            assert!(
                SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &headers)
                    .is_none()
            );
        }
    }
    #[test]
    fn original_callback_signature_lookup_requires_its_complete_source_input() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let source =
            "proc target {a b c d} {}\ninterp alias {} cb {} target FIXED\nlsort -command cb {2 1}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let selection = sorting_signature_lookup(&analysis);
        let prefix = Arc::new(selection.prefix().clone());
        let original = Arc::new(selection.original().clone());
        assert!(
            SourceCallbackSignatureLookup::from_original_lookup(
                &analysis,
                Arc::clone(&prefix),
                Arc::clone(&original)
            )
            .is_some()
        );
        let changed = Analyser::new().analyse(&format!("{source}\n"), "tcl9.0");
        assert!(
            SourceCallbackSignatureLookup::from_original_lookup(
                &changed,
                Arc::clone(&prefix),
                Arc::clone(&original)
            )
            .is_none()
        );
        let other_dialect = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            SourceCallbackSignatureLookup::from_original_lookup(&other_dialect, prefix, original)
                .is_none()
        );
    }
}
