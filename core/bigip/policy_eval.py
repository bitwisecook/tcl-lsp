"""Evaluate parsed BIG-IP LTM policies against captured request state.

Operates on the model produced by :mod:`core.bigip.parser` — purely
declarative, no c-tcl or runtime needed.  The evaluator is the same
under static analysis and ``simulate=true``: LTM policies don't run
arbitrary code, so the captured request fully determines the outcome.

Supported operand vocabulary (the ``medium`` scope agreed for v1):

* ``http-host`` — host header, exact / starts-with / ends-with /
  contains / matches.
* ``http-uri`` with selectors ``host``, ``path``, ``query`` (and the
  default of the full URI when no selector is given).
* ``http-method`` — request method.
* ``http-header`` with ``name <header>`` — value of a named header.
* ``ssl-extension`` with ``name server-name`` — TLS SNI.
* ``tcp`` with selector ``address`` — client source address.

Action surface:

* ``forward select pool <path>``
* ``http-reply redirect location <url>``
* ``http-uri replace path|query|host <value>``
* ``http-header insert name <h> value <v>``
* ``http-header remove name <h>``
* ``tcp reset``

Out of scope for v1: cookie operands/actions, geoip/cert operands,
rate-limit / log / shutdown actions, condition events other than
``request``/``ssl-client-hello`` (we still surface them in the trace,
but don't try to interpret response-phase state).
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from urllib.parse import urlsplit

from .model import BigipPolicy, BigipPolicyAction, BigipPolicyCondition


@dataclass(frozen=True, slots=True)
class RequestState:
    """The captured request state a policy condition can read.

    Build one with :func:`request_state_from_session`.  Header lookups
    are case-insensitive: store header *names* lowercased.
    """

    method: str = ""
    host: str = ""
    uri: str = ""
    path: str = ""
    query: str = ""
    headers: dict[str, str] = field(default_factory=dict)
    sni: str = ""
    tls_version: str = ""
    client_addr: str = ""
    server_addr: str = ""


@dataclass(frozen=True, slots=True)
class ConditionTrace:
    """Per-condition trace: what the condition saw and whether it matched."""

    operand: str
    selector: str
    operator: str
    expected: tuple[str, ...]
    actual: str
    matched: bool
    name: str = ""  # http-header / ssl-extension target name
    negate: bool = False
    event: str = ""
    note: str = ""  # populated when we couldn't evaluate (operand unsupported, no data)


@dataclass(frozen=True, slots=True)
class ActionTrace:
    """An action that fired (only present on matched rules)."""

    target: str
    verb: str
    value: str = ""  # most-relevant single value: pool / location / "name=value"


@dataclass(frozen=True, slots=True)
class RuleDecision:
    """Per-rule decision: which conditions matched and (if fired) which actions ran."""

    rule: str
    ordinal: int
    matched: bool
    fired: bool
    conditions: tuple[ConditionTrace, ...]
    actions: tuple[ActionTrace, ...]


@dataclass(frozen=True, slots=True)
class PolicyDecision:
    """Per-policy decision: the rule walk plus the strategy that picked the winner."""

    policy: str
    strategy: str
    rules: tuple[RuleDecision, ...]


# Public API


def evaluate_policy(policy: BigipPolicy, state: RequestState) -> PolicyDecision:
    """Evaluate ``policy`` against ``state`` and return per-rule decisions.

    Strategies:

    * ``first-match`` — runs only the first rule whose conditions all match.
    * ``all-match`` — runs actions of every matched rule.
    * ``best-match`` (and unknown) — picks the matched rule with the
      largest condition count.  F5's true best-match weights operand
      specificity; this is a deliberate approximation we surface in
      :attr:`PolicyDecision.strategy` so consumers can warn.
    """
    matched_indexes: list[int] = []
    rules_built: list[RuleDecision] = []
    for rule in policy.rules:
        cond_traces = tuple(_evaluate_condition(c, state) for c in rule.conditions)
        # A rule with no conditions is unconditional (matches always);
        # this matches TMSH semantics for ``default`` rules.
        all_ok = all(t.matched for t in cond_traces)
        rules_built.append(
            RuleDecision(
                rule=rule.name,
                ordinal=rule.ordinal,
                matched=all_ok,
                fired=False,
                conditions=cond_traces,
                actions=(),
            )
        )

    strategy = (policy.strategy or "first-match").lower()
    if strategy == "first-match":
        for i, decision in enumerate(rules_built):
            if decision.matched:
                matched_indexes.append(i)
                break
    elif strategy == "all-match":
        matched_indexes = [i for i, d in enumerate(rules_built) if d.matched]
    else:  # best-match approximation
        best_i = -1
        best_score = -1
        for i, decision in enumerate(rules_built):
            if not decision.matched:
                continue
            score = len(decision.conditions)
            if score > best_score:
                best_score = score
                best_i = i
        if best_i >= 0:
            matched_indexes.append(best_i)

    fired = set(matched_indexes)
    final: list[RuleDecision] = []
    for i, decision in enumerate(rules_built):
        if i in fired:
            actions = tuple(_action_trace(a) for a in policy.rules[i].actions)
            final.append(
                RuleDecision(
                    rule=decision.rule,
                    ordinal=decision.ordinal,
                    matched=True,
                    fired=True,
                    conditions=decision.conditions,
                    actions=actions,
                )
            )
        else:
            final.append(decision)

    return PolicyDecision(policy=policy.full_path, strategy=strategy, rules=tuple(final))


def request_state_from_session(session) -> RequestState:  # noqa: ANN001 — Session lives in explain_flow
    """Derive a :class:`RequestState` from an explain-flow Session.

    Only the front-side client flow is consulted — that's the request
    state the policy evaluates against.  Header keys are lowercased.
    """
    fc = session.front.client
    headers = {k.lower(): v for k, v in (fc.http_request_headers or {}).items()}
    # Host header trumps the captured Host field if present.
    host = (headers.get("host") or fc.http_host or "").strip()
    return RequestState(
        method=fc.http_method or "",
        host=host,
        uri=fc.http_uri or "",
        path=fc.http_path or _split_path(fc.http_uri or ""),
        query=fc.http_query or _split_query(fc.http_uri or ""),
        headers=headers,
        sni=fc.tls_sni or "",
        tls_version=fc.tls_chosen_version or fc.tls_version or "",
        client_addr=fc.src_ip or "",
        server_addr=fc.dst_ip or "",
    )


# Internals


def _evaluate_condition(cond: BigipPolicyCondition, state: RequestState) -> ConditionTrace:
    actual, note = _extract_actual(cond, state)
    if cond.operator in ("exists", "missing"):
        present = bool(actual)
        raw = present if cond.operator == "exists" else not present
    else:
        raw = _apply_operator(cond.operator, actual, cond.values, cond.case_insensitive)
    matched = raw if not cond.negate else not raw
    return ConditionTrace(
        operand=cond.operand,
        selector=cond.selector,
        operator=cond.operator,
        expected=cond.values,
        actual=actual,
        matched=matched,
        name=cond.name,
        negate=cond.negate,
        event=cond.event,
        note=note,
    )


def _extract_actual(cond: BigipPolicyCondition, state: RequestState) -> tuple[str, str]:
    """Return ``(observed_value, note)`` for *cond*.

    ``note`` is non-empty when we deliberately couldn't evaluate
    (operand unsupported in v1, or required data absent from the
    capture).  The trace surfaces it so the operator knows why a
    condition didn't match.
    """
    op = cond.operand
    sel = cond.selector
    if op == "http-host":
        return state.host, ("" if state.host else "no Host captured")
    if op == "http-uri":
        if sel == "path":
            return state.path, ("" if state.path else "no URI captured")
        if sel == "query":
            return state.query, ("" if state.query else "no query string captured")
        if sel == "host":
            return state.host, ("" if state.host else "no Host captured")
        return state.uri, ("" if state.uri else "no URI captured")
    if op == "http-method":
        return state.method, ("" if state.method else "no method captured")
    if op == "http-header":
        if not cond.name:
            return "", "http-header condition has no `name` — cannot evaluate"
        v = state.headers.get(cond.name.lower(), "")
        return v, ("" if v else f"header {cond.name!r} not seen in capture")
    if op == "ssl-extension":
        if cond.name and cond.name.lower() != "server-name":
            return "", f"ssl-extension {cond.name!r} not supported in v1"
        return state.sni, ("" if state.sni else "no SNI captured")
    if op == "tcp":
        if sel == "address":
            return state.client_addr, ""
        return "", f"tcp selector {sel!r} not supported in v1"
    return "", f"operand {op!r} not supported in v1"


def _apply_operator(
    operator: str, actual: str, values: tuple[str, ...], case_insensitive: bool
) -> bool:
    if not values:
        # equals/contains/etc with no values can never match.
        return False
    a = actual
    vs = values
    if case_insensitive:
        a = a.lower()
        vs = tuple(v.lower() for v in values)
    if operator == "equals":
        return a in vs
    if operator == "starts-with":
        return any(a.startswith(v) for v in vs)
    if operator == "ends-with":
        return any(a.endswith(v) for v in vs)
    if operator == "contains":
        return any(v in a for v in vs)
    if operator == "matches":
        flags = re.IGNORECASE if case_insensitive else 0
        return any(re.search(v, actual, flags) is not None for v in values)
    if operator == "less":
        return _numeric_compare(actual, values, lambda x, y: x < y)
    if operator == "greater":
        return _numeric_compare(actual, values, lambda x, y: x > y)
    if operator == "less-or-equal":
        return _numeric_compare(actual, values, lambda x, y: x <= y)
    if operator == "greater-or-equal":
        return _numeric_compare(actual, values, lambda x, y: x >= y)
    return False


def _numeric_compare(actual: str, values: tuple[str, ...], op) -> bool:  # noqa: ANN001
    try:
        a = float(actual)
    except (TypeError, ValueError):
        return False
    for v in values:
        try:
            if op(a, float(v)):
                return True
        except (TypeError, ValueError):
            continue
    return False


def _action_trace(action: BigipPolicyAction) -> ActionTrace:
    return ActionTrace(
        target=action.target,
        verb=action.verb,
        value=_action_summary_value(action),
    )


def _action_summary_value(action: BigipPolicyAction) -> str:
    if action.target == "forward" and action.pool:
        return action.pool
    if action.target == "http-reply" and action.location:
        return action.location
    if action.target == "http-uri" and action.verb == "replace":
        if action.path:
            return f"path={action.path}"
        if action.query:
            return f"query={action.query}"
    if action.target == "http-header":
        if action.verb == "insert":
            return f"{action.name}={action.value}"
        if action.verb == "remove":
            return action.name
    if action.target == "tcp" and action.verb == "reset":
        return "RST"
    # Fall back to the most relevant non-empty field so callers always
    # see something actionable in the trace.
    for v in (action.pool, action.location, action.name, action.value, action.path, action.query):
        if v:
            return v
    return ""


def _split_path(uri: str) -> str:
    if not uri:
        return ""
    return urlsplit(uri).path or uri.split("?", 1)[0]


def _split_query(uri: str) -> str:
    if not uri:
        return ""
    return urlsplit(uri).query or (uri.split("?", 1)[1] if "?" in uri else "")
