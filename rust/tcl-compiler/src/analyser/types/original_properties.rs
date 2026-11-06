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
    producer_input: crate::analyser::ResolvedAnalysisInput,
}

impl OriginalSourcePropertyMetadata {
    pub(crate) fn new(
        declaration: SourceOriginalNameOccurrence,
        side: MemberSide,
        metadata: PropertyDef,
        kind: TclOoPropertyKind,
        getter: Option<SignatureSourceNameInput>,
        setter: Option<SignatureSourceNameInput>,
        provenance: (
            tcl_registry::definer::SourcePropertyAccessorAdvice,
            crate::analyser::ResolvedAnalysisInput,
        ),
    ) -> Option<Self> {
        let (accessor_advice, producer_input) = provenance;
        for input in getter.iter().chain(setter.iter()) {
            match input {
                SignatureSourceNameInput::OriginalWord(_)
                | SignatureSourceNameInput::OriginalValue(_) => {}
                SignatureSourceNameInput::OriginalVariableRoot(_)
                | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
            }
        }
        let key = declaration.name_input();
        if accessor_advice.recipe() != key.policy().recipe()
            || key.lexer_config() != producer_input.lexer_config()
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
            producer_input,
        })
    }

    /// Whether this row was issued by the current analysis input and realm.
    /// This source-only check grants no class association or runtime state.
    #[must_use]
    pub fn matches_producer_input(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        analysis.resolved_input.as_ref().is_some_and(|input| {
            &self.producer_input == input
                && analysis.body_lexer_config == Some(input.lexer_config())
                && self.declaration.site().source.source_image()
                    == self.declaration.name_input().source_image()
                && analysis.matches_original_source_image(
                    self.declaration.name_input().source_image(),
                    input.lexer_config(),
                )
                && analysis.retained_command_realm().is_some_and(|realm| {
                    let bindings = realm.source_bindings_ref();
                    let origin = &self.declaration.site().source;
                    realm.matches_resolved_analysis_input(input)
                        && bindings.source_origin() == Some(origin)
                })
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
        match selector {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if selector.policy() != self.input.policy() {
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
        match selector {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if selector.policy() != self.input.policy() {
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
        if !self.matches_producer_input(analysis) {
            return false;
        }
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
    /// Whether every row on this side was issued by the current analysis.
    /// This does not assert that the side's inventory is complete.
    #[must_use]
    pub fn side_matches_producer_input(
        &self,
        side: MemberSide,
        analysis: &crate::analyser::AnalysisResult,
    ) -> bool {
        self.declarations
            .iter()
            .filter(|row| row.side == side)
            .all(|row| row.matches_producer_input(analysis))
    }

    /// Fold a side only when every contributing row belongs to this analysis.
    /// A foreign row cannot hide a current declaration or imply absence.
    #[must_use]
    pub fn properties_for_analysis(
        &self,
        side: MemberSide,
        analysis: &crate::analyser::AnalysisResult,
    ) -> Option<Vec<&OriginalSourcePropertyMetadata>> {
        self.side_matches_producer_input(side, analysis)
            .then(|| self.properties(side))?
    }

    /// Exact own source declaration; caller owns class identity and currency.
    #[must_use]
    pub fn property_for_input(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
    ) -> Option<&OriginalSourcePropertyMetadata> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        self.properties(side)?
            .into_iter()
            .find(|property| same_name(&property.input, input))
    }
}

fn same_name(left: &SignatureSourceNameInput, right: &SignatureSourceNameInput) -> bool {
    left.policy() == right.policy() && left.bytes() == right.bytes()
}

#[cfg(test)]
mod producer_context_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn property_fold_rejects_foreign_rows_before_same_name_selection() {
        let source = "oo::configurable create C {property size}";
        let current = Analyser::new().analyse(source, "tcl9.0");
        let input = current.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let distinct = std::sync::Arc::new(
            context.with_command_store(std::sync::Arc::clone(context.commands())),
        );
        let other_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            distinct,
            input.lexer_config(),
        );
        assert_ne!(&other_input, input);
        let foreign = Analyser::new()
            .with_resolved_input(other_input)
            .analyse(source, "tcl9.0");
        let owner = current.original_class_declarations().next().unwrap();
        let other = foreign.original_class_declarations().next().unwrap();
        let valid = &owner.metadata().original_properties;
        let alien = &other.metadata().original_properties;
        assert_eq!(
            valid
                .properties_for_analysis(MemberSide::Instance, &current)
                .unwrap()
                .len(),
            1
        );
        assert!(
            alien
                .properties_for_analysis(MemberSide::Instance, &current)
                .is_none()
        );
        // Exercise identical bytes/input with a deliberately different source owner.
        let mut wrong_origin = valid.declarations().next().unwrap().clone();
        assert!(wrong_origin.matches_producer_input(&current));
        let mut site = wrong_origin.declaration.site().clone();
        site.source = std::sync::Arc::new(crate::command_binding::SourceOriginId::loaded_image(
            "elsewhere.tcl".into(),
            "different owner".into(),
            wrong_origin.declaration.name_input().source_image().clone(),
        ));
        wrong_origin.declaration =
            crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(
                &site,
                wrong_origin.declaration.name_input().clone(),
            )
            .unwrap();
        assert!(!wrong_origin.matches_producer_input(&current));
        let another =
            Analyser::new().analyse("oo::configurable create C {property other}", "tcl9.0");
        let other_class = another.original_class_declarations().next().unwrap();
        let other_property = other_class
            .metadata()
            .original_properties
            .declarations()
            .next()
            .unwrap();
        assert!(!other_property.matches_producer_input(&current));
        for later in [true, false] {
            let mut combined = valid.clone();
            combined.absorb(alien, later);
            assert_eq!(combined.properties(MemberSide::Instance).unwrap().len(), 1);
            assert!(
                combined
                    .properties_for_analysis(MemberSide::Instance, &current)
                    .is_none()
            );
            assert_eq!(
                combined
                    .properties_for_analysis(MemberSide::ClassObject, &current)
                    .unwrap(),
                [] as [&crate::analyser::types::original_properties::OriginalSourcePropertyMetadata; 0]
            );
        }
    }
}

#[cfg(test)]
mod configuration_provenance_tests {
    use crate::analyser::Analyser;

    #[test]
    fn logical_input_property_row_keeps_its_producer_context() {
        let source = "oo::configurable create C {property size}";
        let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
        profile.name = "explicit-logical-property-producer";
        let profile = profile.intern();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let class = analysis
            .original_class_declarations()
            .next()
            .expect("logical class receipt");
        let property = class
            .metadata()
            .original_properties
            .declarations()
            .next()
            .expect("logical property receipt");
        assert!(property.matches_producer_input(&analysis));
    }

    #[test]
    fn inherited_source_property_row_keeps_its_producer_context() {
        let parent_source = "source child.tcl";
        let parent = Analyser::new().analyse(parent_source, "tcl9.0");
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            parent_source,
            &parent,
            0,
        )
        .expect("source load receipt");
        let point = load
            .source_point_entry(&parent)
            .expect("source point receipt");
        let source = "oo::configurable create C {property size}";
        let child = Analyser::new()
            .analyse_with_source_point_entry(source, "tcl9.0", &point)
            .expect("inherited analysis");
        let class = child
            .original_class_declarations()
            .next()
            .expect("inherited class receipt");
        let property = class
            .metadata()
            .original_properties
            .declarations()
            .next()
            .expect("inherited property receipt");
        assert!(property.matches_producer_input(&child));
    }

    #[test]
    fn current_configuration_property_rows_keep_their_producer_context() {
        let source = "oo::configurable create C {property initial}; oo::define C {property added}; C create obj; oo::objdefine obj {property own}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let class_rows = analysis
            .original_class_configurations()
            .flat_map(|row| row.properties().declarations())
            .collect::<Vec<_>>();
        assert!(
            !class_rows.is_empty(),
            "expected actual class-configuration property source rows"
        );
        assert!(
            class_rows
                .iter()
                .all(|row| row.matches_producer_input(&analysis))
        );
    }
}
