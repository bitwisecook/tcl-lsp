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

//! Source declaration reference lenses.
//!
//! Native source analysis enumerates genuine declarations through
//! `original_declaration::declarations` and retains each sealed readonly
//! identity. Local counts use the same canonical original reference kernel;
//! indexed command counts remain retained source advice. The server resolves
//! the identity against independently current participating documents before
//! publishing its reference locations and final count. Optional labels and
//! `qname` fields are presentation and cannot reissue the identity.
//!
//! A source declaration or count supplies no native dispatch, object lifetime
//! or edit permission. Missing complete source/configuration correspondence
//! withdraws original advice. Equal bytes in independent URI owners remain
//! independent candidates.
//!
//! Logical source lenses enumerate actual retained procedure headers (including
//! superseded declarations), completed class snapshots and original member
//! vectors. Their distinct opaque identity survives the Server's actual source,
//! input, provider, publication and index lease. Display labels and old client
//! ranges never select a declaration. Reference reports retain partial positive
//! call/class-command occurrences; unsupported property/lifecycle, captured and
//! noncall references remain debt rather than a complete reference census.
//!
//! Every clickable resolved lens uses the same location set for its title and
//! reference command. [`reference_count_title`] renders the count only.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

mod logical_source;
pub use logical_source::{
    LogicalLensIdentity, LogicalLensReferenceAdvice, LogicalSourceLens,
    logical_lens_reference_advice, logical_source_lenses,
};

/// One code-lens entry — anchor range plus a command label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLens {
    /// Independent current Logical source selection; never a Native identity.
    pub logical_identity: Option<LogicalLensIdentity>,
    /// Sealed readonly declaration for current source and wire resolution.
    pub identity: Option<crate::original_declaration::OriginalDeclarationIdentity>,
    /// Conditional source-reference availability, separate from its positive count.
    pub reference_coverage: Option<crate::original_declaration::ReadonlyReferenceCoverage>,
    /// Anchor range for the lens.
    pub range: LspRange,
    /// Command label shown to the user.
    pub command_title: String,
    /// Command identifier sent on click.
    pub command: String,
    /// Qualified name of the symbol the lens annotates (`::greet`),
    /// surfaced in the LSP lens `data` for the references-jump command.
    pub qname: String,
}

/// Render positive counts without presenting an unknown source census as absence.
#[must_use]
pub fn readonly_reference_count_title(
    count: usize,
    coverage: crate::original_declaration::ReadonlyReferenceCoverage,
) -> String {
    use crate::original_declaration::ReadonlyReferenceCoverage;
    match coverage {
        ReadonlyReferenceCoverage::ExistingInventory
        | ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus => reference_count_title(count),
        ReadonlyReferenceCoverage::Partial if count > 0 => {
            format!("at least {}", reference_count_title(count))
        }
        ReadonlyReferenceCoverage::Partial | ReadonlyReferenceCoverage::Unavailable => {
            "References unavailable".to_owned()
        }
    }
}

/// Compute code lenses for the document.
///
/// `analysis` is the analyser result for the document; when
/// `None`, returns an empty vector (preserves the stub call
/// shape for callers that haven't yet plumbed analysis
/// through).  `workspace` / `current_uri`, when provided, add
/// cross-document call sites to the proc / class reference
/// counts so the lens reflects workspace-wide usage.
#[must_use]
pub fn code_lenses(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: Option<&AnalysisResult>,
    workspace: Option<&crate::workspace_index::WorkspaceIndex>,
    current_uri: &str,
) -> Vec<CodeLens> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let _dialect = current.profile();
    if !analysis.allows_lexical_declaration_advice() {
        let Some(identities) =
            crate::original_declaration::declarations(current_uri, source, analysis)
        else {
            return Vec::new();
        };
        let index = LineIndex::new(source);
        return identities.into_iter().filter(|identity| !matches!(identity.role(),
            crate::original_declaration::OriginalDeclarationRole::Document))
            .filter_map(|identity| {
                let report = crate::original_declaration::readonly_reference_report(&identity, source, analysis, true)?;
                let mut count = report.spans().len();
                if let Some(workspace) = workspace {
                    let declaration = identity.procedure_metadata(analysis).map(|row| row.name())
                        .or_else(|| identity.class_metadata(analysis).filter(|_| identity.role() == crate::original_declaration::OriginalDeclarationRole::Class).map(|row| row.name()));
                    if let Some(name) = declaration {
                        let owners = workspace.original_procedure_declarations().map(|(_, row)| row.name())
                            .chain(workspace.original_class_declarations().map(|(_, row)| row.name()))
                            .filter(|row| *row == name).count();
                        if owners == 1 { count += workspace.original_invocations_of(name, current_uri, true).len(); }
                    }
                }
                Some(CodeLens { logical_identity: None, range: crate::definition::span_to_range(source, &index, identity.span()),
                    command_title: readonly_reference_count_title(count, report.coverage()), command: String::new(), qname: crate::original_declaration::OriginalDeclarationIdentity::reporting_key(&identity, analysis).unwrap_or_else(|| identity.label()), identity: Some(identity), reference_coverage: Some(report.coverage()) })
            }).collect();
    }
    let documents = [crate::original_declaration::OriginalDeclarationDocument {
        uri: current_uri,
        source,
        analysis,
    }];
    logical_source_lenses(current_uri, &documents)
        .into_iter()
        .flatten()
        .map(|lens| {
            let report = logical_lens_reference_advice(lens.identity(), &documents);
            let coverage = report.as_ref().map_or(
                crate::original_declaration::ReadonlyReferenceCoverage::Unavailable,
                LogicalLensReferenceAdvice::coverage,
            );
            let count = report
                .as_ref()
                .map_or(0, |report| report.possible_source_targets().len());
            CodeLens {
                logical_identity: Some(lens.identity().clone()),
                identity: None,
                reference_coverage: Some(coverage),
                range: lens.range(),
                command_title: readonly_reference_count_title(count, coverage),
                command: String::new(),
                qname: lens.label().to_owned(),
            }
        })
        .collect()
}

/// The lens label for `count` call sites — `"0 references"` /
/// `"1 reference"` / `"N references"`.
///
/// Public so the server can relabel a lens at `codeLens/resolve` time from
/// the workspace-wide site set it resolves there: the count
/// shown and the locations the click opens are then one number by
/// construction, not two independently-derived ones.
#[must_use]
pub fn reference_count_title(count: usize) -> String {
    match count {
        0 => "0 references".to_string(),
        1 => "1 reference".to_string(),
        n => format!("{n} references"),
    }
}

/// Match emitted labels and compatibility keys from current declaration records.
/// This presentation query grants no original declaration or invocation identity.
#[must_use]
pub fn lens_qname_exists(analysis: &AnalysisResult, qname: &str) -> bool {
    if !analysis.allows_lexical_declaration_advice() {
        return analysis
            .retained_command_realm()
            .and_then(|realm| realm.original_source_image())
            .and_then(|image| image.try_text().ok())
            .and_then(|source| crate::original_declaration::declarations("", source, analysis))
            .is_some_and(|identities| {
                identities.iter().any(|identity| {
                    identity.label() == qname
                        || identity.reporting_key(analysis).as_deref() == Some(qname)
                        || identity.class_reports_member_key(analysis, qname)
                })
            });
    }
    if analysis.all_procs.contains_key(qname) || analysis.all_classes.contains_key(qname) {
        return true;
    }
    analysis.all_classes.iter().any(|(class_q, class_def)| {
        class_def.methods.iter().any(|(name, member)| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_member_key(class_q, name, false) == qname
        }) || class_def.class_methods.iter().any(|(name, member)| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_member_key(class_q, name, true) == qname
        }) || class_def.properties.iter().any(|(name, member)| {
            !member.name_span.is_empty()
                && tcl_compiler::analyser::class_property_key(class_q, name) == qname
        }) || (class_def
            .constructors
            .last()
            .is_some_and(|member| !member.name_span.is_empty())
            && tcl_compiler::analyser::class_constructor_key(class_q) == qname)
            || (class_def
                .destructor
                .as_ref()
                .is_some_and(|member| !member.name_span.is_empty())
                && tcl_compiler::analyser::class_destructor_key(class_q) == qname)
    })
}

/// Count invocations targeting the given class — `ClassName
/// new ...`, `ClassName create instance`, and inheritance
/// references in `oo::class create Sub { superclass ClassName
/// ... }`.  Excludes the class's own `oo::class create
/// ClassName` declaration site.
#[cfg(test)]
mod readonly_source_count_tests {
    use super::*;
    use crate::original_declaration::ReadonlyReferenceCoverage;

    #[test]
    fn partial_source_lens_titles_do_not_report_unknown_absence() {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        assert_eq!(
            readonly_reference_count_title(2, ReadonlyReferenceCoverage::Partial),
            "at least 2 references"
        );
        assert_eq!(
            readonly_reference_count_title(1, ReadonlyReferenceCoverage::Partial),
            "at least 1 reference"
        );
        assert_eq!(
            readonly_reference_count_title(0, ReadonlyReferenceCoverage::Partial),
            "References unavailable"
        );
        assert_eq!(
            readonly_reference_count_title(3, ReadonlyReferenceCoverage::Unavailable),
            "References unavailable"
        );
        assert_eq!(
            readonly_reference_count_title(0, ReadonlyReferenceCoverage::ExistingInventory),
            "0 references"
        );
        assert_eq!(
            readonly_reference_count_title(2, ReadonlyReferenceCoverage::ExistingInventory),
            "2 references"
        );
    }

    #[test]
    fn complete_source_census_titles_report_exact_counts() {
        assert_eq!(
            readonly_reference_count_title(
                1,
                ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
            ),
            "1 reference"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    fn source_currency_analysis(
        source: &str,
        environment: &str,
        registry: std::sync::Arc<tcl_registry::CommandRegistry>,
    ) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                tcl_registry::model::ingress::resolve_environment(environment)
                    .default_context_registry()
                    .with_command_store(registry),
            ),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(analysis.allows_lexical_declaration_advice());
        analysis
    }

    #[test]
    fn original_logical_lenses_keep_complete_source_currency_and_actual_profile() {
        // naming.consumer.original-code-lens
        // docs/design/analysis/name-resolution-proofs/original-code-lens.md
        let source = "proc p {} {p}; p";
        let registry = std::sync::Arc::new(tcl_registry::CommandRegistry::build_default());
        let wrong_profile =
            tcl_registry::model::ingress::resolve_environment("f5-irules").analyser_profile();
        let lenses = |text: &str, analysis: &AnalysisResult| {
            code_lenses(
                text,
                wrong_profile,
                Some(analysis),
                None,
                "file:///source.tcl",
            )
        };
        for environment in ["tcl8.4", "tcl8.6"] {
            let analysis = source_currency_analysis(source, environment, registry.clone());
            let current = lenses(source, &analysis);
            assert_eq!(current.len(), 1);
            assert_eq!(current[0].qname, "::p");
            assert!(lenses("proc q {} {q}; q", &analysis).is_empty());
            let mut stale = analysis.clone();
            let config = stale.body_lexer_config.as_mut().unwrap();
            config.strict_quoting = !config.strict_quoting;
            assert!(lenses(source, &stale).is_empty());
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(!missing.all_procs.is_empty());
            assert!(lenses(source, &missing).is_empty());
            let mut unavailable = analysis.clone();
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: environment.into(),
                overlay: u64::MAX - 404,
            });
            assert!(lenses(source, &unavailable).is_empty());
            let mut foreign_store = tcl_registry::CommandRegistry::build_default();
            let mut descriptor = foreign_store.get("set").unwrap().clone();
            descriptor.name = "source_currency_pack_control";
            foreign_store.insert(descriptor);
            let donor =
                source_currency_analysis(source, environment, std::sync::Arc::new(foreign_store));
            let mut foreign = analysis.clone();
            foreign.resolved_input = donor.resolved_input;
            assert!(lenses(source, &foreign).is_empty());
            assert_eq!(lenses(source, &analysis).len(), 1);
        }
    }

    #[test]
    fn original_lenses_keep_repeated_and_opaque_source_declarations_without_ui_maps() {
        // Implementation contract: naming.consumer.original-code-lens
        // docs/design/analysis/name-resolution-proofs/original-code-lens.md
        let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; proc repeated {} {}; proc repeated {value} {}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let lenses = code_lenses(source, profile, Some(&analysis), None, "file:///lenses.tcl");
        assert_eq!(lenses.len(), 4);
        assert_ne!(lenses[0].identity, lenses[1].identity);
        assert_ne!(lenses[2].identity, lenses[3].identity);
        assert!(
            lenses
                .iter()
                .all(|lens| lens.identity.as_ref().unwrap().is_current(
                    "file:///lenses.tcl",
                    source,
                    &analysis
                ))
        );
        assert!(
            code_lenses(
                "proc changed {} {}",
                profile,
                Some(&analysis),
                None,
                "file:///lenses.tcl"
            )
            .is_empty()
        );
    }

    #[test]
    fn empty_lenses_when_analysis_is_none() {
        assert!(
            code_lenses(
                "proc foo {} {}\n",
                tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
                None,
                None,
                ""
            )
            .is_empty()
        );
    }

    #[test]
    fn lens_per_user_proc() {
        let src = "proc foo {} {}\nproc bar {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        assert_eq!(lenses.len(), 2, "{lenses:?}");
    }

    #[test]
    fn lens_shows_zero_references_for_unused_proc() {
        let src = "proc lonely {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].command_title, "0 references");
    }

    #[test]
    fn lens_shows_singular_for_one_reference() {
        let src = "proc helper {} {}\nhelper\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "1 reference");
    }

    #[test]
    fn lens_counts_multiple_references() {
        let src = "proc tool {} {}\ntool\ntool\ntool\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let tool = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("tool lens");
        assert_eq!(tool.command_title, "3 references");
    }

    #[test]
    fn lens_anchors_at_proc_name_span() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        assert_eq!(lenses.len(), 1);
        // `greet` starts at column 5 (after `proc `).
        assert_eq!(lenses[0].range.start_character, 5);
    }

    #[test]
    fn method_lens_counts_a_method_return_captured_dispatch_site() {
        // The lens count comes from the same resolver Find
        // References uses, so a `$b greet` site typed only by the lattice's
        // method-return edge must be counted too.
        let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let greet = lenses
            .iter()
            .find(|l| l.qname.contains("::method::greet"))
            .expect("greet method lens");
        assert_eq!(
            greet.command_title, "1 reference",
            "the lattice-typed `$b greet` site must be counted: {lenses:?}"
        );
    }

    // class lenses

    #[test]
    fn lens_per_class() {
        let src = "oo::class create Greeter {}\noo::class create Helper {}\n";
        let analysis = analyse(src);
        // This is a positive class-source control: missing class inventory
        // must fail rather than silently skip the lens assertions.
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let class_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| l.command_title.contains("reference"))
            .collect();
        assert!(class_lenses.len() >= 2, "{lenses:?}");
    }

    #[test]
    fn lens_counts_class_constructor_calls() {
        // `MyClass new` and `MyClass create instance` are
        // constructor calls — both contribute to the reference
        // count.
        let src = concat!(
            "oo::class create MyClass {}\n",
            "MyClass new\n",
            "MyClass create instance\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let myclass_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("class lens at line 0");
        // Two constructor calls.
        assert_eq!(
            myclass_lens.command_title, "2 references",
            "got {:?}",
            myclass_lens.command_title,
        );
    }

    // class-member lenses

    #[test]
    fn lens_counts_method_calls_within_class_body() {
        // Intra-class self-dispatch in TclOO is `my greet`, not a bare
        // `greet` (a bare word resolves as a normal command, never the
        // object's method).  `greet` is dispatched twice from `twice`.
        let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        // Find the lens anchored on line 1 (the `greet`
        // declaration's name span).
        let greet_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("greet lens");
        assert_eq!(greet_lens.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_bare_word_in_method_body_is_not_a_self_dispatch() {
        // FP guard: a bare `greet` inside a sibling method body is a normal
        // command call, NOT TclOO self-dispatch (that is `my greet`), so it
        // must not be counted as a reference to the method.  The old
        // head-token heuristic wrongly counted these.  tclsh 8.6/9.0 raise
        // `invalid command name "greet"` here, but an embedding's `unknown`
        // or `::greet` can still reach the method, so the count stays partial.
        let src = "oo::class create C {\n    method greet {} {}\n    method other {} { greet ; greet }\n}\n";
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let greet_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("greet lens");
        assert_eq!(
            greet_lens.command_title, "References unavailable",
            "{lenses:?}"
        );
    }

    #[test]
    fn lens_reports_zero_for_uncalled_method() {
        let src = "oo::class create C {\n    method orphan {} {}\n}\n";
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let orphan_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("orphan lens");
        assert_eq!(orphan_lens.command_title, "0 references", "{lenses:?}");
    }

    // qname wiring: a method / classmethod lens must carry a
    // non-empty `qname` matching `tcl_compiler::analyser::class_member_key`
    // so the server treats it as resolvable (`has_qname` in
    // `tcl-lsp-server`'s `code_lens` handler) and attaches a clickable
    // `tcl-lsp.showReferences` command via `codeLens/resolve`, instead of
    // leaving it an inert bare title.  The wire-level resolve round-trip
    // itself is covered by
    // `tcl-lsp-server`'s `code_lens_resolve_wires_show_references_command_for_method`
    // / `..._for_classmethod` tests; these lock in the `qname` this crate
    // hands the server.

    #[test]
    fn method_lens_carries_class_member_key_qname() {
        // FN→TP for the shape with a `variable` and
        // `constructor` declared before the `method` in the class body.
        let src = "oo::class create Bar {\n   variable _options\n    constructor {args} {\n         set _options $args\n    }\n\n    method get {key} {\n        return [dict get $_options $key]\n    }\n\n}\nset b [Bar new]\nputs [$b get foo]\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("get lens");
        assert_eq!(
            get_lens.qname,
            tcl_compiler::analyser::class_member_key("::Bar", "get", false),
            "{lenses:?}"
        );
        assert!(!get_lens.qname.is_empty(), "empty qname stays inert");
    }

    #[test]
    fn classmethod_lens_carries_class_member_key_qname() {
        let src =
            "oo::class create Factory {\n    classmethod make {} { return [Factory new] }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let make_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("make lens");
        assert_eq!(
            make_lens.qname,
            tcl_compiler::analyser::class_member_key("::Factory", "make", true),
            "{lenses:?}"
        );
    }

    #[test]
    fn method_and_classmethod_qnames_never_collide_on_same_name() {
        // FP guard: a class with both a `method foo` and a `classmethod foo`
        // must get two distinct, disambiguated qnames — a naive
        // `{class}::{name}` key would collide and let `codeLens/resolve`
        // attach one member's locations to the other's lens.
        let src = "oo::class create C {\n    method foo {} {}\n    classmethod foo {} {}\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let qnames: std::collections::HashSet<&str> = lenses
            .iter()
            .filter(|l| l.range.start_line == 1 || l.range.start_line == 2)
            .map(|l| l.qname.as_str())
            .collect();
        assert_eq!(qnames.len(), 2, "{lenses:?}");
        assert!(qnames.contains("::C::method::foo"), "{qnames:?}");
        assert!(qnames.contains("::C::classmethod::foo"), "{qnames:?}");
    }

    // property-member lenses

    /// `property` is Tcl 9.0+ — the shared `analyse()` helper above fixes the
    /// dialect at 8.6, so these tests analyse at 9.0 directly (mirrors
    /// `rename.rs`'s property-rename tests).
    fn analyse90(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl9.0").clone()
    }

    #[test]
    fn lens_counts_property_my_dispatch_within_class_body() {
        // A property's auto-generated accessor dispatches through `my
        // <property>`, exactly like a method — `bump` reads/writes `size`
        // twice, so its lens must read "2 references" the same way a
        // twice-dispatched method would (`lens_counts_method_calls_within_class_body`).
        let src = "oo::class create Widget {\n    property size -get {return $mySize} -set {set mySize $value}\n    method bump {} { my size ; my size }\n}\n";
        let analysis = analyse90(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(size_lens.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_reports_zero_for_unused_property() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(size_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn property_lens_carries_class_property_key_qname() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let size_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("size lens");
        assert_eq!(
            size_lens.qname,
            tcl_compiler::analyser::class_property_key("::Widget", "size"),
            "{lenses:?}"
        );
        assert!(!size_lens.qname.is_empty(), "empty qname stays inert");
    }

    #[test]
    fn property_qname_never_collides_with_a_same_named_method() {
        // FP guard mirroring `method_and_classmethod_qnames_never_collide_on_same_name`:
        // methods and properties are independent tables, so a `property
        // color` and a `method color` on the same class must still get two
        // distinct, disambiguated lenses.
        let src = "oo::class create C {\n    property color\n    method color {} {}\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let qnames: std::collections::HashSet<&str> = lenses
            .iter()
            .filter(|l| l.range.start_line == 1 || l.range.start_line == 2)
            .map(|l| l.qname.as_str())
            .collect();
        assert_eq!(qnames.len(), 2, "{lenses:?}");
        assert!(qnames.contains("::C::property::color"), "{qnames:?}");
        assert!(qnames.contains("::C::method::color"), "{qnames:?}");
    }

    #[test]
    fn lens_counts_each_name_independently_in_a_multi_name_property_declaration() {
        // `property x y` declares two independent properties from one command
        // (verified structurally by `extract_property_defs`'s own
        // `property_subcommand_records_multiple_names` test) — each name must
        // get its own lens with its own independently-counted `my <name>`
        // dispatch sites, not a shared or merged count.
        let src = "oo::class create Widget {\n    property x y\n    method bump {} { my x ; my y ; my y }\n}\n";
        let analysis = analyse90(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let x_lens = lenses
            .iter()
            .find(|l| l.qname == "::Widget::property::x")
            .expect("x lens");
        let y_lens = lenses
            .iter()
            .find(|l| l.qname == "::Widget::property::y")
            .expect("y lens");
        assert_eq!(x_lens.command_title, "1 reference", "{lenses:?}");
        assert_eq!(y_lens.command_title, "2 references", "{lenses:?}");
    }

    // constructor / destructor next-chain lenses

    #[test]
    fn constructor_lens_counts_subclass_next_chain() {
        let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let base_ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::constructor")
            .expect("Base constructor lens");
        assert_eq!(base_ctor_lens.command_title, "1 reference", "{lenses:?}");
        // `Sub` also declares its own constructor, so it gets a lens too —
        // nothing chains *into* Sub's, so it reads zero.
        let sub_ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Sub::constructor")
            .expect("Sub constructor lens");
        assert_eq!(sub_ctor_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn destructor_lens_counts_subclass_next_chain() {
        let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let base_dtor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::destructor")
            .expect("Base destructor lens");
        assert_eq!(base_dtor_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn constructor_lens_reports_zero_when_no_subclass_chains() {
        let src = "oo::class create Base {\n    constructor {} { }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let ctor_lens = lenses
            .iter()
            .find(|l| l.qname == "::Base::constructor")
            .expect("constructor lens");
        assert_eq!(ctor_lens.command_title, "0 references", "{lenses:?}");
    }

    #[test]
    fn class_with_no_explicit_constructor_or_destructor_gets_neither_lens() {
        let src = "oo::class create Plain {\n    method use {} {}\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        assert!(
            lenses
                .iter()
                .all(|l| !l.qname.ends_with("::constructor") && !l.qname.ends_with("::destructor")),
            "{lenses:?}"
        );
    }

    #[test]
    fn constructor_lens_carries_class_constructor_key_qname() {
        let src = "oo::class create Bar {\n    constructor {} { }\n}\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let ctor_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("constructor lens");
        assert_eq!(
            ctor_lens.qname,
            tcl_compiler::analyser::class_constructor_key("::Bar"),
            "{lenses:?}"
        );
    }

    // workspace-index: cross-document reference counts

    #[test]
    fn proc_lens_counts_cross_document_calls() {
        use crate::workspace_index::WorkspaceIndex;
        // lib.tcl defines `helper` with no local callers;
        // consumer.tcl calls it twice.
        let lib_src = "proc helper {} {}\n";
        let lib = analyse(lib_src);
        let consumer = analyse("helper\nhelper\n");
        let index = WorkspaceIndex::from_documents([
            ("file:///lib.tcl", &lib),
            ("file:///consumer.tcl", &consumer),
        ]);
        // The lens on lib.tcl's `helper` counts the two
        // cross-document calls.
        let lenses = code_lenses(
            lib_src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&lib),
            Some(&index),
            "file:///lib.tcl",
        );
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn proc_lens_without_workspace_counts_local_only() {
        let src = "proc helper {} {}\nhelper\n";
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let helper = lenses
            .iter()
            .find(|l| l.range.start_line == 0)
            .expect("helper lens");
        assert_eq!(helper.command_title, "1 reference", "{lenses:?}");
    }

    // The lens count must equal the reference list.

    /// Parse a `"N reference(s)"` title back into its integer count.
    fn title_count(title: &str) -> usize {
        title
            .split_once(' ')
            .and_then(|(n, _)| n.parse().ok())
            .unwrap_or_else(|| panic!("unparseable lens title: {title:?}"))
    }

    /// The lens count for the proc at `name_line` must equal the number of
    /// call sites `references` (the peek / Find All References) resolves
    /// with `include_declaration = false`.
    fn assert_lens_matches_references(src: &str, name_line: u32) {
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let lens = lenses
            .iter()
            .find(|l| l.range.start_line == name_line)
            .unwrap_or_else(|| panic!("no lens on line {name_line}: {lenses:?}"));
        let refs = crate::references::references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            lens.range.start_line,
            lens.range.start_character,
            &analysis,
            false,
        );
        assert_eq!(
            title_count(&lens.command_title),
            refs.len(),
            "lens {:?} disagrees with references {:?} for source {src:?}",
            lens.command_title,
            refs,
        );
    }

    #[test]
    fn lens_count_matches_references_forward_ref() {
        // Headline symptom: a call *before* the definition. The
        // name-only resolved-name tally reported 0 here; the lens must
        // agree with the resolver, which finds the forward call.
        let src = "foo\nproc foo {} {}\n";
        assert_lens_matches_references(src, 1);
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let foo = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("foo lens");
        assert_eq!(foo.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_count_matches_references_after_def() {
        assert_lens_matches_references("proc foo {} {}\nfoo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_qualified_call() {
        assert_lens_matches_references("proc foo {} {}\n::foo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_ns_qualified_call() {
        assert_lens_matches_references("namespace eval ns { proc foo {} {} }\nns::foo\n", 0);
    }

    #[test]
    fn lens_count_matches_references_call_inside_body() {
        assert_lens_matches_references("proc foo {} {}\nproc bar {} { foo }\n", 0);
    }

    #[test]
    fn lens_count_matches_references_cmd_substitution() {
        assert_lens_matches_references("proc foo {} {}\nset x [foo]\n", 0);
    }

    #[test]
    fn lens_count_matches_references_unused_proc() {
        assert_lens_matches_references("proc foo {} {}\n", 0);
    }

    #[test]
    fn lens_count_matches_references_same_name_other_namespace() {
        // The namespace-aware resolver must not count a same-named proc
        // call from a different namespace; the lens count follows it so the
        // two can never drift (the precise divergence this guards against).
        let src = concat!(
            "namespace eval a { proc helper {} {} }\n",
            "namespace eval b { proc helper {} {} ; helper }\n",
        );
        assert_lens_matches_references(src, 0); // a::helper
        assert_lens_matches_references(src, 1); // b::helper
    }

    #[test]
    fn lens_does_not_credit_ambiguous_forward_ref_to_other_namespace() {
        // A forward `foo` call inside namespace
        // `a`, with both `::a::foo` and `::b::foo` defined, must be credited
        // only to `::a::foo`.  A tail-name resolver would attribute the
        // unresolved call to *both* procs, falsely showing `1 reference`
        // above `::b::foo`.  The namespace-aware resolver the lens count is
        // derived from scopes the call to its own namespace.
        let src = concat!(
            "namespace eval a { foo ; proc foo {} {} }\n",
            "namespace eval b { proc foo {} {} }\n",
        );
        let analysis = analyse(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let by_qname = |q: &str| {
            lenses
                .iter()
                .find(|l| l.qname == q)
                .unwrap_or_else(|| panic!("no lens for {q}: {lenses:?}"))
        };
        assert_eq!(
            by_qname("::a::foo").command_title,
            "1 reference",
            "{lenses:?}"
        );
        assert_eq!(
            by_qname("::b::foo").command_title,
            "0 references",
            "{lenses:?}"
        );
    }

    // A method lens must count external `$obj method` sites.
    //
    // TP  — a real reference (`$obj method`, `my method`) is counted.
    // FP  — a non-reference (`dict get`, a bare word) is *not* counted.
    // TN  — a method with genuinely no references reads `0 references`.
    // FN  — an external `$obj method` call, which a body-head heuristic
    //       misses, is counted.

    /// The shared fixture for the four cases above.
    const ISSUE_864_SRC: &str = concat!(
        "oo::class create Bar {\n",
        "   variable _options\n",
        "    constructor {args} {\n",
        "         set _options $args\n",
        "    }\n",
        "\n",
        "    method get {key} {\n",
        "        return [dict get $_options $key]\n",
        "    }\n",
        "\n",
        "}\n",
        "set b [Bar new]\n",
        "puts [$b get foo]\n",
    );

    #[test]
    fn lens_counts_external_obj_method_call_issue_864() {
        // The known Bar instance supplies one reference to its get method.
        let analysis = analyse(ISSUE_864_SRC);
        assert!(!analysis.all_classes.is_empty(), "class not analysed");
        let lenses = code_lenses(
            ISSUE_864_SRC,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("method get lens on line 6");
        assert_eq!(get_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_matches_peek_for_issue_864_method() {
        // The lens count and Find All References must agree exactly.
        assert_lens_matches_references(ISSUE_864_SRC, 6);
    }

    #[test]
    fn lens_does_not_count_dict_get_as_method_ref() {
        // FP guard: the method is named `get` and its own body contains
        // `dict get $_options $key`.  The `get` word there is the `dict`
        // ensemble subcommand, not a dispatch of the method, so it must not
        // inflate the count.  With only the external `$b get foo`, the count
        // is exactly 1.
        let analysis = analyse(ISSUE_864_SRC);
        let lenses = code_lenses(
            ISSUE_864_SRC,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let get_lens = lenses
            .iter()
            .find(|l| l.range.start_line == 6)
            .expect("method get lens");
        assert_eq!(get_lens.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_counts_multiple_external_obj_method_calls() {
        // TP: two distinct instances each dispatch `bark` once.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "set a [Dog new]\n",
            "set b [Dog new]\n",
            "$a bark\n",
            "puts [$b bark]\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "2 references", "{lenses:?}");
    }

    #[test]
    fn lens_counts_obj_method_from_create_instance() {
        // TP: `Dog create rex` binds `rex` as a Dog instance command, so
        // `rex bark` is a method dispatch.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "Dog create rex\n",
            "rex bark\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_counts_inherited_method_on_subclass_instance() {
        // TP + inheritance: `Base`'s `speak` is inherited by `Derived`; a
        // `Derived` instance dispatching `speak` resolves to `Base`'s copy,
        // so it counts toward `Base::speak`'s lens.
        let src = concat!(
            "oo::class create Base {\n",
            "    method speak {} {}\n",
            "}\n",
            "oo::class create Derived {\n",
            "    superclass Base\n",
            "}\n",
            "set d [Derived new]\n",
            "$d speak\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let speak = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("speak lens");
        assert_eq!(speak.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_matches_peek_for_inherited_method_with_subclass_my_dispatch() {
        // The lens uses `method_references_for_class`, which counts
        // a `my speak` call in an *inheriting* subclass body; the peek from the
        // declaration counts it too through the same resolver, so lens
        // and peek can't drift.  Here `Base::speak` is referenced by `Derived`'s
        // `my speak` and by `$d speak` — two sites.
        let src = concat!(
            "oo::class create Base {\n",
            "    method speak {} {}\n",
            "}\n",
            "oo::class create Derived {\n",
            "    superclass Base\n",
            "    method twice {} { my speak }\n",
            "}\n",
            "set d [Derived new]\n",
            "$d speak\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let speak = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("speak lens");
        assert_eq!(speak.command_title, "2 references", "{lenses:?}");
        // Lens and peek (from the `Base::speak` declaration) must agree.
        assert_lens_matches_references(src, 1);
    }

    #[test]
    fn lens_counts_namespaced_class_method() {
        // TP: a class defined inside a namespace, instance created and its
        // method dispatched — the qualified-name resolution must hold up.
        let src = concat!(
            "namespace eval app {\n",
            "    oo::class create Widget {\n",
            "        method render {} {}\n",
            "    }\n",
            "}\n",
            "set w [app::Widget new]\n",
            "$w render\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        // `method render` sits on line 2.
        let render = lenses
            .iter()
            .find(|l| l.range.start_line == 2)
            .expect("render lens");
        assert_eq!(render.command_title, "1 reference", "{lenses:?}");
        assert_lens_matches_references(src, 2);
    }

    #[test]
    fn lens_counts_obj_method_call_inside_proc_body() {
        // TP: the dispatch lives inside a proc body, which the top-level
        // segment scan skips — the resolver descends proc bodies explicitly.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "set d [Dog new]\n",
            "proc run {} {\n",
            "    global d\n",
            "    $d bark\n",
            "}\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "1 reference", "{lenses:?}");
    }

    #[test]
    fn lens_unknown_instance_method_is_not_counted() {
        // TN/FP guard: `$x bark` where `x` has no recorded class is not a
        // resolvable dispatch, so it must not count toward `Dog::bark`.
        // tclsh 8.6/9.0: with `x` unset the site raises `can't read "x"`, but
        // an embedding that sets `x` can reach `bark`, so the count stays
        // partial.
        let src = concat!(
            "oo::class create Dog {\n",
            "    method bark {} {}\n",
            "}\n",
            "$x bark\n",
        );
        let analysis = analyse(src);
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        let bark = lenses
            .iter()
            .find(|l| l.range.start_line == 1)
            .expect("bark lens");
        assert_eq!(bark.command_title, "References unavailable", "{lenses:?}");
    }

    #[test]
    fn lens_classmethod_count_matches_peek() {
        // A class-side method (`classmethod`) gets a lens, and its count must
        // equal the peek exactly.  External class-side dispatch (`Factory
        // build`) is not an instance-receiver form, so the count is 0 here —
        // and the peek agrees, which is the invariant under test.
        let src = concat!(
            "oo::class create Factory {\n",
            "    classmethod build {} {}\n",
            "}\n",
            "Factory build\n",
        );
        let analysis = analyse(src);
        // Locate the classmethod lens by name span; skip if the analyser
        // build for this dialect didn't record the class-side method.
        let build_span = match analysis
            .all_classes
            .values()
            .find_map(|c| c.class_methods.get("build"))
        {
            Some(m) if !m.name_span.is_empty() => m.name_span,
            _ => return,
        };
        let li = LineIndex::new(src);
        let name_line = li.position_at_utf16(build_span.start(), src).line;
        assert_lens_matches_references(src, name_line);
    }

    // `lens_qname_exists` — `codeLens/resolve`'s targeted existence check.

    #[test]
    fn lens_qname_exists_finds_a_plain_proc() {
        let analysis = analyse("proc foo {} {}\n");
        assert!(lens_qname_exists(&analysis, "::foo"));
        assert!(!lens_qname_exists(&analysis, "::bar"));
    }

    #[test]
    fn lens_qname_exists_finds_a_plain_class() {
        let analysis = analyse("oo::class create Widget {}\n");
        assert!(
            !analysis.all_classes.is_empty(),
            "positive class lens fixture must produce class metadata"
        );
        assert!(lens_qname_exists(&analysis, "::Widget"));
        assert!(!lens_qname_exists(&analysis, "::Other"));
    }

    #[test]
    fn lens_qname_exists_finds_a_method_and_classmethod_and_disambiguates() {
        let src = "oo::class create C {\n    method foo {} {}\n    classmethod foo {} {}\n}\n";
        let analysis = analyse(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "foo", false)
        ));
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "foo", true)
        ));
        // A name neither table declares must not exist.
        assert!(!lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_member_key("::C", "missing", false)
        ));
    }

    #[test]
    fn lens_qname_exists_finds_a_property() {
        let src = "oo::class create Widget {\n    property size\n}\n";
        let analysis = analyse90(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_property_key("::Widget", "size")
        ));
        assert!(!lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_property_key("::Widget", "missing")
        ));
    }

    #[test]
    fn lens_qname_exists_finds_constructor_and_destructor() {
        let src = "oo::class create Bar {\n    constructor {} {}\n    destructor {}\n}\n";
        let analysis = analyse(src);
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_constructor_key("::Bar")
        ));
        assert!(lens_qname_exists(
            &analysis,
            &tcl_compiler::analyser::class_destructor_key("::Bar")
        ));
        // A class with no explicit constructor/destructor gets neither.
        let plain = analyse("oo::class create Plain {\n    method use {} {}\n}\n");
        assert!(!lens_qname_exists(
            &plain,
            &tcl_compiler::analyser::class_constructor_key("::Plain")
        ));
        assert!(!lens_qname_exists(
            &plain,
            &tcl_compiler::analyser::class_destructor_key("::Plain")
        ));
    }

    #[test]
    fn lens_qname_exists_rejects_an_unrelated_qname() {
        let src = "proc foo {} {}\noo::class create Bar {\n    method m {} {}\n}\n";
        let analysis = analyse(src);
        assert!(!lens_qname_exists(&analysis, "::nope"));
        assert!(!lens_qname_exists(&analysis, "::Bar::method::nope"));
        assert!(!lens_qname_exists(&analysis, "::NoSuchClass::method::m"));
    }

    #[test]
    fn original_lens_reporting_keys_keep_owner_currency_and_ignore_display_maps() {
        let source = "namespace eval N {proc helper {} {}}\n";
        let mut analysis = analyse(source);
        assert!(lens_qname_exists(&analysis, "helper"));
        assert!(lens_qname_exists(&analysis, "::N::helper"));
        assert!(!lens_qname_exists(&analysis, "::helper"));
        analysis.all_procs.clear();
        assert!(lens_qname_exists(&analysis, "helper"));
        assert!(lens_qname_exists(&analysis, "::N::helper"));
        analysis.resolved_input = None;
        assert!(!lens_qname_exists(&analysis, "helper"));
        assert!(!lens_qname_exists(&analysis, "::N::helper"));
    }

    #[test]
    fn original_class_member_reporting_keys_require_current_source_without_display_maps() {
        let source = "oo::class create C {method foo {} {}; classmethod foo {} {}}\n";
        let mut analysis = analyse(source);
        analysis.all_classes.clear();
        for class_side in [false, true] {
            assert!(lens_qname_exists(
                &analysis,
                &tcl_compiler::analyser::class_member_key("::C", "foo", class_side),
            ));
            assert!(!lens_qname_exists(
                &analysis,
                &tcl_compiler::analyser::class_member_key("::Other", "foo", class_side),
            ));
        }
        analysis.resolved_input = None;
        for class_side in [false, true] {
            assert!(!lens_qname_exists(
                &analysis,
                &tcl_compiler::analyser::class_member_key("::C", "foo", class_side),
            ));
        }
    }

    #[test]
    fn lens_qname_exists_agrees_with_code_lenses_output() {
        // Structural parity: every qname `code_lenses` actually emits a lens
        // for must be found by the targeted check, and nothing else is.
        let src = concat!(
            "proc helper {} {}\n",
            "oo::class create Bar {\n",
            "    variable _x\n",
            "    constructor {} { set _x 0 }\n",
            "    destructor {}\n",
            "    method get {} { return $_x }\n",
            "    classmethod make {} { return [Bar new] }\n",
            "    property colour\n",
            "}\n",
            "helper\n",
            "set b [Bar new]\n",
            "$b get\n",
        );
        let analysis = analyse90(src);
        let lenses = code_lenses(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Some(&analysis),
            None,
            "",
        );
        assert!(!lenses.is_empty(), "sanity: some lenses expected");
        for lens in &lenses {
            assert!(
                lens_qname_exists(&analysis, &lens.qname),
                "code_lenses emitted a qname the targeted check rejects: {}",
                lens.qname,
            );
        }
    }
}

#[cfg(test)]
mod native_lens_presentation_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn native_own_object_method_lens_keeps_identity_when_class_display_key_is_unavailable() {
        let source = r"oo::class create C {}; C create object; oo::objdefine object method p\uD800 {} {}; object p\uD800";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let configuration = analysis.original_object_configurations().next().unwrap();
        let method = configuration.members().declarations().next().unwrap();
        let identity = crate::original_declaration::OriginalDeclarationIdentity::for_object_method(
            "file:///object.tcl",
            source,
            &analysis,
            configuration.target(),
            method,
        )
        .unwrap();
        assert!(identity.class_metadata(&analysis).is_none());
        assert!(identity.reporting_key(&analysis).is_none());
        let lenses = code_lenses(
            source,
            crate::profile_for_dialect("tcl8.6"),
            Some(&analysis),
            None,
            identity.uri(),
        );
        let lens = lenses
            .iter()
            .find(|lens| lens.identity.as_ref() == Some(&identity))
            .unwrap();
        assert_eq!(lens.qname, identity.label());
        assert_eq!(lens.identity.as_ref(), Some(&identity));
    }
}
