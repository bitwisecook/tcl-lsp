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
    reporting_label: String,
    namespace: SourceNamespaceKey,
    parameters_text: String,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    declaration_span: Option<Span>,
    module_entry: Option<SourceAnalysisEntry>,
    module_profile: Option<&'static tcl_dialect::DialectProfile>,
    lowered_body: Option<Arc<Script>>,
    source_scope: Option<Arc<super::OriginalAuthoredSourceBodyScope>>,
    conditional_source_geometry:
        Option<Arc<crate::command_binding::OriginalAuthoredProcedureSourceGeometry>>,
}

pub(crate) fn original_authored_procedure_body(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<OriginalAuthoredProcedureBody> {
    if let Some(geometry) = tokens
        .source_binding
        .as_ref()?
        .original_authored_procedure_source_geometry(tokens, registry)
    {
        let body_word = geometry.body_word().clone();
        let body_span = super::source_scoped_body::original_written_script_body_span(&body_word)?;
        return Some(OriginalAuthoredProcedureBody {
            header: tokens.clone(),
            site: tokens.source_binding.as_ref()?.invocation_site()?.clone(),
            body_word,
            body_span,
            config: geometry.config(),
            registry: registry.snapshot().semantic_key(),
            name: geometry.name().to_owned(),
            reporting_label: geometry.reporting_label(),
            namespace: geometry.namespace().clone(),
            parameters_text: geometry.parameters_text().to_owned(),
            parameters: geometry.parameters().to_vec(),
            declaration_span: None,
            module_entry: None,
            module_profile: None,
            lowered_body: None,
            source_scope: None,
            conditional_source_geometry: Some(Arc::new(geometry)),
        });
    }
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
    (site.source.source_image() == body_word.image()).then_some(())?;
    Some(OriginalAuthoredProcedureBody {
        header: tokens.clone(),
        site,
        body_word,
        body_span,
        config: grammar.config(),
        registry: registry.snapshot().semantic_key(),
        name: name_text,
        reporting_label: tcl_syntax::naming::authored_source_command_spelling(&coordinate)?,
        namespace,
        parameters_text,
        parameters,
        declaration_span: None,
        module_entry: None,
        module_profile: None,
        lowered_body: None,
        source_scope: None,
        conditional_source_geometry: None,
    })
}

impl OriginalAuthoredProcedureBody {
    pub(crate) fn has_unclosed_declaration_source_coverage(&self) -> bool {
        self.conditional_source_geometry
            .as_ref()
            .is_some_and(|geometry| geometry.has_unclosed_source_coverage())
    }

    pub(crate) fn reporting_label(&self, module: &Module, procedure: &Procedure) -> Option<&str> {
        self.matches_procedure(module, procedure)
            .then_some(self.reporting_label.as_str())
    }

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
    /// Descriptive source arity from this unchanged original header/formals.
    /// This supplies no Native formal slots, activation or accepted call.
    pub(crate) fn original_argument_count_shape(
        &self,
        module: &Module,
        procedure: &Procedure,
    ) -> Option<tcl_syntax::formal_params::FormalArgumentCountShape> {
        self.matches_procedure(module, procedure).then_some(())?;
        Some(tcl_syntax::formal_params::formal_argument_count_shape(
            self.parameters
                .iter()
                .map(|formal| (formal.name == "args", formal.default.is_some())),
            module.parameter_grammar()?,
        ))
    }

    pub(crate) fn parameter_names(&self) -> impl Iterator<Item = &str> {
        self.parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
    }
    pub(crate) fn create_source_scope(
        &self,
        module: &Module,
    ) -> Option<Arc<super::OriginalAuthoredSourceBodyScope>> {
        super::OriginalAuthoredSourceBodyScope::capture(
            &self.header,
            &self.body_word,
            &self.namespace,
            module,
        )
    }
    pub(crate) fn retain_lowered_body(
        mut self,
        body: &Script,
        module: &Module,
        declaration_span: Span,
        scope: Option<Arc<super::OriginalAuthoredSourceBodyScope>>,
    ) -> Self {
        self.source_scope = scope;
        self.declaration_span = Some(declaration_span);
        self.lowered_body = Some(Arc::new(body.clone()));
        self.module_entry = Some(module.source_entry.clone());
        self.module_profile = module.dialect_profile;
        self
    }
    pub(crate) fn owns_original_source_scope(
        &self,
        scope: &super::OriginalAuthoredSourceBodyScope,
    ) -> bool {
        self.source_scope
            .as_ref()
            .is_some_and(|original| scope.belongs_to_original_scope(original))
    }

    pub(crate) fn covers_installed_analysis_source(
        &self,
        module: &Module,
        procedure: &Procedure,
        installed: &crate::command_binding::SourceInstalledProcedureBody,
    ) -> bool {
        self.matches_procedure(module, procedure)
            && installed.allocation.incarnation
                == crate::command_binding::AllocationIncarnation::First
            && installed.allocation.site == self.site
            && installed.namespace_key == self.namespace
            && installed.parameters == self.parameters
            && installed.source.origin == self.site.source
            && installed.source.mapping
                == crate::command_binding::ExecutedScriptMapping::Contiguous {
                    base: self.body_span.start(),
                }
            && self
                .body_word
                .image()
                .bytes()
                .get(self.body_span.as_range())
                == Some(installed.source.text.bytes())
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
            || self.body_word.image() != &module.source
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
            || self
                .source_scope
                .as_ref()
                .is_some_and(|scope| !scope.matches_module(module))
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
            && original.reporting_label == self.reporting_label
            && original.namespace == self.namespace
            && original.parameters_text == self.parameters_text
            && original.parameters == self.parameters
            && original.conditional_source_geometry == self.conditional_source_geometry
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
    fn conditional_authored_scalar_geometry_cannot_supply_a_definite_ssa_definition() {
        let registry = CommandRegistry::build_default();
        let source = "rename $old decl\nproc kept {} {set value 1; return $value}";
        let unit = crate::compilation_unit::CompilationUnit::build_for_with_config(
            source,
            &registry,
            false,
            tcl_lexer::LexerConfig::default(),
        );
        let module = &unit.ir_module;
        let (&id, card) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "kept")
            .expect("actual conditional scalar source declaration");
        assert!(card.matches_procedure(module, &module.body_units[&id]));
        assert!(card.has_unclosed_declaration_source_coverage());
        assert!(unit.function_for_body(Some(id)).is_some());
        let write = u32::try_from(source.find("set value").unwrap()).unwrap();
        assert!(
            crate::ssa::original_analysis_definition_at_unit(
                &unit,
                tcl_lexer::Span::new(write + 4, write + 9),
                &registry,
            )
            .is_none()
        );
    }

    #[test]
    fn conditional_authored_declaration_geometry_keeps_original_world_and_closed_refusals() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "rename $old decl\nproc kept {arg} {list $arg; return untouched}";
        let module = crate::lowering::lower_to_ir(source, registry);
        let (&id, card) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "kept")
            .expect("original conditional declaration, not an installed procedure");
        let procedure = &module.body_units[&id];
        assert!(card.matches_procedure(&module, procedure));
        assert!(card.has_unclosed_declaration_source_coverage());
        assert!(procedure.publication.is_none());
        assert!(procedure.body.executed_source.is_none());
        assert!(!module.installed_procedure_body_units.contains_key(&id));
        let scope = card
            .source_scope
            .as_ref()
            .expect("actual original body scope");
        assert!(scope.matches_module(&module));
        assert!(scope.has_unclosed_declaration_source_coverage());
        let geometry = card
            .header
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(&card.header, registry)
            .expect("same conditional source row and original vector");
        assert!(geometry.has_unclosed_source_coverage());
        assert_eq!(geometry.body_word().image(), &module.source);
        assert_eq!(geometry.body_word().config(), module.native_lexer_config());
        assert_eq!(
            geometry
                .body_word()
                .image()
                .bytes()
                .get(card.body_span.as_range()),
            Some(b"list $arg; return untouched".as_slice())
        );
        assert!(
            super::super::original_authored_source_structure(&card.header, registry)
                .is_none_or(|source| source.into_closed_source_grammar().is_none())
        );
        assert!(
            super::super::admitted_native_compiler_invocation(registry, None, &card.header)
                .is_none()
        );
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .expect("original conditional body vector");
            assert!(super::super::original_authored_scalar_invocation(tokens, registry).is_none());
        }
        let mut altered = card.header.clone();
        altered.argv_texts[1] = "changed".to_owned();
        assert!(
            altered
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_procedure_source_geometry(&altered, registry)
                .is_none()
        );
        let mut changed = module.clone();
        changed.source =
            tcl_lexer::SourceImage::document(&source.replace("untouched", "different"));
        assert!(!card.matches_procedure(&changed, &changed.body_units[&id]));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(
            card.header
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_procedure_source_geometry(&card.header, foreign)
                .is_none()
        );
        for unsupported in [
            "rename $old decl\n$declare kept {arg} {list retained}",
            "rename $old decl\nproc kept {arg(0)} {list retained}",
            "proc proc args {}; proc kept {arg} {list retained}",
        ] {
            let rejected = crate::lowering::lower_to_ir(unsupported, registry);
            assert!(
                rejected
                    .original_authored_declaration_body_units
                    .values()
                    .all(|card| card.name() != "kept"),
                "{unsupported}"
            );
        }
    }

    #[test]
    fn authored_source_declaration_bodies_are_not_independent_executable_roots() {
        let registry = CommandRegistry::build_default();
        let source = "proc subject {} {set value 1; return OK}\nset root VALUE\n";
        let module = crate::lowering::lower_to_ir(source, &registry);
        assert_eq!(module.original_authored_declaration_body_units.len(), 1);
        let roots = module.executable_script_roots();
        assert!(
            roots
                .iter()
                .any(|(script, _)| std::ptr::eq(*script, &raw const module.top_level))
        );
        for (&id, card) in &module.original_authored_declaration_body_units {
            assert!(module.procedure_ids.owns(id));
            let procedure = module.body_units.get(&id).unwrap();
            assert!(card.matches_procedure(&module, procedure));
            assert!(
                roots
                    .iter()
                    .all(|(script, _)| !std::ptr::eq(*script, &raw const procedure.body))
            );
            assert!(
                module
                    .original_source_procedure_bodies()
                    .any(|(owned, body)| { *owned == id && std::ptr::eq(body, procedure) })
            );
            assert!(procedure.publication.is_none());
            assert!(procedure.body.executed_source.is_none());
        }
    }

    fn assert_unentered_authored_body(body: &crate::ir::Script) {
        assert!(body.executed_source.is_none());
        assert!(body.native_compilation_admission.is_none());
        assert!(body.native_compilation_failure.is_none());
    }

    fn assert_bytecode_keeps_authored_bodies_unpublished(registry: &CommandRegistry, source: &str) {
        let bytecode = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::new(registry).for_bytecode_backend(),
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
        assert_eq!(
            card.header,
            *module
                .top_level
                .retained_source_tokens_for_statement(&module.top_level.statements[0])
                .unwrap()
        );
        assert!(card.source_scope.as_ref().unwrap().matches_module(&module));
        let body_tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        assert!(
            body_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_authored_source_body_scope_for_name_roles(body_tokens, &registry)
                .is_some()
        );
        assert!(
            super::super::normal_representation_invocation_with_metadata_context(
                &registry,
                None,
                body_tokens
            )
            .is_none()
        );
        assert!(
            super::super::admitted_native_compiler_invocation(&registry, None, body_tokens)
                .is_none()
        );
        let scope = card.source_scope.as_ref().unwrap();
        assert!(scope.original_words(body_tokens, &registry).is_some());
        let mut stale = body_tokens.clone();
        stale.argv_texts[1] = "different".to_owned();
        assert!(scope.original_words(&stale, &registry).is_none());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        assert!(scope.original_words(body_tokens, foreign).is_none());
        assert_unentered_authored_body(&procedure.body);
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
                    changed.body_units.get_mut(id).unwrap().span =
                        Span::new(1, u32::try_from(source.len()).unwrap());
                }
                _ => unreachable!(),
            }
            assert!(
                !card.matches_procedure(&changed, changed.body_units.get(id).unwrap()),
                "facet {facet}"
            );
        }
        assert_bytecode_keeps_authored_bodies_unpublished(&registry, source);
    }
}
