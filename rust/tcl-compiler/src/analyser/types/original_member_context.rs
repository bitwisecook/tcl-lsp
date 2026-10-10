// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly lexical member identity, separately from entered receiver frames.

use super::{
    AnalysisResult, ClassDef, MemberSide, OriginalSourceMethodMetadata,
    OriginalSourceSpecialMemberMetadata,
};
use crate::{
    analyser::ResolvedAnalysisInput,
    command_binding::CommandAllocationSite,
    signature_scan::{
        original_name::SourceDeclarationMetadata,
        scope::{
            SignatureSourceCommand, SignatureSourceLookup, SignatureSourceNameKey,
            SourceCommandPublication,
        },
    },
};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Token};
use tcl_registry::{
    native_compilation::NativeCompilationGrammar, native_tcloo_compilation::NativeTclOoHelper,
};
use tcl_syntax::{
    naming::{NativeNameProtocol, NativeNameQualification},
    word_rules::WordValueRules,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderArity {
    Absent,
    Signature(tcl_registry::Arity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Declaration {
    Method(Box<OriginalSourceMethodMetadata>),
    Special(Box<OriginalSourceSpecialMemberMetadata>),
}

/// Genuine source class publication, canonical member and whole body word.
/// This supplies no installed allocation, entered frame or dispatch receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLexicalMemberContext {
    class: SignatureSourceCommand,
    declaration: Declaration,
    body: NativeWord,
    input: ResolvedAnalysisInput,
}

impl OriginalLexicalMemberContext {
    pub(crate) fn from_class_body(
        class: &ClassDef,
        body: &Token,
        parameters: Option<&Token>,
        input: ResolvedAnalysisInput,
        image: &SourceImage,
        config: LexerConfig,
    ) -> Option<Self> {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let publication = class.source_name.as_ref()?;
        if publication.publication() != SourceCommandPublication::TclOoObject
            || !matches!(publication.policy().recipe(), NativeNameProtocol::C(_))
        {
            return None;
        }
        let matches = |word: &NativeWord, formal: Option<&NativeWord>| {
            word.image() == image
                && word.config() == config
                && word.tokens().contains(body)
                && match (parameters, formal) {
                    (Some(token), Some(word)) => {
                        word.image() == image
                            && word.config() == config
                            && word.tokens().contains(token)
                    }
                    (None, None) => true,
                    _ => false,
                }
        };
        let methods = class
            .original_members
            .declarations()
            .filter(|method| {
                method.side() == MemberSide::Instance
                    && !method.native_class_delegate()
                    && method.name_purpose()
                        == tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod
                    && method.original_name_input().policy() == publication.policy()
                    && method
                        .body_word()
                        .is_some_and(|body| matches(body, method.parameters_word()))
            })
            .map(|method| Declaration::Method(Box::new(method.clone())));
        let special = class
            .original_special_members
            .declarations()
            .filter(|method| {
                method.side() == MemberSide::Instance
                    && matches(method.body_word(), method.parameters_word())
            })
            .map(|method| Declaration::Special(Box::new(method.clone())));
        let mut selected = methods.chain(special);
        let declaration = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let body = match &declaration {
            Declaration::Method(method) => method.body_word()?.clone(),
            Declaration::Special(method) => method.body_word().clone(),
        };
        Some(Self {
            class: publication.clone(),
            declaration,
            body,
            input,
        })
    }

    /// Exact source publication; this is not a current class allocation.
    #[must_use]
    pub const fn class_publication(&self) -> &SignatureSourceCommand {
        &self.class
    }
    /// Canonical ordinary declaration, independently of a later moved route.
    #[must_use]
    pub fn ordinary_method(&self) -> Option<&OriginalSourceMethodMetadata> {
        match &self.declaration {
            Declaration::Method(method) => Some(method),
            Declaration::Special(_) => None,
        }
    }
    /// Canonical constructor/destructor role, without a fabricated member name.
    #[must_use]
    pub fn special_member(&self) -> Option<&OriginalSourceSpecialMemberMetadata> {
        match &self.declaration {
            Declaration::Special(method) => Some(method),
            Declaration::Method(_) => None,
        }
    }
    /// Exact source-only instance table axis; class-object bodies do not borrow it.
    #[must_use]
    pub fn side(&self) -> MemberSide {
        match &self.declaration {
            Declaration::Method(method) => method.side(),
            Declaration::Special(method) => method.side(),
        }
    }
    /// Actual declaration worker site, independently of its reporting label.
    #[must_use]
    pub fn declaration_site(&self) -> &CommandAllocationSite {
        match &self.declaration {
            Declaration::Method(method) => method.declaration().site(),
            Declaration::Special(method) => method.site(),
        }
    }
    /// Whole original script role; no body execution follows from retention.
    #[must_use]
    pub const fn body_word(&self) -> &NativeWord {
        &self.body
    }
    /// Complete immutable source configuration and Registry context.
    #[must_use]
    pub const fn resolved_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Exact source/configuration correspondence, without editable geometry.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.body_word().image() == image
            && self.body_word().config() == config
            && self.input.lexer_config() == config
    }
    /// Unique retained source class with this canonical declaration. A duplicate
    /// publication or missing declaration declines; runtime ownership is separate.
    #[must_use]
    pub fn source_class<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ClassDef>> {
        if analysis.resolved_input.as_ref() != Some(&self.input) {
            return None;
        }
        let mut records = analysis
            .original_class_declarations()
            .filter(|class| class.name() == &self.class);
        let record = records.next()?;
        if records.next().is_some() {
            return None;
        }
        let declared =
            match &self.declaration {
                Declaration::Method(method) => record
                    .metadata()
                    .original_members
                    .declarations()
                    .any(|candidate| {
                        candidate.declaration() == method.declaration()
                            && candidate.side() == method.side()
                            && candidate.parameters_word() == method.parameters_word()
                            && candidate.body_word() == method.body_word()
                    }),
                Declaration::Special(method) => record
                    .metadata()
                    .original_special_members
                    .declarations()
                    .any(|candidate| {
                        candidate.site() == method.site()
                            && candidate.kind() == method.kind()
                            && candidate.side() == method.side()
                            && candidate.body_word() == method.body_word()
                            && candidate.parameters_word() == method.parameters_word()
                    }),
            };
        declared.then_some(record)
    }
}

/// Exact Registry source helper shape retained with its lexical body owner.
/// Relative `nextto` targets lacking the actual object namespace remain unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLexicalNextCall {
    context: OriginalLexicalMemberContext,
    invocation: crate::registry_invocation::source_structure::OriginalRegistryWords,
    original_words: Vec<NativeWord>,
    target: Option<SignatureSourceLookup>,
}

impl OriginalLexicalNextCall {
    pub(crate) fn from_source(
        context: &OriginalLexicalMemberContext,
        analysis: &AnalysisResult,
        source: &str,
        head: &Token,
    ) -> Option<Self> {
        if !context.matches_source(&SourceImage::document(source), context.input.lexer_config()) {
            return None;
        }
        if analysis.resolved_input.as_ref() != Some(&context.input) {
            return None;
        }
        let range = context.body_word().content_span().ok()?;
        let body =
            source.get(usize::try_from(range.start()).ok()?..usize::try_from(range.end()).ok()?)?;
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            range.start(),
            context.input.lexer_config(),
        )
        .into_iter()
        .find(|command| command.argv.first() == Some(head))?;
        let original_tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            context.input.lexer_config(),
            &command,
        );
        let original_words = crate::registry_invocation::original_native_compiler_words(
            &SourceImage::document(source),
            original_tokens.words(),
            head.span.start(),
            context.input.lexer_config(),
        )?;
        let invocation = crate::registry_invocation::source_structure::source_registry_words(
            source, analysis, &command,
        );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_LEXICAL_NEXT offset={} vector={} retained_schema={}",
                head.span.start(),
                invocation.is_some(),
                analysis
                    .original_conditional_registry_metadata_in_source(
                        &SourceImage::document(source),
                        context.input.lexer_config(),
                        head.span.start(),
                    )
                    .is_some(),
            );
        }
        let invocation = invocation?;
        let registry = context.input.context_registry();
        let helper = invocation.with_source_schema(&registry, |schema| {
            schema.semantics.native_compilation.map(|spec| spec.grammar)
        });
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_LEXICAL_NEXT offset={} schema={helper:?} arguments={}",
                head.span.start(),
                invocation.arguments().len(),
            );
        }
        let helper = helper?;
        let has_target = match helper? {
            NativeCompilationGrammar::TclOoHelper(NativeTclOoHelper::Next) => false,
            NativeCompilationGrammar::TclOoHelper(NativeTclOoHelper::NextTo) => true,
            _ => return None,
        };
        let target = if has_target {
            let original = invocation.operands().first()?.as_ref()?.input()?;
            let namespace = context.class.body_scope()?;
            let projection = original
                .policy()
                .recipe()
                .command_lookup_input(namespace.context()?, original.bytes())
                .ok()?;
            if projection.qualification() != NativeNameQualification::Absolute {
                return None;
            }
            Some(SignatureSourceLookup::from_input(namespace, original)?)
        } else {
            None
        };
        Some(Self {
            context: context.clone(),
            invocation,
            original_words,
            target,
        })
    }
    /// Actual selected source form contains an independently retained class operand.
    #[must_use]
    pub const fn has_target_class(&self) -> bool {
        self.target.is_some()
    }
    /// Genuine lexical context; it cannot substitute for an entered receiver.
    #[must_use]
    pub const fn member_context(&self) -> &OriginalLexicalMemberContext {
        &self.context
    }
    /// Whole source invocation and its retained applicability obligations.
    #[must_use]
    pub const fn original_invocation(
        &self,
    ) -> &crate::registry_invocation::source_structure::OriginalRegistryWords {
        &self.invocation
    }

    /// Complete original call geometry, including an expanded trailing word.
    /// Captured operands retain no written call-site word of their own.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original_words
    }

    /// Argument count from the authentic selected effective vector. A target
    /// class operand is data and excluded only by the retained helper grammar.
    #[must_use]
    pub fn source_argument_count(&self) -> Option<tcl_registry::InvocationArgumentCount> {
        let context = self.context.input.context_registry();
        self.invocation.with_source_schema(&context, |schema| {
            tcl_registry::resolved_invocation::count_invocation_argument_layout(
                tcl_registry::Arity::any(),
                schema.words.arguments(),
                usize::from(self.has_target_class()),
                schema.semantics.options,
            )
        })?
    }

    pub(crate) fn source_arity(
        &self,
        analysis: &AnalysisResult,
        source: &str,
    ) -> Option<tcl_registry::Arity> {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let image = SourceImage::document(source);
        let config = self.context.input.lexer_config();
        if !self.context.matches_source(&image, config)
            || !self.invocation.matches_source(&image, config)
        {
            return None;
        }
        let class = self.context.source_class(analysis)?;
        let order =
            crate::analyser::class_hierarchy::original_metadata::original_instance_metadata_order(
                analysis, class,
            )?;
        let start = self.provider_start(analysis, &order, class)?;
        for provider in order.iter().skip(start) {
            if let ProviderArity::Signature(arity) = self.provider_arity(provider.metadata())? {
                return Some(arity);
            }
        }

        None
    }

    fn provider_start(
        &self,
        analysis: &AnalysisResult,
        order: &[&SourceDeclarationMetadata<ClassDef>],
        class: &SourceDeclarationMetadata<ClassDef>,
    ) -> Option<usize> {
        let declaring = order
            .iter()
            .position(|provider| provider.declaration_site() == class.declaration_site())?;
        let Some(target) = &self.target else {
            return declaring.checked_add(1);
        };
        let occupied = analysis
            .original_procedure_declarations()
            .map(|record| (record.name(), None));
        let selected = target.first_matching_publications(
            analysis
                .original_class_declarations()
                .map(|record| (record.name(), Some(record.declaration_site())))
                .chain(occupied),
        );
        let [Some(site)] = selected.as_slice() else {
            return None;
        };
        let target = order
            .iter()
            .position(|provider| provider.declaration_site() == *site)?;
        (target > declaring).then_some(target)
    }

    fn provider_arity(&self, provider: &ClassDef) -> Option<ProviderArity> {
        let parameters = match &self.context.declaration {
            Declaration::Method(method) => {
                let methods = provider.original_members.methods(MemberSide::Instance)?;
                let input = method.original_name_input();
                let Some(candidate) = methods.into_iter().find(|candidate| {
                    candidate.original_name_input().policy() == input.policy()
                        && candidate.original_name_input().bytes() == input.bytes()
                }) else {
                    return Some(ProviderArity::Absent);
                };
                if candidate.native_class_delegate()
                    || candidate.name_purpose()
                        != tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod
                {
                    return None;
                }
                candidate.parameters_word()?.clone()
            }
            Declaration::Special(method) => {
                let Some(candidate) = provider
                    .original_special_members
                    .declarations()
                    .filter(|candidate| {
                        candidate.side() == MemberSide::Instance
                            && candidate.kind() == method.kind()
                    })
                    .last()
                else {
                    return Some(ProviderArity::Absent);
                };
                if candidate.kind()
                    == tcl_registry::definer::DefinitionSpecialMemberKind::Destructor
                {
                    return Some(ProviderArity::Signature(tcl_registry::Arity::exact(0)));
                }
                candidate.parameters_word()?.clone()
            }
        };
        Some(ProviderArity::Signature(parameter_arity(
            &parameters,
            self.context.class.policy(),
            self.invocation.dialect()?,
        )?))
    }
}

pub(super) fn parameter_arity(
    word: &NativeWord,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    dialect: tcl_registry::InvocationDialect,
) -> Option<tcl_registry::Arity> {
    let key = SignatureSourceNameKey::from_original_native_word(
        word,
        WordValueRules::from_config(&word.config()),
        policy,
    )?;
    let formals = crate::signature_scan::formal_parameters::SignatureSourceFormalParameters::from_original_input(&key, dialect)?;
    crate::signature_scan::arity::arity_from_count_shape(formals.argument_count_shape())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, Scope};

    fn contexts(scope: &Scope) -> Vec<&OriginalLexicalMemberContext> {
        let mut found = scope
            .original_member_context
            .as_deref()
            .into_iter()
            .collect::<Vec<_>>();
        found.extend(scope.children.iter().flat_map(contexts));
        found
    }

    fn method<'a>(analysis: &'a AnalysisResult, bytes: &[u8]) -> &'a OriginalLexicalMemberContext {
        contexts(&analysis.global_scope)
            .into_iter()
            .find(|context| {
                context
                    .ordinary_method()
                    .is_some_and(|method| method.original_name_input().bytes() == bytes)
            })
            .unwrap()
    }

    fn call(
        context: &OriginalLexicalMemberContext,
        analysis: &AnalysisResult,
        source: &str,
    ) -> Option<OriginalLexicalNextCall> {
        let range = context.body_word().content_span().ok()?;
        let body =
            source.get(usize::try_from(range.start()).ok()?..usize::try_from(range.end()).ok()?)?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            range.start(),
            context.resolved_input().lexer_config(),
        );
        OriginalLexicalNextCall::from_source(
            context,
            analysis,
            source,
            commands.first()?.argv.first()?,
        )
    }

    #[test]
    fn original_lexical_member_keeps_colons_lifecycle_and_source_ownership_separate() {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let source = "oo::class create C {method a::b {} {my a::b}; constructor {} {}; destructor {}; self method a::b {} {}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_classes.clear();
            let context = method(&analysis, b"a::b");
            assert_eq!(
                context.class_publication().slot().simple.as_bytes(),
                b"C",
                "{dialect}"
            );
            assert_eq!(context.side(), MemberSide::Instance);
            assert!(context.source_class(&analysis).is_some());
            assert!(context.matches_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap()
            ));
            assert_eq!(
                contexts(&analysis.global_scope).len(),
                3,
                "{dialect}: class-object body is separate"
            );
            let special = contexts(&analysis.global_scope)
                .into_iter()
                .filter_map(OriginalLexicalMemberContext::special_member)
                .collect::<Vec<_>>();
            assert_eq!(special.len(), 2);
            assert!(special.iter().any(|method| method.kind()
                == tcl_registry::definer::DefinitionSpecialMemberKind::Constructor));
            assert!(special.iter().any(|method| method.kind()
                == tcl_registry::definer::DefinitionSpecialMemberKind::Destructor));
            let offset = u32::try_from(source.find("my a::b").unwrap()).unwrap();
            let realm = analysis.retained_command_realm().unwrap();
            let binding = realm
                .source_bindings_ref()
                .invocation_at_source("my", offset);
            assert!(
                binding
                    .receiver_self_method_entry(analysis.resolved_registry().unwrap())
                    .is_none(),
                "{dialect}: declaration context is not an entered receiver"
            );
            assert!(!context.matches_source(
                &SourceImage::document(&source.replace("my a::b", "my other")),
                analysis.body_lexer_config.unwrap()
            ));
            let mut foreign = analysis.clone();
            let original = foreign.resolved_input.as_ref().unwrap();
            let generation = original.context_registry();
            foreign.resolved_input = Some(ResolvedAnalysisInput::new(
                original.analyser_profile(),
                original.unit_profile(),
                std::sync::Arc::new(
                    generation
                        .with_command_store(generation.commands().snapshot().shared_registry()),
                ),
                original.lexer_config(),
            ));
            assert!(
                context.source_class(&foreign).is_none(),
                "independent Registry/source entry remains distinct"
            );
        }
    }

    #[test]
    fn original_lexical_next_uses_counted_member_and_complete_source_mro() {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        let source = "oo::class create Base {method a::b {first {second value}} {}}\n\
                      oo::class create Middle {superclass Base}\n\
                      oo::class create C {superclass Middle; method a::b {} {next 1 2 3}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let context = contexts(&analysis.global_scope)
                .into_iter()
                .find(|context| context.class_publication().slot().simple.as_bytes() == b"C")
                .unwrap();
            let call = call(context, &analysis, source).expect(dialect);
            assert_eq!(call.member_context(), context);
            assert_eq!(
                call.source_arity(&analysis, source),
                Some(tcl_registry::Arity::new(1, 2)),
                "{dialect}"
            );
            assert!(
                call.source_arity(&analysis, &source.replace("value", "other"))
                    .is_none()
            );
        }
        let mixed = "oo::class create M {method a::b {arg} {}}\n\
                     oo::class create C {mixin M; method a::b {} {next 1 2}}";
        let analysis = Analyser::new().analyse(mixed, "tcl9.0");
        let context = contexts(&analysis.global_scope)
            .into_iter()
            .find(|context| context.class_publication().slot().simple.as_bytes() == b"C")
            .unwrap();
        let class = context.source_class(&analysis).unwrap();
        let order =
            crate::analyser::class_hierarchy::original_metadata::original_instance_metadata_order(
                &analysis, class,
            )
            .unwrap();
        assert_eq!(
            order
                .iter()
                .map(|class| class.name().slot().simple.as_bytes())
                .collect::<Vec<_>>(),
            [b"M".as_slice(), b"C".as_slice()]
        );
        // A mixin precedes this lexical declaring class. Its source advice
        // does not become a following provider or actual receiver dispatch.
        assert!(
            call(context, &analysis, mixed)
                .unwrap()
                .source_arity(&analysis, mixed)
                .is_none()
        );
    }

    #[test]
    fn original_lexical_next_count_uses_effective_words_and_whole_call_geometry() {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        for (invoke, minimum, indeterminate) in [
            ("next 1 2", 2, false),
            ("::::next 1 2 3", 3, false),
            ("next 1 {*}$unknown", 1, true),
            ("nextto ::Base 1 2", 2, false),
        ] {
            let source = format!(
                "oo::class create Base {{method f {{a b}} {{}}}}\noo::class create C {{superclass Base; method f {{}} {{{invoke}}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let context = contexts(&analysis.global_scope)
                .into_iter()
                .find(|context| context.class_publication().slot().simple.as_bytes() == b"C")
                .unwrap();
            let next = call(context, &analysis, &source).expect(invoke);
            let count = next.source_argument_count().expect(invoke);
            assert_eq!(
                (count.minimum, count.indeterminate),
                (minimum, indeterminate),
                "{invoke}"
            );
            let first = next.original_words().first().unwrap().span();
            let last = next.original_words().last().unwrap().span();
            assert_eq!(
                &source
                    [usize::try_from(first.start()).unwrap()..usize::try_from(last.end()).unwrap()],
                invoke
            );
            assert_eq!(
                next.source_arity(&analysis, &source),
                Some(tcl_registry::Arity::exact(2))
            );
        }
    }

    #[test]
    fn original_lexical_next_declines_relative_targets_and_occupied_forward_routes() {
        // naming.tcloo.original-lexical-member-context
        // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
        for (invocation, expected) in [("nextto ::Base 1 2", true), ("nextto Base 1 2", false)] {
            let source = format!(
                "oo::class create Base {{method a::b {{arg}} {{}}}}\n\
                                  oo::class create C {{superclass Base; method a::b {{}} {{{invocation}}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let context = contexts(&analysis.global_scope)
                .into_iter()
                .find(|context| context.class_publication().slot().simple.as_bytes() == b"C")
                .unwrap();
            let selected = call(context, &analysis, &source);
            assert_eq!(selected.is_some(), expected, "{invocation}");
            if let Some(selected) = selected {
                assert_eq!(
                    selected.source_arity(&analysis, &source),
                    Some(tcl_registry::Arity::exact(1))
                );
            }
        }
        let source = "oo::class create Base {forward a::b puts}\n\
                      oo::class create C {superclass Base; method a::b {} {next 1 2 3}}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let context = method(&analysis, b"a::b");
        assert!(
            call(context, &analysis, source)
                .unwrap()
                .source_arity(&analysis, source)
                .is_none()
        );
    }
}
