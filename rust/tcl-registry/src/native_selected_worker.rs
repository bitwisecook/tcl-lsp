// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual installed worker selection from unchanged original compiler words.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationSelection, NativeCompilationSpec,
    NativeNamedInvocationProtocol,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_instruction_plan::{NativeInstructionPlan, NativeNamedInvocationInstruction};
use crate::{CommandRegistry, InvocationDialect};
use tcl_runtime_api::native_compilation::{NativeCompilationBinding, NativeCompilerHookPresence};

/// Native result of the actual selected ensemble worker compiler.
/// This owns no public lookup, map configuration or runtime callable authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalSelectedWorkerCompilation {
    /// The original ensemble invocation is compiled with ordinary dispatch.
    PublicGeneric,
    /// Native compilation retains this original named invocation layout.
    Named(NativeNamedInvocationInstruction),
    /// The actual worker selects an implemented original instruction recipe.
    Operation {
        /// Actual installed registration's descriptor.
        spec: NativeCompilationSpec,
        /// Its independent selection at this original compiler point.
        selection: NativeCompilationSelection,
        /// Its original operand/preparation recipe.
        plan: Box<NativeInstructionPlan>,
    },
    /// A registration, original operand or executable recipe remains unavailable.
    Unavailable,
}

/// Original source and compiler context of an actual selected worker.
#[derive(Clone, Copy)]
pub struct OriginalSelectedWorkerInvocation<'a> {
    /// Complete retained original compiler word vector.
    pub words: &'a NativeCompilerWords<'a>,
    /// First operand after the selected original ensemble members.
    pub operand_from: usize,
    /// Canonical original ensemble members in selection order.
    pub replacements: &'a [Vec<u8>],
    /// Actual native compiler dialect, independently of logical handler policy.
    pub dialect: InvocationDialect,
    /// Original compiler frame and exception/loop environment.
    pub context: NativeCompilationContext,
}

/// One complete original ensemble compiler descent and its retained registrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSelectedWorkerPathCompilation {
    /// Actual leaf compiler result, including a genuine public-dispatch decline.
    pub compilation: OriginalSelectedWorkerCompilation,
    /// Actual final selected node, distinct from its printed invocation name.
    pub worker: NativeCompilationBinding,
    /// First unchanged original operand after all selected ensemble members.
    pub operand_from: usize,
    /// Public and nested compiler registrations reached before operand evaluation.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
}

/// Descend actual nested ensemble compilers through their original map objects.
/// A dynamic nested member makes the original public invocation generic; missing
/// original registrations remain unavailable. The callback grants no handler facts.
#[must_use]
pub fn compile_original_selected_worker_path(
    registry: &CommandRegistry,
    worker: &NativeCompilationBinding,
    invocation: OriginalSelectedWorkerInvocation<'_>,
    mut prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    mut select: impl FnMut(
        &NativeCompilationBinding,
        &tcl_runtime_api::native_compilation::NativeEnsembleCompiler,
        Option<&[u8]>,
        Option<crate::native_compilation::NativeCompilationWordShape>,
    ) -> crate::native_ensemble::NativeEnsembleWorkerSelection,
) -> OriginalSelectedWorkerPathCompilation {
    use crate::native_ensemble::NativeEnsembleWorkerSelection;
    let mut worker = worker.clone();
    let mut operand_from = invocation.operand_from;
    let mut replacements = invocation.replacements.to_vec();
    let compilation = loop {
        if worker.has_execution_trace {
            break OriginalSelectedWorkerCompilation::PublicGeneric;
        }
        let Some(configuration) = worker
            .compiler
            .as_ref()
            .filter(|_| worker.compiler_hook == NativeCompilerHookPresence::Present)
            .and_then(|compiler| compiler.ensemble.as_ref())
        else {
            break compile_original_selected_worker(
                registry,
                &worker,
                OriginalSelectedWorkerInvocation {
                    operand_from,
                    replacements: &replacements,
                    ..invocation
                },
            );
        };
        let Ok(selector) = crate::native_ensemble::original_ensemble_selector_at(
            invocation.words,
            operand_from,
            invocation.dialect,
        ) else {
            break OriginalSelectedWorkerCompilation::Unavailable;
        };
        let selected = select(
            &worker,
            configuration,
            selector.as_ref().and_then(|word| word.literal.as_deref()),
            selector.as_ref().map(|word| word.shape),
        );
        let (member, next) = match selected {
            NativeEnsembleWorkerSelection::Generic => {
                break OriginalSelectedWorkerCompilation::PublicGeneric;
            }
            NativeEnsembleWorkerSelection::Unknown => {
                break OriginalSelectedWorkerCompilation::Unavailable;
            }
            NativeEnsembleWorkerSelection::Worker { member, binding } => (member, binding),
        };
        let Some(nested) = nested_compiler_registration(
            &prerequisite,
            &worker,
            &next,
            replacements.last().map(Vec::as_slice),
        ) else {
            break OriginalSelectedWorkerCompilation::Unavailable;
        };
        prerequisite.nested_compilers.push(nested);
        replacements.push(member.as_bytes().to_vec());
        operand_from += 1;
        worker = *next;
    };
    prerequisite.selected_worker = Some(worker.clone());
    OriginalSelectedWorkerPathCompilation {
        compilation,
        worker,
        operand_from,
        prerequisite,
    }
}

fn nested_compiler_registration(
    public: &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    ensemble: &NativeCompilationBinding,
    selected: &NativeCompilationBinding,
    member: Option<&[u8]>,
) -> Option<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite> {
    let parent = public.nested_compilers.last().unwrap_or(public);
    let configuration = parent.compiler.ensemble.as_ref()?;
    let original = configuration
        .map
        .iter()
        .find(|(name, _)| Some(name.as_bytes()) == member)
        .and_then(|(_, prefix)| match prefix.as_slice() {
            [Some(original)] => Some(original.clone()),
            _ => None,
        })?;
    Some(
        tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
            interpreter: public.interpreter,
            lookup_namespace_token: configuration.namespace_token,
            invocation_word: original,
            slot: ensemble.slot.clone(),
            namespace_token: ensemble.namespace_token,
            token: ensemble.token,
            implementation_generation: ensemble.implementation_generation,
            compiler: ensemble.compiler.clone()?,
            selected_worker: Some(selected.clone()),
            nested_compilers: Vec::new(),
            guard: public.guard,
        },
    )
}

/// Select a retained worker, distinguishing native veto, decline and success.
/// `operand_from` addresses the complete original vector after all selected
/// ensemble members. `replacements` contains those canonical original members.
/// Reported worker names remain literal data and never become lookup operands.
#[must_use]
pub fn compile_original_selected_worker(
    registry: &CommandRegistry,
    worker: &NativeCompilationBinding,
    invocation: OriginalSelectedWorkerInvocation<'_>,
) -> OriginalSelectedWorkerCompilation {
    use OriginalSelectedWorkerCompilation as Result;
    let OriginalSelectedWorkerInvocation {
        words,
        operand_from,
        replacements,
        dialect,
        context,
    } = invocation;
    let Some(arguments_from) = operand_from.checked_sub(1) else {
        return Result::Unavailable;
    };
    if worker.has_execution_trace {
        return Result::PublicGeneric;
    }
    let name = tcl_syntax::naming::native_command_full_name_bytes(&worker.slot);
    let fallback = || match crate::native_ensemble::no_hook_worker_is_named(Some(dialect)) {
        Some(false) => Result::PublicGeneric,
        Some(true) => crate::native_instruction_plan::native_named_invocation_instruction(
            words,
            dialect,
            &name,
            arguments_from,
            NativeNamedInvocationProtocol::EnsembleRewrite,
            replacements,
        )
        .map_or(Result::Unavailable, Result::Named),
        None => Result::Unavailable,
    };
    match worker.compiler_hook {
        NativeCompilerHookPresence::Absent => return fallback(),
        NativeCompilerHookPresence::Unknown => return Result::Unavailable,
        NativeCompilerHookPresence::Present => {}
    }
    let Some(spec) = worker
        .compiler
        .as_ref()
        .filter(|compiler| compiler.ensemble.is_none())
        .and_then(|compiler| {
            registry.native_compilation_for_registration(&compiler.registry_identity, dialect)
        })
    else {
        return Result::Unavailable;
    };
    if matches!(
        spec.grammar,
        crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation { .. }
    ) {
        use crate::native_instruction_plan::NativeNamedWorkerCompilation;
        return match crate::native_instruction_plan::native_named_worker_instruction(
            spec,
            words,
            operand_from,
            dialect,
            &name,
        ) {
            NativeNamedWorkerCompilation::Generic => fallback(),
            NativeNamedWorkerCompilation::Named(recipe) => Result::Named(recipe),
            NativeNamedWorkerCompilation::Unavailable => Result::Unavailable,
        };
    }
    let selection =
        spec.select_registered_worker_native_words(words, operand_from, Some(dialect), context);
    let preparations = match dialect.tcl_version.and_then(|version| {
        crate::native_instruction_plan::original_dictionary_preparations(
            spec,
            words,
            operand_from,
            version,
            context,
        )
    }) {
        Some(Ok(preparations)) => preparations,
        Some(Err(_)) => return Result::Unavailable,
        None => Vec::new(),
    };
    match selection {
        NativeCompilationSelection::Generic => match fallback() {
            Result::Named(mut recipe) => {
                recipe.preparations = preparations;
                Result::Named(recipe)
            }
            Result::PublicGeneric if !preparations.is_empty() => Result::Operation {
                spec,
                selection,
                plan: Box::new(NativeInstructionPlan::GenericPreparation(preparations)),
            },
            result => result,
        },
        NativeCompilationSelection::NamedInvocation { .. } => {
            crate::native_instruction_plan::native_named_invocation_instruction(
                words,
                dialect,
                &name,
                arguments_from,
                NativeNamedInvocationProtocol::Direct,
                &[],
            )
            .map_or(Result::Unavailable, |mut recipe| {
                recipe.preparations = preparations;
                Result::Named(recipe)
            })
        }
        NativeCompilationSelection::Inline { .. } | NativeCompilationSelection::CompileError => {
            crate::native_instruction_plan::native_registered_worker_instruction_plan(
                spec,
                selection,
                words,
                operand_from,
                dialect,
                context,
            )
            .map_or(Result::Unavailable, |plan| Result::Operation {
                spec,
                selection,
                plan: Box::new(plan),
            })
        }
        NativeCompilationSelection::Unknown => Result::Unavailable,
    }
}
