// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Logical C8.4 Eval words over an independently retained physical C9 core.
use super::Value;
use crate::compiled::NativeCompilerPolicy;
use std::rc::Rc;
use tcl_registry::native_eval_object::{EvalObjectPurpose, NativeEvalObjectProtocol};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

#[derive(Clone, Copy)]
pub(crate) struct LogicalEvalSourceEntry {
    policy: NativeCompilerPolicy,
}

impl LogicalEvalSourceEntry {
    pub(crate) fn is_current(self, policy: &NativeCompilerPolicy) -> bool {
        self.policy == *policy
    }

    pub(crate) fn value(self, literal: &tcl_bytecode::NativeStringLiteral) -> Value {
        debug_assert!(self.policy.source_word_provider.is_some());
        Value::new_native_string_bytes(literal.bytes())
    }
}

impl crate::Vm {
    pub(crate) fn logical_eval_source_entry(
        &self,
        strings: NativeStringProtocol,
        selected: NativeEvalObjectProtocol,
        purpose: EvalObjectPurpose,
    ) -> Result<Option<LogicalEvalSourceEntry>, ValueError> {
        let dialect = self.native_invocation_dialect();
        if purpose != EvalObjectPurpose::Eval || dialect.native_eval_object_protocol().is_some() {
            return Ok(None);
        }
        let Some(provider) = self.logical_providers.eval_object else {
            return Ok(None);
        };
        if dialect.eval_object_protocol(Some(provider)) != Some(selected)
            || selected.compiles_source(purpose)
        {
            return Ok(None);
        }
        let unavailable =
            || ValueError::CommandProtocolUnavailable("logical eval source word issuer");
        let words = self
            .logical_providers
            .source_words
            .ok_or_else(unavailable)?;
        let source = dialect
            .logical_source_string_protocol(words, self.native_execution_profile())
            .ok_or_else(unavailable)?;
        let names = self.logical_providers.names.ok_or_else(unavailable)?;
        let variables = self
            .logical_providers
            .compiled_variables
            .ok_or_else(unavailable)?;
        let variables = dialect
            .authored_logical_compiled_variable_protocol(variables)
            .ok_or_else(unavailable)?;
        let c84 = tcl_dialect::TclVersion::V8_4;
        let c9 = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
        if source != NativeStringProtocol::C(c84)
            || self.source_string_protocol() != Some(source)
            || self.lexer_config().escapes != source.escape_syntax()
            || self
                .lexer_config()
                .grammar_over(self.native_execution_profile().grammar)
                != dialect.lexer_grammar
            || names != tcl_syntax::naming::NamePolicyProtocol::authored_tcl(c84)
            || self.name_policy_protocol() != Some(names)
            || self.compiled_variable_protocol() != Some(variables)
            || strings != c9
            || self
                .actual_native_invocation_dialect()
                .native_string_protocol()
                != Some(strings)
        {
            return Err(unavailable());
        }
        let engine = self.actual_engine_profile.ok_or_else(unavailable)?;
        if self
            .native_core_pin
            .as_ref()
            .is_none_or(|core| !std::ptr::eq(core.profile, engine))
        {
            return Err(unavailable());
        }
        Ok(Some(LogicalEvalSourceEntry {
            policy: self.native_compiler_policy(),
        }))
    }

    pub(crate) fn logical_eval_source_unit(
        &self,
        asm: &Rc<tcl_bytecode::FunctionAsm>,
        namespace: &tcl_runtime_api::ByteNamespacePath,
        strings: NativeStringProtocol,
        selected: NativeEvalObjectProtocol,
        purpose: EvalObjectPurpose,
    ) -> Result<Option<crate::compiled::CompiledUnit>, ValueError> {
        let Some(entry) = self.logical_eval_source_entry(strings, selected, purpose)? else {
            return Ok(None);
        };
        if asm.required_compiled_local_layout.is_some()
            || !crate::literal_pool::plain_source_operand_plan(asm)
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "logical eval source operand plan",
            ));
        }
        let mut unit = crate::compiled::CompiledUnit::new(
            Rc::clone(asm),
            namespace.clone(),
            self.profile_generation(),
            self.trace_deopt_epoch(),
            None,
            self.native_interpreter_identity(),
            crate::compiled::CompilerProvenance::LogicalEval(self.compiler_generation()),
        );
        unit.compiled_local_layout = None;
        unit.direct_source_operands =
            Some(crate::literal_pool::PlainSourceOperands::LogicalEval(entry));
        Ok(Some(unit))
    }
}

#[cfg(test)]
mod tests;
