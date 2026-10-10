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

//! The embedder surface: what a Rust host needs to run small Tcl bodies at
//! rate, register commands of its own, and bound what a body may do.
//!
//! Hosts can register stateful commands, enumerate exact native name bytes,
//! select a command whitelist and limit script execution.
//!
//! - **Call a function without a driver script.** [`Vm::invoke_command`] is
//!   public, and [`Vm::compile_function`] / [`Vm::invoke_function`] pre-compile
//!   a body once into a [`FunctionHandle`] and run it with no per-call
//!   `FunctionAsm` clone — the deep copy [`Vm::run_function`] performs.
//! - **Register a stateful command.** [`Vm::register_native_command`] takes an
//!   `Rc<dyn NativeCommand>`, so an embedder's command can carry state (the
//!   emitter verbs of a `SpecTcl` hook family collect what the body emitted).
//! - **Say what a host command provides.** [`Vm::package_provide`] does what
//!   `package provide` does, and [`Vm::library_loaded`] records a library `info
//!   loaded` lists, for a host command that loads native code.
//! - **Reach the variables.** [`Vm::read_variable`], [`Vm::write_variable`] and
//!   [`Vm::unset_variable`] do what `set` and `unset` do in the current frame,
//!   element names and traces included; [`Vm::get_var`], [`Vm::set_var`] and
//!   [`Vm::unset_var`] are the scalar-only forms beneath them.
//! - **Take an error as the host's own.** [`Vm::publish_caught_error`] leaves
//!   `$errorInfo` and `$errorCode` as a `catch` would, for a host command that
//!   evaluated a script and swallowed what it raised.
//! - **Restrict the command table.** [`Vm::retain_commands_bytes`] selects
//!   commands by exact native display bytes. [`Vm::retain_commands`] provides
//!   a Unicode whitelist and removes names that cannot be represented in it.
//! - **Bound the work.** [`Vm::set_command_limit`] arms the `commands` limit
//!   the VM now enforces, and [`Vm::commands_run`] reports the fuel spent.
//! - **Confine the stores.** [`Vm::set_stores_confined`] keeps every write in
//!   the running procedure's own frame, so a body cannot leave state behind
//!   for its next call.

use std::cell::RefCell;
use std::rc::Rc;

use crate::command::{BuiltinFn, Command, NativeCommand, opt_get_checked};
use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Code, Completion};

/// A host compilation boundary preserves native Tcl diagnostics and provider
/// refusals as separate cases. Host failures cannot become guest completions.
#[derive(Debug, Clone)]
pub enum VmCompilationError {
    /// A native Tcl definition or source-validation diagnostic.
    Tcl(TclError),
    /// The execution engine requires an unavailable native provider/capability.
    Host(tcl_runtime_api::NativeExecutionError),
}

impl std::fmt::Display for VmCompilationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcl(error) => match error.message_unicode() {
                Ok(message) => formatter.write_str(&message),
                Err(refusal) if error.is_host() => std::fmt::Display::fmt(&refusal, formatter),
                Err(_) => write!(
                    formatter,
                    "native Tcl error bytes: {:?}",
                    error.message_bytes()
                ),
            },
            Self::Host(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for VmCompilationError {}

/// The host procedure-definition error uses the shared compilation boundary.
pub type ProcedureDefinitionError = VmCompilationError;

/// A native implementation and grammar explicitly granted by an embedding host.
/// Command spelling and source namespace never confer this capability.
#[derive(Clone)]
pub(crate) struct FrameworkBuiltinCapability {
    alias: String,
    implementation: BuiltinFn,
    profile: &'static tcl_dialect::DialectProfile,
}

impl NativeCommand for FrameworkBuiltinCapability {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let saved = vm.active_native_profile.replace(self.profile);
        let result = (self.implementation)(vm, args);
        vm.active_native_profile = saved;
        result
    }
}

/// A pre-compiled Tcl body, ready to run any number of times.
///
/// Holds the compiled activation behind an `Rc`, so invoking it is a pointer
/// clone rather than a deep copy of the bytecode — the difference between a
/// hook body that costs its own execution and one that pays for its
/// compilation shape on every call site. It also retains its source to refresh
/// the activation when the owning VM changes dialect profile or namespace, or
/// a specialised command binding no longer resolves to the implementation it
/// assumed.
#[derive(Clone)]
pub struct FunctionHandle {
    source: tcl_lexer::SourceImage,
    state: Rc<RefCell<FunctionHandleState>>,
}

struct FunctionHandleState {
    unit: crate::compiled::CompiledUnit,
    owner_nonce: u64,
}

impl std::fmt::Debug for FunctionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionHandle")
            .field("source", &self.source)
            .field("owner_nonce", &self.state.borrow().owner_nonce)
            .field(
                "instructions",
                &self.state.borrow().unit.asm.instructions.len(),
            )
            .field(
                "profile_generation",
                &self.state.borrow().unit.profile_generation,
            )
            .field("command_epoch", &self.state.borrow().unit.command_epoch)
            .field(
                "source_namespace",
                &self.state.borrow().unit.source_namespace,
            )
            .field(
                "compiler_generation",
                &self.state.borrow().unit.compiler.generation(),
            )
            .finish()
    }
}

impl Vm {
    /// Compile `src` to a reusable [`FunctionHandle`], registering any procs it
    /// defines. Needs an injected
    /// [`CompileService`](tcl_runtime_api::CompileService), like every other
    /// runtime-compilation path.
    ///
    /// # Panics
    /// Panics when the host must provide an unsupported compilation capability.
    /// Use [`Self::try_compile_function`] to handle that refusal explicitly.
    pub fn compile_function(&mut self, src: &str) -> Result<FunctionHandle, TclError> {
        match self.try_compile_function(src) {
            Ok(handle) => Ok(handle),
            Err(VmCompilationError::Tcl(error)) => Err(error),
            Err(VmCompilationError::Host(error)) => {
                panic!("native compiler provider required: {error}")
            }
        }
    }

    /// Compile reusable source with provider refusals outside Tcl diagnostics.
    ///
    /// # Errors
    /// Returns a native source diagnostic or a typed host refusal.
    pub fn try_compile_function(
        &mut self,
        src: &str,
    ) -> Result<FunctionHandle, VmCompilationError> {
        self.host_execution_depth += 1;
        let result = self.compile_function_internal(src);
        self.host_execution_depth -= 1;
        self.finish_host_compilation(result)
    }

    fn compile_function_internal(&mut self, src: &str) -> Result<FunctionHandle, TclError> {
        Ok(FunctionHandle {
            source: tcl_lexer::SourceImage::document(src),
            state: Rc::new(RefCell::new(FunctionHandleState {
                unit: self.compile_script_cached(src)?,
                owner_nonce: self.owner_nonce,
            })),
        })
    }

    /// Compile an original byte image into a reusable function handle.
    /// The source channel, native namespace segments and actual compiler entry
    /// remain part of cache identity. No Unicode source view is required.
    ///
    /// # Errors
    /// Returns guest parse errors or an explicit native compiler/provider refusal.
    pub fn try_compile_function_bytes(
        &mut self,
        source: &tcl_lexer::SourceImage,
    ) -> Result<FunctionHandle, VmCompilationError> {
        self.host_execution_depth += 1;
        let result = self
            .compile_script_cached_bytes(source)
            .map(|unit| FunctionHandle {
                source: source.clone(),
                state: Rc::new(RefCell::new(FunctionHandleState {
                    unit,
                    owner_nonce: self.owner_nonce,
                })),
            });
        self.host_execution_depth -= 1;
        self.finish_host_compilation(result)
    }

    /// Define a Tcl procedure from the host, without going through the `proc`
    /// command.
    ///
    /// The original body is retained here and prepared on activation through
    /// the same native body owner as a Tcl-defined procedure.
    /// Host-defined rather than `proc`-defined because an embedder building a
    /// sandbox removes `proc` from the command table
    /// ([`Self::retain_commands`]) — the procedure it wants to *call* must not
    /// depend on the command it wants to *forbid*, and ordering the two would
    /// be a trap.
    ///
    /// Proc semantics are what make each invocation independent: its own
    /// locals, and `return` as an ordinary early exit rather than an error.
    ///
    /// # Panics
    /// Panics when the native definition provider refuses the requested capability.
    /// Use [`Self::try_define_procedure`] to retain the typed host error.
    pub fn define_procedure(
        &mut self,
        name: &str,
        parameters: &[&str],
        body: &str,
    ) -> Result<(), TclError> {
        match self.try_define_procedure(name, parameters, body) {
            Ok(()) => Ok(()),
            Err(VmCompilationError::Tcl(error)) => Err(error),
            Err(VmCompilationError::Host(error)) => {
                panic!("native compiler provider required: {error}")
            }
        }
    }

    /// Define a host procedure while retaining native definition refusal as a
    /// neutral host error. Body preparation belongs to invocation.
    ///
    /// # Errors
    /// Returns the native definition error or neutral host refusal separately.
    pub fn try_define_procedure(
        &mut self,
        name: &str,
        parameters: &[&str],
        body: &str,
    ) -> Result<(), ProcedureDefinitionError> {
        self.host_execution_depth += 1;
        let result = self.define_procedure_internal(name, parameters, body);
        self.host_execution_depth -= 1;
        self.finish_host_compilation(result)
    }

    fn finish_host_compilation<T>(
        &mut self,
        result: Result<T, TclError>,
    ) -> Result<T, VmCompilationError> {
        if let Some(error) = self.take_host_boundary_refusal() {
            return Err(VmCompilationError::Host(error));
        }
        result.map_err(VmCompilationError::Tcl)
    }

    fn define_procedure_internal(
        &mut self,
        name: &str,
        parameters: &[&str],
        body: &str,
    ) -> Result<(), TclError> {
        let namespace = self.current_ns().to_owned();
        let parameter_grammar = self
            .native_invocation_dialect()
            .parameter_grammar()
            .ok_or_else(|| {
                self.compile_service_error(
                    tcl_runtime_api::CompileError::Unsupported(
                        "native parameter grammar is not selected".into(),
                    ),
                    body,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::ProcedureBody,
                )
            })?;
        let parameter_source = parameters.join(" ");
        let parameter_value = Value::from_native_string_bytes(parameter_source.as_bytes());
        let (parameters, has_args) =
            crate::command::parse_params_value(self, &parameter_value, name.as_bytes())
                .map_err(TclError::from_completion)?;
        let (registered, slot, ns_id) = self
            .native_procedure_publication(name.as_bytes())
            .map_err(|error| {
                let _ = self.refuse_host_command(error.to_string());
                TclError::from_execution_failure(
                    self.execution_refusal
                        .clone()
                        .expect("publication refusal retained"),
                )
            })?;
        let table_key = self
            .jim_command_table_key_for_original(
                name.as_bytes(),
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .map_err(|error| {
                let _ = self.refuse_host_command(error.to_string());
                TclError::from_execution_failure(
                    self.execution_refusal
                        .clone()
                        .expect("publication refusal retained"),
                )
            })?;
        self.define_proc_binding_with_jim_key(
            crate::command::ProcDef {
                native_resources: std::rc::Rc::default(),
                name: registered,
                command_ns_id: slot.namespace,
                simple_name: slot.simple,
                namespace: self.namespace_path_for_token(ns_id),
                ns_id,
                params: parameters,
                parameter_grammar,
                has_args,
                native_jim_namespace: None,
                native_parameters: (parameter_grammar == tcl_dialect::ParameterGrammar::Jim)
                    .then_some(parameter_value),

                native_header: tcl_registry::native_procedure::procedure_header_compilation(
                    self.native_invocation_dialect(),
                    Some(&parameter_source),
                    Some(body),
                    Some(false),
                ),
                statics: None,
                body: None,
                body_src: Value::new_native_string_bytes(body.as_bytes()),
                usage_name: None,
                call_identity: None,
            },
            table_key,
        );
        Ok(())
    }

    /// Arm a wall-clock cap: `budget` from now, after which the VM's poll
    /// ([`interp limit time`](Vm::limit_check_tick)) fails the running body
    /// with `time limit exceeded`. `None` disarms it.
    ///
    /// Polled rather than pre-emptive — a body between polls can overrun by
    /// the poll interval, which is why the command budget, not this, is the
    /// containment guarantee. This is the belt to its braces: it catches a
    /// body that spends its time inside *one* command.
    pub fn set_wall_clock_budget(&mut self, budget: Option<std::time::Duration>) {
        let deadline = budget.map(|budget| {
            let now = self.host_rc().clock().now_millis();
            let millis = i128::try_from(budget.as_millis()).unwrap_or(i128::from(i64::MAX));
            now.saturating_add(millis)
        });
        self.set_time_limit_deadline(deadline);
    }

    /// Run a pre-compiled body to completion in the current frame.
    ///
    /// The invoke-by-handle path: no compilation, no `FunctionAsm` clone, and
    /// no driver script — the three costs an embedder would otherwise pay
    /// per call.
    ///
    /// # Panics
    /// Panics when refreshing or executing the handle needs a native provider.
    /// Use [`Self::try_invoke_function`] to preserve the typed host refusal.
    #[must_use]
    pub fn invoke_function(&mut self, handle: &FunctionHandle) -> Completion<Value> {
        self.try_invoke_function(handle)
            .expect("function handle requires a genuine native execution provider")
    }

    /// Refresh and invoke a reusable handle without exposing internal host
    /// refusal unwinds as guest completions.
    ///
    /// # Errors
    /// Returns the typed compiler or reached-expression provider obligation.
    pub fn try_invoke_function(
        &mut self,
        handle: &FunctionHandle,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let result = self.invoke_function_internal(handle);
        self.host_execution_depth -= 1;
        self.finish_host_execution(result)
    }

    fn invoke_function_internal(&mut self, handle: &FunctionHandle) -> Completion<Value> {
        if handle.state.borrow().owner_nonce != self.owner_nonce {
            return crate::interp::err("FunctionHandle belongs to a different Vm");
        }
        let profile_changed =
            handle.state.borrow().unit.profile_generation != self.profile_generation();
        let compiler_changed = !handle
            .state
            .borrow()
            .unit
            .compiler
            .is_current_service(self.compiler_generation());
        let epoch_changed = handle.state.borrow().unit.native_cache
            != self.native_cache_stamp_for_source_namespace(&self.source_namespace_path());
        let namespace = self.source_namespace_path();
        let namespace_changed = handle.state.borrow().unit.source_namespace != namespace;
        let interpreter_changed =
            handle.state.borrow().unit.interpreter != self.native_interpreter_identity();
        let bindings_match = {
            let state = handle.state.borrow();
            self.function_command_bindings_match_with_manifest(
                &state.unit.asm,
                state.unit.manifest.as_deref(),
            )
        };
        if profile_changed
            || compiler_changed
            || epoch_changed
            || namespace_changed
            || interpreter_changed
            || !bindings_match
        {
            let unit = match self.compile_script_cached_bytes(&handle.source) {
                Ok(unit) => unit,
                Err(error) => return crate::command::completion_from_tcl_error(self, error),
            };
            *handle.state.borrow_mut() = FunctionHandleState {
                unit,
                owner_nonce: self.owner_nonce,
            };
        }
        self.run_compiled_unit(handle.state.borrow().unit.clone())
    }

    /// Register an embedder command using the selected script declaration policy.
    ///
    /// Plain and qualified names publish as procedure declarations in the actual
    /// current namespace. Original byte extents and qualifier routing belong to
    /// the shared naming owner. Existing commands in the selected slot are replaced.
    /// An unavailable policy retains a host execution refusal; use
    /// [`Self::try_register_native_command`] to receive it directly.
    pub fn register_native_command(&mut self, name: &str, command: Rc<dyn NativeCommand>) {
        if let Err(error) = self.try_register_native_command(name, command) {
            let _ = self.refuse_host_command(error.to_string());
        }
    }

    /// Register a host command through the same checked script-publication owner.
    ///
    /// # Errors
    /// Refuses an unavailable naming policy, actual namespace context or retired
    /// interpreter before publishing a command. A compatible source profile
    /// supplies no missing native naming authority.
    pub fn try_register_native_command(
        &mut self,
        name: &str,
        command: Rc<dyn NativeCommand>,
    ) -> Result<(), crate::NativeCommandLookupUnavailable> {
        self.register_written_command(name, Command::Native(command))
            .map(|_| ())
    }

    /// Grant a private alias a concrete native host grammar. The original must
    /// still be a native builtin; a user replacement cannot acquire its identity.
    /// Child interpreters inherit this explicit host contract, then install their
    /// own alias token. Deferred bodies retain the user's compilation dialect.
    /// Provide a package from the host, as `package provide name version` does.
    ///
    /// The version is validated for the release the VM emulates, a package
    /// already provided at a different version is refused with the error `package
    /// provide` gives (`conflicting versions provided for package "p": 1.0, then
    /// 2.0`, `TCL PACKAGE VERSIONCONFLICT`), and the same version again is a
    /// no-op. A later `package require` is satisfied from what was provided here.
    pub fn package_provide(&mut self, name: &str, version: &str) -> Result<(), TclError> {
        self.begin_embedding_call()?;
        let completion = crate::cmd_package::pkg_provide(
            self,
            &[Value::string(name), Value::string(version)],
            self.runtime_version(),
        );
        let result = if completion.code.is_ok() {
            Ok(())
        } else {
            Err(completion)
        };
        self.finish_embedding_result(result)
    }

    /// Record that the library `prefix` has been loaded into the interpreter,
    /// from `file_name` (empty for one linked into the program), so `info loaded`
    /// lists it. A prefix is listed once, under the file it was first loaded from.
    pub fn library_loaded(&mut self, file_name: &str, prefix: &str) {
        self.note_library_loaded(file_name, prefix);
    }

    /// Read the variable `name` as `set name` does in the current frame: a
    /// scalar, or an array element spelt `a(k)`, with its read traces fired. A
    /// variable that is not there is the error `set` raises
    /// (`can't read "x": no such variable`).
    ///
    /// # Errors
    /// Retains the original guest completion or reached typed host refusal.
    pub fn read_variable(&mut self, name: &str) -> Result<Value, TclError> {
        self.begin_embedding_call()?;
        let result = self.read_variable_result_bytes(name.as_bytes(), None);
        self.finish_embedding_result(result)
    }

    /// Set the variable `name` as `set name value` does in the current frame: a
    /// scalar, or an array element spelt `a(k)`, with its write traces fired. A
    /// store `set` refuses (an array, a constant, a confined store) is the error
    /// `set` raises.
    ///
    /// # Errors
    /// Retains original guest options and any trace's typed host refusal.
    pub fn write_variable(&mut self, name: &str, value: Value) -> Result<(), TclError> {
        self.begin_embedding_call()?;
        let result = self.store_var_result(name, value).map(|_| ());
        self.finish_embedding_result(result)
    }

    /// Unset the variable `name` as `unset name` does in the current frame: a
    /// scalar, an array element spelt `a(k)` or a whole array, with its unset
    /// traces fired. One that is not there is the error `unset` raises
    /// (`can't unset "x": no such variable`).
    ///
    /// # Errors
    /// A reached host trace refusal remains independent of the guest unset result.
    pub fn unset_variable(&mut self, name: &str) -> Result<(), TclError> {
        self.begin_embedding_call()?;
        let result = self.unset_one(name, true);
        self.finish_embedding_result(result)
    }

    fn begin_embedding_call(&mut self) -> Result<(), TclError> {
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        self.host_execution_depth += 1;
        Ok(())
    }

    fn finish_embedding_result<T>(
        &mut self,
        result: Result<T, Completion<Value>>,
    ) -> Result<T, TclError> {
        self.host_execution_depth -= 1;
        if let Some(refusal) = self.take_host_boundary_refusal() {
            return Err(TclError::from_execution_failure(refusal));
        }
        result.map_err(TclError::from_completion)
    }

    /// Read an exact completion option through this interpreter's shared
    /// Dictionary/List owner and checked original byte getter. Absence is `None`;
    /// an explicitly empty value is `Some(Vec::new())`. Original key/value
    /// headers are retained, and no diagnostic supplies option identity.
    /// This lookup remains in the current entry and does not reset a refusal.
    ///
    /// # Errors
    /// Returns the first retained Host cause before or after a reached getter,
    /// or the checked option owner's genuine guest conversion failure.
    pub fn completion_option_bytes_checked(
        &mut self,
        completion: &Completion<Value>,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, TclError> {
        let value = opt_get_checked(self, &completion.options, key)?;
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(cause));
        }
        let bytes = value
            .map(|value| {
                tcl_syntax::value::ValueOps::native_string_bytes(self, &value)
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| crate::command::completion_option_failure(self, error))
            })
            .transpose()?;
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(TclError::from_execution_failure(cause));
        }
        Ok(bytes)
    }

    /// Publish `$errorInfo` and `$errorCode` for `completion`, an error a host
    /// command took as its own: one that evaluated a script and swallowed its
    /// failure leaves them as a `catch` of the script would have, so what the
    /// script raised is what the next command reads. A completion that is not an
    /// error publishes nothing, and nothing is published while stores are
    /// confined to the activation.
    /// Options retain their original objects and use this interpreter's checked
    /// completion planner, byte getters and first retained host-failure owner.
    ///
    /// # Errors
    /// Returns the first retained host failure before reading or publishing guest
    /// values. Reached option/key/result access remains byte-preserving and fallible.
    pub fn publish_caught_error(&mut self, completion: &Completion<Value>) -> Result<(), TclError> {
        self.begin_embedding_call()?;
        let result = (|| {
            if completion.code != Code::Error {
                return Ok(());
            }
            let options = self
                .completion_options_snapshot(completion)
                .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
            let original = opt_get_checked(self, &options, b"-errorinfo")
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?
                .unwrap_or_else(|| completion.result.clone());
            let info = tcl_syntax::value::ValueOps::native_string_bytes(self, &original)
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
            let code = opt_get_checked(self, &options, b"-errorcode")
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?
                .unwrap_or_else(|| Value::from_string_bytes(b"NONE".as_slice()));
            let _ = self.take_error_info();
            self.publish_error(&info, &code);
            Ok(())
        })();
        self.finish_embedding_result(result)
    }

    /// Every command name currently registered, sorted.
    #[must_use]
    pub fn register_framework_builtin(
        &mut self,
        alias: &str,
        original: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        let Some(Command::Builtin(implementation)) = self.lookup_command(original) else {
            return false;
        };
        let capability = FrameworkBuiltinCapability {
            alias: alias.to_owned(),
            implementation,
            profile,
        };
        if !self.install_framework_builtin(capability.clone()) {
            return false;
        }
        self.framework_builtins.push(capability);
        true
    }

    pub(crate) fn install_framework_builtin(
        &mut self,
        capability: FrameworkBuiltinCapability,
    ) -> bool {
        let alias = capability.alias.clone();
        match self.register_written_command(&alias, Command::Native(Rc::new(capability))) {
            Ok(_) => true,
            Err(error) => {
                let _ = self.refuse_host_command(error.to_string());
                false
            }
        }
    }

    /// Every registered command's exact native display bytes, sorted.
    #[must_use]
    pub fn command_names_bytes(&self) -> Vec<NameBytes> {
        let mut names: Vec<_> = self
            .registered_command_entries()
            .into_iter()
            .map(|(_, display)| display)
            .collect();
        names.sort_unstable();
        names
    }

    /// Checked Unicode view of every registered command, sorted.
    ///
    /// # Errors
    /// Returns a decoding error if any native name is not valid UTF-8. Use
    /// [`Self::command_names_bytes`] to enumerate every native name.
    pub fn command_names(&self) -> Result<Vec<String>, std::str::Utf8Error> {
        self.command_names_bytes()
            .into_iter()
            .map(|name| name.try_utf8().map(str::to_owned))
            .collect()
    }

    /// Delete `name` from the command table, reporting whether it was there.
    pub fn remove_command(&mut self, name: &str) -> bool {
        // This is an embedder-owned teardown API, not Tcl's `rename`/`interp
        // hide` surface.  Keep it able to remove a command even when the
        // command is hidden by the emulated release.
        self.take_command_unchecked(name).is_some()
    }

    /// Keep exactly the native command displays accepted by `keep`.
    ///
    /// The callback sees complete retained display bytes, never private storage
    /// keys. All metadata borrows end before `keep` or command deletion callbacks
    /// run. Returns the number of entries selected for removal.
    pub fn retain_commands_bytes(&mut self, keep: &dyn Fn(&[u8]) -> bool) -> usize {
        let doomed: Vec<String> = self
            .registered_command_entries()
            .into_iter()
            .filter_map(|(key, display)| (!keep(display.as_bytes())).then_some(key))
            .collect();
        let removed = doomed.len();
        for key in doomed {
            self.remove_registered_command(&key);
        }
        removed
    }

    /// Keep commands whose exact Unicode display is accepted by `keep`.
    ///
    /// Native names that cannot be represented as UTF-8 are removed. Use
    /// [`Self::retain_commands_bytes`] when the whitelist includes opaque names.
    pub fn retain_commands(&mut self, keep: &dyn Fn(&str) -> bool) -> usize {
        self.retain_commands_bytes(&|name| core::str::from_utf8(name).is_ok_and(keep))
    }

    /// Arm the `commands` limit: the body may dispatch at most `limit`
    /// commands before every further dispatch fails with `command count limit
    /// exceeded`. `None` disarms it.
    ///
    /// The count is the interp's own ([`Self::commands_run`]), so refill fuel
    /// per invocation with [`Self::reset_command_count`].
    pub fn set_command_limit(&mut self, limit: Option<u64>) {
        let value = limit.map(|limit| i64::try_from(limit).unwrap_or(i64::MAX));
        self.set_command_limit_value(value);
    }

    /// Arm the value-size limit: any single value the body builds may be at
    /// most `bytes` long, and an attempt to exceed it fails with `value size
    /// limit exceeded`. `None` disarms it.
    ///
    /// The `commands` and `time` limits cannot express this. One `string
    /// repeat` opcode dispatches a single command and returns in microseconds
    /// while asking the allocator for an arbitrary amount, so both budgets are
    /// still nearly full when the process dies of OOM — the one sandbox
    /// failure that cannot be contained and reported, because it takes the
    /// server down with the hook.
    pub fn set_value_size_limit(&mut self, bytes: Option<u64>) {
        self.set_value_size_limit_value(bytes);
    }

    /// The armed value-size limit, if any.
    #[must_use]
    pub fn value_size_limit(&self) -> Option<u64> {
        self.value_size_limit_value()
    }

    /// Confine every store to the running procedure's own frame, or release
    /// the confinement. While confined, a store, an array's creation or an
    /// unset whose name resolves anywhere else — a `::`-qualified name, a
    /// namespace variable, a global, a local linked to another frame — fails
    /// as a Tcl error (`can't set "::n": stores are confined to the
    /// activation`, or `can't unset …`) before anything is written or
    /// removed.
    /// Confining also removes the globals the host's bootstrap wrote
    /// (`::env`, `::tcl_platform`, the library paths), again after a host
    /// swap; releasing the confinement does not restore them. A hosted body
    /// whose writes all stay in its own activation leaves nothing for a
    /// later call to read and finds no host environment to read, so its
    /// answer depends on its arguments alone.
    pub fn set_stores_confined(&mut self, confined: bool) {
        self.set_stores_confined_value(confined);
    }

    /// Whether stores are confined to the running procedure's own frame.
    #[must_use]
    pub fn stores_confined(&self) -> bool {
        self.stores_confined_value()
    }

    /// The armed `commands` limit, if any.
    #[must_use]
    pub fn command_limit(&self) -> Option<u64> {
        self.command_limit_value()
            .map(|value| u64::try_from(value).unwrap_or(0))
    }

    /// Commands dispatched since the last [`Self::reset_command_count`].
    #[must_use]
    pub fn commands_run(&self) -> u64 {
        self.command_count()
    }

    /// Zero the command counter — refill the fuel before an invocation.
    pub fn reset_command_count(&mut self) {
        self.reset_command_count_inner();
    }
}

#[cfg(test)]
mod embedding_settlement_tests {
    use super::*;

    #[test]
    fn public_completion_option_bytes_retains_opaque_empty_and_absent_members() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software API control: these constructed carriers do not establish an
        // original C Tcl/Jim physical-object or completion observation.
        let mut vm = crate::native_fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
        );
        let key = Value::new_native_string_bytes(b"-opaque\0\xff".as_slice());
        let value = Value::new_native_string_bytes(b"VALUE\0\xfe".as_slice());
        let empty = Value::new_native_string_bytes(b"".as_slice());
        let pairs = vec![
            (key.clone(), value.clone()),
            (Value::new_native_string_bytes(b"-empty".as_slice()), empty),
        ];
        let list = Value::list(
            pairs
                .iter()
                .flat_map(|(key, value)| [key.clone(), value.clone()])
                .collect(),
        );
        let dictionary = tcl_syntax::value::ValueOps::new_dict_checked(&mut vm, pairs).unwrap();
        for options in [list, dictionary] {
            let identity = options.native_object_identity();
            let was_dictionary = options.cached_dictionary_bucket_count();
            let completion = Completion::new(Code::Error, Value::int(17), options);
            assert_eq!(
                vm.completion_option_bytes_checked(&completion, b"-opaque\0\xff")
                    .unwrap(),
                Some(b"VALUE\0\xfe".to_vec())
            );
            assert_eq!(
                vm.completion_option_bytes_checked(&completion, b"-opaque")
                    .unwrap(),
                None
            );
            assert_eq!(
                vm.completion_option_bytes_checked(&completion, b"-empty")
                    .unwrap(),
                Some(Vec::new())
            );
            assert_eq!(
                vm.completion_option_bytes_checked(&completion, b"-absent")
                    .unwrap(),
                None
            );
            assert_eq!(completion.options.native_object_identity(), identity);
            assert_eq!(
                completion.options.cached_dictionary_bucket_count(),
                was_dictionary
            );
            assert!(completion.options.resident_string_bytes().is_none());
            assert!(completion.result.resident_string_bytes().is_none());
            if was_dictionary.is_some() {
                completion
                    .options
                    .with_cached_dictionary_representation(|pairs, _| {
                        assert!(pairs.iter().any(|(original_key, original_value)| {
                            original_key.is_same_object(&key)
                                && original_value.is_same_object(&value)
                        }));
                    })
                    .unwrap();
                assert!(completion.options.cached_list_representation().is_none());
            }
        }
    }

    #[test]
    fn public_completion_option_bytes_preserves_first_host_before_any_getter() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // The retained refusal is a software owner control, not a measured
        // original-provider failure or a claim that a getter was reached.
        let mut vm = crate::native_fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
        );
        vm.set_var("before", Value::string("RETAINED")).unwrap();
        let key = Value::int(41);
        let value = Value::int(42);
        let completion = Completion::new(
            Code::Error,
            Value::int(17),
            Value::list(vec![key.clone(), value.clone()]),
        );
        let cause = tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable;
        vm.refuse_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(cause));
        for wanted in [b"41".as_slice(), b"ABSENT".as_slice()] {
            let error = vm
                .completion_option_bytes_checked(&completion, wanted)
                .unwrap_err();
            assert!(matches!(error,
                TclError::Host(crate::error::TclHostFailure::Execution(
                    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(original)
                )) if original == cause
            ));
            assert!(key.resident_string_bytes().is_none());
            assert!(value.resident_string_bytes().is_none());
            assert!(completion.options.resident_string_bytes().is_none());
            assert!(completion.result.resident_string_bytes().is_none());
            assert_eq!(
                vm.execution_refusal,
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    cause
                ))
            );
        }
        assert_eq!(
            vm.get_var("before")
                .unwrap()
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"RETAINED"
        );
    }

    #[test]
    fn embedding_settlement_retains_complete_original_guest_result_and_options() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for code in [
            Code::Error,
            Code::Return,
            Code::Break,
            Code::Continue,
            Code::Other(7),
        ] {
            let mut vm = Vm::new();
            let result = Value::from_string_bytes(b"RESULT\0\xff".as_slice());
            let options = Value::list(vec![
                Value::string("-tag"),
                Value::from_string_bytes(b"OPTION\0\xfe".as_slice()),
            ]);
            vm.begin_embedding_call().unwrap();
            let error = vm
                .finish_embedding_result::<()>(Err(Completion::new(
                    code,
                    result.clone(),
                    options.clone(),
                )))
                .unwrap_err();
            let completion = error.into_completion().expect("actual guest producer");
            assert_eq!(completion.code, code);
            assert!(completion.result.is_same_object(&result));
            assert!(completion.options.is_same_object(&options));
            assert_eq!(completion.result.string_bytes().as_ref(), b"RESULT\0\xff");
            assert!(vm.take_host_boundary_refusal().is_none());
        }
    }
}

#[cfg(test)]
mod procedure_definition_tests {
    use super::*;

    #[test]
    fn embedded_definition_retains_unprepared_source_without_a_compiler() {
        let mut vm = Vm::new();
        vm.try_define_procedure("unentered", &["x"], "set value \"")
            .unwrap();
        let crate::command::Command::Proc(definition) = vm.lookup_command("unentered").unwrap()
        else {
            panic!("retained original procedure")
        };
        assert!(definition.body.is_none());
        assert_eq!(definition.body_src.string_bytes().as_ref(), b"set value \"");
    }

    fn vm() -> Vm {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::default(),
        ));
        vm
    }

    #[test]
    fn embedded_procedure_captures_its_allocated_namespace_before_compilation() {
        for name in ["root_body", "::spectcl::unit::body"] {
            let mut vm = vm();
            vm.try_define_procedure(name, &["words"], "return [llength $words]")
                .expect("actual definition namespace owns compilation");
            let result = vm
                .try_invoke_command(
                    name,
                    &[Value::list(vec![Value::string("a"), Value::string("b")])],
                )
                .expect("original body executes");
            assert_eq!(result.code, tcl_runtime_api::Code::Ok);
            assert_eq!(result.result.string_bytes().as_ref(), b"2");
            let crate::command::Command::Proc(definition) = vm.lookup_command(name).unwrap() else {
                panic!("original procedure")
            };
            let original = Value::new_native_string_bytes(name.as_bytes());
            let selected = vm
                .resolve_original_command_key_at(vm.current_ns_id(), &original)
                .unwrap()
                .unwrap();
            let (namespace, _) = vm.command_slot_parts(&selected).unwrap();
            assert_eq!(namespace, definition.ns_id);
        }
    }

    #[test]
    fn embedded_procedure_recreated_namespace_gets_its_current_token() {
        let mut vm = vm();
        vm.try_define_procedure("::N::body", &[], "return OLD")
            .unwrap();
        let crate::command::Command::Proc(old) = vm.lookup_command("::N::body").unwrap() else {
            panic!("old procedure")
        };
        assert_eq!(
            vm.try_eval_source("namespace delete ::N").unwrap().code,
            tcl_runtime_api::Code::Ok
        );
        vm.try_define_procedure("::N::body", &[], "return NEW")
            .unwrap();
        let crate::command::Command::Proc(new) = vm.lookup_command("::N::body").unwrap() else {
            panic!("new procedure")
        };
        assert_ne!(old.ns_id, new.ns_id);
        assert_eq!(vm.namespace_token_for_written("N"), Some(new.ns_id));
        let result = vm.try_invoke_command("::N::body", &[]).unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"NEW");
    }

    #[test]
    fn folded_eval_uses_final_caller_locals_and_propagates_child_control() {
        for (body, expected) in [
            (
                "eval {set later CHILD}; set later PARENT; return $later",
                "PARENT",
            ),
            (
                "set count 0; while {1} {incr count; eval {break}; incr count}; return $count",
                "1",
            ),
            ("eval {set literal [list CHILD]}; return $literal", "CHILD"),
        ] {
            let mut vm = vm();
            vm.try_define_procedure("::eval_boundary::p", &[], body)
                .unwrap();
            let completion = vm.try_invoke_command("::eval_boundary::p", &[]).unwrap();
            assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{body}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_bytes(),
                "{body}"
            );
        }
    }

    #[test]
    fn invalid_embedded_formals_do_not_create_the_definition_namespace() {
        let mut vm = vm();
        assert!(
            vm.try_define_procedure("::invalid_header::body", &["{x y z}"], "return OK")
                .is_err()
        );
        assert!(vm.namespace_token_for_written("invalid_header").is_none());
        assert!(vm.lookup_command("::invalid_header::body").is_none());
    }

    #[test]
    fn guest_proc_keeps_its_missing_namespace_header_failure() {
        let mut vm = vm();
        let result = vm
            .try_invoke_command(
                "proc",
                &[
                    Value::string("::missing_header::body"),
                    Value::empty(),
                    Value::string("return OK"),
                ],
            )
            .unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Error);
        assert!(vm.namespace_token_for_written("missing_header").is_none());
        assert!(vm.lookup_command("::missing_header::body").is_none());
    }
}
