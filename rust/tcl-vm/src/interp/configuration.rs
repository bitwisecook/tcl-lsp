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

//! What an interpreter is configured with: the pinned release and dialect profile, logical providers, the command surface, native protocols, the host and compiler, debug hooks and channels.

use super::{NamespacePath, ROOT_INTERP, Vm};
use crate::compiled::{CompiledUnit, CompilerProvenance};
#[cfg(test)]
use std::io::Write;
use std::rc::Rc;
use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{ArtefactIdentityManifest, RuntimeContext};

mod command_surface;
mod host_integration;
mod profiles_and_protocols;
mod providers;

impl Vm {
    /// Actual owner and current interpreter identity for retained object bridges.
    /// A bridge must authenticate its private carrier before using these values.
    #[must_use]
    pub fn native_object_bridge_identity(&self) -> (u64, u64) {
        (
            self.owner_nonce,
            u64::try_from(self.cur.0).expect("interpreter slot fits u64"),
        )
    }

    /// Unit-test shorthand for [`crate::test_support::native_vm`]: a
    /// native-core Tcl 9.0 interpreter writing `puts` output to stdout.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new() -> Self {
        crate::test_support::native_vm()
    }

    /// Unit-test shorthand for [`crate::test_support::native_vm_with_output`].
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_output(out: Box<dyn Write>) -> Self {
        crate::test_support::native_vm_with_output(out)
    }

    /// Pin the Tcl release this VM's number/expr grammar emulates —
    /// callers thread `DialectProfile::vm_runtime_version` here, and the
    /// release-reporting globals (`tcl_version` / `tcl_patchLevel`) are
    /// re-derived immediately. The version vocabulary owns the release
    /// predicates used by the VM, so individual command implementations do
    /// not interpret a dialect or version name themselves.
    /// Install *this* interpreter's numeral grammar before it executes.
    ///
    /// The grammar is a property of the interpreter — it emulates exactly one
    /// release for its whole life, as C settles with a build-time `KILL_OCTAL`.
    /// The store backing it is thread-local, though, and one thread can hold
    /// **several** interpreters: a host that builds a 9.0 `Vm` beside a live 8.6
    /// one would otherwise retune the 8.6 one, so `expr {010}` in it started
    /// answering 10 instead of 8. Installing at construction alone is not
    /// enough — that makes the answer depend on which interpreter was built
    /// last.
    ///
    /// Claiming it at every execution entry (`run_module`, `eval_source`,
    /// `eval_expr`) makes each interpreter's own grammar hold for its own work,
    /// whatever else shares the thread, and re-entrancy is safe because the
    /// inner interpreter re-claims on the way in and the outer one re-claims on
    /// its next entry. Pinned by
    /// `a_second_interpreter_does_not_change_the_first_ones_grammar`.
    pub(crate) fn claim_number_grammar(&self) {
        tcl_syntax::number::set_runtime_syntax(self.runtime_version.number_syntax());
        tcl_syntax::number::set_runtime_release(self.runtime_version);
    }

    pub fn set_runtime_version(&mut self, version: tcl_dialect::TclVersion) {
        // A bare release pin is the matching plain-Tcl profile: the emulated
        // release is one fact carrying both the runtime semantics and the
        // command-surface availability point. The release name
        // is a dialect *name*, so it resolves through the one ingress seam
        // (`crate::environment`) rather than through `by_name`.
        self.set_dialect_profile(crate::environment::profile_for_dialect(
            version.dialect_profile_name(),
        ));
    }

    /// Pin the dialect profile this VM emulates — the profile form of
    /// [`Self::set_runtime_version`], for hosts whose dialect is a vendor
    /// profile rather than a plain Tcl release. The runtime version follows
    /// the profile's pinned `vm_runtime_version`, and the profile's
    /// availability point becomes the builtin command-surface filter
    /// ([`Self::builtin_command_visible_for_surface`]).
    pub fn set_dialect_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.install_pin(tcl_registry::model::PinnedContext::for_profile(profile));
    }

    /// Pin the world this VM runs in: the environment, the release point within
    /// it, the build, the package floors and the registry overlay generation,
    /// resolved through the same ingress the compiler uses. The profile the
    /// environment resolves to is what [`Self::set_dialect_profile`] would pin,
    /// and the generation at the context's overlay is held for as long as the
    /// pin stands.
    ///
    /// A compiled unit states the context it was compiled for, and is admitted
    /// only at the rungs whose fields agree with this one. A pin made from a
    /// native command takes effect at the first command boundary after that
    /// command returns: the command finishes under the context it entered with.
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
        context: &RuntimeContext,
    ) -> Result<(), tcl_registry::model::PinError> {
        let pin = crate::environment::pin_context(context)?;
        if self.native_command_depth > 0 {
            self.deferred_pin = Some((pin, self.native_command_depth));
        } else {
            self.deferred_pin = None;
            self.install_pin(pin);
        }
        Ok(())
    }

    /// Install a pin [`Self::pin_context`] deferred, once the native command
    /// that made it has returned, and re-issue the operations entered under
    /// the old context to the new one. Each running function is then checked
    /// against its manifest at its next command boundary, the deopt epoch the
    /// pin advances saying so, and finishes plain if it no longer fits.
    pub(crate) fn install_deferred_pin(&mut self) {
        if self
            .deferred_pin
            .as_ref()
            .is_some_and(|(_, depth)| self.native_command_depth < *depth)
            && let Some((pin, _)) = self.deferred_pin.take()
        {
            self.install_pin_reissuing(pin, true);
        }
    }

    /// The world this VM is pinned to.
    #[must_use]
    pub fn runtime_context(&self) -> &RuntimeContext {
        &self.pin.context
    }

    /// The identity this VM holds — its pinned context and the pack facts it
    /// was given — in the shape a compiled artefact states its own, which a
    /// unit's manifest is compared with.
    #[must_use]
    pub fn held_identity(&self) -> &ArtefactIdentityManifest {
        self.pin.identity()
    }

    /// The pin this VM stands on, for an interpreter built to share its world.
    pub(crate) fn pinned_context(&self) -> tcl_registry::model::PinnedContext {
        self.pin.clone()
    }

    /// Stand on `pin`, a pin another interpreter of this build already holds.
    pub(crate) fn inherit_pin(&mut self, pin: tcl_registry::model::PinnedContext) {
        self.install_pin(pin);
    }

    pub(super) fn install_pin(&mut self, pin: tcl_registry::model::PinnedContext) {
        self.install_pin_reissuing(pin, false);
    }

    /// Install `pin`; with `reissue`, the operations entered under the old
    /// context are moved to it before anything runs under it.
    pub(super) fn install_pin_reissuing(
        &mut self,
        mut pin: tcl_registry::model::PinnedContext,
        reissue: bool,
    ) {
        let profile = pin.profile;
        let profile_changed = !std::ptr::eq(self.pin.profile, profile);
        if profile_changed
            || pin.context != self.pin.context
            || !std::ptr::eq(self.command_surface_profile, profile)
        {
            self.advance_native_operation_context();
        }
        pin.restate(&self.pack_facts);
        if self.native_core_pin.as_ref().is_some_and(|core| {
            std::ptr::eq(core.profile, profile)
                && self
                    .actual_engine_profile
                    .is_some_and(|engine| std::ptr::eq(engine, profile))
        }) {
            self.native_core_pin = Some(pin.clone());
        }
        if pin.context != self.pin.context {
            // A unit admitted under the old context is checked again at its
            // next entry.
            self.bump_trace_deopt_epoch();
        }
        self.pin = pin;
        if reissue {
            self.reissue_entered_native_operations();
        }
        // The 8.4 `namespace path` tier gate (M10.1) and the availability
        // gate change resolution outcomes, so the command-resolution memo
        // (M16.4) must not survive a version flip.
        self.bump_cmd_epoch();
        if profile_changed {
            self.profile_generation = self.profile_generation.wrapping_add(1);
            // Dynamic modules and their precompiled-proc sidecars may contain
            // profile-sensitive lowerings. ProcDef retains source and is
            // rebuilt lazily; these cache-only forms do not, so discard them.
            // Active frames carry this generation and fail closed at the
            // trampoline seam if a native callback changes profile mid-run.
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        self.command_surface_profile = profile;
        self.command_surface_point = Some(crate::environment::surface_point(profile));
        self.profile_registry =
            (!profile.is_fallback()).then(|| crate::environment::store_for_profile(profile));
        self.runtime_version = profile.vm_runtime_version;
        let recipe = self
            .actual_native_invocation_dialect()
            .native_error_objects_protocol();
        self.native_errors.error_stack.configure(recipe);
        if let Err(error) = self.configure_c84_execution_environment_error_names() {
            let _ = crate::command::completion_from_cmd_error(self, error);
        }
        self.select_dictionary_containers();
        self.install_native_precision_trace();
        // Standard channels are VM-wide handles shared by the interpreter
        // tree. Only a real profile change on the root establishes new
        // release defaults: pinning a child must not replace the parent's
        // live `fconfigure` state, and re-pinning the root to the same profile
        // is not a channel lifecycle event.
        if self.cur == ROOT_INTERP && profile_changed {
            self.reset_standard_channel_configs();
        }
        // Install the release's numeric grammar for this runtime: `0755` is 493
        // under 8.6 and 755 under 9.0, `0b`/`0o` exist from 8.5 and `0d` / `_`
        // separators from 9.0. C settles this at build time (`KILL_OCTAL`), so
        // it is a property of the runtime rather than of each conversion — see
        // `tcl_syntax::number::set_runtime_syntax`. Embedders call this while
        // building the interpreter; the VM does not support switching release
        // mid-execution.
        self.claim_number_grammar();
        self.write_release_globals();
        // `package provide Tcl` is a release fact, not an engine constant, and
        // the pre-provided entries were written against the *previous* pin, so
        // this re-derives them. Guarded because the very first
        // profile pin happens inside `register_builtins`, before the `package`
        // command itself is registered; that call site provides them directly.
        if self.commands.contains_key("package") {
            crate::cmd_package::provide_core_packages(self);
        }
        if profile_changed {
            self.refresh_actual_stock_builtins();
        }
        if let Err(error) = self.initialize_original_jim_host_keys() {
            let _ = self.refuse_host_command(error.to_string());
        }
    }

    /// The Tcl release this VM emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.runtime_version
    }

    /// The selected C Tcl release's command-kernel grammars (`scan`, `clock`,
    /// `binary`); another family keeps each kernel's default grammar.
    #[must_use]
    pub(crate) fn kernel_release_policy(
        &self,
    ) -> Option<&'static tcl_dialect::release_policy::ReleasePolicyFacts> {
        (self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl))
            .then(|| self.runtime_version.release_policy())
    }

    /// The dialect profile this VM validates its command surface against
    /// (see [`Self::set_dialect_profile`]).
    #[must_use]
    pub fn dialect_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.pin.profile
    }

    /// The release's `${…}` close rule — `Tcl_ParseVarName`'s brace-form
    /// delimiting, which the 8.x family and 9.x disagree about.
    ///
    /// Read from the same `DialectProfile::grammar` the compile path's
    /// `LexerConfig` comes from, so the interpreted `subst` engine and the
    /// compiled word path cannot answer `${a{b}c}` differently.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn braced_var_style(&self) -> tcl_dialect::BracedVarStyle {
        self.source_profile().grammar.braced_var
    }

    /// The whole lexer grammar this VM emulates, as one `LexerConfig`.
    ///
    /// The pinned profile's `LexerGrammar` is the *same* value the compile
    /// path lexes the document under, so every re-read of script text on the
    /// interpreted side (`subst`, a compiled word, a `[…]` body) reads it
    /// under that one grammar rather than assembling a few axes over the
    /// default (`docs/design/registry/dialect-profile-model.md` §2.5). Picking axes
    /// one at a time is how a Jim VM kept C's `$(…)` reading and an iRules
    /// VM kept C's `}{`.
    #[must_use]
    pub(crate) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(self.source_profile().grammar)
    }

    /// Query original Document completeness under this VM's full source configuration.
    /// This retains the Document newline channel and creates no native value,
    /// command selection or evaluation entry. The String worker independently
    /// uses its actual selected invocation grammar.
    ///
    /// # Errors
    /// Returns unavailable selected grammar or original Document geometry.
    pub fn source_is_complete(
        &self,
        source: &str,
    ) -> Result<bool, tcl_syntax::script_completeness::ScriptCompletenessUnavailable> {
        tcl_syntax::script_completeness::script_image_is_complete(
            &tcl_lexer::SourceImage::document(source),
            self.lexer_config(),
        )
    }

    /// Generation of the dialect profile used to compile dynamic bytecode.
    #[must_use]
    pub(crate) fn profile_generation(&self) -> u64 {
        self.profile_generation
    }

    #[must_use]
    pub(crate) fn compiler_generation(&self) -> u64 {
        self.compiler_generation
    }

    pub(crate) fn compiled_unit(
        &mut self,
        asm: Rc<FunctionAsm>,
        source_namespace: NamespacePath,
    ) -> CompiledUnit {
        let literal_pool = self.create_native_literal_pool(&asm, &source_namespace);
        self.unentered_compiled_unit(
            asm,
            source_namespace,
            CompilerProvenance::CurrentService(self.compiler_generation),
        )
        .with_literal_pool(literal_pool)
        .with_source_location(self.script_stack.last().map(|file| {
            tcl_runtime_api::script_source_location::ScriptSourceLocation {
                file: file.clone(),
                line: 1,
            }
        }))
    }

    /// Retain an assembly candidate without executing its literal registrations.
    /// A procedure cache entry becomes a native local object array only when
    /// that exact artifact is selected for an actual procedure body.
    pub(super) fn unentered_compiled_unit(
        &self,
        asm: Rc<FunctionAsm>,
        source_namespace: NamespacePath,
        compiler: CompilerProvenance,
    ) -> CompiledUnit {
        let native_cache = self.native_cache_stamp_for_source_namespace(&source_namespace);
        let fixed_math_calls = self.retain_native_fixed_math_calls(&asm);
        let mut unit = CompiledUnit::new(
            asm,
            source_namespace,
            self.profile_generation,
            self.trace_deopt_epoch(),
            native_cache.as_ref(),
            self.native_interpreter_identity(),
            compiler,
        );
        unit.fixed_math_calls = fixed_math_calls.map(Rc::new);
        unit
    }

    /// Admit embedder-owned assembly for this execution without claiming that
    /// the current compile service produced it. Source-bearing children retain
    /// this marker and are recompiled through the service before reuse.
    pub(crate) fn admitted_foreign_unit(
        &mut self,
        asm: Rc<FunctionAsm>,
        source_namespace: NamespacePath,
    ) -> CompiledUnit {
        let literal_pool = self.create_native_literal_pool(&asm, &source_namespace);
        self.unentered_compiled_unit(
            asm,
            source_namespace,
            CompilerProvenance::AdmittedForeign(self.compiler_generation),
        )
        .with_literal_pool(literal_pool)
    }

    /// The backslash-escape grammar this VM decodes under — the pinned
    /// profile's, so a VM emulating 8.5 reads `\x4142` as `B` and one
    /// emulating 9.0 reads it as `A42`.
    #[must_use]
    pub fn escape_syntax(&self) -> tcl_dialect::EscapeSyntax {
        self.source_profile().grammar.escapes
    }
}
