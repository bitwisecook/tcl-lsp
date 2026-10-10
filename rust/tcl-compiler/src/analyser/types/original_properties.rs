// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original property declaration advice, independent of method/accessor slots.

use super::{MemberSide, PropertyDef};
use crate::signature_scan::{
    original_name::SourceOriginalNameOccurrence, scope::SignatureSourceNameInput,
};
use tcl_registry::commands::tcl::TclOoPropertyKind;

/// A Registry-selected property declaration. Its counted source name and
/// getter/setter bodies remain original values; neither supplies a generated
/// method name, native property installation or option-lookup correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourcePropertyMetadata {
    declaration: SourceOriginalNameOccurrence,
    input: SignatureSourceNameInput,
    side: MemberSide,
    kind: TclOoPropertyKind,
    metadata: PropertyDef,
    getter: Option<SignatureSourceNameInput>,
    setter: Option<SignatureSourceNameInput>,
    accessor_advice: tcl_registry::definer::SourcePropertyAccessorAdvice,
}

impl OriginalSourcePropertyMetadata {
    pub(crate) fn new(
        declaration: SourceOriginalNameOccurrence,
        side: MemberSide,
        metadata: PropertyDef,
        kind: TclOoPropertyKind,
        getter: Option<SignatureSourceNameInput>,
        setter: Option<SignatureSourceNameInput>,
        accessor_advice: tcl_registry::definer::SourcePropertyAccessorAdvice,
    ) -> Option<Self> {
        let key = declaration.name_input();
        if accessor_advice.recipe() != key.policy().recipe()
            || metadata.name_span != key.span()
            || getter
                .iter()
                .chain(setter.iter())
                .any(|input| input.policy() != key.policy())
        {
            return None;
        }
        let input = SignatureSourceNameInput::OriginalWord(key.clone());
        Some(Self {
            declaration,
            input,
            side,
            kind,
            metadata,
            getter,
            setter,
            accessor_advice,
        })
    }

    /// Exact original property word and independently retained definition site.
    #[must_use]
    pub const fn declaration(&self) -> &SourceOriginalNameOccurrence {
        &self.declaration
    }
    /// Counted property value, without method-name or option-string projection.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Purpose-selected formatted option candidate, retaining the original
    /// declaration separately. This supplies no installed property or reached
    /// accessor lookup. Raw zero and native encoded zero stay independent.
    #[must_use]
    pub fn option_name(&self) -> Option<tcl_syntax::naming::NativeNameProjection<'_>> {
        self.input
            .policy()
            .recipe()
            .oo_property_option_name(self.input.bytes())
            .ok()
    }
    /// Authentic definition receiver axis at this declaration.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Registry-selected kind for this property's own option sequence.
    #[must_use]
    pub const fn kind(&self) -> TclOoPropertyKind {
        self.kind
    }
    /// Reporting metadata only; its String name cannot supply identity.
    #[must_use]
    pub const fn metadata(&self) -> &PropertyDef {
        &self.metadata
    }
    /// Original custom getter body, when a complete producer was retained.
    #[must_use]
    pub const fn getter(&self) -> Option<&SignatureSourceNameInput> {
        self.getter.as_ref()
    }
    /// Authored accessor names proposed by the independently selected grammar.
    /// These are source completion advice, without native method installation.
    #[must_use]
    pub const fn accessor_methods(&self) -> &'static [&'static str] {
        self.accessor_advice.methods()
    }
    /// Argument-name geometry of the declaration's selected source accessor
    /// protocol. The caller separately owns actual receiver/selector currency.
    #[must_use]
    pub fn accessor_layout(
        &self,
        selector: &SignatureSourceNameInput,
        arguments: usize,
    ) -> Option<tcl_registry::definer::SourcePropertyAccessorLayout> {
        if selector.policy() != self.input.policy()
            || matches!(selector, SignatureSourceNameInput::OriginalVariableRoot(_))
        {
            return None;
        }
        self.accessor_advice.layout(selector.bytes(), arguments)
    }
    /// Prospective property-name completion role at a genuine existing or
    /// immediately next operand. This remains distinct from invocation validity.
    #[must_use]
    pub fn accessor_completion_role(
        &self,
        selector: &SignatureSourceNameInput,
        ordinal: usize,
        arguments: usize,
    ) -> Option<tcl_registry::definer::SourcePropertyCompletionRole> {
        if selector.policy() != self.input.policy()
            || matches!(selector, SignatureSourceNameInput::OriginalVariableRoot(_))
        {
            return None;
        }
        self.accessor_advice
            .completion_name_role(selector.bytes(), ordinal, arguments)
    }
    /// Exact canonical source property and whole declaration image. This
    /// readonly correspondence does not require an entered definition body and
    /// grants no installed property, reached accessor or editable option value.
    #[must_use]
    pub fn matches_original_declaration_source(
        &self,
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        provider: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ClassDef,
        >,
    ) -> bool {
        // naming.tcloo.original-property-accessor-source-advice
        // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
        let Some(receipt) = crate::command_binding::OriginalSourceClassDeclaration::from_class(
            source, analysis, provider,
        ) else {
            return false;
        };
        let Some(canonical) = receipt.source_class(analysis) else {
            return false;
        };
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        let Some(key) = self.input.original_word_key() else {
            return false;
        };
        std::ptr::eq(canonical, provider)
            && canonical
                .metadata()
                .original_properties
                .declarations()
                .any(|property| std::ptr::eq(property, self))
            && key == self.declaration.name_input()
            && key.original_word().image() == &tcl_lexer::SourceImage::document(source)
            && key.original_word().config() == input.lexer_config()
    }

    /// Original custom setter body, when a complete producer was retained.
    #[must_use]
    pub const fn setter(&self) -> Option<&SignatureSourceNameInput> {
        self.setter.as_ref()
    }
}

/// Ordered source property declarations. Missing declaration-name coverage is
/// explicit; the reporting map cannot complete a withdrawn receiver axis.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginalSourcePropertyLedger {
    declarations: Vec<OriginalSourcePropertyMetadata>,
    unavailable: Vec<MemberSide>,
}

impl OriginalSourcePropertyLedger {
    pub(crate) fn declare(&mut self, declaration: OriginalSourcePropertyMetadata) {
        self.declarations.push(declaration);
    }
    pub(crate) fn withdraw(&mut self, side: MemberSide) {
        if !self.unavailable.contains(&side) {
            self.unavailable.push(side);
        }
    }
    pub(crate) fn absorb(&mut self, other: &Self, other_ran_second: bool) {
        for side in &other.unavailable {
            self.withdraw(*side);
        }
        let mut declarations = if other_ran_second {
            std::mem::take(&mut self.declarations)
        } else {
            other.declarations.clone()
        };
        let tail = if other_ran_second {
            &other.declarations
        } else {
            &self.declarations
        };
        for declaration in tail {
            if !declarations.contains(declaration) {
                declarations.push(declaration.clone());
            }
        }
        self.declarations = declarations;
    }
    /// Canonical original declarations, including a subsequently redefined name.
    pub fn declarations(&self) -> impl Iterator<Item = &OriginalSourcePropertyMetadata> {
        self.declarations.iter()
    }
    /// Last observed source declaration for each exact counted name/policy.
    /// This is source candidate metadata, without table existence or dispatch.
    #[must_use]
    pub fn properties(&self, side: MemberSide) -> Option<Vec<&OriginalSourcePropertyMetadata>> {
        if self.unavailable.contains(&side) {
            return None;
        }
        let mut selected: Vec<&OriginalSourcePropertyMetadata> = Vec::new();
        for declaration in self
            .declarations
            .iter()
            .filter(|declaration| declaration.side == side)
        {
            selected.retain(|old| !same_name(&old.input, &declaration.input));
            selected.push(declaration);
        }
        Some(selected)
    }
    /// Exact own source declaration; caller owns class identity and currency.
    #[must_use]
    pub fn property_for_input(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
    ) -> Option<&OriginalSourcePropertyMetadata> {
        if matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_)) {
            return None;
        }
        self.properties(side)?
            .into_iter()
            .find(|property| same_name(&property.input, input))
    }
}

fn same_name(left: &SignatureSourceNameInput, right: &SignatureSourceNameInput) -> bool {
    left.policy() == right.policy() && left.bytes() == right.bytes()
}
