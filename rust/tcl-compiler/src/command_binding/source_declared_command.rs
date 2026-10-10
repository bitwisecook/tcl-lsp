// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Document assistance contracts, independently of Registry implementations.

mod logical_body;
pub use logical_body::{OriginalDeclaredLogicalBodyContext, OriginalDeclaredSourceBodyFrame};

use super::{CommandAllocationSite, SourceAdviceNameInput, SourceInvocationBinding};
use crate::analyser::ResolvedAnalysisInput;
use crate::registry_invocation::EffectiveInvocationWord;
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::model::DeclaredCommand;

/// Explicit source obligations; a document contract is never an implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalDeclaredSourceObligation {
    /// A document or workspace declaration supplies untrusted assistance only.
    DeclarationApplicability,
    /// The explicit Logical source domain supplies no Native naming recipe.
    LogicalSourceApplicability,
    /// Earlier source effects have unresolved runtime table alternatives.
    UnknownEarlierMutation,
    /// The genuine source body/frame recipe does not establish future entry.
    DeferredLogicalBodyApplicability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredSourceSelection {
    site: CommandAllocationSite,
    original: Arc<[NativeWord]>,
    head: SourceAdviceNameInput,
    declaration: Arc<DeclaredCommand>,
    obligations: Vec<OriginalDeclaredSourceObligation>,
    logical_body: Option<Arc<OriginalDeclaredLogicalBodyContext>>,
}
impl DeclaredSourceSelection {
    pub(super) fn authored(
        site: CommandAllocationSite,
        original: &[NativeWord],
        head: SourceAdviceNameInput,
        declaration: Arc<DeclaredCommand>,
        unknown: bool,
    ) -> Option<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        if original.is_empty() || original.iter().any(|word| word.group().expand) {
            return None;
        }
        let mut obligations = vec![OriginalDeclaredSourceObligation::DeclarationApplicability];
        if head.logical_input().is_some() {
            obligations.push(OriginalDeclaredSourceObligation::LogicalSourceApplicability);
        }
        if unknown {
            obligations.push(OriginalDeclaredSourceObligation::UnknownEarlierMutation);
        }
        Some(Self {
            site,
            original: Arc::from(original),
            head,
            declaration,
            obligations,
            logical_body: None,
        })
    }
    pub(super) const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }

    pub(super) fn with_logical_body(
        mut self,
        body: Arc<OriginalDeclaredLogicalBodyContext>,
    ) -> Option<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        self.head.logical_input()?;
        if !body.owns_words(&self.original) {
            return None;
        }
        self.obligations
            .push(OriginalDeclaredSourceObligation::DeferredLogicalBodyApplicability);
        self.logical_body = Some(body);
        Some(self)
    }

    pub(crate) fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
}

/// Sealed source declaration and complete direct-written argv geometry.
/// No builtin traits, target, Native entry, body execution or Normal result
/// follows from this receipt. The retained declaration owns every role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredCommandWords {
    selected: DeclaredSourceSelection,
    input: ResolvedAnalysisInput,
    arguments: Vec<EffectiveInvocationWord>,
}
impl OriginalDeclaredCommandWords {
    pub(crate) fn from_selected(
        selected: DeclaredSourceSelection,
        input: &ResolvedAnalysisInput,
    ) -> Option<Self> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let first = selected.original.first()?;
        let image = first.image();
        let config = input.lexer_config();
        if selected.site.source.source_image() != image
            || selected
                .original
                .iter()
                .any(|word| word.image() != image || word.config() != config)
            || selected.head.original_word() != Some(first)
            || selected.logical_body.as_ref().is_some_and(|body| {
                !body.matches_input(input) || !body.owns_words(&selected.original)
            })
        {
            return None;
        }
        let protocol = selected
            .head
            .native_input()
            .map(|input| input.policy().string_protocol());
        let values = match protocol {
            Some(protocol) => Some(
                tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    &selected.original,
                    protocol,
                )
                .ok()?,
            ),
            None => None,
        };
        let arguments = selected
            .original
            .iter()
            .enumerate()
            .skip(1)
            .map(|(index, word)| {
                let bytes = match &values {
                    Some(values) => values.literal(index).map(Vec::from),
                    None => tcl_syntax::word_rules::original_static_word_ascii_presentation(word),
                };
                bytes.map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                    EffectiveInvocationWord::ByteLiteral(Arc::from(bytes))
                })
            })
            .collect();
        Some(Self {
            selected,
            input: input.clone(),
            arguments,
        })
    }
    /// Genuine conditional Logical body, source namespace and frame recipe.
    /// This supplies no entered frame or runtime namespace identity.
    #[must_use]
    pub fn logical_body_context(&self) -> Option<&OriginalDeclaredLogicalBodyContext> {
        self.selected.logical_body.as_deref()
    }

    /// Immutable authored contract, including its full typed provenance.
    #[must_use]
    pub fn descriptor(&self) -> &DeclaredCommand {
        &self.selected.declaration
    }
    /// Genuine complete source invocation, including the whole original head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.selected.original
    }
    /// Naming input under its independently selected Native or Logical purpose.
    #[must_use]
    pub const fn original_head(&self) -> &SourceAdviceNameInput {
        &self.selected.head
    }
    /// Exact call occurrence; this is not an entered invocation or allocation.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.selected.site
    }
    /// Complete immutable source interpretation and actual Registry context.
    #[must_use]
    pub const fn resolved_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Original post-head source values, with substitutions explicitly dynamic.
    #[must_use]
    pub fn arguments(&self) -> &[EffectiveInvocationWord] {
        &self.arguments
    }
    /// Exact original word for this direct-written post-head argument ordinal.
    #[must_use]
    pub fn argument_word(&self, argument: usize) -> Option<&NativeWord> {
        self.selected.original.get(argument.checked_add(1)?)
    }
    /// Effective post-head ordinal is the same written post-head ordinal.
    /// Expansion children and inserted alias operands cannot acquire this seal.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        self.argument_word(argument).map(|_| argument)
    }
    /// Optional-slot roles come solely from the authored declaration owner.
    #[must_use]
    pub fn supplied_argument_roles(&self) -> Option<Vec<(usize, tcl_registry::ArgRole)>> {
        self.descriptor()
            .supplied_argument_roles(self.arguments.len())
    }
    /// Source Body geometry selected by this declaration's own supplied roles.
    /// Declarations without script timing yield Syntax only; requesting
    /// `PotentialEvaluation` cannot borrow builtin or entered-body semantics.
    #[must_use]
    pub fn source_script_bodies_for(
        &self,
        purpose: crate::registry_invocation::OriginalSourceScriptPurpose,
    ) -> Vec<crate::registry_invocation::OriginalDeclaredSourceScriptBody> {
        crate::registry_invocation::declared_source_script_bodies_for(self, purpose)
    }

    /// Argument-count contract from this declaration, without builtin inheritance.
    #[must_use]
    pub fn source_arity(&self) -> Option<tcl_registry::Arity> {
        self.descriptor().source_arity()
    }
    /// Retained conditional applicability; no runtime selection follows.
    #[must_use]
    pub fn obligations(&self) -> &[OriginalDeclaredSourceObligation] {
        &self.selected.obligations
    }
}

impl SourceInvocationBinding {
    pub(crate) fn original_declared_source_selection(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<DeclaredSourceSelection> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        let first = original.first()?;
        if tokens.synthetic.is_some()
            || config != input.lexer_config()
            || site.source.source_image() != first.image()
            || !matches!(first.image().channel(), tcl_lexer::SourceChannel::Document)
        {
            return None;
        }
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if state.baseline.registry_snapshot.as_ref()
            != Some(
                &input
                    .context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key(),
            )
        {
            return None;
        }
        let name = self.original_written_name_input(tokens, 0)?;
        if name.original_word_key()?.original_word() != first {
            return None;
        }
        let candidate = state.original_declared_candidate_from_input(
            &self.lookup_namespace_key,
            &name,
            &state.baseline.declared_commands,
        )?;
        let context = input.context_registry();
        let surface = state
            .baseline
            .declared_surface
            .as_ref()?
            .document_surface(context.commands());
        let declaration = surface.declared_command(&candidate.name)?;
        DeclaredSourceSelection::authored(
            site.clone(),
            original,
            SourceAdviceNameInput::Native(name),
            Arc::new(declaration.clone()),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::registry_invocation::source_structure::source_declared_command_words;

    fn last_command(source: &str, analysis: &AnalysisResult) -> crate::segmenter::SegmentedCommand {
        crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .into_iter()
        .last()
        .unwrap()
    }

    #[test]
    fn original_declared_words_keep_roles_provenance_and_direct_ordinals() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {?kind? expression:expr}\n# tcl-lsp: stubs-end\nselect {$value + 1}";
        for profile in ["tcl8.6", "tcl9.0", "tcl"] {
            let analysis = Analyser::new().analyse(source, profile);
            let command = last_command(source, &analysis);
            let receipt =
                source_declared_command_words(source, &analysis, &command).expect(profile);
            assert_eq!(
                receipt.descriptor().provenance(),
                tcl_dialect::model::Provenance::Document
            );
            assert_eq!(receipt.original_head().bytes(), b"select");
            assert_eq!(receipt.source_arity(), Some(tcl_registry::Arity::new(1, 2)));
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Expr)])
            );
            assert_eq!(receipt.written_argument(0), Some(0));
            assert_eq!(receipt.written_argument(1), None);
            assert_eq!(
                receipt.argument_word(0).unwrap().image(),
                &tcl_lexer::SourceImage::document(source)
            );
            assert!(
                source_declared_command_words(&(source.to_owned() + " "), &analysis, &command)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_declared_script_bodies_keep_source_roles_without_timing_authority() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        use crate::registry_invocation::OriginalSourceScriptPurpose;
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {?kind? script:body}\n# tcl-lsp: stubs-end\nhold {\n# café 😀 \\\nputs hidden\n}";
        for profile in ["tcl8.6", "tcl9.0", "tcl"] {
            let analysis = Analyser::new().analyse(source, profile);
            let command = last_command(source, &analysis);
            let words = source_declared_command_words(source, &analysis, &command).unwrap();
            let bodies = words.source_script_bodies_for(OriginalSourceScriptPurpose::Syntax);
            let [body] = bodies.as_slice() else {
                panic!("{profile}: {bodies:?}");
            };
            assert_eq!(body.original_container(), words.argument_word(0).unwrap());
            assert_eq!(body.source_words(), &words);
            assert_eq!(
                source.get(body.content_span().as_range()),
                Some("\n# café 😀 \\\nputs hidden\n")
            );
            assert!(body.matches_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
            assert!(
                body.matches_context(&analysis.resolved_input.as_ref().unwrap().context_registry())
            );
            assert!(
                words
                    .source_script_bodies_for(OriginalSourceScriptPurpose::PotentialEvaluation)
                    .is_empty(),
                "a declared Body role supplies no timing"
            );
            assert!(!body.matches_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                analysis.body_lexer_config.unwrap()
            ));
            let mut stale = analysis.clone();
            stale.resolved_input = None;
            assert!(source_declared_command_words(source, &stale, &command).is_none());
        }
    }

    #[test]
    fn original_declared_script_bodies_refuse_dynamic_cooked_and_shadowed_geometry() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        use crate::registry_invocation::OriginalSourceScriptPurpose;
        for suffix in [
            "hold $script",
            "hold [list x]",
            r#"hold "\u0023 note\nputs hidden""#,
            "hold prefix{body}",
        ] {
            let source = format!(
                "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {{script:body}}\n# tcl-lsp: stubs-end\n{suffix}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let command = last_command(&source, &analysis);
            let words = source_declared_command_words(&source, &analysis, &command).unwrap();
            assert!(
                words
                    .source_script_bodies_for(OriginalSourceScriptPurpose::Syntax)
                    .is_empty(),
                "{suffix}"
            );
        }
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub hold {script:body}\n# tcl-lsp: stubs-end\nproc hold {value} {}\nhold {\n# note \\\nputs hidden\n}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            source_declared_command_words(source, &analysis, &last_command(source, &analysis))
                .is_none()
        );
    }

    #[test]
    fn original_declared_words_refuse_replacements_and_builtin_trait_donation() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr {value}\n# tcl-lsp: stubs-end\nexpr {$value + 1}";
        for profile in ["tcl8.6", "tcl9.0", "tcl"] {
            let analysis = Analyser::new().analyse(source, profile);
            let command = last_command(source, &analysis);
            let receipt =
                source_declared_command_words(source, &analysis, &command).expect(profile);
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Value)])
            );
            assert!(
                crate::registry_invocation::source_structure::source_registry_words(
                    source, &analysis, &command
                )
                .is_none()
            );
            for suffix in [
                "proc expr {args} {}; expr {$value + 1}",
                "rename expr {}; expr {$value + 1}",
                "expr {*}{one two}",
                "$selected {$value + 1}",
            ] {
                let changed = format!(
                    "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr {{value}}\n# tcl-lsp: stubs-end\n{suffix}"
                );
                let result = Analyser::new().analyse(&changed, profile);
                assert!(
                    source_declared_command_words(
                        &changed,
                        &result,
                        &last_command(&changed, &result)
                    )
                    .is_none(),
                    "{profile}: {suffix}"
                );
            }
        }
    }

    #[test]
    fn original_declared_words_use_current_contracts_in_every_document_ingress() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub check {expression:expr}\n# tcl-lsp: stubs-end\ncheck {$value + 1}";
        let config = tcl_lexer::LexerConfig::for_file_grammar(
            tcl_dialect::DialectProfile::plain_tcl().grammar,
        );
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let results = [
            Analyser::new().analyse(source, "tcl"),
            Analyser::new()
                .analyse_chunked(source, vec![commands.clone()], "tcl")
                .0,
            Analyser::new().analyse_commands(source, &commands, "tcl", true),
        ];
        for analysis in results {
            let receipt =
                source_declared_command_words(source, &analysis, &last_command(source, &analysis))
                    .unwrap();
            assert_eq!(
                receipt.descriptor().provenance(),
                tcl_dialect::model::Provenance::Document
            );
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Expr)])
            );
        }
    }

    fn command_at(
        source: &str,
        analysis: &AnalysisResult,
        fragment: &str,
    ) -> crate::segmenter::SegmentedCommand {
        crate::segmenter::segment_commands_with_offset_and_config(
            fragment,
            u32::try_from(source.find(fragment).unwrap()).unwrap(),
            analysis.body_lexer_config.unwrap(),
        )
        .remove(0)
    }

    fn logical_receipt_at(
        source: &str,
        fragment: &str,
    ) -> (AnalysisResult, crate::segmenter::SegmentedCommand) {
        let analysis = Analyser::new().analyse(source, "tcl");
        let command = command_at(source, &analysis, fragment);
        (analysis, command)
    }

    #[test]
    fn original_declared_words_retain_qualified_and_nested_procedure_scope() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        use crate::command_binding::{
            OriginalDeclaredSourceBodyFrame, OriginalDeclaredSourceObligation,
        };
        let prefix = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub check {value}\n# tcl-lsp: stubs-end\n# tcl-lsp: stubs-begin\n# tcl-lsp: stub ::N::check {expression:expr}\n# tcl-lsp: stubs-end\n";
        for definition in [
            "proc ::N::p {} {check {$value + 1}}",
            "proc ::N::p {} {if {1} {check {$value + 1}}}",
            "proc ::N::p {} {proc q {} {check {$value + 1}}}",
        ] {
            let source = format!("{prefix}{definition}");
            let (analysis, command) = logical_receipt_at(&source, "check {$value + 1}");
            let receipt =
                source_declared_command_words(&source, &analysis, &command).expect(definition);
            let scope = receipt.logical_body_context().expect(definition);
            assert_eq!(
                scope.frame_recipe(),
                OriginalDeclaredSourceBodyFrame::Procedure
            );
            assert_eq!(scope.namespace_recipe().as_segments()[0].as_bytes(), b"N");
            assert_eq!(receipt.descriptor().name, "::N::check");
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Expr)])
            );
            assert_eq!(receipt.written_argument(0), Some(0));
            assert_eq!(
                scope.body().original_container().image(),
                &tcl_lexer::SourceImage::document(&source)
            );
            assert!(
                receipt
                    .obligations()
                    .contains(&OriginalDeclaredSourceObligation::DeferredLogicalBodyApplicability)
            );
            assert!(
                crate::registry_invocation::source_structure::source_registry_words(
                    &source, &analysis, &command
                )
                .is_none()
            );
            assert!(
                source_declared_command_words(&(source.clone() + " "), &analysis, &command)
                    .is_none()
            );
        }
    }

    #[test]
    fn original_declared_body_scope_keeps_global_fallback_without_root_stamping() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        for (definition, namespace) in [
            ("proc p {} {check {argument}}", None),
            ("proc ::N::p {} {check {argument}}", Some("N")),
        ] {
            let source = format!(
                "# tcl-lsp: stubs-begin\n# tcl-lsp: stub check {{value}}\n# tcl-lsp: stubs-end\n{definition}"
            );
            let (analysis, command) = logical_receipt_at(&source, "check {argument}");
            let receipt =
                source_declared_command_words(&source, &analysis, &command).expect(definition);
            let scope = receipt.logical_body_context().unwrap();
            assert_eq!(receipt.descriptor().name, "check");
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Value)])
            );
            match namespace {
                Some(name) => assert_eq!(
                    scope.namespace_recipe().as_segments()[0].as_bytes(),
                    name.as_bytes()
                ),
                None => assert!(scope.namespace_recipe().is_root()),
            }
        }
    }

    #[test]
    fn original_declared_words_retain_static_namespace_source_ancestry() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        use crate::command_binding::OriginalDeclaredSourceBodyFrame;
        for (definition, expected) in [
            ("namespace eval ::N {check {$value + 1}}", vec!["N"]),
            (
                "namespace eval ::N {namespace eval child {check {$value + 1}}}",
                vec!["N", "child"],
            ),
            (
                "namespace eval ::N {proc p {} {check {$value + 1}}}",
                vec!["N"],
            ),
        ] {
            let name = format!("::{}::check", expected.join("::"));
            let source = format!(
                "# tcl-lsp: stubs-begin\n# tcl-lsp: stub {name} {{expression:expr}}\n# tcl-lsp: stubs-end\n{definition}"
            );
            let (analysis, command) = logical_receipt_at(&source, "check {$value + 1}");
            let receipt =
                source_declared_command_words(&source, &analysis, &command).expect(definition);
            let scope = receipt.logical_body_context().unwrap();
            let namespace: Vec<_> = scope
                .namespace_recipe()
                .as_segments()
                .iter()
                .map(tcl_core_types::NameBytes::as_bytes)
                .collect();
            assert_eq!(
                namespace,
                expected.into_iter().map(str::as_bytes).collect::<Vec<_>>()
            );
            assert_eq!(
                scope.frame_recipe(),
                if definition.contains("proc p") {
                    OriginalDeclaredSourceBodyFrame::Procedure
                } else {
                    OriginalDeclaredSourceBodyFrame::Namespace
                }
            );
            assert_eq!(receipt.descriptor().name, name);
            assert_eq!(
                scope.scope_name_input().original_word().image(),
                &tcl_lexer::SourceImage::document(&source)
            );
        }
    }

    #[test]
    fn original_declared_body_scope_refuses_unowned_names_and_known_replacements() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let prefix = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub check {expression:expr}\n# tcl-lsp: stubs-end\n# tcl-lsp: stubs-begin\n# tcl-lsp: stub ::N::check {expression:expr}\n# tcl-lsp: stubs-end\n";
        for body in [
            "proc ::N::p {} {proc check {args} {}; check {$value + 1}}",
            "proc ::N::p {} {rename check {}; check {$value + 1}}",
            "namespace eval $unknown {check {$value + 1}}",
            "namespace eval [set name ::N] {check {$value + 1}}",
            "set data {check {$value + 1}}",
            "namespace inscope ::N {check {$value + 1}}",
        ] {
            let source = format!("{prefix}{body}");
            let (analysis, command) = logical_receipt_at(&source, "check {$value + 1}");
            assert!(
                source_declared_command_words(&source, &analysis, &command).is_none(),
                "{body}"
            );
        }
        let source = format!(
            "{prefix}namespace eval ::N {{proc ::check {{args}} {{}}}}\ncheck {{$value + 1}}"
        );
        let analysis = Analyser::new().analyse(&source, "tcl");
        assert!(
            source_declared_command_words(&source, &analysis, &last_command(&source, &analysis))
                .is_none()
        );
    }

    #[test]
    fn original_declared_words_require_the_authentic_stub_block() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        for profile in ["tcl8.6", "tcl9.0", "tcl"] {
            let orphan = "# tcl-lsp: stub check {expression:expr}\ncheck {$value + 1}";
            let analysis = Analyser::new().analyse(orphan, profile);
            assert!(
                source_declared_command_words(orphan, &analysis, &last_command(orphan, &analysis),)
                    .is_none()
            );
            let framed = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub check {expression:expr}\n# tcl-lsp: stubs-end\ncheck {$value + 1}";
            let analysis = Analyser::new().analyse(framed, profile);
            let receipt =
                source_declared_command_words(framed, &analysis, &last_command(framed, &analysis))
                    .expect(profile);
            assert_eq!(receipt.descriptor().name, "check");
            assert_eq!(
                receipt.supplied_argument_roles(),
                Some(vec![(0, tcl_registry::ArgRole::Expr)])
            );
            assert_eq!(
                receipt.argument_word(0).unwrap().image(),
                &tcl_lexer::SourceImage::document(framed)
            );
        }
    }
}
