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

//! Creating an interpreter: the default host and release, the constructors and the state they allocate.

use super::super::limits::{LimitSet, RECURSION_LIMIT};
use super::super::native_error_headers::{NativeErrorStack, NativeReturnOptions};
use super::super::{
    native_compilation, native_operation_currency, native_unicode_getter, stock_ensembles,
    CmdArena, Code, ExceptionState, Interp, InterpState,
};
use crate::frame::FrameStack;
#[cfg(test)]
use crate::interp::constructor_diagnostics::ConstructorDiagnostics;
#[cfg(test)]
use crate::interp::constructor_diagnostics::Kind;
use crate::namespace::{Namespaces, GLOBAL};
use crate::{builtins, obj, parse};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use tcl_registry::special_vars::BootstrapDialect;
use tcl_runtime_api::guard::{GuardDomain, GuardManager, OwnedGuardManager};
use tcl_runtime_api::jim_error_stack::JimErrorStack;
use tcl_runtime_api::CommandEvaluation;

/// The release a fresh [`Interp`] emulates until an embedder pins another with
/// [`Interp::set_runtime_version`] — the same default as `tcl_vm::Vm` and as
/// `tcl_syntax::number`'s ambient grammar, so an interpreter nobody configures
/// reads numerals exactly as the release it reports.
pub(in super::super) const DEFAULT_RUNTIME_VERSION: tcl_dialect::TclVersion =
    tcl_dialect::TclVersion::V9_0;

/// Install `version`'s build-time facts as this thread's ambient ones: the C
/// regex engine's implicit compile flags (see
/// [`crate::regex_capi::set_runtime_release`]), and the numeric-literal
/// grammar, so every numeral this runtime reads (`expr`, `format`, `dict`, `incr`, the
/// bignum tower — all of which parse through `tcl_syntax::number::parse_whole`
/// with `ParseFlags::default()`) follows the emulated release: `0755` is 493
/// under 8.4/8.6 and 755 under 9.0, `0b`/`0o` exist from 8.5, and `0d` plus `_`
/// digit separators from 9.0.
///
/// C settles this at build time (`#define`/`#undef KILL_OCTAL` in
/// `tclStrToD.c`), so it is a property of the runtime rather than of each
/// conversion — hence ambient state rather than an argument threaded through
/// every `Tcl_GetIntFromObj`-shaped call. Called from [`Interp::new`] and
/// [`Interp::set_runtime_version`], i.e. everywhere a release is established.
pub(super) fn install_ambient_release(version: tcl_dialect::TclVersion) {
    tcl_syntax::number::set_runtime_syntax(version.number_syntax());
    crate::regex_capi::set_runtime_release(version);
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    parse::install_native_list_policy(
        dialect.lexer_grammar.list_parse,
        dialect.lexer_grammar.escapes,
    );
    obj::install_double_string_policy(tcl_dialect::DoubleStringPolicy::for_tcl_version(version));
    tcl_syntax::number::set_runtime_release(version);
}

pub(crate) fn default_host() -> Rc<dyn tcl_platform::Host> {
    #[cfg(test)]
    let mut constructor_diagnostic = ConstructorDiagnostics::new(Kind::Host);
    #[cfg(not(target_arch = "wasm32"))]
    let host = Rc::new(tcl_host_native::NativeHost::new()) as Rc<dyn tcl_platform::Host>;
    #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
    let host = Rc::new(crate::host_wasm::WasiHost::new()) as Rc<dyn tcl_platform::Host>;
    #[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
    let host = Rc::new(crate::host_wasm::BrowserHost::new()) as Rc<dyn tcl_platform::Host>;
    #[cfg(test)]
    constructor_diagnostic.stage("host_ready");
    host
}

/// The shipped command surfaces, installed at interpreter creation unless the
/// host already installed its own set.
fn install_surfaces() {
    if tcl_registry::try_installed().is_none() {
        let _ = tcl_registry::install_shipped();
    }
}

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result, and the predefined variables that C installs in
    /// `Tcl_CreateInterp`.
    pub fn new() -> Interp {
        Self::with_host(default_host())
    }

    /// Create an interpreter whose first bootstrap reads from `host`.
    ///
    /// Restricted and synthetic embedders should prefer this constructor so
    /// no process-host values are ever installed, even transiently.
    pub fn with_host(host: Rc<dyn tcl_platform::Host>) -> Interp {
        Self::with_host_bootstrap(host, None)
    }

    /// Construct the selected native core variable inventory before any library
    /// or CLI arguments. Host/build path bytes are supplied independently.
    /// The registered command set remains this backend's supported commands.
    pub fn with_native_core(
        host: Rc<dyn tcl_platform::Host>,
        profile: &'static tcl_dialect::DialectProfile,
        inputs: tcl_registry::special_vars::NativeBootstrapInputs,
    ) -> Option<Interp> {
        install_surfaces();
        let protocol =
            tcl_native_model::InvocationDialect::of_profile(profile).native_bootstrap_protocol()?;
        if protocol.names().tcl_version().is_some() {
            tcl_syntax::list::split_native_list_bytes(
                &inputs.package_path,
                protocol.names().string_protocol(),
            )
            .ok()?;
        }
        Some(Self::with_host_bootstrap(
            host,
            Some((profile, protocol, inputs)),
        ))
    }

    fn with_host_bootstrap(
        host: Rc<dyn tcl_platform::Host>,
        native: Option<(
            &'static tcl_dialect::DialectProfile,
            tcl_registry::special_vars::NativeBootstrapProtocol,
            tcl_registry::special_vars::NativeBootstrapInputs,
        )>,
    ) -> Interp {
        install_surfaces();
        #[cfg(test)]
        let mut constructor_diagnostic = ConstructorDiagnostics::new(Kind::Bootstrap);
        let profile = native.as_ref().map_or_else(
            || crate::environment::profile_for_dialect(""),
            |(profile, _, _)| *profile,
        );
        let version = native
            .as_ref()
            .map_or(DEFAULT_RUNTIME_VERSION, |(profile, _, _)| {
                profile.vm_runtime_version
            });
        #[cfg(test)]
        constructor_diagnostic.stage("profile_selected");
        let _constructor_allocation = obj::JimAllocationScope::enter(None, Rc::new(|| true));
        let result = obj::new_obj();
        let system_encoding = host.system_encoding();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut guards = GuardManager::default();
        // Object dispatch still has mutation sites spread through the TclOO
        // engine. Fail closed until those sites have one mutation owner. The
        // interpreter-policy surface is centralised on
        // `invalidate_interpreter_policy` below and can issue live guards.
        guards.poison(GuardDomain::ObjectDispatch);
        let mut interp = Interp(Rc::new(InterpState {
            native_literal_world: Rc::new(RefCell::new(Default::default())),
            native_literal_arrays: RefCell::new(Vec::new()),
            native_execution_constants: RefCell::new(None),
            native_command_interpreter:
                tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner:
                        tcl_runtime_api::native_compilation::NativeInterpreterIdentity::fresh_owner(
                        ),
                    interpreter: 0,
                },
            native_compiler_pass_owner: std::cell::OnceCell::new(),
            jim_local_depth: Cell::new(0),
            jim_active_command_workers: RefCell::new(Vec::new()),
            jim_command_references: RefCell::new(Vec::new()),
            frames: RefCell::new(FrameStack::new()),
            namespaces: RefCell::new(Namespaces::new()),
            native_alias_registrations: RefCell::new(Default::default()),
            array_operation_targets: RefCell::new(Vec::new()),
            active_var_trace_scopes: RefCell::new(Vec::new()),
            guards: RefCell::new(OwnedGuardManager::new(guards)),
            guard_lookup_environment_revision: Cell::new(Some(0)),
            guard_command_lookup_policy_revision: Cell::new(Some(0)),
            guard_unknown_binding: Cell::new(None),
            guarded_definitions: RefCell::new(Vec::new()),
            guarded_commands: RefCell::new(std::collections::BTreeMap::new()),
            registry_builtin_names: RefCell::new(std::collections::HashMap::new()),
            original_byte_command_registrations: RefCell::new(std::collections::HashMap::new()),
            original_byte_command_registration_revision: RefCell::new(None),
            native_compilation: native_compilation::CompilationCell::default(),
            stock_ensembles: RefCell::new(stock_ensembles::StockEnsembles::default()),
            unsupported_builtins: RefCell::new(std::collections::HashSet::new()),
            current_ns: Cell::new(GLOBAL),
            recursion_depth: Cell::new(0),
            recursion_limit: Cell::new(RECURSION_LIMIT),
            packages: RefCell::new(crate::cmd_package::PackageState::with_core(version)),
            initialised_library: RefCell::new(None),
            script_stack: RefCell::new(Vec::new()),
            channels: RefCell::new(crate::cmd_chan::ChannelTable::new(version, system_encoding)),
            return_code: Cell::new(Code::Ok),
            return_level: Cell::new(1),
            return_options: RefCell::new(NativeReturnOptions::default()),
            traces: RefCell::new(crate::cmd_trace::TraceTable::default()),
            native_error_cells: RefCell::new(Vec::new()),
            native_precision_cell: Cell::new(None),
            exc: RefCell::new(ExceptionState::default()),
            c_api_errors: Cell::new(0),
            error_line: Cell::new(1),
            children: RefCell::new(std::collections::BTreeMap::new()),
            interp_counter: Cell::new(0),
            hidden: RefCell::new(std::collections::BTreeMap::new()),
            parent: RefCell::new(Weak::new()),
            native_child_interpreter: Cell::new(false),
            native_child_command_generation: Cell::new(None),
            is_safe: Cell::new(false),
            eval_active: Cell::new(0),
            pending_delete: Cell::new(false),
            host: RefCell::new(host),
            oo: Rc::new(RefCell::new(crate::cmd_oo::OoState::default())),
            cmd_frames: RefCell::new(Vec::new()),
            arg_lines: RefCell::new(Vec::new()),
            arg_locs: RefCell::new(Vec::new()),
            evaluation: Cell::new(CommandEvaluation::Direct),
            next_evaluation: Cell::new(None),
            compiled_literals: RefCell::new(Vec::new()),
            proc_body: Cell::new(false),
            next_proc_body: Cell::new(false),
            next_top_level_file: Cell::new(false),
            compiled_control_body: Cell::new(None),
            namespace_frame_command: Cell::new("eval"),
            eval_depth: Cell::new(0),
            var_trace_epoch: Cell::new(1),
            cmd_count: Cell::new(0),
            native_proc_dispatches: Cell::new(0),
            exit_code: Cell::new(None),
            measure_overhead: Cell::new(0.0),
            bgerror: RefCell::new(None),
            bg_queue: RefCell::new(Vec::new()),
            events: RefCell::new(crate::cmd_event::EventQueue::default()),
            coros: RefCell::new(std::collections::BTreeMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            coro_probe_error: RefCell::new(None),
            ensemble_rewrite: RefCell::new(None),
            handler_usage_adapters: RefCell::new(Vec::new()),
            #[cfg(have_tommath)]
            rand_seed: Cell::new(None),
            error_stack: RefCell::new(NativeErrorStack::default()),
            jim_error_stack: RefCell::new(JimErrorStack::default()),
            jim_evaluation_frames: RefCell::new(Vec::new()),
            jim_invocation_borrows: RefCell::new(Vec::new()),
            jim_procedure_level: Cell::new(0),
            native_dispatch_depth: Cell::new(0),
            native_operation_context_epoch: Cell::new(Some(0)),
            entered_native_operations: RefCell::new(Vec::new()),
            native_unicode_getter: RefCell::new(None),
            deferred_tailcalls: RefCell::new(Vec::new()),
            during: Cell::new(None),
            result: Cell::new(result),
            cmd_arena: RefCell::new(CmdArena::default()),
            limits: RefCell::new(LimitSet::default()),
            budget: RefCell::new(crate::budget::Budget::default()),
            stores_confined: Cell::new(false),
            limit_tick: Cell::new(0),
            debug_frame: Cell::new(false),
            runtime_version: Cell::new(version),
            invocation_dialect: Cell::new(None),
            invocation_protocols: Cell::new(None),
            jim_object_context: RefCell::new(None),
            jim_teardown_started: Cell::new(false),
            // The "no dialect pinned" ingress: the lenient environment,
            // whose unit profile is the permissive fallback that hides
            // nothing. `set_dialect_profile` replaces all three together.
            dialect_profile: Cell::new(profile),
            observed_names: RefCell::new(None),
            observed_frame_storage: RefCell::new(None),
            logical_name_provider: Cell::new(None),
            logical_expression_parse_provider: Cell::new(None),
            logical_source_word_provider: Cell::new(None),
            logical_eval_object_provider: Cell::new(None),
            profile_registry: Cell::new(
                native
                    .as_ref()
                    .map(|(profile, _, _)| crate::environment::store_for_profile(profile)),
            ),
            dialect_point: Cell::new(Some(crate::environment::surface_point(profile))),
            pin: RefCell::new(tcl_registry::model::PinnedContext::for_profile(profile)),
            registry_object_roots: RefCell::new(std::collections::HashMap::new()),
            builtin_surface_visibility: RefCell::default(),
            retiring_oo_commands: RefCell::new(std::collections::HashSet::new()),
            original_command_retirement_recipes: RefCell::new(Vec::new()),
        }));
        #[cfg(test)]
        constructor_diagnostic.stage("state_allocated");
        interp.configure_variable_storage();
        interp.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        interp.namespaces.borrow_mut().set_native_command_version(
            interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .map(|protocol| protocol.version()),
        );
        if native.is_some() {
            let dialect = interp.native_invocation_dialect();
            parse::install_native_list_policy(
                dialect.lexer_grammar.list_parse,
                dialect.lexer_grammar.escapes,
            );
            if let Some(policy) = profile.double_string_policy() {
                obj::install_double_string_policy(policy);
            }
        }
        // The numeric grammar is thread-ambient and may have been left on
        // another release by an interpreter built earlier on this thread, so a
        // fresh interp installs its own rather than inheriting whatever is
        // there (`set_runtime_version` re-installs when an embedder repins).
        install_ambient_release(version);
        let _root_operation = match native_operation_currency::NativeOperationScope::enter(&interp)
        {
            Ok(scope) => scope,
            Err(first) => {
                interp.refuse_native_execution(first);
                return interp;
            }
        };
        if let Err(first) = interp.configure_native_error_stack_checked() {
            interp.refuse_native_execution(first);
            return interp;
        }
        #[cfg(test)]
        constructor_diagnostic.stage("policies_ready");
        if let Some((_, protocol, _)) = &native {
            builtins::install_native_core(&mut interp, *protocol);
        } else {
            builtins::install(&mut interp);
        }
        #[cfg(test)]
        constructor_diagnostic.stage("builtins_ready");
        interp.install_jim_local_commands();
        #[cfg(test)]
        constructor_diagnostic.stage("locals_ready");
        interp.seal_native_compiler_tokens();
        #[cfg(test)]
        constructor_diagnostic.stage("compiler_sealed");

        // C sets `tcl_version`/`tcl_patchLevel` in `Tcl_CreateInterp`
        // (9.0.4 `generic/tclBasic.c:1346-1347`), **not** in `Tcl_Init` — so
        // they exist in an interpreter that never sources `init.tcl`.
        // The ordinary embedding host also supplies application and library
        // globals. The explicit native constructor consumes only its selected
        // physical root allocation plan.
        if let Some((_, protocol, inputs)) = native {
            // The constructor's original entry covers both root publication
            // and its final result. Failed bootstrap cannot publish success.
            let scope = match native_operation_currency::NativeOperationScope::enter(&interp) {
                Ok(scope) => scope,
                Err(first) => {
                    interp.refuse_native_execution(first);
                    return interp;
                }
            };
            let bootstrapped = interp.bootstrap_native_core(protocol, &inputs);
            let current = scope.currency().ensure_current_or_refuse();
            if bootstrapped.is_err() || current.is_err() {
                return interp;
            }
            if protocol.names().string_protocol()
                == tcl_syntax::native_string::NativeStringProtocol::Jim084
            {
                let context = interp
                    .native_jim_object_context()
                    .map_err(|error| interp.report_cmd_error(error.into()));
                let context = match interp.checked_bootstrap_outcome(scope.currency(), context) {
                    Ok(context) => context,
                    Err(_) => return interp,
                };
                if scope.currency().ensure_current_or_refuse().is_err() {
                    return interp;
                }
                interp.set_result(context.empty_object().as_ptr());
            } else {
                // Tcl_CreateInterp returns its fresh untyped empty result.
                interp.set_result_bytes(b"");
            }
            if scope.currency().ensure_current_or_refuse().is_err() {
                return interp;
            }
            // The actual backend and original constructor have both completed.
            // Generic construction/profile changes never issue this entry.
            let string_protocol = protocol.names().string_protocol();
            if string_protocol.tcl_version().is_some() {
                let entry = match native_unicode_getter::NativeUnicodeGetterEntry::issue_after_native_constructor(
                    &interp, string_protocol,
                ) {
                    Ok(entry) => entry,
                    Err(_) => return interp,
                };
                *interp.native_unicode_getter.borrow_mut() = Some(Rc::new(entry));
            }
        } else {
            interp.set_startup_globals();
        }
        #[cfg(test)]
        constructor_diagnostic.stage("globals_ready");
        interp
    }
}
