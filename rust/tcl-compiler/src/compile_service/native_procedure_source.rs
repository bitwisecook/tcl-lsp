// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compiler-owned bodies from an original live procedure compilation request.

use crate::compile_service::BytecodeCompileService;
use crate::ir::{Module, Procedure, ProcedureBodyKind, ProcedureId, TopLevelKind};
use tcl_runtime_api::{
    CompileError, NativeCompilationEntry, OriginalProcedureCompileRequest, ProcedureDispatch,
};

/// A compiled body under its own compiler Module issuer. Its declaration
/// receipt retains no native role and establishes no callable publication.
pub struct NativeCompiledProcedure {
    module: Module,
    id: ProcedureId,
    request: OriginalProcedureCompileRequest,
}

impl NativeCompiledProcedure {
    /// Lower only the immutable operands supplied by the original declaration.
    ///
    /// # Errors
    /// Refuses unavailable declaration ownership, metadata or source geometry.
    pub fn lower(
        request: &OriginalProcedureCompileRequest,
        registry: &tcl_registry::CommandRegistry,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self, CompileError> {
        if !request.matches_entry(entry) || entry.profile != profile.cache_key() {
            return Err(CompileError::Unsupported(
                "original procedure compilation owner is unavailable".to_owned(),
            ));
        }
        let target = request.target();
        let text = target.source.try_text().map_err(|error| {
            CompileError::Unsupported(format!("original procedure text is unavailable: {error}"))
        })?;
        let ordinary = request.ordinary_formals().ok_or_else(|| {
            CompileError::Unsupported(
                "ordinary original formal bindings are unavailable".to_owned(),
            )
        })?;
        if BytecodeCompileService::native_entry_options(entry, Some(profile))
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::parameter_grammar)
            != Some(tcl_dialect::ParameterGrammar::Tcl)
        {
            return Err(CompileError::Unsupported(
                "active ordinary formal policy is unavailable".to_owned(),
            ));
        }
        let params = ordinary
            .iter()
            .map(|name| name.try_utf8().map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| CompileError::Unsupported(error.to_string()))?;
        let config = BytecodeCompileService::native_entry_config(
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            Some(entry),
        );
        let mut module =
            crate::lowering::lower_original_procedure_bytes_module_for_bytecode_with_options(
                request,
                registry,
                config,
                Some(profile),
                dispatch == ProcedureDispatch::Plain,
                Some(BytecodeCompileService::native_entry_options(
                    entry,
                    Some(profile),
                )),
            )
            .map_err(|error| CompileError::Unsupported(format!("{error:?}")))?;
        let end = u32::try_from(target.source.len())
            .map_err(|error| CompileError::Unsupported(error.to_string()))?;
        let body = std::mem::take(&mut module.top_level);
        let id = module.retain_body_unit(Procedure {
            name: request.reported_name().to_owned(),
            qualified_name: request.reported_name().to_owned(),
            publication: None,
            declaration_site: None,
            body_kind: ProcedureBodyKind::Installed,
            params,
            span: tcl_lexer::Span::new(0, end),
            body,
            // Bound storage names do not recover the original invocation header.
            params_raw: String::new(),
            body_source: Some(text.to_owned()),
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        });
        Ok(Self {
            module,
            id,
            request: request.clone(),
        })
    }

    /// Whether two observations name the same original native declaration.
    /// Identity equality does not establish current ownership or safe execution.
    #[must_use]
    pub fn same_declaration(&self, other: &Self) -> bool {
        self.request.declaration() == other.request.declaration()
    }

    /// The genuine compiler unit; an Installed body has no callable publication.
    #[must_use]
    pub const fn module(&self) -> &Module {
        &self.module
    }

    /// Borrow the issued body only under its unchanged native source context.
    /// This checks custody; escape analysis and compiler admission still decide
    /// whether physical relocation or frame elision is safe.
    #[must_use]
    pub fn original_compiled_body<'a>(
        &self,
        module: &'a Module,
        registry: &tcl_registry::CommandRegistry,
        entry: &NativeCompilationEntry,
    ) -> Option<(ProcedureId, &'a Procedure)> {
        if !self.request.matches_entry(entry)
            || self.request.ordinary_formals().is_none()
            || self.request.target().source.len() != self.request.original_source().len()
            || !module.procedure_ids.owns(self.id)
            || module.source != self.module.source
            || module.source_entry != self.module.source_entry
            || module.source_entry.native_entry.as_deref() != Some(entry)
            || module.source_metadata_input != self.module.source_metadata_input
            || module.lexer_config != self.module.lexer_config
            || module.native_namespace != self.module.native_namespace
            || module.top_level_kind != TopLevelKind::ProcedureBody
            || module.top_level != self.module.top_level
            || registry.snapshot().semantic_key()
                != self.module.resolved_registry().snapshot().semantic_key()
        {
            return None;
        }
        let retained = module.retained_source_bindings.as_ref()?;
        let original = self.module.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(module, registry)?;
        if !std::sync::Arc::ptr_eq(retained, original) {
            return None;
        }
        let procedure = module.body_units.get(&self.id)?;
        (self.module.body_units.get(&self.id) == Some(procedure)).then_some((self.id, procedure))
    }
}
