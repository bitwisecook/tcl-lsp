// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execution ordering for registry-resolved script operands.

use crate::hooks::LoweringHookId;
use crate::{ArgRole, InvocationFacts, SemanticOperationId, Traits};

/// Closed ordering knowledge for an invocation's immediate script operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptBodyFlow {
    /// The command retains its bodies for later invocation.
    Deferred,
    /// No script operand executes immediately.
    None,
    /// Expression operands evaluate in order, including lazy substitutions.
    Expressions(Vec<usize>),
    /// All remaining argv operands form one concatenated expression value.
    ConcatenatedExpression {
        /// Effective argv position of the first expression operand.
        argument_offset: usize,
    },
    /// Listed operands run in order in the selected execution frame.
    Sequence(Vec<usize>),
    /// Remaining script arguments are concatenated before a single evaluation.
    ConcatenatedScript {
        /// First script argument after dispatcher/level selection.
        argument_offset: usize,
    },
    /// Execute operands and catch their Tcl completions; process exit escapes.
    Capture(crate::catch_invocation::CatchInvocationSelection),
    /// Try handlers select captured completions and finally preserves or replaces them.
    Try(Option<crate::TryControlInvocation>),
    /// At most one listed body runs; a path may select no body.
    Alternatives(Vec<usize>),
    /// Descriptor-selected direct and decoded list-element case bodies.
    CaseBodies(crate::case_bodies::CaseBodyOperands),
    /// Ordered condition/body pairs. A later condition is reached only when
    /// earlier conditions select no body; `None` denotes the else body.
    Conditional(Vec<(Option<usize>, usize)>),
    /// Initial scripts run once, then condition evaluation and repeated scripts.
    Loop {
        /// Initial scripts in execution order.
        initial: Vec<usize>,
        /// Expression operands evaluated before every iteration.
        conditions: Vec<usize>,
        /// Repeated scripts in execution order.
        repeated: Vec<usize>,
        /// Scripts reached when the loop body completes with `continue`.
        continued: Vec<usize>,
    },
    /// An anonymous procedure selected from the native lambda-list grammar.
    Lambda(crate::lambda_invocation::LambdaInvocationSelection),
    /// Dictionary entry mapping and completion-sensitive writeback surround a body.
    DictionaryScope(crate::dictionary_scope::DictionaryScopeSelection),
    /// The separately authored lifecycle descriptor owns phase ordering.
    CapturedLifecycle,
    /// Body roles exist but their execution ordering is not established.
    Unknown(Vec<usize>),
}

/// Select ordering from proved semantic operation and authored traits, never
/// from source command spelling or the lexical order of body annotations.
#[must_use]
pub fn script_body_flow(facts: &InvocationFacts) -> ScriptBodyFlow {
    let offset = facts.argument_offset;
    let bodies = facts
        .arg_roles
        .iter()
        .filter_map(|(index, role)| {
            (*role == ArgRole::Body).then_some(offset + usize::from(*index))
        })
        .collect::<Vec<_>>();
    if facts.traits.contains(Traits::DEFERS_BODY) {
        return ScriptBodyFlow::Deferred;
    }
    match facts.body_execution {
        Some(crate::body_execution::BodyExecutionSpec::CapturedLifecycle(_)) => {
            return ScriptBodyFlow::CapturedLifecycle;
        }
        Some(crate::body_execution::BodyExecutionSpec::CallerFrameSequence) => {
            return ScriptBodyFlow::Sequence(bodies);
        }
        Some(crate::body_execution::BodyExecutionSpec::DictionaryScope(_)) => {
            return ScriptBodyFlow::DictionaryScope(
                crate::dictionary_scope::DictionaryScopeSelection::Unknown,
            );
        }
        Some(crate::body_execution::BodyExecutionSpec::DeferredGlobalScript) => {
            return ScriptBodyFlow::Deferred;
        }
        Some(
            crate::body_execution::BodyExecutionSpec::SourceFile
            | crate::body_execution::BodyExecutionSpec::ArrayIteration
            | crate::body_execution::BodyExecutionSpec::SubstitutionTemplate,
        )
        | None => {}
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Apply) {
        return ScriptBodyFlow::Lambda(
            crate::lambda_invocation::LambdaInvocationSelection::Unknown,
        );
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Expr) {
        return ScriptBodyFlow::ConcatenatedExpression {
            argument_offset: offset,
        };
    }
    if bodies.is_empty() {
        let expressions = facts
            .arg_roles
            .iter()
            .filter_map(|(index, role)| {
                (*role == ArgRole::Expr).then_some(offset + usize::from(*index))
            })
            .collect::<Vec<_>>();
        return if expressions.is_empty() {
            ScriptBodyFlow::None
        } else {
            ScriptBodyFlow::Expressions(expressions)
        };
    }
    match facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::For) => ScriptBodyFlow::Loop {
            initial: vec![offset],
            conditions: vec![offset + 1],
            repeated: vec![offset + 3, offset + 2],
            continued: vec![offset + 2],
        },
        SemanticOperationId::StructuredLowering(LoweringHookId::While) => ScriptBodyFlow::Loop {
            initial: Vec::new(),
            conditions: vec![offset],
            repeated: vec![offset + 1],
            continued: Vec::new(),
        },
        SemanticOperationId::StructuredLowering(LoweringHookId::If) => conditional_flow(facts),
        SemanticOperationId::StructuredLowering(LoweringHookId::Switch) => {
            ScriptBodyFlow::Alternatives(bodies)
        }
        SemanticOperationId::StructuredLowering(
            LoweringHookId::NamespaceEval | LoweringHookId::Eval | LoweringHookId::Uplevel,
        ) => ScriptBodyFlow::ConcatenatedScript {
            argument_offset: bodies[0],
        },
        SemanticOperationId::StructuredLowering(LoweringHookId::Catch) => {
            ScriptBodyFlow::Capture(crate::catch_invocation::CatchInvocationSelection::Unknown)
        }
        _ if facts.traits.contains(Traits::HAS_LOOP_BODY) => ScriptBodyFlow::Loop {
            initial: Vec::new(),
            conditions: Vec::new(),
            repeated: bodies,
            continued: Vec::new(),
        },
        _ => ScriptBodyFlow::Unknown(bodies),
    }
}

fn conditional_flow(facts: &InvocationFacts) -> ScriptBodyFlow {
    let mut roles = facts.arg_roles.clone();
    roles.sort_by_key(|(index, _)| *index);
    let mut condition = None;
    let mut branches = Vec::new();
    for (index, role) in roles {
        match role {
            ArgRole::Expr => condition = Some(facts.argument_offset + usize::from(index)),
            ArgRole::Body => {
                branches.push((condition.take(), facts.argument_offset + usize::from(index)));
            }
            _ => {}
        }
    }
    ScriptBodyFlow::Conditional(branches)
}

/// Select value-sensitive execution topology using the same native argument
/// grammar as lowering and runtime capture. Unknown layouts retain unknown.
#[must_use]
pub fn script_body_flow_for_invocation(
    facts: &InvocationFacts,
    arguments: crate::InvocationArguments<'_>,
) -> ScriptBodyFlow {
    if facts.body_execution == Some(crate::body_execution::BodyExecutionSpec::ArrayIteration) {
        return match crate::array_iteration::select(arguments, facts.argument_offset) {
            crate::array_iteration::ArrayIterationSelection::Valid(plan) => ScriptBodyFlow::Loop {
                initial: Vec::new(),
                conditions: Vec::new(),
                repeated: vec![plan.body_at],
                continued: Vec::new(),
            },
            crate::array_iteration::ArrayIterationSelection::Invalid => ScriptBodyFlow::None,
            crate::array_iteration::ArrayIterationSelection::Unknown => ScriptBodyFlow::Unknown(
                facts
                    .arg_roles
                    .iter()
                    .filter_map(|&(index, role)| {
                        (role == ArgRole::Body)
                            .then_some(facts.argument_offset + usize::from(index))
                    })
                    .collect(),
            ),
        };
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Apply) {
        return ScriptBodyFlow::Lambda(crate::lambda_invocation::select_lambda_invocation(
            arguments,
            facts.argument_offset,
        ));
    }
    if let Some(crate::body_execution::BodyExecutionSpec::DictionaryScope(scope)) =
        facts.body_execution
    {
        return ScriptBodyFlow::DictionaryScope(scope.select(arguments, facts.argument_offset));
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Try) {
        return ScriptBodyFlow::Try(crate::selected_try_control_invocation(
            arguments,
            facts.argument_offset,
        ));
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Catch) {
        return ScriptBodyFlow::Capture(arguments.dialect().map_or(
            crate::catch_invocation::CatchInvocationSelection::Unknown,
            |dialect| crate::catch_invocation::select_catch_invocation(arguments, dialect),
        ));
    }
    script_body_flow(facts)
}
