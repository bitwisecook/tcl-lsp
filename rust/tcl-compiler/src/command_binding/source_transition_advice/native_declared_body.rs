// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Native source headers retain their independent source body/schema.

use super::source_procedure::OriginalSourceProcedureBody;
use crate::command_binding::formal_topology::OriginalFormalTopology;
use crate::command_binding::{SourceCommandBindings, SourceNamespaceKey};
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalNativeSourceDeclaration {
    procedure: Arc<OriginalSourceProcedureBody>,
    name: SignatureSourceNameInput,
    formals: OriginalFormalTopology,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    namespace: SourceNamespaceKey,
    scope: SignatureNamespaceScope,
}
impl OriginalNativeSourceDeclaration {
    pub(crate) fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        self.procedure.declaration.site()
    }
    pub(crate) fn name_input(&self) -> &SignatureSourceNameInput {
        &self.name
    }
    pub(crate) fn slot(&self) -> &tcl_core_types::ByteCommandSlot {
        &self.procedure.declaration_slot
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        std::str::from_utf8(self.formals.original_input().bytes()).ok()
    }
    pub(crate) fn body_span(&self) -> tcl_lexer::Span {
        self.procedure.body.content_span()
    }
    pub(crate) fn body_text(&self) -> Option<&str> {
        self.site()
            .source
            .source_image()
            .try_text()
            .ok()?
            .get(self.body_span().as_range())
    }
    pub(crate) fn label(&self) -> Option<String> {
        std::str::from_utf8(&tcl_syntax::naming::native_command_full_name_bytes(
            self.slot(),
        ))
        .ok()
        .map(str::to_owned)
    }
    pub(crate) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        !input.has_logical_source_name_context()
            && !input.has_hosted_source_name_context()
            && self
                .procedure
                .declaration
                .matches_source(image, input.lexer_config())
            && self
                .procedure
                .declaration
                .matches_context(input.borrowed_context_registry())
            && self
                .procedure
                .body
                .matches_source(image, input.lexer_config())
            && self
                .procedure
                .body
                .matches_context(input.borrowed_context_registry())
            && self.formals.original_input().policy() == self.name.policy()
            && self.procedure.declaration.dialect().parameter_grammar()
                == Some(self.formals.parameter_grammar())
            && self.procedure.declaration_slot == self.procedure.slot
            && self.namespace.display().as_deref() == self.scope.display().as_deref()
    }
}
impl SourceCommandBindings {
    pub(crate) fn original_native_source_declarations(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<Arc<OriginalNativeSourceDeclaration>>> {
        if input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || self.source_metadata_owner().source_analysis_input() != Some(input)
        {
            return None;
        }
        let seed = self.original_source_transition_seed()?;
        let tape = seed.prepare_for(
            input.borrowed_context_registry(),
            super::OriginalSourceCataloguePurpose::CompletePrefix,
        )?;
        let inventory = tape.procedure_publications()?;
        if !inventory.matches_source_context(
            self.source_origin()?.source_image(),
            input.lexer_config(),
            input.borrowed_context_registry(),
        ) {
            return None;
        }
        let image = self.source_origin()?.source_image();
        let cards = inventory
            .original_declarations()
            .iter()
            .filter_map(|procedure| {
                let name = procedure.name.native_input()?;
                let roles = procedure.declaration.roles()?;
                let mut parameters = roles
                    .iter()
                    .filter(|(_, role)| *role == tcl_registry::ArgRole::ParamList);
                let argument = parameters.next()?.0;
                if parameters.next().is_some() {
                    return None;
                }
                let word = &procedure.declaration.arguments().get(argument)?.original;
                let producer = SignatureSourceNameKey::from_original_native_word(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
                    name.policy(),
                )?;
                let formals = OriginalFormalTopology::from_original_key(
                    producer,
                    procedure.declaration.dialect(),
                )?;
                let scope = match name.policy().recipe() {
                    tcl_syntax::naming::NativeNameProtocol::C(_) => {
                        SignatureNamespaceScope::C(procedure.declaration_slot.namespace.clone())
                    }
                    recipe @ tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                        SignatureNamespaceScope::Jim(
                            recipe
                                .jim_procedure_namespace(
                                    procedure.declaration_slot.simple.as_bytes(),
                                )
                                .ok()?
                                .into(),
                        )
                    }
                };
                let namespace = SourceNamespaceKey::authored(scope.display()?);
                let card = OriginalNativeSourceDeclaration {
                    procedure: Arc::clone(procedure),
                    name: name.clone(),
                    parameters: formals.advisory_parameters(),
                    formals,
                    namespace,
                    scope,
                };
                card.matches_source(image, input).then(|| Arc::new(card))
            })
            .collect();
        Some(cards)
    }
}
