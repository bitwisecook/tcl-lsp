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

//! Live VM driver — run the TMM-sim orchestrator Tcl on [`tcl-vm`] and drive a
//! real event round-trip.
//!
//! This is the runtime half of the iRule test session.
//! Where [`crate::session::SessionPlan`] assembles the *bootstrap script*, a
//! [`LiveSession`] actually stands the orchestrator up on a bytecode VM, loads
//! an iRule, fires events, and reads back the pool/node decisions, captured
//! logs, and assertion results — entirely in-process, no `tclsh` subprocess.
//!
//! The simulation itself is the ~500 KB of Tcl under `rust/tcl-irule-test/tcl/`
//! (orchestrator + TMM shim + command mocks); the driver sources those files in
//! the same order `runner.tcl` does, then exposes a thin Rust API over the
//! `::orch::` command surface.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_registry::f5::{BigIpExecutionContext, evidence::BigIpBuild, naming::BigIpNameEvent};
use tcl_runtime_api::authored_tmm::AuthoredObservedFrameStorageContext;
use tcl_runtime_api::{Completion, NativeExecutionError};
use tcl_syntax::list::{join_list, list_element};
use tcl_vm::{Code, NativeCommand, Value, Vm};

/// The framework files the orchestrator depends on, in source order — mirrors
/// the `source` block at the top of `runner.tcl`. `_mock_stubs.tcl` is sourced
/// conditionally (it may not be generated), matching the runner.
const FRAMEWORK_FILES: &[&str] = &[
    "compat84.tcl",
    "state_layers.tcl",
    "tmm_shim.tcl",
    "expr_ops.tcl",
    "profiler.tcl",
    "command_mocks.tcl",
    "itest_core.tcl",
    "orchestrator.tcl",
];

/// Purpose-specific original-source findings for an explicit appliance profile.
/// Dynamic runtime evaluation never consumes this loader-policy receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleLoaderRefusal {
    /// Original byte classification; unsupported is distinct from measured rejection.
    OriginalBytes {
        /// Explicit measured appliance/build selection.
        profile: tcl_registry::irules_policy::MeasuredIrulesLoaderProfile,
        /// Build-scoped original byte finding.
        finding: tcl_registry::irules_policy::MeasuredIrulesSourceRefusal,
    },
    /// Existing shared analyser rule-loader diagnostic on the original source.
    OriginalPolicy {
        /// Explicit measured appliance/build selection.
        profile: tcl_registry::irules_policy::MeasuredIrulesLoaderProfile,
        /// Shared diagnostic classification, not a guessed guest error code.
        code: tcl_compiler::analyser::types::DiagCode,
        /// Original source-policy explanation.
        message: String,
    },
}

/// A session failure, keeping guest text, host refusal and Unicode projection
/// separate. Use [`LiveSession::eval_completion`] for byte-valued guest results
/// and their original completion options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The orchestrator library directory or a required file is missing.
    MissingLib(String),
    /// A Tcl evaluation returned an error completion (the message is the result).
    Eval(String),
    /// Original load-phase refusal, distinct from guest runtime completion.
    LoaderPolicy(RuleLoaderRefusal),
    /// A host execution refusal, outside guest Tcl completion.
    Execution(NativeExecutionError),
    /// The textual session API cannot represent the original result bytes.
    Projection(tcl_syntax::raw_string::UnicodeAccessError),
}

fn session_projection_error(
    vm: &mut Vm,
    original: &tcl_vm::embed::OriginalOperandProjection,
    error: tcl_vm::TclError,
) -> SessionError {
    let error = match error {
        tcl_vm::TclError::Host(failure) => vm.retain_original_operand_failure(original, failure),
        tcl_vm::TclError::Guest(_) => {
            unreachable!("raw String/context projection supplies only operational refusals")
        }
    };
    match error {
        tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(error),
            ),
        )) => SessionError::Projection(error),
        tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(error)) => {
            SessionError::Execution(error)
        }
        _ => unreachable!("the actual first-cause owner returns an execution carrier"),
    }
}

fn session_capture_error(error: tcl_vm::TclError) -> SessionError {
    match error {
        tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(error)) => {
            SessionError::Execution(error)
        }
        _ => unreachable!("original observation capture supplies only an execution refusal"),
    }
}

fn session_settlement_error(error: tcl_vm::TclError) -> SessionError {
    match error {
        tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(error),
            ),
        )) => SessionError::Projection(error),
        tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(error)) => {
            SessionError::Execution(error)
        }
        _ => unreachable!("original observation settlement supplies only an execution refusal"),
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLib(p) => write!(f, "iRule-test library not found: {p}"),
            Self::Eval(m) => write!(f, "orchestrator error: {m}"),
            Self::LoaderPolicy(reason) => write!(f, "original rule load refused: {reason:?}"),
            Self::Execution(error) => write!(f, "execution refused: {error}"),
            Self::Projection(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SessionError {}

/// The `CompileService` the VM uses to compile the orchestrator Tcl and any
/// runtime `eval` / command substitution: the real Rust compiler pipeline,
/// built from the iRules profile. Ordinary event source uses the TMM's Tcl
/// 8.4.6 grammar: no TIP-157 `{*}` expansion, the 8.x first-close `${…}` rule,
/// and the iRules-only `}{` separator. Explicit framework host activations
/// supply their native engine's source profile to the same compiler service.
type Svc = BytecodeCompileService;

/// A shared, in-memory sink for the VM's `puts` output.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct MeasuredEventNames {
    build: Cell<Option<BigIpBuild>>,
    next_domain: Cell<u64>,
}

struct EventSourceHook {
    selection: Rc<MeasuredEventNames>,
    tmm: u64,
    domain: u64,
}

impl NativeCommand for EventSourceHook {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let [event, source] = args else {
            return vm
                .refuse_host_command("event source hook requires original event and body".into());
        };
        let reached = vm.with_original_operand_projection(|vm, original| {
            let event = vm.original_operand_unicode(original, event)?;
            vm.check_original_operand_projection(original)?;
            let Some(build) = self
                .selection
                .build
                .get()
                .filter(|_| event == "HTTP_REQUEST")
            else {
                return vm
                    .try_eval_original_source_value(source)
                    .map_err(tcl_vm::TclError::from_execution_failure);
            };
            let event = BigIpNameEvent::HttpRequest;
            let context = BigIpExecutionContext::TmmIRule;
            let Some(policy) =
                tcl_registry::f5::naming::observed_name_policy(build, context, event)
            else {
                return Ok(
                    vm.refuse_host_command("measured event naming issuer is unavailable".into())
                );
            };
            let storage = AuthoredObservedFrameStorageContext {
                policy,
                tmm: self.tmm,
                domain: self.domain,
            };
            vm.with_original_observed_event_inputs(original, build, context, event, storage, |vm| {
                vm.try_eval_original_source_value(source)
            })
        });
        reached.unwrap_or_else(|error| original_completion(vm, error))
    }
}

fn original_completion(vm: &mut Vm, error: tcl_vm::TclError) -> Completion<Value> {
    match error.into_completion() {
        Ok(guest) => guest,
        Err(failure) => vm.refuse_tcl_host_failure(failure),
    }
}

struct EnrollEventHook(Rc<MeasuredEventNames>);
impl NativeCommand for EnrollEventHook {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let [child, tmm] = args else {
            return vm.refuse_host_command(
                "event worker enrolment requires actual child and worker".into(),
            );
        };
        let reached = vm.with_original_operand_projection(|vm, original| {
            let child = vm.original_operand_unicode(original, child)?;
            let text = vm.original_operand_unicode(original, tmm)?;
            let Ok(tmm) = text.parse::<u64>() else {
                return Ok(vm.refuse_host_command("event worker identity is unavailable".into()));
            };
            vm.check_original_operand_projection(original)?;
            let domain = self.0.next_domain.get();
            let Some(next) = domain.checked_add(1) else {
                return Ok(vm.refuse_host_command("event domain identity is exhausted".into()));
            };
            self.0.next_domain.set(next);
            let command = Rc::new(EventSourceHook {
                selection: Rc::clone(&self.0),
                tmm,
                domain,
            });
            let reached = vm.try_register_child_native_command(
                &child,
                "::tmm::_observed_event_source",
                command,
            );
            vm.check_original_operand_projection(original)?;
            if !reached? {
                return Ok(vm.refuse_host_command("actual event worker is unavailable".into()));
            }
            Ok(Completion::new(Code::Ok, Value::empty(), Value::empty()))
        });
        reached.unwrap_or_else(|error| original_completion(vm, error))
    }
}

/// A live orchestrator session running on a bytecode VM.
pub struct LiveSession {
    vm: Vm,
    output: Rc<RefCell<Vec<u8>>>,
    measured_loader: Option<tcl_registry::irules_policy::MeasuredIrulesLoaderProfile>,
    measured_names: Rc<MeasuredEventNames>,
}

impl LiveSession {
    /// Stand a session up: build a VM, source the orchestrator framework from
    /// `lib_dir`, and run `::orch::init`. `lib_dir` is the directory holding
    /// `orchestrator.tcl` (i.e. `rust/tcl-irule-test/tcl/`).
    ///
    /// # Errors
    /// [`SessionError::MissingLib`] if `lib_dir` or a required file is absent,
    /// or [`SessionError::Eval`] / [`SessionError::Execution`] if the framework
    /// fails to load.
    pub fn new(lib_dir: &Path) -> Result<Self, SessionError> {
        if !lib_dir.join("orchestrator.tcl").is_file() {
            return Err(SessionError::MissingLib(lib_dir.display().to_string()));
        }
        let output = Rc::new(RefCell::new(Vec::new()));
        let host_profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Capture(Rc::clone(&output))),
            Rc::new(tcl_vm::host_native::NativeHost::new()),
            host_profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("explicit C9 core bootstrap");
        // Source grammar remains the measured F5 dialect. The surrounding
        // host needs modern control machinery for persistent flow frames;
        // tmm_shim restricts the commands visible during event execution.
        let profile = DialectProfile::irules();
        vm.set_dialect_profile(profile);
        assert!(vm.set_command_surface_profile(host_profile));
        vm.set_compiler(Box::new(Svc::for_profile(profile)));
        // This simulator explicitly supplies a C9 host engine. Its F5 source
        // grammar and logical event policies remain separate from that physical
        // engine; broad command visibility alone never selects it.
        assert!(vm.set_native_engine_profile(host_profile));
        assert!(vm.set_logical_eval_object_provider(
            tcl_native_model::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_source_word_provider(
            tcl_native_model::LogicalSourceWordProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_expression_parse_provider(
            tcl_native_model::LogicalExpressionParseProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_quote_provider(
            tcl_native_model::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
        ));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        ));
        assert!(vm.set_logical_compiled_variable_provider(
            tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_source_file_provider(Some(
            tcl_native_model::source_file::LogicalSourceFileProvider::Tcl84CoreSimulation,
        )));
        assert!(vm.set_logical_math_function_provider(Some(
            tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider::Tcl84Core,
        )));
        assert!(vm.set_logical_package_provider(Some(
            tcl_registry::native_package::AuthoredPackageProvider::Tcl84Core,
        )));
        assert!(vm.install_irules_rule_callable_simulation());
        assert!(vm.install_irules_counted_string_simulation());
        vm.install_irules_timer_simulation();
        vm.install_irules_static_simulation();
        for (alias, native) in [
            ("::tmm::_host_catch", "catch"),
            ("::tmm::_host_return", "return"),
        ] {
            assert!(vm.register_framework_builtin(alias, native, host_profile));
        }
        let measured_names = Rc::new(MeasuredEventNames {
            build: Cell::new(None),
            next_domain: Cell::new(1),
        });
        vm.register_native_command(
            "::tmm::_observed_event_source",
            Rc::new(EventSourceHook {
                selection: Rc::clone(&measured_names),
                tmm: 0,
                domain: 0,
            }),
        );
        vm.register_native_command(
            "::tmm::_observed_event_enroll",
            Rc::new(EnrollEventHook(Rc::clone(&measured_names))),
        );
        let mut session = Self {
            vm,
            output,
            measured_loader: None,
            measured_names,
        };
        session.bootstrap(lib_dir)?;
        session.eval_host_initialization("::tmm::_static_enroll")?;
        #[cfg(debug_assertions)]
        Self::observe_native_framework_phase("before-first-init");
        session.eval_host_initialization("::orch::init")?;
        #[cfg(debug_assertions)]
        Self::observe_native_framework_phase("after-first-init");
        // The pure functions run their shared cores, registered before
        // `register_all` looks for each command's mock.
        crate::pure_functions::register(&mut session.vm);
        #[cfg(debug_assertions)]
        Self::observe_native_framework_phase("before-final-init");
        session.eval_final_host_initialization("::orch::init")?;
        #[cfg(debug_assertions)]
        Self::observe_native_framework_phase("after-final-init");
        Ok(session)
    }

    #[cfg(debug_assertions)]
    fn observe_native_framework_phase(phase: &str) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ROWS: AtomicU64 = AtomicU64::new(0);
        if std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1")
            && ROWS
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    (n < 32).then_some(n + 1)
                })
                .is_ok()
        {
            eprintln!("TCL_NATIVE_FRAMEWORK_PHASE {phase}");
        }
    }

    /// Select an independently measured event naming issuer. Only the exact
    /// supported `build/TMM/HTTP_REQUEST` context enters counted frame storage.
    /// An unsupported selection clears the previous issuer and returns false.
    pub fn set_measured_event_name_build(&mut self, build: Option<BigIpBuild>) -> bool {
        let selected = build.filter(|build| {
            tcl_registry::f5::naming::observed_name_policy(
                *build,
                BigIpExecutionContext::TmmIRule,
                BigIpNameEvent::HttpRequest,
            )
            .is_some()
        });
        self.measured_names.build.set(selected);
        build.is_none() || selected.is_some()
    }

    /// Stand a session up against the framework Tcl embedded in the binary
    /// (no on-disk `rust/tcl-irule-test/tcl/` checkout needed). The bundled
    /// directory is materialised for the lifetime of the call and removed
    /// afterwards — its contents are sourced into the VM, so nothing is lost.
    ///
    /// # Errors
    /// [`SessionError::MissingLib`] if the bundle cannot be written, or a
    /// bootstrap error from the framework load.
    pub fn embedded() -> Result<Self, SessionError> {
        let lib = crate::embedded::EmbeddedLib::materialise()
            .map_err(|e| SessionError::MissingLib(e.to_string()))?;
        Self::new(lib.path())
    }

    /// Source the framework files (absolute paths, in dependency order); the
    /// optional `_mock_stubs.tcl` is sourced only when present, like the runner.
    fn bootstrap(&mut self, lib_dir: &Path) -> Result<(), SessionError> {
        for file in FRAMEWORK_FILES {
            let path = lib_dir.join(file);
            if !path.is_file() {
                return Err(SessionError::MissingLib(path.display().to_string()));
            }
            // `command_mocks.tcl` runs before `itest_core.tcl`; slot the
            // optional generated stubs in between, matching `runner.tcl`.
            if *file == "itest_core.tcl" {
                let stubs = lib_dir.join("_mock_stubs.tcl");
                if stubs.is_file() {
                    self.source_file(&stubs)?;
                }
            }
            self.source_file(&path)?;
        }
        Ok(())
    }

    /// Source a single file by absolute path through the VM's `source` (so the
    /// framework's `[file dirname [info script]]` lookups resolve correctly).
    fn source_file(&mut self, path: &Path) -> Result<(), SessionError> {
        let script = format!("source {}", list_element(&path.display().to_string()));
        self.eval_host_initialization(&script)
    }

    fn eval_host_initialization(&mut self, script: &str) -> Result<(), SessionError> {
        let completion = self
            .vm
            .try_eval_native_host_source(script)
            .map_err(SessionError::Execution)?;
        // The existing evaluation producer owns execution. This receipt begins
        // at its returned result projection and claims no earlier script stage.
        let original = self
            .vm
            .capture_original_operand_projection()
            .map_err(session_capture_error)?;
        let reached = (|| {
            let reached = if completion.code == Code::Error {
                match self
                    .vm
                    .original_operand_unicode(&original, &completion.result)
                {
                    Ok(text) => Err(SessionError::Eval(text)),
                    Err(error) => Err(session_projection_error(&mut self.vm, &original, error)),
                }
            } else {
                Ok(())
            };
            drop(completion);
            self.vm
                .check_original_operand_projection(&original)
                .map_err(|error| session_projection_error(&mut self.vm, &original, error))?;
            reached
        })();
        let settled = self
            .vm
            .finish_original_operand_projection(original)
            .map_err(session_settlement_error);
        settled?;
        reached
    }

    /// Evaluate a Tcl script without projecting its result or options as text.
    /// Guest errors and abrupt completions remain successful host results.
    ///
    /// # Errors
    /// [`SessionError::Execution`] retains the VM's actual host refusal.
    pub fn eval_completion(
        &mut self,
        script: &str,
    ) -> Result<Completion<tcl_vm::Value>, SessionError> {
        self.vm
            .try_eval_source(script)
            .map_err(SessionError::Execution)
    }

    /// Evaluate a Tcl `script` and return its checked Unicode result.
    /// Consumers needing original bytes or options use [`Self::eval_completion`].
    ///
    /// # Errors
    /// [`SessionError::Eval`] on an error completion or guest exit,
    /// [`SessionError::Execution`] on a host refusal, and
    /// [`SessionError::Projection`] on non-Unicode result bytes.
    pub fn eval(&mut self, script: &str) -> Result<String, SessionError> {
        let result = self.eval_completion(script);
        self.settle_public_evaluation(result)
    }

    // The final framework initialization previously used public eval. Preserve
    // its exit/result settlement while entering the actual selected host.
    fn eval_final_host_initialization(&mut self, script: &str) -> Result<String, SessionError> {
        let result = self
            .vm
            .try_eval_native_host_source(script)
            .map_err(SessionError::Execution);
        self.settle_public_evaluation(result)
    }

    fn settle_public_evaluation(
        &mut self,
        result: Result<Completion<Value>, SessionError>,
    ) -> Result<String, SessionError> {
        // Keep the existing reached exit settlement before result projection.
        if let Some(code) = self.vm.take_exit() {
            return Err(SessionError::Eval(format!("script called exit {code}")));
        }
        let completion = result?;
        let original = self
            .vm
            .capture_original_operand_projection()
            .map_err(session_capture_error)?;
        let reached = (|| {
            let text = self
                .vm
                .original_operand_unicode(&original, &completion.result)
                .map_err(|error| session_projection_error(&mut self.vm, &original, error));
            let code = completion.code;
            drop(completion);
            self.vm
                .check_original_operand_projection(&original)
                .map_err(|error| session_projection_error(&mut self.vm, &original, error))?;
            let text = text?;
            if code == Code::Error {
                Err(SessionError::Eval(text))
            } else {
                Ok(text)
            }
        })();
        let settled = self
            .vm
            .finish_original_operand_projection(original)
            .map_err(session_settlement_error);
        settled?;
        reached
    }

    /// Load an iRule into the orchestrator (`::orch::load_irule`) as one Tcl
    /// word. The shared list encoder preserves arbitrary source literally.
    ///
    /// # Errors
    /// Propagates an orchestrator/compile error.
    pub fn load_irule(&mut self, source: &str) -> Result<(), SessionError> {
        self.validate_original_rule_source(source)?;
        self.eval(&format!("::orch::load_irule {}", list_element(source)))
            .map(|_| ())
    }

    /// Load a rule whose optional identity was supplied by configuration or
    /// the caller, preserving per-rule procedure ownership.
    ///
    /// # Errors
    /// Propagates an orchestrator/compile error.
    pub fn load_rule(&mut self, rule: &crate::session::RuleSource) -> Result<(), SessionError> {
        self.validate_original_rule_source(&rule.source)?;
        self.eval(&rule.load_command()).map(|_| ())
    }

    /// Select build-specific original loader checks independently of runtime
    /// evaluation and of the simulator's physical host capabilities.
    pub fn set_measured_loader_profile(
        &mut self,
        profile: Option<tcl_registry::irules_policy::MeasuredIrulesLoaderProfile>,
    ) {
        self.measured_loader = profile;
    }

    fn validate_original_rule_source(&self, source: &str) -> Result<(), SessionError> {
        use tcl_compiler::analyser::types::DiagCode;
        let Some(profile) = self.measured_loader else {
            return Ok(());
        };
        if let Some(finding) = profile.original_source_refusal(source.as_bytes()) {
            return Err(SessionError::LoaderPolicy(
                RuleLoaderRefusal::OriginalBytes { profile, finding },
            ));
        }
        let result = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        if let Some(diagnostic) = result.diagnostics.iter().find(|diagnostic| {
            matches!(
                diagnostic.code,
                DiagCode::Irule2004 | DiagCode::Irule5005 | DiagCode::Irule5006
            )
        }) {
            return Err(SessionError::LoaderPolicy(
                RuleLoaderRefusal::OriginalPolicy {
                    profile,
                    code: diagnostic.code,
                    message: diagnostic.message.clone(),
                },
            ));
        }
        Ok(())
    }

    /// Recreate one explicitly named rule. Its procedure/handler owners retire,
    /// its new initializers run on each worker, and shared static cells remain.
    ///
    /// # Errors
    /// Requires an explicit identity and propagates loader/execution failures.
    pub fn replace_rule(&mut self, rule: &crate::session::RuleSource) -> Result<(), SessionError> {
        self.validate_original_rule_source(&rule.source)?;
        let identity = rule.identity.as_ref().ok_or_else(|| {
            SessionError::Eval("rule replacement requires explicit ownership".into())
        })?;
        self.eval(&format!(
            "::itest::unload_rule {}",
            list_element(identity.as_path())
        ))?;
        self.load_rule(rule)
    }

    /// Run one HTTP request through the configured flow (`::orch::run_http_request`
    /// with the given `args` fragment, e.g. `-host api.example.com -uri /`).
    ///
    /// # Errors
    /// Propagates an orchestrator error (e.g. no matching flow chain).
    pub fn run_http_request(&mut self, args: &str) -> Result<String, SessionError> {
        self.eval(&format!("::orch::run_http_request {args}"))
    }

    /// Fire a single event by name (`::orch::fire`), returning its
    /// `fired …` result list. Unlike [`run_http_request`](Self::run_http_request)
    /// this dispatches the event directly, so it drives non-HTTP handlers
    /// (`CLIENT_ACCEPTED`, `LB_SELECTED`, …) without a full request lifecycle.
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn fire_event(&mut self, event: &str) -> Result<String, SessionError> {
        self.eval(&format!("::orch::fire {event}"))
    }

    /// Advance the authored simulator clock and execute due callbacks in their
    /// original worker/connection activations. Complete guest options and host
    /// refusals remain separate in each report; this is not a hardware clock.
    pub fn advance_time(
        &mut self,
        milliseconds: u64,
    ) -> Vec<tcl_runtime_api::retained_activation::ScheduledCallbackReport<tcl_vm::Value>> {
        self.vm.advance_irules_time(milliseconds)
    }

    /// Fire a sequence of events in order (`::orch::fire_sequence`), returning
    /// the per-event result list. Gated events are skipped, matching the
    /// orchestrator's ordering rules.
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn fire_sequence(&mut self, events: &[&str]) -> Result<String, SessionError> {
        let joined = join_list(events);
        self.eval(&format!("::orch::fire_sequence {}", list_element(&joined)))
    }

    /// Register a data-group for `class match` lookups
    /// (`::orch::add_datagroup`). `records` is the Tcl list body of `{key val …}`
    /// entries; `dg_type` is `string`, `ip`, or `integer`.
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn add_datagroup(
        &mut self,
        name: &str,
        dg_type: &str,
        records: &str,
    ) -> Result<(), SessionError> {
        self.eval(&format!(
            "::orch::add_datagroup {} {} {}",
            list_element(name),
            list_element(dg_type),
            list_element(records),
        ))
        .map(|_| ())
    }

    /// The pool the iRule selected on the last request (empty if none).
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn pool_selected(&mut self) -> Result<String, SessionError> {
        self.eval("set ::state::lb::pool")
    }

    /// The recorded decision log as a Tcl list (one `{category action args}`
    /// per entry).
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn decisions(&mut self) -> Result<String, SessionError> {
        self.eval("::itest::get_decisions")
    }

    /// The captured iRule log messages.
    ///
    /// # Errors
    /// Propagates an orchestrator error.
    pub fn logs(&mut self) -> Result<String, SessionError> {
        self.eval("::state::log_capture::get")
    }

    /// Drain and return everything written to the VM's stdout (`puts`) so far.
    pub fn take_output(&mut self) -> String {
        let bytes = std::mem::take(&mut *self.output.borrow_mut());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Borrow the underlying VM (for advanced drivers needing direct access).
    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "live/original_projection_tests.rs"]
mod original_projection_tests;
