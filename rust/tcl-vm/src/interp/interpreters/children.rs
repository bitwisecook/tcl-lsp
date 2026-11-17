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

//! Child interpreters: creating and naming a child, its dialect profile, `interp debug` and `bgerror`, safe interpreters, evaluating in a child and deleting it.

use super::super::commands::{CommandSidecarKey, NativeCommandLookupUnavailable};
use super::super::{
    CommandSlot, InterpId, InterpState, NativeBootstrapPurpose, Vm, authored_math,
    authored_package, authored_rule_callables, authored_tmm_static, err, native_name_consumers,
    native_operation_currency,
};
use crate::command::Command;
use crate::value::Value;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_dialect::PackagePrefer;
use tcl_runtime_api::{Code, Completion, ROOT_NS};

/// `interp debug`'s one option word (`debugTypes[]`, `tclInterp.c`): C resolves
/// it with `Tcl_GetIndexFromObj(…, "debug option", 0)`, so `-f`/`-fr`
/// abbreviate and a miss is `bad debug option "…": must be -frame` — a
/// one-entry table is never `ambiguous`, not even for the empty word.
const DEBUG_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("debug option", &["-frame"]);

impl Vm {
    /// A fresh child interpreter's *bare* state (no builtins yet): its own
    /// command table, namespaces, variables, and channels, but the parent's
    /// output sink, compile service, host, and runtime version (`interp
    /// create` inherits these). Populated by [`Self::create_child`] once it is
    /// in the arena and can be entered.
    pub(in super::super) fn fork_child_state(&self) -> Box<InterpState> {
        let mut child = InterpState::fresh(
            Rc::clone(&self.out),
            self.owner_nonce,
            u64::try_from(self.interps.len()).expect("interpreter slot fits u64"),
            self.bootstrap.clone(),
        );
        child.compiler.clone_from(&self.compiler);
        child.host = Rc::clone(&self.host);
        child.runtime_version = self.runtime_version;
        child.actual_engine_profile = self.actual_engine_profile;
        child.native_core_pin.clone_from(&self.native_core_pin);
        if let Some(core) = &mut child.native_core_pin {
            core.restate(&child.pack_facts);
        }
        child.logical_providers.quote = self.logical_providers.quote;
        child.logical_providers.expression_parse = self.logical_providers.expression_parse;
        child.logical_providers.source_words = self.logical_providers.source_words;
        child.logical_providers.source_file = self.logical_providers.source_file;
        child.logical_providers.eval_object = self.logical_providers.eval_object;
        child.logical_providers.numeric = self.logical_providers.numeric;
        child.logical_providers.math_functions = self.logical_providers.math_functions;
        child.authored_packages = self
            .authored_packages
            .as_ref()
            .map(|state| authored_package::AuthoredPackageState::new(state.provider));
        child.authored_rule_providers = self.authored_rule_providers;
        child.authored_math = self
            .logical_providers
            .math_functions
            .map(|_| authored_math::AuthoredMathState::default());
        child.logical_providers.names = self.logical_providers.names;
        child.logical_providers.compiled_variables = self.logical_providers.compiled_variables;
        child.command_surface_profile = self.command_surface_profile;
        child
            .framework_builtins
            .clone_from(&self.framework_builtins);
        // The pin's three derived facts travel together: the surface
        // profile, the mask its environment gates under, and the
        // generation's store. A child that inherited two of the three would
        // gate the parent's registry under the unpinned permissive mask.
        child.command_surface_point = self.command_surface_point;
        child.profile_registry = self.profile_registry;
        // The pin's context and the registry generation it holds come with
        // the profile: a child that kept the unpinned context would state an
        // identity no module compiled for its profile agrees with.
        child.pin = self.pin.clone();
        child.pin.restate(&child.pack_facts);
        Box::new(child)
    }

    /// Jim's globally named zero-argument interpreter factory.
    pub(crate) fn create_jim_child(&mut self) -> String {
        let name = format!("::interp.handle{}", self.counters.interp_counter);
        self.counters.interp_counter += 1;
        self.create_child(Some(name), false)
    }

    /// `interp create ?-safe? ?name?` — make a child interpreter in the arena,
    /// registering it as a command in the current interp. Returns the (possibly
    /// auto-generated) name.
    ///
    /// A safe parent's children are safe whether or not `-safe` was asked for
    /// (C's `Tcl_CreateChild`), so safety is settled here before the child's
    /// command table is populated and scrubbed.
    pub(crate) fn create_child(&mut self, name: Option<String>, safe: bool) -> String {
        let safe = safe || self.state.is_safe;
        let name = name.unwrap_or_else(|| {
            let n = format!("interp{}", self.counters.interp_counter);
            self.counters.interp_counter += 1;
            n
        });
        self.create_child_named_bytes(NameBytes::from(name.as_bytes()), safe)
            .expect("explicit host child naming protocol");
        name
    }

    fn initialise_child_command_world(
        &mut self,
        id: InterpId,
        safe: bool,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.in_interp(id, |vm| {
            let saved = vm.active_native_profile;
            vm.active_native_profile = vm.actual_engine_profile.or(saved);
            let entered = (|| -> Result<(), tcl_runtime_api::NativeExecutionError> {
                let scope = native_operation_currency::NativeOperationScope::enter_native(vm)?;
                // The child's own error objects follow its engine, as the
                // root's do at construction.
                let recipe = vm
                    .actual_native_invocation_dialect()
                    .native_error_objects_protocol();
                vm.native_errors.error_stack.configure(recipe);
                vm.configure_c84_execution_environment_error_names()
                    .map_err(|error| {
                        let _ = crate::command::completion_from_cmd_error(vm, error);
                        vm.execution_refusal
                            .clone()
                            .expect("C execution environment refusal")
                    })?;
                let NativeBootstrapPurpose { protocol, inputs } = vm.bootstrap.clone();
                crate::command::register_builtins_with_native_core(vm, protocol);
                scope.check_native(vm)?;
                vm.bootstrap_native_core(protocol, &inputs);
                scope.check_native(vm)?;
                if protocol
                    .child_initialisation()
                    .initialises_jim_static_extensions()
                {
                    for &library in
                        tcl_registry::native_scripted_distribution::NativeScriptedLibrary::ALL
                    {
                        vm.install_scripted_library(library);
                        scope.check_native(vm)?;
                    }
                }
                scope.check_native(vm)?;
                vm.enable_command_semantics_tracking();
                if vm.authored_timers.installed {
                    crate::retained_activation::register_provider(vm);
                }
                if vm.authored_rule_providers.callables
                    || vm.authored_rule_providers.counted_strings
                {
                    authored_rule_callables::register_provider(vm);
                }
                if vm.authored_packages.is_some() {
                    authored_package::register_provider(vm);
                }
                if vm.authored_tmm_static.policy.is_some() {
                    authored_tmm_static::register_provider(vm);
                }
                for capability in vm.framework_builtins.clone() {
                    vm.install_framework_builtin(capability);
                }
                if safe {
                    vm.make_safe();
                }
                scope.check_native(vm)
            })();
            vm.active_native_profile = saved;
            entered
        })
    }

    pub(in super::super) fn create_child_named_bytes(
        &mut self,
        name: NameBytes,
        safe: bool,
    ) -> Result<InterpId, NativeCommandLookupUnavailable> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let slot = if recipe.is_jim084() {
            self.native_alias_publication_slot(name.as_bytes())?
        } else {
            let path = self.namespace_path_for_token(self.current_ns_id());
            let selected = recipe
                .command_c_api_publication_projection(
                    tcl_syntax::naming::NativeNameContext::new(&path),
                    name.as_bytes(),
                )
                .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            let namespace =
                self.materialise_native_command_projection(self.current_ns_id(), &selected);
            CommandSlot {
                namespace,
                simple: selected.into_slot().simple,
            }
        };
        let safe = safe || self.is_safe();
        let parent = self.cur;
        let child = self.fork_child_state();
        {
            let protocol = &child.bootstrap.protocol;
            let actual = child.actual_engine_profile.and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).native_bootstrap_protocol()
            });
            if actual != Some(*protocol) {
                return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
            }
        }
        let id = self.new_interp_slot(child, parent);
        // The child replays its constructor's inputs in its own command world.
        let entered = self.initialise_child_command_world(id, safe);
        if let Err(first) = entered {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first));
            self.retire_interp(id);
            return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
        }
        if let Some(initialisation) = self
            .actual_native_invocation_dialect()
            .native_bootstrap_protocol()
            .map(tcl_registry::special_vars::NativeBootstrapProtocol::child_initialisation)
            .filter(|initialisation| initialisation.initialises_jim_static_extensions())
            && let Err(error) = self.copy_original_jim_child_startup_variables(id, initialisation)
        {
            let _ = crate::command::completion_from_cmd_error(self, error);
            self.retire_interp(id);
            return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
        }

        // The interp *path* name keys `children` as written (interp names are
        // their own universe, never namespace-parsed); the command it creates
        // in this interp is an ordinary command name, so its key qualifies.
        self.children.insert(name, id);
        self.register_command_in_slot(slot, Command::ChildInterp(id));
        // C's `CreateChild` runs `Tcl_Init` in a trusted child, so it has
        // `init.tcl`'s `unknown` and `auto_load` (`package.test` loads
        // tcltest into one). The child reads the parent's library.
        // C reads `TCL_PKG_PREFER_LATEST` as each interpreter is created, so
        // a child sees an `env` written since its parent began.
        if self.host_rc().env().get("TCL_PKG_PREFER_LATEST").is_some() {
            self.in_interp(id, |child| {
                child.state.package_prefer = PackagePrefer::Latest;
            });
        }
        if !safe && let Some(library) = self.state.initialised_library.clone() {
            self.in_interp(id, |child| {
                if child.set_var_bytes(b"::tcl_library", library).is_ok() {
                    let _ = child.init_library();
                }
            });
        }
        Ok(id)
    }

    /// The [`InterpId`] of a direct child by name, or `None`.
    pub(crate) fn child_id(&self, name: &str) -> Option<InterpId> {
        self.children.get(name.as_bytes()).copied()
    }

    /// Locate the exact command token that currently names `child`. Child
    /// interpreter paths and Tcl command locations are separate identity
    /// domains: rename/hide can move the command without changing the child
    /// arena token, and deferred namespace deletion can retain it.
    fn child_command_sidecar_key(&self, child: InterpId) -> Option<CommandSidecarKey> {
        self.commands
            .iter()
            .find_map(|(key, command)| {
                matches!(command, Command::ChildInterp(id) if *id == child)
                    .then(|| CommandSidecarKey::visible(key))
            })
            .or_else(|| {
                self.hidden.commands.iter().find_map(|(key, command)| {
                    matches!(command, Command::ChildInterp(id) if *id == child)
                        .then(|| CommandSidecarKey::hidden(key))
                })
            })
            .or_else(|| {
                self.name_world
                    .borrow()
                    .ns_deferral
                    .retained
                    .values()
                    .find_map(|record| {
                        record.commands.iter().find_map(|(key, command)| {
                            matches!(command, Command::ChildInterp(id) if *id == child)
                                .then(|| CommandSidecarKey::visible(key))
                        })
                    })
            })
    }

    /// Select the dialect profile of a direct child interpreter. This is the
    /// host-side counterpart to configuring a standalone [`Vm`]: child state
    /// is parked in the interpreter arena, so route through `in_interp` rather
    /// than mutating a copied state and thereby skipping its bytecode-cache
    /// invalidation owner.
    pub fn set_child_dialect_profile(
        &mut self,
        name: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        let Some(id) = self.child_id(name) else {
            return false;
        };
        self.in_interp(id, |child| child.set_dialect_profile(profile));
        true
    }

    /// `interp debug ?-frame ?bool??` on this interp. `-frame` is a one-way
    /// switch (once on, stays on); returns the settings list / the frame bool.
    ///
    /// C checks the argument count before the option word, so
    /// `interp debug i -x 1 2` is `wrong # args`, not `bad debug option`;
    /// `usage` is the caller's own `should be` text, which differs between the
    /// `interp debug path …` and `$child debug …` spellings.
    pub(crate) fn debug_apply(
        &mut self,
        args: &[Value],
        usage: impl AsRef<[u8]>,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let usage = usage.as_ref();
        crate::expr::with_original_numeric_command(
            self,
            tcl_cmd_core::CmdError::from_execution_refusal,
            |vm, context| vm.debug_apply_in_original(args, usage, context),
        )
    }

    fn debug_apply_in_original(
        &mut self,
        args: &[Value],
        usage: &[u8],
        context: crate::expr::OriginalNumericCommandContext<'_>,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        let (opt, rest) = match args {
            [] => {
                return Ok(Value::list(vec![
                    Value::string("-frame"),
                    Value::int(i64::from(self.debug_frame)),
                ]));
            }
            [opt, rest @ ..] if rest.len() <= 1 => (opt, rest),
            _ => return Err(tcl_cmd_core::CmdError::wrong_args_bytes(usage)),
        };
        self.native_static_option_index_checked(opt, DEBUG_OPTIONS.names(), false, "debug option")?;
        context.current(self)?;
        if let [value] = rest {
            let enabled = if let Some(authored) = context.authored {
                let reached = value.as_bool_with_stages(Some(authored));
                context.current(self)?;
                reached.map_err(|error| {
                    crate::expr::numeric_command_guest_error(self, context, error)
                })?
            } else {
                crate::expr::original_numeric_command_ops(
                    self,
                    context,
                    tcl_registry::native_numeric_error::NativeNumericCommandPurpose::DebugFrame,
                    |original, selected| match original.getter(value, selected.getter_kind())? {
                        tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(boolean) => {
                            Ok(boolean.is_true())
                        }
                        _ => Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "debug Boolean output",
                        )
                        .into()),
                    },
                )?
            };
            context.current(self)?;
            if enabled {
                self.debug_frame = true;
            }
        }
        Ok(Value::int(i64::from(self.debug_frame)))
    }

    /// `interp bgerror ?cmdPrefix?` on this interp — get/set the background-error
    /// handler.
    pub(crate) fn bgerror_apply(&mut self, args: &[Value]) -> Result<Value, Completion<Value>> {
        if let [prefix] = args {
            let valid = match tcl_syntax::value::ValueOps::list_len(self, prefix) {
                Ok(length) => length > 0,
                Err(error) if error.native_access_refusal().is_some() => {
                    return Err(crate::command::completion_from_cmd_error(
                        self,
                        error.into(),
                    ));
                }
                Err(_) => false,
            };
            if !valid {
                let mut detail = tcl_cmd_core::CmdError::new_bytes(
                    b"cmdPrefix must be list of length >= 1".to_vec(),
                )
                .into_byte_details();
                if let Some(code) = self
                    .actual_native_invocation_dialect()
                    .native_event_protocol()
                    .and_then(tcl_registry::native_event::NativeEventProtocol::background_prefix_format_code)
                {
                    detail.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec());
                }
                return Err(crate::command::completion_from_cmd_error(
                    self,
                    tcl_cmd_core::CmdError::from_byte_details(detail),
                ));
            }
            self.bgerror_handler = prefix.clone().into_native_reference();
        }
        Ok(self.bgerror_handler.clone())
    }

    /// Sorted names of this interp's direct children (`interp children`).
    pub(crate) fn child_names(&self) -> Vec<NameBytes> {
        let mut names: Vec<NameBytes> = self.children.keys().cloned().collect();
        names.sort();
        names
    }

    /// `interp issafe ?path?` for the current interp.
    pub(crate) fn is_safe(&self) -> bool {
        self.state.is_safe
    }

    pub(crate) fn mark_interp_trusted(&mut self, id: InterpId) {
        let names: Vec<NameBytes> = self.st_of(id).map_or_default(|state| {
            state
                .hidden
                .tokens_by_name
                .iter()
                .filter(|(_, key)| state.hidden.commands.contains_key(*key))
                .map(|(name, _)| name.clone())
                .collect()
        });
        self.in_interp(id, |vm| {
            vm.bump_cmd_epoch();
            vm.invalidate_lookup_guards();
            for name in names {
                let slot = CommandSlot {
                    namespace: ROOT_NS,
                    simple: name.clone(),
                };
                if vm.command_at_exact_slot(&slot, false).is_none() {
                    vm.expose_own_command_bytes(name.as_bytes(), name.as_bytes())
                        .expect("a non-colliding hidden command exposes");
                }
            }
            vm.is_safe = false;
        });
    }

    /// Make this interp safe (`interp create -safe`): hide the commands that
    /// reach the host (filesystem, processes, sockets, the interpreter loader).
    /// Hidden commands move to `hidden.commands`, invocable via
    /// `interp invokehidden` and restorable with `interp expose`.
    fn make_safe(&mut self) {
        self.bump_cmd_epoch();
        self.invalidate_lookup_guards();
        // The hide list is the registry's `Traits::SAFE_INTERP_HIDDEN` query,
        // not a name list this engine keeps: C's own set is
        // the `CmdInfo` rows lacking `CMD_IS_SAFE` plus the whole-command rows
        // of `unsafeEnsembleCommands`, and that is what the trait records.
        //
        // Narrowed per release by what this interpreter actually carries: a
        // command that is not in the table, or that the pinned surface hides,
        // is not there to be hidden — which is how `unload` (8.5+) and `zipfs`
        // (9.0+) stay out of an older pin's hidden set without a second
        // availability rule. Gating on visibility matters as well as on
        // presence: parking an unavailable builtin in `hidden.commands` would
        // let `interp invokehidden` reach it past the surface check.
        //
        // `after` / `vwait` are correctly absent from the trait — confirmed
        // present and callable inside a real safe child on tclsh 8.6.14
        // (`s eval {info commands after}` returns `after`); an earlier
        // hand-typed list here once hid them, breaking legitimate safe-interp
        // code using `after idle` / `after cancel`.
        for &c in tcl_registry::safe_interp_hidden_commands() {
            if !self
                .commands
                .get(c)
                .is_some_and(|command| self.builtin_command_visible_for_surface(c, command))
            {
                continue;
            }
            let import_origin = self.imported_commands.get(c).cloned();
            let command_generation = self.live_command_generation(c);
            let builtin_identity = self.builtin_identity_for_key(c);
            if let Some(cmd) = self.commands.remove(c) {
                self.note_command_unbound(c);
                self.name_world
                    .borrow_mut()
                    .command_identity
                    .generations
                    .remove(c);
                self.imported_commands.remove(c);
                self.builtin_identities.remove(c);
                let hidden_key = self.reserve_hidden_storage_key(NameBytes::from(c));
                self.hidden.commands.insert(hidden_key, cmd);
                if let Some(generation) = command_generation {
                    self.hidden.generations.insert(c.to_string(), generation);
                }
                if let Some(origin) = import_origin {
                    self.hidden.imports.insert(c.to_string(), origin);
                }
                if let Some(identity) = builtin_identity {
                    self.hidden
                        .builtin_identities
                        .insert(c.to_string(), identity);
                }
            }
        }
        self.scrub_host_globals_for_safe();
        self.is_safe = true;
    }

    pub(in super::super) fn scrub_host_globals_for_safe(&mut self) {
        for key in tcl_platform::bootstrap::safe_scrub_keys() {
            let _ = self.unset_one(&format!("::tcl_platform({key})"), false);
        }
        let _ = self.unset_one("::env", false);
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            let _ = self.unset_one(&tcl_syntax::naming::root_unrooted_key(name), false);
        }
    }

    /// Assemble original script operands with the selected native concat owner,
    /// then evaluate their counted source bytes in the actual target interpreter.
    pub(crate) fn eval_values_in_interp(
        &mut self,
        id: InterpId,
        operands: &[Value],
    ) -> Completion<Value> {
        let scope = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let reached = self.eval_values_in_interp_in_original_name(id, operands, &scope);
        self.original_name_finish(scope, reached)
            .unwrap_or_else(|error| crate::command::completion_from_cmd_error(self, error))
    }

    pub(crate) fn eval_values_in_interp_in_original_name(
        &mut self,
        id: InterpId,
        operands: &[Value],
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Completion<Value> {
        if let Err(error) = original_name_scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }

        if !self.interp_alive(id) {
            return err("could not find interpreter");
        }
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native interpreter script protocol unavailable".to_owned());
        };
        let script = if !policy.recipe().is_jim084() && operands.len() == 1 {
            operands[0].clone()
        } else {
            let reached = tcl_cmd_core::list::concat_selected(self, operands);
            match self.original_name_outcome(original_name_scope, reached) {
                Ok(script) => script,
                Err(error) => return crate::command::completion_from_cmd_error(self, error),
            }
        };
        if policy.recipe().is_jim084() {
            return self.eval_original_jim_child_object(id, &script);
        }
        let bytes = match self.native_name_operand_bytes_checked(&script) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let source = tcl_lexer::SourceImage::native(bytes.as_ref());
        let completion = self.in_interp(id, |vm| {
            let mut completion = match vm.eval_source_image_at_internal(&source, None) {
                Ok(completion) => completion,
                Err(error) => crate::command::completion_from_tcl_error(vm, error),
            };
            if completion.code == Code::Error {
                completion.options = match vm.completion_options_snapshot(&completion) {
                    Ok(options) => options,
                    Err(error) => return crate::command::completion_from_tcl_error(vm, error),
                };
            }
            // C hands the child's result to the caller (`TclTransferResult`)
            // before `Tcl_Release` can delete the child, so the result
            // outlives the child's objects.
            if vm.interp_retires_on_return(id) {
                completion.result = completion.result.into_native_reference();
                completion.options = completion.options.into_native_reference();
            }
            completion
        });
        if let Err(error) = original_name_scope.check(self) {
            drop(completion);
            return crate::command::completion_from_cmd_error(self, error);
        }
        if completion.code == Code::Error {
            let restored = self.restore_completion_error_state(&completion);
            if let Err(error) = original_name_scope.check(self) {
                drop(completion);
                return crate::command::completion_from_cmd_error(self, error);
            }
            if let Err(error) = restored {
                return crate::command::completion_from_tcl_error(self, error);
            }
        }
        completion
    }

    /// `interp delete path …` — destroy interpreter `id`: unhook it from its
    /// parent (the `children` map entry and the command that names it) and tear
    /// it down.  The state drops now, or (if it is still executing — a
    /// re-entered child, a target mid-callback) when its last in-flight
    /// evaluation unwinds; its children and the aliases targeting it are swept
    /// regardless (see [`Self::retire_interp`]).  The root interp cannot be
    /// deleted.  Returns whether the interp existed.
    pub(crate) fn delete_interp(&mut self, id: InterpId) -> bool {
        if !self
            .interps
            .get(id.0)
            .is_some_and(|s| !s.dying && !s.is_root())
        {
            return false;
        }
        if self.interps[id.0].command_dying {
            // The naming command is already being deleted, and its deletion is
            // what retires the interp. Deleting it again only unlinks the name
            // (C's `Tcl_DeleteCommandFromToken` on a dying command): the
            // interp stays in its parent's table until the outer deletion ends.
            if let Some(parent) = self.interps[id.0].parent {
                self.in_interp(parent, |vm| {
                    if let Some(token) = vm
                        .child_command_sidecar_key(id)
                        .and_then(|key| vm.command_token_identity(&key))
                    {
                        vm.unlink_dying_command_name(&token);
                    }
                });
            }
            return true;
        }
        self.retire_authored_static_worker(id);
        // Unhook the child mapping + command in the owning parent (the interp
        // that created it — where the `children` entry and the naming command
        // live), which may or may not be the current interp.
        if let Some(parent) = self.interps[id.0].parent {
            let name = self.st_of(parent).and_then(|st| {
                st.children
                    .iter()
                    .find(|&(_, &v)| v == id)
                    .map(|(k, _)| k.clone())
            });
            if let Some(name) = name {
                self.in_interp(parent, |vm| {
                    let command_key = vm.child_command_sidecar_key(id);
                    if let Some(key) = command_key {
                        vm.retire_command_lifecycle_key(&key);
                    }
                    vm.children.remove(&name);
                    vm.bump_cmd_epoch();
                    vm.invalidate_lookup_guards();
                });
            }
        }
        self.retire_interp(id);
        true
    }
}
