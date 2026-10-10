// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Variable compiler purposes retain actual source and preparation owners.

use super::{
    ModuleCommandBindings, SourceExecutionContext,
    original_name_value::OriginalSourceVariableCompilation,
};
use crate::{
    place::Place,
    signature_scan::{scope::SignatureSourceNameInput, variable_name::SignatureSourceVariableRoot},
    var_resolve::ResolveContext,
};
use tcl_registry::{
    CommandRegistry, TraceOperation,
    native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        NativeCompilationSelection,
    },
};
use tcl_syntax::{
    naming::{NativeCompiledVariableEnvironment, NativeCompiledVariableProtocol},
    native_variable_words::NativeVariableWordOperand,
};

/// A genuine original Logical substitution can consult only the existing
/// symbolic source storage. It issues no Native root, compiler or read receipt.
pub(super) fn logical_original_substitution_access(
    original: Option<&(&str, crate::ir::SourceSite)>,
    arena: Option<&tcl_lexer::ExecutablePartArena>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Place> {
    // naming.source.logical-original-variable-substitution
    // docs/design/analysis/name-resolution-proofs/logical-original-variable-substitution.md
    let input = super::logical_definition::logical_entry(state, context)?;
    let (spelling, site) = original?;
    let arena = arena?;
    let origin = state.current_source_origin.as_ref()?;
    if site.provenance != crate::ir::Provenance::Source
        || &state.variable_frame != context.frame
        || state.source_variables.namespace != context.namespace
        || !matches!(origin.kind(), super::SourceOriginKind::Authored(_))
        || origin.source_image().channel() != tcl_lexer::SourceChannel::Document
        || arena.image() != origin.source_image()
        || arena.config() != context.config
        || input.lexer_config() != context.config
        || arena.image().bytes().get(site.span.as_range()) != Some(spelling.as_bytes())
    {
        return None;
    }
    Some(crate::var_resolve::resolve_substitution_access(
        spelling,
        &state.source_variables,
        context.registry,
        TraceOperation::Read,
    ))
}

/// Source compilation selection, separate from any command's inline opcode.
/// The original lexical component cannot manufacture this receipt itself.
#[derive(Clone)]
pub(crate) struct OriginalSourceLexicalCompilation {
    image: tcl_lexer::SourceImage,
    origin: std::sync::Arc<super::SourceOriginId>,
    execution_frame: crate::var_resolve::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
    compilation: NativeCompilationContext,
    protocol: Option<NativeCompiledVariableProtocol>,
    locals: Option<super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>,
    activation: Option<String>,
    frame: crate::var_resolve::VariableFrameKind,
    policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
}

impl OriginalSourceLexicalCompilation {
    pub(super) fn at_source(
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        let options = super::SourceAnalysisOptions {
            native_entry: state.baseline.native_entry.as_deref(),
            invocation_dialect: state.baseline.dialect,
            compiled_variable_provider: state.baseline.compiled_variable_provider,
            ..Default::default()
        };
        let origin = state.current_source_origin.as_ref()?;
        let protocol = options.compiled_variable_protocol();
        let locals = context
            .compilation_snapshot
            .and_then(|snapshot| snapshot.source_locals.as_deref())
            .filter(|inventory| {
                protocol.is_some_and(|protocol| {
                    inventory.owns(
                        &super::CommandAllocationSite {
                            source: std::sync::Arc::clone(origin),
                            offset: context.invocation_offset,
                        },
                        context.config,
                        context.frame,
                        context.compilation,
                        protocol,
                    )
                })
            })
            .cloned();
        Some(Self {
            image: origin.source_image().clone(),
            origin: std::sync::Arc::clone(origin),
            execution_frame: context.frame.clone(),
            config: context.config,
            compilation: context.compilation,
            protocol,
            locals,
            activation: state.source_variables.activation.clone(),
            frame: state.source_variables.frame_kind,
            policy: state.source_variables.execution_name_policy,
        })
    }

    pub(crate) fn resolve_root(
        &self,
        root: &SignatureSourceVariableRoot,
        element: Option<&SignatureSourceNameInput>,
        context: &ResolveContext,
        registry: &CommandRegistry,
        operation: TraceOperation,
    ) -> Place {
        let result = self.resolve_root_inner(root, element, context, registry, operation);
        if cfg!(debug_assertions)
            && std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some()
        {
            let input = SignatureSourceNameInput::OriginalVariableRoot(root.clone());
            let lookup =
                self.protocol
                    .zip(environment(self.compilation))
                    .map(|(protocol, environment)| {
                        protocol.substitution_lookup(
                            root.bytes(),
                            !root.is_separate_array_root(),
                            environment,
                        )
                    });
            eprintln!(
                "original lexical read site={} frame_matches={} image_matches={} config_matches={} policy_matches={} input_current={} index_matches={} mode={:?} compilation_frame={:?} protocol={} locals={} lookup={lookup:?} result={:?} dynamic_bindings={} dynamic_traces={}",
                root.part_span().start(),
                context.activation == self.activation && context.frame_kind == self.frame,
                root.source_image() == &self.image,
                root.lexer_config() == self.config,
                context.execution_name_policy == self.policy,
                input.is_current(context),
                root.is_separate_array_root() == element.is_some(),
                self.compilation.mode,
                self.compilation.frame,
                self.protocol.is_some(),
                self.locals.is_some(),
                result.kind,
                context.dynamic_bindings,
                context.dynamic_traces,
            );
        }
        result
    }

    fn resolve_root_inner(
        &self,
        root: &SignatureSourceVariableRoot,
        element: Option<&SignatureSourceNameInput>,
        context: &ResolveContext,
        registry: &CommandRegistry,
        operation: TraceOperation,
    ) -> Place {
        if context.activation != self.activation
            || context.frame_kind != self.frame
            || context.execution_name_policy != self.policy
            || root.source_image() != &self.image
            || root.lexer_config() != self.config
            || root.is_separate_array_root() != element.is_some()
            || element.is_some_and(|element| {
                element.policy() != root.policy() || !element.is_current(context)
            })
        {
            return crate::place::unknown_top();
        }
        let input = SignatureSourceNameInput::OriginalVariableRoot(root.clone());
        if !input.is_current(context) {
            return crate::place::unknown_top();
        }
        if self.compilation.mode == NativeCompilationMode::Direct {
            return match element {
                Some(element) => crate::var_resolve::resolve_original_element_inputs(
                    &input, element, context, registry, operation,
                ),
                None => crate::var_resolve::resolve_original_name_input(
                    &input, context, registry, false, operation,
                ),
            };
        }
        if self.compilation.mode != NativeCompilationMode::BytecodeObject {
            return crate::place::unknown_top();
        }
        let Some(protocol) = self.protocol else {
            return crate::place::unknown_top();
        };
        let Some(environment) = environment(self.compilation) else {
            return crate::place::unknown_top();
        };
        let lookup =
            protocol.substitution_lookup(root.bytes(), !root.is_separate_array_root(), environment);
        if lookup == tcl_syntax::naming::NativeCompiledVariableLookup::DynamicName {
            return crate::var_resolve::resolve_original_compiled_variable(
                root.bytes(),
                element.map(SignatureSourceNameInput::bytes),
                lookup,
                protocol,
                context,
                registry,
                operation,
            );
        }
        let Some(locals) = self.locals.as_ref() else {
            // The complete formal prefix and byte-unique comparison remain
            // independently sound when a broader compiler attempt is unknown.
            return crate::var_resolve::resolve_original_compiled_variable(
                root.bytes(),
                element.map(SignatureSourceNameInput::bytes),
                lookup,
                protocol,
                context,
                registry,
                operation,
            );
        };
        if !locals.owns(
            &super::CommandAllocationSite {
                source: std::sync::Arc::clone(&self.origin),
                offset: root.part_span().start(),
            },
            self.config,
            &self.execution_frame,
            self.compilation,
            protocol,
        ) {
            return crate::place::unknown_top();
        }
        let Some(primary) = OriginalCompiledVariablePrimary::from_inventory(
            locals,
            root.bytes(),
            self.compilation,
            protocol,
        ) else {
            return crate::place::unknown_top();
        };
        crate::var_resolve::resolve_original_compiled_primary(
            &primary,
            element.map(SignatureSourceNameInput::bytes),
            context,
            registry,
            operation,
        )
    }
}
fn environment(compilation: NativeCompilationContext) -> Option<NativeCompiledVariableEnvironment> {
    (compilation.mode == NativeCompilationMode::BytecodeObject).then_some(match compilation.frame {
        NativeCompilationFrame::ProcedureCode => {
            NativeCompiledVariableEnvironment::DeclareProcedure
        }
        NativeCompilationFrame::ScriptCode => NativeCompiledVariableEnvironment::None,
        NativeCompilationFrame::Unknown => return None,
    })
}

/// First counted primary of a complete selected source compilation. Only this
/// module can issue it from the authenticated ordered inventory; it is neither
/// a physical CPP slot nor a successful variable operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalCompiledVariablePrimary {
    name: tcl_core_types::NameBytes,
    compiler: NativeCompiledVariableProtocol,
}

impl OriginalCompiledVariablePrimary {
    fn from_inventory(
        inventory: &super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory,
        requested: &[u8],
        compilation: NativeCompilationContext,
        compiler: NativeCompiledVariableProtocol,
    ) -> Option<Self> {
        Some(Self {
            name: inventory.primary_for(requested, compilation)?.clone(),
            compiler,
        })
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }
    pub(crate) const fn compiler(&self) -> NativeCompiledVariableProtocol {
        self.compiler
    }
}

/// One variable operand selected by the authentic retained instruction plan.
/// This owns its complete original word and independent consumer purpose;
/// neither its spelling nor a materialised value can issue this receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalCompiledVariableOperand {
    word: tcl_lexer::NativeWord,
    purpose: OriginalVariableOperandPurpose,
    activation: Option<String>,
    frame: crate::var_resolve::VariableFrameKind,
    policy: tcl_syntax::naming::ExecutionNamePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum OriginalVariableOperandPurpose {
    Unavailable,
    Runtime(Box<SignatureSourceNameInput>),
    Compiled {
        name: Vec<u8>,
        index: Option<Vec<u8>>,
        lookup: tcl_syntax::naming::NativeCompiledVariableLookup,
        protocol: NativeCompiledVariableProtocol,
        primary: Option<OriginalCompiledVariablePrimary>,
    },
}

impl OriginalCompiledVariableOperand {
    pub(crate) fn resolve(
        &self,
        context: &ResolveContext,
        registry: &CommandRegistry,
        whole_array: bool,
        operation: TraceOperation,
    ) -> Place {
        if context.activation != self.activation
            || context.frame_kind != self.frame
            || context.execution_name_policy != Some(self.policy)
            || self.policy.native_recipe().is_none_or(|policy| {
                self.word.config().escapes != policy.string_protocol().escape_syntax()
            })
        {
            return crate::place::unknown_top();
        }
        let mut result = match &self.purpose {
            OriginalVariableOperandPurpose::Unavailable => return crate::place::unknown_top(),
            OriginalVariableOperandPurpose::Runtime(input) => {
                return crate::var_resolve::resolve_original_name_input(
                    input,
                    context,
                    registry,
                    whole_array,
                    operation,
                );
            }
            OriginalVariableOperandPurpose::Compiled {
                name,
                index,
                lookup,
                protocol,
                primary,
            } => {
                if whole_array && index.is_some() {
                    return crate::place::unknown_top();
                }
                match primary {
                    Some(primary) => crate::var_resolve::resolve_original_compiled_primary(
                        primary,
                        index.as_deref(),
                        context,
                        registry,
                        operation,
                    ),
                    None => crate::var_resolve::resolve_original_compiled_variable(
                        name,
                        index.as_deref(),
                        *lookup,
                        *protocol,
                        context,
                        registry,
                        operation,
                    ),
                }
            }
        };
        if whole_array && result.kind != crate::place::PlaceKind::Unknown {
            result.kind = crate::place::PlaceKind::ArrayWhole;
            result = crate::var_resolve::project_access(result, context, operation);
        }
        result
    }
}

/// Select the exact written operand from the retained inline compiler recipe.
/// Missing/static-dynamic-index provenance declines; source roles never issue it.
pub(crate) fn original_compiler_variable_operand(
    compilation: &OriginalSourceVariableCompilation<'_>,
    written: usize,
    context: &ResolveContext,
) -> Option<OriginalCompiledVariableOperand> {
    use tcl_registry::native_instruction_plan::NativeInstructionPlan;
    let preparation = compilation.structured()?;
    let NativeCompilationSelection::Inline { guard, .. } = compilation.selection() else {
        return None;
    };
    if preparation.compilation_site() != compilation.site()
        || preparation.dependency().guard != *guard
    {
        return None;
    }
    let (target_word, target) = match preparation.recipe() {
        NativeInstructionPlan::Load {
            target_word,
            target,
        }
        | NativeInstructionPlan::Store {
            target_word,
            target,
            ..
        }
        | NativeInstructionPlan::Increment {
            target_word,
            target,
            ..
        }
        | NativeInstructionPlan::Append {
            target_word,
            target,
            ..
        } => (*target_word, target),
        _ => return None,
    };
    if target_word != written {
        return None;
    }
    original_compiler_operand_from_target(
        compilation,
        written,
        OriginalVariableTarget::Word(target),
        context,
    )
}

/// One sequential list-assignment target selected by the retained native plan.
/// The target index addresses the actual plan, including parser-expanded
/// members; it is not inferred from the effective argv or variable spelling.
pub(crate) fn original_compiler_list_assignment_operand(
    compilation: &OriginalSourceVariableCompilation<'_>,
    target_index: usize,
    context: &ResolveContext,
) -> Option<OriginalCompiledVariableOperand> {
    use tcl_registry::{
        native_compiler_word_projection::NativeCompilerWordOperand,
        native_instruction_plan::NativeInstructionPlan,
        native_list_operations_compilation::{
            NativeListOperationInstruction, NativeListVariableOperand,
        },
    };
    let preparation = compilation.structured()?;
    let NativeCompilationSelection::Inline { guard, .. } = compilation.selection() else {
        return None;
    };
    if preparation.compilation_site() != compilation.site()
        || preparation.dependency().guard != *guard
    {
        return None;
    }
    let NativeInstructionPlan::ListOperations(NativeListOperationInstruction::Assign {
        targets,
        ..
    }) = preparation.recipe()
    else {
        return None;
    };
    match targets.get(target_index)? {
        NativeListVariableOperand::Original {
            operand: NativeCompilerWordOperand::Original(written),
            variable,
        } => original_compiler_operand_from_target(
            compilation,
            *written,
            OriginalVariableTarget::Word(variable),
            context,
        ),
        NativeListVariableOperand::ExpandedLiteral {
            operand: NativeCompilerWordOperand::LiteralExpansion { original_word, .. },
            name,
            index,
        } => original_compiler_operand_from_target(
            compilation,
            *original_word,
            OriginalVariableTarget::ExpandedLiteral(name, index.as_deref()),
            context,
        ),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum OriginalVariableTarget<'a> {
    Word(&'a NativeVariableWordOperand),
    ExpandedLiteral(&'a [u8], Option<&'a [u8]>),
}

fn original_compiler_operand_from_target(
    compilation: &OriginalSourceVariableCompilation<'_>,
    written: usize,
    target: OriginalVariableTarget<'_>,
    context: &ResolveContext,
) -> Option<OriginalCompiledVariableOperand> {
    let words = crate::registry_invocation::original_native_compiler_words(
        compilation.site().source.source_image(),
        compilation.original_words(),
        compilation.site().offset,
        compilation.config(),
    )?;
    let word = words.get(written)?.clone();
    let policy = context.execution_name_policy?;
    let purpose = (|| {
        let name_policy = policy.native_recipe()?;
        Some(if matches!(target, OriginalVariableTarget::Word(NativeVariableWordOperand::DynamicWord)) {
            OriginalVariableOperandPurpose::Runtime(Box::new(compilation.evaluated_original_word(written)?))
        } else {
            let protocol = compilation.protocol()?;
            let environment = environment(compilation.compilation())?;
            let (name, index) = match target {
                OriginalVariableTarget::Word(NativeVariableWordOperand::Literal { name, index, .. }) => {
                    (name.clone(), index.clone())
                }
                OriginalVariableTarget::ExpandedLiteral(name, index) => {
                    (name.to_vec(), index.map(<[u8]>::to_vec))
                }
                OriginalVariableTarget::Word(NativeVariableWordOperand::CompoundArray { name, index, .. }) => {
                    let value = compilation
                        .evaluated_original_variable_index(written, &word, index, context)
                        .or_else(|| {
                            crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_index(
                                &word,
                                tcl_syntax::word_rules::WordValueRules::from_config(&compilation.config()),
                                name_policy,
                            ).map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue)
                        })?;
                    (name.clone(), Some(value.bytes().to_vec()))
                }
                OriginalVariableTarget::Word(NativeVariableWordOperand::DynamicWord) => unreachable!(),
            };
            let lookup = protocol.command_lookup(&name, environment);
            let primary = if lookup == tcl_syntax::naming::NativeCompiledVariableLookup::DynamicName {
                None
            } else {
                match compilation.local_inventory() {
                    Some(locals) => Some(OriginalCompiledVariablePrimary::from_inventory(locals,
                        &name, compilation.compilation(), protocol)?),
                    None => None,
                }
            };
            OriginalVariableOperandPurpose::Compiled { lookup, name, index, protocol, primary }
        })
    })()
    .unwrap_or(OriginalVariableOperandPurpose::Unavailable);
    Some(OriginalCompiledVariableOperand {
        word,
        purpose,
        activation: context.activation.clone(),
        frame: context.frame_kind,
        policy,
    })
}

pub(crate) fn resolve_original_compiler_variable_operand(
    compilation: &OriginalSourceVariableCompilation<'_>,
    written: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: TraceOperation,
) -> Place {
    original_compiler_variable_operand(compilation, written, context)
        .map_or_else(crate::place::unknown_top, |operand| {
            operand.resolve(context, registry, false, operation)
        })
}

/// Direct local declaration selected by one authentic namespace compilation
/// recipe. A byte string or original word cannot construct this receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalCompiledNamespaceLocal {
    primary: tcl_core_types::NameBytes,
    activation: String,
    frame: crate::var_resolve::VariableFrameKind,
    policy: tcl_syntax::naming::ExecutionNamePolicy,
}

impl OriginalCompiledNamespaceLocal {
    pub(crate) fn binding_slot(&self, context: &ResolveContext) -> Option<Place> {
        (context.activation.as_ref() == Some(&self.activation)
            && context.frame_kind == self.frame
            && context.execution_name_policy == Some(self.policy))
        .then_some(())?;
        let slot = crate::var_resolve::original_compiled_local_destination(&self.primary, context);
        (slot.kind != crate::place::PlaceKind::Unknown).then_some(slot)
    }
}

/// A namespace opcode's local binding name comes from its original recipe
/// ordinal. It is declaration data only; alias installation and observers
/// require their independently selected reached operation.
pub(crate) fn original_compiler_namespace_local_name(
    compilation: &OriginalSourceVariableCompilation<'_>,
    written: usize,
    context: &ResolveContext,
) -> Option<OriginalCompiledNamespaceLocal> {
    use tcl_registry::{
        native_compiler_word_projection::NativeCompilerWordOperand,
        native_namespace_binding_compilation::NativeNamespaceBindingOutcome,
    };
    let preparation = compilation.namespace()?;
    let NativeCompilationSelection::Inline { guard, .. } = compilation.selection() else {
        return None;
    };
    if preparation.compilation_site != *compilation.site()
        || preparation.dependency().guard != *guard
        || preparation.recipe().outcome != NativeNamespaceBindingOutcome::Inline
        || environment(compilation.compilation())?
            != NativeCompiledVariableEnvironment::DeclareProcedure
    {
        return None;
    }
    let mut bindings = preparation.recipe().bindings.iter().filter(|binding| {
        matches!(binding.name, NativeCompilerWordOperand::Original(ordinal) if ordinal == written)
    });
    let binding = bindings.next()?;
    if bindings.next().is_some() {
        return None;
    }
    Some(OriginalCompiledNamespaceLocal {
        primary: match compilation.local_inventory() {
            Some(locals) => locals
                .primary_for(&binding.local, compilation.compilation())?
                .clone(),
            None => crate::var_resolve::original_compiled_variable_primary(
                &binding.local,
                tcl_syntax::naming::NativeCompiledVariableLookup::CreateLocal,
                compilation.protocol()?,
                context,
            )?,
        },
        activation: context.activation.clone()?,
        frame: context.frame_kind,
        policy: context.execution_name_policy?,
    })
}

#[cfg(test)]
mod ordered_primary_tests {
    use super::*;

    fn ordered_primary_bindings(
        source: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> super::super::SourceCommandBindings {
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        super::super::SourceCommandBindings::analyse_image_in_frame_with_options(
            &tcl_lexer::SourceImage::native(source),
            &crate::var_resolve::VariableExecutionFrame::Global,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    // Implementation contract: naming.variable.ordered-compiler-primary-cell
    // docs/design/analysis/name-resolution-proofs/ordered-compiler-primary-cell.md
    fn ordered_compiler_primary_is_consumed_before_runtime_and_lifetime_projection() {
        let source = b"proc p {formal} {set k\0a 1; set k\0b 2}; p VALUE";
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let bindings = ordered_primary_bindings(source, registry);
        let visit = bindings
            .compiler_invocations
            .values()
            .flatten()
            .find(|visit| visit.source_locals.is_some())
            .expect("genuine complete procedure compilation");
        let inventory = visit.source_locals.as_deref().unwrap();
        let compilation = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        let compiler = NativeCompiledVariableProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let primary = OriginalCompiledVariablePrimary::from_inventory(
            inventory,
            b"k\0b",
            compilation,
            compiler,
        )
        .unwrap();
        assert_eq!(primary.bytes(), b"k\0a");
        assert!(
            OriginalCompiledVariablePrimary::from_inventory(
                inventory,
                b"k\0bb",
                compilation,
                compiler
            )
            .is_none()
        );
        assert!(
            OriginalCompiledVariablePrimary::from_inventory(
                inventory,
                b"k\0b",
                NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..compilation
                },
                compiler
            )
            .is_none()
        );
        let context = &visit.table.state.source_variables;
        let selected_cell = crate::var_resolve::resolve_original_compiled_primary(
            &primary,
            None,
            context,
            registry,
            TraceOperation::Read,
        );
        assert_eq!(
            selected_cell
                .cell
                .as_ref()
                .expect("conditional local cell")
                .name
                .as_bytes(),
            b"k\0a"
        );
        let runtime = crate::var_resolve::resolve_original_compiled_variable(
            b"k\0b",
            None,
            tcl_syntax::naming::NativeCompiledVariableLookup::DynamicName,
            compiler,
            context,
            registry,
            TraceOperation::Read,
        );
        assert_eq!(
            runtime
                .cell
                .as_ref()
                .expect("independent counted runtime name")
                .name
                .as_bytes(),
            b"k\0b"
        );
        let mut unknown = context.as_ref().clone();
        unknown
            .unknown_bindings
            .insert(crate::var_resolve::cell_key(&selected_cell));
        assert_eq!(
            crate::var_resolve::resolve_original_compiled_primary(
                &primary,
                None,
                &unknown,
                registry,
                TraceOperation::Read
            )
            .kind,
            crate::place::PlaceKind::Unknown
        );
    }
}

#[cfg(test)]
mod logical_substitution_tests {
    use super::*;
    use std::sync::Arc;

    fn logical_input() -> crate::analyser::ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        )
    }

    fn inspect_original_read<T>(
        source: &str,
        input: &crate::analyser::ResolvedAnalysisInput,
        inspect: impl FnOnce(
            &mut ModuleCommandBindings,
            &(&str, crate::ir::SourceSite),
            &tcl_lexer::ExecutablePartArena,
            SourceExecutionContext<'_>,
        ) -> T,
    ) -> T {
        let context = input.context_registry();
        let registry = context.commands();
        let config = input.lexer_config();
        let options = super::super::SourceAnalysisOptions {
            logical_source_input: Some(input),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                input.unit_profile(),
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let mut state =
            ModuleCommandBindings::initial_with_options(registry, options, Some(config));
        state.current_source_origin = Some(Arc::new(super::super::SourceOriginId::authored_image(
            tcl_lexer::SourceImage::document(source),
        )));
        // Chosen symbolic incoming storage; this is no executed Tcl store.
        Arc::make_mut(&mut state.source_variables)
            .bind_literal_incoming("scalar", "TARGET", registry);
        Arc::make_mut(&mut state.source_variables).bind_literal_incoming(
            "array(key)",
            "ELEMENT",
            registry,
        );
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .next()
            .unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        let word = tokens.words().get(1).unwrap();
        let original = super::super::original_variable_source(source, 0, word.source(), config)
            .expect("authentic original variable component");
        let arena = super::super::original_variable_arena(Some(&original), &state, config)
            .expect("original whole image and arena");
        let namespace = super::super::SourceNamespaceKey::authored("::");
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let selected = super::super::root_source_execution_context(
            &frame, "::", &namespace, config, registry, options,
        );
        inspect(&mut state, &original, &arena, selected)
    }

    #[test]
    fn logical_original_substitution_reads_only_the_existing_symbolic_storage() {
        // naming.source.logical-original-variable-substitution
        // docs/design/analysis/name-resolution-proofs/logical-original-variable-substitution.md
        // Admission into chosen symbolic inputs, not a Native process/value proof.
        let input = logical_input();
        for (source, expected) in [
            ("consume $scalar", "TARGET"),
            ("consume ${scalar}", "TARGET"),
            ("consume $array(key)", "ELEMENT"),
        ] {
            inspect_original_read(source, &input, |state, original, arena, context| {
                let place = logical_original_substitution_access(
                    Some(original),
                    Some(arena),
                    state,
                    context,
                )
                .expect("genuine Logical original read");
                assert_eq!(
                    state
                        .source_variables
                        .literal_contents_at(&place, context.registry),
                    Some(expected)
                );
                assert!(
                    state
                        .source_variables
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        .is_none()
                );
            });
        }
        for source in ["consume $missing", "consume $array($scalar)"] {
            inspect_original_read(source, &input, |state, original, arena, context| {
                let place = logical_original_substitution_access(
                    Some(original),
                    Some(arena),
                    state,
                    context,
                )
                .unwrap();
                assert!(
                    state
                        .source_variables
                        .literal_contents_at(&place, context.registry)
                        .is_none()
                );
            });
        }
    }

    #[test]
    fn logical_original_substitution_keeps_observers_and_dynamic_storage_open() {
        // naming.source.logical-original-variable-substitution
        // docs/design/analysis/name-resolution-proofs/logical-original-variable-substitution.md
        let input = logical_input();
        for traced in [false, true] {
            inspect_original_read(
                "consume $scalar",
                &input,
                |state, original, arena, context| {
                    let variables = Arc::make_mut(&mut state.source_variables);
                    if traced {
                        variables.dynamic_traces = true;
                    } else {
                        variables.dynamic_bindings = true;
                    }
                    let place = logical_original_substitution_access(
                        Some(original),
                        Some(arena),
                        state,
                        context,
                    )
                    .unwrap();
                    assert!(
                        state
                            .source_variables
                            .literal_contents_at(&place, context.registry)
                            .is_none()
                    );
                },
            );
        }
    }

    #[test]
    fn logical_original_substitution_refuses_missing_native_and_stale_ownership() {
        // naming.source.logical-original-variable-substitution
        // docs/design/analysis/name-resolution-proofs/logical-original-variable-substitution.md
        let input = logical_input();
        inspect_original_read(
            "consume $scalar",
            &input,
            |state, original, arena, context| {
                assert!(
                    logical_original_substitution_access(None, Some(arena), state, context)
                        .is_none()
                );
                assert!(
                    logical_original_substitution_access(Some(original), None, state, context)
                        .is_none()
                );
                let mut changed = original.clone();
                changed.1.provenance = crate::ir::Provenance::Opaque;
                assert!(
                    logical_original_substitution_access(
                        Some(&changed),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
                let changed = ("$other", original.1.clone());
                assert!(
                    logical_original_substitution_access(
                        Some(&changed),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
                let mut stale = context;
                stale.config.strict_quoting ^= true;
                assert!(
                    logical_original_substitution_access(Some(original), Some(arena), state, stale)
                        .is_none()
                );
                let original_origin = state.current_source_origin.clone();
                state.current_source_origin =
                    Some(Arc::new(super::super::SourceOriginId::authored_image(
                        tcl_lexer::SourceImage::document("consume $other"),
                    )));
                assert!(
                    logical_original_substitution_access(
                        Some(original),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
                state.current_source_origin = Some(Arc::new(super::super::SourceOriginId::loaded(
                    Arc::from("captured.tcl"),
                    Arc::from("captured-source"),
                    &Arc::from("consume $scalar"),
                )));
                assert!(
                    logical_original_substitution_access(
                        Some(original),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
                state.current_source_origin = original_origin;
                let original_frame = state.variable_frame.clone();
                state.variable_frame = crate::var_resolve::VariableExecutionFrame::Procedure {
                    namespace: "::".to_owned(),
                    identity: "other-symbolic-entry".to_owned(),
                };
                assert!(
                    logical_original_substitution_access(
                        Some(original),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
                state.variable_frame = original_frame;
                Arc::make_mut(&mut state.baseline).logical_source_input = None;
                assert!(
                    logical_original_substitution_access(
                        Some(original),
                        Some(arena),
                        state,
                        context
                    )
                    .is_none()
                );
            },
        );
        for engine in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
            "f5-tmsh",
        ] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let native = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            inspect_original_read(
                "consume $scalar",
                &native,
                |state, original, arena, context| {
                    assert!(
                        logical_original_substitution_access(
                            Some(original),
                            Some(arena),
                            state,
                            context
                        )
                        .is_none(),
                        "{engine}"
                    );
                },
            );
        }
    }
}
