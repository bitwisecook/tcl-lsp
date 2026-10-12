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

//! Shared, source-preserving plans for scripts evaluated by runtime invocations.

use crate::ir::{Script, SourceSite};

/// The registry-owned whole-lifecycle iteration topology.
pub use tcl_registry::body_execution::BodyRepetition as RegionRepetition;

/// An explicit prerequisite retained with semantic expansion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionRegionDependency {
    /// Original declaration-frame handler interpretation for analytical phases.
    /// This supplies no entered invocation, native CPP or executable dispatch.
    ConditionalDispatch {
        /// Original scoped source binding, retaining conditional reachability.
        binding: Box<crate::command_binding::SourceInvocationBinding>,
        /// Exact accepted declaration activation layout.
        frame: crate::var_resolve::VariableExecutionFrame,
    },
    /// Conditional lifecycle phase-entry interpretation in its declared frame.
    ConditionalPhaseDispatch {
        /// Registry-owned lifecycle phase.
        phase: crate::command_binding::SourceBodyPhase,
        /// Scoped original phase snapshot; no actual phase entry is granted.
        binding: Box<crate::command_binding::SourceInvocationBinding>,
        /// Original declaration activation retained by the phase owner.
        frame: crate::var_resolve::VariableExecutionFrame,
    },
    /// Every completed scoped declaration traversal omits this phase.
    /// This describes the selected handler condition, not actual execution.
    ConditionalNotEnteredPhase(crate::command_binding::SourceBodyPhase),
    /// Original invocation's live dispatch, argument prefix and activation proof.
    Dispatch(Box<crate::command_binding::SourceInvocationBinding>),
    /// Selected stock loader contract, including lookup and state dependencies.
    Provider(Box<crate::command_binding::TrustedPackageLoader>),
    /// Live wrapper dispatch and activation at a lifecycle phase boundary.
    PhaseDispatch {
        /// Registry-owned lifecycle phase.
        phase: crate::command_binding::SourceBodyPhase,
        /// Shared source-state proof at that phase entry.
        binding: Box<crate::command_binding::SourceInvocationBinding>,
    },
    /// Completed source execution proves this phase unentered at the region's
    /// exact invocation site. An absent phase inventory alone is insufficient.
    NotEnteredPhase(crate::command_binding::SourceBodyPhase),
    /// The live command implements the registry-selected contract.
    Implementation(String),
    /// A version-dependent stock helper is original where present, or absent
    /// only in authored provider alternatives that do not require it.
    OptionalImplementation(String),
    /// An optional wrapper hook is absent in the selected command environment.
    AbsentCommand(String),
    /// A package implementation has the selected versioned semantics.
    Package {
        /// Required package identity.
        name: String,
        /// Verified version or authored stock intersection contract.
        version: String,
    },
}

/// Whether external wrapper selection can omit the entire region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionSelection {
    /// Every invocation satisfying its prerequisites enters the first phase.
    Always,
    /// External configuration may omit every phase.
    MaySkip,
    /// Selection proves that no phase runs.
    Never,
}

/// Destination selected by a phase's Tcl completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionTarget {
    /// Enter a phase in the same region.
    Phase(usize),
    /// Complete the wrapper normally.
    Exit,
    /// Propagate the original completion to the enclosing script.
    Propagate,
}

/// One phase and its completion routing, independent of a backend's inlining.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutionPhase {
    /// Source-accurate lowered script in the invocation's variable frame.
    pub script: Script,
    /// Route after normal Tcl completion.
    pub normal: RegionTarget,
    /// Route after every non-OK Tcl completion.
    pub abrupt: RegionTarget,
}

/// Effects remaining after explicit script phases, separate from their effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapperEffectProjection {
    /// A transparent wrapper contributes no effects after its script.
    Transparent,
    /// Reporting, matching, framework state and extension effects remain opaque.
    /// This conservatism occurs at the join, never before the authored scripts.
    OpaqueResidual,
}

impl WrapperEffectProjection {
    /// Resolve the residual only; never reapply the full invocation footprint.
    #[must_use]
    pub fn resolve(self) -> tcl_registry::world_effect::EffectFootprint {
        match self {
            Self::Transparent => tcl_registry::world_effect::EffectFootprint::default(),
            Self::OpaqueResidual => {
                tcl_registry::world_effect::EffectFootprint::conservative_unknown_invocation()
            }
        }
    }
}

/// Body inventory with a possible entry proof, distinct from executable phases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PossibleBodyRegion {
    /// Shared handler topology; entering any phase remains conditional.
    pub topology: tcl_registry::native_compilation::PossibleBodyTopology,
    /// Effective argument position corresponding to each phase script.
    pub arguments: Vec<usize>,
    /// Unchanged original body sources in the wrapper's own frame. These are
    /// diagnostic advice, not entered scripts or completed body effects.
    /// Each item corresponds to the same phase index as `arguments`.
    pub conditional_sources: Vec<Option<ConditionalBodySource>>,
    /// Source-positioned conditional operands; absent expressions retain May entry.
    pub conditions: Vec<PossibleBodyCondition>,
}

/// Original script text that may be selected in a retained variable frame.
///
/// This receipt supplies lexical ownership for diagnostics. It supplies no
/// execution, normal completion, command identity inside the body, or store
/// authority. A missing receipt means unavailable advice, not an empty body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConditionalBodySource {
    source: crate::command_binding::ExecutedScriptSource,
    frame: crate::var_resolve::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
}

impl ConditionalBodySource {
    pub(crate) fn new(
        source: crate::command_binding::ExecutedScriptSource,
        frame: crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        (!matches!(frame, crate::var_resolve::VariableExecutionFrame::Unknown)
            && matches!(
                source.mapping,
                crate::command_binding::ExecutedScriptMapping::Contiguous { .. }
            ))
        .then_some(Self {
            source,
            frame,
            config,
        })
    }

    /// Exact unchanged source slice, with its original source instance and base.
    #[must_use]
    pub fn source(&self) -> &crate::command_binding::ExecutedScriptSource {
        &self.source
    }

    /// Frame in which the selected handler would interpret this body.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    /// Retained lexical policy; an editor profile cannot replace its grammar.
    #[must_use]
    pub fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// One normal handler test, independent of compiler opcode selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PossibleBodyCondition {
    /// Effective argument containing the native expression value.
    pub argument: usize,
    /// Native expression parsed only from a proven frozen operand value.
    pub expression: Option<crate::expr_ast::ExprNode>,
    /// Original word extent for diagnostic ownership.
    pub source: SourceSite,
    /// Affine source base; transformed values have no original AST offsets.
    pub expression_base: Option<u32>,
}

/// Frozen wrapper inputs and authored dictionary mapping/writeback protocol.
/// Names are resolved at their actual entry or epilogue, never at lowering time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictionaryScopeRegion {
    /// Selected mapping, missing-key and completion contract.
    pub plan: tcl_registry::dictionary_scope::DictionaryScopePlan,
    /// Original evaluation provenance; this snapshot never reevaluates words.
    pub invocation: crate::ir::CommandTokens,
    /// Effective argument bytes frozen at wrapper dispatch; absent values are unknown.
    pub arguments: Vec<Option<String>>,
}

/// A proved caller-frame execution plan; the original invocation remains IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvaluatedBodyRegion {
    /// Original invocation site and transformation provenance.
    pub source: SourceSite,
    /// Prerequisites establishing implementation and wrapper extension facts.
    pub dependencies: Vec<ExecutionRegionDependency>,
    /// External selection of this invocation's scripts.
    pub selection: RegionSelection,
    /// Phases in semantic execution order.
    pub phases: Vec<ExecutionPhase>,
    /// Wrapper completion effects after the explicitly represented phases.
    pub residual_effects: WrapperEffectProjection,
    /// Optional dictionary ingress and completion-sensitive writeback protocol.
    pub scope: Option<Box<DictionaryScopeRegion>>,
    /// Analysis-only possible body inventory; runtime retains the original call.
    pub possible_bodies: Option<Box<PossibleBodyRegion>>,
    /// Iterations repeat phases and residual effects, without rebuilding argv.
    pub repetition: RegionRepetition,
}

impl EvaluatedBodyRegion {
    /// Model a transparent current-frame script without changing runtime dispatch.
    #[must_use]
    pub fn transparent(
        source: SourceSite,
        script: Script,
        dependencies: Vec<ExecutionRegionDependency>,
    ) -> Self {
        Self {
            source,
            dependencies,
            selection: RegionSelection::Always,
            phases: vec![ExecutionPhase {
                script,
                normal: RegionTarget::Exit,
                abrupt: RegionTarget::Propagate,
            }],
            residual_effects: WrapperEffectProjection::Transparent,
            scope: None,
            possible_bodies: None,
            repetition: RegionRepetition::Once,
        }
    }

    /// Model captured setup/body/cleanup using one shared completion graph.
    #[must_use]
    pub fn captured_lifecycle(
        source: SourceSite,
        setup: Script,
        body: Script,
        cleanup: Script,
        selection: RegionSelection,
        dependencies: Vec<ExecutionRegionDependency>,
    ) -> Self {
        Self {
            source,
            dependencies,
            selection,
            phases: vec![
                ExecutionPhase {
                    script: setup,
                    normal: RegionTarget::Phase(1),
                    abrupt: RegionTarget::Phase(2),
                },
                ExecutionPhase {
                    script: body,
                    normal: RegionTarget::Phase(2),
                    abrupt: RegionTarget::Phase(2),
                },
                ExecutionPhase {
                    script: cleanup,
                    normal: RegionTarget::Exit,
                    abrupt: RegionTarget::Exit,
                },
            ],
            residual_effects: WrapperEffectProjection::OpaqueResidual,
            scope: None,
            possible_bodies: None,
            repetition: RegionRepetition::Once,
        }
    }

    /// Surround one caller-frame body with dictionary entry and writeback effects.
    /// Consumers must interpret the retained scope through the shared place owner.
    #[must_use]
    pub fn dictionary_scope(
        source: SourceSite,
        script: Script,
        scope: DictionaryScopeRegion,
        dependencies: Vec<ExecutionRegionDependency>,
    ) -> Self {
        let mut region = Self::transparent(source, script, dependencies);
        region.scope = Some(Box::new(scope));
        region
    }

    /// Reject incomplete or cyclic compatibility plans before interpreting them.
    ///
    /// This is a structural check. The lowering ingress separately proves and
    /// retains every dependency before attaching the plan to command tokens.
    ///
    /// ```
    /// use tcl_compiler::execution_region::{EvaluatedBodyRegion, ExecutionRegionDependency};
    /// use tcl_compiler::ir::{Script, SourceSite};
    /// use tcl_lexer::Span;
    /// let region = EvaluatedBodyRegion::transparent(
    ///     SourceSite::source(Span::new(0, 12)), Script::new(),
    ///     vec![ExecutionRegionDependency::Implementation("selected-contract".into())],
    /// );
    /// assert!(region.valid());
    /// ```
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.dependencies.is_empty()
            && !self.phases.is_empty()
            && self.phases.iter().enumerate().all(|(index, phase)| {
                [phase.normal, phase.abrupt]
                    .iter()
                    .all(|target| match target {
                        RegionTarget::Phase(next) => *next > index && *next < self.phases.len(),
                        RegionTarget::Exit | RegionTarget::Propagate => true,
                    })
            })
    }

    /// Apply the independently proved package iteration contract.
    #[must_use]
    pub fn with_repetition(mut self, repetition: RegionRepetition) -> Self {
        self.repetition = repetition;
        self
    }
}

#[cfg(test)]
pub(crate) fn evaluated_region_test_fixture() -> crate::executable_ir::ExecutableFunction {
    use crate::executable_ir::{ExecutableFunctionId, build_linear_executable_ir};
    use crate::ir::Statement;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;
    use tcl_registry::model::semantic::SemanticContext;
    let registry = CommandRegistry::build_default();
    let mut script = lower_to_ir("tcltest::test n d -body {return answer}", &registry).top_level;
    let region = EvaluatedBodyRegion::captured_lifecycle(
        SourceSite::source(script.statements[0].span()),
        lower_to_ir("set x 1", &registry).top_level,
        lower_to_ir("return answer", &registry).top_level,
        lower_to_ir("unset x", &registry).top_level,
        RegionSelection::MaySkip,
        vec![ExecutionRegionDependency::Implementation(
            "fixture-contract".into(),
        )],
    );
    match &mut script.statements[0] {
        Statement::Call {
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            tokens: Some(tokens),
            ..
        } => *tokens = tokens.clone().with_evaluated_body(region),
        _ => panic!("expected runtime invocation"),
    }
    build_linear_executable_ir(
        &registry,
        Some(SemanticContext::for_environment("tcl8.6")),
        ExecutableFunctionId::new(0),
        &script,
    )
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executable_ir::{ExecutableInstruction, ExecutableTerminator};

    #[test]
    fn executable_graph_captures_phase_return_and_preserves_one_wrapper_boundary() {
        let executable = evaluated_region_test_fixture();
        executable.validate().unwrap();
        let instructions: Vec<_> = executable
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .collect();
        assert_eq!(
            instructions
                .iter()
                .filter(|instruction| matches!(
                    instruction,
                    ExecutableInstruction::CompleteEvaluatedRegion(_)
                ))
                .count(),
            1
        );
        assert!(executable.blocks.iter().any(|block| matches!(
            block.terminator,
            Some(ExecutableTerminator::RegionChoice { .. })
        )));
        assert!(instructions.iter().any(|instruction| matches!(
            instruction,
            ExecutableInstruction::JoinCompletion { .. }
        )));
        assert!(!instructions.iter().any(|instruction| matches!(instruction, ExecutableInstruction::Invoke(invoke) if invoke.original_words.first().is_some_and(|word| matches!(word, crate::ir::WordExpr::Literal { text, .. } if text == "tcltest::test")))));
    }
}
