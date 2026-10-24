// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded source proposals for an actual private Native transcript consumer.
//! A plan is correspondence and analysis data; only the owning runtime scope
//! can admit its proposed source. Ordinary optimiser paths receive no grant.

use tcl_registry::{CommandRegistry, SemanticOperationId};
use tcl_runtime_api::closed_native_consumer::{
    ClosedNativeBackend, ClosedNativeConsumerScope, ClosedSourceRefusal, ClosedSourceRewrite,
};

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{CommandTokens, Statement, WordExpr};
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, TclValue};

/// Maximum accepted UTF-8 source bytes.
pub const MAX_CLOSED_SOURCE_BYTES: usize = 256;
/// Existing shared source-walk stack budget required by the owning worker.
pub const MIN_CLOSED_STACK_BYTES: u32 = crate::depth_guard::MIN_SOURCE_WALK_STACK;
/// Maximum top-level commands; each may contain at most one expression invocation.
pub const MAX_CLOSED_COMMANDS: usize = 32;
/// Maximum nodes in each admitted arithmetic tree.
pub const MAX_CLOSED_EXPRESSION_NODES: usize = 64;
/// Maximum recursive arithmetic depth.
pub const MAX_CLOSED_EXPRESSION_DEPTH: usize = 8;
/// Inclusive magnitude bound for every integer leaf and intermediate result.
pub const MAX_CLOSED_INTEGER: i64 = 1_000_000;
/// Maximum complete private stdout transcript bytes.
pub const MAX_CLOSED_OUTPUT_BYTES: usize = 256;

/// Complete checked input and candidate for the deliberately small private domain.
/// Fields cannot be authored by callers, and this type grants no Native entry.
pub struct ClosedSourcePlan {
    source: String,
    rewritten: String,
    profile: tcl_dialect::DialectProfileKey,
    registry: tcl_registry::RegistrySnapshot,
    scalars: Vec<String>,
    commands: Vec<String>,
    stdout: Vec<u8>,
}

impl ClosedSourcePlan {
    /// Exact original source owned by this correspondence plan.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Passive proposed text; it has no ordinary execution grant.
    #[must_use]
    pub fn rewritten_source(&self) -> &str {
        &self.rewritten
    }
    /// Ordinary scalar roots the actual provider must prove initially absent.
    #[must_use]
    pub fn scalar_names(&self) -> &[String] {
        &self.scalars
    }
    /// Canonical original command targets the runtime must independently authenticate.
    #[must_use]
    pub fn command_names(&self) -> &[String] {
        &self.commands
    }
    /// Analysis-only bounded transcript predicted by the admitted integer operations.
    #[must_use]
    pub fn expected_stdout(&self) -> &[u8] {
        &self.stdout
    }
    /// Complete selected metadata policy used to prepare this source.
    #[must_use]
    pub fn profile(&self) -> &tcl_dialect::DialectProfileKey {
        &self.profile
    }

    /// Whether the entire retained command-store semantic key remains current.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry.semantic_key() == registry.snapshot().semantic_key()
    }

    /// Revalidate the complete selected source in the same original metadata.
    /// A candidate cannot introduce new commands/cells or different transcript.
    #[must_use]
    pub fn accepts_candidate(&self, candidate: &str, registry: &CommandRegistry) -> bool {
        if !self.matches_registry(registry)
            || (candidate != self.source && candidate != self.rewritten)
        {
            return false;
        }
        let Ok(candidate) = prepare_closed_source(candidate, registry, self.profile.profile())
        else {
            return false;
        };
        candidate.stdout == self.stdout
            && candidate.scalars == self.scalars
            && candidate
                .commands
                .iter()
                .all(|command| self.commands.contains(command))
    }
}

/// Parse every original command using retained invocation owners. The caller's
/// registry/profile is metadata only; no closure follows from this preparation.
pub fn prepare_closed_source(
    source: &str,
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<ClosedSourcePlan, ClosedSourceRefusal> {
    if source.len() > MAX_CLOSED_SOURCE_BYTES {
        return Err(ClosedSourceRefusal::Budget);
    }
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    if dialect.execution_point().is_none()
        || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    source_preparation_budget(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    )?;
    source_preparation_domain(source, registry, profile)?;
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile);
    let module = &cu.ir_module;
    let input = module
        .source_metadata_input
        .as_ref()
        .ok_or(ClosedSourceRefusal::Source)?;
    let context = crate::registry_invocation::retained_module_metadata_context(registry, module)
        .ok_or(ClosedSourceRefusal::Source)?;
    if !module.procedures.is_empty()
        || module.top_level.statements.is_empty()
        || module.top_level.statements.len() > MAX_CLOSED_COMMANDS
    {
        return Err(ClosedSourceRefusal::Source);
    }
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config());
    if commands.len() != module.top_level.statements.len()
        || commands
            .iter()
            .zip(&module.top_level.statements)
            .any(|(command, statement)| command.is_partial || command.span != statement.span())
    {
        return Err(ClosedSourceRefusal::Source);
    }
    let policy = FoldPolicy::for_profile(
        crate::tcl_expr_eval::leading_zero_is_octal(profile),
        Some(profile),
    )
    .with_invocation_dialect(dialect);
    let mut builder = PlanBuilder {
        source,
        registry,
        profile,
        config: input.lexer_config(),
        context,
        policy,
        env: Env::new(),
        names: Vec::new(),
        commands: Vec::new(),
        stdout: Vec::new(),
        replacements: Vec::new(),
    };
    for statement in &module.top_level.statements {
        builder.statement(&module.top_level, statement)?;
    }
    builder.finish()
}

// Resource geometry only. These shared scanners confer no command or expression
// semantics, and their conservative refusal precedes the full compiler walk.
fn source_preparation_budget(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Result<(), ClosedSourceRefusal> {
    if !tcl_lexer::Lexer::with_config(source, config)
        .braced_word_nesting_within(tcl_core_types::RecursionLimit(8))
        .map_err(|_| ClosedSourceRefusal::Source)?
    {
        return Err(ClosedSourceRefusal::Budget);
    }
    let brackets = tcl_lexer::BracketIndex::build_with_config(source, config);
    let mut depth = 0;
    for &(_, delta) in brackets.events() {
        depth = (depth + delta).max(0);
        if depth > 1 {
            return Err(ClosedSourceRefusal::Budget);
        }
    }
    Ok(())
}

// Resource-only preflight over shared syntax and registry metadata. In particular,
// a small spelling of exponentiation must not reach general CU analysis before
// the bounded domain is known. Success returns no source owner or execution grant;
// the complete retained-source checks below still run independently.
fn source_preparation_domain(
    source: &str,
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<(), ClosedSourceRefusal> {
    use tcl_registry::IntrinsicId;
    use tcl_registry::hooks::LoweringHookId;
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    let policy = FoldPolicy::for_profile(
        crate::tcl_expr_eval::leading_zero_is_octal(profile),
        Some(profile),
    )
    .with_invocation_dialect(dialect);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    if commands.is_empty() || commands.len() > MAX_CLOSED_COMMANDS {
        return Err(ClosedSourceRefusal::Budget);
    }
    let source_map = tcl_lexer::SourceMap::new(source);
    let mut env = Env::new();
    let mut output_bytes = 0;
    for command in &commands {
        if command.is_partial {
            return Err(ClosedSourceRefusal::Source);
        }
        let tokens = CommandTokens::from_segmented(&source_map, config, command);
        match preparation_operation(registry, tokens.words(), dialect)? {
            SemanticOperationId::StructuredLowering(LoweringHookId::Set) => {
                let (name, value) = assignment_words(tokens.words(), policy)?;
                env.insert(name.to_owned(), EnvValue::Str(value.to_owned()));
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite) => {
                let [_, value] = tokens.words() else {
                    return Err(ClosedSourceRefusal::Source);
                };
                let number = match value {
                    WordExpr::Literal { text, .. } => canonical_integer(text, policy)?,
                    WordExpr::Variable { spelling, .. } => {
                        let tree = crate::expr_parser::parse_expr(spelling, Some(profile.name));
                        env_integer(&env, simple_reference(&tree)?, policy)?
                    }
                    WordExpr::CommandSubstitution {
                        spelling,
                        source: site,
                    } => {
                        let calls = crate::word_subst::lifted_calls(Some(&tokens), config);
                        let [call] = calls.as_slice() else {
                            return Err(ClosedSourceRefusal::Source);
                        };
                        if call.span != site.span
                            || source.get(call.span.as_range()) != Some(spelling.as_str())
                        {
                            return Err(ClosedSourceRefusal::Source);
                        }
                        let child = call.tokens.as_ref().ok_or(ClosedSourceRefusal::Source)?;
                        let [
                            WordExpr::Literal { .. },
                            WordExpr::BracedLiteral { text, .. },
                        ] = child.words()
                        else {
                            return Err(ClosedSourceRefusal::Source);
                        };
                        if preparation_operation(registry, child.words(), dialect)?
                            != SemanticOperationId::StructuredLowering(LoweringHookId::Expr)
                        {
                            return Err(ClosedSourceRefusal::Source);
                        }
                        let parser =
                            tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
                        let tcl_registry::runtime_expr_validation::ExpressionPreparationProof::Prepared(prepared) =
                            tcl_registry::runtime_expr_validation::prepare_expression_witness(text, &parser, None)
                        else {
                            return Err(ClosedSourceRefusal::Source);
                        };
                        bounded_arithmetic(prepared.tree(), &env, policy, 0, &mut 0)?
                    }
                    _ => return Err(ClosedSourceRefusal::Source),
                };
                output_bytes += render_number(number, policy)?.len() + 1;
                if output_bytes > MAX_CLOSED_OUTPUT_BYTES {
                    return Err(ClosedSourceRefusal::Budget);
                }
            }
            _ => return Err(ClosedSourceRefusal::Source),
        }
    }
    Ok(())
}

fn preparation_operation(
    registry: &CommandRegistry,
    words: &[WordExpr],
    dialect: tcl_registry::InvocationDialect,
) -> Result<SemanticOperationId, ClosedSourceRefusal> {
    if !matches!(words.first(), Some(WordExpr::Literal { .. })) {
        return Err(ClosedSourceRefusal::Source);
    }
    let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
        crate::registry_invocation::resolve_word_exprs_with_dialect(
            registry,
            None,
            words,
            Some(dialect),
        )
        .map_err(|_| ClosedSourceRefusal::Source)?
    else {
        return Err(ClosedSourceRefusal::Source);
    };
    Ok(facts.operation)
}

fn assignment_words(
    words: &[WordExpr],
    policy: FoldPolicy,
) -> Result<(&str, &str), ClosedSourceRefusal> {
    let [
        _,
        WordExpr::Literal { text: name, .. },
        WordExpr::Literal { text: value, .. },
    ] = words
    else {
        return Err(ClosedSourceRefusal::Source);
    };
    if !plain_scalar(name) {
        return Err(ClosedSourceRefusal::Source);
    }
    canonical_integer(value, policy)?;
    Ok((name, value))
}

struct PlanEdit {
    range: std::ops::Range<usize>,
    value: String,
}

struct PlanOutput {
    number: i64,
    edit: Option<PlanEdit>,
}

struct PlanBuilder<'a> {
    source: &'a str,
    registry: &'a CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    policy: FoldPolicy,
    env: Env,
    names: Vec<String>,
    commands: Vec<String>,
    stdout: Vec<u8>,
    replacements: Vec<PlanEdit>,
}

impl PlanBuilder<'_> {
    fn statement(
        &mut self,
        script: &crate::ir::Script,
        statement: &Statement,
    ) -> Result<(), ClosedSourceRefusal> {
        use tcl_registry::IntrinsicId;
        use tcl_registry::hooks::LoweringHookId;
        if !matches!(
            statement,
            Statement::AssignConst { .. } | Statement::Call { .. }
        ) {
            return Err(ClosedSourceRefusal::Source);
        }
        let tokens = script
            .retained_source_tokens_for_statement(statement)
            .ok_or(ClosedSourceRefusal::Source)?;
        let invocation =
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                self.registry,
                Some(self.context.as_ref().into()),
                tokens,
            )
            .ok_or(ClosedSourceRefusal::Source)?;
        original_target(tokens)?;
        if !matches!(tokens.words().first(), Some(WordExpr::Literal { .. })) {
            return Err(ClosedSourceRefusal::Source);
        }
        self.commands
            .push(invocation.facts.canonical_command.clone());
        match invocation.facts.operation {
            SemanticOperationId::StructuredLowering(LoweringHookId::Set) => {
                self.assignment(tokens.words())
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite) => self.output(tokens),
            _ => Err(ClosedSourceRefusal::Source),
        }
    }

    fn assignment(&mut self, words: &[WordExpr]) -> Result<(), ClosedSourceRefusal> {
        let (name, value) = assignment_words(words, self.policy)?;
        if !self.names.iter().any(|held| held == name) {
            self.names.push(name.to_owned());
        }
        self.env
            .insert(name.to_owned(), EnvValue::Str(value.to_owned()));
        Ok(())
    }

    fn output(&mut self, tokens: &CommandTokens) -> Result<(), ClosedSourceRefusal> {
        let [_, value] = tokens.words() else {
            return Err(ClosedSourceRefusal::Source);
        };
        let output = match value {
            WordExpr::Literal { text, .. } => PlanOutput {
                number: canonical_integer(text, self.policy)?,
                edit: None,
            },
            WordExpr::Variable {
                spelling,
                source: site,
            } => {
                let tree = crate::expr_parser::parse_expr(spelling, Some(self.profile.name));
                let name = simple_reference(&tree)?;
                let number = env_integer(&self.env, name, self.policy)?;
                if self.source.get(site.span.as_range()) != Some(spelling.as_str()) {
                    return Err(ClosedSourceRefusal::Source);
                }
                PlanOutput {
                    number,
                    edit: Some(PlanEdit {
                        range: site.span.as_range(),
                        value: render_number(number, self.policy)?,
                    }),
                }
            }
            WordExpr::CommandSubstitution {
                spelling,
                source: site,
            } => self.expression_output(tokens, spelling, site)?,
            _ => return Err(ClosedSourceRefusal::Source),
        };
        self.stdout
            .extend_from_slice(render_number(output.number, self.policy)?.as_bytes());
        self.stdout.push(b'\n');
        if self.stdout.len() > MAX_CLOSED_OUTPUT_BYTES {
            return Err(ClosedSourceRefusal::Budget);
        }
        if let Some(edit) = output.edit {
            self.replacements.push(edit);
        }
        Ok(())
    }

    fn expression_output(
        &mut self,
        tokens: &CommandTokens,
        spelling: &str,
        site: &crate::ir::SourceSite,
    ) -> Result<PlanOutput, ClosedSourceRefusal> {
        use tcl_registry::hooks::LoweringHookId;
        let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(self.profile);
        parser.lexer_grammar = self.config.grammar_over(parser.lexer_grammar);
        let expressions = crate::word_subst::lifted_source_expressions_with_metadata_context(
            Some(tokens),
            self.registry,
            Some(self.context.as_ref().into()),
            self.config,
            parser,
        );
        let [expression] = expressions.as_slice() else {
            return Err(ClosedSourceRefusal::Source);
        };
        if expression.span.start() != site.span.start()
            || self.source.get(expression.span.as_range()) != Some(spelling)
            || expression.executed_source.is_none()
        {
            return Err(ClosedSourceRefusal::Source);
        }
        let executed = expression
            .executed_source
            .as_ref()
            .ok_or(ClosedSourceRefusal::Source)?;
        if executed.original_expression_mapping().is_none() {
            return Err(ClosedSourceRefusal::Source);
        }
        let tcl_registry::runtime_expr_validation::ExpressionPreparationProof::Prepared(prepared) =
            tcl_registry::runtime_expr_validation::prepare_expression_witness(
                &executed.text,
                &parser,
                None,
            )
        else {
            return Err(ClosedSourceRefusal::Source);
        };
        if prepared.tree() != &expression.expression {
            return Err(ClosedSourceRefusal::Source);
        }
        let calls = crate::word_subst::lifted_calls(Some(tokens), self.config);
        let [call] = calls.as_slice() else {
            return Err(ClosedSourceRefusal::Source);
        };
        let child = call.tokens.as_ref().ok_or(ClosedSourceRefusal::Source)?;
        original_target(child)?;
        let child_invocation =
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                self.registry,
                Some(self.context.as_ref().into()),
                child,
            )
            .ok_or(ClosedSourceRefusal::Source)?;
        if child_invocation.facts.operation
            != SemanticOperationId::StructuredLowering(LoweringHookId::Expr)
            || !matches!(
                child.words(),
                [WordExpr::Literal { .. }, WordExpr::BracedLiteral { .. }]
            )
        {
            return Err(ClosedSourceRefusal::Source);
        }
        self.commands
            .push(child_invocation.facts.canonical_command.clone());
        let mut nodes = 0;
        let number = bounded_arithmetic(prepared.tree(), &self.env, self.policy, 0, &mut nodes)?;
        Ok(PlanOutput {
            number,
            edit: Some(PlanEdit {
                range: expression.span.as_range(),
                value: render_number(number, self.policy)?,
            }),
        })
    }

    fn finish(mut self) -> Result<ClosedSourcePlan, ClosedSourceRefusal> {
        self.names.sort();
        self.commands.sort();
        self.commands.dedup();
        self.replacements.sort_by_key(|edit| edit.range.start);
        if self
            .replacements
            .windows(2)
            .any(|pair| pair[0].range.end > pair[1].range.start)
        {
            return Err(ClosedSourceRefusal::Source);
        }
        let mut rewritten = self.source.to_owned();
        for edit in self.replacements.into_iter().rev() {
            rewritten.replace_range(edit.range, &edit.value);
        }
        Ok(ClosedSourcePlan {
            source: self.source.to_owned(),
            rewritten,
            profile: self.profile.cache_key(),
            registry: self.registry.snapshot(),
            scalars: self.names,
            commands: self.commands,
            stdout: self.stdout,
        })
    }
}

/// This proposal is selected only by the live private owner. No ordinary
/// Optimisation, retained Native operand or source-entry proof is produced.
pub fn optimise_closed_source<'owner, B: ClosedNativeBackend>(
    plan: &ClosedSourcePlan,
    registry: &CommandRegistry,
    scope: &ClosedNativeConsumerScope<'owner, B>,
) -> Result<ClosedSourceRewrite<'owner, B>, ClosedSourceRefusal> {
    if plan.source != scope.source()
        || &plan.profile != scope.profile()
        || !plan.matches_registry(registry)
        || tcl_registry::InvocationDialect::of_profile(plan.profile.profile()).execution_point()
            != Some(scope.execution_point())
    {
        return Err(ClosedSourceRefusal::Currency);
    }
    scope.select(plan.rewritten.clone())
}

fn original_target(tokens: &CommandTokens) -> Result<(), ClosedSourceRefusal> {
    tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_execution_target())
        .filter(|target| target.registry_backed)
        .map(|_| ())
        .ok_or(ClosedSourceRefusal::Source)
}

fn plain_scalar(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'_')
}

fn simple_reference(tree: &ExprNode) -> Result<&str, ClosedSourceRefusal> {
    let ExprNode::Var { text, name, .. } = tree else {
        return Err(ClosedSourceRefusal::Source);
    };
    if !plain_scalar(name) || text != &format!("${name}") {
        return Err(ClosedSourceRefusal::Source);
    }
    Ok(name)
}

fn canonical_integer(text: &str, policy: FoldPolicy) -> Result<i64, ClosedSourceRefusal> {
    use tcl_syntax::number::{Number, ParseFlags};
    let flags = ParseFlags {
        integer_only: true,
        no_whitespace: true,
        no_underscore: true,
        syntax: policy.numbers_or_inferred(),
    };
    let Some(Number::Int(value)) = tcl_syntax::number::parse_whole_with(text, flags) else {
        return Err(ClosedSourceRefusal::Source);
    };
    if tcl_syntax::number::spell_wide(value, policy.spelling()).as_deref() != Some(text) {
        return Err(ClosedSourceRefusal::Source);
    }
    if !(-MAX_CLOSED_INTEGER..=MAX_CLOSED_INTEGER).contains(&value) {
        return Err(ClosedSourceRefusal::Budget);
    }
    Ok(value)
}

fn env_integer(env: &Env, name: &str, policy: FoldPolicy) -> Result<i64, ClosedSourceRefusal> {
    let Some(EnvValue::Str(value)) = env.get(name) else {
        return Err(ClosedSourceRefusal::Source);
    };
    canonical_integer(value, policy)
}

fn bounded_arithmetic(
    tree: &ExprNode,
    env: &Env,
    policy: FoldPolicy,
    depth: usize,
    nodes: &mut usize,
) -> Result<i64, ClosedSourceRefusal> {
    *nodes += 1;
    if depth > MAX_CLOSED_EXPRESSION_DEPTH || *nodes > MAX_CLOSED_EXPRESSION_NODES {
        return Err(ClosedSourceRefusal::Budget);
    }
    match tree {
        ExprNode::Literal { text, .. } => {
            canonical_integer(text, policy)?;
        }
        ExprNode::Var { .. } => {
            env_integer(env, simple_reference(tree)?, policy)?;
        }
        ExprNode::Binary {
            op: BinOp::Add | BinOp::Sub | BinOp::Mul,
            left,
            right,
        } => {
            bounded_arithmetic(left, env, policy, depth + 1, nodes)?;
            bounded_arithmetic(right, env, policy, depth + 1, nodes)?;
        }
        _ => return Err(ClosedSourceRefusal::Source),
    }
    // Analysis records conversion obligations; it is deliberately not an
    // executable fold. Only the consuming private scope can hide those effects.
    let evaluation = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
        tree,
        env,
        policy,
        &|_, _| None,
        None,
    )
    .ok_or(ClosedSourceRefusal::Source)?;
    let TclValue::Int(value) = evaluation.value else {
        return Err(ClosedSourceRefusal::Source);
    };
    if !(-MAX_CLOSED_INTEGER..=MAX_CLOSED_INTEGER).contains(&value) {
        return Err(ClosedSourceRefusal::Budget);
    }
    Ok(value)
}

fn render_number(value: i64, policy: FoldPolicy) -> Result<String, ClosedSourceRefusal> {
    let text = crate::tcl_expr_eval::format_tcl_value_with_policy(&TclValue::Int(value), policy)
        .ok_or(ClosedSourceRefusal::Source)?;
    if canonical_integer(&text, policy)? != value {
        return Err(ClosedSourceRefusal::Source);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "set x 1\nputs [expr {$x + 1}]\nputs $x\n";

    fn profile() -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile()
    }

    #[test]
    fn closed_plan_remains_correspondence_without_a_consumer() {
        let registry = CommandRegistry::build_default();
        let plan = prepare_closed_source(SOURCE, &registry, profile()).unwrap();
        assert_eq!(plan.source(), SOURCE);
        assert_eq!(plan.rewritten_source(), "set x 1\nputs 2\nputs 1\n");
        assert_eq!(plan.expected_stdout(), b"2\n1\n");
        assert!(plan.accepts_candidate(plan.rewritten_source(), &registry));
        assert!(!plan.accepts_candidate("set x 1\nputs 3\nputs 1\n", &registry));
        // A plan exposes no scope constructor or ordinary Optimisation.
        let open =
            crate::optimiser::manager::optimise_with_dialect(SOURCE, &registry, Some(profile()));
        let rewritten = crate::optimiser::manager::apply_optimisations(SOURCE, &open);
        assert!(rewritten.contains("[expr {$x + 1}]"));
    }

    #[test]
    fn closed_plan_refuses_unbounded_and_unclassified_expression_effects() {
        let registry = CommandRegistry::build_default();
        for source in [
            "set x 1\nputs [expr {$x + [incr x]}]\n",
            "set x 1\nputs [expr {rand()}]\n",
            "set x 1\nputs [expr {$x + 1 garbage}]\n",
            "set x 1\nputs [expr {$x + 1; puts 9}]\n",
            "set x 1\nputs [expr {1.0 + $x}]\n",
            "set x 1\nputs [expr {$x / 0}]\n",
            "set x 1\nputs [expr {2 ** 1000000}]\n",
            "set x 1000000\nputs [expr {$x * $x}]\n",
            "set x 01\nputs $x\n",
            "set x 1\nputs [expr {$x + 1}\n",
            "set x 1\nputs {1}suffix\n",
            "set x 1\nunknown_call\nputs $x\n",
            "set x 1\nset command puts\n$command $x\n",
            "set x 1\nrename puts saved_puts\nputs $x\n",
            "set x 1\ntrace add variable x read observe\nputs $x\n",
            "set x 1\nchan push stdout observe\nputs $x\n",
            "set x 1\nreturn $x\n",
        ] {
            assert!(
                prepare_closed_source(source, &registry, profile()).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn preparation_rejects_large_work_before_full_analysis() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            source_preparation_domain(SOURCE, &registry, profile()),
            Ok(())
        );
        for source in [
            "puts [expr {2 ** 1000000000}]",
            "puts [expr {1 << 1000000000}]",
            "puts [expr {sqrt(1)}]",
            "puts [expr {[unknown_call]}]",
            "unknown_call",
        ] {
            assert_eq!(
                source_preparation_domain(source, &registry, profile()),
                Err(ClosedSourceRefusal::Source),
                "{source}"
            );
        }
        assert_eq!(
            source_preparation_domain("puts [expr {1000000 * 1000000}]", &registry, profile()),
            Err(ClosedSourceRefusal::Budget)
        );
    }

    #[test]
    fn closed_plan_bounds_source_before_compilation_unit_preparation() {
        let registry = CommandRegistry::build_default();
        for source in [
            format!("{}1{}", "[".repeat(1024), "]".repeat(1024)),
            format!("puts {}1{}", "{".repeat(10), "}".repeat(10)),
            "puts [set x [set y 1]]".to_owned(),
        ] {
            assert!(matches!(
                prepare_closed_source(&source, &registry, profile()),
                Err(ClosedSourceRefusal::Budget)
            ));
        }
        assert!(matches!(
            prepare_closed_source(
                &"x".repeat(MAX_CLOSED_SOURCE_BYTES + 1),
                &registry,
                profile()
            ),
            Err(ClosedSourceRefusal::Budget)
        ));
    }

    #[test]
    fn closed_plan_retains_the_exact_source_and_registry_semantics() {
        let mut registry = CommandRegistry::build_default();
        let plan = prepare_closed_source(SOURCE, &registry, profile()).unwrap();
        assert!(!plan.accepts_candidate("set x 1\nputs [expr {$x + 2}]\nputs $x\n", &registry));
        registry.insert(tcl_registry::CommandSpec {
            name: "closed_consumer_new_command",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        assert!(!plan.matches_registry(&registry));
        assert!(!plan.accepts_candidate(plan.rewritten_source(), &registry));
    }
}
