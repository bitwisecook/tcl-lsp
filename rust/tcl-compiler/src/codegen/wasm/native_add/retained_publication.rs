//! Inert source candidates for a body checked only after real publication and entry.
//! This module issues no Native body, frame, integer or handler execution proof.

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, NativeLowering};
use tcl_syntax::expr::{BinOp, ExprNode};

use crate::compilation_unit::CompilationUnit;
use crate::ir::{CommandTokens, Procedure, ProcedureId, Statement, WordExpr};
use crate::registry_invocation::InvocationMetadataContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetainedI64Publication {
    pub(crate) procedure: ProcedureId,
    pub(crate) span: Span,
    pub(crate) parameters: Vec<u8>,
    pub(crate) body: Vec<u8>,
    pub(crate) guard: tcl_runtime_api::guard::GuardIdentity,
    pub(in crate::codegen::wasm) word_plan: super::super::leaf_invoke::WasmLeafInvokePlan,
}

pub(super) fn candidate(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> Option<RetainedI64Publication> {
    let module = &unit.ir_module;
    let retained = module.retained_source_bindings.as_ref()?;
    retained.positioned_for_module(module, registry)?;
    let mut procedures = module.procedures.iter();
    let (id, procedure) = procedures.next()?;
    if procedures.next().is_some() {
        return None;
    }
    if !module.procedure_ids.owns(*id)
        || !retained.matches_original_procedure_summary(*id, procedure)
        || !source_add_shape(procedure, module.native_lexer_config())
    {
        return None;
    }
    let site = procedure.declaration_site.as_ref()?;
    let mut matching = module.top_level.statements.iter().filter_map(|statement| {
        let Statement::Call {
            span,
            tokens: Some(tokens),
            ..
        } = statement
        else {
            return None;
        };
        (tokens.source_binding.as_ref()?.invocation_site()? == site).then_some((*span, tokens))
    });
    let (span, tokens) = matching.next()?;
    if matching.next().is_some() || tokens.synthetic.is_some() {
        return None;
    }
    site.source
        .original_lexical_command(&tokens.word_exprs, site.offset, module.lexer_config)?;
    let [head, _, parameters, body] = tokens.word_exprs.as_slice() else {
        return None;
    };
    if literal(head)? != "proc"
        || literal(parameters)? != procedure.params_raw
        || literal(body)? != procedure.body_source.as_deref()?
    {
        return None;
    }
    Some(RetainedI64Publication {
        procedure: *id,
        span,
        parameters: literal(parameters)?.as_bytes().to_vec(),
        body: literal(body)?.as_bytes().to_vec(),
        guard: publication_guard(module, registry, tokens)?,
        word_plan: super::super::leaf_invoke::plan_leaf_invocation(
            &tokens.word_exprs,
            module.native_lexer_config(),
        )
        .ok()?,
    })
}

fn literal(word: &WordExpr) -> Option<&str> {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => Some(text),
        _ => None,
    }
}

fn source_add_shape(procedure: &Procedure, config: tcl_lexer::LexerConfig) -> bool {
    let [first, second] = procedure.params.as_slice() else {
        return false;
    };
    if first == second {
        return false;
    }
    let mut names = procedure.params_raw.split_whitespace();
    if names.next() != Some(first.as_str())
        || names.next() != Some(second.as_str())
        || names.next().is_some()
    {
        return false;
    }
    let [
        Statement::Return {
            expr:
                Some(ExprNode::Binary {
                    op: BinOp::Add,
                    left,
                    right,
                }),
            ..
        },
    ] = procedure.body.statements.as_slice()
    else {
        return false;
    };
    matches!((left.as_ref(), right.as_ref()),
        (ExprNode::Var { name: left, text: left_text, .. }, ExprNode::Var { name: right, text: right_text, .. })
        if left == first && right == second
            && plain_scalar(left_text, first, config) && plain_scalar(right_text, second, config))
}

fn plain_scalar(text: &str, expected: &str, config: tcl_lexer::LexerConfig) -> bool {
    matches!(crate::native_lowering::cells::variable_reference_place(text, config),
        Ok(crate::native_lowering::cells::CellPlace::Named { name }) if name == expected)
}

// Same actual global definition guard as the existing guarded native definition path.
// It authorizes only original dispatch; the runtime actual artifact decides the body.
fn publication_guard(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<tcl_runtime_api::guard::GuardIdentity> {
    if tokens.word_exprs.len() != 4 || tokens.word_exprs.iter().any(|word| literal(word).is_none())
    {
        return None;
    }
    let entry = module.source_entry.native_entry.as_deref()?;
    let profile = module.dialect_profile?;
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point?);
    if entry.frame != tcl_runtime_api::native_compilation::NativeCompilationFrame::Global
        || !entry.execution_name_policy.is_some_and(|policy| {
            policy.native_recipe().is_some_and(|recipe| {
                recipe.authority() == tcl_syntax::naming::NamePolicyAuthority::Native
            })
        })
        || entry.invocation_policy != Some(profile.cache_key())
        || !dialect.has_same_execution_policy(tcl_registry::InvocationDialect::of_profile(profile))
        || module.native_lexer_config() != tcl_lexer::LexerConfig::for_profile(Some(profile))
        || !entry
            .namespace_context(entry.current_namespace)
            .ok()?
            .path
            .as_segments()
            .is_empty()
    {
        return None;
    }
    let metadata = InvocationMetadataContext::for_module(registry, module)?;
    (module.original_invocation_source_purpose(registry, metadata)
        == Some(crate::command_binding::OriginalInvocationSourcePurpose::Execution))
    .then_some(())?;
    NativeLowering::Definition.definition_guard_identity(dialect)
}
