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

//! The world an interpreter is pinned to: the host, the release and dialect profile, the logical providers, the runtime context and the system encoding.

use super::super::{native_operation_currency, Code, Interp};
use super::construction::install_ambient_release;
#[cfg(test)]
use crate::interp::constructor_diagnostics::ConstructorDiagnostics;
#[cfg(test)]
use crate::interp::constructor_diagnostics::Kind;
use crate::obj::TclObj;
use crate::{obj, parse};
use std::rc::Rc;

impl Interp {
    // capability host

    /// The capability host (filesystem/`env`/`clock`/subprocess seam). Returns an
    /// independent `Rc` handle, not a borrow, so a command can hold the host
    /// while still taking `&mut self` for its `ValueOps` (e.g. the `exec`
    /// adapter, which needs both at once).
    #[must_use]
    pub(crate) fn host(&self) -> Rc<dyn tcl_platform::Host> {
        self.0.host.borrow().clone()
    }

    /// Swap the capability host (e.g. a test installing a sandboxed,
    /// no-subprocess host to prove the capability gate, or a safe interp taking
    /// a restricted one). Interior-mutable since the interp is shared via `Rc`.
    pub fn set_host(&self, host: Rc<dyn tcl_platform::Host>) {
        self.invalidate_native_operation_context();
        self.invalidate_interpreter_policy();
        let previous = std::mem::replace(&mut *self.0.host.borrow_mut(), host.clone());
        let mut interp = self.clone();
        let scope = match native_operation_currency::NativeOperationScope::enter(&interp) {
            Ok(scope) => scope,
            Err(first) => {
                interp.refuse_native_execution(first);
                drop(previous);
                return;
            }
        };
        drop(previous);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        let system_encoding = host.system_encoding();
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        self.channels
            .borrow()
            .reset_process_state_if_owner(self.runtime_version(), system_encoding);
        if scope.currency().ensure_current_or_refuse().is_err()
            || interp.rebootstrap_host_globals().is_err()
        {
            return;
        }
        if interp.is_safe.get() {
            interp.scrub_host_globals_for_safe();
        }
    }

    // emulated Tcl release

    /// Pin the Tcl release this interpreter emulates.
    ///
    /// Mirrors [`tcl_vm::Vm::set_runtime_version`]'s contract for the
    /// tree-walking runtime: every release-dependent *semantic* is derived
    /// from this one value rather than being set independently, so the two
    /// engines cannot drift apart by having one of them updated and not the
    /// other. Today that is the numeric-literal grammar (see
    /// [`install_ambient_release`]), the namespace-scope variable fallback
    /// (TIP 278), and the release-reporting globals.
    ///
    /// The fallback is a property of the **namespace table**, which every
    /// interpreter owns privately, so a child (`interp create`) and a safe
    /// interpreter each resolve against their own global namespace — setting
    /// it here never reaches across an interpreter boundary.
    pub fn set_runtime_version(&mut self, version: tcl_dialect::TclVersion) {
        // A bare release pin is the matching plain-Tcl profile: the emulated
        // release is one fact carrying the runtime semantics, the lexing
        // grammar, and the command-surface availability mask. The release
        // name is a dialect *name*, so it resolves through the one ingress
        // seam (`crate::environment`) rather than through `by_name`.
        self.set_dialect_profile(crate::environment::profile_for_dialect(
            version.dialect_name(),
        ));
    }

    /// Install an independently authored script-object evaluation recipe.
    /// The separately selected actual host supplies physical materialisation.
    #[must_use]
    pub fn set_logical_eval_object_provider(
        &mut self,
        provider: tcl_registry::native_eval_object::LogicalEvalObjectProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || self
                .native_invocation_dialect()
                .eval_object_protocol(Some(provider))
                .is_none()
        {
            return false;
        }
        if !self
            .logical_eval_object_provider
            .get()
            .is_some_and(|(installed, host)| {
                installed == provider && host.cache_key() == actual_host.cache_key()
            })
        {
            self.invalidate_interpreter_policy();
            self.invalidate_native_operation_context();
            self.logical_eval_object_provider
                .set(Some((provider, actual_host)));
        }
        true
    }

    pub(in super::super) fn eval_object_protocol(
        &self,
    ) -> Option<tcl_registry::native_eval_object::NativeEvalObjectProtocol> {
        self.native_invocation_dialect()
            .invocation_eval_object_protocol(
                self.logical_eval_object_provider
                    .get()
                    .map(|(provider, _)| provider),
            )
    }

    pub(crate) fn eval_frame_dialect(&self) -> tcl_registry::InvocationDialect {
        let dialect = self.native_invocation_dialect();
        if dialect.native_eval_object_protocol().is_some() {
            return dialect;
        }
        self.logical_eval_object_provider.get().map_or_else(
            || dialect,
            |(_, host)| tcl_registry::InvocationDialect::of_profile(host),
        )
    }

    pub(in super::super) fn eval_object_bytes(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        let check = || {
            self.check_entered_native_operation().map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "entered original script-object String getter",
                )
            })?;
            obj::check_native_liveness(original)
        };
        check()?;
        let _original_lifetime = obj::NativeObjectLifetime::retain(original);
        let dialect = self.eval_frame_dialect();
        let protocol = dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native script-object string materialisation",
            ),
        )?;
        let host = self.host();
        // Only C8.4's integer updater reaches the host formatter.
        let formatter = (protocol
            == tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
        .then(|| host.native_integer_formatter())
        .flatten();
        check()?;
        let result =
            crate::dict::native_object_bytes_with_integer_formatter(original, protocol, formatter);
        // Preserve reached getter effects and its first refusal even on Err.
        check()?;
        result
    }

    /// Install source-word materialisation independently of physical object
    /// conversion and expression/name providers. The host engine is explicit.
    #[must_use]
    pub fn set_logical_source_word_provider(
        &mut self,
        provider: tcl_registry::invocation_words::LogicalSourceWordProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || self
                .native_invocation_dialect()
                .logical_source_string_protocol(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        let capability = Some((provider, actual_host.cache_key()));
        if self.logical_source_word_provider.get() != capability {
            self.invalidate_interpreter_policy();
            self.invalidate_native_operation_context();
            self.logical_source_word_provider.set(capability);
        }
        true
    }

    pub(crate) fn source_string_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        let dialect = self.native_invocation_dialect();
        dialect
            .native_source_string_protocol()
            .filter(|protocol| protocol.escape_syntax() == self.lexer_config().escapes)
            .or_else(|| {
                self.logical_source_word_provider
                    .get()
                    .and_then(|(provider, _)| {
                        dialect.logical_source_string_protocol(provider, self.dialect_profile())
                    })
            })
    }

    /// Install the embedding host's authored logical parser capability with
    /// an independently selected actual host engine. Compatibility versions
    /// and command availability cannot supply that host receipt.
    #[must_use]
    pub fn set_logical_expression_parse_provider(
        &mut self,
        provider: tcl_registry::invocation_words::LogicalExpressionParseProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        let dialect = tcl_registry::InvocationDialect::of_profile(self.dialect_profile());
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || dialect
                .logical_expression_parse_context(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        let capability = Some((provider, actual_host.cache_key()));
        if self.logical_expression_parse_provider.get() != capability {
            self.invalidate_interpreter_policy();
            self.invalidate_native_operation_context();
            self.logical_expression_parse_provider.set(capability);
        }
        true
    }

    pub(crate) fn logical_expression_parse_policy(
        &self,
    ) -> Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )> {
        self.logical_expression_parse_provider
            .get()
            .filter(|(provider, _)| {
                self.native_invocation_dialect()
                    .logical_expression_parse_context(*provider, self.dialect_profile())
                    .is_some()
            })
    }

    pub(crate) fn expression_evaluation_policy(
        &self,
    ) -> Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy> {
        use tcl_registry::native_expression_program::{
            authored_expression_evaluation_policy, native_expression_evaluation_policy,
        };
        if let Some((provider, _)) = self.logical_expression_parse_policy() {
            return authored_expression_evaluation_policy(self.dialect_profile(), provider, None);
        }
        let dialect = self.native_invocation_dialect();
        let point = dialect.execution_point()?;
        let profile = if tcl_registry::InvocationDialect::of_profile(self.dialect_profile())
            .execution_point()
            == Some(point)
        {
            self.dialect_profile()
        } else {
            tcl_dialect::DialectProfile::find(dialect.tcl_version?.dialect_profile_name())?
        };
        native_expression_evaluation_policy(profile, point)
    }

    #[cfg(have_tommath)]
    pub(crate) fn expression_parse_context(&self) -> tcl_syntax::expr::parser::ExprParseContext {
        let dialect = self.native_invocation_dialect();
        self.logical_expression_parse_policy()
            .and_then(|(provider, _)| {
                dialect.logical_expression_parse_context(provider, self.dialect_profile())
            })
            .unwrap_or_else(|| dialect.expression_parse_context(Some(self.dialect_profile())))
    }

    /// Pin the dialect profile this interpreter emulates — the profile form
    /// of [`Self::set_runtime_version`], for hosts whose dialect is a vendor
    /// profile rather than a plain Tcl release. The runtime version follows
    /// the profile's pinned `vm_runtime_version`, scripts are lexed with the
    /// profile's grammar, and the profile's availability mask becomes the
    /// builtin command-surface filter.
    pub fn set_dialect_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        #[cfg(test)]
        let mut constructor_diagnostic = ConstructorDiagnostics::new(Kind::Pin);
        self.install_pin(tcl_registry::model::PinnedContext::for_profile(profile));
        #[cfg(test)]
        constructor_diagnostic.stage("profile_installed");
    }

    /// Pin the world this interpreter runs in: the environment, the release
    /// point within it, the build, the package floors and the registry overlay
    /// generation, resolved through the same ingress the compiler uses. The
    /// profile the environment resolves to is what
    /// [`Self::set_dialect_profile`] would pin, and the generation at the
    /// context's overlay is held for as long as the pin stands.
    ///
    /// # Errors
    ///
    /// [`PinError`](tcl_registry::model::PinError) when the ingress does not
    /// agree with the context — no such environment, a release or build that
    /// is not the environment's point, or an overlay nothing has installed,
    /// which is an error and never the un-overlaid generation under another
    /// name. The pin is unchanged.
    pub fn pin_context(
        &mut self,
        context: &tcl_runtime_api::RuntimeContext,
    ) -> Result<(), tcl_registry::model::PinError> {
        self.install_pin(crate::environment::pin_context(context)?);
        Ok(())
    }

    /// The world this interpreter is pinned to.
    #[must_use]
    pub fn runtime_context(&self) -> tcl_runtime_api::RuntimeContext {
        tcl_runtime_api::RuntimeContext::clone(&self.0.pin.borrow().context)
    }

    /// The pinned context itself, retained without copying; a later pin
    /// installs a new allocation, so pointer equality detects no change.
    pub(crate) fn shared_runtime_context(&self) -> std::sync::Arc<tcl_runtime_api::RuntimeContext> {
        std::sync::Arc::clone(&self.0.pin.borrow().context)
    }

    /// The identity this interpreter states of itself — its pinned context,
    /// this build's ABI, intrinsic table and embedded library, and no pack
    /// facts — in the shape a compiled artefact states its own.
    #[must_use]
    pub fn held_identity(&self) -> tcl_runtime_api::ArtefactIdentityManifest {
        self.0.pin.borrow().identity().clone()
    }

    pub(in super::super) fn install_pin(&mut self, pin: tcl_registry::model::PinnedContext) {
        let context_changed = self.0.pin.borrow().context != pin.context;
        let profile = pin.profile;
        if context_changed || !std::ptr::eq(self.dialect_profile(), profile) {
            self.invalidate_native_operation_context();
        }
        *self.0.pin.borrow_mut() = pin;
        let version = profile.vm_runtime_version;
        // Ahead of the unchanged-profile short-circuit: the numeric grammar is
        // *thread*-ambient, not per-interp, so "this interp already emulates
        // `version`" does not imply the thread's grammar is this interp's. A
        // second interpreter constructed on a thread where an earlier one
        // installed 8.4 must re-install its own release even when its version
        // field needs no change.
        install_ambient_release(version);
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        parse::install_native_list_policy(
            dialect.lexer_grammar.list_parse,
            dialect.lexer_grammar.escapes,
        );
        if let Some(policy) = profile.double_string_policy() {
            obj::install_double_string_policy(policy);
        }
        if std::ptr::eq(self.dialect_profile(), profile) {
            if context_changed {
                self.invalidate_interpreter_policy();
            }
            return;
        }
        self.invalidate_interpreter_policy();
        self.0.dialect_profile.set(profile);
        self.0.invocation_dialect.set(None);
        self.0.invocation_protocols.set(None);
        if dialect.return_options_protocol()
            == Some(tcl_cmd_core::return_options::ReturnOptionsProtocol::Jim084)
        {
            self.return_level.set(0);
            self.return_code.set(Code::Ok);
        }
        self.refresh_native_math_function_table();
        self.0
            .profile_registry
            .set((!profile.is_fallback()).then(|| crate::environment::store_for_profile(profile)));
        self.0
            .dialect_point
            .set(Some(crate::environment::surface_point(profile)));
        self.0.runtime_version.set(version);
        self.0.invocation_dialect.set(None);
        self.0.invocation_protocols.set(None);
        self.channels
            .borrow()
            .reset_standard_channels_if_owner(version);
        self.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        self.configure_variable_storage();
        self.namespaces.borrow_mut().set_native_command_version(
            self.native_invocation_dialect()
                .native_command_name_protocol()
                .map(|protocol| protocol.version()),
        );
        if let Err(error) = self.publish_original_jim_registration_keys() {
            self.report_cmd_error(error.into());
            return;
        }
        if self.configure_native_error_stack_checked().is_err() {
            return;
        }
        self.write_release_globals();
        self.install_native_precision_trace();
        // `package provide Tcl` is a release fact, not a runtime constant, and
        // the pre-provided entries were written against the *previous* pin —
        // re-derive them.
        match self.native_invocation_dialect().native_package_protocol() {
            Some(tcl_registry::native_package::NativePackageProtocol::C(release)) => {
                self.packages.borrow_mut().provide_core(release)
            }
            Some(tcl_registry::native_package::NativePackageProtocol::Jim084) | None => {
                self.packages.borrow_mut().clear_core()
            }
        }
        self.refresh_stock_ensembles();
        self.install_jim_local_commands();
        crate::cmd_proc::install_stock_scripted_wrappers(self);
        // Scripted wrapper installation changes the command generation epoch.
        // Re-attest only the actual untouched stock String allocation and map.
        self.attest_stock_string_implementation();
        self.seal_native_compiler_attachments();
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            match self.native_jim_object_context() {
                Ok(context) => self.set_result(context.empty_object().as_ptr()),
                Err(error) => {
                    self.report_cmd_error(error.into());
                }
            }
        }
    }

    /// The Tcl release this interpreter emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.0.runtime_version.get()
    }

    /// The mutable `encoding system` value shared by this interpreter tree.
    #[must_use]
    pub(crate) fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.channels.borrow().system_encoding()
    }

    pub(crate) fn default_system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.channels.borrow().default_system_encoding()
    }

    /// Replace the system encoding used to initialise subsequently opened
    /// channels. Existing channel handles retain their own configuration.
    pub(crate) fn set_system_encoding(&self, encoding: tcl_platform::SystemEncoding) {
        self.channels.borrow().set_system_encoding(encoding);
    }

    /// The dialect profile this interpreter validates its command surface
    /// against (see [`Self::set_dialect_profile`]).
    #[must_use]
    pub fn dialect_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.0.dialect_profile.get()
    }
}
