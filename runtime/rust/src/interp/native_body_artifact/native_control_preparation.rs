// SPDX-License-Identifier: AGPL-3.0-or-later
//! Chronological original compiler visits shared by protected control operations.
use super::*;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
use tcl_registry::native_control_compilation::NativeControlPreparationStep as Visit;

#[derive(Default)]
pub(super) struct PreparedControlOperands {
    pub(super) declined_script: bool,
    pub(super) words: HashMap<usize, WordInstruction>,
    pub(super) literals: HashMap<Operand, usize>,
    pub(super) temporaries: Vec<usize>,
    pub(super) expressions: HashMap<Operand, native_control::PreparedNativeExpression>,
}
impl Builder<'_> {
    pub(super) fn prepare_control_steps(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        visits: &[Visit],
        depth: u32,
    ) -> Result<PreparedControlOperands, ValueError> {
        let mut prepared = PreparedControlOperands::default();
        for visit in visits {
            match visit {
                Visit::BooleanProbe(probe) => {
                    if !probe.matches_original(captured, self.stamp.physical) {
                        return Err(unavailable("original native Boolean probe operand"));
                    }
                }
                Visit::DeclareLocal(name) => {
                    self.local(name, None)
                        .ok_or_else(|| unavailable("native control named local declaration"))?;
                }
                Visit::DeclareAnonymousLocal => {
                    prepared.temporaries.push(self.lvt.intern_anonymous());
                }
                Visit::Word(operand) => match operand {
                    Operand::Original(index) => {
                        if !prepared.words.contains_key(index) {
                            let word = self.namespace_word(captured, *index, false, depth)?;
                            prepared.words.insert(*index, word);
                        }
                    }
                    Operand::LiteralExpansion { value, .. } => {
                        prepared
                            .literals
                            .entry(operand.clone())
                            .or_insert_with(|| self.literals.intern_bytes(value));
                    }
                },
                Visit::Literal(bytes) => {
                    self.literals.intern_bytes(bytes);
                }
                Visit::Integer(value) => {
                    self.original_literal(obj::Owned::fresh(obj::new_wide_int_obj(*value)));
                }
                Visit::List(members) => {
                    self.literals
                        .register_private_constant_list(members, self.stamp.source_protocol);
                }
                Visit::Script { span, context, .. }
                | Visit::SpeculativeScript { span, context, .. } => {
                    let speculative = matches!(visit, Visit::SpeculativeScript { .. });
                    let retained_scripts = speculative.then(|| {
                        self.scripts
                            .keys()
                            .copied()
                            .collect::<std::collections::HashSet<_>>()
                    });
                    let old_parse_failure =
                        speculative.then(|| self.parse_failure.take()).flatten();
                    let old_compilation_failure = speculative
                        .then(|| self.compilation_failure.take())
                        .flatten();
                    let previous = self.context;
                    self.context = match context {
                        tcl_registry::native_compilation::NativeCompiledBodyContext::Inherit => previous,
                        tcl_registry::native_compilation::NativeCompiledBodyContext::ExceptionRange => previous.with_inline_exception_range(),
                        tcl_registry::native_compilation::NativeCompiledBodyContext::Loop => Context {
                            loop_depth: previous.loop_depth.checked_add(1).ok_or_else(|| unavailable("native control loop compiler depth"))?,
                            ..previous
                        },
                    };
                    let result = self.script(*span, depth + 1);
                    self.context = previous;
                    if speculative {
                        let rejected =
                            self.parse_failure.is_some() || self.compilation_failure.is_some();
                        self.parse_failure = old_parse_failure;
                        self.compilation_failure = old_compilation_failure;
                        if result.is_err() && rejected {
                            let retained_scripts =
                                retained_scripts.expect("speculative script checkpoint");
                            self.scripts
                                .retain(|span, _| retained_scripts.contains(span));
                            prepared.declined_script = true;
                            break;
                        }
                    }
                    result?;
                }
                Visit::Expression(operand) => {
                    let program =
                        tcl_registry::native_expression_program::prepare_native_expression_program(
                            captured,
                            operand,
                            self.interp.native_invocation_dialect(),
                        )
                        .map_err(|_| unavailable("native control original expression program"))?;
                    let expression = self.prepare_body_expression(&program, depth)?;
                    prepared.expressions.insert(operand.clone(), expression);
                }
            }
        }
        Ok(prepared)
    }

    pub(super) fn validate_control_boolean_probes(
        &self,
        captured: &NativeCompilerWords<'_>,
        visits: &[Visit],
    ) -> Result<(), ValueError> {
        if tcl_registry::native_expression_program::control_boolean_probes_match(
            visits,
            captured,
            self.stamp.expression_policy.as_ref(),
            tcl_registry::InvocationDialect::for_version(self.stamp.physical),
        ) {
            Ok(())
        } else {
            Err(unavailable("original Boolean pruning evaluation policy"))
        }
    }
}
