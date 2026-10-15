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

//! BPF-profile-pinned Tcl source lowering.

use tcl_compiler::ir::{IfClause, WordExpr};
use tcl_compiler::lowering::{lower_to_ir_with_dialect, source_nesting_limit};
use tcl_compiler::{Module, Script, Statement, parse_expr_for_profile, rebase_script};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::registry::CommandRegistry;

mod phase;
pub(crate) use phase::{BpfPreparedFunction, BpfSourceScript};

/// Original Document source for one DSL script. Child regions retain the full
/// image and configuration; their IR and diagnostics use region-local spans.
#[derive(Debug, Clone)]
pub struct OriginalBpfSource {
    image: SourceImage,
    config: LexerConfig,
    region: Span,
    purpose: tcl_lexer::SourceSegmentationPurpose,
    registry: Option<tcl_registry::RegistrySnapshot>,
    module_context: Option<tcl_compiler::ir::SourceBpfModuleContext>,
}

impl OriginalBpfSource {
    /// Retain the actual source and complete configuration of a lowered module.
    #[must_use]
    pub fn from_module(module: &Module) -> Self {
        Self {
            image: module.source.clone(),
            config: module.native_lexer_config(),
            registry: module.registry_snapshot.clone(),
            module_context: module
                .registry_snapshot
                .as_ref()
                .and_then(|snapshot| module.source_bpf_module_context(snapshot.registry())),
            purpose: tcl_lexer::SourceSegmentationPurpose::WholeInput,
            region: Span::new(
                0,
                u32::try_from(module.source.bytes().len()).expect("source span fits"),
            ),
        }
    }

    fn from_body(
        word: &NativeWord,
        registry: Option<tcl_registry::RegistrySnapshot>,
        module_context: Option<tcl_compiler::ir::SourceBpfModuleContext>,
    ) -> Option<Self> {
        let value = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
        let region = word.content_span().ok()?;
        // A decoded or concatenated value cannot acquire affine source geometry.
        (word.image().bytes().get(region.as_range())? == value.as_slice()).then_some(Self {
            image: word.image().clone(),
            config: word.config(),
            registry,
            module_context,
            region,
            purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
        })
    }

    pub(crate) fn base(&self) -> u32 {
        self.region.start()
    }

    fn text(&self) -> Option<&str> {
        std::str::from_utf8(self.image.bytes().get(self.region.as_range())?).ok()
    }

    fn command_words(
        &self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        args: &[String],
    ) -> Option<Vec<NativeWord>> {
        tokens?.original_configured_command_words(
            self.image.clone(),
            self.region,
            self.config,
            self.purpose,
            args,
        )
    }

    pub(crate) fn argument(
        &self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        args: &[String],
        index: usize,
    ) -> Option<NativeWord> {
        self.command_words(tokens, args)?
            .into_iter()
            .nth(index.checked_add(1)?)
    }

    /// Select closed DSL metadata only after authenticating the entire original
    /// command. Unavailable source is an error, independently of whether its
    /// reported head would select any descriptor.
    pub(crate) fn command_spec(
        &self,
        statement: &Statement,
        registry: &CommandRegistry,
        missing: crate::diag::BpfDiag,
    ) -> Result<Option<&'static tcl_registry::CommandSpec>, crate::diag::BpfError> {
        Ok(self
            .original_head(statement, registry, missing)?
            .and_then(|head| registry.get_for_surface(&head, registry.own_surface_query())))
    }

    pub(crate) fn original_head(
        &self,
        statement: &Statement,
        registry: &CommandRegistry,
        missing: crate::diag::BpfDiag,
    ) -> Result<Option<String>, crate::diag::BpfError> {
        self.check_registry(registry, missing, statement.span())?;
        let (args, tokens) = match statement {
            Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } => {
                (args, tokens)
            }
            _ => return Ok(None),
        };
        let words = self.command_words(tokens.as_ref(), args).ok_or_else(|| {
            crate::diag::BpfError::new(
                missing,
                statement.span(),
                "DSL command requires unchanged original words",
            )
        })?;
        let head = words
            .first()
            .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
            .and_then(|value| String::from_utf8(value).ok())
            .ok_or_else(|| {
                crate::diag::BpfError::new(
                    crate::diag::BpfDiag::OutOfSubset,
                    statement.span(),
                    "DSL command head must be an original static scalar",
                )
            })?;
        Ok(Some(head))
    }

    pub(crate) fn declaration_kind(
        &self,
        statement: &Statement,
        registry: &CommandRegistry,
        missing: crate::diag::BpfDiag,
    ) -> Result<Option<tcl_registry::bpf_op::BpfDeclKind>, crate::diag::BpfError> {
        Ok(
            match self
                .command_spec(statement, registry, missing)?
                .and_then(|spec| spec.bpf_op)
                .map(|op| op.kind)
            {
                Some(tcl_registry::bpf_op::BpfOpKind::Framework(kind)) => Some(kind),
                _ => None,
            },
        )
    }

    /// Prove that a complete original region is represented before deriving a
    /// default or absence. Structural fields retain their separate lowering
    /// obligations; this witness only covers command inventory and source shape.
    pub(crate) fn validate_script(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        missing: crate::diag::BpfDiag,
    ) -> Result<(), crate::diag::BpfError> {
        self.check_registry(registry, missing, Span::empty(0))?;
        let failure = || {
            crate::diag::BpfError::new(
                missing,
                script
                    .statements
                    .first()
                    .map_or(Span::empty(0), Statement::span),
                "DSL script requires its complete original command inventory",
            )
        };
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.image.clone(),
            self.region,
            self.config,
            self.purpose,
        )
        .map_err(|_| failure())?;
        if plan.fatal_tail.is_some() {
            return Err(failure());
        }
        let original = self.lower(registry).top_level;
        if original.statements.len() != plan.commands.len()
            || script.statements.len() != original.statements.len()
        {
            return Err(failure());
        }
        for ((statement, expected), command) in script
            .statements
            .iter()
            .zip(&original.statements)
            .zip(&plan.commands)
        {
            if statement.span() != expected.span()
                || std::mem::discriminant(statement) != std::mem::discriminant(expected)
                || self.base().checked_add(expected.span().start()) != Some(command.span.start())
            {
                return Err(failure());
            }
            command
                .words
                .first()
                .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                .ok_or_else(|| {
                    crate::diag::BpfError::new(
                        crate::diag::BpfDiag::OutOfSubset,
                        statement.span(),
                        "DSL command head must be an original static scalar",
                    )
                })?;
            self.command_spec(statement, registry, missing)?;
        }
        Ok(())
    }

    /// A closed DSL scalar from the authenticated whole command's actual word.
    /// This is Unicode source data, independent of affine script-body geometry.
    pub(crate) fn scalar(
        &self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        args: &[String],
        index: usize,
    ) -> Option<String> {
        let word = self.argument(tokens, args, index)?;
        String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            &word,
        )?)
        .ok()
    }

    pub(crate) fn body(
        &self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        args: &[String],
        index: usize,
    ) -> Option<Self> {
        Self::from_body(
            &self.argument(tokens, args, index)?,
            self.registry.clone(),
            self.module_context.clone(),
        )
    }

    pub(crate) fn selected_body(
        &self,
        statement: &Statement,
        index: usize,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let words = selected_original_words(statement, registry).ok()??;
        let word = words.get(index.checked_add(1)?)?;
        // A retained command from a foreign source cannot borrow this input's
        // module context merely because the catalogue selection is current.
        (word.image() == &self.image).then_some(())?;
        Self::from_body(word, self.registry.clone(), self.module_context.clone())
    }

    fn check_registry(
        &self,
        registry: &CommandRegistry,
        missing: crate::diag::BpfDiag,
        span: Span,
    ) -> Result<(), crate::diag::BpfError> {
        if self.module_context.is_some()
            && self.registry.as_ref().is_some_and(|original| {
                original.semantic_key() == registry.snapshot().semantic_key()
            })
        {
            Ok(())
        } else {
            Err(crate::diag::BpfError::new(
                missing,
                span,
                "DSL source requires its original registry context",
            ))
        }
    }

    pub(crate) fn prepared_body(
        &self,
        registry: &CommandRegistry,
    ) -> Result<Script, crate::diag::BpfError> {
        phase::prepared_body(self, registry)
    }

    pub(crate) fn lower(&self, registry: &CommandRegistry) -> Module {
        let text = self.text().expect("checked original Unicode body");
        let mut module =
            lower_bpf_source_with_config(text, registry, self.purpose.config(self.config));
        discard_execution_proofs(&mut module.top_level);
        module
    }
}

/// The actual selected row for the unchanged command vector. An authenticated
/// unknown catalogue head is distinct from unavailable naming representation.
pub(crate) fn selected_command(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Result<Option<&'static tcl_registry::CommandSpec>, crate::diag::BpfError> {
    let (args, tokens) = selected_tokens(statement)?;
    tokens
        .source_bpf_command
        .as_ref()
        .and_then(|binding| binding.descriptor(tokens, args, registry))
        .ok_or_else(|| missing_selection(statement.span()))
}

pub(crate) fn selected_head<'a>(
    statement: &'a Statement,
    registry: &CommandRegistry,
) -> Result<&'a str, crate::diag::BpfError> {
    let (args, tokens) = selected_tokens(statement)?;
    tokens
        .source_bpf_command
        .as_ref()
        .and_then(|binding| binding.head(tokens, args, registry))
        .ok_or_else(|| missing_selection(statement.span()))
}

fn selected_tokens(
    statement: &Statement,
) -> Result<(&[String], &tcl_compiler::ir::CommandTokens), crate::diag::BpfError> {
    match statement {
        Statement::Call {
            args,
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            args,
            tokens: Some(tokens),
            ..
        } => Ok((args, tokens)),
        _ => Err(missing_selection(statement.span())),
    }
}

pub(crate) fn selected_original_words<'a>(
    statement: &'a Statement,
    registry: &CommandRegistry,
) -> Result<Option<&'a [NativeWord]>, crate::diag::BpfError> {
    let (args, tokens) = selected_tokens(statement)?;
    let binding = tokens
        .source_bpf_command
        .as_ref()
        .ok_or_else(|| missing_selection(statement.span()))?;
    let _ = binding
        .descriptor(tokens, args, registry)
        .ok_or_else(|| missing_selection(statement.span()))?;
    Ok(binding.original_words(tokens, args, registry))
}

pub(crate) fn selected_argument(
    statement: &Statement,
    index: usize,
    registry: &CommandRegistry,
) -> Result<Option<String>, crate::diag::BpfError> {
    let (args, _) = selected_tokens(statement)?;
    Ok(match selected_original_words(statement, registry)? {
        None => args.get(index).cloned(),
        Some(words) => index
            .checked_add(1)
            .and_then(|index| words.get(index))
            .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
            .and_then(|bytes| String::from_utf8(bytes).ok()),
    })
}

/// Original operands are decoded by the shared word owner. Generated operands
/// are already immutable DSL data from the selected typed operation producer.
pub(crate) fn selected_arguments(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Result<Vec<String>, crate::diag::BpfError> {
    let (args, _) = selected_tokens(statement)?;
    match selected_original_words(statement, registry)? {
        None => Ok(args.to_vec()),
        Some(words) => words
            .iter()
            .skip(1)
            .map(|word| {
                tcl_syntax::word_rules::original_static_word_unicode_value(word)
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .ok_or_else(|| {
                        crate::diag::BpfError::new(
                            crate::diag::BpfDiag::OutOfSubset,
                            statement.span(),
                            "DSL operand requires its original static scalar",
                        )
                    })
            })
            .collect(),
    }
}

fn missing_selection(span: Span) -> crate::diag::BpfError {
    crate::diag::BpfError::new(
        crate::diag::BpfDiag::OutOfSubset,
        span,
        "DSL command requires its retained original or typed generated selection",
    )
}

/// Rejection-only compatibility display for unowned CFG diagnostics. This
/// projection cannot issue an executable BPF program or a source binding.
pub(crate) fn source_command_head(statement: &Statement) -> Option<&str> {
    let (command, tokens) = match statement {
        Statement::Call {
            command, tokens, ..
        }
        | Statement::Barrier {
            command, tokens, ..
        } => (command, tokens),
        _ => return None,
    };
    let Some(tokens) = tokens else {
        return Some(command);
    };
    if tokens
        .expand_word
        .as_ref()
        .is_some_and(|words| words.first() == Some(&true))
    {
        return None;
    }
    match tokens.word_exprs.first()? {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => Some(text),
        _ => None,
    }
}

pub(crate) fn source_command_spec(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Option<&'static tcl_registry::CommandSpec> {
    registry.get_for_surface(
        source_command_head(statement)?,
        registry.own_surface_query(),
    )
}

pub(crate) fn source_bpf_operation(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Option<&'static tcl_registry::bpf_op::BpfOpSpec> {
    source_command_spec(statement, registry)?.bpf_op
}

pub(crate) fn synthetic_operation_call(
    operation: tcl_registry::bpf_op::BpfOpKind,
    args: Vec<String>,
    span: Span,
    registry: &CommandRegistry,
) -> Result<Statement, crate::diag::BpfError> {
    let command = registry.bpf_command_for(operation).ok_or_else(|| {
        crate::diag::BpfError::new(
            crate::diag::BpfDiag::OutOfSubset,
            span,
            format!("Registry has no BPF operation {operation:?}"),
        )
    })?;
    let tokens =
        tcl_compiler::ir::CommandTokens::generated_bpf_operation(operation, &args, span, registry)
            .ok_or_else(|| missing_selection(span))?;
    Ok(Statement::Call {
        span,
        command: command.to_owned(),
        canonical_command: None,
        args,
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    })
}

/// Lower BPF Tcl source under the BPF profile for every top-level and nested
/// body re-segmentation.
pub(crate) fn lower_bpf_source(source: &str, registry: &CommandRegistry) -> Module {
    lower_bpf_source_with_config(
        source,
        registry,
        LexerConfig::for_profile(registry.profile()),
    )
}

fn lower_bpf_source_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Module {
    let mut module = lower_to_ir_with_dialect(
        source,
        registry,
        // The same profile the lowering is given, so the re-segmentation
        // grammar and the dialect axis can never disagree.
        config,
        // The BPF profile as the registry itself resolved it — this crate
        // deliberately has no compile-time path to `tcl-dialect`, and reaches a
        // profile only through `CommandRegistry::profile()`.
        registry.profile(),
    );
    let config = module.native_lexer_config();
    expand_conditionals(&mut module.top_level, source, registry, config);
    module
}

/// BPF is a closed static language: unknown commands, command factories and
/// replacements are rejected by its frontend. Its authored control syntax is
/// consequently selected independently of Tcl's live execution admission.
fn expand_conditionals(
    root: &mut Script,
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) {
    let mut pending = vec![(root, 0)];
    while let Some((script, depth)) = pending.pop() {
        if source_nesting_limit().exceeded(depth) {
            continue;
        }
        for statement in &mut script.statements {
            if let Some(conditional) = conditional_statement(statement, source, registry, config) {
                *statement = conditional;
            }
            pending.extend(
                statement
                    .child_scripts_mut()
                    .into_iter()
                    .map(|body| (body, depth + 1)),
            );
        }
    }
}

fn conditional_statement(
    statement: &Statement,
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<Statement> {
    let (args, span, tokens) = match statement {
        Statement::Call {
            args,
            span,
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            args,
            span,
            tokens: Some(tokens),
            ..
        } => (args, span, tokens),
        _ => return None,
    };
    let WordExpr::Literal { text: head, .. } = tokens.word_exprs.first()? else {
        return None;
    };
    if tokens.word_exprs.len() != args.len() + 1
        || tokens
            .expand_word
            .as_ref()
            .is_some_and(|words| words.iter().any(|expanded| *expanded))
    {
        return None;
    }
    let words = args.iter().map(String::as_str).collect::<Vec<_>>();
    let plan = registry.bpf_conditional_operands(head, &words)?;
    // Require every original word to be literal, including the noise words.
    for (word, value) in tokens.word_exprs[1..].iter().zip(args) {
        original_literal(word, value, source)?;
    }
    let mut clauses = Vec::new();
    let mut else_body = None;
    let mut else_span = None;
    for (condition, body_index) in plan {
        let body_word = tokens.word_exprs.get(body_index + 1)?;
        let (body_text, body_base, body_span) =
            original_literal(body_word, &args[body_index], source)?;
        let mut body =
            lower_to_ir_with_dialect(body_text, registry, config, registry.profile()).top_level;
        discard_execution_proofs(&mut body);
        rebase_script(&mut body, i64::from(body_base));
        if let Some(condition_index) = condition {
            let (text, base, condition_span) = original_literal(
                tokens.word_exprs.get(condition_index + 1)?,
                &args[condition_index],
                source,
            )?;
            clauses.push(IfClause {
                condition: parse_expr_for_profile(text, registry.profile()),
                condition_span,
                condition_base: Some(base),
                body,
                body_span,
            });
        } else {
            else_body = Some(body);
            else_span = Some(body_span);
        }
    }
    Some(Statement::If {
        span: *span,
        clauses,
        else_body,
        else_span,
    })
}

/// Validate the affine mapping against the actual source bytes; decoded or
/// reconstructed text cannot receive an invented condition/body base.
fn original_literal<'a>(
    word: &WordExpr,
    value: &str,
    source: &'a str,
) -> Option<(&'a str, u32, Span)> {
    let (text, site, prefix) = match word {
        WordExpr::Literal { text, source } => (text, source, 0),
        WordExpr::BracedLiteral { text, source } => (text, source, 1),
        _ => return None,
    };
    if text != value {
        return None;
    }
    let base = site.span.start().checked_add(prefix)?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    let original = source.get(base as usize..end as usize)?;
    (original == text).then_some((original, base, Span::new(base, end)))
}

/// Independently lowered body receipts belong to their original Tcl entry.
/// Keep their syntax and spans, never import those execution permissions into
/// a BPF-derived control region.
fn discard_execution_proofs(script: &mut Script) {
    tcl_compiler::ir::for_each_script_mut(script, &mut |script| {
        script.executed_source = None;
        script.implicit_math_invocations.clear();
        script.expression_preparations.clear();
        script.command_binding_sites = Default::default();
        script.procedure_binding_requirements = Default::default();
        script.native_compilation_failure = None;
        script.native_compilation_admission = None;
        for statement in &mut script.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.source_binding = None;
                tokens.nested_bindings.clear();
                tokens.variable_accesses.clear();
                tokens.evaluated_body = None;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_registry_context_is_retained_through_body_projection() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = lower_bpf_source("deny drop", registry);
        let source = OriginalBpfSource::from_module(&module);
        source
            .validate_script(
                &module.top_level,
                registry,
                crate::diag::BpfDiag::OutOfSubset,
            )
            .unwrap();
        let foreign = CommandRegistry::build_default();
        assert!(
            source
                .command_spec(
                    &module.top_level.statements[0],
                    &foreign,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .is_err()
        );
        assert!(
            source
                .validate_script(
                    &module.top_level,
                    &foreign,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .is_err()
        );
        assert!(crate::capability::collect_policy(&module.top_level, &foreign, &source).is_err());
        assert!(crate::deploy::collect_attach(&module.top_level, &foreign, &source).is_err());
        assert!(crate::profile::collect_profile(&module.top_level, &foreign, &source).is_err());
        assert!(crate::template::collect_templates(&module.top_level, &foreign, &source).is_err());
        let module = lower_bpf_source("when XDP {deny drop}", registry);
        let source = OriginalBpfSource::from_module(&module);
        let Statement::Call { args, tokens, .. } = &module.top_level.statements[0] else {
            panic!("actual original body carrier");
        };
        let body = source.body(tokens.as_ref(), args, 1).unwrap();
        assert_eq!(body.registry, source.registry);
        let lowered = body.lower(registry);
        body.validate_script(
            &lowered.top_level,
            registry,
            crate::diag::BpfDiag::OutOfSubset,
        )
        .unwrap();
        assert!(
            body.validate_script(
                &lowered.top_level,
                &foreign,
                crate::diag::BpfDiag::OutOfSubset
            )
            .is_err()
        );
    }

    #[test]
    fn complete_original_inventory_is_required_before_collector_defaults() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = lower_bpf_source("deny drop\nattach xdp eth0\n", registry);
        let source = OriginalBpfSource::from_module(&module);
        source
            .validate_script(
                &module.top_level,
                registry,
                crate::diag::BpfDiag::OutOfSubset,
            )
            .unwrap();
        for count in [0, 1] {
            let mut partial = module.top_level.clone();
            partial.statements.truncate(count);
            assert!(crate::capability::collect_policy(&partial, registry, &source).is_err());
            assert!(crate::deploy::collect_attach(&partial, registry, &source).is_err());
            assert!(crate::profile::collect_profile(&partial, registry, &source).is_err());
            assert!(crate::template::collect_templates(&partial, registry, &source).is_err());
        }
        let mut changed = module.top_level.clone();
        let mut replacement = lower_bpf_source("return", registry)
            .top_level
            .statements
            .remove(0);
        let Statement::Return { span, .. } = &mut replacement else {
            panic!("original return node");
        };
        *span = changed.statements[0].span();
        changed.statements[0] = replacement;
        assert!(
            source
                .validate_script(&changed, registry, crate::diag::BpfDiag::OutOfSubset)
                .is_err()
        );
        assert!(crate::capability::collect_policy(&changed, registry, &source).is_err());
        assert!(crate::deploy::collect_attach(&changed, registry, &source).is_err());
        assert!(crate::profile::collect_profile(&changed, registry, &source).is_err());
        assert!(crate::template::collect_templates(&changed, registry, &source).is_err());

        let foreign = lower_bpf_source("deny pass\nattach xdp eth0\n", registry);
        let foreign_source = OriginalBpfSource::from_module(&foreign);
        assert!(
            foreign_source
                .validate_script(
                    &module.top_level,
                    registry,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .is_err()
        );
        let lenient = lower_bpf_source_with_config(
            "deny {drop}tail",
            registry,
            LexerConfig {
                strict_quoting: false,
                ..LexerConfig::for_profile(registry.profile())
            },
        );
        let strict = lower_bpf_source_with_config(
            "deny {drop}tail",
            registry,
            LexerConfig {
                strict_quoting: true,
                ..LexerConfig::for_profile(registry.profile())
            },
        );
        let strict_source = OriginalBpfSource::from_module(&strict);
        assert!(
            strict_source
                .validate_script(
                    &lenient.top_level,
                    registry,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .is_err()
        );
    }

    #[test]
    fn original_head_selection_cannot_erase_declarations_or_claim_default_absence() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        for text in [
            "deny drop",
            "attach xdp eth0",
            "profile eth",
            "template t {} {pass}",
            "pass",
        ] {
            let mut module = lower_bpf_source(text, registry);
            let source = OriginalBpfSource::from_module(&module);
            source
                .command_spec(
                    &module.top_level.statements[0],
                    registry,
                    crate::diag::BpfDiag::OutOfSubset,
                )
                .unwrap();
            let Statement::Call {
                command,
                tokens: Some(tokens),
                ..
            } = &mut module.top_level.statements[0]
            else {
                panic!("actual original DSL call");
            };
            let WordExpr::Literal { text, .. } = &mut tokens.word_exprs[0] else {
                panic!("literal original head");
            };
            *text = "ignored".into();
            tokens.argv_texts[0] = "ignored".into();
            *command = "ignored".into();
            assert!(
                source
                    .command_spec(
                        &module.top_level.statements[0],
                        registry,
                        crate::diag::BpfDiag::OutOfSubset
                    )
                    .is_err()
            );
            assert!(
                crate::capability::collect_policy(&module.top_level, registry, &source).is_err()
            );
            assert!(crate::deploy::collect_attach(&module.top_level, registry, &source).is_err());
            assert!(crate::profile::collect_profile(&module.top_level, registry, &source).is_err());
            assert!(
                crate::template::collect_templates(&module.top_level, registry, &source).is_err()
            );
        }
    }

    #[test]
    fn original_head_selection_decodes_configured_words_and_distinguishes_dynamic_or_missing() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        for text in [r"\x64eny drop", "{deny} drop", "\"deny\" drop"] {
            let module = lower_bpf_source(text, registry);
            let source = OriginalBpfSource::from_module(&module);
            assert_eq!(
                source
                    .declaration_kind(
                        &module.top_level.statements[0],
                        registry,
                        crate::diag::BpfDiag::OutOfSubset
                    )
                    .unwrap(),
                Some(tcl_registry::bpf_op::BpfDeclKind::Deny)
            );
        }
        let module = lower_bpf_source("not_bpf arg", registry);
        let source = OriginalBpfSource::from_module(&module);
        assert!(
            source
                .command_spec(
                    &module.top_level.statements[0],
                    registry,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .unwrap()
                .is_none()
        );
        let mut module = lower_bpf_source("pass", registry);
        let source = OriginalBpfSource::from_module(&module);
        let Statement::Call { tokens, .. } = &mut module.top_level.statements[0] else {
            panic!("actual original call");
        };
        *tokens = None;
        assert!(
            source
                .command_spec(
                    &module.top_level.statements[0],
                    registry,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .is_err()
        );
        let module = lower_bpf_source("$head arg", registry);
        let source = OriginalBpfSource::from_module(&module);
        assert_eq!(
            source
                .command_spec(
                    &module.top_level.statements[0],
                    registry,
                    crate::diag::BpfDiag::OutOfSubset
                )
                .unwrap_err()
                .code,
            crate::diag::BpfDiag::OutOfSubset
        );
    }

    #[test]
    fn body_owner_retains_configuration_and_rejects_invented_or_cooked_geometry() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let config = LexerConfig {
            strict_quoting: true,
            ..LexerConfig::for_profile(registry.profile())
        };
        let module = lower_bpf_source_with_config("when XDP {pass}\n", registry, config);
        let source = OriginalBpfSource::from_module(&module);
        let Statement::Call { args, tokens, .. } = &module.top_level.statements[0] else {
            panic!("DSL declaration");
        };
        let body = source.body(tokens.as_ref(), args, 1).unwrap();
        assert_eq!(body.config, config);
        assert_eq!(body.text(), Some("pass"));
        assert_eq!(body.base(), u32::try_from("when XDP {".len()).unwrap());
        let lowered = body.lower(registry);
        assert_eq!(lowered.native_lexer_config(), config.nested());
        assert!(lowered.top_level.executed_source.is_none());
        assert!(lowered.top_level.native_compilation_admission.is_none());
        let mut changed = args.clone();
        changed[1] = "drop".into();
        assert!(source.body(tokens.as_ref(), &changed, 1).is_none());
        assert!(source.body(None, args, 1).is_none());
        let mut forged = tokens.clone().unwrap();
        let WordExpr::Literal { text, .. } = &mut forged.word_exprs[0] else {
            panic!("literal head");
        };
        *text = "loop".into();
        forged.argv_texts[0] = "loop".into();
        assert!(source.body(Some(&forged), args, 1).is_none());
        let mut forged = tokens.clone().unwrap();
        let WordExpr::Literal { text, .. } = &mut forged.word_exprs[1] else {
            panic!("literal event");
        };
        *text = "TC".into();
        forged.argv_texts[1] = "TC".into();
        let mut changed = args.clone();
        changed[0] = "TC".into();
        assert!(source.body(Some(&forged), &changed, 1).is_none());

        let parent = lower_bpf_source("not_bpf when XDP {pass}\n", registry);
        let parent_source = OriginalBpfSource::from_module(&parent);
        let Statement::Call {
            args,
            tokens: Some(tokens),
            ..
        } = &parent.top_level.statements[0]
        else {
            panic!("original call");
        };
        let mut shortened = tokens.clone();
        shortened.word_exprs.remove(0);
        shortened.argv_texts.remove(0);
        assert!(
            parent_source
                .body(Some(&shortened), &args[1..], 1)
                .is_none()
        );

        let cooked = lower_bpf_source("when XDP {pass\\\n }\n", registry);
        let source = OriginalBpfSource::from_module(&cooked);
        let Statement::Call { args, tokens, .. } = &cooked.top_level.statements[0] else {
            panic!("DSL declaration");
        };
        assert!(source.body(tokens.as_ref(), args, 1).is_none());
    }
}
