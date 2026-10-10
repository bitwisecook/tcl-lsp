// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Constructor signature advice from a genuine original class factory call.

use super::{AnalysisResult, MemberSide};
use crate::command_binding::{OriginalSourceConstructorCall, OriginalSourceConstructorShape};
use tcl_lexer::{SourceImage, Span};

/// Complete source factory/call join with an independently retained source-order
/// obligation. This supplies no class allocation, entered constructor or result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalConstructorArityCall {
    original: OriginalSourceConstructorCall,
    enforce_order: bool,
}
/// Issued readonly constructor signature assessment. Transport consumers read
/// the emitter's retained count; they do not reselect a shape or parse messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalConstructorArityAdvice {
    call: std::sync::Arc<OriginalConstructorArityCall>,
    count: tcl_registry::InvocationArgumentCount,
    arity: tcl_registry::Arity,
}
impl OriginalConstructorArityAdvice {
    pub(crate) fn assess(
        call: std::sync::Arc<OriginalConstructorArityCall>,
        analysis: &AnalysisResult,
        source: &str,
    ) -> Option<Self> {
        let arity = call.source_arity(analysis, source)?;
        let count = call.source_argument_count(analysis)?;
        Some(Self { call, count, arity })
    }
    /// Genuine original source call, including declaration and applicability.
    #[must_use]
    pub fn call(&self) -> &OriginalConstructorArityCall {
        &self.call
    }
    /// Emitting owner's complete effective count and expansion obligations.
    #[must_use]
    pub const fn argument_count(&self) -> &tcl_registry::InvocationArgumentCount {
        &self.count
    }
    /// Readonly source signature, without a successful constructor receipt.
    #[must_use]
    pub const fn arity(&self) -> tcl_registry::Arity {
        self.arity
    }
}

impl OriginalConstructorArityCall {
    pub(crate) fn from_source(
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
        enforce_order: bool,
    ) -> Option<Self> {
        let original = crate::registry_invocation::source_structure::source_constructor_call_at(
            source, analysis, offset,
        )?;
        Some(Self {
            original,
            enforce_order,
        })
    }
    /// Retained original factory, class publication, lineage and complete argv.
    #[must_use]
    pub const fn original_call(&self) -> &OriginalSourceConstructorCall {
        &self.original
    }
    /// Exact argument axis excludes the selected manufacturer keyword only.
    /// Structural creation operands remain counted independently of formals.
    #[must_use]
    pub fn source_argument_count(
        &self,
        analysis: &AnalysisResult,
    ) -> Option<tcl_registry::InvocationArgumentCount> {
        let shape = self.original.constructor_shape(analysis)?;
        let offset = usize::from(matches!(shape, OriginalSourceConstructorShape::Method(_)));
        let arguments = self.original.arguments();
        let words = arguments
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        tcl_registry::resolved_invocation::count_invocation_argv(
            tcl_registry::InvocationArguments::structured(&words),
            offset,
        )
    }
    /// Whole original extent; captured words cannot supply a written endpoint.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        Some(Span::new(
            self.original.original_words().first()?.span().start(),
            self.original.original_words().last()?.span().end(),
        ))
    }
    pub(crate) fn source_arity(
        &self,
        analysis: &AnalysisResult,
        source: &str,
    ) -> Option<tcl_registry::Arity> {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let input = analysis.resolved_input.as_ref()?;
        let context = input.context_registry();
        self.original
            .matches_source_context(
                &SourceImage::document(source),
                input.lexer_config(),
                &context,
            )
            .then_some(())?;
        let declaration = self.original.class_declaration();
        let grammar = declaration.grammar(&context)?;
        // This consumer asks TclOO lifecycle signature questions. Provider
        // bare-name syntax remains a distinct selected source construction role.
        if grammar.family != tcl_registry::definer::DefinerFamily::TclOo {
            return None;
        }
        let class = declaration.source_class(analysis)?;
        let shape = self.original.constructor_shape(analysis)?;
        let prefix = usize::from(matches!(shape, OriginalSourceConstructorShape::Method(_)));
        let extra = u16::try_from(shape.constructor_args_from().checked_sub(prefix)?).ok()?;
        let order =
            crate::analyser::class_hierarchy::original_metadata::original_instance_metadata_order(
                analysis, class,
            )?;
        for provider in order {
            let selected = provider
                .metadata()
                .original_special_members
                .declarations()
                .filter(|member| {
                    member.side() == MemberSide::Instance
                        && member.kind()
                            == tcl_registry::definer::DefinitionSpecialMemberKind::Constructor
                        && (!self.enforce_order
                            || member.site().offset < self.original.site().offset)
                })
                .last();
            let Some(selected) = selected else {
                continue;
            };
            // Inherited signatures retain their own genuine factory grammar,
            // naming policy and formal dialect; the child cannot issue them.
            let provider_declaration =
                crate::command_binding::OriginalSourceClassDeclaration::from_class(
                    source, analysis, provider,
                )?;
            let provider_grammar = provider_declaration.grammar(&context)?;
            if provider_grammar.family != tcl_registry::definer::DefinerFamily::TclOo {
                return None;
            }
            let policy = provider.name().policy();
            let dialect = provider_declaration.factory().dialect();
            // naming.tcloo.empty-lifecycle-definition
            // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
            // Static source body value, not a trimmed script or native argv.
            let values = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                std::slice::from_ref(selected.body_word()),
                policy.string_protocol(),
            )
            .ok()?;
            match provider_grammar.source_special_member_formal_validation_applicability(
                selected.kind(), values.literal(0),
            )? {
                tcl_registry::definer::SourceFormalValidationApplicability::SkippedRemovedLifecycle => continue,
                tcl_registry::definer::SourceFormalValidationApplicability::Required => {}
            }
            let arity = super::original_member_context::parameter_arity(
                selected.parameters_word()?,
                policy,
                dialect,
            )?;
            return bumped(arity, extra);
        }
        bumped(tcl_registry::Arity::at_least(0), extra)
    }
}
fn bumped(arity: tcl_registry::Arity, extra: u16) -> Option<tcl_registry::Arity> {
    crate::signature_scan::arity::arity_from_count_shape(
        tcl_syntax::formal_params::FormalArgumentCountShape {
            minimum: usize::from(arity.min).checked_add(usize::from(extra))?,
            maximum: if arity.is_unlimited() {
                None
            } else {
                Some(usize::from(arity.max).checked_add(usize::from(extra))?)
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    #[test]
    fn original_constructor_signatures_keep_canonical_factory_and_effective_count() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for (prefix, invoke, count, unknown) in [
            ("", "C new 1", 1, false),
            ("rename C moved; ", "moved new 1", 1, false),
            ("interp alias {} make {} C new; ", "make 1", 1, false),
            ("interp alias {} make {} C new fixed; ", "make", 1, false),
            ("", "C new 1 {*}$tail", 1, true),
        ] {
            let source =
                format!("oo::class create C {{constructor {{a b}} {{ }}}}\n{prefix}{invoke}");
            let mut analysis = Analyser::new().analyse(&source, "tcl9.0");
            analysis.all_classes.clear();
            let offset = u32::try_from(source.rfind(invoke).unwrap()).unwrap();
            let call = OriginalConstructorArityCall::from_source(&source, &analysis, offset, true)
                .expect(invoke);
            let advice = OriginalConstructorArityAdvice::assess(
                std::sync::Arc::new(call),
                &analysis,
                &source,
            )
            .expect(invoke);
            assert_eq!(advice.arity(), tcl_registry::Arity::exact(2));
            assert_eq!(
                (
                    advice.argument_count().minimum,
                    advice.argument_count().indeterminate
                ),
                (count, unknown)
            );
            assert_eq!(&source[advice.call().span().unwrap().as_range()], invoke);
            assert!(
                advice
                    .call()
                    .original_call()
                    .class_declaration()
                    .source_class(&analysis)
                    .is_some()
            );
            assert!(
                advice
                    .call()
                    .source_arity(&analysis, &source.replace("a b", "a c"))
                    .is_none()
            );
        }
    }
    #[test]
    fn original_constructor_signature_respects_exact_lifecycle_body_values() {
        // naming.tcloo.empty-lifecycle-definition
        // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
        // The pinned ASCII CLI probe measures declaration/call behaviour;
        // this test checks conditional source signature advice only.
        for (parameters, body, expected) in [
            ("a b", "", Some(tcl_registry::Arity::at_least(0))),
            ("{a b c}", "", Some(tcl_registry::Arity::at_least(0))),
            ("a b", " ", Some(tcl_registry::Arity::exact(2))),
            ("a b", "# comment", Some(tcl_registry::Arity::exact(2))),
            ("{a b c}", " ", None),
        ] {
            let source =
                format!("oo::class create C {{constructor {{{parameters}}} {{{body}}}}}\nC new 1");
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
            let call = OriginalConstructorArityCall::from_source(&source, &analysis, offset, true)
                .unwrap();
            assert_eq!(
                call.source_arity(&analysis, &source),
                expected,
                "parameters={parameters:?} body={body:?}"
            );
        }
        let source = "oo::class create Base {constructor {a b} { }}\noo::class create C {superclass Base; constructor {a b c} {}}\nC new 1";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
        let call =
            OriginalConstructorArityCall::from_source(source, &analysis, offset, true).unwrap();
        assert_eq!(
            call.source_arity(&analysis, source),
            Some(tcl_registry::Arity::exact(2))
        );
    }

    #[test]
    fn original_constructor_signatures_keep_mandatory_names_and_known_replacements() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for (invoke, arity, count) in [
            ("C create", tcl_registry::Arity::at_least(1), 0),
            ("C new a b", tcl_registry::Arity::at_least(0), 2),
        ] {
            let source = format!("oo::class create C {{}}\n{invoke}");
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let offset = u32::try_from(source.rfind(invoke).unwrap()).unwrap();
            let call = OriginalConstructorArityCall::from_source(&source, &analysis, offset, true)
                .unwrap();
            assert_eq!(call.source_arity(&analysis, &source), Some(arity));
            assert_eq!(
                call.source_argument_count(&analysis).unwrap().minimum,
                count
            );
        }
        for replacement in ["rename C {}; ", "proc C args {}; "] {
            let source =
                format!("oo::class create C {{constructor {{a b}} {{ }}}}\n{replacement}C new 1");
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
            assert!(
                OriginalConstructorArityCall::from_source(&source, &analysis, offset, true)
                    .is_none()
            );
        }
    }
}
