// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native compiler plans from immutable byte registrations.

use tcl_lexer::word_parts::{ExecutablePart, NativeWord};
use tcl_lexer::{SourceImage, Span, native_script_words_in};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationGrammar, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationSpec, NativeCompilationWordShape,
};
use tcl_registry::native_compiler_words::{NativeCompilerWords, NativeCompilerWordsUnavailable};
use tcl_registry::{CommandRegistry, InvocationDialect};
use tcl_runtime_api::NativeCompilationEntry;
use tcl_runtime_api::native_compilation::{NativeCompilationBinding, NativeCompilerHookPresence};

/// Original generic dispatch or an independently registered opcode recipe.
/// A registered plan still requires an emitter implementing its operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeByteCommandPlan {
    /// Native compilation emits ordinary dispatch without choosing its handler.
    Generic,
    /// The original compiler selected a retained, supported operation.
    Registered(Box<NativeByteRegisteredCommand>),
    /// Actual configured no-hook worker and original ensemble usage rewrite.
    Named(Box<NativeByteNamedInvocation>),
}

/// Original named worker selection, without retaining its callable body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeByteNamedInvocation {
    /// Exact parser operand layout and canonical member replacement.
    pub recipe: tcl_registry::native_instruction_plan::NativeNamedInvocationInstruction,
    /// Original public compiler/configuration and actual selected worker.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
}

/// Exact original registration and authored compiler recipe. This grants no
/// normal-handler, variable contents, purity or runtime object-class facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeByteRegisteredCommand {
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
    /// Original configured public compiler and selected worker, when delegated.
    pub prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
}

/// Unavailable original compilation, distinct from generic or guest rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeByteCommandUnavailable {
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
pub(crate) fn native_byte_command_plan(
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
    if binding
        .compiler
        .as_ref()
        .is_some_and(|compiler| compiler.ensemble.is_some())
    {
        return ensemble_command_plan(&captured, binding, entry, registry, dialect, context);
    }
    registered_command_plan(&captured, binding, dialect, registry, context)
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
    use tcl_registry::native_selected_worker::{
        OriginalSelectedWorkerCompilation, OriginalSelectedWorkerInvocation,
    };
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
            let prerequisite = selected.prerequisite;
            match selected.compilation {
                OriginalSelectedWorkerCompilation::PublicGeneric => {
                    Ok(NativeByteCommandPlan::Generic)
                }
                OriginalSelectedWorkerCompilation::Named(recipe) => Ok(
                    NativeByteCommandPlan::Named(Box::new(NativeByteNamedInvocation {
                        recipe,
                        prerequisite,
                    })),
                ),
                OriginalSelectedWorkerCompilation::Operation {
                    spec, selection, ..
                } => Ok(NativeByteCommandPlan::Registered(Box::new(
                    NativeByteRegisteredCommand {
                        spec,
                        selection,
                        context,
                        operand_from: selected.operand_from,
                        binding: selected.worker,
                        dependencies: Vec::new(),
                        prerequisite: Some(prerequisite),
                    },
                ))),
                OriginalSelectedWorkerCompilation::Unavailable => {
                    Err(NativeByteCommandUnavailable::Recipe)
                }
            }
        }
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
        .name_protocol
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    if tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(policy) {
        return Err(NativeByteCommandUnavailable::Entry);
    }
    let version = point
        .tcl_version()
        .ok_or(NativeByteCommandUnavailable::Entry)?;
    Ok(InvocationDialect::for_version(version))
}

fn registered_command_plan(
    captured: &NativeCompilerWords<'_>,
    binding: &NativeCompilationBinding,
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
    match selection {
        NativeCompilationSelection::Generic
            if spec.namespace_binding_kind().is_none()
                && !matches!(
                    spec.grammar,
                    NativeCompilationGrammar::Conditional
                        | NativeCompilationGrammar::ForLoop
                        | NativeCompilationGrammar::Catch
                        | NativeCompilationGrammar::Try
                        | NativeCompilationGrammar::Foreach
                        | NativeCompilationGrammar::WhileLoop
                ) =>
        {
            Ok(NativeByteCommandPlan::Generic)
        }
        NativeCompilationSelection::Generic | NativeCompilationSelection::Inline { .. } => {
            if tcl_registry::native_instruction_plan::native_instruction_plan(
                spec, selection, captured, 1, dialect, context,
            )
            .is_err()
            {
                return Err(NativeByteCommandUnavailable::Recipe);
            }
            Ok(NativeByteCommandPlan::Registered(Box::new(
                NativeByteRegisteredCommand {
                    spec,
                    selection,
                    context,
                    operand_from: 1,
                    binding: binding.clone(),
                    dependencies: Vec::new(),
                    prerequisite: None,
                },
            )))
        }
        _ => Err(NativeByteCommandUnavailable::Recipe),
    }
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
    let Ok(dialect) = compiler_dialect(entry) else {
        return false;
    };
    let Some(protocol) = entry.source_string_protocol else {
        return false;
    };
    NativeCompilerWords::capture(words, protocol).is_ok_and(|words| {
        tcl_registry::native_instruction_plan::native_instruction_plan(
            plan.spec,
            plan.selection,
            &words,
            plan.operand_from,
            dialect,
            plan.context,
        )
        .is_ok()
    })
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
mod tests {
    use super::*;
    use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};
    use tcl_lexer::{Lexer, LexerConfig, group_commands_bytes};
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilationBinding, NativeCompilationFrame,
        NativeCompilationNamespace, NativeCompilerHookPresence, NativeInterpreterIdentity,
        NativeVariableObserverPresence,
    };

    #[test]
    fn original_namespace_legacy_declines_non_upvar_before_emitter_admission() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for source in [
            b"namespace eval N {}".as_slice(),
            b"namespace {upvar} N x y",
            b"namespace upvar N x",
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                tcl_lexer::Span::new(
                    0,
                    u32::try_from(source.len()).expect("fixture source fits Span"),
                ),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            assert_eq!(
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
                Ok(NativeByteCommandPlan::Generic)
            );
        }
        let source = b"namespace upvar N x y";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let Ok(NativeByteCommandPlan::Registered(plan)) =
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
        else {
            panic!("native C8.5 accepted namespace upvar compiler");
        };
        assert!(matches!(
            plan.selection,
            NativeCompilationSelection::Inline { .. }
        ));
    }

    #[test]
    fn original_monolithic_match_retains_real_hook_and_registration_guards() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let source = b"string match * $subject";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        let plan = native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
            .unwrap();
        let NativeByteCommandPlan::Registered(selected) = plan else {
            panic!("original monolithic StringMatch compiler");
        };
        assert!(matches!(
            selected.spec.grammar,
            NativeCompilationGrammar::StringMatch(
                tcl_registry::native_string_compilation::NativeStringMatchScope::PublicMember
            )
        ));
        assert_eq!(
            selected.binding.compiler_hook,
            NativeCompilerHookPresence::Present
        );
        let original = entry
            .lookup_command_bytes(entry.current_namespace, b"string")
            .unwrap()
            .unwrap();
        assert_eq!(selected.binding.token, original.token);
        assert_eq!(selected.binding.compiler, original.compiler);
        for presence in [
            NativeCompilerHookPresence::Absent,
            NativeCompilerHookPresence::Unknown,
        ] {
            let mut changed = entry.clone();
            changed
                .commands
                .iter_mut()
                .find(|binding| binding.token == original.token)
                .unwrap()
                .compiler_hook = presence;
            let plan =
                native_byte_command_plan(&parsed.commands[0].words, &changed, &registry, context);
            if presence == NativeCompilerHookPresence::Absent {
                assert_eq!(plan.unwrap(), NativeByteCommandPlan::Generic);
            } else {
                assert!(plan.is_err());
            }
        }
    }

    #[test]
    fn registered_concat_uses_shared_recipe_and_original_worker_authority() {
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in tcl_dialect::TclVersion::ALL {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let mut entry = crate::environment_ingress::captured_native_entry(profile);
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            for source in [
                b"concat { A } { B }".as_slice(),
                b"concat $a $b",
                b"concat {*}{A B} $c",
            ] {
                let parsed = native_script_words_in(
                    SourceImage::native(source),
                    tcl_lexer::Span::new(
                        0,
                        u32::try_from(source.len()).expect("fixture source fits Span"),
                    ),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let plan =
                    native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
                        .unwrap();
                if version < tcl_dialect::TclVersion::V8_6 {
                    assert_eq!(plan, NativeByteCommandPlan::Generic);
                } else {
                    assert!(
                        matches!(plan, NativeByteCommandPlan::Registered(_)),
                        "{version:?}: {source:?}"
                    );
                }
            }
            let source = b"concat $a $b";
            let parsed = native_script_words_in(
                SourceImage::native(source.as_slice()),
                tcl_lexer::Span::new(
                    0,
                    u32::try_from(source.len()).expect("fixture source fits Span"),
                ),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let target = entry
                .commands
                .iter_mut()
                .find(|binding| binding.slot.simple.as_bytes() == b"concat")
                .unwrap();
            target.implementation_generation += 1;
            target.implementation = NativeCommandImplementation::Opaque;
            target.compiler_hook = NativeCompilerHookPresence::Absent;
            target.compiler = None;
            assert_eq!(
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
                Ok(NativeByteCommandPlan::Generic)
            );
            target_unknown(&mut entry, b"concat");
            assert_eq!(
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Registration)
            );
        }
    }

    #[test]
    fn captured_c85_namespace_root_retains_accepted_and_partial_upvar_preparation() {
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingKind, NativeNamespaceBindingOutcome,
        };
        let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        let original_binding = entry
            .lookup_command_bytes(entry.current_namespace, b"namespace")
            .unwrap()
            .unwrap();
        assert!(
            original_binding
                .compiler
                .as_ref()
                .unwrap()
                .ensemble
                .is_none()
        );
        for (source, inline) in [
            (b"namespace upvar N x y".as_slice(), true),
            (b"namespace upvar N x y z bad(k)", false),
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                Span::new(
                    0,
                    u32::try_from(source.len()).expect("fixture source fits Span"),
                ),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = &parsed.commands[0].words;
            let NativeByteCommandPlan::Registered(plan) =
                native_byte_command_plan(words, &entry, &registry, context).unwrap()
            else {
                panic!("original namespace compiler preparation must remain registered");
            };
            assert_eq!(&plan.binding, original_binding);
            assert_eq!(plan.spec.grammar, NativeCompilationGrammar::NamespaceLegacy);
            let captured =
                NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
            let tcl_registry::native_instruction_plan::NativeInstructionPlan::NamespaceBindings(
                recipe,
            ) = tcl_registry::native_instruction_plan::native_instruction_plan(
                plan.spec,
                plan.selection,
                &captured,
                plan.operand_from,
                tcl_registry::InvocationDialect::of_profile(profile),
                plan.context,
            )
            .unwrap()
            else {
                panic!("actual monolithic compiler must select original namespace binding visits");
            };
            assert_eq!(recipe.kind, NativeNamespaceBindingKind::Upvar);
            assert_eq!(
                recipe.outcome == NativeNamespaceBindingOutcome::Inline,
                inline
            );
            assert!(!recipe.visits.is_empty());
        }
    }

    #[test]
    fn captured_namespace_no_hook_workers_retain_original_named_rewrites() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            };
            for source in [
                b"namespace eval N {}".as_slice(),
                b"namespace inscope N {}",
                b"namespace ensemble exists N",
            ] {
                let parsed = native_script_words_in(
                    SourceImage::native(source),
                    Span::new(
                        0,
                        u32::try_from(source.len()).expect("fixture source fits Span"),
                    ),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let NativeByteCommandPlan::Named(plan) =
                    native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
                        .unwrap()
                else {
                    panic!("original no-hook worker must retain the native named rewrite");
                };
                let worker = plan.prerequisite.selected_worker.as_ref().unwrap();
                assert_eq!(worker.compiler_hook, NativeCompilerHookPresence::Absent);
                assert_eq!(plan.recipe.arguments_from, 1);
                assert_eq!(
                    plan.prerequisite.lookup_namespace_token,
                    entry.current_namespace
                );
                assert_eq!(plan.prerequisite.interpreter, entry.interpreter);
            }
            assert!(
                registry
                    .native_compilation_for_registration(
                        "tcl::namespace::unregistered",
                        tcl_registry::InvocationDialect::of_profile(profile)
                    )
                    .is_none()
            );
        }
    }

    fn target_unknown(entry: &mut NativeCompilationEntry, name: &[u8]) {
        entry
            .commands
            .iter_mut()
            .find(|binding| binding.slot.simple.as_bytes() == name)
            .unwrap()
            .compiler_hook = NativeCompilerHookPresence::Unknown;
    }

    #[test]
    fn registered_recipe_without_shared_instruction_remains_unavailable() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let source = b"llength $items";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        assert_eq!(
            native_byte_command_plan(
                &parsed.commands[0].words,
                &entry,
                &registry,
                NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    ..Default::default()
                }
            ),
            Err(NativeByteCommandUnavailable::Recipe)
        );
    }

    fn entry(version: tcl_dialect::TclVersion) -> NativeCompilationEntry {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let point = tcl_dialect::model::DialectPoint::for_tcl_version(version);
        NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: 7,
                interpreter: 0,
            },
            epoch: 3,
            profile: profile.cache_key(),
            invocation_policy: Some(profile.cache_key()),
            expression_policy:
                tcl_registry::native_expression_program::native_expression_evaluation_policy(
                    profile, point,
                ),
            execution_point: Some(point),
            name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
            compiled_variable_protocol:
                tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
            compiled_local_layout: None,
            ensemble_target_objects: None,
            source_string_protocol: tcl_registry::InvocationDialect::of_point(point)
                .native_source_string_protocol(),
            lexer_grammar: Some(profile.grammar),
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            variable_observers: NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: true,
            commands: vec![],
            namespaces: vec![namespace(1, ByteNamespacePath::root(), true)],
            current_namespace: 1,
            frame: NativeCompilationFrame::Global,
        }
    }

    fn namespace(token: u64, path: ByteNamespacePath, visible: bool) -> NativeCompilationNamespace {
        NativeCompilationNamespace {
            path,
            token,
            visible,
            jim_namespace_object: None,
            exports: vec![],
            command_path: vec![],
            unknown_handler: None,
        }
    }

    fn binding(
        namespace: &NativeCompilationNamespace,
        name: &[u8],
        hook: NativeCompilerHookPresence,
    ) -> NativeCompilationBinding {
        NativeCompilationBinding {
            slot: NativeByteCommandSlot::new(namespace.path.clone(), NameBytes::from(name)),
            namespace_token: namespace.token,
            token: 11,
            implementation_generation: 4,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: hook,
            compiler: None,
            procedure_header: None,
            has_execution_trace: false,
        }
    }

    fn install_original_target_fixture(entry: &mut NativeCompilationEntry) {
        let mut identity = 1;
        entry.ensemble_target_objects = Some(entry.commands.iter().flat_map(|command| {
            command.compiler.as_ref().and_then(|compiler| compiler.ensemble.as_ref()).into_iter().flat_map(move |configuration| {
                configuration.map.iter().flat_map(move |(member, prefix)| prefix.iter().enumerate().map(move |(position, name)| (command.token, member.clone(), position, name.clone())))
            })
        }).map(|(token, member, position, name)| {
            let observation = tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation {
                ensemble_token: token, member, prefix_index: position, object_identity: identity,
                resident_name: name.clone(), primary: if name.is_some() { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::StockString } else { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::Unavailable },
            };
            identity += 1; observation
        }).collect());
    }

    fn source_module(source: &[u8], entry: &NativeCompilationEntry) -> crate::ir::Module {
        let image = SourceImage::native(source);
        let config = LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let statements = group_commands_bytes(&tokens, image.bytes(), config)
            .into_iter()
            .map(|group| crate::ir::Statement::NativeCall {
                span: group.span,
                words: group
                    .words
                    .iter()
                    .map(|word| {
                        NativeWord::from_group(image.clone(), config, &tokens, word).unwrap()
                    })
                    .collect(),
            })
            .collect();
        let mut module = crate::ir::Module {
            source: image,
            native_namespace: Some(
                entry
                    .namespaces
                    .iter()
                    .find(|row| row.token == entry.current_namespace)
                    .unwrap()
                    .path
                    .clone(),
            ),
            ..crate::ir::Module::default()
        };
        module.top_level.statements = statements;
        module
    }

    #[test]
    fn original_generic_selection_precedes_inline_emitter_availability() {
        use tcl_runtime_api::native_compilation::NativeCommandCompiler;
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in tcl_dialect::TclVersion::ALL {
            let mut entry = entry(version);
            let mut target = binding(
                &entry.namespaces[0],
                b"original_worker",
                if version < tcl_dialect::TclVersion::V9_1 {
                    NativeCompilerHookPresence::Absent
                } else {
                    NativeCompilerHookPresence::Present
                },
            );
            target.compiler = Some(NativeCommandCompiler {
                registry_identity: "uplevel".into(),
                ensemble: None,
            });
            entry.commands.push(target);
            let module = source_module(b"original_worker 1 {error BODY}", &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original words");
            };
            let selected = native_byte_command_plan(words, &entry, &registry, context);
            if version < tcl_dialect::TclVersion::V9_1 {
                assert!(
                    matches!(selected, Ok(NativeByteCommandPlan::Generic)),
                    "{version:?}"
                );
            } else {
                let NativeByteCommandPlan::Registered(plan) = selected.unwrap() else {
                    panic!("C9.1 original Uplevel compiler recipe");
                };
                assert_eq!(plan.binding, entry.commands[0]);
                assert_eq!(plan.context, context);
                assert_eq!(plan.operand_from, 1);
                assert!(plan.dependencies.is_empty());
                assert!(
                    plan.prerequisite.is_none(),
                    "ordinary compiler has no delegated worker"
                );
                let original =
                    NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap())
                        .unwrap();
                assert_eq!(
                    tcl_registry::native_instruction_plan::native_instruction_plan(
                        plan.spec,
                        plan.selection,
                        &original,
                        plan.operand_from,
                        InvocationDialect::for_version(version),
                        context,
                    )
                    .unwrap(),
                    tcl_registry::native_instruction_plan::NativeInstructionPlan::Uplevel(
                        tcl_registry::native_instruction_plan::NativeUplevelInstruction {
                            level_word: Some(1),
                            script_words: 2..3,
                        },
                    ),
                );
            }
            if version == tcl_dialect::TclVersion::V9_1 {
                let script_context = NativeCompilationContext {
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..context
                };
                assert!(matches!(
                    native_byte_command_plan(words, &entry, &registry, script_context),
                    Ok(NativeByteCommandPlan::Generic)
                ));
                entry.commands[0].compiler = None;
                assert!(matches!(
                    native_byte_command_plan(words, &entry, &registry, context),
                    Err(NativeByteCommandUnavailable::Registration)
                ));
            }
        }
    }

    #[test]
    fn original_dynamic_expanded_head_keeps_generic_timing_and_literal_cpp_obligations() {
        // Native six-engine control: tests/data/native_expanded_head/observations.tsv.
        // Modern C uses invokeExpanded for a dynamic head and storeScalar for
        // the literal parser-projected head, including command replacement.
        let observations = include_str!("../tests/data/native_expanded_head/observations.tsv");
        assert_eq!(observations.lines().count(), 6);
        assert!(
            observations
                .lines()
                .any(|row| row.starts_with("8.4|0|0|1|"))
        );
        assert!(
            observations
                .lines()
                .any(|row| row.starts_with("jim|0|0|0|A|1|"))
        );
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            let observation = observations
                .lines()
                .find(|row| row.starts_with(&format!("{}|", version.version_string())))
                .unwrap();
            let fields: Vec<_> = observation.split('|').collect();
            assert_eq!(
                &fields[1..3],
                &["1", "1"],
                "dynamic dispatch and literal inline CPP"
            );
            assert_eq!(
                &fields[9..13],
                &[
                    "1",
                    "{too many nested evaluations (infinite loop?)}",
                    "0",
                    "A"
                ]
            );
            let entry = entry(version);
            for source in [b"{*}$command VALUE".as_slice(), b"{*}$command set x A"] {
                let module = source_module(source, &entry);
                let crate::ir::Statement::NativeCall { words, .. } =
                    &module.top_level.statements[0]
                else {
                    panic!("original expanded command");
                };
                assert_eq!(
                    native_byte_command_plan(words, &entry, &registry, context).unwrap(),
                    NativeByteCommandPlan::Generic,
                    "{version:?}"
                );
                assert!(closed_original_byte_compilation(
                    &module, &entry, &registry, context
                ));
                assert!(closed_generic_byte_compilation(&module, &entry));
                let mut missing_point = entry.clone();
                missing_point.execution_point = None;
                assert_eq!(
                    native_byte_command_plan(words, &missing_point, &registry, context),
                    Err(NativeByteCommandUnavailable::Entry)
                );
                let mut unsupported = entry.clone();
                unsupported.execution_point =
                    Some(tcl_dialect::model::DialectPoint::for_tcl_version(
                        tcl_dialect::TclVersion::V8_4,
                    ));
                unsupported.name_protocol =
                    tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                        unsupported.execution_point.unwrap(),
                    );
                assert_eq!(
                    native_byte_command_plan(words, &unsupported, &registry, context),
                    Err(NativeByteCommandUnavailable::Recipe)
                );
            }
            for source in [b"{*}{set x} A".as_slice(), b"{*}{} set x A"] {
                let module = source_module(source, &entry);
                let crate::ir::Statement::NativeCall { words, .. } =
                    &module.top_level.statements[0]
                else {
                    panic!("original projected command");
                };
                assert_eq!(
                    native_byte_command_plan(words, &entry, &registry, context),
                    Err(NativeByteCommandUnavailable::Recipe)
                );
                assert!(!closed_generic_byte_compilation(&module, &entry));
            }
        }
    }

    #[test]
    fn byte_generic_certificate_requires_original_absent_hook_or_closed_absence() {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        let module = source_module(b"opaque\xff ARG", &entry);
        assert!(closed_generic_byte_compilation(&module, &entry));
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"opaque\xff",
            NativeCompilerHookPresence::Absent,
        ));
        assert!(closed_generic_byte_compilation(&module, &entry));
        for hook in [
            NativeCompilerHookPresence::Present,
            NativeCompilerHookPresence::Unknown,
        ] {
            entry.commands[0].compiler_hook = hook;
            assert!(!closed_generic_byte_compilation(&module, &entry));
        }
        entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
        entry.closed = false;
        assert!(!closed_generic_byte_compilation(&module, &entry));
        entry.closed = true;
        entry.execution_point = None;
        assert!(!closed_generic_byte_compilation(&module, &entry));
    }

    #[test]
    fn configured_byte_worker_uses_its_installed_named_compiler() {
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut entry = entry(version);
            let mut worker = binding(
                &entry.namespaces[0],
                b"private",
                NativeCompilerHookPresence::Present,
            );
            worker.token = 12;
            worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "::tcl::info::locals".into(),
                ensemble: None,
            });
            let mut public = binding(
                &entry.namespaces[0],
                b"ensemble",
                NativeCompilerHookPresence::Present,
            );
            public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "custom-ensemble".into(),
                ensemble: Some(
                    tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                        namespace_token: entry.current_namespace,
                        map: vec![("member".into(), vec![Some("private".into())])],
                        subcommands: None,
                        prefixes: true,
                        parameters: vec![],
                        unknown_handler: None,
                    },
                ),
            });
            entry.commands.extend([public, worker.clone()]);
            install_original_target_fixture(&mut entry);
            for (source, accepted) in [
                (b"ensemble mem $pattern".as_slice(), true),
                (b"ensemble member a b".as_slice(), false),
            ] {
                let module = source_module(source, &entry);
                let crate::ir::Statement::NativeCall { words, .. } =
                    &module.top_level.statements[0]
                else {
                    panic!("original native words")
                };
                let result = native_byte_command_plan(words, &entry, &registry, context).unwrap();
                let NativeByteCommandPlan::Named(selected) = result else {
                    panic!("installed worker compiler")
                };
                assert_eq!(
                    selected.recipe.protocol,
                    if accepted {
                        tcl_registry::native_compilation::NativeNamedInvocationProtocol::Direct
                    } else {
                        tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite
                    }
                );
                assert_eq!(selected.recipe.name, b"::private");
                assert_eq!(
                    selected.prerequisite.selected_worker.as_ref(),
                    Some(&worker)
                );
                if accepted {
                    assert_eq!(selected.recipe.words, [tcl_registry::native_instruction_plan::NativeNamedInvocationWord::Original(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(2))]);
                } else {
                    assert_eq!(selected.recipe.words.len(), 4);
                }
                assert!(closed_original_byte_compilation(
                    &module, &entry, &registry, context
                ));
            }
        }
    }

    #[test]
    fn original_selected_worker_trace_veto_and_dictionary_operation_are_distinct() {
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut entry = entry(version);
            let mut worker = binding(
                &entry.namespaces[0],
                b"private",
                NativeCompilerHookPresence::Present,
            );
            worker.token = 12;
            worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "::tcl::dict::get".into(),
                ensemble: None,
            });
            let mut public = binding(
                &entry.namespaces[0],
                b"ensemble",
                NativeCompilerHookPresence::Present,
            );
            public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "custom-ensemble".into(),
                ensemble: Some(
                    tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                        namespace_token: entry.current_namespace,
                        map: vec![("member".into(), vec![Some("private".into())])],
                        subcommands: None,
                        prefixes: true,
                        parameters: vec![],
                        unknown_handler: None,
                    },
                ),
            });
            entry.commands.extend([public, worker.clone()]);
            install_original_target_fixture(&mut entry);
            let module = source_module(b"ensemble member $dictionary $key", &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("retained original source");
            };
            let NativeByteCommandPlan::Registered(selected) =
                native_byte_command_plan(words, &entry, &registry, context).unwrap()
            else {
                panic!("actual dictionary worker instruction");
            };
            assert_eq!(selected.operand_from, 2);
            assert_eq!(selected.binding, worker);
            assert_eq!(
                selected
                    .prerequisite
                    .as_ref()
                    .unwrap()
                    .selected_worker
                    .as_ref(),
                Some(&worker)
            );
            let captured =
                NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
            let tcl_registry::native_instruction_plan::NativeInstructionPlan::DictionaryLookup(
                recipe,
            ) = tcl_registry::native_instruction_plan::native_instruction_plan(
                selected.spec,
                selected.selection,
                &captured,
                2,
                InvocationDialect::for_version(version),
                context,
            )
            .unwrap()
            else {
                panic!("shared dictionary original geometry");
            };
            assert_eq!(recipe.operands, [tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(2), tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(3)]);
            entry.commands[1].has_execution_trace = true;
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context).unwrap(),
                NativeByteCommandPlan::Generic
            );
        }
    }

    fn configured_collision_fixture(
        version: tcl_dialect::TclVersion,
        target: &str,
    ) -> (
        NativeCompilationEntry,
        NativeCompilationBinding,
        NativeCompilationBinding,
    ) {
        let mut entry = entry(version);
        entry.namespaces.push(namespace(
            2,
            ByteNamespacePath::from_segments([b"a:".as_slice()]),
            true,
        ));
        entry.namespaces.push(namespace(
            3,
            ByteNamespacePath::from_segments([b"a".as_slice()]),
            true,
        ));
        let mut worker = binding(
            &entry.namespaces[1],
            b"w",
            NativeCompilerHookPresence::Absent,
        );
        worker.token = 12;
        let mut collision = binding(
            &entry.namespaces[2],
            b"w",
            NativeCompilerHookPresence::Absent,
        );
        collision.token = 13;
        let mut public = binding(
            &entry.namespaces[0],
            b"ensemble",
            NativeCompilerHookPresence::Present,
        );
        public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "custom-ensemble".into(),
            ensemble: Some(
                tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                    namespace_token: 2,
                    map: vec![("member".into(), vec![Some(target.into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: vec![],
                    unknown_handler: None,
                },
            ),
        });
        entry
            .commands
            .extend([public, worker.clone(), collision.clone()]);
        install_original_target_fixture(&mut entry);
        (entry, worker, collision)
    }

    fn cached_worker_target(
        entry: &NativeCompilationEntry,
        version: tcl_dialect::TclVersion,
        worker: &NativeCompilationBinding,
    ) -> tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary {
        use tcl_runtime_api::native_command_name::{
            NativeCommandNameCache, NativeCommandNameLookupState, NativeCommandNameReference,
            NativeCommandNameTarget,
        };
        let cache = NativeCommandNameCache {
            interpreter: entry.interpreter,
            version,
            slot: worker.slot.clone(),
            namespace_token: 2,
            token: worker.token,
            implementation_generation: worker.implementation_generation,
            command_epoch: 4,
            reference: None,
        };
        let lookup = NativeCommandNameLookupState {
            interpreter: entry.interpreter,
            reference: NativeCommandNameReference {
                namespace_token: 2,
                command_reference_epoch: 1,
            },
            target: Some(NativeCommandNameTarget {
                token: worker.token,
                implementation_generation: worker.implementation_generation,
                command_epoch: 4,
                namespace_token: 2,
                namespace_dying: false,
            }),
        };
        tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::CommandName {
            origin: version,
            cache: Some(cache),
            lookup: Some(lookup),
        }
    }

    #[test]
    fn original_map_cache_selects_its_node_and_missing_evidence_stays_unknown() {
        use tcl_registry::native_ensemble::{
            NativeEnsembleWorkerSelection, select_worker_in_entry,
        };
        use tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary;
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let (mut entry, worker, collision) = configured_collision_fixture(version, "::a:::w");
            let before = entry.clone();
            entry.ensemble_target_objects.as_mut().unwrap()[0].primary =
                cached_worker_target(&entry, version, &worker);
            assert_ne!(entry, before);
            assert!(entry.same_compilation_world(&before));
            let configuration = entry.commands[0]
                .compiler
                .as_ref()
                .unwrap()
                .ensemble
                .as_ref()
                .unwrap()
                .clone();
            let select = |entry: &NativeCompilationEntry| {
                select_worker_in_entry(
                    entry,
                    11,
                    &configuration,
                    Some(b"member"),
                    Some(tcl_registry::native_compilation::NativeCompilationWordShape::Literal),
                    Some(InvocationDialect::for_version(version)),
                )
            };
            assert_eq!(
                select(&entry),
                NativeEnsembleWorkerSelection::Worker {
                    member: "member".into(),
                    binding: Box::new(worker.clone())
                }
            );
            let mut stale = entry.clone();
            let NativeEnsembleTargetPrimary::CommandName { lookup, .. } =
                &mut stale.ensemble_target_objects.as_mut().unwrap()[0].primary
            else {
                unreachable!()
            };
            lookup
                .as_mut()
                .unwrap()
                .target
                .as_mut()
                .unwrap()
                .command_epoch += 1;
            assert_eq!(
                select(&stale),
                NativeEnsembleWorkerSelection::Worker {
                    member: "member".into(),
                    binding: Box::new(collision)
                }
            );
            let mut missing = entry.clone();
            missing.ensemble_target_objects = None;
            assert_eq!(select(&missing), NativeEnsembleWorkerSelection::Unknown);
            let mut unavailable = entry.clone();
            unavailable.ensemble_target_objects.as_mut().unwrap()[0].primary =
                NativeEnsembleTargetPrimary::Unavailable;
            assert_eq!(select(&unavailable), NativeEnsembleWorkerSelection::Unknown);
            let mut mutation = entry.clone();
            mutation.epoch += 1;
            assert!(!entry.same_compilation_world(&mutation));
        }
    }

    #[test]
    fn literal_expansion_selects_original_ensemble_member_and_dictionary_operands() {
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut entry = entry(version);
            let mut worker = binding(
                &entry.namespaces[0],
                b"private",
                NativeCompilerHookPresence::Present,
            );
            worker.token = 12;
            worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "::tcl::dict::get".into(),
                ensemble: None,
            });
            let mut public = binding(
                &entry.namespaces[0],
                b"dict",
                NativeCompilerHookPresence::Present,
            );
            public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                registry_identity: "dict".into(),
                ensemble: Some(
                    tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                        namespace_token: 1,
                        map: vec![("get".into(), vec![Some("private".into())])],
                        subcommands: None,
                        prefixes: true,
                        parameters: vec![],
                        unknown_handler: None,
                    },
                ),
            });
            entry.commands.extend([public, worker]);
            install_original_target_fixture(&mut entry);
            let module = source_module(b"dict {*}{get {k v} k}", &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original expansion geometry");
            };
            let NativeByteCommandPlan::Registered(selected) =
                native_byte_command_plan(words, &entry, &registry, context).unwrap()
            else {
                panic!("actual expanded member compiler");
            };
            let captured =
                NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
            let tcl_registry::native_instruction_plan::NativeInstructionPlan::DictionaryLookup(
                recipe,
            ) = tcl_registry::native_instruction_plan::native_instruction_plan(
                selected.spec,
                selected.selection,
                &captured,
                selected.operand_from,
                InvocationDialect::for_version(version),
                context,
            )
            .unwrap()
            else {
                panic!("dictionary compiler recipe");
            };
            assert_eq!(recipe.key_count, 1);
            assert_eq!(recipe.operands.len(), 2);
            for operand in recipe.operands {
                assert!(matches!(operand, tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion { original_word: 1, .. }));
            }
        }
    }

    #[test]
    fn configured_byte_worker_priming_keeps_original_token_despite_reported_colons() {
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let (entry, worker, collision) = configured_collision_fixture(version, "w");
            let module = source_module(b"ensemble mem VALUE", &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original native command");
            };
            let selected = native_byte_command_plan(words, &entry, &registry, context).unwrap();
            if version == tcl_dialect::TclVersion::V8_5 {
                assert_eq!(selected, NativeByteCommandPlan::Generic);
                continue;
            }
            let NativeByteCommandPlan::Named(selected) = selected else {
                panic!("actual no-hook worker selection");
            };
            assert_eq!(
                selected.prerequisite.selected_worker.as_ref(),
                Some(&worker)
            );
            assert_eq!(selected.recipe.name, b"::a:::w");
            assert_eq!(
                entry
                    .lookup_command_bytes(entry.current_namespace, &selected.recipe.name)
                    .unwrap(),
                Some(&collision)
            );
            let literal = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
                entry.execution_point.unwrap(), entry.name_protocol.unwrap(),
                tcl_runtime_api::native_command_name::NativeLiteralContext { interpreter: entry.interpreter, namespace_token: entry.current_namespace, entry_epoch: entry.epoch, namespace_path: ByteNamespacePath::root() },
                &selected.recipe.name, selected.prerequisite.clone(),
            ).unwrap();
            let priming = literal.priming.unwrap();
            assert_eq!(priming.binding, worker);
            assert!(priming.matches_entry(&entry));
            let mut changed = entry.clone();
            changed.commands[0]
                .compiler
                .as_mut()
                .unwrap()
                .ensemble
                .as_mut()
                .unwrap()
                .prefixes = false;
            assert!(!priming.matches_entry(&changed));
            let mut changed = entry.clone();
            changed.commands[1].implementation_generation += 1;
            assert!(!priming.matches_entry(&changed));
            changed = entry.clone();
            changed.closed = false;
            assert!(!priming.matches_entry(&changed));
        }
    }

    #[test]
    fn byte_compiler_plan_retains_original_registration_for_opaque_ordinary_operands() {
        let registry = CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for (identity, source) in [
            ("set", b"renamed\xff x\xff\0tail VALUE".as_slice()),
            ("incr", b"renamed\xff x\xff\0tail 1".as_slice()),
            ("list", b"renamed\xff A\xff\0tail B".as_slice()),
            ("append", b"renamed\xff x\xff\0tail $value".as_slice()),
            ("lappend", b"renamed\xff x\xff\0tail $value".as_slice()),
            ("return", b"renamed\xff $value".as_slice()),
        ] {
            let mut entry = entry(tcl_dialect::TclVersion::V9_0);
            let mut registration = binding(
                &entry.namespaces[0],
                b"renamed\xff",
                NativeCompilerHookPresence::Present,
            );
            registration.compiler =
                Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
                    registry_identity: identity.into(),
                    ensemble: None,
                });
            entry.commands.push(registration.clone());
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("retained original word vector");
            };
            let plan = native_byte_command_plan(words, &entry, &registry, context).unwrap();
            let NativeByteCommandPlan::Registered(plan) = plan else {
                panic!("authenticated original compiler");
            };
            assert_eq!(plan.binding, registration);
            assert_eq!(plan.operand_from, 1);
            assert!(plan.dependencies.is_empty());
            assert!(matches!(
                plan.selection,
                NativeCompilationSelection::Inline { .. }
            ));
            assert!(!closed_generic_byte_compilation(&module, &entry));
            assert!(closed_original_byte_compilation(
                &module, &entry, &registry, context
            ));
            entry.commands[0].compiler = None;
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Registration)
            );
            assert!(!closed_original_byte_compilation(
                &module, &entry, &registry, context
            ));
            entry.commands[0].compiler_hook = NativeCompilerHookPresence::Unknown;
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Registration)
            );
            entry.source_string_protocol = None;
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Entry)
            );
        }
    }

    #[test]
    fn byte_compiler_plan_refuses_unsupported_registered_recipes_and_nested_hooks() {
        let registry = CommandRegistry::build_default();
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        let mut registration = binding(
            &entry.namespaces[0],
            b"worker",
            NativeCompilerHookPresence::Present,
        );
        registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "expr".into(),
            ensemble: None,
        });
        entry.commands.push(registration);
        let module = source_module(b"worker $x", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original words");
        };
        assert_eq!(
            native_byte_command_plan(
                words,
                &entry,
                &registry,
                NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                }
            ),
            Err(NativeByteCommandUnavailable::Recipe)
        );
        assert!(!closed_generic_byte_compilation(
            &source_module(b"opaque\xff $a([worker])", &entry),
            &entry
        ));
        entry.source_string_protocol = None;
        assert!(!closed_generic_byte_compilation(
            &source_module(b"opaque\xff", &entry),
            &entry
        ));
    }

    #[test]
    fn registered_certificate_keeps_nested_original_hook_obligations() {
        let registry = CommandRegistry::build_default();
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        let mut registration = binding(
            &entry.namespaces[0],
            b"set",
            NativeCompilerHookPresence::Present,
        );
        registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "set".into(),
            ensemble: None,
        });
        entry.commands.push(registration);
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"child",
            NativeCompilerHookPresence::Present,
        ));
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            ..Default::default()
        };
        let module = source_module(b"set a\xff([child]) VALUE", &entry);
        assert!(!closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
        assert!(closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        let malformed = source_module(b"set a\xff([child {a}b]) VALUE", &entry);
        assert!(!closed_original_byte_compilation(
            &malformed, &entry, &registry, context
        ));
        entry.source_string_protocol = None;
        assert!(!closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
    }

    #[test]
    fn byte_generic_certificate_uses_search_path_and_retained_namespace_incarnations() {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        entry.namespaces.push(namespace(
            2,
            ByteNamespacePath::from_segments([b"N".as_slice()]),
            false,
        ));
        entry.namespaces.push(namespace(
            3,
            ByteNamespacePath::from_segments([b"P".as_slice()]),
            true,
        ));
        entry.namespaces[1].command_path = vec![3];
        entry.current_namespace = 2;
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"worker\xff",
            NativeCompilerHookPresence::Absent,
        ));
        entry.commands.push(binding(
            &entry.namespaces[2],
            b"worker\xff",
            NativeCompilerHookPresence::Present,
        ));
        let module = source_module(b"worker\xff", &entry);
        assert!(!closed_generic_byte_compilation(&module, &entry));
        entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
        assert!(closed_generic_byte_compilation(&module, &entry));
        let descendant = source_module(b"child::worker\xff", &entry);
        assert!(!closed_generic_byte_compilation(&descendant, &entry));
        entry
            .namespaces
            .push(namespace(4, entry.namespaces[1].path.clone(), true));
        assert!(!closed_generic_byte_compilation(&descendant, &entry));
        entry.namespaces[1].command_path = vec![99];
        assert!(!closed_generic_byte_compilation(&module, &entry));
    }

    #[test]
    fn byte_generic_certificate_checks_every_original_bracket_child() {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"compileMe",
            NativeCompilerHookPresence::Present,
        ));
        for source in [
            b"opaque\xff [compileMe]".as_slice(),
            b"$head [absent [compileMe]]",
            b"[compileMe] ARG",
            b"opaque\xff $array([compileMe])",
        ] {
            assert!(
                !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
                "{source:?}"
            );
        }
        entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
        assert!(closed_generic_byte_compilation(
            &source_module(b"opaque\xff [absent [compileMe]]", &entry),
            &entry
        ));
        assert!(!closed_generic_byte_compilation(
            &source_module(b"{*}$head ARG", &entry),
            &entry
        ));
    }

    #[test]
    fn byte_generic_certificate_checks_bracket_hooks_beyond_advisory_index_depth() {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"compileMe",
            NativeCompilerHookPresence::Present,
        ));
        let mut source = b"opaque\xff ".to_vec();
        for _ in 0..2000 {
            source.extend_from_slice(b"$a(");
        }
        source.extend_from_slice(b"[compileMe]");
        source.extend(std::iter::repeat_n(b')', 2000));
        let module = source_module(&source, &entry);
        assert!(!closed_generic_byte_compilation(&module, &entry));
        entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
        assert!(closed_generic_byte_compilation(&module, &entry));
    }

    #[test]
    fn byte_generic_certificate_never_looks_up_a_dynamic_compiler_head() {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        entry.closed = false;
        for source in [
            b"$head ARG".as_slice(),
            b"prefix${head} ARG",
            b"$array($key) ARG",
        ] {
            assert!(
                closed_generic_byte_compilation(&source_module(source, &entry), &entry),
                "{source:?}"
            );
        }
        assert!(!closed_generic_byte_compilation(
            &source_module(b"$head [literalChild]", &entry),
            &entry
        ));
        entry.closed = true;
        assert!(closed_generic_byte_compilation(
            &source_module(b"[absent] ARG", &entry),
            &entry
        ));
        entry.name_protocol = None;
        assert!(!closed_generic_byte_compilation(
            &source_module(b"$head ARG", &entry),
            &entry
        ));
    }

    #[test]
    fn byte_generic_certificate_keeps_native_constant_head_rules_by_release() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = version.dialect_profile_name();
            let mut entry = entry(version);
            entry.commands.push(binding(
                &entry.namespaces[0],
                b"worker",
                NativeCompilerHookPresence::Present,
            ));
            for source in [b"{worker}".as_slice(), b"\"worker\""] {
                assert!(
                    !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
                    "{dialect} {source:?}"
                );
            }
            let escaped = source_module(b"wor\\ker", &entry);
            assert_eq!(
                closed_generic_byte_compilation(&escaped, &entry),
                dialect == "tcl8.4" || dialect == "tcl8.5",
                "{dialect}"
            );
        }
    }
}
