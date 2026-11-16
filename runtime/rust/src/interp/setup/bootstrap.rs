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

//! Bootstrapping a fresh interpreter: the release and startup globals, the native core cells, host globals, `auto_path` and the library.

use super::super::{
    drop_fresh, native_arguments, native_operation_currency, new_string, Code, Interp,
    VariableTraceGuestOutcome,
};
use crate::frame::VarError;
use crate::namespace::GLOBAL;
use crate::obj::TclObj;
use crate::{builtins, obj};

impl Interp {
    /// Write the release-reporting globals (`tcl_version` / `tcl_patchLevel`)
    /// for the currently-emulated release.  Called at interpreter creation
    /// (C does this in `Tcl_CreateInterp`) and again whenever
    /// [`Self::set_runtime_version`] changes the answer.
    pub(crate) fn write_release_globals(&mut self) {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => {
                self.refuse_native_execution(first);
                return;
            }
        };
        let version = self.runtime_version();
        // These are owned bootstrap slots, not guest name-lookup operations.
        // Installing an unsupported native profile must not manufacture an
        // unrelated variable-access refusal before the requested operation.
        for (name, val) in [
            (&b"tcl_version"[..], version.version_string()),
            (b"tcl_patchLevel", version.patchlevel()),
        ] {
            let value = obj::Owned::fresh(new_string(val.as_bytes()));
            let previous = self
                .namespaces_mut()
                .var_table_mut(GLOBAL)
                .publish_scalar(name, value.as_ptr());
            // Release the actual displaced owner after the namespace borrow.
            drop(previous);
            drop(value);
            if let Err(first) = scope.currency().ensure_current_or_refuse() {
                self.refuse_native_execution(first);
                return;
            }
        }
    }

    /// Bootstrap the standard library like C's `Tcl_Init`: source
    /// `$tcl_library/init.tcl`. After this the
    /// pure-Tcl `unknown`/auto-load/`package` machinery is live, so
    /// `package require` works through `pkgIndex.tcl`/`tclIndex`.
    /// Install the embedding host's globals, including application arguments
    /// and a selected library path. Native core construction is separate.
    pub(crate) fn set_startup_globals(&mut self) {
        let set = |i: &mut Interp, name: &[u8], val: &[u8]| {
            let o = new_string(val);
            if i.var_set(name, o).is_err() {
                drop_fresh(o);
            }
        };
        // `tcl_version`/`tcl_patchLevel` are NOT set here: C sets them in
        // `Tcl_CreateInterp`, and so does this runtime (see `Interp::new`).
        // Re-derived rather than re-literalled so a non-9.0
        // `set_runtime_version` is not silently overwritten by `Tcl_Init`.
        self.write_release_globals();
        set(self, b"::tcl_interactive", b"0");
        set(self, b"::argv", b"");
        set(self, b"::argv0", b"");
        set(self, b"::argc", b"0");
        let _ = self.rebootstrap_host_globals();
    }

    pub(in super::super) fn bootstrap_native_core(
        &mut self,
        protocol: tcl_registry::special_vars::NativeBootstrapProtocol,
        inputs: &tcl_registry::special_vars::NativeBootstrapInputs,
    ) -> Result<(), Code> {
        use tcl_registry::special_vars::{NativeBootstrapPurpose, NativeBootstrapVariable as V};
        let scope = native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let snapshot = tcl_platform::bootstrap::snapshot_checked(
            &*self.host(),
            "treewalk",
            env!("CARGO_PKG_VERSION"),
            || scope.currency().ensure_current_or_refuse(),
        )
        .map_err(|first| self.refuse_native_execution(first))?;
        let allocations = protocol
            .allocations(
                NativeBootstrapPurpose::CreateInterpreter,
                inputs.default_library.is_some(),
            )
            .ok_or_else(|| {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native core allocation plan",
                    ),
                )
            })?;
        let installs_environment = allocations.contains(&V::Environment);
        for variable in allocations {
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            let name = variable.name().as_bytes();
            // Jim_SetVariableStr creates the original name before the table entry.
            let original_jim_root = (protocol.names().string_protocol()
                == tcl_syntax::native_string::NativeStringProtocol::Jim084)
                .then(|| obj::Owned::fresh(new_string(name)));
            match variable {
                // C publishes these with TCL_GLOBAL_ONLY, so bootstrap inside a
                // procedure never selects a same-named local.
                V::ErrorInfo | V::ErrorCode | V::Precision => {
                    let allocated = self
                        .prepare_trace_registration(name, None, Some(0))
                        .map(|_| ());
                    scope
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(|first| self.refuse_native_execution(first))?;
                    let allocated =
                        allocated.map_err(|error| crate::builtins::var_error(self, name, error));
                    self.checked_bootstrap_outcome(scope.currency(), allocated)?;
                }
                V::Environment | V::Platform => {
                    let allocated = self.ensure_array_at(name, 0);
                    scope
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(|first| self.refuse_native_execution(first))?;
                    let allocated =
                        allocated.map_err(|error| crate::builtins::var_error(self, name, error));
                    self.checked_bootstrap_outcome(scope.currency(), allocated)?;
                    if let Some(original) = &original_jim_root {
                        crate::vars::retain_original_jim_variable_key(
                            &self.frames.borrow(),
                            &self.namespaces.borrow(),
                            crate::namespace::GLOBAL,
                            name,
                            original.as_ptr(),
                        );
                        let installed = self.install_original_jim_variable(original.as_ptr(), name);
                        let installed =
                            installed.map_err(|error| self.report_cmd_error(error.into()));
                        self.checked_bootstrap_outcome(scope.currency(), installed)?;
                    }
                    let entries: Vec<(&str, &str)> = if variable == V::Environment {
                        snapshot
                            .environment()
                            .iter()
                            .map(|(key, value)| (key.as_str(), value.as_str()))
                            .collect()
                    } else {
                        snapshot
                            .platform()
                            .iter()
                            .map(|(key, value)| (*key, value.as_str()))
                            .collect()
                    };
                    for (key, value) in entries {
                        self.publish_host_bootstrap_value(
                            scope.currency(),
                            name,
                            Some(key.as_bytes()),
                            value.as_bytes(),
                        )?;
                    }
                }
                V::PatchLevel => self.publish_host_bootstrap_value(
                    scope.currency(),
                    name,
                    None,
                    self.runtime_version().patchlevel().as_bytes(),
                )?,
                V::Version => self.publish_host_bootstrap_value(
                    scope.currency(),
                    name,
                    None,
                    self.runtime_version().version_string().as_bytes(),
                )?,
                V::PackagePath => {
                    let string_protocol = protocol.names().string_protocol();
                    let members = tcl_syntax::list::split_native_list_bytes(
                        &inputs.package_path,
                        string_protocol,
                    )
                    .map_err(|error| {
                        self.report_cmd_error(
                            tcl_syntax::value::ValueError::NativeListParse {
                                error,
                                source: inputs.package_path.clone(),
                                protocol: string_protocol,
                            }
                            .into(),
                        )
                    });
                    let members = self.checked_bootstrap_outcome(scope.currency(), members)?;
                    let members: Vec<_> = members
                        .into_iter()
                        .map(|member| obj::Owned::fresh(new_string(&member)))
                        .collect();
                    let pointers: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
                    let value = obj::Owned::fresh(crate::list::new_list_obj_native(
                        &pointers,
                        string_protocol,
                    ));
                    scope
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(|first| self.refuse_native_execution(first))?;
                    let written = self.publish_host_bootstrap_object(
                        scope.currency(),
                        name,
                        None,
                        value.as_ptr(),
                    );
                    drop(value);
                    drop(members);
                    self.checked_bootstrap_outcome(scope.currency(), written)?;
                }
                V::AutoPath => self.publish_host_bootstrap_value(
                    scope.currency(),
                    name,
                    None,
                    &inputs.package_path,
                )?,
                V::DefaultLibrary => {
                    let library = inputs.default_library.as_deref().ok_or_else(|| {
                        self.refuse_native_access(
                            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                                "native default library input",
                            ),
                        )
                    })?;
                    self.publish_host_bootstrap_value(scope.currency(), name, None, library)?;
                }
                V::Interactive => {
                    self.publish_host_bootstrap_value(scope.currency(), name, None, b"0")?
                }
                V::Argv0 | V::Argc | V::Argv => {
                    return Err(self.refuse_native_access(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "native constructor main-argument allocation",
                        ),
                    ));
                }
            }
            if !matches!(variable, V::Environment | V::Platform) {
                if let Some(original) = &original_jim_root {
                    crate::vars::retain_original_jim_variable_key(
                        &self.frames.borrow(),
                        &self.namespaces.borrow(),
                        crate::namespace::GLOBAL,
                        name,
                        original.as_ptr(),
                    );
                    let installed = self.install_original_jim_variable(original.as_ptr(), name);
                    let installed = installed.map_err(|error| self.report_cmd_error(error.into()));
                    self.checked_bootstrap_outcome(scope.currency(), installed)?;
                }
            }
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
        }
        // The selected native birth plan owns this array and its host trace.
        // Install only after all original roots have been published successfully.
        if installs_environment && !self.is_safe.get() && !self.stores_confined.get() {
            self.arm_env_trace();
        }
        self.install_native_error_variable_traces_checked()?;
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        let installed = self.install_native_precision_trace_checked();
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        installed.map_err(|first| self.refuse_native_execution(first))?;
        Ok(())
    }

    /// Replace the host-derived globals under the original entered context.
    /// Storage retirement and publication are checked without firing traces
    /// during the quiet clear boundary.
    pub(super) fn rebootstrap_host_globals(&mut self) -> Result<(), Code> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|first| self.refuse_native_execution(first))?;
        let snapshot = tcl_platform::bootstrap::snapshot_checked(
            &*self.host(),
            "treewalk",
            env!("CARGO_PKG_VERSION"),
            || scope.currency().ensure_current_or_refuse(),
        )
        .map_err(|first| self.refuse_native_execution(first))?;
        self.disarm_env_trace();
        for name in tcl_platform::bootstrap::HOST_ARRAYS {
            self.quiet_var_unset_at(tcl_syntax::naming::root_unrooted_key(name).as_bytes(), 0);
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
        }
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.quiet_var_unset_at(tcl_syntax::naming::root_unrooted_key(name).as_bytes(), 0);
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
        }
        for name in [b"::tcl_platform".as_slice(), b"::env"] {
            let allocated = self.ensure_array(name);
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            let allocated =
                allocated.map_err(|error| crate::builtins::var_error(self, name, error));
            self.checked_bootstrap_outcome(scope.currency(), allocated)?;
        }
        self.publish_host_bootstrap_value(
            scope.currency(),
            b"::tcl_library",
            None,
            snapshot.tcl_library().as_bytes(),
        )?;
        self.publish_host_bootstrap_value(scope.currency(), b"::auto_path", None, b"")?;
        for (name, value) in snapshot.platform() {
            self.publish_host_bootstrap_value(
                scope.currency(),
                b"::tcl_platform",
                Some(name.as_bytes()),
                value.as_bytes(),
            )?;
        }
        for (name, value) in snapshot.environment() {
            self.publish_host_bootstrap_value(
                scope.currency(),
                b"::env",
                Some(name.as_bytes()),
                value.as_bytes(),
            )?;
        }
        // C's `TclSetupEnv` traces `env` in every interpreter it creates; a
        // safe one loses the array, and a confined one never reads the host.
        if !self.is_safe.get() && !self.stores_confined.get() {
            self.arm_env_trace();
        }
        Ok(())
    }

    fn publish_host_bootstrap_value(
        &mut self,
        currency: &native_operation_currency::NativeOperationCurrency,
        name: &[u8],
        element: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<(), Code> {
        currency
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        let value = obj::Owned::fresh(new_string(bytes));
        let written = self.publish_host_bootstrap_object(currency, name, element, value.as_ptr());
        drop(value);
        self.checked_bootstrap_outcome(currency, written)
    }

    /// Publish a caller-held original object into its root bootstrap cell.
    /// The addressed store and selected C Write observer are distinct stages;
    /// the latter holds the original receiver through callback retirement.
    fn publish_host_bootstrap_object(
        &mut self,
        currency: &native_operation_currency::NativeOperationCurrency,
        name: &[u8],
        element: Option<&[u8]>,
        value: *mut TclObj,
    ) -> Result<(), Code> {
        currency
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        let selected = if self.has_variable_traces()
            && self.native_c_variable_name_protocol().is_some()
        {
            if self.store_escapes_at(name, 0) {
                let refused = Err(crate::builtins::var_error(self, name, VarError::Confined));
                return self.checked_bootstrap_outcome(currency, refused);
            }
            let input = self.separate_variable_input(name, element);
            currency
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            let input = input.map_err(|error| crate::builtins::var_error(self, name, error));
            let input = self.checked_bootstrap_outcome(currency, input)?;
            let root = input.root().selected().to_vec();
            let member = input.element().map(|element| element.selected().to_vec());
            let captured = crate::vars::capture_variable_store_receiver_at(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                &root,
                member.clone(),
                0,
            );
            currency
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            let captured = captured.map_err(|error| crate::builtins::var_error(self, name, error));
            let captured = self.checked_bootstrap_outcome(currency, captured)?;
            let (receiver, home) = captured.ok_or_else(|| {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native bootstrap store receiver",
                    ),
                )
            })?;
            Some((receiver, home, root, member))
        } else {
            None
        };
        let outcome = (|| {
            let written = match &selected {
                Some((receiver, _, _, _)) => receiver.store(value),
                None => match element {
                    Some(element) => self.var_set_elem_at(name, element, value, 0),
                    None => self.var_set_at(name, value, 0),
                },
            };
            currency
                .ensure_current_or_refuse()
                .map_err(|first| self.refuse_native_execution(first))?;
            let written = written.map_err(|error| crate::builtins::var_error(self, name, error));
            self.checked_bootstrap_outcome(currency, written)?;
            if let Some((_, home, root, member)) = &selected {
                let access = self.trace_access(name, root, member.as_deref(), home, false);
                let traced = self.fire_var_trace_resolved(home, &access, b"write");
                currency
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                let traced = traced.map_err(|first| self.refuse_native_execution(first))?;
                match traced {
                    VariableTraceGuestOutcome::Complete => {}
                    VariableTraceGuestOutcome::GuestError => {
                        let failed =
                            Err(crate::builtins::var_error(self, name, VarError::TraceError));
                        return self.checked_bootstrap_outcome(currency, failed);
                    }
                }
            }
            Ok(())
        })();
        drop(selected);
        self.checked_bootstrap_outcome(currency, outcome)
    }

    /// Keep the original first cause after reached error reporting or cleanup.
    /// A current Guest outcome is preserved without replaying its producer.
    pub(in super::super) fn checked_bootstrap_outcome<T>(
        &mut self,
        currency: &native_operation_currency::NativeOperationCurrency,
        outcome: Result<T, Code>,
    ) -> Result<T, Code> {
        currency
            .ensure_current_or_refuse()
            .map_err(|first| self.refuse_native_execution(first))?;
        outcome
    }

    pub(crate) fn init_auto_path(&mut self, argv: &[*mut TclObj]) -> Code {
        if argv.len() != 1 {
            return self.wrong_args_for_invocation(argv, b"");
        }
        let arguments = match native_arguments::OriginalArguments::enter(self, argv) {
            Ok(arguments) => arguments,
            Err(error) => return self.report_cmd_error(error),
        };
        let result = self.init_auto_path_entered(&arguments);
        match arguments.check() {
            Ok(()) => result,
            Err(error) => self.report_cmd_error(error),
        }
    }

    fn init_auto_path_entered(&mut self, arguments: &native_arguments::OriginalArguments) -> Code {
        let existing = self.read_named_variable(b"::auto_path").ok();
        if let Err(error) = arguments.check() {
            return self.report_cmd_error(error);
        }
        if self.is_safe() {
            if existing.is_none() {
                let value = new_string(b"");
                if let Err(error) = self.store_var_result(b"::auto_path", None, value) {
                    return builtins::var_error(self, b"::auto_path", error);
                }
            }
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let existing = match existing.map(|value| self.auto_path_list(value)).transpose() {
            Ok(existing) => existing,
            Err(error) => return error,
        };
        let mut environment = Vec::new();
        if existing.is_none() {
            if let Ok(value) = self.read_named_variable(b"::env(TCLLIBPATH)") {
                environment = match self.auto_path_list(value) {
                    Ok(environment) => environment,
                    Err(error) => return error,
                };
                let home = self.host().env().get("HOME");
                if let Err(error) = arguments.check() {
                    return self.report_cmd_error(error);
                }
                environment = match tcl_cmd_core::platform::auto_path_environment(
                    environment,
                    home.as_deref().map(str::as_bytes),
                    tcl_cmd_core::path::PathReading::of_release(self.runtime_version()),
                ) {
                    Ok(paths) => paths,
                    Err(error) => return self.report_cmd_error(error),
                };
            }
        }
        let library = match self.read_named_variable(b"::tcl_library") {
            Ok(value) => match self.native_object_string_bytes(value) {
                Ok(bytes) => Some(bytes),
                Err(error) => return self.report_cmd_error(error.into()),
            },
            Err(_) => None,
        };
        let executable = self.host().env().current_exe();
        if let Err(error) = arguments.check() {
            return self.report_cmd_error(error);
        }
        let installation = tcl_cmd_core::platform::auto_path_installation(
            library.as_deref(),
            executable.as_deref().map(str::as_bytes),
            tcl_cmd_core::path::PathReading::of_release(self.runtime_version()),
        );
        let packages = self
            .read_named_variable(b"::tcl_pkgPath")
            .ok()
            .and_then(|value| self.auto_path_list(value).ok())
            .unwrap_or_default();
        if let Err(error) = arguments.check() {
            return self.report_cmd_error(error);
        }
        let paths = tcl_platform::bootstrap::auto_path(
            existing,
            &environment,
            &installation,
            &packages,
            false,
        );
        let elements = paths
            .iter()
            .map(|path| new_string(path))
            .collect::<Vec<_>>();
        let value = self.new_list_object(&elements);
        match self.store_var_result(b"::auto_path", None, value) {
            Ok(()) => Code::Ok,
            Err(error) => builtins::var_error(self, b"::auto_path", error),
        }
    }

    fn auto_path_list(&mut self, value: *mut TclObj) -> Result<Vec<Vec<u8>>, Code> {
        let original = obj::Owned::retain(value);
        let elements = tcl_syntax::value::ValueOps::list_elements(self, &original.as_ptr())
            .map_err(|error| self.report_cmd_error(error.into()))?;
        elements
            .into_iter()
            .map(|element| {
                self.native_object_string_bytes(element)
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| self.report_cmd_error(error.into()))
            })
            .collect()
    }

    pub fn init_library(&mut self) -> Code {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(first) => return self.refuse_native_execution(first),
        };
        let lib = self.host().env().get("TCL_LIBRARY").unwrap_or_default();
        // Source init.tcl, which sets up unknown/auto-load/package + appends
        // tcl_library (and its parent) to auto_path.
        let init_path = format!("{lib}/init.tcl");
        let bytes = self
            .host()
            .filesystem()
            .and_then(|fs| fs.read(&init_path).ok());
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            return self.refuse_native_execution(first);
        }
        match bytes {
            Some(bytes) => {
                if let Err(code) = self.publish_host_bootstrap_value(
                    scope.currency(),
                    b"::tcl_library",
                    None,
                    lib.as_bytes(),
                ) {
                    return code;
                }
                let active = self.begin_package_initialization();
                let code = self.eval_sourced(&bytes, init_path.as_bytes());
                self.end_package_initialization(active);
                match scope.currency().ensure_current_or_refuse() {
                    Ok(()) => code,
                    Err(first) => self.refuse_native_execution(first),
                }
            }
            None => {
                let mut m = b"can't find ".to_vec();
                m.extend_from_slice(init_path.as_bytes());
                m.extend_from_slice(b" (set TCL_LIBRARY)");
                self.error(&m)
            }
        }
    }

    /// Initialise the selected library while keeping host refusals separate.
    ///
    /// # Errors
    /// Returns the original execution refusal before reading guest values.
    pub fn init_library_completion(
        &mut self,
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let code = self.init_library();
        scope.currency().ensure_current_or_refuse()?;
        crate::completion::capture_bytes(self, code)
    }
}
