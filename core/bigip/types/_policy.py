"""Typed BIG-IP policy condition / action values.

``ltm policy`` rules have two nested mini-languages — one for
conditions (predicates that gate when the rule fires) and one for
actions (what happens when the predicate matches).  The legacy
parser flattens both to dataclass fields with raw-token tuples;
this module surfaces the structure so the query DSL can filter
policies by what they actually match on / do, and the rewrite
layer can update one clause safely without disturbing siblings.

Examples:

    rules {
        match-host {
            conditions {
                0 { http-host all-strings values { example.com } }
            }
            actions {
                0 { forward select pool /Common/web_pool }
            }
        }
    }

The matching specs (in :mod:`core.bigip.registry.value_specs`)
wire these typed values into the registry's parse / project /
render / references dispatch.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class LtmPolicyCondition:
    """One clause inside a policy rule's ``conditions { ... }`` block.

    ``index`` is the integer key BIG-IP assigns each clause (clauses
    are ordered).  ``operand`` is the protocol element the clause
    matches against (``http-host`` / ``http-uri`` / ``ssl-cert`` /
    ``tcp`` / ``ip``...).  ``selector`` is the sub-attribute of the
    operand (e.g. ``all-strings``, ``port``); empty when the operand
    has no sub-attribute.  ``operator`` is the match operator
    (``equals`` / ``contains`` / ``starts-with`` / ``matches`` —
    when omitted in the source we default to ``equals``).  ``values``
    is the ordered list of literals the operator compares against
    (``values { example.com foo.example.com }``).  ``negate`` /
    ``case_insensitive`` / ``external`` mirror the documented flags.
    ``event`` is the policy event the condition runs against
    (``request`` / ``response`` / ``proxy-request`` / ...); empty
    means the global policy event.
    """

    index: int = 0
    operand: str = ""
    selector: str = ""
    operator: str = "equals"
    values: tuple[str, ...] = ()
    negate: bool = False
    case_insensitive: bool = False
    external: bool = False
    event: str = ""
    raw: str = ""

    @classmethod
    def from_raw(cls, index: int, body: str) -> "LtmPolicyCondition":
        """Build a condition from its keyed-block body.

        ``body`` is the text inside the ``{...}`` braces of one
        numbered condition.  The parser walks token-by-token and
        recognises the documented condition vocabulary.  Unknown
        tokens are captured implicitly via ``raw`` for round-trip
        fidelity.
        """
        tokens = body.split()
        operand = ""
        selector = ""
        operator = "equals"
        values: list[str] = []
        negate = False
        case_insensitive = False
        external = False
        event = ""
        idx = 0
        # Token-by-token recognition: the first non-flag token is
        # the operand; subsequent tokens fill flags / operator /
        # values until we hit the ``values { ... }`` sub-block.
        _OPERATORS = {
            "equals",
            "contains",
            "starts-with",
            "ends-with",
            "matches",
            "less",
            "greater",
            "less-or-equal",
            "greater-or-equal",
            "not-equal",
        }
        _EVENTS = {
            "request",
            "response",
            "proxy-request",
            "proxy-response",
            "proxy-connect",
            "client-accepted",
            "server-connected",
            "ssl-client-hello",
            "ssl-client-server-hello-send",
            "ws-request",
            "ws-response",
        }
        while idx < len(tokens):
            tok = tokens[idx]
            if tok == "not":
                negate = True
                idx += 1
                continue
            if tok == "case-insensitive":
                case_insensitive = True
                idx += 1
                continue
            if tok == "external":
                external = True
                idx += 1
                continue
            if tok in _OPERATORS:
                operator = tok
                idx += 1
                continue
            if tok in _EVENTS:
                event = tok
                idx += 1
                continue
            if tok == "values" and idx + 1 < len(tokens) and tokens[idx + 1] == "{":
                # Consume tokens until the closing brace.
                idx += 2
                while idx < len(tokens) and tokens[idx] != "}":
                    values.append(tokens[idx])
                    idx += 1
                if idx < len(tokens):
                    idx += 1  # skip the ``}``
                continue
            if not operand:
                operand = tok
                idx += 1
                continue
            if not selector:
                selector = tok
                idx += 1
                continue
            idx += 1
        return cls(
            index=index,
            operand=operand,
            selector=selector,
            operator=operator,
            values=tuple(values),
            negate=negate,
            case_insensitive=case_insensitive,
            external=external,
            event=event,
            raw=body.strip(),
        )

    def __str__(self) -> str:
        if self.raw:
            return f"{self.index} {{ {self.raw} }}"
        parts: list[str] = []
        if self.event:
            parts.append(self.event)
        if self.operand:
            parts.append(self.operand)
        if self.selector:
            parts.append(self.selector)
        if self.negate:
            parts.append("not")
        if self.case_insensitive:
            parts.append("case-insensitive")
        if self.external:
            parts.append("external")
        if self.operator and self.operator != "equals":
            parts.append(self.operator)
        if self.values:
            parts.append("values { " + " ".join(self.values) + " }")
        body = " ".join(parts)
        return f"{self.index} {{ {body} }}" if body else f"{self.index} {{ }}"


@dataclass(frozen=True, slots=True)
class LtmPolicyAction:
    """One clause inside a policy rule's ``actions { ... }`` block.

    Policy actions describe what to do when a rule's conditions
    match — forward to a pool, redirect, insert a header, replace
    a URL component, etc.  The clause has a verb (``forward`` /
    ``redirect`` / ``insert`` / ``replace`` / ``log`` / ``persist``),
    an optional target/select form, and a structured set of
    parameter slots.  We model the common slots explicitly and
    keep the raw body for less common variants.

    ``index`` is the integer key BIG-IP assigns each clause.
    ``verb`` is the action verb.  ``target`` is the action target
    (typically ``request`` / ``response`` / ``proxy-request`` /
    ``proxy-response``).  ``pool`` / ``redirect_location`` /
    ``status`` carry the most common verbs' parameters.  ``raw``
    preserves the body for round-trip and for less-common verbs.
    """

    index: int = 0
    target: str = ""
    verb: str = ""
    select: bool = False
    pool: str = ""
    redirect_location: str = ""
    status: int | None = None
    raw: str = ""

    @classmethod
    def from_raw(cls, index: int, body: str) -> "LtmPolicyAction":
        tokens = body.split()
        target = ""
        verb = ""
        select = False
        pool = ""
        redirect_location = ""
        status: int | None = None
        idx = 0
        _TARGETS = {
            "request",
            "response",
            "proxy-request",
            "proxy-response",
            "proxy-connect",
            "client-accepted",
            "server-connected",
            "ssl-client-hello",
            "ws-request",
            "ws-response",
        }
        _VERBS = {
            "forward",
            "redirect",
            "insert",
            "remove",
            "replace",
            "log",
            "persist",
            "reset",
            "set-variable",
            "tcl",
            "enable",
            "disable",
            "drop",
            "decompress",
            "compress",
            "cache",
            "classify",
            "expire",
            "rewrite",
            "shutdown",
        }
        while idx < len(tokens):
            tok = tokens[idx]
            if tok in _TARGETS and not target:
                target = tok
                idx += 1
                continue
            if tok in _VERBS and not verb:
                verb = tok
                idx += 1
                continue
            if tok == "select":
                select = True
                idx += 1
                continue
            if tok == "pool" and idx + 1 < len(tokens):
                pool = tokens[idx + 1]
                idx += 2
                continue
            if tok == "location" and idx + 1 < len(tokens):
                redirect_location = tokens[idx + 1]
                idx += 2
                continue
            if tok == "status" and idx + 1 < len(tokens):
                try:
                    status = int(tokens[idx + 1])
                except ValueError:
                    status = None
                idx += 2
                continue
            idx += 1
        return cls(
            index=index,
            target=target,
            verb=verb,
            select=select,
            pool=pool,
            redirect_location=redirect_location,
            status=status,
            raw=body.strip(),
        )

    def references(self) -> tuple[tuple[str, str], ...]:
        """Yield ``(kind, full-path)`` pairs for every outbound
        reference this action carries.

        Today only the ``forward select pool <path>`` shape carries
        a path reference; future actions (``insert ... profile X``,
        ``rewrite uri /Common/policy_X``) can extend this set as
        the spec catalogue grows.
        """
        if self.verb == "forward" and self.pool:
            return (("ltm pool", self.pool),)
        return ()

    def __str__(self) -> str:
        if self.raw:
            return f"{self.index} {{ {self.raw} }}"
        parts: list[str] = []
        if self.target:
            parts.append(self.target)
        if self.verb:
            parts.append(self.verb)
        if self.select:
            parts.append("select")
        if self.pool:
            parts.append(f"pool {self.pool}")
        if self.redirect_location:
            parts.append(f"location {self.redirect_location}")
        if self.status is not None:
            parts.append(f"status {self.status}")
        body = " ".join(parts)
        return f"{self.index} {{ {body} }}" if body else f"{self.index} {{ }}"


__all__ = ["LtmPolicyAction", "LtmPolicyCondition"]
