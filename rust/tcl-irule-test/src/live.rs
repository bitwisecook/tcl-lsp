//! Live VM driver — run the TMM-sim orchestrator Tcl on [`tcl-vm`] and drive a
//! real event round-trip.
//!
//! This is the runtime half of the iRule test session.
//! Where [`crate::session::SessionPlan`] assembles the *bootstrap script*, a
//! [`LiveSession`] actually stands the orchestrator up on a bytecode VM, loads
//! an iRule, fires events, and reads back the pool/node decisions, captured
//! logs, and assertion results — entirely in-process, no `tclsh` subprocess.
//!
//! The simulation itself is the ~500 KB of Tcl under `tooling/irule_test/tcl/`
//! (orchestrator + TMM shim + command mocks); the driver sources those files in
//! the same order `runner.tcl` does, then exposes a thin Rust API over the
//! `::orch::` command surface.

use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, CompileError, CompileService, Vm};

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

/// A session error: a failed bootstrap, compile, or orchestrator command. Holds
/// the Tcl-level message so callers can surface it verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// The orchestrator library directory or a required file is missing.
    MissingLib(String),
    /// A Tcl evaluation returned an error completion (the message is the result).
    Eval(String),
    /// A compile failure in the VM's compile service.
    Compile(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLib(p) => write!(f, "iRule-test library not found: {p}"),
            Self::Eval(m) => write!(f, "orchestrator error: {m}"),
            Self::Compile(m) => write!(f, "compile error: {m}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// The `CompileService` the VM uses to compile the orchestrator Tcl and any
/// runtime `eval` / command substitution: the real Rust compiler pipeline.
struct Svc(CommandRegistry);

impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

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
    /// `orchestrator.tcl` (i.e. `tooling/irule_test/tcl/`).
    ///
    /// # Errors
    /// [`SessionError::MissingLib`] if `lib_dir` or a required file is absent,
    /// or [`SessionError::Eval`] / [`SessionError::Compile`] if the framework
    /// fails to load.
    pub fn new(lib_dir: &Path) -> Result<Self, SessionError> {
        if !lib_dir.join("orchestrator.tcl").is_file() {
            return Err(SessionError::MissingLib(lib_dir.display().to_string()));
        }
        let output = Rc::new(RefCell::new(Vec::new()));
        let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&output))));
        vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
        let mut session = Self { vm, output };
        session.bootstrap(lib_dir)?;
        session.eval("::orch::init")?;
        Ok(session)
    }

    /// Stand a session up against the framework Tcl embedded in the binary
    /// (no on-disk `tooling/irule_test/tcl/` checkout needed). The bundled
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
        let script = format!("source {{{}}}", path.display());
        self.eval(&script).map(|_| ())
    }

    /// Evaluate a Tcl `script` and return its string result.
    ///
    /// # Errors
    /// [`SessionError::Eval`] on an error completion, [`SessionError::Compile`]
    /// if the script does not compile.
    pub fn eval(&mut self, script: &str) -> Result<String, SessionError> {
        let result = self.vm.eval_source(script);
        // A guest `exit` records a code on the VM rather than killing the host;
        // surface it as a handleable error and clear it so the next eval is
        // clean (the session, not the process, decides what to do).
        if let Some(code) = self.vm.take_exit() {
            return Err(SessionError::Eval(format!("script called exit {code}")));
        }
        match result {
            Ok(c) if c.code == Code::Error => {
                Err(SessionError::Eval(c.result.to_str().to_string()))
            }
            Ok(c) => Ok(c.result.to_str().to_string()),
            Err(e) => Err(SessionError::Compile(e.message)),
        }
    }

    /// Load an iRule into the orchestrator (`::orch::load_irule`). The source is
    /// passed as a braced word, so it must be brace-balanced (iRule bodies are).
    ///
    /// # Errors
    /// Propagates an orchestrator/compile error.
    pub fn load_irule(&mut self, source: &str) -> Result<(), SessionError> {
        self.eval(&format!("::orch::load_irule {{{source}}}"))
            .map(|_| ())
    }

    /// Run one HTTP request through the configured flow (`::orch::run_http_request`
    /// with the given `args` fragment, e.g. `-host api.example.com -uri /`).
    ///
    /// # Errors
    /// Propagates an orchestrator error (e.g. no matching flow chain).
    pub fn run_http_request(&mut self, args: &str) -> Result<String, SessionError> {
        self.eval(&format!("::orch::run_http_request {args}"))
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

    /// The in-crate orchestrator Tcl directory (the source the binary also
    /// embeds via `embedded.rs`).
    fn lib_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tcl")
            .canonicalize()
            .expect("orchestrator tcl dir")
    }

    #[test]
    fn live_session_routes_request_to_pool() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");
        s.eval("::orch::configure -profiles {TCP HTTP}").unwrap();
        s.eval("::orch::add_pool api_pool {10.0.1.1:8080}").unwrap();
        s.load_irule(
            "when HTTP_REQUEST {\n  if { [HTTP::host] eq \"api.example.com\" } {\n    pool api_pool\n  }\n}",
        )
        .unwrap();
        s.run_http_request("-host api.example.com -uri /").unwrap();
        assert_eq!(s.pool_selected().unwrap(), "api_pool");
    }

    #[test]
    fn live_session_records_reject_decision() {
        let mut s = LiveSession::new(&lib_dir()).expect("session");
        s.eval("::orch::configure -profiles {TCP HTTP}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  reject\n}").unwrap();
        s.run_http_request("-host evil.example.com -uri /").unwrap();
        let decisions = s.decisions().unwrap();
        assert!(
            decisions.contains("connection reject"),
            "decisions: {decisions}"
        );
    }

    #[test]
    fn embedded_session_routes_request() {
        let mut s = LiveSession::embedded().expect("embedded session");
        s.eval("::orch::configure -profiles {TCP HTTP}").unwrap();
        s.eval("::orch::add_pool web {10.0.2.1:80}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  pool web\n}").unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        assert_eq!(s.pool_selected().unwrap(), "web");
    }

    #[test]
    fn embedded_session_includes_generated_stubs() {
        // `_mock_stubs.tcl` provides mocks for registry-only iRule commands (no
        // hand-written mock). Without it bundled, a stub-only command like
        // `ACCESS::session` errors "invalid command name" inside the handler,
        // which `fire_event`'s `catch` stops — so the following `pool` never
        // runs. Reaching the pool selection proves the stub dispatched.
        let mut s = LiveSession::embedded().expect("embedded session");
        s.eval("::orch::configure -profiles {TCP HTTP}").unwrap();
        s.eval("::orch::add_pool web {10.0.2.1:80}").unwrap();
        s.load_irule("when HTTP_REQUEST {\n  ACCESS::session\n  pool web\n}")
            .unwrap();
        s.run_http_request("-host x.example.com -uri /").unwrap();
        assert_eq!(
            s.pool_selected().unwrap(),
            "web",
            "stub-only ACCESS::session must dispatch, not abort the handler"
        );
    }

    #[test]
    fn guest_exit_does_not_kill_the_host() {
        // The whole point of routing `exit` through a VM completion: a guest
        // script calling `exit` must NOT terminate this test process. If it
        // still called `std::process::exit`, the test binary would die here.
        let mut s = LiveSession::embedded().expect("embedded session");
        match s.eval("exit 7") {
            Err(SessionError::Eval(_)) => {}
            other => panic!("exit should surface as a handleable error, got {other:?}"),
        }
        // The session is still usable afterwards.
        assert_eq!(s.eval("expr {1 + 1}").unwrap(), "2");
    }

    #[test]
    fn missing_lib_dir_errors() {
        match LiveSession::new(Path::new("/no/such/dir")) {
            Err(SessionError::MissingLib(_)) => {}
            Err(other) => panic!("wrong error: {other}"),
            Ok(_) => panic!("expected a MissingLib error"),
        }
    }
}
