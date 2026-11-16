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

//! Original native compiler plans from immutable byte registrations.

use tcl_lexer::word_parts::{ExecutablePart, NativeWord};
use tcl_lexer::{SourceImage, Span, native_script_words_in};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationGrammar, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationSpec, NativeCompilationWordShape,
};
use tcl_registry::native_compiler_words::{NativeCompilerWords, NativeCompilerWordsUnavailable};
use tcl_registry::native_instruction_plan::NativeInstructionPlan;
use tcl_registry::{CommandRegistry, InvocationDialect};
use tcl_runtime_api::NativeCompilationEntry;
use tcl_runtime_api::native_compilation::{NativeCompilationBinding, NativeCompilerHookPresence};

/// Original generic dispatch or an independently registered opcode recipe.
/// A registered plan still requires an emitter implementing its operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub enum NativeByteCommandPlan {
    /// Native compilation emits ordinary dispatch without choosing its handler.
    Generic,
    /// The original compiler selected a retained, supported operation.
    Registered(Box<NativeByteRegisteredCommand>),
    /// Actual configured no-hook worker and original ensemble usage rewrite.
    Named(Box<NativeByteNamedInvocation>),
}

/// Original named worker selection, without retaining its callable body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct NativeByteNamedInvocation {
    /// Exact parser operand layout and canonical member replacement.
    pub recipe: tcl_registry::native_instruction_plan::NativeNamedInvocationInstruction,
    /// Original public compiler/configuration and actual selected worker.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
}

/// Exact original registration and authored compiler recipe. This grants no
/// normal-handler, variable contents, purity or runtime object-class facts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct NativeByteRegisteredCommand {
    /// Independently authored descriptor of the actual installed compiler.
    pub spec: NativeCompilationSpec,
    /// Selected operation and original compiler validation boundary.
    pub selection: NativeCompilationSelection,
    /// Original native compiler frame and control-depth entry for this selection.
    pub context: NativeCompilationContext,
    /// First operand in the complete original word vector, including its head.
    pub operand_from: usize,
    /// Command token and implementation incarnation from this entry snapshot.
    pub binding: NativeCompilationBinding,
    /// Additional original compiler dependencies, separate from late dispatch.
    pub dependencies: Vec<NativeCompilationBinding>,
    recipe: OriginalRegisteredRecipe,
    /// Original configured public compiler and selected worker, when delegated.
    pub prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
}

/// The factory-selected recipe and its exact original operand/selection basis.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalRegisteredRecipe {
    words: Vec<NativeWord>,
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    context: NativeCompilationContext,
    operand_from: usize,
    instruction: NativeInstructionPlan,
}

impl NativeByteRegisteredCommand {
    /// Consume the original factory recipe without changing its compiler purpose.
    /// A different source vector or selection basis cannot reuse its preparation.
    #[doc(hidden)]
    #[must_use]
    pub fn has_generic_preparation(&self) -> bool {
        matches!(
            self.recipe.instruction,
            NativeInstructionPlan::GenericPreparation(_)
        )
    }

    #[doc(hidden)]
    #[must_use]
    pub fn original_instruction(&self, words: &[NativeWord]) -> Option<&NativeInstructionPlan> {
        let recipe = &self.recipe;
        (recipe.words == words
            && recipe.spec == self.spec
            && recipe.selection == self.selection
            && recipe.context == self.context
            && recipe.operand_from == self.operand_from)
            .then_some(&recipe.instruction)
    }
}

/// Unavailable original compilation, distinct from generic or guest rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub enum NativeByteCommandUnavailable {
    /// Physical engine or independent name/source issuer is missing or conflicts.
    Entry,
    /// The original command vector could not retain its lexical/value evidence.
    Words(NativeCompilerWordsUnavailable),
    /// The immutable snapshot cannot settle one exact lookup.
    Lookup(tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable),
    /// Actual hook registration is unknown or lacks its compiler descriptor.
    Registration,
    /// Mutable ensemble or additional original compiler dependencies are unproved.
    Dependencies,
    /// A descriptor or executable emitter for this selected recipe is unavailable.
    Recipe,
}

/// Resolve one original compiler head and select its authenticated recipe.
/// The physical point selects compiler grammar; `source_string_protocol` forms
/// word values independently. Returned registration prerequisites must be
/// retained by the emitter before any argument effect. This function checks no
/// runtime normal-handler semantics and performs no live callback or lookup.
/// Nested bracket compiler obligations remain the whole-script caller's duty.
///
/// # Errors
/// Returns missing original source/entry ownership, registration, dependencies
/// or implemented opcode capability. This is not a Tcl completion.
#[doc(hidden)]
pub fn native_byte_command_plan(
    words: &[NativeWord],
    entry: &NativeCompilationEntry,
    registry: &CommandRegistry,
    context: NativeCompilationContext,
) -> Result<NativeByteCommandPlan, NativeByteCommandUnavailable> {
    let dialect = compiler_dialect(entry)?;
    let protocol = entry
        .source_string_protocol
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    let captured = NativeCompilerWords::capture(words, protocol)
        .map_err(NativeByteCommandUnavailable::Words)?;
    let head = captured.shapes()[0];
    if head == NativeCompilationWordShape::Expanded {
        return if unprojected_expanded_head(&captured, dialect)? {
            Ok(NativeByteCommandPlan::Generic)
        } else {
            // Literal TEXT expansion can select a registered compiler after
            // parser projection. It cannot acquire generic handler timing.
            Err(NativeByteCommandUnavailable::Recipe)
        };
    }
    if context.mode == NativeCompilationMode::Direct
        || entry.inline_compilation_disabled
        || head.compiler_head(Some(dialect)) == Some(false)
    {
        return Ok(NativeByteCommandPlan::Generic);
    }
    if context.mode != NativeCompilationMode::BytecodeObject
        || head.compiler_head(Some(dialect)) != Some(true)
    {
        return Err(NativeByteCommandUnavailable::Entry);
    }
    let head = captured
        .literal(0)
        .ok_or(NativeByteCommandUnavailable::Registration)?;
    let binding = entry
        .lookup_command_bytes(entry.current_namespace, head)
        .map_err(NativeByteCommandUnavailable::Lookup)?;
    let Some(binding) = binding else {
        return Ok(NativeByteCommandPlan::Generic);
    };
    if binding.has_execution_trace || binding.compiler_hook == NativeCompilerHookPresence::Absent {
        return Ok(NativeByteCommandPlan::Generic);
    }
    if binding.compiler_hook != NativeCompilerHookPresence::Present {
        return Err(NativeByteCommandUnavailable::Registration);
    }
    // An empty variadic procedure's `TclCompileNoOp` only evaluates the
    // substituted operands and yields ""; invoking it does the same.
    if binding.compiler.is_none()
        && binding.procedure_header == Some(tcl_dialect::NativeProcedureHeaderCompilation::NoOp)
    {
        return Ok(NativeByteCommandPlan::Generic);
    }
    if binding
        .compiler
        .as_ref()
        .is_some_and(|compiler| compiler.ensemble.is_some())
    {
        return ensemble_command_plan(&captured, binding, entry, registry, dialect, context);
    }
    registered_command_plan(&captured, binding, entry, dialect, registry, context)
}

fn ensemble_command_plan(
    captured: &NativeCompilerWords<'_>,
    binding: &NativeCompilationBinding,
    entry: &NativeCompilationEntry,
    registry: &CommandRegistry,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeByteCommandPlan, NativeByteCommandUnavailable> {
    use tcl_registry::native_ensemble::NativeEnsembleWorkerSelection;
    use tcl_registry::native_selected_worker::OriginalSelectedWorkerInvocation;
    let configuration = binding
        .compiler
        .as_ref()
        .and_then(|compiler| compiler.ensemble.as_ref())
        .ok_or(NativeByteCommandUnavailable::Registration)?;
    let head = captured
        .literal(0)
        .ok_or(NativeByteCommandUnavailable::Registration)?;
    let selector = tcl_registry::native_ensemble::original_ensemble_selector(captured, dialect)
        .map_err(|_| NativeByteCommandUnavailable::Recipe)?;
    match tcl_registry::native_ensemble::select_worker_in_entry(
        entry,
        binding.token,
        configuration,
        selector.as_ref().and_then(|word| word.literal.as_deref()),
        selector.as_ref().map(|word| word.shape),
        Some(dialect),
    ) {
        NativeEnsembleWorkerSelection::Generic => Ok(NativeByteCommandPlan::Generic),
        NativeEnsembleWorkerSelection::Unknown => Err(NativeByteCommandUnavailable::Dependencies),
        NativeEnsembleWorkerSelection::Worker {
            member,
            binding: worker,
        } => {
            let prerequisite =
                tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
                    interpreter: entry.interpreter,
                    lookup_namespace_token: entry.current_namespace,
                    invocation_word: head.into(),
                    slot: binding.slot.clone(),
                    namespace_token: binding.namespace_token,
                    token: binding.token,
                    implementation_generation: binding.implementation_generation,
                    compiler: binding
                        .compiler
                        .clone()
                        .ok_or(NativeByteCommandUnavailable::Registration)?,
                    selected_worker: Some((*worker).clone()),
                    nested_compilers: Vec::new(),
                    guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
                };
            let replacements = [member.as_bytes().to_vec()];
            let selected =
                tcl_registry::native_selected_worker::compile_original_selected_worker_path(
                    registry,
                    &worker,
                    OriginalSelectedWorkerInvocation {
                        words: captured,
                        operand_from: 2,
                        replacements: &replacements,
                        dialect,
                        context,
                    },
                    prerequisite,
                    |ensemble, configuration, member, shape| {
                        tcl_registry::native_ensemble::select_worker_in_entry(
                            entry,
                            ensemble.token,
                            configuration,
                            member,
                            shape,
                            Some(dialect),
                        )
                    },
                );
            selected_worker_command_plan(captured, selected, context)
        }
    }
}

fn selected_worker_command_plan(
    captured: &NativeCompilerWords<'_>,
    selected: tcl_registry::native_selected_worker::OriginalSelectedWorkerPathCompilation,
    context: NativeCompilationContext,
) -> Result<NativeByteCommandPlan, NativeByteCommandUnavailable> {
    use tcl_registry::native_selected_worker::OriginalSelectedWorkerCompilation;
    let prerequisite = selected.prerequisite;
    match selected.compilation {
        OriginalSelectedWorkerCompilation::PublicGeneric => Ok(NativeByteCommandPlan::Generic),
        OriginalSelectedWorkerCompilation::Named(recipe) => Ok(NativeByteCommandPlan::Named(
            Box::new(NativeByteNamedInvocation {
                recipe,
                prerequisite,
            }),
        )),
        OriginalSelectedWorkerCompilation::Operation {
            spec,
            selection,
            plan,
        } => Ok(NativeByteCommandPlan::Registered(Box::new(
            NativeByteRegisteredCommand {
                spec,
                selection,
                context,
                operand_from: selected.operand_from,
                binding: selected.worker,
                dependencies: Vec::new(),
                recipe: OriginalRegisteredRecipe {
                    words: captured.original_words().to_vec(),
                    spec,
                    selection,
                    context,
                    operand_from: selected.operand_from,
                    instruction: *plan,
                },
                prerequisite: Some(prerequisite),
            },
        ))),
        OriginalSelectedWorkerCompilation::Unavailable => Err(NativeByteCommandUnavailable::Recipe),
    }
}

fn unprojected_expanded_head(
    captured: &NativeCompilerWords<'_>,
    dialect: InvocationDialect,
) -> Result<bool, NativeByteCommandUnavailable> {
    use tcl_registry::native_compiler_word_projection::{
        NativeCompilerWordOperand, project_native_compiler_words,
    };
    let version = dialect
        .tcl_version
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    if version < tcl_dialect::TclVersion::V8_5 {
        return Ok(false);
    }
    let projected = project_native_compiler_words(captured, version)
        .map_err(|_| NativeByteCommandUnavailable::Recipe)?;
    Ok(projected.first().is_some_and(|head| {
        head.shape == NativeCompilationWordShape::Expanded
            && head.operand == NativeCompilerWordOperand::Original(0)
    }))
}

fn compiler_dialect(
    entry: &NativeCompilationEntry,
) -> Result<InvocationDialect, NativeByteCommandUnavailable> {
    let point = entry
        .execution_point
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    let policy = entry
        .command_name_policy()
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    if tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(policy) {
        return Err(NativeByteCommandUnavailable::Entry);
    }
    point
        .tcl_version()
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    Ok(InvocationDialect::of_point(point))
}

fn registered_command_plan(
    captured: &NativeCompilerWords<'_>,
    binding: &NativeCompilationBinding,
    entry: &NativeCompilationEntry,
    dialect: InvocationDialect,
    registry: &CommandRegistry,
    context: NativeCompilationContext,
) -> Result<NativeByteCommandPlan, NativeByteCommandUnavailable> {
    let compiler = binding
        .compiler
        .as_ref()
        .ok_or(NativeByteCommandUnavailable::Registration)?;
    if compiler.ensemble.is_some() {
        return Err(NativeByteCommandUnavailable::Dependencies);
    }
    let spec = registry
        .native_compilation_for_original_registration(
            &compiler.registry_identity,
            captured,
            1,
            dialect,
        )
        .ok_or(NativeByteCommandUnavailable::Recipe)?;
    let dependencies = spec
        .implementation_prerequisites(dialect)
        .ok_or(NativeByteCommandUnavailable::Dependencies)?;
    // These initial ordinary recipes have no ensemble/private path. A nonempty
    // path needs original mapping attestation, not merely same-named slot rows.
    if !dependencies.is_empty() {
        return Err(NativeByteCommandUnavailable::Dependencies);
    }
    let selection = spec.select_native_words(captured, 1, Some(dialect), context);
    if spec.grammar == NativeCompilationGrammar::NamespaceLegacy
        && selection == NativeCompilationSelection::Generic
        && tcl_registry::native_namespace_upvar_compilation::compile_native_namespace_upvar(
            captured,
            1,
            dialect
                .tcl_version
                .ok_or(NativeByteCommandUnavailable::Recipe)?,
            context,
        )
        .map_err(|_| NativeByteCommandUnavailable::Recipe)?
        .visits
        .is_empty()
    {
        return Ok(NativeByteCommandPlan::Generic);
    }
    let generic_preparation = dialect
        .tcl_version
        .and_then(|version| {
            tcl_registry::native_instruction_plan::original_dictionary_preparations(
                spec, captured, 1, version, context,
            )
        })
        .is_some();
    match selection {
        NativeCompilationSelection::Generic
            if !generic_preparation
                && spec.namespace_binding_kind().is_none()
                && !matches!(
                    spec.grammar,
                    NativeCompilationGrammar::Array { .. }
                        | NativeCompilationGrammar::Conditional
                        | NativeCompilationGrammar::ForLoop
                        | NativeCompilationGrammar::Catch
                        | NativeCompilationGrammar::Try
                        | NativeCompilationGrammar::Foreach
                        | NativeCompilationGrammar::WhileLoop
                ) =>
        {
            Ok(NativeByteCommandPlan::Generic)
        }
        NativeCompilationSelection::NamedInvocation { .. } => registered_named_command_plan(
            captured, binding, entry, dialect, spec, selection, context,
        ),
        NativeCompilationSelection::Generic | NativeCompilationSelection::Inline { .. } => {
            let instruction = tcl_registry::native_instruction_plan::native_instruction_plan(
                spec, selection, captured, 1, dialect, context,
            )
            .map_err(|_| NativeByteCommandUnavailable::Recipe)?;
            Ok(NativeByteCommandPlan::Registered(Box::new(
                NativeByteRegisteredCommand {
                    spec,
                    selection,
                    context,
                    operand_from: 1,
                    binding: binding.clone(),
                    dependencies: Vec::new(),
                    recipe: OriginalRegisteredRecipe {
                        words: captured.original_words().to_vec(),
                        spec,
                        selection,
                        context,
                        operand_from: 1,
                        instruction,
                    },
                    prerequisite: None,
                },
            )))
        }
        _ => Err(NativeByteCommandUnavailable::Recipe),
    }
}

fn registered_named_command_plan(
    captured: &NativeCompilerWords<'_>,
    binding: &NativeCompilationBinding,
    entry: &NativeCompilationEntry,
    dialect: InvocationDialect,
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    context: NativeCompilationContext,
) -> Result<NativeByteCommandPlan, NativeByteCommandUnavailable> {
    let tcl_registry::native_instruction_plan::NativeInstructionPlan::NamedInvocation(mut recipe) =
        tcl_registry::native_instruction_plan::native_instruction_plan(
            spec, selection, captured, 1, dialect, context,
        )
        .map_err(|_| NativeByteCommandUnavailable::Recipe)?
    else {
        return Err(NativeByteCommandUnavailable::Recipe);
    };
    recipe.name = tcl_syntax::naming::native_command_full_name_bytes(&binding.slot);
    let prerequisite = tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
        interpreter: entry.interpreter,
        lookup_namespace_token: entry.current_namespace,
        invocation_word: captured
            .literal(0)
            .ok_or(NativeByteCommandUnavailable::Recipe)?
            .into(),
        slot: binding.slot.clone(),
        namespace_token: binding.namespace_token,
        token: binding.token,
        implementation_generation: binding.implementation_generation,
        compiler: binding
            .compiler
            .clone()
            .ok_or(NativeByteCommandUnavailable::Registration)?,
        selected_worker: None,
        nested_compilers: Vec::new(),
        guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
    };
    Ok(NativeByteCommandPlan::Named(Box::new(
        NativeByteNamedInvocation {
            recipe,
            prerequisite,
        },
    )))
}

/// Certify every original native command and bracket child against the exact
/// entry snapshot. Registered operations additionally require the implemented
/// original operand owner. Runtime handler facts cannot satisfy this check.
pub(crate) fn closed_original_byte_compilation(
    module: &crate::ir::Module,
    entry: &NativeCompilationEntry,
    registry: &CommandRegistry,
    context: NativeCompilationContext,
) -> bool {
    closed_byte_commands(module, entry, |words, scripts| {
        let Ok(plan) = native_byte_command_plan(words, entry, registry, context) else {
            return false;
        };
        if let NativeByteCommandPlan::Registered(plan) = plan
            && !registered_operand_available(words, entry, &plan)
        {
            return false;
        }
        queue_child_scripts(words, scripts)
    })
}

fn registered_operand_available(
    words: &[NativeWord],
    entry: &NativeCompilationEntry,
    plan: &NativeByteRegisteredCommand,
) -> bool {
    compiler_dialect(entry).is_ok()
        && entry.source_string_protocol.is_some()
        && plan.original_instruction(words).is_some()
}

/// Certify original generic C compilation, without granting registered opcodes.
/// Every bracket child retains its full original image, channel and body span;
/// an unavailable child or compiler hook retains the provider obligation.
#[cfg(test)]
pub(crate) fn closed_generic_byte_compilation(
    module: &crate::ir::Module,
    entry: &NativeCompilationEntry,
) -> bool {
    closed_byte_commands(module, entry, |words, scripts| {
        generic_command(words, entry, scripts)
    })
}

fn closed_byte_commands(
    module: &crate::ir::Module,
    entry: &NativeCompilationEntry,
    mut command_available: impl FnMut(
        &[NativeWord],
        &mut Vec<(SourceImage, Span, tcl_lexer::LexerConfig)>,
    ) -> bool,
) -> bool {
    if compiler_dialect(entry).is_err() {
        return false;
    }
    let Some(namespace) = module.native_namespace.as_ref() else {
        return false;
    };
    let mut contexts = entry
        .namespaces
        .iter()
        .filter(|row| row.token == entry.current_namespace);
    if contexts.next().is_none_or(|row| &row.path != namespace) || contexts.next().is_some() {
        return false;
    }
    let mut scripts = Vec::new();
    for statement in &module.top_level.statements {
        let crate::ir::Statement::NativeCall { words, .. } = statement else {
            return false;
        };
        if words.iter().any(|word| word.image() != &module.source)
            || !command_available(words, &mut scripts)
        {
            return false;
        }
    }
    while let Some((image, region, config)) = scripts.pop() {
        let Ok(plan) = native_script_words_in(image, region, config) else {
            return false;
        };
        if plan.fatal_tail.is_some() {
            return false;
        }
        for command in plan.commands {
            if !command_available(&command.words, &mut scripts) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
fn generic_command(
    words: &[NativeWord],
    entry: &NativeCompilationEntry,
    scripts: &mut Vec<(SourceImage, Span, tcl_lexer::LexerConfig)>,
) -> bool {
    let Some(protocol) = entry.source_string_protocol else {
        return false;
    };
    let Ok(captured) = NativeCompilerWords::capture(words, protocol) else {
        return false;
    };
    let Ok(dialect) = compiler_dialect(entry) else {
        return false;
    };
    let head = captured.shapes()[0];
    if head == NativeCompilationWordShape::Expanded {
        return unprojected_expanded_head(&captured, dialect)
            .is_ok_and(|unprojected| unprojected && queue_child_scripts(words, scripts));
    }
    match head.compiler_head(Some(dialect)) {
        Some(false) => {}
        Some(true) => {
            let Some(bytes) = captured.literal(0) else {
                return false;
            };
            match entry.lookup_command_bytes(entry.current_namespace, bytes) {
                Ok(Some(binding))
                    if binding.compiler_hook == NativeCompilerHookPresence::Absent
                        || binding.has_execution_trace => {}
                Ok(None) => {}
                _ => return false,
            }
        }
        None => return false,
    }
    queue_child_scripts(words, scripts)
}

fn queue_child_scripts(
    words: &[NativeWord],
    scripts: &mut Vec<(SourceImage, Span, tcl_lexer::LexerConfig)>,
) -> bool {
    for word in words {
        let arena = word.executable_parts();
        for component in arena.all_parts() {
            match component.part {
                ExecutablePart::Command { body } => {
                    if arena.bytes(body).is_none() {
                        return false;
                    }
                    scripts.push((arena.image().clone(), body, word.config()));
                }
                ExecutablePart::ParseError(_) | ExecutablePart::Expression { .. } => return false,
                ExecutablePart::Text(_) | ExecutablePart::Variable { .. } => {}
            }
        }
    }
    true
}

#[cfg(test)]
mod tests;
