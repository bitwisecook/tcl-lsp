// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original standalone declaration bodies retain their independent source owner.

use crate::command_binding::{CommandAllocationSite, SourceAnalysisEntry, SourceNamespaceKey};
use crate::ir::{CommandTokens, Module, Procedure, ProcedureBodyKind, Script};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, NativeWord, Span};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Conditional source declaration syntax; no publication, frame or Native entry.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalAuthoredProcedureBody {
    header: CommandTokens,
    site: CommandAllocationSite,
    body_word: NativeWord,
    body_span: Span,
    config: LexerConfig,
    registry: RegistrySemanticKey,
    name: String,
    namespace: SourceNamespaceKey,
    parameters_text: String,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    declaration_span: Option<Span>,
    module_entry: Option<SourceAnalysisEntry>,
    module_profile: Option<&'static tcl_dialect::DialectProfile>,
    lowered_body: Option<Arc<Script>>,
}

pub(crate) fn original_authored_procedure_body(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<OriginalAuthoredProcedureBody> {
    let grammar = super::original_authored_source_structure(tokens, registry)?
        .into_closed_source_grammar()?;
    let positions = grammar.with_source_schema(registry, |schema| {
        schema.authored_source_procedure_arguments()
    })?;
    // The retained standalone source key proves this root scope. Other source
    // namespace purposes need their own component-preserving scope producer.
    let SourceNamespaceKey::Authored(current) = grammar.source_namespace() else {
        return None;
    };
    (current == "::" && grammar.config().list_parse == tcl_dialect::ListParse::Strict)
        .then_some(())?;
    let name_word = grammar.original_argument_word(positions.name)?;
    let name = tcl_syntax::word_rules::original_static_word_unicode_value(name_word)?;
    let name_text = std::str::from_utf8(&name).ok()?.to_owned();
    let coordinate = tcl_syntax::naming::authored_source_command_slot(
        &tcl_core_types::ByteNamespacePath::root(),
        &name,
    )?;
    let namespace = SourceNamespaceKey::authored(tcl_syntax::naming::checked_namespace_path_key(
        &coordinate.namespace,
    )?);
    let parameters_text = grammar.argument_literal(positions.parameters)?;
    let parameters = tcl_syntax::formal_params::parse_formal_parameters(&parameters_text).ok()?;
    let body_word = grammar.original_body_word(positions.body)?.clone();
    let body_span = super::source_scoped_body::original_written_script_body_span(&body_word)?;
    let site = tokens.source_binding.as_ref()?.invocation_site()?.clone();
    (site.source.source_image() == body_word.source_image()).then_some(())?;
    Some(OriginalAuthoredProcedureBody {
        header: tokens.clone(),
        site,
        body_word,
        body_span,
        config: grammar.config(),
        registry: registry.snapshot().semantic_key(),
        name: name_text,
        namespace,
        parameters_text,
        parameters,
        declaration_span: None,
        module_entry: None,
        module_profile: None,
        lowered_body: None,
    })
}

impl OriginalAuthoredProcedureBody {
    pub(crate) fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    pub(crate) fn body_span(&self) -> Span {
        self.body_span
    }
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn parameters_text(&self) -> &str {
        &self.parameters_text
    }
    pub(crate) fn parameter_names(&self) -> impl Iterator<Item = &str> {
        self.parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
    }
    pub(crate) fn retain_lowered_body(
        mut self,
        body: &Script,
        module: &Module,
        declaration_span: Span,
    ) -> Self {
        self.declaration_span = Some(declaration_span);
        self.lowered_body = Some(Arc::new(body.clone()));
        self.module_entry = Some(module.source_entry.clone());
        self.module_profile = module.dialect_profile;
        self
    }
    pub(crate) fn matches_procedure(&self, module: &Module, procedure: &Procedure) -> bool {
        if module.source_metadata_input.is_some()
            || module.source_entry.native_entry.is_some()
            || module.source_entry.hosted_execution_context.is_some()
            || module.source_entry.unknown_entry
            || self.module_entry.as_ref() != Some(&module.source_entry)
            || self.module_profile != module.dialect_profile
            || !module
                .registry_snapshot
                .as_ref()
                .is_some_and(|owner| owner.semantic_key() == self.registry)
            || self.config != module.native_lexer_config()
            || self.registry != module.resolved_registry().snapshot().semantic_key()
            || self.body_word.source_image() != &module.source
            || procedure.body_kind != ProcedureBodyKind::OriginalDeclaration
            || procedure.publication.is_some()
            || procedure.declaration_site.as_ref() != Some(&self.site)
            || self.declaration_span != Some(procedure.span)
            || procedure.span.start() != self.site.offset
            || procedure.name != self.name
            || procedure.body_offset != self.body_span.start()
            || procedure.params_raw != self.parameters_text
            || !procedure
                .params
                .iter()
                .map(String::as_str)
                .eq(self.parameter_names())
            || procedure.body.namespace_context.as_deref() != Some(&self.namespace)
            || procedure.body.executed_source.is_some()
            || self.lowered_body.as_deref() != Some(&procedure.body)
        {
            return false;
        }
        let Some(original) =
            original_authored_procedure_body(&self.header, module.resolved_registry())
        else {
            return false;
        };
        original.site == self.site
            && original.body_word == self.body_word
            && original.body_span == self.body_span
            && original.config == self.config
            && original.registry == self.registry
            && original.name == self.name
            && original.namespace == self.namespace
            && original.parameters_text == self.parameters_text
            && original.parameters == self.parameters
            && procedure.body_source.as_deref()
                == module
                    .source
                    .try_text()
                    .ok()
                    .and_then(|text| text.get(self.body_span.as_range()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_declaration_cards_retain_actual_source_bodies_without_publication() {
        let registry = CommandRegistry::build_default();
        let source = "proc {$literal} {value} {set out $value}";
        let module = crate::lowering::lower_to_ir(source, &registry);
        assert!(module.procedures.is_empty());
        assert!(module.source_entry.native_entry.is_none());
        assert!(module.source_metadata_input.is_none());
        assert_eq!(module.original_authored_declaration_body_units.len(), 1);
        let (id, card) = module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = module.body_units.get(id).unwrap();
        assert!(card.matches_procedure(&module, procedure));
        assert_eq!(procedure.name, "$literal");
        assert_eq!(procedure.params, ["value"]);
        assert!(procedure.publication.is_none());
        assert!(
            super::super::admitted_native_compiler_invocation(&registry, None, &card.header)
                .is_none()
        );
        assert!(
            super::super::normal_representation_invocation_with_metadata_context(
                &registry,
                None,
                &card.header
            )
            .is_none()
        );
        assert!(procedure.body.executed_source.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
        assert!(procedure.body.native_compilation_failure.is_none());
        assert!(
            module
                .original_source_procedure_bodies()
                .any(|(owned, _)| owned == id)
        );
        let mut changed = module.clone();
        changed.body_units.get_mut(id).unwrap().qualified_name = "report label only".to_owned();
        assert!(card.matches_procedure(&changed, changed.body_units.get(id).unwrap()));
        for facet in 0..8 {
            let mut changed = module.clone();
            match facet {
                0 => changed.source = tcl_lexer::SourceImage::document("different source"),
                1 => changed.body_units.get_mut(id).unwrap().body_kind = ProcedureBodyKind::Named,
                2 => changed.body_units.get_mut(id).unwrap().params_raw = "different".to_owned(),
                3 => changed
                    .body_units
                    .get_mut(id)
                    .unwrap()
                    .body
                    .statements
                    .clear(),
                4 => changed.registry_snapshot = None,
                5 => changed.dialect_profile = Some(tcl_dialect::DialectProfile::plain_tcl()),
                6 => changed.source_entry.unknown_entry = true,
                7 => {
                    changed.body_units.get_mut(id).unwrap().span = Span::new(1, source.len() as u32)
                }
                _ => unreachable!(),
            }
            assert!(
                !card.matches_procedure(&changed, changed.body_units.get(id).unwrap()),
                "facet {facet}"
            );
        }
        let bytecode = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::new(&registry).bytecode(),
            source,
        );
        assert!(bytecode.original_authored_declaration_body_units.is_empty());
        assert!(!bytecode.top_level.statements.is_empty());
        assert!(bytecode.top_level.statements.iter().any(|statement| {
            bytecode
                .top_level
                .retained_source_tokens_for_statement(statement)
                .is_some_and(|tokens| {
                    tokens.argv_texts == ["proc", "$literal", "value", "set out $value"]
                })
        }));
    }
}
