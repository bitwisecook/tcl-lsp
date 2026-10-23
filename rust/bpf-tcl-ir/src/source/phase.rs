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

//! Complete original DSL scripts and their fixed transformation phases.

use std::sync::Arc;

use tcl_compiler::cfg::Function;
use tcl_compiler::ir::SourceBpfModuleContext;
use tcl_compiler::{Module, Script, Statement};
use tcl_lexer::LexerConfig;
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, RegistrySnapshot};

use super::{OriginalBpfSource, discard_execution_proofs};
use crate::diag::{BpfDiag, BpfError};

/// A complete script issued from the actual module input. Command receipts
/// describe individual selections; this owner separately retains their census
/// and the source context through transformations.
#[derive(Debug, Clone)]
pub(crate) struct BpfSourceScript {
    script: Script,
    source: OriginalBpfSource,
    context: Arc<SourceBpfModuleContext>,
    registry: RegistrySnapshot,
    environment: Arc<BpfEnvironment>,
}

#[derive(Debug, Clone)]
struct BpfEnvironment {
    profile: Option<crate::profile::BpfProfileSpec>,
    templates: Vec<crate::template::TemplateDef>,
    policy: crate::capability::CapabilityPolicy,
}

/// Actual CFG output of the fixed, fully expanded DSL pipeline.
pub(crate) struct BpfPreparedFunction {
    function: Function,
    registry: RegistrySnapshot,
    config: LexerConfig,
    _context: Arc<SourceBpfModuleContext>,
    _effects: tcl_compiler::ir::SourceBpfClosedEffects,
}

impl BpfPreparedFunction {
    pub(crate) fn function(&self, registry: &CommandRegistry) -> Result<&Function, BpfError> {
        if self.registry.semantic_key() != registry.snapshot().semantic_key() {
            return Err(unavailable(Span::empty(0)));
        }
        Ok(&self.function)
    }

    pub(crate) fn config(&self) -> LexerConfig {
        self.config
    }
}

impl BpfSourceScript {
    pub(crate) fn from_module(
        module: &Module,
        registry: &CommandRegistry,
    ) -> Result<Self, BpfError> {
        let source = OriginalBpfSource::from_module(module);
        let context = source
            .module_context
            .as_ref()
            .filter(|context| context.matches_module(module, registry))
            .ok_or_else(|| unavailable(Span::empty(0)))?;
        let mut supplied = module.top_level.clone();
        discard_execution_proofs(&mut supplied);
        let mut script = source.lower(registry).top_level;
        // Compare complete syntax, including conditions, children and retained
        // naming context. Only the independent Native advice is discarded.
        if supplied != script {
            return Err(unavailable(Span::empty(0)));
        }
        source.validate_script(&script, registry, BpfDiag::OutOfSubset)?;
        bind_original(&mut script, &source, registry)?;
        let environment = BpfEnvironment {
            profile: crate::profile::collect_profile(&script, registry, &source)?,
            templates: crate::template::collect_templates(&script, registry, &source)?,
            policy: crate::capability::collect_policy(&script, registry, &source)?,
        };
        Ok(Self {
            script,
            source: source.clone(),
            context: Arc::new(context.clone()),
            registry: registry.snapshot(),
            environment: Arc::new(environment),
        })
    }

    pub(crate) fn script(&self) -> &Script {
        &self.script
    }

    pub(crate) fn matches_module(&self, module: &Module, registry: &CommandRegistry) -> bool {
        self.registry.semantic_key() == registry.snapshot().semantic_key()
            && self.context.matches_module(module, registry)
    }
    pub(crate) fn original_source(&self) -> &OriginalBpfSource {
        &self.source
    }

    /// Project a reached handler body from this phase's genuine selected call.
    pub(crate) fn handler_body(
        &self,
        selected: &crate::frontend::SelectedWhenDeclaration<'_>,
        registry: &CommandRegistry,
    ) -> Result<Self, BpfError> {
        let (phase, statement, source) = selected.original_body();
        if !std::ptr::eq(self, phase)
            || !self
                .script
                .statements
                .iter()
                .any(|held| std::ptr::eq(held, statement))
        {
            return Err(unavailable(statement.span()));
        }
        let spec = super::selected_command(statement, registry)?;
        if spec.and_then(|spec| spec.bpf_op).map(|op| op.kind)
            != Some(tcl_registry::bpf_op::BpfOpKind::Framework(
                tcl_registry::bpf_op::BpfDeclKind::When,
            ))
        {
            return Err(unavailable(statement.span()));
        }
        let source = source.clone();
        let script =
            prepared_body(&source, registry).map_err(|error| error.offset(source.base()))?;
        Ok(Self {
            script,
            source,
            context: Arc::clone(&self.context),
            registry: self.registry.clone(),
            environment: Arc::clone(&self.environment),
        })
    }

    /// Remove only actual metadata declarations from the complete root phase.
    pub(crate) fn without_declarations(
        mut self,
        registry: &CommandRegistry,
    ) -> Result<Self, BpfError> {
        let mut kept = Vec::with_capacity(self.script.statements.len());
        for statement in self.script.statements {
            let declaration = if matches!(
                statement,
                Statement::Call { .. } | Statement::Barrier { .. }
            ) {
                super::selected_command(&statement, registry)?
                    .and_then(|spec| spec.bpf_op)
                    .map(|op| op.kind)
            } else {
                None
            };
            if !matches!(
                declaration,
                Some(tcl_registry::bpf_op::BpfOpKind::Framework(
                    tcl_registry::bpf_op::BpfDeclKind::Profile
                        | tcl_registry::bpf_op::BpfDeclKind::Template
                        | tcl_registry::bpf_op::BpfDeclKind::Allow
                        | tcl_registry::bpf_op::BpfDeclKind::Deny
                        | tcl_registry::bpf_op::BpfDeclKind::Attach
                ))
            ) {
                kept.push(statement);
            }
        }
        self.script = Script::from_statements(kept);
        Ok(self)
    }

    /// The fixed pipeline alone issues a prepared function. Its selected
    /// templates, profile and policy come from the original complete module.
    pub(crate) fn prepare_function(
        self,
        module: &Module,
        name: &str,
        registry: &CommandRegistry,
    ) -> Result<BpfPreparedFunction, BpfError> {
        if !self.matches_module(module, registry) {
            return Err(unavailable(Span::empty(0)));
        }
        let used =
            crate::template::expand_uses(&self.script, &self.environment.templates, registry)?;
        let unrolled = crate::unroll::unroll_loops(&used, registry, &self.source)?;
        let expanded = match self.environment.profile.as_ref() {
            Some(profile) => crate::profile::expand_fields(&unrolled, profile, registry)?,
            None => unrolled,
        };
        crate::capability::check_policy(&expanded, &self.environment.policy, registry)?;
        let effects = self
            .context
            .closed_effects_for(module, registry, &expanded, self.source.config)
            .ok_or_else(|| unavailable(Span::empty(0)))?;
        let function = tcl_compiler::cfg_builder::build_cfg_function_in_bpf_selected_scope(
            name,
            &expanded,
            module,
            registry,
            self.source.config,
            &effects,
        )
        .ok_or_else(|| unavailable(Span::empty(0)))?;
        Ok(BpfPreparedFunction {
            function,
            registry: self.registry,
            config: self.source.config,
            _context: self.context,
            _effects: effects,
        })
    }
}

pub(super) fn prepared_body(
    source: &OriginalBpfSource,
    registry: &CommandRegistry,
) -> Result<Script, BpfError> {
    let mut script = source.lower(registry).top_level;
    source.validate_script(&script, registry, BpfDiag::OutOfSubset)?;
    bind_original(&mut script, source, registry)?;
    Ok(script)
}

fn unavailable(span: Span) -> BpfError {
    BpfError::new(
        BpfDiag::OutOfSubset,
        span,
        "DSL phase requires its complete original source and naming context",
    )
}

/// This only receives independently lowered, fully compared original syntax.
/// A child owner is projected from the selected original body word, never from
/// the child's report span or the text of a possible command name.
fn bind_original(
    script: &mut Script,
    source: &OriginalBpfSource,
    registry: &CommandRegistry,
) -> Result<(), BpfError> {
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        source.image.clone(),
        source.region,
        source.config,
        source.purpose,
    )
    .map_err(|_| unavailable(Span::empty(0)))?;
    if plan.fatal_tail.is_some() || plan.commands.len() != script.statements.len() {
        return Err(unavailable(Span::empty(0)));
    }
    for (statement, command) in script.statements.iter_mut().zip(plan.commands) {
        let span = statement.span();
        match statement {
            Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } => {
                let tokens = tokens.as_mut().ok_or_else(|| unavailable(span))?;
                if !tokens.install_source_bpf_command(
                    source.image.clone(),
                    source.region,
                    source.config,
                    source.purpose,
                    args,
                    registry,
                ) {
                    return Err(unavailable(span));
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                let values = command
                    .words
                    .iter()
                    .map(|word| {
                        tcl_syntax::word_rules::original_static_word_unicode_value(word)
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .ok_or_else(|| unavailable(span))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let (head, args) = values.split_first().ok_or_else(|| unavailable(span))?;
                let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
                let operands = registry
                    .bpf_conditional_operands(head, &refs)
                    .ok_or_else(|| unavailable(span))?;
                let mut clauses = clauses.iter_mut();
                let mut else_body = else_body.as_mut();
                for (condition, index) in operands {
                    let word = command
                        .words
                        .get(index + 1)
                        .ok_or_else(|| unavailable(span))?;
                    let child = OriginalBpfSource::from_body(
                        word,
                        source.registry.clone(),
                        source.module_context.clone(),
                    )
                    .ok_or_else(|| unavailable(span))?;
                    let body = if condition.is_some() {
                        &mut clauses.next().ok_or_else(|| unavailable(span))?.body
                    } else {
                        else_body.take().ok_or_else(|| unavailable(span))?
                    };
                    let delta = i64::from(
                        child
                            .base()
                            .checked_sub(source.base())
                            .ok_or_else(|| unavailable(span))?,
                    );
                    tcl_compiler::rebase_script(body, -delta);
                    let expected = child.lower(registry).top_level;
                    if *body != expected {
                        return Err(unavailable(span));
                    }
                    child.validate_script(body, registry, BpfDiag::OutOfSubset)?;
                    bind_original(body, &child, registry)?;
                    tcl_compiler::rebase_script(body, delta);
                }
                if clauses.next().is_some() || else_body.is_some() {
                    return Err(unavailable(span));
                }
            }
            // Exact syntax survives for the subset diagnostic. It cannot gain
            // a command receipt by being represented as a structured IR node.
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_phase_retains_original_selection_and_refuses_changed_syntax_or_context() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = super::super::lower_bpf_source("if {$x > 0} {drop} else {pass}", registry);
        let phase = BpfSourceScript::from_module(&module, registry).unwrap();
        assert!(phase.matches_module(&module, registry));
        let Statement::If { clauses, .. } = &phase.script().statements[0] else {
            panic!("authentic structured conditional");
        };
        let Statement::Call {
            args,
            tokens: Some(tokens),
            ..
        } = &clauses[0].body.statements[0]
        else {
            panic!("authentic selected body command");
        };
        assert!(
            tokens
                .source_bpf_command
                .as_ref()
                .unwrap()
                .descriptor(tokens, args, registry)
                .unwrap()
                .is_some()
        );
        let mut changed = module.clone();
        changed.top_level.statements.clear();
        assert!(BpfSourceScript::from_module(&changed, registry).is_err());
        let mut changed = module.clone();
        let Statement::If { clauses, .. } = &mut changed.top_level.statements[0] else {
            unreachable!()
        };
        clauses[0].body.statements.clear();
        assert!(BpfSourceScript::from_module(&changed, registry).is_err());
        let mut changed = module.clone();
        let Statement::If { clauses, .. } = &mut changed.top_level.statements[0] else {
            unreachable!()
        };
        clauses[0].condition = tcl_compiler::parse_expr_for_profile("0", registry.profile());
        assert!(BpfSourceScript::from_module(&changed, registry).is_err());
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::from("pass");
        assert!(!phase.matches_module(&changed, registry));
        assert!(BpfSourceScript::from_module(&changed, registry).is_err());
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(!phase.matches_module(&changed, registry));
        assert!(BpfSourceScript::from_module(&changed, registry).is_err());
    }
    #[test]
    fn fixed_phase_preserves_mixed_template_origins_and_generated_operation_identity() {
        crate::frontend::compile_module(
            r"template t {{a b}} {setint result {${a b}}}
when XDP {use t {a b=7}; pass}",
        )
        .expect(
            "reached use decodes actual original scalar words and retains template body source",
        );
        crate::frontend::compile_module(
            "profile p {field setint 0 8}\ntemplate t {v} {}\nwhen XDP {use t v=1; pass}",
        )
        .expect("generated scalar binding cannot become a same-spelling profile field");
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = super::super::lower_bpf_source(
            "template t {} {loop 1 i {setint value 1}}\nwhen XDP {use t; pass}",
            registry,
        );
        let phase = BpfSourceScript::from_module(&module, registry).unwrap();
        let statement = &phase.script().statements[1];
        let selected =
            crate::frontend::select_when_declaration(statement, &phase, registry).unwrap();
        let body = phase.handler_body(&selected, registry).unwrap();
        let function = body
            .prepare_function(&module, "a label with spaces", registry)
            .unwrap();
        crate::lower::lower_prepared_function(&function, crate::ir::ProgType::Xdp, registry)
            .expect(
                "mixed template and handler coordinates retain their actual command selections",
            );
        let mut changed = module.clone();
        changed.source = tcl_lexer::SourceImage::from("pass");
        let selected =
            crate::frontend::select_when_declaration(statement, &phase, registry).unwrap();
        let body = phase.handler_body(&selected, registry).unwrap();
        assert!(body.prepare_function(&changed, "main", registry).is_err());
    }

    #[test]
    fn closed_bpf_effect_phase_accepts_static_socket_program_and_refuses_substitution() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let module = super::super::lower_bpf_source("map m hash 4 8 1; accept", registry);
        let phase = BpfSourceScript::from_module(&module, registry).unwrap();
        let prepared = phase
            .prepare_function(&module, "main", registry)
            .expect("static selected BPF operations have no Tcl call effects");
        crate::lower::lower_prepared_function(
            &prepared,
            crate::ir::ProgType::SocketFilter,
            registry,
        )
        .expect("socket filter accepts its selected verdict");

        let dynamic =
            super::super::lower_bpf_source("map m hash 4 8 1; accept [unknown]", registry);
        let phase = BpfSourceScript::from_module(&dynamic, registry).unwrap();
        assert!(phase.prepare_function(&dynamic, "main", registry).is_err());
    }

    #[test]
    fn losing_a_selected_command_never_proves_an_empty_policy_or_map_inventory() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        // Socket filters use accept/drop. Keep the former pass spelling as a
        // negative control so it cannot masquerade as a selection failure.
        let incompatible = super::super::lower_bpf_source("map m hash 4 8 1; pass", registry);
        let incompatible_phase = BpfSourceScript::from_module(&incompatible, registry).unwrap();
        let incompatible_prepared = incompatible_phase
            .prepare_function(&incompatible, "main", registry)
            .expect("pass remains a selected command before program-type validation");
        let error = crate::lower::lower_prepared_function(
            &incompatible_prepared,
            crate::ir::ProgType::SocketFilter,
            registry,
        )
        .unwrap_err();
        assert_eq!(error.code, crate::diag::BpfDiag::OutOfSubset);
        assert!(error.msg.contains("`pass` is not a valid verdict"));

        let module = super::super::lower_bpf_source("map m hash 4 8 1; accept", registry);
        let phase = BpfSourceScript::from_module(&module, registry).unwrap();
        let mut script = phase.script().clone();
        let Statement::Call { tokens, .. } = &mut script.statements[0] else {
            unreachable!()
        };
        *tokens = None;
        assert!(
            crate::capability::check_policy(
                &script,
                &crate::capability::CapabilityPolicy::default(),
                registry
            )
            .is_err()
        );
        assert!(crate::template::expand_uses(&script, &[], registry).is_err());
        assert!(crate::unroll::unroll_loops(&script, registry, phase.original_source()).is_err());
        let mut prepared = phase
            .clone()
            .without_declarations(registry)
            .unwrap()
            .prepare_function(&module, "main", registry)
            .unwrap();
        crate::lower::lower_prepared_function(
            &prepared,
            crate::ir::ProgType::SocketFilter,
            registry,
        )
        .expect("actual map and verdict selection baseline");
        let selected = prepared
            .function
            .blocks
            .values_mut()
            .flat_map(|block| &mut block.statements)
            .find(|statement| {
                super::super::selected_command(statement, registry)
                    .ok()
                    .flatten()
                    .and_then(|spec| spec.bpf_op)
                    .is_some_and(|op| op.kind == tcl_registry::bpf_op::BpfOpKind::MapDeclare)
            })
            .expect("actual selected map declaration");
        let Statement::Call { tokens, .. } = selected else {
            unreachable!()
        };
        *tokens = None;
        assert!(
            crate::lower::lower_prepared_function(
                &prepared,
                crate::ir::ProgType::SocketFilter,
                registry
            )
            .is_err()
        );
    }
    #[test]
    fn only_the_selected_when_header_body_can_enter_its_actual_phase() {
        let registry = tcl_registry::model::ingress::static_context_for("bpf").commands();
        let text = "when XDP priority 7 {drop}";
        let module = super::super::lower_bpf_source(text, registry);
        let phase = BpfSourceScript::from_module(&module, registry).unwrap();
        let selected = crate::frontend::select_when_declaration(
            &phase.script().statements[0],
            &phase,
            registry,
        )
        .unwrap();
        let body = phase.handler_body(&selected, registry).unwrap();
        assert_eq!(
            body.original_source().base() as usize,
            text.find("drop").unwrap()
        );
        assert_eq!(
            super::super::selected_head(&body.script().statements[0], registry).unwrap(),
            "drop"
        );
        let other_phase = phase.clone();
        assert!(other_phase.handler_body(&selected, registry).is_err());
    }
}
