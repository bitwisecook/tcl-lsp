// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Entered logical file syntax, independent of file and body execution authority.
use super::{Value, Vm, native_name_consumers::OriginalNameConsumerScope};
use crate::compiled::NativeCompilerPolicy;
use tcl_cmd_core::{CmdError, encoding::NativeSourceFileProtocol};
use tcl_registry::source_file::{
    LogicalSourceFileProvider, LogicalSourceFileRecipe, SourceFileGrammar,
};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

pub(crate) struct LogicalSourceFileEntry<'a> {
    names: &'a OriginalNameConsumerScope,
    args: &'a [Value],
    provider: LogicalSourceFileProvider,
    recipe: LogicalSourceFileRecipe,
    policy: NativeCompilerPolicy,
}

fn unavailable() -> CmdError {
    ValueError::CommandProtocolUnavailable("logical source file receipt").into()
}

impl LogicalSourceFileEntry<'_> {
    pub(crate) fn grammar(&self) -> SourceFileGrammar {
        self.recipe.grammar()
    }

    pub(crate) fn protocol(&self) -> NativeSourceFileProtocol {
        self.recipe.protocol()
    }

    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.names.operation_scope().check(vm)?;
        let core_current = vm.actual_engine_profile.is_some_and(|engine| {
            vm.native_core_pin
                .as_ref()
                .is_some_and(|core| std::ptr::eq(core.profile, engine))
        });
        if !core_current
            || vm.logical_providers.source_file != Some(self.provider)
            || vm.native_compiler_policy() != self.policy
        {
            return Err(unavailable());
        }
        Ok(())
    }

    pub(crate) fn admit_original_bytes(
        &self,
        vm: &mut Vm,
        index: usize,
        original: &Value,
        bytes: &[u8],
    ) -> Result<(), CmdError> {
        self.check(vm)?;
        if self.args.len() != 1
            || index != 0
            || !self
                .args
                .get(index)
                .is_some_and(|arg| std::ptr::eq(arg, original))
            || !bytes.is_ascii()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "logical source filename byte correspondence",
            )
            .into());
        }
        Ok(())
    }
}

impl Vm {
    /// Install an explicit logical file recipe, separately from source words.
    /// It grants no physical getter, filesystem or compiled-body permission.
    #[must_use]
    pub fn set_logical_source_file_provider(
        &mut self,
        provider: Option<LogicalSourceFileProvider>,
    ) -> bool {
        if provider.is_some_and(|provider| self.logical_file_recipe(provider).is_none()) {
            return false;
        }
        if self.logical_providers.source_file == provider {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.source_file = provider;
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    fn logical_file_recipe(
        &self,
        provider: LogicalSourceFileProvider,
    ) -> Option<LogicalSourceFileRecipe> {
        let dialect = self.native_invocation_dialect();
        let recipe = dialect.logical_source_file_recipe(provider)?;
        let eval = self.logical_providers.eval_object?;
        dialect.eval_object_protocol(Some(eval))?;
        let words = self.logical_providers.source_words?;
        let source =
            dialect.logical_source_string_protocol(words, self.native_execution_profile())?;
        let names = self.logical_providers.names?;
        let variables = dialect.authored_logical_compiled_variable_protocol(
            self.logical_providers.compiled_variables?,
        )?;
        let c84 = tcl_dialect::TclVersion::V8_4;
        if dialect.native_source_file_protocol().is_some()
            || source != NativeStringProtocol::C(c84)
            || self.source_string_protocol() != Some(source)
            || self.lexer_config().escapes != source.escape_syntax()
            || self
                .lexer_config()
                .grammar_over(self.native_execution_profile().grammar)
                != dialect.lexer_grammar
            || names != tcl_syntax::naming::NamePolicyProtocol::authored_tcl(c84)
            || self.name_policy_protocol() != Some(names)
            || self.compiled_variable_protocol() != Some(variables)
            || self
                .actual_native_invocation_dialect()
                .native_string_protocol()
                != Some(NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0))
        {
            return None;
        }
        let engine = self.actual_engine_profile?;
        self.native_core_pin
            .as_ref()
            .filter(|core| std::ptr::eq(core.profile, engine))?;
        Some(recipe)
    }

    pub(crate) fn logical_source_file_entry<'a>(
        &self,
        names: &'a OriginalNameConsumerScope,
        args: &'a [Value],
    ) -> Result<Option<LogicalSourceFileEntry<'a>>, CmdError> {
        let Some(provider) = self.logical_providers.source_file else {
            return Ok(None);
        };
        let recipe = self.logical_file_recipe(provider).ok_or_else(unavailable)?;
        Ok(Some(LogicalSourceFileEntry {
            names,
            args,
            provider,
            recipe,
            policy: self.native_compiler_policy(),
        }))
    }
}
