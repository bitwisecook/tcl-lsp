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

use super::*;
use std::path::PathBuf;

struct InitPolicyObserver(Rc<RefCell<Vec<Option<tcl_syntax::naming::NamePolicyProtocol>>>>);

impl NativeCommand for InitPolicyObserver {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        if !args.is_empty() {
            return vm
                .refuse_host_command("initialization policy observer takes no arguments".into());
        }
        self.0.borrow_mut().push(vm.name_policy_protocol());
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}

#[test]
fn native_framework_initialization_restores_f5_policy_after_success_and_error() {
    let mut session = LiveSession::new(&lib_dir()).expect("session");
    let before = session.vm.name_policy_protocol();
    assert_eq!(
        before,
        Some(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert_eq!(session.vm.dialect_profile().name, "f5-irules");
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let point = tcl_registry::InvocationDialect::of_profile(host)
        .execution_point()
        .unwrap();
    let host_policy = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point);
    assert!(host_policy.is_some());
    assert_ne!(before, host_policy);
    let observed = Rc::new(RefCell::new(Vec::new()));
    session.vm.register_native_command(
        "::tmm::_native_init_policy_probe",
        Rc::new(InitPolicyObserver(Rc::clone(&observed))),
    );
    session
        .eval_final_host_initialization("::tmm::_native_init_policy_probe")
        .unwrap();
    assert_eq!(observed.borrow().as_slice(), &[host_policy]);
    assert_eq!(session.vm.name_policy_protocol(), before);
    assert_eq!(session.vm.dialect_profile().name, "f5-irules");
    let error = session
        .eval_final_host_initialization(
            "::tmm::_native_init_policy_probe; error EXPECTED_NATIVE_INIT_FAILURE",
        )
        .unwrap_err();
    assert!(
        matches!(error, SessionError::Eval(ref message) if message == "EXPECTED_NATIVE_INIT_FAILURE")
    );
    assert_eq!(observed.borrow().as_slice(), &[host_policy, host_policy]);
    assert_eq!(session.vm.name_policy_protocol(), before);
    assert_eq!(session.vm.dialect_profile().name, "f5-irules");
    // The framework blocks public exit and retains the actual host worker.
    let blocked_exit = session
        .eval_final_host_initialization("exit 7")
        .unwrap_err();
    assert!(
        matches!(blocked_exit, SessionError::Eval(ref message) if message == "invalid command name \"exit\"")
    );
    assert!(session.vm.take_exit().is_none());
    assert_eq!(session.vm.name_policy_protocol(), before);
    let exit = session
        .eval_final_host_initialization("::tmm::_native_init_policy_probe; ::tmm::_orig_exit 7")
        .unwrap_err();
    assert!(
        matches!(exit, SessionError::Eval(ref message) if message == "script called exit 7"),
        "unexpected final host initialization exit settlement: {exit:?}"
    );
    assert!(session.vm.take_exit().is_none());
    assert_eq!(
        observed.borrow().as_slice(),
        &[host_policy, host_policy, host_policy]
    );
    assert_eq!(session.vm.name_policy_protocol(), before);
    assert_eq!(session.vm.dialect_profile().name, "f5-irules");
}

#[test]
fn measured_event_names_enter_the_actual_connection_frame_and_restore_policy() {
    let mut session = LiveSession::new(&lib_dir()).expect("session");
    let baseline = session.vm.execution_name_policy();
    assert!(session.set_measured_event_name_build(Some(BigIpBuild::MEASURED_21_1_0_1)));
    session.eval("::itest::_execute_event_body HTTP_REQUEST {set {__tcl_lsp_2286_r2286m_nul_A\0B} counted}").expect("counted event write");
    assert_eq!(
        session
            .eval(
                "::itest::_execute_event_body HTTP_REQUEST {set {__tcl_lsp_2286_r2286m_nul_A\0B}}"
            )
            .expect("same connection read"),
        "0 counted {}"
    );
    assert_eq!(session.vm.execution_name_policy(), baseline);
    let failed = session
        .eval("::itest::_execute_event_body HTTP_REQUEST {error EXPECTED}")
        .expect("guest error completion");
    assert!(failed.starts_with("1 EXPECTED"));
    assert_eq!(session.vm.execution_name_policy(), baseline);
    assert!(session.set_measured_event_name_build(None));
    assert_eq!(session.vm.execution_name_policy(), baseline);
}

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
        vm: tcl_vm::test_support::native_vm(),
        output: Rc::default(),
        measured_loader: None,
        measured_names: Rc::default(),
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
        vm: tcl_vm::test_support::native_vm(),
        output: Rc::default(),
        measured_loader: None,
        measured_names: Rc::default(),
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
    assert_eq!(
        tcl_irules::when_blocks(quoted_close),
        [] as [tcl_syntax::event_handler::EventHandler; 0]
    );
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
/// expression arguments in command positions, including `for`'s second
/// argument. Quoted data, braced data and comments stay unchanged.
/// Each negative assertion has a positive control for the rewriter.
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
fn authored_functions_keep_independent_worker_random_and_precision_contexts() {
    let mut session = LiveSession::embedded().expect("framework");
    assert_eq!(session.eval("expr {abs(077)}").unwrap(), "63");
    assert_eq!(session.eval("expr {sqrt(2)}").unwrap(), "1.41421356237");
    session
        .eval("::orch::configure_tests -tmm_count 2")
        .unwrap();
    let worker0 = "::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 0]";
    let worker1 = "::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 1]";
    assert_eq!(
        session
            .eval(&format!("{worker0} {{expr {{srand(1)}}}}"))
            .unwrap(),
        "7.82636925943e-06"
    );
    assert_eq!(
        session
            .eval(&format!("{worker1} {{expr {{rand()}}}}"))
            .unwrap(),
        "7.82636925943e-06"
    );
    assert_eq!(
        session
            .eval(&format!(
                "{worker0} {{set tcl_precision 4;expr {{sqrt(2)}}}}"
            ))
            .unwrap(),
        "1.414"
    );
    assert_eq!(
        session
            .eval(&format!("{worker1} {{expr {{sqrt(2)}}}}"))
            .unwrap(),
        "1.41421356237"
    );
    assert_eq!(
        session
            .eval(&format!("{worker0} {{expr {{rand()}}}}"))
            .unwrap(),
        "0.1315"
    );
    assert_eq!(
        session
            .eval(&format!("{worker1} {{expr {{rand()}}}}"))
            .unwrap(),
        "0.131537788143"
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
fn rule_creation_and_recreation_initialise_only_the_owned_rule_on_four_workers() {
    use crate::session::{RuleIdentity, RuleSource};
    let mut session = LiveSession::embedded().expect("framework");
    session
        .eval("::orch::configure_tests -tmm_count 4")
        .unwrap();
    let a = RuleSource::named(
        RuleIdentity::new("/Common/a").unwrap(),
        "when RULE_INIT {set static::collision INIT_A}; when HTTP_REQUEST {set static::event A}",
    );
    let b = RuleSource::named(
        RuleIdentity::new("/Common/b").unwrap(),
        "when RULE_INIT {set static::collision INIT_B}; when HTTP_REQUEST {set static::event B}",
    );
    session.load_rule(&a).unwrap();
    session.load_rule(&b).unwrap();
    for worker in 0..4 {
        assert_eq!(
            session
                .eval(&format!("::orch::tmm_get_static {worker} collision"))
                .unwrap(),
            "INIT_B"
        );
    }
    session.eval("::orch::tmm_select 2; ::tmm::_orig_interp eval [lindex $::orch::_tmm_interpreters 2] {unset ::static::collision; set ::static::collision EVENT}").unwrap();
    for worker in 0..4 {
        assert_eq!(
            session
                .eval(&format!("::orch::tmm_get_static {worker} collision"))
                .unwrap(),
            if worker == 2 { "EVENT" } else { "INIT_B" }
        );
    }
    session.replace_rule(&a).unwrap();
    for worker in 0..4 {
        session
            .eval(&format!("::orch::tmm_select {worker}"))
            .unwrap();
        assert_eq!(
            session
                .eval(&format!("::orch::tmm_get_static {worker} collision"))
                .unwrap(),
            "INIT_A"
        );
    }
    session.replace_rule(&b).unwrap();
    for worker in 0..4 {
        assert_eq!(
            session
                .eval(&format!("::orch::tmm_get_static {worker} collision"))
                .unwrap(),
            "INIT_B"
        );
    }
}

#[test]
fn measured_loader_checks_original_source_and_keeps_dynamic_runtime_commands() {
    use tcl_registry::irules_policy::MeasuredIrulesLoaderProfile;
    let mut session = LiveSession::embedded().expect("framework");
    session
        .set_measured_loader_profile(Some(MeasuredIrulesLoaderProfile::BigIp21_1_0_1Build0_0_26));
    for source in [
        "when HTTP_REQUEST {namespace eval N {}}",
        "when HTTP_REQUEST {interp slaves}",
        "when HTTP_REQUEST {package provide Tcl}",
        "when HTTP_REQUEST {rename missing moved}",
        "proc same {} {return A}; when HTTP_REQUEST {same}",
        "when HTTP_REQUEST {proc p {} {return x}}",
        "when HTTP_REQUEST {set x é}",
        "when HTTP_REQUEST {set x e\u{301}}",
        "when HTTP_REQUEST {set x A\0B}",
    ] {
        assert!(session.load_irule(source).is_err(), "{source}");
    }
    session
        .load_irule(
            r"when HTTP_REQUEST {
            set n [list namespace eval ::probe {set n 11}]
            set p [list package provide Tcl]
            set q [list namespace path]
            set static::measured [list [eval $n] [eval $p] [catch {eval $q} message] $message]
        }",
        )
        .unwrap();
    assert!(
        !session
            .fire_event("HTTP_REQUEST")
            .unwrap()
            .contains("code 1")
    );
    let result = session.eval("set ::static::measured").unwrap();
    assert!(result.starts_with("11 8.4 1 "), "{result}");
    assert!(result.contains("bad option"), "{result}");
    assert!(
        session
            .eval("::tmm::_orig_package provide Tcl")
            .unwrap()
            .starts_with("9.0")
    );
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
                    call /Common/folder/helpers::mutate local
                    set static::seen [list [call shared] [call /Common/folder/helpers::shared] [call /Other/helpers::nested] $local $::init_global]
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
        "1 1",
        "selection does not rerun rule-creation initialisation"
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
fn measured_rule_lookup_uses_partition_root_and_keeps_activated_callables() {
    use crate::session::{RuleIdentity, RuleSource};
    let mut session = LiveSession::embedded().expect("framework");
    session
        .eval("::orch::configure_tests -tmm_count 4")
        .unwrap();
    let source = r"
            proc identify {} {return partition}
            proc inspect {name} {
                upvar 1 $name cell
                set cell changed
                set command [list namespace current]
                set lookup [list namespace which -command /Common/helpers::identify]
                return [list [eval $command] [eval $lookup]]
            }
        ";
    session
        .load_rule(&RuleSource::library(
            RuleIdentity::new("/Common/helpers").unwrap(),
            source,
        ))
        .unwrap();
    session
        .load_rule(&RuleSource::library(
            RuleIdentity::new("/Common/folder/helpers").unwrap(),
            "proc identify {} {return folder}",
        ))
        .unwrap();
    session.load_rule(&RuleSource::named(RuleIdentity::new("/Common/folder/caller").unwrap(), r"
            proc identify {} {return caller}
            when HTTP_REQUEST {
                set local original
                set context [call helpers::inspect local]
                set static::owner_result [list [call identify] [call helpers::identify] [call /Common/folder/helpers::identify] $context $local]
            }
        ")).unwrap();
    for deleted in [false, true] {
        if deleted {
            session
                .eval("::itest::unload_rule /Common/helpers")
                .unwrap();
        }
        for worker in 0..4 {
            session
                .eval(&format!("::orch::tmm_select {worker}"))
                .unwrap();
            let result = session.fire_event("HTTP_REQUEST").unwrap();
            assert!(
                !result.contains("code 1"),
                "worker {worker}, deleted {deleted}: {result}"
            );
            assert_eq!(
                session
                    .eval(&format!("::orch::tmm_get_static {worker} owner_result"))
                    .unwrap(),
                "caller partition folder {:: {}} changed"
            );
        }
    }
    session
        .load_rule(&RuleSource::library(
            RuleIdentity::new("/Common/helpers").unwrap(),
            source,
        ))
        .unwrap();
    session.eval("::itest::clear_irule").unwrap();
    assert_eq!(
        session
            .eval("::tmm::_orig_namespace exists ::tmm::_rule_callables")
            .unwrap(),
        "0"
    );
}

/// Inside the simulation a list is a Tcl 8.4 object and in the framework a
/// Tcl 9 one: each realm renders and parses by its own recipe, as tclsh
/// 8.4.20 and 9.0.4 do, and a list sealed by one is refused by the other.
#[test]
fn each_realm_keeps_its_own_list_recipe() {
    // (script, tclsh 8.4.20, tclsh 9.0.4)
    const CASES: &[(&str, &str, &str)] = &[
        ("list #a b", "#a b", "{#a} b"),
        ("lindex {a\\x4142 b} 0", "aB", "aA42"),
    ];
    let mut s = LiveSession::new(&lib_dir()).expect("session");
    for (script, tcl84, tcl90) in CASES {
        assert_eq!(s.eval(script).unwrap(), *tcl84, "authored `{script}`");
        assert_eq!(
            s.eval_final_host_initialization(script).unwrap(),
            *tcl90,
            "framework `{script}`"
        );
    }
    for (version, tcl84) in [
        (tcl_dialect::TclVersion::V8_4, true),
        (tcl_dialect::TclVersion::V9_0, false),
    ] {
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        for (script, old, new) in CASES {
            let outcome =
                tcl_test_support::run_script(&tclsh.path, format!("puts [{script}]\n").as_bytes())
                    .expect("tclsh");
            let text = outcome.strict_text().expect("tclsh answers");
            assert_eq!(text.trim_end(), if tcl84 { *old } else { *new }, "{script}");
        }
    }
    s.eval_final_host_initialization("set ::crossing [list a b]")
        .unwrap();
    assert!(
        s.eval("lindex $::crossing 0").is_err(),
        "an authored reader cannot reparse a framework-sealed list"
    );
}

#[test]
fn measured_counted_lengths_keep_dynamic_bytes_and_literal_strings_separate_from_host() {
    let mut session = LiveSession::embedded().expect("framework");
    session.load_irule(r#"
            when HTTP_REQUEST {
                set literal "😀"
                set dynamic [binary format H* f09f918df09f8fbd]
                set nul [binary format H* 410042]
                set static::lengths [list [string length $literal] [string len $literal] [string length $dynamic] [string length $nul]]
            }
        "#).unwrap();
    let result = session.fire_event("HTTP_REQUEST").unwrap();
    assert!(!result.contains("code 1"), "{result}");
    assert_eq!(session.eval("set ::static::lengths").unwrap(), "4 4 8 3");
    assert_eq!(session.eval("::tmm::_orig_string length 😀").unwrap(), "1");
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

#[test]
fn an_irule_expression_reads_numerals_by_the_f5_policy_not_the_host_engine() {
    // The ::expr override evaluates the rewritten expression in the caller's
    // context, not under the framework's host catch. F5 expressions take 8.4
    // rules (BIG-IP check 23); tclsh 8.4.20 reads 077 as octal.
    let mut session = LiveSession::embedded().expect("framework");
    for (script, expected) in [
        ("expr {077}", "63"),
        ("expr {abs(077)}", "63"),
        ("set e 077; expr $e", "63"),
        ("proc octal {} {expr {077 + 1}}; octal", "64"),
    ] {
        assert_eq!(session.eval(script).unwrap(), expected, "{script}");
    }
}
