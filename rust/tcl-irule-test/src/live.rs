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

use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_runtime_api::{Completion, NativeExecutionError};
use tcl_syntax::list::{join_list, list_element};
use tcl_vm::{Code, Vm};

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

/// A session failure, keeping guest text, host refusal and Unicode projection
/// separate. Use [`LiveSession::eval_completion`] for byte-valued guest results
/// and their original completion options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The orchestrator library directory or a required file is missing.
    MissingLib(String),
    /// A Tcl evaluation returned an error completion (the message is the result).
    Eval(String),
    /// A host execution refusal, outside guest Tcl completion.
    Execution(NativeExecutionError),
    /// The textual session API cannot represent the original result bytes.
    Projection(tcl_syntax::raw_string::UnicodeAccessError),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLib(p) => write!(f, "iRule-test library not found: {p}"),
            Self::Eval(m) => write!(f, "orchestrator error: {m}"),
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

/// A live orchestrator session running on a bytecode VM.
pub struct LiveSession {
    vm: Vm,
    output: Rc<RefCell<Vec<u8>>>,
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
        let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&output))));
        // Source grammar remains the measured F5 dialect. The surrounding
        // host needs modern control machinery for persistent flow frames;
        // tmm_shim restricts the commands visible during event execution.
        let profile = DialectProfile::irules();
        vm.set_dialect_profile(profile);
        assert!(vm.set_command_surface_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
        ));
        vm.set_compiler(Box::new(Svc::for_profile(profile)));
        let host_profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        // This simulator explicitly supplies a C9 host engine. Its F5 source
        // grammar and logical event policies remain separate from that physical
        // engine; broad command visibility alone never selects it.
        assert!(vm.set_native_engine_profile(host_profile));
        assert!(vm.set_logical_eval_object_provider(
            tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_source_word_provider(
            tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_expression_parse_provider(
            tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_quote_provider(
            tcl_registry::invocation_words::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
        ));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
        ));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        ));
        assert!(vm.set_logical_compiled_variable_provider(
            tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation,
        ));
        vm.install_irules_timer_simulation();
        vm.install_irules_static_simulation();
        for (alias, native) in [
            ("::tmm::_host_catch", "catch"),
            ("::tmm::_host_return", "return"),
        ] {
            assert!(vm.register_framework_builtin(alias, native, host_profile));
        }
        let mut session = Self { vm, output };
        session.bootstrap(lib_dir)?;
        session.eval_host_initialization("::tmm::_static_enroll")?;
        session.eval_host_initialization("::orch::init")?;
        Ok(session)
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
        if completion.code == Code::Error {
            return Err(SessionError::Eval(
                completion
                    .result
                    .try_to_str()
                    .map_err(SessionError::Projection)?
                    .to_string(),
            ));
        }
        Ok(())
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
        // A guest `exit` records a code on the VM rather than killing the host;
        // surface it as a handleable error and clear it so the next eval is
        // clean (the session, not the process, decides what to do).
        if let Some(code) = self.vm.take_exit() {
            return Err(SessionError::Eval(format!("script called exit {code}")));
        }
        let completion = result?;
        let text = completion
            .result
            .try_to_str()
            .map_err(SessionError::Projection)?
            .to_string();
        if completion.code == Code::Error {
            Err(SessionError::Eval(text))
        } else {
            Ok(text)
        }
    }

    /// Load an iRule into the orchestrator (`::orch::load_irule`) as one Tcl
    /// word. The shared list encoder preserves arbitrary source literally.
    ///
    /// # Errors
    /// Propagates an orchestrator/compile error.
    pub fn load_irule(&mut self, source: &str) -> Result<(), SessionError> {
        self.eval(&format!("::orch::load_irule {}", list_element(source)))
            .map(|_| ())
    }

    /// Load a rule whose optional identity was supplied by configuration or
    /// the caller, preserving per-rule procedure ownership.
    ///
    /// # Errors
    /// Propagates an orchestrator/compile error.
    pub fn load_rule(&mut self, rule: &crate::session::RuleSource) -> Result<(), SessionError> {
        self.eval(&rule.load_command()).map(|_| ())
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn completion_api_preserves_guest_bytes_before_text_projection() {
        fn byte_error(_: &mut Vm, _: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
            Completion::new(
                Code::Error,
                tcl_vm::Value::from_string_bytes(&b"guest \xff\0tail"[..]),
                tcl_vm::Value::list(vec![
                    tcl_vm::Value::string("-errorcode"),
                    tcl_vm::Value::from_string_bytes(&b"RAW \xfe\0code"[..]),
                ]),
            )
        }
        let mut session = LiveSession {
            vm: Vm::new(),
            output: Rc::default(),
        };
        session.vm.set_compiler(Box::new(Svc::default()));
        session.vm.register("byte_error", byte_error);
        let completion = session.eval_completion("byte_error").unwrap();
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"guest \xff\0tail"
        );
        let options = completion.options.as_list().unwrap();
        let error_code = options
            .as_chunks::<2>()
            .0
            .iter()
            .find(|pair| pair[0].string_bytes().as_ref() == b"-errorcode")
            .expect("original error code");
        assert_eq!(error_code[1].string_bytes().as_ref(), b"RAW \xfe\0code");
        assert!(matches!(
            session.eval("byte_error"),
            Err(SessionError::Projection(_))
        ));
    }

    #[test]
    fn completion_api_keeps_missing_execution_provider_outside_guest_errors() {
        let mut session = LiveSession {
            vm: Vm::new(),
            output: Rc::default(),
        };
        assert!(matches!(
            session.eval_completion("set reached 1"),
            Err(SessionError::Execution(
                NativeExecutionError::CompileServiceRefusal(_)
            ))
        ));
        assert!(session.vm.get_var("reached").is_none());
    }

    /// The in-crate orchestrator Tcl directory (the source the binary also
    /// embeds via `embedded.rs`).
    fn lib_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tcl")
            .canonicalize()
            .expect("orchestrator tcl dir")
    }

    #[test]
    fn native_completion_capabilities_do_not_expand_the_user_grammar() {
        let mut session = LiveSession::new(&lib_dir()).unwrap();
        assert_eq!(
            session
                .eval("::tmm::_host_catch {set x 7} result options")
                .unwrap(),
            "0"
        );
        assert_eq!(
            session
                .eval("catch {catch {set x 8} result options} message; set message")
                .unwrap(),
            "wrong # args: should be \"catch command ?varName?\""
        );
        assert_eq!(
            session
                .eval("catch {return -options {-code 0 -level 0} ok} message; set message")
                .unwrap(),
            "bad option \"-options\": must be -code, -errorcode, or -errorinfo"
        );
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        session.load_irule("when HTTP_REQUEST {catch {catch {set x 1} result options} message; set static::message $message}").unwrap();
        for worker in 0..2 {
            session
                .eval(&format!("::orch::tmm_select {worker}"))
                .unwrap();
            session.fire_event("HTTP_REQUEST").unwrap();
            assert_eq!(
                session
                    .eval(&format!("::orch::tmm_get_static {worker} message"))
                    .unwrap(),
                "wrong # args: should be \"catch command ?varName?\""
            );
        }
    }

    /// Return a session to a clean slate before the next scenario in the same
    /// `#[test]` runs.
    ///
    /// `::orch::reset` is the very primitive `::orch::test` runs before every
    /// test body: it clears the whole `::state::*` tree (pools, data-groups,
    /// LB selection, HTTP request/response state, captured logs, `static::`
    /// variables), the loaded iRule (`::itest::clear_irule`) and the recorded
    /// decisions (`::itest::reset_decisions`). `::orch::configure` is *not*
    /// covered by it — the config array persists — so the profile stack is
    /// re-applied here and each scenario starts from the same footing.
    fn scenario(s: &mut LiveSession) {
        s.eval("::orch::reset").expect("scenario reset");
        s.eval("::orch::configure -profiles {TCP HTTP}")
            .expect("scenario profiles");
    }

    /// Run the load-time rewrite (`rewrite_irule_source`) over one source
    /// string and return what it produced.
    fn rewrite(s: &mut LiveSession, source: &str) -> String {
        s.eval(&format!(
            "::tmm::expr_ops::rewrite_irule_source {}",
            list_element(source)
        ))
        .expect("rewrite_irule_source")
    }

    /// The refusal a word-form unary earns in front of a custom comparison,
    /// and the neighbouring shapes that must keep working. Split out of the
    /// word-boolean test above only to keep that function under clippy's
    /// line cap; it runs in the same session.
    fn unmeasured_unary_grouping_is_refused(s: &mut LiveSession) {
        // A word-form unary sitting
        // directly in front of the left operand of a custom comparison has no
        // measured grouping. `UNARY_BP` (24) in
        // `rust/tcl-syntax/src/expr/parser.rs` is the parser's shared default
        // for every prefix operator, not an F5 measurement: `not` is
        // deliberately absent from `F5_TCL_PRECEDENCE_ROWS`
        // (`lookup("not") == None`), and
        // `docs/design/f5/bigip-irule-parser-measurements.md` pins only
        // `if {not 0}` (§4a `expr_and_or_not`), which exercises no binding
        // power against `starts_with` at all. So the rewriter must refuse
        // rather than pick `not (A starts_with B)` or `(not A) starts_with B`.
        s.eval("set ::u /v1/users").unwrap();
        match s.eval("expr {not $u starts_with \"/skip\"}") {
            Err(SessionError::Eval(m)) => {
                assert!(
                    m.contains("unmeasured"),
                    "the refusal must say the grouping is unmeasured: {m}"
                );
                assert!(
                    m.contains("not $u starts_with"),
                    "the refusal must name the expression: {m}"
                );
            }
            other => panic!("an unmeasured `not` grouping must be refused, got {other:?}"),
        }
        // …and at load time too, where the source rewrite runs.
        match s.load_irule(
            "when HTTP_REQUEST {\n  if { not [HTTP::uri] starts_with \"/skip\" } {\n    pool api_pool\n  }\n}",
        ) {
            Err(SessionError::Eval(m)) => assert!(
                m.contains("unmeasured"),
                "the load-time refusal must say the grouping is unmeasured: {m}"
            ),
            other => panic!("an unmeasured `not` grouping must be refused at load, got {other:?}"),
        }

        // positive controls for that refusal — the neighbouring shapes must
        // keep working, so the refusal is narrow and not a blanket ban.
        assert_eq!(
            s.eval("expr {not 0}").unwrap(),
            "1",
            "a bare `not` must still evaluate"
        );
        s.eval("set ::nx 0").unwrap();
        s.eval("set ::ny 1").unwrap();
        assert_eq!(
            s.eval("expr {not $nx and $ny}").unwrap(),
            "1",
            "`not X and Y` is measured (`and` is 6/7, below any unary) and must still evaluate"
        );
        assert_eq!(
            s.eval("expr {not ($u starts_with \"/skip\")}").unwrap(),
            "1",
            "parenthesising the operand names the grouping and must be accepted"
        );
        assert_eq!(
            s.eval("expr {$u starts_with \"/v1\"}").unwrap(),
            "1",
            "the custom comparison itself must still rewrite: the machinery fires"
        );
    }

    /// Request-lifecycle scenarios: pool selection, the reject decision,
    /// direct non-HTTP event dispatch, `HTTP::respond` commitment, and the
    /// fluent `was_called_with` decision matcher.
    ///
    /// These share one session on purpose. Standing the orchestrator up costs
    /// ~7 s (a fresh `Vm`, a fresh `CommandRegistry`, and sourcing the ~320 KB
    /// framework bundle) and nextest runs one process per test, so a `LazyLock`
    /// fixture cannot amortise it — driving several scenarios through a single
    /// session is what removes the repeated bootstrap. `scenario()` restores
    /// isolation between them; each assertion names the scenario it belongs to.
    #[test]
    fn live_session_drives_the_request_lifecycle() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // routes_request_to_pool: a guarded `pool` selects the named pool.
        scenario(&mut s);
        s.eval("::orch::add_pool api_pool {10.0.1.1:8080}").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [HTTP::host] eq \"api.example.com\" } {\n    pool api_pool\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "api_pool",
            "routes_request_to_pool: the matching host must select api_pool"
        );

        // records_reject_decision: `reject` lands in the decision log.
        scenario(&mut s);
        s.load_irule("when HTTP_REQUEST {\n  reject\n}").unwrap();
        s.run_http_request("-host evil.example.com -uri /").unwrap();
        let decisions = s.decisions().unwrap();
        assert!(
            decisions.contains("connection reject"),
            "records_reject_decision: decisions: {decisions}"
        );

        // dispatches_non_http_event: a `when CLIENT_ACCEPTED` handler fires
        // directly through `fire_event`, without a full HTTP request lifecycle.
        scenario(&mut s);
        s.load_irule("when CLIENT_ACCEPTED {\n  log local0. \"accepted-here\"\n}")
            .unwrap();
        let result = s.fire_event("CLIENT_ACCEPTED").unwrap();
        assert!(
            result.contains("fired 1"),
            "dispatches_non_http_event: fire result: {result}"
        );
        assert!(
            s.logs().unwrap().contains("accepted-here"),
            "dispatches_non_http_event: the CLIENT_ACCEPTED handler's log output should be captured"
        );

        // http_respond_commits_response: `HTTP::respond` sets
        // `::state::http::response_committed`; the flag must be observable
        // (the simulator reads this exact variable to populate
        // `SimOutcome::response_committed`).
        scenario(&mut s);
        s.load_irule("when HTTP_REQUEST {\n  HTTP::respond 200 content \"ok\"\n}")
            .unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        assert_eq!(
            s.eval("set ::state::http::response_committed").unwrap(),
            "1",
            "http_respond_commits_response: HTTP::respond must set the committed flag"
        );

        // fluent_decision_was_called_with_scans_all_calls: `was_called_with`
        // must pass if ANY matching decision carries the expected value, not
        // only the first. An iRule that calls `pool a` then `pool b` must
        // satisfy `was_called_with "b"`.
        scenario(&mut s);
        s.eval("::orch::add_pool a {10.0.0.1:80}").unwrap();
        s.eval("::orch::add_pool b {10.0.0.2:80}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  pool a\n  pool b\n}")
            .unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        // `assert_that` returns 1 on pass, 0 on fail. The later `pool b` call
        // must satisfy `was_called_with "b"` even though `pool a` was logged
        // first (the pre-fix code stopped at the first match and failed).
        assert_eq!(
            s.eval("::orch::assert_that decision lb pool_select was_called_with \"b\"")
                .unwrap(),
            "1",
            "fluent_decision_was_called_with_scans_all_calls: must scan all matching decisions"
        );
        // The first call's value still passes too.
        assert_eq!(
            s.eval("::orch::assert_that decision lb pool_select was_called_with \"a\"")
                .unwrap(),
            "1",
            "fluent_decision_was_called_with_scans_all_calls: the first call's value still passes"
        );
        // FP-guard: a value that was never used fails.
        assert_eq!(
            s.eval("::orch::assert_that decision lb pool_select was_called_with \"c\"")
                .unwrap(),
            "0",
            "fluent_decision_was_called_with_scans_all_calls: an unused value must fail"
        );
    }

    /// Executable parity corpus for the self-hosted Tcl loader boundary.
    /// Priority/timing are loader-only modifiers; event discovery must match
    /// the canonical Rust walker for complete ordinary handlers.
    #[test]
    fn itest_loader_event_registration_matches_rust_when_blocks() {
        let source = "# when IGNORED { nope }\nwhen HTTP_REQUEST priority 20 { # body comment\n set ::parity_first one }\nwhen CLIENT_ACCEPTED timing on { set ::parity_nested {nested { braces }} }\nwhen HTTP_REQUEST { set ::parity_second two }\n";
        let expected: std::collections::BTreeSet<_> = tcl_irules::when_blocks(source)
            .into_iter()
            .map(|block| block.event)
            .collect();
        let mut session = LiveSession::new(&lib_dir()).expect("session");
        scenario(&mut session);
        session.load_irule(source).expect("load parity corpus");
        let actual: std::collections::BTreeSet<_> = session
            .eval("::itest::registered_events")
            .expect("registered events")
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        assert_eq!(actual, expected);
        // Registration records preserve multiplicity and priority, rather
        // than merely proving that each event name appears once.
        let records = session
            .eval("array get ::itest::event_handlers")
            .expect("records");
        assert_eq!(
            records.matches("::_irh_HTTP_REQUEST_").count(),
            2,
            "{records}"
        );
        assert_eq!(
            records.matches("::_irh_CLIENT_ACCEPTED_").count(),
            1,
            "{records}"
        );
        assert!(records.contains("20"), "priority record missing: {records}");
        assert!(
            records.contains("500"),
            "default-priority record missing: {records}"
        );
        session
            .eval("::itest::fire_event HTTP_REQUEST")
            .expect("fire HTTP");
        session
            .eval("::itest::fire_event CLIENT_ACCEPTED")
            .expect("fire client");
        assert_eq!(session.eval("set ::parity_first").unwrap(), "one");
        assert_eq!(session.eval("set ::parity_second").unwrap(), "two");
        assert_eq!(
            session.eval("set ::parity_nested").unwrap(),
            "nested { braces }"
        );

        // Tcl's braced-word scanner is quote-blind: the `}` inside quotes
        // closes the handler body, leaving a malformed trailing word. Both
        // boundary owners therefore abstain rather than inventing a complete
        // handler from invalid source.
        let quoted_close = "when RULE_INIT { log local0. \"quoted }\" }";
        assert!(tcl_irules::when_blocks(quoted_close).is_empty());
        assert!(
            session.load_irule(quoted_close).is_err(),
            "pinned loader carve-out"
        );
    }

    /// Every `class match` operator scenario on one session: the hit, the
    /// miss, `starts_with` / `equals` / `ends_with`, and the leading `--`.
    /// `scenario()` clears the data-group table between them, so each
    /// scenario registers exactly the records it names.
    #[test]
    fn live_session_class_match_honours_every_operator() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // class_match_against_datagroup: `class match` resolves against a
        // registered data-group — the request host is a member, so the guarded
        // `pool` selection runs.
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("hosts", "string", "api.example.com 1")
            .unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match [HTTP::host] equals hosts] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "matched",
            "class_match_against_datagroup: a member host must select the pool"
        );

        // class_match_miss_leaves_pool_unset: a non-member host must not
        // match, so the guarded `pool` never runs.
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("hosts", "string", "api.example.com 1")
            .unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match [HTTP::host] equals hosts] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host other.example.com -uri /")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "",
            "class_match_miss_leaves_pool_unset: a non-member host must not select a pool"
        );

        // class_match_starts_with_honours_operator: `starts_with` must test
        // whether the subject BEGINS WITH a record, not exact equality.
        // Record `/api`, URI `/api/v1/x` matches under `starts_with` (but
        // would NOT under `equals`).
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("prefixes", "string", "/api 1").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match [HTTP::uri] starts_with prefixes] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host x.example.com -uri /api/v1/x")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "matched",
            "class_match_starts_with_honours_operator: /api must prefix-match /api/v1/x"
        );

        // class_match_equals_is_not_prefix_match: FP-guard — the same `/api`
        // record under `equals` must NOT match the longer `/api/v1/x`, proving
        // the operator actually changes the comparison rather than always
        // behaving like `contains`.
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("prefixes", "string", "/api 1").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match [HTTP::uri] equals prefixes] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host x.example.com -uri /api/v1/x")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "",
            "class_match_equals_is_not_prefix_match: equals must not behave like starts_with"
        );

        // class_match_ends_with_honours_operator: `ends_with` tests the tail
        // of the subject.
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("suffixes", "string", ".png 1").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match [HTTP::uri] ends_with suffixes] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host x.example.com -uri /img/logo.png")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "matched",
            "class_match_ends_with_honours_operator: .png must suffix-match /img/logo.png"
        );

        // class_match_accepts_leading_dashdash: a leading `--` (or option
        // flags) before the value must be parsed off, not mistaken for the
        // value/operator/datagroup. Before the fix, `dg_name` resolved to
        // `equals` and raised `class "equals" not found`, aborting the handler
        // so `pool` never ran.
        scenario(&mut s);
        s.eval("::orch::add_pool matched {10.0.9.9:80}").unwrap();
        s.add_datagroup("hosts", "string", "api.example.com 1")
            .unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [class match -- [HTTP::host] equals hosts] } {\n    pool matched\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "matched",
            "class_match_accepts_leading_dashdash: the -- flag must be parsed off, not read as the value"
        );
    }

    /// The scenarios that need the framework sourced from the *embedded*
    /// bundle rather than the on-disk `tcl/` checkout, plus the guest-`exit`
    /// containment guarantee (which needs no particular framework state, so it
    /// rides along here rather than paying for a fourth bootstrap).
    #[test]
    fn embedded_session_scenarios() {
        let mut s = LiveSession::embedded().expect("embedded session");

        // embedded_session_routes_request: the bundled framework stands up and
        // routes a request just like the on-disk one.
        scenario(&mut s);
        s.eval("::orch::add_pool web {10.0.2.1:80}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  pool web\n}").unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "web",
            "embedded_session_routes_request: the embedded framework must route to web"
        );

        // embedded_session_includes_generated_stubs: `_mock_stubs.tcl`
        // provides mocks for registry-only iRule commands (no hand-written
        // mock). Without it bundled, a stub-only command like `ACCESS::session`
        // errors "invalid command name" inside the handler, which
        // `fire_event`'s `catch` stops — so the following `pool` never runs.
        // Reaching the pool selection proves the stub dispatched.
        scenario(&mut s);
        s.eval("::orch::add_pool web {10.0.2.1:80}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  ACCESS::session\n  pool web\n}")
            .unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "web",
            "embedded_session_includes_generated_stubs: stub-only ACCESS::session must dispatch, not abort the handler"
        );
        // The generic stub must log the call under the command's real
        // (category, action) — the data table replaced ~1500 per-command stub
        // procs, so guard that the decision log is unchanged for a stub-only
        // command (`ACCESS::session` -> {access session}).
        let decisions = s.decisions().unwrap();
        assert!(
            decisions.contains("access session"),
            "embedded_session_includes_generated_stubs: stub dispatch must record the decision under its real category/action; got: {decisions}"
        );

        // guest_exit_does_not_kill_the_host: the whole point of routing `exit`
        // through a VM completion — a guest script calling `exit` must NOT
        // terminate this test process. If it still called
        // `std::process::exit`, the test binary would die here. Runs last of
        // the three so a mishandled exit cannot mask an earlier scenario.
        scenario(&mut s);
        match s.eval("exit 7") {
            Err(SessionError::Eval(_)) => {}
            other => panic!(
                "guest_exit_does_not_kill_the_host: exit should surface as a handleable error, got {other:?}"
            ),
        }
        // The session is still usable afterwards.
        assert_eq!(
            s.eval("expr {1 + 1}").unwrap(),
            "2",
            "guest_exit_does_not_kill_the_host: the session must stay usable after a guest exit"
        );
    }

    /// The two `-tmm_select` scenarios, in the only order that keeps them
    /// independent: `configure_tests` writes `_test_tmm_select_mode`, which is
    /// exactly the value `reset` restores, so the "unconfigured default"
    /// scenario has to observe the session *before* anything configures it.
    /// It therefore runs first, and no `scenario()` reset precedes it — the
    /// bare `::orch::reset` it performs is the thing under test.
    #[test]
    fn tmm_select_mode_survives_the_per_test_reset() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // tmm_select_defaults_to_manual_after_reset: with no configuration,
        // reset must still yield the "manual" default — the fix restores the
        // configured default, which defaults to manual.
        s.eval("::orch::reset").unwrap();
        assert_eq!(
            s.eval("set ::orch::_tmm_select_mode").unwrap(),
            "manual",
            "tmm_select_defaults_to_manual_after_reset: an unconfigured reset must yield manual"
        );

        // configured_tmm_select_auto_survives_reset: `::orch::test` runs
        // `reset` before every body, and that `reset` must not force
        // `_tmm_select_mode` back to "manual" and so clobber a configured
        // `-tmm_select auto`. Capture the live mode from inside a test body
        // (i.e. after that reset) and prove it's still "auto" so
        // `run_http_request` takes the fakeCMP auto path.
        s.eval("::orch::configure_tests -tmm_count 4 -tmm_select auto -profiles {TCP HTTP}")
            .unwrap();
        s.eval(
            "::orch::test \"tmm-auto\" \"auto mode survives reset\" -body { set ::__captured_mode $::orch::_tmm_select_mode }",
        )
        .unwrap();
        assert_eq!(
            s.eval("set ::__captured_mode").unwrap(),
            "auto",
            "configured_tmm_select_auto_survives_reset: configured -tmm_select auto must survive the per-test reset"
        );
    }

    /// `::scf::_parse_snatpool` used to walk a `ltm snatpool` block, extract
    /// its `members`, and drop them on the floor — the only `_parse_*` proc
    /// that stored nothing. A virtual server's `snatpool` /
    /// `source-address-translation pool` property names one of those objects,
    /// so the reference had no referent to resolve against. It now stores what
    /// it parses in the same shape `_parse_pool` uses.
    #[test]
    fn scf_loader_stores_parsed_snatpool_members() {
        let dir = lib_dir();
        let mut s = LiveSession::new(&dir).expect("session");
        // `scf_loader.tcl` is not part of `FRAMEWORK_FILES`; `SessionPlan`
        // sources it alongside the orchestrator, before `::orch::init` puts
        // the TMM sandbox in the way of `source`/`file`. `::orch::init` has
        // already run here, so reach it through the framework-internal
        // handle the shim keeps, as the compat84 probes do.
        s.eval(&format!(
            "::tmm::_orig_source {{{}}}",
            dir.join("scf_loader.tcl").display()
        ))
        .expect("source scf_loader");

        // Exercise the whole-file scanner and its nested loops before the
        // registry-selected object parser. Loop labels must follow lexical
        // nesting even when the surrounding loop runs in a runtime body.
        assert_eq!(
            s.eval("::scf::_parse_header {ltm snatpool /Common/sp_out}")
                .unwrap(),
            "ltm snatpool /Common/sp_out",
            "the header parser classifies the block as a snatpool object"
        );
        s.eval(
            "::scf::load_string {ltm snatpool /Common/sp_out {\nmembers { /Common/10.0.5.1 /Common/10.0.5.2 }\n}}",
        )
        .expect("load snatpool configuration");

        assert_eq!(
            s.eval("::scf::list_snatpools").unwrap(),
            "/Common/sp_out",
            "the parsed snatpool object must be stored, not discarded"
        );
        assert_eq!(
            s.eval("::scf::snatpool_members /Common/sp_out").unwrap(),
            "/Common/10.0.5.1 /Common/10.0.5.2",
            "the members the parser walks must be readable back"
        );
        // Resolves by short name through `_resolve_name`, exactly like a pool.
        assert_eq!(
            s.eval("::scf::snatpool_members sp_out").unwrap(),
            "/Common/10.0.5.1 /Common/10.0.5.2",
            "short-name resolution must work for snatpools as it does for pools"
        );
        // The `snatpool` name a virtual server records (`_parse_virtual`
        // already stores it under the `snatpool` key) now resolves to a
        // stored object through the same accessor.
        assert_eq!(
            s.eval("::scf::snatpool_members [lindex {destination /Common/10.0.0.1:80 snatpool /Common/sp_out} end]")
                .unwrap(),
            "/Common/10.0.5.1 /Common/10.0.5.2",
            "the name a VS record carries must resolve to the stored snatpool"
        );
        // An unknown name still answers empty rather than erroring.
        assert_eq!(s.eval("::scf::snatpool_members nope").unwrap(), "");
        // `reset` clears the new array with the rest.
        s.eval("::scf::reset").unwrap();
        assert_eq!(
            s.eval("::scf::list_snatpools").unwrap(),
            "",
            "reset must clear the snatpool table"
        );
    }

    /// TMM's word-form boolean operators (`and` / `or` / `not`). Plain Tcl has
    /// no such operators — every oracle from 8.4 to 9.1 rejects `expr {1 and
    /// 1}` ("syntax error … extra tokens" on 8.4, "invalid bareword" on 8.5+)
    /// — so an iRule using them reached a bare `expr` and died. They are
    /// rewritten to the plain-Tcl operators the repository's own F5 dialect
    /// model says they are equivalent to: `or` carries `||`'s binding power
    /// and `and` carries `&&`'s (`F5_TCL_PRECEDENCE_ROWS` in
    /// `rust/tcl-dialect/src/model/expr_grammar.rs`), both short-circuit
    /// (`rust/tcl-syntax/src/expr/eval.rs` runs `WordAnd`/`WordOr` through the
    /// `And`/`Or` arms), and `not` is prefix unary at the shared `UNARY_BP`
    /// like `!` (`unaryop_from_text` in `rust/tcl-syntax/src/expr/parser.rs`).
    ///
    /// Each assertion below is written so a naive rewrite — turning the words
    /// into helper-proc calls — would fail it: proc arguments are evaluated
    /// eagerly, which destroys the short-circuit, and a proc call has no
    /// binding power at all.
    #[test]
    fn tmm_word_boolean_operators_are_rewritten_to_their_tcl_equivalents() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // word_booleans_evaluate: the bare forms answer at all. Under a plain
        // `expr` each of these is a syntax error.
        assert_eq!(s.eval("expr {1 and 1}").unwrap(), "1");
        assert_eq!(s.eval("expr {1 and 0}").unwrap(), "0");
        assert_eq!(s.eval("expr {0 or 1}").unwrap(), "1");
        assert_eq!(s.eval("expr {0 or 0}").unwrap(), "0");
        assert_eq!(s.eval("expr {not 0}").unwrap(), "1");
        assert_eq!(s.eval("expr {not 1}").unwrap(), "0");

        // and_binds_tighter_than_or: `1 or 0 and 0` is `1 || (0 && 0)` = 1.
        // If `and` were the looser of the two it would group as
        // `(1 || 0) && 0` = 0, so this cell discriminates the two orderings.
        assert_eq!(
            s.eval("expr {1 or 0 and 0}").unwrap(),
            "1",
            "and must bind tighter than or, as && does against ||"
        );
        assert_eq!(
            s.eval("expr {0 and 0 or 1}").unwrap(),
            "1",
            "and must bind tighter than or, as && does against ||"
        );

        // not_is_prefix_unary: `not 0 and 0` is `(!0) && 0` = 0. If `not`
        // bound looser than `and` it would be `!(0 && 0)` = 1.
        assert_eq!(
            s.eval("expr {not 0 and 0}").unwrap(),
            "0",
            "not must be prefix unary, binding tighter than and"
        );

        // word_booleans_short_circuit: the right operand must not be
        // evaluated when the left decides the answer. A helper-proc rewrite
        // (`[_or $a $b]`) evaluates both arguments before the proc runs and
        // fails both of these.
        s.eval("set ::sc_probe 0").unwrap();
        assert_eq!(s.eval("expr {1 or [incr ::sc_probe]}").unwrap(), "1");
        assert_eq!(
            s.eval("set ::sc_probe").unwrap(),
            "0",
            "or must short-circuit: the right operand ran"
        );
        s.eval("set ::sc_probe 0").unwrap();
        assert_eq!(s.eval("expr {0 and [incr ::sc_probe]}").unwrap(), "0");
        assert_eq!(
            s.eval("set ::sc_probe").unwrap(),
            "0",
            "and must short-circuit: the right operand ran"
        );
        // …and the operand IS evaluated when the left does not decide it.
        s.eval("set ::sc_probe 0").unwrap();
        assert_eq!(s.eval("expr {1 and [incr ::sc_probe]}").unwrap(), "1");
        assert_eq!(
            s.eval("set ::sc_probe").unwrap(),
            "1",
            "and must still evaluate the right operand when the left is true"
        );

        // word_booleans_compose_with_the_string_operators: the load-time
        // source rewrite has to handle both families in one condition. The
        // `not` operand is parenthesised because an unparenthesised one
        // would be the refused unmeasured grouping below.
        scenario(&mut s);
        s.eval("::orch::add_pool api_pool {10.0.1.1:8080}").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [HTTP::host] eq \"api.example.com\" and not ([HTTP::uri] starts_with \"/skip\") } {\n    pool api_pool\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /v1/users")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "api_pool",
            "an `and`/`not` guard in an iRule must evaluate, not raise"
        );

        // FP-guard: the same iRule must NOT select when the `not` arm is false.
        scenario(&mut s);
        s.eval("::orch::add_pool api_pool {10.0.1.1:8080}").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [HTTP::host] eq \"api.example.com\" and not ([HTTP::uri] starts_with \"/skip\") } {\n    pool api_pool\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /skip/me")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "",
            "the `not` arm must actually gate the selection"
        );

        // word_boundaries_are_respected: the operator spellings must only be
        // recognised as whole tokens. A bare substring rewrite would maul an
        // identifier containing `and`/`or`/`not` and a quoted literal.
        s.eval("set ::android 7").unwrap();
        assert_eq!(
            s.eval("expr {$android + 1}").unwrap(),
            "8",
            "a variable whose name merely contains `and` must be left alone"
        );
        assert_eq!(
            s.eval("expr {\"a and b\" eq \"a and b\"}").unwrap(),
            "1",
            "`and` inside a quoted literal is not an operator"
        );

        unmeasured_unary_grouping_is_refused(&mut s);
    }

    /// The load-time source rewrite (`rewrite_irule_source`) must only touch
    /// expression arguments in genuine *command* positions.
    ///
    /// It used to scan the raw source with `\m(if|elseif|while|expr)\s*\{`,
    /// which cannot tell a command word from a quoted string, a braced data
    /// literal or a comment — so it rewrote Tcl *data*. That was always wrong
    /// but rarely reached: adding the word-form booleans put the common
    /// English words `and` / `or` / `not` into the quick-check regexp, so
    /// ordinary iRule text now enters the rewriter. The same regexp also
    /// missed `for`, whose test expression is its *second* argument.
    ///
    /// Each negative assertion below is paired with a positive control, so a
    /// rewriter that had simply stopped working could not pass.
    #[test]
    fn irule_source_rewrite_only_touches_command_positions() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // positive control: a real command-position condition IS rewritten.
        let cmd_position = "when HTTP_REQUEST {\n    if {1 and 0} { pool a }\n}";
        assert!(
            rewrite(&mut s, cmd_position).contains("1 && 0"),
            "the rewriter must still rewrite a genuine `if` condition"
        );

        // quoted_data_is_not_a_command: `if` inside a quoted word is data.
        let quoted = "when HTTP_REQUEST {\n    set message \"if {1 and 0}\"\n}";
        assert_eq!(
            rewrite(&mut s, quoted),
            quoted,
            "a quoted string is data, not a command position"
        );

        // braced_data_is_not_a_command: nor is a braced literal argument to a
        // command that takes no script there.
        let braced = "when HTTP_REQUEST {\n    set s {expr {1 and 0}}\n}";
        assert_eq!(
            rewrite(&mut s, braced),
            braced,
            "a braced data word is not a script body"
        );

        // comments_are_not_commands.
        let commented = "when HTTP_REQUEST {\n    # if {1 and 0} explains the rule\n    set x 1\n}";
        assert_eq!(
            rewrite(&mut s, commented),
            commented,
            "a comment is not a command position"
        );

        // for_test_is_the_second_argument: `for` reaches Tcl's builtin, whose
        // test expression it evaluates internally, so the word operators in it
        // must be rewritten at source level (an `::for` proc override is not
        // an option — it breaks break/continue/return propagation, which is
        // why `install` only wraps `::expr`).
        let for_loop =
            "when HTTP_REQUEST {\n    for {set i 0} {$i < 3 and $ready} {incr i} { incr n }\n}";
        let rewritten_for = rewrite(&mut s, for_loop);
        assert!(
            rewritten_for.contains("$i < 3 && $ready"),
            "`for`'s test expression must be rewritten: {rewritten_for}"
        );
        assert!(
            rewritten_for.contains("{set i 0}"),
            "`for`'s initialiser must be left alone: {rewritten_for}"
        );

        // …and end to end, over the real load path. Note what this can and
        // cannot prove: the in-process VM compiles an iRule under the F5
        // dialect and evaluates `if` / `while` / `for` conditions with its own
        // expression engine, which already knows the word operators — probing
        // the session shows `if {1 and 0}` and `if {"abc" starts_with "a"}`
        // answering correctly even with `rewrite_irule_source` stubbed out to
        // return its argument. So a passing pool selection here is a smoke
        // check that the rewritten source still compiles and runs, *not*
        // evidence that the rewrite happened; the string assertions above are
        // what bite. The logged datum is the exception — the old regexp
        // rewrite really did maul it, so that assertion is a live guard.
        scenario(&mut s);
        s.eval("::orch::add_pool api_pool {10.0.1.1:8080}").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  set ready 1\n  set n 0\n  for {set i 0} {$i < 3 and $ready} {incr i} {\n    incr n\n  }\n  set message \"if {1 and 0}\"\n  log local0. $message\n  if { $n == 3 } { pool api_pool }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /v1/users")
            .unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "api_pool",
            "the rewritten source must still compile and run its three iterations"
        );
        let logs = s.logs().unwrap();
        assert!(
            logs.contains("if {1 and 0}"),
            "the logged datum must not have been rewritten: {logs}"
        );
    }

    #[test]
    fn missing_lib_dir_errors() {
        match LiveSession::new(Path::new("/no/such/dir")) {
            Err(SessionError::MissingLib(_)) => {}
            Err(other) => panic!("wrong error: {other}"),
            Ok(_) => panic!("expected a MissingLib error"),
        }
    }

    #[test]
    fn event_frames_preserve_cells_aliases_and_traces() {
        let mut session = LiveSession::new(&lib_dir()).expect("session");
        scenario(&mut session);
        session.eval("set ::watch_reads 0; proc ::watch_read {args} {incr ::watch_reads}; proc ::bump {name} {upvar 1 $name cell; incr cell}").unwrap();
        session
            .load_irule(
                r"
            when RULE_INIT {set root_only 7}
            when HTTP_REQUEST {
                set value 4
                bump value
                upvar 0 value alias
                set values(k) a
                trace variable value r ::watch_read
                set ::seen_root $::root_only
                set ::seen_level [info level]
                uplevel 1 {set root_from_flow yes}
            }
            when HTTP_RESPONSE {
                set ::seen_alias $alias
                set ::seen_value $value
                append values(k) b
                set ::seen_array $values(k)
            }
        ",
            )
            .unwrap();
        session.fire_event("RULE_INIT").unwrap();
        session.fire_event("HTTP_REQUEST").unwrap();
        assert_eq!(session.eval("set ::watch_reads").unwrap(), "0");
        let response = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(!response.contains("code 1"), "{response}");
        assert_eq!(session.eval("list $::seen_root $::seen_alias $::seen_value $::seen_array $::watch_reads $::seen_level $::root_from_flow").unwrap(), "7 5 5 ab 2 1 yes");
        session.eval("::state::reset_connection_state").unwrap();
        let missing = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(
            missing.contains("code 1"),
            "a new connection cannot inherit the old cells: {missing}"
        );
    }

    #[test]
    fn worker_interpreters_preserve_cells_traces_and_command_tables() {
        let mut session = LiveSession::new(&lib_dir()).expect("session");
        scenario(&mut session);
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        session
            .load_irule(
                r"
            proc count_read {args} {incr ::static::reads}
            proc bump {name} {upvar 1 $name cell; incr cell}
            when RULE_INIT {
                set static::value 10
                set static::reads 0
                trace variable static::value r count_read
                set root_only 7
            }
            when HTTP_REQUEST {
                set flow_value 4
                call bump flow_value
                upvar 0 ::static::value alias
                set static::seen $alias
                set static::root $::root_only
                set dynamic {proc made_here {} {return yes}}
                eval $dynamic
            }
            when HTTP_RESPONSE {
                set static::flow $flow_value
                set static::seen_again $alias
                set static::proc_result [call made_here]
            }
        ",
            )
            .unwrap();
        session
            .eval("::orch::tmm_select 0; ::orch::tmm_select 1; ::orch::tmm_select 0")
            .unwrap();
        assert_eq!(
            session
                .eval("list [::orch::tmm_get_static 0 reads] [::orch::tmm_get_static 1 reads]")
                .unwrap(),
            "0 0",
            "selecting a worker must not read or recreate user cells"
        );
        let request = session.fire_event("HTTP_REQUEST").unwrap();
        assert!(!request.contains("code 1"), "{request}");
        let response = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(!response.contains("code 1"), "{response}");
        assert_eq!(session.eval("list [::orch::tmm_get_static 0 reads] [::orch::tmm_get_static 1 reads] [::orch::tmm_get_static 0 flow] [::orch::tmm_get_static 0 root] [::orch::tmm_get_static 0 proc_result]").unwrap(), "2 0 5 7 yes");
        assert_eq!(session.eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 1] {llength [info procs made_here]}").unwrap(), "0");
        session.eval("::orch::tmm_select 1").unwrap();
        let response = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(
            response.contains("code 1"),
            "another connection has its own cells: {response}"
        );
    }

    #[test]
    fn rule_init_uses_each_workers_global_cells_once() {
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("set ::seed 999; ::orch::configure_tests -tmm_count 2")
            .unwrap();
        for (worker, seed) in [(0, 11), (1, 29)] {
            session
                .eval(&format!(
                    "::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{set ::seed {seed}; set ::initialisations 0}}"
                ))
                .unwrap();
        }
        session
            .load_irule(
                r"
            proc bump {name} {
                global global_seed
                upvar 1 $name cell
                incr cell
                incr global_seed
                return [list $cell $global_seed]
            }
            when RULE_INIT {
                set static::seed $::seed
                set ::global_seed $::seed
                set static::initialisations [incr ::initialisations]
            }
            when HTTP_REQUEST {
                set static::seen [call bump ::static::seed]
            }
        ",
            )
            .unwrap();
        for (worker, expected) in [(0, "12 12"), (1, "30 30"), (0, "30 13")] {
            session
                .eval(&format!("::orch::tmm_select {worker}"))
                .unwrap();
            let event = session.fire_event("HTTP_REQUEST").unwrap();
            assert!(!event.contains("code 1"), "{event}");
            assert_eq!(
                session
                    .eval(&format!("::orch::tmm_get_static {worker} seen"))
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            session
                .eval("list [::orch::tmm_get_static 0 seed] [::orch::tmm_get_static 1 seed] [::orch::tmm_get_static 0 initialisations] [::orch::tmm_get_static 1 initialisations]")
                .unwrap(),
            "30 30 1 1",
            "RULE_INIT publishes static seeds; globals and later event writes retain worker ownership"
        );
        assert_eq!(
            session
                .eval("list $::seed [info exists ::global_seed] [info exists ::initialisations]")
                .unwrap(),
            "999 0 0",
            "worker global cells do not publish their mutations into the host interpreter"
        );
    }

    #[test]
    fn worker_static_broadcast_and_recreation_preserve_trace_owners() {
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        session
            .load_irule(
                r"
            proc watch {args} {incr ::static::writes}
            when RULE_INIT {
                set root_only 7
                set static::value 10
                set static::writes 0
                trace variable static::value w watch
            }
        ",
            )
            .unwrap();
        session
            .eval("::orch::tmm_select 0; ::orch::tmm_select 1")
            .unwrap();
        session.eval("::orch::configure_static value 20").unwrap();
        assert_eq!(
            session.eval("list [::orch::tmm_get_static 0 value] [::orch::tmm_get_static 1 value] [::orch::tmm_get_static 0 writes] [::orch::tmm_get_static 1 writes]").unwrap(),
            "20 20 1 1",
            "explicit configuration broadcasts stores into the existing worker cells"
        );
        session
            .eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 0] {unset ::static::value; set ::static::value 30}")
            .unwrap();
        session.eval("::orch::configure_static value 21").unwrap();
        assert_eq!(
            session.eval("list [::orch::tmm_get_static 0 value] [::orch::tmm_get_static 1 value] [::orch::tmm_get_static 0 writes] [::orch::tmm_get_static 1 writes]").unwrap(),
            "21 21 1 2",
            "recreating one worker's cell retires only that cell's old trace"
        );
        for worker in 0..2 {
            assert_eq!(
                session.eval(&format!("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{set ::root_only}}")).unwrap(),
                "7",
                "static configuration does not replace ordinary global cells"
            );
        }
        assert_eq!(session.eval("info exists ::static::value").unwrap(), "0");
    }

    #[test]
    fn worker_reset_reloads_independent_globals_and_initialisation() {
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("set ::host_marker HOST; ::orch::configure_tests -tmm_count 2")
            .unwrap();
        let source = r"
            proc bump {} {incr ::global_value; incr ::static::value}
            when RULE_INIT {
                set ::global_value $::seed
                set static::value $::seed
                set static::initialisations [incr ::initialisations]
            }
            when HTTP_REQUEST {call bump}
        ";
        for (epoch, seeds) in [(0, [11, 29]), (1, [41, 59])] {
            if epoch != 0 {
                session.eval("::orch::reset").unwrap();
                assert_eq!(
                    session
                        .eval("llength [::itest::registered_events]")
                        .unwrap(),
                    "0"
                );
                for worker in 0..2 {
                    assert_eq!(
                        session.eval(&format!("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{list [info exists ::global_value] [info exists ::static::value] [llength [info procs bump]]}}")).unwrap(),
                        "0 0 0",
                        "reset retires the previous worker's cells and command table"
                    );
                }
            }
            for (worker, seed) in seeds.into_iter().enumerate() {
                session.eval(&format!("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{set ::seed {seed}; set ::initialisations 0}}")).unwrap();
            }
            session.load_irule(source).unwrap();
            for worker in 0..2 {
                session
                    .eval(&format!("::orch::tmm_select {worker}"))
                    .unwrap();
            }
            session.eval("::orch::tmm_select 0").unwrap();
            let event = session.fire_event("HTTP_REQUEST").unwrap();
            assert!(!event.contains("code 1"), "{event}");
            for (worker, seed) in seeds.into_iter().enumerate() {
                let expected = seed + i32::from(worker == 0);
                assert_eq!(
                    session.eval(&format!("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{list $::global_value $::static::value $::static::initialisations}}")).unwrap(),
                    format!("{expected} {} 1", seeds[1] + i32::from(worker == 0)),
                    "globals initialise once per worker; later RULE_INIT publishes its static seed"
                );
            }
        }
        assert_eq!(
            session
                .eval("list $::host_marker [info exists ::global_value]")
                .unwrap(),
            "HOST 0"
        );
    }

    #[test]
    fn event_errors_preserve_cells_and_restore_worker_rule_activation() {
        use crate::session::{RuleIdentity, RuleSource};
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        session
            .load_rule(&RuleSource::named(
                RuleIdentity::new("/Common/failing").unwrap(),
                r"
            proc mutate {name} {
                upvar 1 $name cell
                incr cell
                incr ::global_value
                incr ::static::value
                error event_failed
            }
            when RULE_INIT {set ::global_value 11; set static::value 11}
            when HTTP_REQUEST {
                set flow_value 4
                upvar 0 flow_value alias
                call mutate flow_value
                set static::unreached 1
            }
            when HTTP_RESPONSE {set static::seen [list $flow_value $alias $::global_value]}
        ",
            ))
            .unwrap();
        session.eval("::orch::tmm_select 0").unwrap();
        let failed = session.fire_event("HTTP_REQUEST").unwrap();
        assert!(failed.contains("code 1 error event_failed"), "{failed}");
        let response = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(!response.contains("code 1"), "{response}");
        assert_eq!(
            session.eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 0] {list $::global_value $::static::value $::static::seen [info exists ::static::unreached] $::itest::_executing_rule $::itest::_current_rule}").unwrap(),
            "12 12 {5 5 12} 0 0 {}",
            "guest errors retain prior writes and connection aliases, then restore framework activation"
        );
        session.eval("::orch::tmm_select 1").unwrap();
        assert_eq!(
            session.eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 1] {list $::global_value $::static::value}").unwrap(),
            "11 11"
        );
        let missing = session.fire_event("HTTP_RESPONSE").unwrap();
        assert!(
            missing.contains("code 1"),
            "another worker cannot inherit the failed event's locals: {missing}"
        );
    }

    #[test]
    fn worker_array_recreation_retires_element_aliases_and_traces() {
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        session
            .load_irule(
                r"
            proc watch {args} {incr ::static::writes}
            when RULE_INIT {
                set static::cells(k) OLD
                set static::writes 0
                trace variable static::cells(k) w watch
            }
            when HTTP_REQUEST {
                upvar 0 ::static::cells(k) alias
                set alias FIRST
                unset ::static::cells
                set ::static::cells(k) NEW
                set static::stale_write [catch {set alias AFTER} message]
                set static::fresh $::static::cells(k)
            }
        ",
            )
            .unwrap();
        session.eval("::orch::tmm_select 0").unwrap();
        let event = session.fire_event("HTTP_REQUEST").unwrap();
        assert!(!event.contains("code 1"), "{event}");
        assert_eq!(
            session.eval("list [::orch::tmm_get_static 0 stale_write] [::orch::tmm_get_static 0 fresh] [::orch::tmm_get_static 0 writes]").unwrap(),
            "1 NEW 1",
            "an element alias cannot revive a retired array root or overwrite its replacement"
        );
        session.eval("::orch::tmm_select 1").unwrap();
        assert_eq!(
            session.eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 1] {list $::static::cells(k) $::static::writes}").unwrap(),
            "OLD 0"
        );
    }

    #[test]
    fn scheduled_callbacks_keep_worker_root_and_connection_cells_distinct() {
        use tcl_runtime_api::retained_activation::ScheduledCallbackOutcome;
        let mut session = LiveSession::embedded().unwrap();
        session
            .eval("::orch::configure_tests -tmm_count 2; ::orch::reset")
            .unwrap();
        for (worker, seed) in [11, 29].into_iter().enumerate() {
            session.eval(&format!("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters {worker}] {{set ::seed {seed}}}")).unwrap();
        }
        session
            .load_irule(
                r"
            proc tick {} { upvar 1 flow alias; incr alias; incr ::static::count }
            when RULE_INIT {
                set static::count $::seed
                after 2 {incr ::static::count}
            }
            when HTTP_REQUEST {
                set flow 4
                after 10 {tick}
            }
            when HTTP_RESPONSE { set static::observed $flow }
        ",
            )
            .unwrap();
        session.eval("::orch::tmm_select 0").unwrap();
        session.fire_event("HTTP_REQUEST").unwrap();
        session.eval("::orch::tmm_select 1").unwrap();
        let reports = session.advance_time(2);
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|report| matches!(&report.outcome, ScheduledCallbackOutcome::Guest(completion) if completion.is_ok())));
        assert_eq!(
            session
                .eval("list [::orch::tmm_get_static 0 count] [::orch::tmm_get_static 1 count]")
                .unwrap(),
            "30 30"
        );
        let reports = session.advance_time(8);
        assert_eq!(reports.len(), 1);
        assert!(
            matches!(&reports[0].outcome, ScheduledCallbackOutcome::Guest(completion) if completion.is_ok())
        );
        assert_eq!(
            session
                .eval("list [::orch::tmm_get_static 0 count] [::orch::tmm_get_static 1 count]")
                .unwrap(),
            "31 30"
        );
        session.eval("::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 0] {::itest::fire_event HTTP_RESPONSE}").unwrap();
        assert_eq!(
            session.eval("::orch::tmm_get_static 0 observed").unwrap(),
            "5"
        );
    }

    #[test]
    fn scheduled_callback_retirement_and_error_reports_preserve_original_options() {
        use tcl_runtime_api::retained_activation::{
            RetainedActivationRefusal, ScheduledCallbackOutcome,
        };
        let mut session = LiveSession::embedded().unwrap();
        session.load_irule(r"
            when RULE_INIT { set static::count 0 }
            when HTTP_REQUEST {
                after 3 {incr ::static::count; return -code error -level 2 -errorcode {TIMER EXACT} boom}
                after 6 {set ::static::stale MUST_NOT_RUN}
            }
        ").unwrap();
        session.fire_event("RULE_INIT").unwrap();
        session.fire_event("HTTP_REQUEST").unwrap();
        let reports = session.advance_time(3);
        assert_eq!(reports.len(), 1);
        let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
            panic!("{:?}", reports[0]);
        };
        assert_eq!(completion.code, tcl_runtime_api::Code::Return);
        assert_eq!(completion.result.string_bytes().as_ref(), b"boom");
        let options = completion.options.try_to_str().unwrap();
        assert!(options.contains("TIMER EXACT"), "{options}");
        assert!(options.contains("-level 2"), "{options}");
        assert_eq!(
            session
                .eval("list $::static::count $::itest::_executing_rule $::itest::_current_rule")
                .unwrap(),
            "1 0 {}"
        );
        session.eval("::itest::reset_connection_frame").unwrap();
        let reports = session.advance_time(3);
        assert!(matches!(
            reports[0].outcome,
            ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame)
        ));
        assert_eq!(session.eval("info exists ::static::stale").unwrap(), "0");
    }

    /// Verifies the harness's own dialect and availability-gate behaviour,
    /// sharing one session for the bootstrap cost like the suites above.
    ///
    /// Host control machinery remains available to the framework; event
    /// execution is restricted to the TMM surface. Source grammar and
    /// expression semantics retain the measured F5 dialect.
    #[test]
    fn retained_event_frame_preserves_native_completion_options() {
        let mut session = LiveSession::embedded().unwrap();
        for (body, expected) in [
            ("return yes", "code 0 result yes"),
            ("return -code error bad", "code 1 error bad"),
            ("break", "outside of a loop"),
            ("continue", "outside of a loop"),
        ] {
            session.eval("::itest::clear_irule").unwrap();
            session
                .load_irule(&format!("when HTTP_REQUEST {{{body}}}"))
                .unwrap();
            let completion = session.fire_event("HTTP_REQUEST").unwrap();
            assert!(completion.contains(expected), "{body}: {completion}");
        }
    }

    #[test]
    fn named_rules_keep_procedures_owned_and_route_call_without_extra_frames() {
        use crate::session::{RuleIdentity, RuleSource};
        let mut session = LiveSession::embedded().expect("framework");
        session
            .eval("::orch::configure_tests -tmm_count 2")
            .unwrap();
        for rule in [
            RuleSource::library(
                RuleIdentity::new("/Common/folder/helpers").unwrap(),
                r"
                proc shared {} {return common}
                proc mutate {name} {upvar 1 $name cell; incr cell}
                when HTTP_REQUEST {error library-event-must-not-run}
            ",
            ),
            RuleSource::library(
                RuleIdentity::new("/Other/helpers").unwrap(),
                r"
                proc shared {} {return other}
                proc nested {} {return [call shared]}
            ",
            ),
            RuleSource::named(
                RuleIdentity::new("/Common/folder/first").unwrap(),
                r"
                proc shared {} {return first}
                when RULE_INIT {set init_global 7; set static::calls 0}
                when HTTP_REQUEST {
                    set local 4
                    call helpers::mutate local
                    set static::seen [list [call shared] [call helpers::shared] [call /Other/helpers::nested] $local $::init_global]
                    incr static::calls
                }
            ",
            ),
            RuleSource::named(
                RuleIdentity::new("/Common/folder/second").unwrap(),
                r"
                proc shared {} {return second}
                when HTTP_REQUEST {set static::second [call shared]}
            ",
            ),
        ] {
            session.load_rule(&rule).expect("explicitly owned source");
        }
        for worker in 0..2 {
            session
                .eval(&format!("::orch::tmm_select {worker}"))
                .unwrap();
            let event = session.fire_event("HTTP_REQUEST").unwrap();
            assert!(!event.contains("code 1"), "{event}");
            assert_eq!(
                session
                    .eval(&format!("::orch::tmm_get_static {worker} seen"))
                    .unwrap(),
                "first common other 5 7"
            );
            assert_eq!(
                session
                    .eval(&format!("::orch::tmm_get_static {worker} second"))
                    .unwrap(),
                "second"
            );
        }
        assert_eq!(
            session
                .eval("list [::orch::tmm_get_static 0 calls] [::orch::tmm_get_static 1 calls]")
                .unwrap(),
            "0 1",
            "the second worker's RULE_INIT publishes a new static seed"
        );
        let duplicate = RuleSource::named(RuleIdentity::new("/Common/folder/first").unwrap(), "");
        assert!(
            session
                .load_rule(&duplicate)
                .unwrap_err()
                .to_string()
                .contains("identity already loaded")
        );
    }

    #[test]
    fn harness_runs_the_tmm_84_surface() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");

        // Framework commands remain available through private host names.
        scenario(&mut s);
        assert_eq!(
            s.eval("::tmm::_orig_dict get [::tmm::_orig_dict create a 1 b 2] b")
                .unwrap(),
            "2",
            "the framework dict command must answer"
        );
        assert_eq!(
            s.eval("::tmm::_orig_lassign {a b c} x y").unwrap(),
            "c",
            "the framework lassign command must answer"
        );

        s.load_irule("when RULE_INIT {dict create a 1}").unwrap();
        let initialisation = s.fire_event("RULE_INIT").unwrap();
        assert!(
            initialisation.contains("invalid command name"),
            "{initialisation}"
        );
        s.eval("::itest::clear_irule").unwrap();

        // sandbox_blocks_hidden_dict: at iRule scope the sandbox blocker owns
        // `dict`, so the 8.5+ spelling stays invalid exactly as on a TMM.
        s.load_irule("when HTTP_REQUEST {dict create a 1}").unwrap();
        let blocked = s.fire_event("HTTP_REQUEST").unwrap();
        assert!(
            blocked.contains("invalid command name"),
            "dict must be blocked in the event: {blocked}"
        );
        assert_eq!(
            s.eval("dict create a 1").unwrap(),
            "a 1",
            "host test code retains modern commands"
        );

        s.eval("::itest::clear_irule").unwrap();
        s.load_irule("when HTTP_REQUEST {::tcl::dict::create a 1}")
            .unwrap();
        let blocked_member = s.fire_event("HTTP_REQUEST").unwrap();
        assert!(
            blocked_member.contains("invalid command name"),
            "qualified members must obey the F5 surface: {blocked_member}"
        );
        s.eval("::itest::clear_irule").unwrap();
        s.load_irule("when HTTP_REQUEST {namespace eval ::static {dict create a 1}}")
            .unwrap();
        let blocked_static_namespace = s.fire_event("HTTP_REQUEST").unwrap();
        assert!(
            blocked_static_namespace.contains("invalid command name"),
            "static is user storage, not a private framework namespace: {blocked_static_namespace}"
        );

        // tmm_grammar_has_no_expansion: `{*}` is not TIP-157 expansion under
        // iRules. Its adjacent close/open braces are the dialect's ghost word
        // separator, so this is two literal list arguments rather than one
        // expanded argument.
        scenario(&mut s);
        assert_eq!(
            s.eval("llength [list {*}{a b}]").unwrap(),
            "2",
            "iRules must use its ghost separator rather than TIP-157 expansion"
        );
        // surface_hides_86_builtins: an 8.6+-only builtin with no polyfill
        // (lmap) does not exist at 8.4 — the miss reports through the
        // sandbox's resolution chain (which has also hidden the `unknown`
        // fallback, so the message names `unknown` rather than `lmap`;
        // either way the engine builtin must not run).
        scenario(&mut s);
        s.load_irule("when HTTP_REQUEST {lmap v {1 2} {expr {$v * 2}}}")
            .unwrap();
        let blocked = s.fire_event("HTTP_REQUEST").unwrap();
        assert!(
            blocked.contains("invalid command name"),
            "lmap must be blocked in the event: {blocked}"
        );
    }
}
