"""Tests for the LTM policy evaluator."""

from __future__ import annotations

from core.bigip.model import (
    BigipPolicy,
    BigipPolicyAction,
    BigipPolicyCondition,
    BigipPolicyRule,
)
from core.bigip.policy_eval import RequestState, evaluate_policy


def _policy(strategy: str, *rules: BigipPolicyRule) -> BigipPolicy:
    return BigipPolicy(
        name="p", full_path="/Common/p", strategy=strategy, rules=tuple(rules)
    )


def _rule(name: str, ordinal: int, *conds_and_acts) -> BigipPolicyRule:
    conds = tuple(c for c in conds_and_acts if isinstance(c, BigipPolicyCondition))
    acts = tuple(a for a in conds_and_acts if isinstance(a, BigipPolicyAction))
    return BigipPolicyRule(
        name=name, ordinal=ordinal, conditions=conds, actions=acts
    )


def _cond(operand, **kw) -> BigipPolicyCondition:
    kw.setdefault("index", 0)
    return BigipPolicyCondition(operand=operand, **kw)


def _act(target, **kw) -> BigipPolicyAction:
    kw.setdefault("index", 0)
    return BigipPolicyAction(target=target, **kw)


# Operands


def test_http_host_equals_match():
    r = _rule(
        "r",
        1,
        _cond("http-host", selector="host", operator="equals", values=("api.example.com",)),
        _act("forward", verb="select", pool="/Common/pool_api"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState(host="api.example.com"))
    assert d.rules[0].matched
    assert d.rules[0].fired
    assert d.rules[0].actions[0].value == "/Common/pool_api"


def test_http_host_equals_miss():
    r = _rule(
        "r",
        1,
        _cond("http-host", selector="host", operator="equals", values=("api.example.com",)),
        _act("forward", verb="select", pool="/Common/pool_api"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState(host="other.com"))
    assert not d.rules[0].matched
    assert not d.rules[0].fired


def test_http_uri_path_starts_with():
    r = _rule(
        "r",
        1,
        _cond("http-uri", selector="path", operator="starts-with", values=("/v1/",)),
        _act("forward", verb="select", pool="/Common/pool_v1"),
    )
    state = RequestState(uri="/v1/health", path="/v1/health")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].fired


def test_http_uri_query_contains():
    r = _rule(
        "r",
        1,
        _cond("http-uri", selector="query", operator="contains", values=("debug=1",)),
    )
    state = RequestState(uri="/x?debug=1&u=2", query="debug=1&u=2")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_http_uri_default_selector_matches_full_uri():
    r = _rule("r", 1, _cond("http-uri", operator="ends-with", values=(".jpg",)))
    state = RequestState(uri="/img/foo.jpg")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_http_method_equals():
    r = _rule(
        "r", 1, _cond("http-method", operator="equals", values=("POST",))
    )
    assert evaluate_policy(_policy("first-match", r), RequestState(method="POST")).rules[0].matched
    assert not evaluate_policy(_policy("first-match", r), RequestState(method="GET")).rules[0].matched


def test_http_header_with_named_target():
    r = _rule(
        "r",
        1,
        _cond(
            "http-header",
            name="X-Forwarded-Proto",
            operator="equals",
            values=("https",),
        ),
    )
    state = RequestState(headers={"x-forwarded-proto": "https"})
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_http_header_missing_when_not_in_capture():
    r = _rule(
        "r",
        1,
        _cond(
            "http-header",
            name="X-Custom",
            operator="equals",
            values=("y",),
        ),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState(headers={}))
    assert not d.rules[0].matched
    assert "not seen in capture" in d.rules[0].conditions[0].note


def test_http_header_no_name_is_unevaluable():
    r = _rule(
        "r",
        1,
        _cond("http-header", operator="equals", values=("x",)),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState(headers={"x": "y"}))
    assert not d.rules[0].matched
    assert "no `name`" in d.rules[0].conditions[0].note


def test_ssl_extension_sni_equals():
    r = _rule(
        "r",
        1,
        _cond(
            "ssl-extension",
            name="server-name",
            operator="equals",
            values=("api.example.com",),
        ),
    )
    state = RequestState(sni="api.example.com")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_ssl_extension_other_name_unsupported_in_v1():
    r = _rule(
        "r",
        1,
        _cond("ssl-extension", name="alpn", operator="equals", values=("h2",)),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert "not supported in v1" in d.rules[0].conditions[0].note


def test_tcp_address_equals():
    r = _rule(
        "r",
        1,
        _cond(
            "tcp", selector="address", operator="equals", values=("203.0.113.7",)
        ),
    )
    state = RequestState(client_addr="203.0.113.7")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


# Operators


def test_negate_inverts_match():
    r = _rule(
        "r",
        1,
        _cond(
            "http-uri",
            selector="path",
            operator="starts-with",
            values=("/internal/",),
            negate=True,
        ),
    )
    state = RequestState(uri="/api/x", path="/api/x")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_case_insensitive_equals():
    r = _rule(
        "r",
        1,
        _cond(
            "http-host",
            operator="equals",
            values=("API.EXAMPLE.COM",),
            case_insensitive=True,
        ),
    )
    state = RequestState(host="api.example.com")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_matches_uses_regex():
    r = _rule(
        "r",
        1,
        _cond(
            "http-uri",
            selector="path",
            operator="matches",
            values=(r"^/v\d+/health$",),
        ),
    )
    state = RequestState(path="/v3/health")
    assert evaluate_policy(_policy("first-match", r), state).rules[0].matched


def test_exists_operator_matches_when_field_present():
    r = _rule(
        "r",
        1,
        _cond("http-header", name="X-Trace", operator="exists"),
    )
    assert evaluate_policy(
        _policy("first-match", r), RequestState(headers={"x-trace": "y"})
    ).rules[0].matched
    assert not evaluate_policy(
        _policy("first-match", r), RequestState(headers={})
    ).rules[0].matched


# Strategy


def test_first_match_fires_only_first_matched_rule():
    r1 = _rule(
        "first",
        1,
        _cond("http-host", operator="equals", values=("a.com",)),
        _act("forward", verb="select", pool="/Common/p1"),
    )
    r2 = _rule(
        "second",
        2,
        _cond("http-host", operator="equals", values=("a.com",)),
        _act("forward", verb="select", pool="/Common/p2"),
    )
    d = evaluate_policy(_policy("first-match", r1, r2), RequestState(host="a.com"))
    assert [r.fired for r in d.rules] == [True, False]
    assert d.rules[0].actions[0].value == "/Common/p1"


def test_all_match_fires_every_matched_rule():
    r1 = _rule(
        "first",
        1,
        _cond("http-host", operator="equals", values=("a.com",)),
        _act("http-header", verb="insert", name="X-A", value="1"),
    )
    r2 = _rule(
        "second",
        2,
        _cond("http-host", operator="equals", values=("a.com",)),
        _act("http-header", verb="insert", name="X-B", value="2"),
    )
    d = evaluate_policy(_policy("all-match", r1, r2), RequestState(host="a.com"))
    assert [r.fired for r in d.rules] == [True, True]


def test_unconditional_rule_matches_with_no_conditions():
    """A rule with zero conditions is always matched (TMSH default-rule semantics)."""
    r = _rule(
        "default",
        99,
        _act("http-reply", verb="redirect", location="https://x/"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert d.rules[0].matched
    assert d.rules[0].fired


def test_first_match_falls_through_to_unconditional_rule():
    r1 = _rule(
        "specific",
        1,
        _cond("http-host", operator="equals", values=("api.example.com",)),
        _act("forward", verb="select", pool="/Common/pool_api"),
    )
    r_default = _rule(
        "default",
        99,
        _act("http-reply", verb="redirect", location="https://www.example.com/"),
    )
    d = evaluate_policy(
        _policy("first-match", r1, r_default), RequestState(host="other.com")
    )
    assert not d.rules[0].fired
    assert d.rules[1].fired
    assert d.rules[1].actions[0].value == "https://www.example.com/"


# Action summary values


def test_action_trace_summarises_http_uri_replace():
    r = _rule(
        "r",
        1,
        _act("http-uri", verb="replace", path="/v2/health"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert d.rules[0].actions[0].value == "path=/v2/health"


def test_action_trace_summarises_header_insert():
    r = _rule(
        "r",
        1,
        _act("http-header", verb="insert", name="X-A", value="1"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert d.rules[0].actions[0].value == "X-A=1"


def test_action_trace_summarises_header_remove():
    r = _rule(
        "r",
        1,
        _act("http-header", verb="remove", name="Server"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert d.rules[0].actions[0].value == "Server"


def test_action_trace_summarises_tcp_reset():
    r = _rule(
        "r",
        1,
        _act("tcp", verb="reset"),
    )
    d = evaluate_policy(_policy("first-match", r), RequestState())
    assert d.rules[0].actions[0].value == "RST"


# request_state_from_session


def test_request_state_from_session_lowercases_headers():
    from core.bigip.policy_eval import request_state_from_session

    class _FakeFlow:
        http_method = "GET"
        http_host = "api.example.com"
        http_uri = "/v1/health?debug=1"
        http_path = "/v1/health"
        http_query = "debug=1"
        http_request_headers = {"X-Forwarded-Proto": "https", "Host": "api.example.com"}
        tls_sni = "api.example.com"
        tls_chosen_version = "TLS1.3"
        tls_version = ""
        src_ip = "203.0.113.7"
        dst_ip = "10.0.0.1"

    class _FakeFront:
        client = _FakeFlow()

    class _FakeSession:
        front = _FakeFront()

    s = request_state_from_session(_FakeSession())
    assert s.method == "GET"
    assert s.host == "api.example.com"
    assert s.path == "/v1/health"
    assert s.query == "debug=1"
    assert s.sni == "api.example.com"
    assert s.tls_version == "TLS1.3"
    assert s.headers["x-forwarded-proto"] == "https"
    assert s.client_addr == "203.0.113.7"
