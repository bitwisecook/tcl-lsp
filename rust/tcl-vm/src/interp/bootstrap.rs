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

//! Building an interpreter: the constructors, fresh interpreter state, the native core bootstrap, host globals, the autoloader and the script libraries.

use super::namespace_context::ProcedureNamespace;
use super::native_name_world::{NameWorldCell, NativeNameWorld};
use super::{
    DefaultHost, FixedMathTable, FreshSemanticEnvironment, GuardedCommands, HiddenCommandState,
    InterpSlot, InterpState, InterpreterCounters, InterpreterDebugHooks, JimEvaluationState,
    LimitSet, LogicalProviders, NamespaceResolutionStacks, NativeBootstrapPurpose,
    NativeCompilationEpochs, NativeCoreRecipe, NativeErrorState, NativeInvocationState,
    NativeRegisteredCommandToken, OutputWriter, PackageState, PendingControl, RECURSION_LIMIT,
    ROOT_INTERP, VarBinding, VarTableOwner, VariableObservers, Vm, VmCommandGuards,
    authored_package, authored_rule_callables, authored_tmm_static, err, err_wrong_args,
    initial_package_prefer, native_operation_currency, ok, standard_host, variable_cleanup,
};
use crate::command::ProcDef;
use crate::frame::CallFrame;
use crate::value::Value;
use crate::vars::{VarArena, VarState as Local, VarTable};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_registry::native_alias_registration::NativeAliasRegistrationTable;
use tcl_runtime_api::guard::{GuardDomain, GuardManager};
use tcl_runtime_api::{Completion, ROOT_NS, VarId};

/// The on-demand autoloader bootstrap (see [`Vm::init_auto_load`]). A focused
/// subset of C's `init.tcl`: `auto_load_index` reads each `tclIndex` on
/// `auto_path` (which sets `auto_index(cmd)` to a `::tcl::Pkg::source <file>`
/// loader), `auto_load` evaluates that loader on first reference, and `unknown`
/// drives it — erroring `invalid command name` when no loader applies. No
/// auto-exec of external programs (the VM is not a shell).
const AUTO_LOAD_BOOTSTRAP: &str = r#"
namespace eval ::tcl::Pkg {}
proc ::tcl::Pkg::source {file} { uplevel #0 [list source $file] }
proc auto_load_index {} {
    global auto_index dir
    foreach dir $::auto_path {
        set f [file join $dir tclIndex]
        if {[file exists $f]} { source $f }
    }
}
proc auto_load {cmd args} {
    global auto_index
    if {![info exists auto_index]} { auto_load_index }
    set bare [string trimleft $cmd :]
    foreach name [list $cmd ::$bare $bare] {
        if {[info exists auto_index($name)]} {
            uplevel #0 $auto_index($name)
            if {[llength [info commands $cmd]]} { return 1 }
            if {[llength [info commands ::$bare]]} { return 1 }
        }
    }
    return 0
}
proc unknown {cmd args} {
    if {[auto_load $cmd]} {
        return [uplevel 1 [linsert $args 0 $cmd]]
    }
    # `unknown` is bootstrap Tcl shared by every emulated release.  `throw`
    # did not exist in 8.4/8.5, so using it here recursively invokes `unknown`
    # precisely when an older profile rejects a newer command.
    return -code error @LOOKUP_ERRORCODE@ "invalid command name \"$cmd\""
}
if {![info exists ::auto_path]} { set ::auto_path {}; catch {lappend ::auto_path [info library]} }
"#;

impl Default for InterpreterCounters {
    fn default() -> Self {
        Self {
            chan_counter: 2,
            interp_counter: 0,
            commands_run: 0,
            limit_tick: 0,
        }
    }
}

impl Vm {
    /// The interpreter constructor: the native core of `profile` names its
    /// commands, over `host`, with the core's root variables and no library or
    /// CLI globals. This backend still registers its supported command
    /// implementations.
    ///
    /// `None` when `profile` has no native core (no measured execution point),
    /// since an interpreter of it could name no command.
    pub fn with_native_core(
        out: Box<dyn Write>,
        host: Rc<dyn tcl_platform::Host>,
        profile: &'static tcl_dialect::DialectProfile,
        inputs: tcl_registry::special_vars::NativeBootstrapInputs,
    ) -> Option<Self> {
        let protocol =
            tcl_registry::InvocationDialect::of_profile(profile).native_bootstrap_protocol()?;
        if protocol.names().tcl_version().is_some() {
            tcl_syntax::list::split_native_list_bytes(
                &inputs.package_path,
                protocol.names().string_protocol(),
            )
            .ok()?;
        }
        Some(Self::with_shared_output_bootstrap(
            Rc::new(RefCell::new(out)),
            (host, profile, protocol, inputs),
        ))
    }

    /// A native-core interpreter of this one's actual engine, over the
    /// platform's standard host, writing to `out`: what a `thread::create` or
    /// `tpool` worker runs on. The engine and bootstrap inputs are captured on
    /// the parent's thread by [`Self::native_core_recipe`].
    pub(crate) fn with_native_core_recipe(
        out: Box<dyn Write>,
        (profile, inputs): NativeCoreRecipe,
    ) -> Option<Self> {
        Self::with_native_core(out, standard_host(), profile, inputs)
    }

    /// The engine profile and bootstrap inputs a sibling interpreter of this
    /// one is built from ([`Self::with_native_core_recipe`]).
    pub(crate) fn native_core_recipe(&self) -> Option<NativeCoreRecipe> {
        let profile = self.actual_engine_profile?;
        Some((profile, self.bootstrap.inputs.clone()))
    }

    fn with_shared_output_bootstrap(
        out: Rc<RefCell<Box<dyn Write>>>,
        (host, profile, protocol, inputs): (
            Rc<dyn tcl_platform::Host>,
            &'static tcl_dialect::DialectProfile,
            tcl_registry::special_vars::NativeBootstrapProtocol,
            tcl_registry::special_vars::NativeBootstrapInputs,
        ),
    ) -> Self {
        let _constructor_allocation =
            crate::value::JimAllocationScope::enter(None, Rc::new(|| true));
        let owner_nonce =
            tcl_runtime_api::native_compilation::NativeInterpreterIdentity::fresh_owner();
        let mut state = Box::new(InterpState::fresh(
            out,
            owner_nonce,
            0,
            NativeBootstrapPurpose {
                protocol,
                inputs: inputs.clone(),
            },
        ));
        state.host = host;
        state.runtime_version = profile.vm_runtime_version;
        state.pin = tcl_registry::model::PinnedContext::for_profile(profile);
        state.command_surface_profile = profile;
        state.actual_engine_profile = Some(profile);
        state.command_surface_point = Some(crate::environment::surface_point(profile));
        state.profile_registry = Some(crate::environment::store_for_profile(profile));
        let system_encoding = state.host.system_encoding();
        let standard_channels = RefCell::new(tcl_cmd_core::channel::StandardChannelConfigs::new(
            state.runtime_version,
            system_encoding,
        ));
        let mut vm = Self {
            owner_nonce,
            state,
            authored_timers: crate::retained_activation::AuthoredTimers::default(),
            authored_tmm_static: authored_tmm_static::AuthoredTmmStaticDomain::default(),
            cur: ROOT_INTERP,
            interps: vec![InterpSlot {
                parked: None,
                active: 0,
                dying: false,
                command_dying: false,
                parent: None,
            }],
            activation_depth: 0,
            execution_refusal: None,
            host_execution_depth: 0,
            deferred_pin: None,
            native_command_depth: 0,
            standard_channels,
            system_encoding,
            default_system_encoding: system_encoding,
            alias_backrefs: Vec::new(),
        };
        tcl_syntax::number::set_runtime_syntax(vm.runtime_version.number_syntax());
        vm.select_dictionary_containers();
        let _root_operation =
            match native_operation_currency::NativeOperationScope::enter_native(&mut vm) {
                Ok(scope) => scope,
                Err(first) => {
                    vm.retain_native_execution_refusal(first);
                    return vm;
                }
            };
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_error_objects_protocol();
        vm.native_errors.error_stack.configure(recipe);
        if let Err(error) = vm.configure_c84_execution_environment_error_names() {
            let _ = crate::command::completion_from_cmd_error(&mut vm, error);
            return vm;
        }
        crate::command::register_builtins_with_native_core(&mut vm, protocol);
        vm.bootstrap_native_core(protocol, &inputs);
        if vm.execution_refusal.is_none() {
            vm.native_core_pin = Some(vm.pin.clone());
        }
        vm.enable_command_semantics_tracking();
        vm
    }
}

impl InterpState {
    /// The guard set a fresh interpreter starts from.
    ///
    /// Interpreter-policy and `TclOO` dispatch mutations do not yet have one
    /// central invalidation owner in this runtime. Fail closed until every
    /// mutation site routes through such an owner; an epoch that is never
    /// advanced must not authorise a speculative path.
    fn fail_closed_guards() -> GuardManager {
        let mut guards = GuardManager::default();
        guards.poison(GuardDomain::Interpreter);
        guards.poison(GuardDomain::ObjectDispatch);
        guards
    }

    /// The fail-closed guards, permissive dialect profile, and matching
    /// command-surface point for an interpreter that has not pinned a dialect.
    fn fresh_semantic_environment() -> FreshSemanticEnvironment {
        let guards = Self::fail_closed_guards();
        let profile = crate::environment::profile_for_dialect("");
        let surface_point = crate::environment::surface_point(profile);
        FreshSemanticEnvironment {
            guards,
            profile,
            surface: surface_point,
        }
    }

    /// A fresh interpreter state writing `puts` output to `out` — no commands
    /// registered yet ([`Vm::with_native_core`] / [`Vm::fork_child`] follow
    /// up with `register_builtins`).
    pub(super) fn fresh(
        out: Rc<RefCell<Box<dyn Write>>>,
        owner: u64,
        interpreter: u64,
        bootstrap: NativeBootstrapPurpose,
    ) -> Self {
        let _constructor_allocation =
            crate::value::JimAllocationScope::enter(None, Rc::new(|| true));
        Self::fresh_in_environment(
            out,
            Self::fresh_semantic_environment(),
            owner,
            interpreter,
            bootstrap,
        )
    }

    fn fresh_literal_world() -> Rc<RefCell<crate::literal_pool::NativeLiteralWorld>> {
        Rc::new(RefCell::new(
            crate::literal_pool::NativeLiteralWorld::default(),
        ))
    }

    fn fresh_compiler_pass_owner(
        owner: u64,
        interpreter: u64,
    ) -> tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner {
        tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner::new(
            tcl_runtime_api::native_compilation::NativeInterpreterIdentity { owner, interpreter },
        )
    }

    #[allow(clippy::too_many_lines)] // one interpreter construction
    fn fresh_in_environment(
        out: OutputWriter,
        environment: FreshSemanticEnvironment,
        owner: u64,
        interpreter: u64,
        bootstrap: NativeBootstrapPurpose,
    ) -> Self {
        Self {
            runtime_version: tcl_dialect::TclVersion::V9_0,
            command_surface_profile: environment.profile,
            framework_builtins: Vec::new(),
            active_native_profile: None,
            actual_engine_profile: None,
            actual_protocols: std::cell::Cell::new(None),
            native_core_pin: None,
            jim_object_context: RefCell::new(None),
            native_c_result: RefCell::new(None),
            native_execution_booleans: None,
            jim_teardown_started: false,
            native_operation_context: Rc::default(),
            logical_providers: LogicalProviders::default(),
            matched_prerequisites: RefCell::new((0, Vec::new())),
            compilation_commands_memo: RefCell::new(None),
            compiler_policy_memo: RefCell::new(None),
            observed_names: None,
            observed_frame_storage: None,
            authored_math: None,
            command_surface_point: Some(environment.surface),
            profile_registry: None,
            profile_generation: 0,
            compiler_generation: 0,
            frames: vec![CallFrame::new(0, ROOT_NS, None, Vec::new())],
            jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStoragePool::default(),
            var_arena: VarArena::default(),
            #[cfg(test)]
            actual_dictionary_search_entries: 0,
            ns_vars: HashMap::new(),
            const_vars: rustc_hash::FxHashSet::default(),
            commands: HashMap::new(),
            name_world: Rc::new(NameWorldCell::new(NativeNameWorld::new(owner, interpreter))),
            alias_registrations: NativeAliasRegistrationTable::default(),
            fixed_math: FixedMathTable::default(),
            module_procs: HashMap::new(),
            resolution_stacks: NamespaceResolutionStacks::default(),
            namespace_ensembles: HashMap::new(),
            scripted_distribution_libraries: HashMap::new(),
            explicit_scripted_libraries: Vec::new(),
            bootstrap,
            native_literal_world: Self::fresh_literal_world(),
            native_compiler_pass_owner: Self::fresh_compiler_pass_owner(owner, interpreter),
            ns_exports: HashMap::new(),
            imported_commands: HashMap::new(),
            builtin_identities: HashMap::new(),
            registry_object_roots: HashMap::new(),
            ns_unknowns: HashMap::new(),
            package_state: PackageState::default(),
            authored_packages: None,
            authored_rule_providers: authored_rule_callables::AuthoredRuleProviders::default(),
            package_table_purpose: authored_package::PackageTablePurpose::Native,
            package_unknown: None,
            package_prefer: initial_package_prefer(),
            variable_observers: VariableObservers::default(),
            next_var_trace_id: 0,
            cmd_traces: HashMap::new(),
            exec_traces: HashMap::new(),
            rename_windows: Vec::new(),
            active_sidecar_handles: Vec::new(),
            exec_step_scopes: Vec::new(),
            compilation_epochs: NativeCompilationEpochs::default(),
            trace_in_progress: std::cell::Cell::new(false),
            firing_cmd_traces: Vec::new(),
            pending_exec_leave: None,
            guards: std::cell::RefCell::new(VmCommandGuards::new(environment.guards)),
            guard_unknown_binding: std::cell::RefCell::new(None),
            guarded_commands: GuardedCommands::default(),
            active_traces: Vec::new(),
            ns_script_frames: Vec::new(),
            out,
            compiler: None,
            pack_facts: Vec::new(),
            initialised_library: None,
            pin: tcl_registry::model::PinnedContext::for_profile(environment.profile),
            debug_hooks: InterpreterDebugHooks::default(),
            pending_exit: None,
            eval_cache: HashMap::new(),
            eval_cache_plain: HashMap::new(),
            native_errors: NativeErrorState::default(),
            jim_errors: JimEvaluationState::default(),
            native_invocation: NativeInvocationState::default(),
            channels: HashMap::new(),
            script_stack: Vec::new(),
            recursion_depth: 0,
            oo_dispatch_depth: 0,
            host: Rc::new(DefaultHost::new()),
            children: HashMap::new(),
            is_safe: false,
            recursion_limit: RECURSION_LIMIT,
            counters: InterpreterCounters::default(),
            hidden: HiddenCommandState::default(),
            debug_frame: false,
            bgerror_handler: Value::string("::tcl::Bgerror"),
            limits: LimitSet::default(),
            rand_seed: 1,
            oo: crate::cmd_oo::OoState::default(),
            coro: crate::cmd_coro::CoroSystem::default(),
            pending: PendingControl::default(),
            events: crate::cmd_event::EventQueue::default(),
            timer_epoch: Rc::new(crate::frame::ActivationIdentity::new()),
            timer_rules: HashMap::new(),
            timer_anchor: None,
            timer_contexts: Vec::new(),
            thread: crate::cmd_thread::ThreadSystem::default(),
        }
    }
}

impl Vm {
    /// Write the release-reporting globals derived from the threaded
    /// runtime version.
    ///
    /// The patch digit comes from [`tcl_dialect::TclVersion::patchlevel`] —
    /// the one table both this VM and `runtime/rust` read, so the two engines
    /// cannot report different patch levels for the same emulated release.
    pub(super) fn write_release_globals(&mut self) {
        self.write_host_bootstrap_scalar(
            "tcl_version",
            Value::string(self.runtime_version.as_package_version()),
        );
        self.write_host_bootstrap_scalar(
            "tcl_patchLevel",
            Value::string(self.runtime_version.patchlevel()),
        );
    }

    pub(super) fn bootstrap_native_core(
        &mut self,
        protocol: tcl_registry::special_vars::NativeBootstrapProtocol,
        inputs: &tcl_registry::special_vars::NativeBootstrapInputs,
    ) {
        use tcl_registry::special_vars::{NativeBootstrapPurpose, NativeBootstrapVariable as V};
        let snapshot = tcl_platform::bootstrap::snapshot(
            &*self.host_rc(),
            "bytecode",
            env!("CARGO_PKG_VERSION"),
        );
        for variable in protocol
            .allocations(
                NativeBootstrapPurpose::CreateInterpreter,
                inputs.default_library.is_some(),
            )
            .expect("selected native core allocation plan")
        {
            let name = variable.name();
            match variable {
                V::ErrorInfo | V::ErrorCode | V::Precision => {
                    let (_, id) = self
                        .ensure_base_var_from(name, 0)
                        .expect("selected native root shell");
                    if variable == V::Precision {
                        self.variable_observers.precision_cell = Some(id);
                    }
                }
                V::Environment | V::Platform => {
                    self.ensure_array(name).expect("fresh native root array");
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
                        self.write_array_raw(
                            name,
                            key,
                            Value::new_native_string_bytes(value.as_bytes()),
                        )
                        .expect("fresh native platform member");
                    }
                }
                V::PatchLevel => self.write_scalar_raw(
                    name,
                    Value::new_native_string_bytes(self.runtime_version.patchlevel().as_bytes()),
                ),
                V::Version => self.write_scalar_raw(
                    name,
                    Value::new_native_string_bytes(
                        self.runtime_version.version_string().as_bytes(),
                    ),
                ),
                V::PackagePath => {
                    let members = tcl_syntax::list::split_native_list_bytes(
                        &inputs.package_path,
                        protocol.names().string_protocol(),
                    )
                    .expect("validated build package path");
                    let members = members
                        .into_iter()
                        .map(|member| Value::new_native_string_bytes(member.into_owned()))
                        .collect();
                    self.write_scalar_raw(
                        name,
                        Value::native_list_constructor(members, protocol.names().string_protocol()),
                    );
                }
                V::AutoPath => self.write_scalar_raw(
                    name,
                    Value::new_native_string_bytes(inputs.package_path.as_slice()),
                ),
                V::DefaultLibrary => self.write_scalar_raw(
                    name,
                    Value::new_native_string_bytes(
                        inputs
                            .default_library
                            .as_deref()
                            .expect("selected platform default library"),
                    ),
                ),
                V::Interactive => {
                    self.write_scalar_raw(name, Value::new_native_string_bytes(b"0".as_slice()));
                }
                V::Argv0 | V::Argc | V::Argv => {
                    unreachable!("constructor plan excludes main arguments")
                }
            }
        }
        self.install_native_error_variable_traces();
        self.install_native_env_trace();
    }

    /// Fixed host slots are already selected root-table keys, not guest names.
    fn write_host_bootstrap_scalar(&mut self, name: &'static str, value: Value) {
        if self.actual_engine_profile.is_some() || self.active_native_profile.is_some() {
            self.write_scalar_raw(name, value);
            return;
        }
        let binding = VarBinding {
            owner: VarTableOwner::Namespace(ROOT_NS),
            name: NameBytes::from(name),
        };
        let resolved = self
            .resolve_binding_var(binding, None, &mut HashSet::new())
            .expect("host root slot");
        self.write_scalar_resolved_raw(resolved, value);
    }

    fn install_host_bootstrap_array(&mut self, name: &'static str) -> VarId {
        let binding = VarBinding {
            owner: VarTableOwner::Namespace(ROOT_NS),
            name: NameBytes::from(name),
        };
        self.bind_new_var(&binding, Local::Array(VarTable::new()))
            .expect("host root array allocation")
    }

    fn write_host_bootstrap_element(&mut self, parent: VarId, key: &str, value: &str) {
        let existing = match self.var_arena.get(parent).map(crate::vars::VarCell::state) {
            Some(Local::Array(elements)) => elements.get(key).copied(),
            _ => unreachable!("host-owned array"),
        };
        if let Some(id) = existing {
            let _ = self
                .var_arena
                .replace_state(id, Local::Scalar(Value::string(value)));
            return;
        }
        let id = self
            .var_arena
            .alloc_element(Local::Scalar(Value::string(value)), parent, key)
            .expect("host array member allocation");
        if self.var_arena.array_insert(parent, key, id) {
            self.var_arena.bind(id);
        } else {
            self.var_arena.discard_unbound(id);
        }
    }

    pub(super) fn unset_global_raw(&mut self, name: &str) {
        let key = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
        let id = self
            .frames
            .first_mut()
            .and_then(|global| global.locals.remove(key));
        if let Some(id) = id {
            let original = variable_cleanup::OriginalVariableCleanup::enter(self);
            original.purge_binding(self, id);
        }
    }

    pub(super) fn rebootstrap_host_globals(&mut self) {
        // The embedder's own bookkeeping, not a store a body made: a host
        // swapped in after the stores were confined still gets its globals.
        let confined = std::mem::replace(&mut self.limits.confined_stores, false);
        let snapshot = tcl_platform::bootstrap::snapshot(
            &*self.host_rc(),
            "bytecode",
            env!("CARGO_PKG_VERSION"),
        );
        for name in tcl_platform::bootstrap::HOST_ARRAYS {
            self.unset_global_raw(name);
        }
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.unset_global_raw(name);
        }
        if self.actual_engine_profile.is_none() && self.active_native_profile.is_none() {
            let platform = self.install_host_bootstrap_array("tcl_platform");
            let environment = self.install_host_bootstrap_array("env");
            self.write_host_bootstrap_scalar("tcl_library", Value::string(snapshot.tcl_library()));
            self.write_host_bootstrap_scalar("auto_path", Value::list(Vec::new()));
            for (name, value) in snapshot.platform() {
                self.write_host_bootstrap_element(platform, name, value);
            }
            for (name, value) in snapshot.environment() {
                self.write_host_bootstrap_element(environment, name, value);
            }
            self.limits.confined_stores = confined;
            self.install_native_env_trace();
            return;
        }
        // A refused root store (a pending refusal or an observed name policy)
        // keeps its first cause; the replaced host's globals stay withdrawn.
        if self.ensure_array("::tcl_platform").is_err() || self.ensure_array("::env").is_err() {
            self.limits.confined_stores = confined;
            return;
        }
        self.write_scalar_raw("::tcl_library", Value::string(snapshot.tcl_library()));
        self.write_scalar_raw("::auto_path", Value::list(Vec::new()));
        for (name, value) in snapshot.platform() {
            let _ = self.write_array_raw("::tcl_platform", name, Value::string(value.as_str()));
        }
        for (name, value) in snapshot.environment() {
            let _ = self.write_array_raw("::env", name, Value::string(value.as_str()));
        }
        self.limits.confined_stores = confined;
        self.install_native_env_trace();
    }

    /// Install the on-demand autoloader: `unknown` / `auto_load` /
    /// `auto_load_index` procs plus `auto_path`, so an unresolved command is
    /// looked up in the library's `tclIndex` and its defining file sourced
    /// (`word.tcl`, etc.) — the mechanism C bootstraps in `init.tcl`. The VM has
    /// no full `init.tcl` path, so this is a focused subset: it does auto-load
    /// (no auto-exec of external programs) and otherwise errors `invalid command
    /// name`, matching C's miss. Requires a compiler (uses `eval_source`); call
    /// after [`Self::set_compiler`]. Returns the bootstrap's completion.
    pub fn init_auto_load(&mut self) -> Completion<Value> {
        let errorcode = if self.runtime_version.has_lookup_error_code("COMMAND") {
            "-errorcode [list TCL LOOKUP COMMAND $cmd]"
        } else {
            ""
        };
        let bootstrap = AUTO_LOAD_BOOTSTRAP.replace("@LOOKUP_ERRORCODE@", errorcode);
        // The bootstrap's caught errors are its own: a script starts with
        // `errorInfo` and `errorCode` as the interpreter had them.
        let mut prior = Vec::new();
        for name in ["::errorInfo", "::errorCode"] {
            match self.read_var_traced(name) {
                Ok(value) => prior.push((name, value)),
                Err(completion) => return completion,
            }
        }
        let completion = match self.eval_source(&bootstrap) {
            Ok(c) => c,
            Err(e) => crate::command::completion_from_tcl_error(self, e),
        };
        // 8.4's interpreter starts with both variables empty; from 8.5 they
        // are absent until an error sets them. Measured on tclsh 8.4.20-9.1.
        let empty = self.runtime_version == tcl_dialect::TclVersion::V8_4;
        for (name, value) in prior {
            match value.or_else(|| empty.then(Value::empty)) {
                Some(value) => {
                    if let Err(completion) = self.store_var_result(name, value) {
                        return completion;
                    }
                }
                None => {
                    self.unset_var(name);
                }
            }
        }
        self.reset_error_state_for_eval();
        completion
    }

    pub(crate) fn init_auto_path(&mut self, args: &[Value]) -> Completion<Value> {
        if !args.is_empty() {
            return err_wrong_args(self, "tcl::InitAutoPath");
        }
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let result = self.init_auto_path_entered(&scope);
        if let Err(error) = scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        match result {
            Ok(value) => ok(value),
            Err(error) => error,
        }
    }

    fn init_auto_path_entered(
        &mut self,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<Value, Completion<Value>> {
        let existing = self.read_var_traced("::auto_path").unwrap_or_default();
        scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        if self.is_safe() {
            if existing.is_none() {
                self.store_var_result("::auto_path", Value::empty())?;
            }
            return Ok(Value::empty());
        }
        let existing = existing
            .as_ref()
            .map(|value| self.auto_path_list(value))
            .transpose()?;
        let mut environment = Vec::new();
        if existing.is_none()
            && let Some(value) = self
                .read_var_traced("::env(TCLLIBPATH)")
                .unwrap_or_default()
        {
            environment = self.auto_path_list(&value)?;
            let home = self.host_rc().env().get("HOME");
            scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            environment = tcl_cmd_core::platform::auto_path_environment(
                environment,
                home.as_deref().map(str::as_bytes),
                tcl_cmd_core::path::PathReading::of_release(self.runtime_version()),
            )
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        }
        let library = self.read_var_traced("::tcl_library").unwrap_or_default();
        let library = library
            .as_ref()
            .map(|value| {
                tcl_syntax::value::ValueOps::native_string_bytes(self, value)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))
            })
            .transpose()?;
        let executable = self.host_rc().env().current_exe();
        scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let installation = tcl_cmd_core::platform::auto_path_installation(
            library.as_deref(),
            executable.as_deref().map(str::as_bytes),
            tcl_cmd_core::path::PathReading::of_release(self.runtime_version()),
        );
        let packages = self
            .read_var_traced("::tcl_pkgPath")
            .unwrap_or_default()
            .as_ref()
            .and_then(|value| self.auto_path_list(value).ok())
            .unwrap_or_default();
        scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let paths = tcl_platform::bootstrap::auto_path(
            existing,
            &environment,
            &installation,
            &packages,
            false,
        );
        let elements = paths
            .into_iter()
            .map(Value::new_native_string_bytes)
            .collect();
        let value = tcl_syntax::value::ValueOps::new_list(self, elements);
        self.store_var_result("::auto_path", value)
    }

    fn auto_path_list(&mut self, value: &Value) -> Result<Vec<Vec<u8>>, Completion<Value>> {
        let elements = tcl_syntax::value::ValueOps::list_elements(self, value)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        elements
            .iter()
            .map(|element| {
                tcl_syntax::value::ValueOps::native_string_bytes(self, element)
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))
            })
            .collect()
    }

    /// Initialise the selected Tcl script library through the same compiler and
    /// evaluator used by `eval`, command substitutions, and ordinary `source`.
    ///
    /// The host selects the exact library through `TCL_LIBRARY`. A compiler
    /// must have been installed with [`Self::set_compiler`].
    pub fn init_library(&mut self) -> Completion<Value> {
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        // `tclInit` tries an existing `tcl_library` first, then a non-empty
        // `env(TCL_LIBRARY)`.
        let selected = match self.read_var_traced("::tcl_library").unwrap_or_default() {
            Some(value) => match tcl_syntax::value::ValueOps::native_string_bytes(self, &value) {
                Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
                Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
            },
            None => None,
        };
        let library =
            selected.unwrap_or_else(|| self.host_rc().env().get("TCL_LIBRARY").unwrap_or_default());
        if let Err(error) = scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        if library.is_empty() {
            return err("can't find Tcl library: TCL_LIBRARY is empty");
        }
        let init_path = format!("{library}/init.tcl");
        let exists = self
            .host_rc()
            .filesystem()
            .and_then(|fs| fs.read(&init_path).ok())
            .is_some();
        if let Err(error) = scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        if !exists {
            return err(format!("can't find {init_path} (set TCL_LIBRARY)"));
        }
        if let Err(completion) = self.set_var_bytes(
            b"::tcl_library",
            Value::new_native_string_bytes(library.as_bytes()),
        ) {
            return completion;
        }
        if let Err(error) = scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        let script = format!("source {}", tcl_syntax::list::list_element(&init_path));
        let initialization = self.begin_package_initialization();
        let completion = match self.eval_source(&script) {
            Ok(completion) => completion,
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        self.end_package_initialization(initialization);
        if let Err(error) = scope.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        if completion.code.is_ok() {
            self.state.initialised_library =
                Some(Value::new_native_string_bytes(library.as_bytes()));
        }
        completion
    }

    /// Source the selected real Tcl library while retaining operational
    /// compiler and value-access refusals outside guest completion.
    ///
    /// # Errors
    /// Returns the reached host refusal after restoring the execution boundary.
    pub fn try_init_library(
        &mut self,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = self.init_library();
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }
}

impl Vm {
    /// Register real library procedure source without inventing an executable
    /// compiler result. Its first activation uses the ordinary lazy admission.
    pub(crate) fn register_bootstrap_procedure(
        &mut self,
        name: &str,
        parameter_source: &str,
        source: &str,
    ) -> Option<(String, NativeRegisteredCommandToken)> {
        self.register_selected_bootstrap_procedure(name, parameter_source, source, None, &mut None)
    }

    fn register_selected_bootstrap_procedure(
        &mut self,
        name: &str,
        parameter_source: &str,
        source: &str,
        helper: Option<tcl_registry::native_jim_lookup::NativeJimNamespaceHelper>,
        source_filename: &mut Option<Value>,
    ) -> Option<(String, NativeRegisteredCommandToken)> {
        // Library declarations belong to the retained native root namespace.
        self.push_ns_token(NameBytes::default(), ROOT_NS);
        let installed = self.register_bootstrap_procedure_in_current_namespace(
            name,
            parameter_source,
            source,
            helper,
            source_filename,
        );
        self.pop_ns();
        installed
    }

    fn register_bootstrap_procedure_in_current_namespace(
        &mut self,
        name: &str,
        parameter_source: &str,
        source: &str,
        helper: Option<tcl_registry::native_jim_lookup::NativeJimNamespaceHelper>,
        source_filename: &mut Option<Value>,
    ) -> Option<(String, NativeRegisteredCommandToken)> {
        let _scope = native_operation_currency::NativeOperationScope::enter(self).ok()?;
        let grammar = self.native_invocation_dialect().parameter_grammar()?;
        let (registered, slot, ns_id) = self.native_procedure_publication(name.as_bytes()).ok()?;
        if self.command_at_exact_slot(&slot, false).is_some() {
            return None;
        }
        let parameter_value = Value::from_native_string_bytes(parameter_source.as_bytes().to_vec());
        let source_value = Value::from_native_string_bytes(source.as_bytes().to_vec());
        let (params, has_args) =
            crate::command::parse_params_value(self, &parameter_value, name.as_bytes())
                .expect("audited native library formal declaration");
        if let Some(helper) = helper {
            let source = helper.static_extension_source();
            let adopted = (|| -> Result<(), tcl_syntax::value::ValueError> {
                let context = self.native_jim_object_context()?;
                if source_filename.is_none() {
                    let filename = Value::new_native_string_bytes(source.filename());
                    filename.bind_native_jim_context(&context)?;
                    *source_filename = Some(filename);
                }
                source_value.install_native_jim_source(
                    crate::value::NativeJimSourceInfo {
                        filename: source_filename
                            .as_ref()
                            .expect("selected source filename")
                            .clone(),
                        line: source.first_line(),
                    },
                    &context,
                )
            })();
            if let Err(error) = adopted {
                let _ = self.original_name_cmd_error(error.into());
                return None;
            }
        }
        let table_key = self
            .jim_command_table_key_for_original(
                name.as_bytes(),
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .ok()?;
        self.define_proc_binding_with_jim_key_and_namespace_helper(
            ProcDef {
                formals: crate::command::FormalParameterCache::default(),
                native_resources: Rc::default(),
                name: registered.clone(),
                command_ns_id: slot.namespace,
                simple_name: slot.simple.clone(),
                execution_namespace: ProcedureNamespace::Resolved {
                    path: self.ns_path(ns_id),
                    token: ns_id,
                },
                params,
                parameter_grammar: grammar,
                has_args,
                native_jim_namespace: None,
                native_parameters: (grammar == tcl_dialect::ParameterGrammar::Jim)
                    .then_some(parameter_value),
                native_header: tcl_registry::native_procedure::procedure_header_compilation_bytes(
                    self.native_invocation_dialect(),
                    Some(parameter_source.as_bytes()),
                    Some(source.as_bytes()),
                    Some(false),
                ),

                statics: None,
                body: None,
                body_src: source_value,
                usage_name: None,
                call_identity: None,
                lambda_term: None,
            },
            table_key,
            helper,
        );
        let token = NativeRegisteredCommandToken {
            owner: self.owner_nonce,
            interpreter: u64::try_from(self.cur.0).expect("interpreter token fits u64"),
            slot,
            generation: self.visible_command_generation(&registered)?,
        };
        Some((registered, token))
    }

    /// Bootstrap each real scripted procedure once per selected distribution library.
    /// Replacement and deletion remain visible through subsequent profile repinning.
    pub(crate) fn refresh_scripted_distribution_libraries(&mut self) {
        for library in self.explicit_scripted_libraries.clone() {
            self.install_selected_scripted_library(library);
        }
    }

    /// Explicit native source-library entry. Each roster retains the original
    /// registered generations, independently of core registration and lookup.
    pub(crate) fn install_scripted_library(
        &mut self,
        library: tcl_registry::native_scripted_distribution::NativeScriptedLibrary,
    ) {
        if !self.explicit_scripted_libraries.contains(&library)
            && tcl_registry::native_scripted_distribution::procedures(
                self.actual_native_invocation_dialect(),
                library,
            )
            .next()
            .is_some()
        {
            self.explicit_scripted_libraries.push(library);
        }
        self.install_selected_scripted_library(library);
    }

    fn install_selected_scripted_library(
        &mut self,
        library: tcl_registry::native_scripted_distribution::NativeScriptedLibrary,
    ) {
        let procedures: Vec<_> = tcl_registry::native_scripted_distribution::procedures(
            self.actual_native_invocation_dialect(),
            library,
        )
        .collect();
        if procedures.is_empty() {
            if let Some(installed) = self.scripted_distribution_libraries.remove(&library) {
                for (key, token) in installed {
                    if self.command_slot(&key).as_ref() == Some(&token.slot)
                        && self.visible_command_generation(&key) == Some(token.generation)
                    {
                        self.remove_command_exact(&key);
                    }
                }
            }
            return;
        }
        if self.scripted_distribution_libraries.contains_key(&library) {
            return;
        }
        let mut installed = Vec::new();
        let mut source_filename = None;
        for procedure in procedures {
            if let Some(binding) = self.register_selected_bootstrap_procedure(
                procedure.command,
                procedure.parameters,
                procedure.body,
                procedure.namespace_helper,
                &mut source_filename,
            ) {
                installed.push(binding);
            }
        }
        self.scripted_distribution_libraries
            .insert(library, installed);
    }
}
