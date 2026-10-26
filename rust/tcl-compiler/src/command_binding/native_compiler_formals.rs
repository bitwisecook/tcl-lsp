// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declaration-owned compiler formal slots, never authored header syntax.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use tcl_core_types::NameBytes;
use tcl_runtime_api::{NativeCompilationEntry, OriginalProcedureCompileRequest};
use tcl_syntax::naming::{NativeCompiledVariableEnvironment, NativeCompiledVariableProtocol};

/// Issued only while lowering this original live native procedure request.
/// This predicts compiler declaration slots; it supplies no entered activation,
/// local values, Normal completion, handler effects or physical frame layout.
#[derive(Debug, Clone)]
pub(crate) struct OriginalNativeCompilerFormals {
    request: OriginalProcedureCompileRequest,
    entry: Arc<NativeCompilationEntry>,
    config: tcl_lexer::LexerConfig,
}

impl PartialEq for OriginalNativeCompilerFormals {
    fn eq(&self, other: &Self) -> bool {
        self.request.declaration() == other.request.declaration()
            && self.request.original_source() == other.request.original_source()
            && self.request.target().source == other.request.target().source
            && self.request.target().namespace == other.request.target().namespace
            && self.request.binding_context() == other.request.binding_context()
            && self.request.target().parameters == other.request.target().parameters
            && self.entry == other.entry
            && self.config == other.config
    }
}
impl Eq for OriginalNativeCompilerFormals {}
impl Hash for OriginalNativeCompilerFormals {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.request.declaration().hash(state);
        self.request.original_source().hash(state);
        self.request.target().source.hash(state);
        self.request.target().namespace.hash(state);
        self.request.binding_context().hash(state);
        self.request.target().parameters.hash(state);
        self.entry.hash(state);
        self.config.hash(state);
    }
}

impl OriginalNativeCompilerFormals {
    pub(crate) fn capture(
        request: &OriginalProcedureCompileRequest,
        entry: Arc<NativeCompilationEntry>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<Self>> {
        let options =
            crate::compile_service::BytecodeCompileService::native_entry_options(&entry, None);
        let protocol = entry.compiled_variable_protocol?;
        if !request.matches_entry(&entry)
            || request.target().source != request.original_source()
            || entry.frame != tcl_runtime_api::native_compilation::NativeCompilationFrame::Procedure
            || options.invocation_dialect?.parameter_grammar()
                != Some(tcl_dialect::ParameterGrammar::Tcl)
            || options.native_lexer_config(config) != config
            || !protocol.supports_environment(NativeCompiledVariableEnvironment::DeclareProcedure)
        {
            return None;
        }
        let parameters = request.ordinary_formals()?;
        if parameters.len() > 4096
            || parameters.iter().enumerate().any(|(index, name)| {
                parameters[..index].iter().any(|previous| {
                    protocol.compiled_local_names_equal(previous.as_bytes(), name.as_bytes())
                })
            })
        {
            return None;
        }
        Some(Arc::new(Self {
            request: request.clone(),
            entry,
            config,
        }))
    }

    pub(crate) fn bind_scope(
        self: &Arc<Self>,
        state: &super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
        incoming: &[String],
    ) -> Option<Arc<OriginalNativeCompilerFormalScope>> {
        let origin = state.current_source_origin.as_ref()?;
        let namespace = super::SourceNamespaceKey::from_native_entry(&self.entry).ok()?;
        if !self.is_live()
            || context.compilation().mode
                != tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject
            || context.compilation().frame
                != tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
            || incoming.len() != self.request.target().parameters.len()
            || !incoming
                .iter()
                .zip(self.request.target().parameters)
                .all(|(name, original)| name.as_bytes() == original.as_bytes())
            || state.baseline.native_entry.as_deref() != Some(self.entry.as_ref())
            || origin.source_image() != self.request.target().source
            || context.config != self.config
            || context.namespace_identity() != namespace
            || !matches!(
                context.frame.layout(),
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
            )
            || state.variable_frame != *context.frame
        {
            return None;
        }
        Some(Arc::new(OriginalNativeCompilerFormalScope {
            owner: Arc::clone(self),
            origin: Arc::clone(origin),
            frame: context.frame.clone(),
            namespace,
        }))
    }

    pub(crate) fn is_live(&self) -> bool {
        self.request.matches_entry(&self.entry)
    }
}

/// The exact original root and nominal compiler frame that consumed a request.
/// Equal bytes in a foreign or nested source activation cannot borrow this scope.
#[derive(Debug, Clone)]
pub(crate) struct OriginalNativeCompilerFormalScope {
    owner: Arc<OriginalNativeCompilerFormals>,
    origin: Arc<super::SourceOriginId>,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace: super::SourceNamespaceKey,
}
impl PartialEq for OriginalNativeCompilerFormalScope {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && Arc::ptr_eq(&self.origin, &other.origin)
            && self.frame == other.frame
            && self.namespace == other.namespace
    }
}
impl Eq for OriginalNativeCompilerFormalScope {}
impl Hash for OriginalNativeCompilerFormalScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        Arc::as_ptr(&self.origin).hash(state);
        self.frame.hash(state);
        self.namespace.hash(state);
    }
}
impl OriginalNativeCompilerFormalScope {
    pub(crate) fn parameters_for_entry(
        &self,
        chunk: &super::CommandAllocationSite,
        length: usize,
        state: &super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
        protocol: NativeCompiledVariableProtocol,
    ) -> Option<&[NameBytes]> {
        if !self.owner.is_live()
            || state.baseline.native_entry.as_deref() != Some(self.owner.entry.as_ref())
            || self.owner.entry.compiled_variable_protocol != Some(protocol)
            || chunk.offset != 0
            || length != self.owner.request.target().source.len()
            || !Arc::ptr_eq(&chunk.source, &self.origin)
            || context.config != self.owner.config
            || context.frame != &self.frame
            || context.namespace_identity() != self.namespace
        {
            return None;
        }
        self.owner.request.ordinary_formals()
    }

    pub(crate) fn owns_source(
        &self,
        site: &super::CommandAllocationSite,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.is_live()
            && Arc::ptr_eq(&site.source, &self.origin)
            && u32::try_from(self.owner.request.target().source.len())
                .is_ok_and(|end| site.offset < end)
            && frame == &self.frame
            && config == self.owner.config
    }

    pub(crate) fn is_live(&self) -> bool {
        self.owner.is_live()
    }
}
