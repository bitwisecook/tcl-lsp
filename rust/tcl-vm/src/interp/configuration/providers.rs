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

//! The native engine profile and the logical providers an embedding host installs: quoting, script objects, source words, expression parsing, numerics, compiled variables and names.

use super::super::Vm;

impl Vm {
    /// Select the actual native engine supplied by this embedding host.
    ///
    /// This is independent of the source grammar, logical dialect and visible
    /// command surface. Only a measured engine profile is accepted; a vendor's
    /// compatible Tcl version is not evidence of an actual engine. Existing host
    /// replacements remain replacements, rather than becoming stock commands.
    /// Returns false when the profile has no measured native execution point.
    #[must_use]
    pub fn set_native_engine_profile(
        &mut self,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if tcl_native_model::InvocationDialect::of_profile(profile)
            .execution_point()
            .is_none()
        {
            return false;
        }
        if self
            .actual_engine_profile
            .is_some_and(|old| std::ptr::eq(old, profile))
        {
            return true;
        }
        self.advance_native_operation_context();
        self.actual_engine_profile = Some(profile);
        self.native_core_pin = None;
        let recipe = self
            .actual_native_invocation_dialect()
            .native_error_objects_protocol();
        self.native_errors.error_stack.configure(recipe);
        if let Err(error) = self.configure_c84_execution_environment_error_names() {
            let _ = crate::command::completion_from_cmd_error(self, error);
            return false;
        }
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        if self.initialize_original_jim_host_keys().is_err() {
            return false;
        }
        self.refresh_actual_stock_builtins();
        true
    }

    /// Install a logical simulation quote capability supplied by the embedding
    /// host. The actual engine must already be explicit; availability cannot
    /// grant this provider. This does not license native vendor compiler facts.
    #[must_use]
    pub fn set_logical_quote_provider(
        &mut self,
        provider: tcl_native_model::LogicalExpressionQuoteProvider,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || tcl_native_model::InvocationDialect::of_profile(self.dialect_profile())
                .logical_expression_quote_control(provider)
                .is_none()
        {
            return false;
        }
        if self.logical_providers.quote == Some(provider) {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.quote = Some(provider);
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    /// Install an explicitly authored script-object dispatch capability.
    /// Its logical recipe does not authenticate the physical string updater.
    #[must_use]
    pub fn set_logical_eval_object_provider(
        &mut self,
        provider: tcl_native_model::native_eval_object::LogicalEvalObjectProvider,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || self
                .native_invocation_dialect()
                .eval_object_protocol(Some(provider))
                .is_none()
        {
            return false;
        }
        if self.logical_providers.eval_object != Some(provider) {
            self.advance_native_operation_context();
            self.logical_providers.eval_object = Some(provider);
            self.bump_cmd_epoch();
            self.profile_generation = self.profile_generation.wrapping_add(1);
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        true
    }

    pub(crate) fn eval_object_protocol(
        &self,
    ) -> Option<tcl_native_model::native_eval_object::NativeEvalObjectProtocol> {
        self.native_invocation_dialect()
            .invocation_eval_object_protocol(self.logical_providers.eval_object)
    }

    /// Install source-word byte materialisation supplied explicitly by the host.
    /// The host engine must already be selected independently.
    #[must_use]
    pub fn set_logical_source_word_provider(
        &mut self,
        provider: tcl_native_model::LogicalSourceWordProvider,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || tcl_native_model::InvocationDialect::of_profile(self.dialect_profile())
                .logical_source_string_protocol(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        if self.logical_providers.source_words != Some(provider) {
            self.advance_native_operation_context();
            self.logical_providers.source_words = Some(provider);
            self.bump_cmd_epoch();
            self.profile_generation = self.profile_generation.wrapping_add(1);
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        true
    }

    /// Independently selected source escape recipe, never a physical receiver recipe.
    #[must_use]
    pub fn source_string_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        let dialect = self.native_invocation_dialect();
        dialect
            .native_source_string_protocol()
            .filter(|protocol| protocol.escape_syntax() == self.lexer_config().escapes)
            .or_else(|| {
                self.logical_providers.source_words.and_then(|provider| {
                    tcl_native_model::InvocationDialect::of_profile(
                        self.expression_source_profile(),
                    )
                    .logical_source_string_protocol(provider, self.expression_source_profile())
                })
            })
    }

    /// Install an explicit logical parser/template capability independently
    /// of the physical host engine and command availability. Native vendor
    /// grammar and compiler receipts remain unchanged.
    #[must_use]
    pub fn set_logical_expression_parse_provider(
        &mut self,
        provider: tcl_native_model::LogicalExpressionParseProvider,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || tcl_native_model::InvocationDialect::of_profile(self.dialect_profile())
                .logical_expression_parse_context(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        if self.logical_providers.expression_parse != Some(provider) {
            self.advance_native_operation_context();
            self.logical_providers.expression_parse = Some(provider);
            self.bump_cmd_epoch();
            self.profile_generation = self.profile_generation.wrapping_add(1);
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        true
    }

    pub(crate) fn expression_template_policy(
        &self,
    ) -> Option<tcl_native_model::TemplateParseErrors> {
        let dialect = self.native_invocation_dialect();
        tcl_native_model::TemplateParseErrors::for_dialect(dialect).or_else(|| {
            self.logical_providers
                .expression_parse
                .and_then(|provider| {
                    dialect.logical_expression_template_policy(provider, self.dialect_profile())
                })
        })
    }

    /// Install an embedding host's authored logical numeric simulation. This
    /// never grants a native vendor getter, compiler or hardware result receipt.
    #[must_use]
    pub fn set_logical_numeric_provider(
        &mut self,
        provider: tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || tcl_native_model::InvocationDialect::of_profile(self.dialect_profile())
                .authored_logical_numeric_simulation(provider)
                .is_none()
        {
            return false;
        }
        if self.logical_providers.numeric == Some(provider) {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.numeric = Some(provider);
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    /// Install an explicit logical compiler-local provider after selecting a
    /// physical host. The provider never re-keys published names or grants a
    /// native compiler receipt; cached artifacts retain its distinct identity.
    #[must_use]
    pub fn set_logical_compiled_variable_provider(
        &mut self,
        provider: tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || self
                .native_invocation_dialect()
                .authored_logical_compiled_variable_protocol(provider)
                .is_none()
        {
            return false;
        }
        if self.logical_providers.compiled_variables == Some(provider) {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.compiled_variables = Some(provider);
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    /// Install an explicitly authored logical naming and materialisation policy.
    /// The actual native engine must be independently selected. This provider
    /// grants no vendor-native compiler, hardware or object receipt and never
    /// changes the byte slots of already-published commands or variables.
    #[must_use]
    pub fn set_logical_name_provider(
        &mut self,
        provider: tcl_syntax::naming::NamePolicyProtocol,
    ) -> bool {
        if self.actual_engine_profile.is_none()
            || self
                .native_invocation_dialect()
                .authored_logical_name_simulation(provider)
                .is_none()
        {
            return false;
        }
        if self.logical_providers.names == Some(provider) {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.names = Some(provider);
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    /// Callback-free selected policy; it supplies no native environment or entry.
    pub(crate) fn numeric_policy_context(&self) -> crate::expr::NumericContext<'static> {
        let dialect = self.native_invocation_dialect();
        crate::expr::NumericContext {
            original: None,
            environment: None,
            dialect,
            simulation: self
                .logical_providers
                .numeric
                .and_then(|provider| dialect.authored_logical_numeric_simulation(provider)),
        }
    }

    /// The actual selected authored simulation, without querying a native Host.
    pub(crate) fn authored_numeric_context(&self) -> Option<crate::expr::NumericContext<'static>> {
        let context = self.numeric_policy_context();
        context.simulation.map(|_| context)
    }

    #[cfg(test)]
    pub(crate) fn numeric_context(&self) -> crate::expr::NumericContext<'_> {
        let policy = self.numeric_policy_context();
        crate::expr::NumericContext {
            original: None,
            environment: self.host.numeric_environment(),
            dialect: policy.dialect,
            simulation: policy.simulation,
        }
    }

    pub(super) fn refresh_actual_stock_builtins(&mut self) {
        let saved = self.active_native_profile;
        self.active_native_profile = self.actual_engine_profile.or(saved);
        crate::cmd_info::refresh_profile(self);
        crate::cmd_file::refresh_profile(self);
        crate::cmd_array::refresh_profile(self);
        crate::cmd_binary::refresh_profile(self);
        crate::cmd_clock::refresh_profile(self);
        crate::command::refresh_encoding_profile(self);
        crate::cmd_namespace::refresh_profile(self);
        crate::cmd_string::refresh_profile(self);
        crate::command::refresh_jim_exists(self);
        crate::cmd_dict::refresh_profile(self);
        self.active_native_profile = saved;
    }

    pub(crate) fn expression_evaluation_policy(
        &self,
    ) -> Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy> {
        use tcl_registry::native_expression_program::{
            authored_expression_evaluation_policy, native_expression_evaluation_policy,
        };
        let profile = self.native_execution_profile();
        if self.active_native_profile.is_none()
            && let Some(parser) = self.logical_providers.expression_parse
            && let Some(mut policy) = authored_expression_evaluation_policy(
                profile,
                parser,
                self.numeric_policy_context().simulation,
            )
        {
            policy.authored_functions = self.logical_providers.math_functions;
            return Some(policy);
        }
        let point = self.actual_native_invocation_dialect().execution_point()?;
        native_expression_evaluation_policy(profile, point)
    }
}
