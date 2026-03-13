"""Core iRule → F5 XC static translator.

Walks the IR produced by :func:`lower_to_ir` and builds an
:class:`XCTranslationResult` mapping iRule constructs to XC-native
equivalents (routes, service policies, header actions, origin pools).
"""

from __future__ import annotations

import re as _re
from dataclasses import dataclass
from dataclasses import replace as _dc_replace

from ..commands.registry import REGISTRY
from ..compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCommand,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprUnary,
    UnaryOp,
    expr_text,
)
from ..compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRProcedure,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ..compiler.lowering import lower_to_ir
from .mapping import (
    ADVISORY_EVENTS,
    COMMAND_XC_MAP,
    HEADER_SUBCOMMAND_MAP,
    TRANSLATABLE_EVENTS,
    UNTRANSLATABLE_EVENTS,
    UNTRANSLATABLE_PREFIXES,
)
from .xc_model import (
    TranslateStatus,
    TranslationItem,
    XCConstructKind,
    XCCookieMatch,
    XCDirectResponseAction,
    XCHeaderAction,
    XCHeaderMatch,
    XCHostMatch,
    XCMethodMatch,
    XCOriginPool,
    XCOriginPoolRef,
    XCPathMatch,
    XCQueryMatch,
    XCRedirectAction,
    XCRoute,
    XCServicePolicy,
    XCServicePolicyRule,
    XCTranslationResult,
    XCWafExclusionRule,
)

_MAX_DEPTH = 10

_HTTP_PATH_COMMANDS = ("HTTP::path", "HTTP::uri")
_EQUALITY_OPS = (BinOp.STR_EQ, BinOp.EQ, BinOp.STR_EQUALS)
_NEGATED_EQUALITY_OPS = (BinOp.STR_NE, BinOp.NE)


# Enclosing context (threaded through from enclosing if/switch)


@dataclass(frozen=True, slots=True)
class _EnclosingContext:
    """Match criteria accumulated from enclosing conditions."""

    path: XCPathMatch | None = None
    host: XCHostMatch | None = None
    method: XCMethodMatch | None = None
    ip_prefix_set: str | None = None
    header_matches: tuple[XCHeaderMatch, ...] = ()
    cookie_matches: tuple[XCCookieMatch, ...] = ()
    query_match: XCQueryMatch | None = None
    ip_prefix_list: tuple[str, ...] = ()


_EMPTY_ENCLOSING = _EnclosingContext()


# Low-level helpers


def _extract_command_name(text: str) -> str | None:
    """Extract the command name from a ``[CMD ...]`` substitution text."""
    text = text.strip()
    if text.startswith("[") and text.endswith("]"):
        text = text[1:-1].strip()
    parts = text.split()
    return parts[0] if parts else None


def _is_http_getter(text: str) -> str | None:
    """If *text* is a ``[HTTP::xxx]`` getter, return the command name.

    Also recognises ``[string tolower [HTTP::xxx]]`` and
    ``[string toupper [HTTP::xxx]]`` as case-insensitive wrappers.
    """
    cmd = _extract_command_name(text)
    if cmd and cmd.startswith("HTTP::"):
        return cmd
    # Handle [string tolower/toupper [HTTP::xxx]]
    if cmd == "string":
        inner = text.strip()
        if inner.startswith("[") and inner.endswith("]"):
            inner = inner[1:-1].strip()
        parts = inner.split(None, 2)
        if len(parts) >= 3 and parts[1] in ("tolower", "toupper"):
            return _is_http_getter(parts[2])
    return None


def _parse_command_parts(text: str) -> list[str]:
    """Split ``[cmd arg1 "arg2"]`` into parts, handling quoted strings."""
    text = text.strip()
    if text.startswith("[") and text.endswith("]"):
        text = text[1:-1].strip()
    parts: list[str] = []
    i = 0
    while i < len(text):
        while i < len(text) and text[i] in (" ", "\t"):
            i += 1
        if i >= len(text):
            break
        if text[i] == '"':
            i += 1
            start = i
            while i < len(text) and text[i] != '"':
                if text[i] == "\\":
                    i += 1
                i += 1
            parts.append(text[start:i])
            if i < len(text):
                i += 1
        elif text[i] == "[":
            depth = 0
            start = i
            while i < len(text):
                if text[i] == "[":
                    depth += 1
                elif text[i] == "]":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                i += 1
            parts.append(text[start:i])
        else:
            start = i
            while i < len(text) and text[i] not in (" ", "\t"):
                i += 1
            parts.append(text[start:i])
    return parts


def _switch_subject_kind(subject: str) -> str | None:
    """Classify what an IRSwitch subject is switching on.

    Returns ``"path"``, ``"host"``, ``"uri"``, ``"method"``, ``"header"``
    or ``None`` if unrecognised.
    """
    cmd = _is_http_getter(subject)
    if cmd in _HTTP_PATH_COMMANDS:
        return "path"
    if cmd == "HTTP::host":
        return "host"
    if cmd == "HTTP::method":
        return "method"
    if cmd == "HTTP::header":
        return "header"
    return None


def _glob_to_prefix(pattern: str) -> XCPathMatch | None:
    """Convert a glob pattern like ``/api/*`` to an XC path match."""
    if pattern.endswith("*") and "*" not in pattern[:-1] and "?" not in pattern:
        prefix = pattern[:-1]
        if prefix:
            return XCPathMatch(match_type="prefix", value=prefix)
    return XCPathMatch(match_type="exact", value=pattern)


def _glob_to_regex(pattern: str) -> str:
    """Convert a glob pattern to a regex string."""
    parts: list[str] = []
    for ch in pattern:
        if ch == "*":
            parts.append(".*")
        elif ch == "?":
            parts.append(".")
        else:
            parts.append(_re.escape(ch))
    return "^" + "".join(parts) + "$"


def _expr_command_text(node: ExprNode) -> str | None:
    """Extract raw text from an ExprCommand node."""
    if isinstance(node, ExprCommand):
        return node.text
    return None


def _expr_string_value(node: ExprNode) -> str | None:
    """Extract the string value from a literal or string node."""
    if isinstance(node, ExprString):
        text = node.text
        # ExprString.text may include surrounding quotes from the lexer
        if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
            return text[1:-1]
        return text
    if isinstance(node, ExprRaw):
        text = node.text.strip()
        if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
            return text[1:-1]
    return None


# Negation / decomposition helpers


def _unwrap_negation(node: ExprNode) -> tuple[ExprNode, bool]:
    """Unwrap ``!`` or ``not`` prefix, returning ``(inner, is_negated)``."""
    if isinstance(node, ExprUnary) and node.op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
        return node.operand, True
    return node, False


def _decompose_and_condition(node: ExprNode) -> list[ExprNode]:
    """Flatten nested ``&&`` / ``and`` conditions into sub-conditions."""
    if isinstance(node, ExprBinary) and node.op in (BinOp.AND, BinOp.WORD_AND):
        return _decompose_and_condition(node.left) + _decompose_and_condition(node.right)
    return [node]


def _decompose_or_condition(node: ExprNode) -> list[ExprNode]:
    """Flatten nested ``||`` / ``or`` conditions into branches."""
    if isinstance(node, ExprBinary) and node.op in (BinOp.OR, BinOp.WORD_OR):
        return _decompose_or_condition(node.left) + _decompose_or_condition(node.right)
    return [node]


# Condition extraction helpers


def _condition_to_method_match(node: ExprNode) -> XCMethodMatch | None:
    """Try to extract an HTTP method match from a condition expression."""
    inner, negated = _unwrap_negation(node)
    match inner:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::method" and str_val:
                return XCMethodMatch(methods=(str_val.upper(),), invert=negated)
        case ExprBinary(op=op, left=left, right=right) if op in _NEGATED_EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::method" and str_val:
                return XCMethodMatch(methods=(str_val.upper(),), invert=not negated)
    return None


def _condition_to_path_match(node: ExprNode) -> XCPathMatch | None:
    """Try to extract a path match from a condition expression."""
    inner, negated = _unwrap_negation(node)
    match inner:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(match_type="exact", value=str_val, invert=negated)
        case ExprBinary(op=BinOp.STARTS_WITH, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(match_type="prefix", value=str_val, invert=negated)
        case ExprBinary(op=BinOp.ENDS_WITH, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(match_type="suffix", value=str_val, invert=negated)
        case ExprBinary(op=BinOp.CONTAINS, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(match_type="regex", value=_re.escape(str_val), invert=negated)
        case ExprBinary(op=BinOp.MATCHES_REGEX, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(match_type="regex", value=str_val, invert=negated)
        case ExprBinary(op=BinOp.MATCHES_GLOB, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) in _HTTP_PATH_COMMANDS and str_val:
                return XCPathMatch(
                    match_type="regex", value=_glob_to_regex(str_val), invert=negated
                )
    return None


def _condition_to_host_match(node: ExprNode) -> XCHostMatch | None:
    """Try to extract a host match from a condition expression."""
    match node:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::host" and str_val:
                return XCHostMatch(match_type="exact", value=str_val)
        case ExprBinary(op=BinOp.MATCHES_REGEX, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::host" and str_val:
                return XCHostMatch(match_type="regex", value=str_val)
        case ExprBinary(op=BinOp.CONTAINS, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::host" and str_val:
                return XCHostMatch(match_type="regex", value=_re.escape(str_val))
    return None


def _condition_to_header_match(node: ExprNode) -> XCHeaderMatch | None:
    """Extract header match from a condition expression.

    Recognises:
    - ``[HTTP::header value "X-Foo"] eq "bar"``
    - ``[HTTP::header "X-Foo"] eq "bar"``
    - ``[HTTP::header exists "X-Foo"]`` (boolean context)
    - ``[HTTP::header value "X-Foo"] contains "bar"``
    """
    inner, negated = _unwrap_negation(node)
    match inner:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            hdr = _parse_header_getter(left) or _parse_header_getter(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if hdr and str_val:
                return XCHeaderMatch(name=hdr, match_type="exact", value=str_val, invert=negated)
        case ExprBinary(op=op, left=left, right=right) if op in _NEGATED_EQUALITY_OPS:
            hdr = _parse_header_getter(left) or _parse_header_getter(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if hdr and str_val:
                return XCHeaderMatch(
                    name=hdr, match_type="exact", value=str_val, invert=not negated
                )
        case ExprBinary(op=BinOp.CONTAINS, left=left, right=right):
            hdr = _parse_header_getter(left)
            str_val = _expr_string_value(right)
            if hdr and str_val:
                return XCHeaderMatch(
                    name=hdr, match_type="regex", value=_re.escape(str_val), invert=negated
                )
        case ExprBinary(op=BinOp.MATCHES_REGEX, left=left, right=right):
            hdr = _parse_header_getter(left)
            str_val = _expr_string_value(right)
            if hdr and str_val:
                return XCHeaderMatch(name=hdr, match_type="regex", value=str_val, invert=negated)
        case ExprCommand():
            exists_name = _parse_header_exists(inner)
            if exists_name:
                return XCHeaderMatch(name=exists_name, match_type="presence", invert=negated)
    return None


def _parse_header_getter(node: ExprNode) -> str | None:
    """Parse ``[HTTP::header value "name"]`` or ``[HTTP::header "name"]`` → header name."""
    if not isinstance(node, ExprCommand):
        return None
    parts = _parse_command_parts(node.text)
    if len(parts) < 2 or parts[0] != "HTTP::header":
        return None
    if parts[1] == "value" and len(parts) >= 3:
        return parts[2]
    if parts[1] not in ("insert", "replace", "remove", "set", "exists", "values"):
        return parts[1]
    return None


def _parse_header_exists(node: ExprCommand) -> str | None:
    """Parse ``[HTTP::header exists "name"]`` → header name."""
    parts = _parse_command_parts(node.text)
    if len(parts) >= 3 and parts[0] == "HTTP::header" and parts[1] == "exists":
        return parts[2]
    return None


def _condition_to_cookie_match(node: ExprNode) -> XCCookieMatch | None:
    """Extract cookie match from a condition expression.

    Recognises:
    - ``[HTTP::cookie "name"] eq "value"``
    - ``[HTTP::cookie exists "name"]`` (boolean context)
    """
    inner, negated = _unwrap_negation(node)
    match inner:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cookie = _parse_cookie_getter(left) or _parse_cookie_getter(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cookie and str_val:
                return XCCookieMatch(name=cookie, match_type="exact", value=str_val, invert=negated)
        case ExprBinary(op=BinOp.CONTAINS, left=left, right=right):
            cookie = _parse_cookie_getter(left)
            str_val = _expr_string_value(right)
            if cookie and str_val:
                return XCCookieMatch(
                    name=cookie, match_type="regex", value=_re.escape(str_val), invert=negated
                )
        case ExprBinary(op=BinOp.MATCHES_REGEX, left=left, right=right):
            cookie = _parse_cookie_getter(left)
            str_val = _expr_string_value(right)
            if cookie and str_val:
                return XCCookieMatch(name=cookie, match_type="regex", value=str_val, invert=negated)
        case ExprCommand():
            exists_name = _parse_cookie_exists(inner)
            if exists_name:
                return XCCookieMatch(name=exists_name, match_type="presence", invert=negated)
    return None


def _parse_cookie_getter(node: ExprNode) -> str | None:
    """Parse ``[HTTP::cookie "name"]`` or ``[HTTP::cookie value "name"]`` → cookie name."""
    if not isinstance(node, ExprCommand):
        return None
    parts = _parse_command_parts(node.text)
    if len(parts) < 2 or parts[0] != "HTTP::cookie":
        return None
    if parts[1] == "value" and len(parts) >= 3:
        return parts[2]
    if parts[1] not in ("insert", "remove", "exists", "names"):
        return parts[1]
    return None


def _parse_cookie_exists(node: ExprCommand) -> str | None:
    """Parse ``[HTTP::cookie exists "name"]`` → cookie name."""
    parts = _parse_command_parts(node.text)
    if len(parts) >= 3 and parts[0] == "HTTP::cookie" and parts[1] == "exists":
        return parts[2]
    return None


def _condition_to_query_match(node: ExprNode) -> XCQueryMatch | None:
    """Extract query string match from a condition expression.

    Recognises:
    - ``[HTTP::query] eq "exact"``
    - ``[HTTP::query] contains "param=val"``
    - ``[HTTP::query] matches_regex "pattern"``
    """
    match node:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::query" and str_val:
                return XCQueryMatch(match_type="exact", value=str_val)
        case ExprBinary(op=BinOp.CONTAINS, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::query" and str_val:
                return XCQueryMatch(match_type="regex", value=_re.escape(str_val))
        case ExprBinary(op=BinOp.MATCHES_REGEX, left=left, right=right):
            cmd_text = _expr_command_text(left)
            str_val = _expr_string_value(right)
            if cmd_text and _is_http_getter(cmd_text) == "HTTP::query" and str_val:
                return XCQueryMatch(match_type="regex", value=str_val)
    return None


def _condition_to_direct_ip(node: ExprNode) -> str | None:
    """Extract a direct IP address match from ``[IP::client_addr] eq "x.x.x.x"``."""
    match node:
        case ExprBinary(op=op, left=left, right=right) if op in _EQUALITY_OPS:
            cmd_text = _expr_command_text(left) or _expr_command_text(right)
            str_val = _expr_string_value(right) or _expr_string_value(left)
            if cmd_text:
                cmd = _extract_command_name(cmd_text)
                if cmd == "IP::client_addr" and str_val:
                    return str_val
    return None


def _condition_to_ip_prefix_set(node: ExprNode) -> str | None:
    """Extract an IP prefix set (data-group) reference from a condition.

    Recognises ``[class match [IP::client_addr] equals DataGroupName]``.
    """
    if isinstance(node, ExprCommand):
        text = node.text.strip()
        if text.startswith("[") and text.endswith("]"):
            text = text[1:-1].strip()
        parts = text.split()
        if (
            len(parts) >= 5
            and parts[0] == "class"
            and parts[1] == "match"
            and "IP::client_addr" in text
        ):
            return parts[-1]
    return None


def _condition_to_class_match_method(node: ExprNode) -> XCMethodMatch | None:
    """Extract method match from ``[class match [HTTP::method] equals MethodGroup]``."""
    if isinstance(node, ExprCommand):
        parts = _parse_command_parts(node.text)
        if (
            len(parts) >= 5
            and parts[0] == "class"
            and parts[1] == "match"
            and "HTTP::method" in node.text
        ):
            # Data-group name is last part — cannot resolve statically
            # Return a placeholder match indicating method matching
            return XCMethodMatch(methods=())
    return None


def _extract_all_matches(
    subconditions: list[ExprNode],
) -> _EnclosingContext:
    """Extract all recognisable match criteria from a list of AND sub-conditions."""
    path: XCPathMatch | None = None
    host: XCHostMatch | None = None
    method: XCMethodMatch | None = None
    ip_prefix_set: str | None = None
    headers: list[XCHeaderMatch] = []
    cookies: list[XCCookieMatch] = []
    query: XCQueryMatch | None = None
    ips: list[str] = []

    for sub in subconditions:
        if not path:
            path = _condition_to_path_match(sub)
            if path:
                continue
        if not host:
            host = _condition_to_host_match(sub)
            if host:
                continue
        if not method:
            method = _condition_to_method_match(sub)
            if method:
                continue
        if not method:
            method = _condition_to_class_match_method(sub)
            if method:
                continue
        if not ip_prefix_set:
            ip_prefix_set = _condition_to_ip_prefix_set(sub)
            if ip_prefix_set:
                continue
        hdr = _condition_to_header_match(sub)
        if hdr:
            headers.append(hdr)
            continue
        cookie = _condition_to_cookie_match(sub)
        if cookie:
            cookies.append(cookie)
            continue
        if not query:
            query = _condition_to_query_match(sub)
            if query:
                continue
        ip = _condition_to_direct_ip(sub)
        if ip:
            ips.append(ip)

    return _EnclosingContext(
        path=path,
        host=host,
        method=method,
        ip_prefix_set=ip_prefix_set,
        header_matches=tuple(headers),
        cookie_matches=tuple(cookies),
        query_match=query,
        ip_prefix_list=tuple(ips),
    )


# Context for translation state


class _TranslationContext:
    """Mutable accumulator for translation results."""

    def __init__(self) -> None:
        self.routes: list[XCRoute] = []
        self.policy_rules: list[XCServicePolicyRule] = []
        self.origin_pools: dict[str, XCOriginPool] = {}  # name → pool
        self.header_actions: list[XCHeaderAction] = []
        self.waf_exclusion_rules: list[XCWafExclusionRule] = []
        self.items: list[TranslationItem] = []
        self._rule_counter = 0
        self._current_event: str = ""

    def next_rule_name(self) -> str:
        self._rule_counter += 1
        return f"rule-{self._rule_counter}"

    def add_origin_pool(self, name: str, source_range=None):
        """Register an origin pool by name (deduplicated)."""
        if name not in self.origin_pools:
            self.origin_pools[name] = XCOriginPool(name=name)
        self.items.append(
            TranslationItem(
                status=TranslateStatus.TRANSLATED,
                kind=XCConstructKind.ORIGIN_POOL,
                irule_command=f"pool {name}",
                irule_range=source_range,
                xc_description=f"Origin pool: {name}",
                diagnostic_code="XC100",
            )
        )


# IR walking


def _merge_enclosing(parent: _EnclosingContext, child: _EnclosingContext) -> _EnclosingContext:
    """Merge child matches into parent, child wins on non-None fields."""
    return _EnclosingContext(
        path=child.path or parent.path,
        host=child.host or parent.host,
        method=child.method or parent.method,
        ip_prefix_set=child.ip_prefix_set or parent.ip_prefix_set,
        header_matches=child.header_matches or parent.header_matches,
        cookie_matches=child.cookie_matches or parent.cookie_matches,
        query_match=child.query_match or parent.query_match,
        ip_prefix_list=child.ip_prefix_list or parent.ip_prefix_list,
    )


def _enclosing_has_matches(enc: _EnclosingContext) -> bool:
    """Return True if any match criterion is set."""
    return bool(
        enc.path
        or enc.host
        or enc.method
        or enc.ip_prefix_set
        or enc.header_matches
        or enc.cookie_matches
        or enc.query_match
        or enc.ip_prefix_list
    )


def _walk_statement(
    stmt: IRStatement,
    ctx: _TranslationContext,
    depth: int,
    *,
    enclosing: _EnclosingContext = _EMPTY_ENCLOSING,
) -> None:
    """Process a single IR statement, extracting XC constructs into *ctx*."""
    if depth > _MAX_DEPTH:
        return

    match stmt:
        # switch on HTTP::path / HTTP::host / HTTP::uri
        case IRSwitch(subject=subject, arms=arms, default_body=default_body, range=rng):
            kind = _switch_subject_kind(subject)
            if kind == "path":
                for arm in arms:
                    if arm.fallthrough or arm.body is None:
                        continue
                    path_match = _glob_to_prefix(arm.pattern)
                    _walk_script(
                        arm.body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, path=path_match),
                    )
                if default_body is not None:
                    _walk_script(
                        default_body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, path=None),
                    )
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.ROUTE,
                        irule_command=f"switch {subject}",
                        irule_range=rng,
                        xc_description="XC L7 routes with path matching",
                        diagnostic_code="XC101",
                    )
                )
            elif kind == "host":
                for arm in arms:
                    if arm.fallthrough or arm.body is None:
                        continue
                    host_match = XCHostMatch(match_type="exact", value=arm.pattern)
                    _walk_script(
                        arm.body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, host=host_match),
                    )
                if default_body is not None:
                    _walk_script(
                        default_body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, host=None),
                    )
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.ROUTE,
                        irule_command=f"switch {subject}",
                        irule_range=rng,
                        xc_description="XC L7 routes with host matching",
                        diagnostic_code="XC101",
                    )
                )
            elif kind == "method":
                for arm in arms:
                    if arm.fallthrough or arm.body is None:
                        continue
                    method_match = XCMethodMatch(methods=(arm.pattern.upper(),))
                    _walk_script(
                        arm.body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, method=method_match),
                    )
                if default_body is not None:
                    _walk_script(
                        default_body,
                        ctx,
                        depth + 1,
                        enclosing=_dc_replace(enclosing, method=None),
                    )
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.SERVICE_POLICY_RULE,
                        irule_command=f"switch {subject}",
                        irule_range=rng,
                        xc_description="XC service policy rules with method matching",
                        diagnostic_code="XC102",
                    )
                )
            else:
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.PARTIAL,
                        kind=XCConstructKind.ROUTE,
                        irule_command=f"switch {subject}",
                        irule_range=rng,
                        xc_description="Switch on dynamic value — needs manual XC config",
                        note="Cannot statically determine match criteria",
                        diagnostic_code="XC200",
                    )
                )
                for arm in arms:
                    if arm.body:
                        _walk_script(arm.body, ctx, depth + 1)
                if default_body is not None:
                    _walk_script(default_body, ctx, depth + 1)

        # if conditions
        case IRIf(clauses=clauses, else_body=else_body, range=rng):
            any_matched = False
            for clause in clauses:
                # Check for OR at the top level first
                or_branches = _decompose_or_condition(clause.condition)
                if len(or_branches) > 1:
                    # Each OR branch produces a separate walk of the body
                    for branch in or_branches:
                        and_subs = _decompose_and_condition(branch)
                        matches = _extract_all_matches(and_subs)
                        merged = _merge_enclosing(enclosing, matches)
                        if _enclosing_has_matches(matches):
                            any_matched = True
                        _walk_script(clause.body, ctx, depth + 1, enclosing=merged)
                else:
                    and_subs = _decompose_and_condition(clause.condition)
                    matches = _extract_all_matches(and_subs)
                    merged = _merge_enclosing(enclosing, matches)
                    if _enclosing_has_matches(matches):
                        any_matched = True
                    _walk_script(clause.body, ctx, depth + 1, enclosing=merged)

            if else_body is not None:
                _walk_script(else_body, ctx, depth + 1)
            if any_matched:
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.SERVICE_POLICY_RULE,
                        irule_command="if condition",
                        irule_range=rng,
                        xc_description="XC service policy rule with match criteria",
                        diagnostic_code="XC102",
                    )
                )
            else:
                cond_text = expr_text(clauses[0].condition) if clauses else "?"
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.PARTIAL,
                        kind=XCConstructKind.SERVICE_POLICY_RULE,
                        irule_command=f"if {{{cond_text}}}",
                        irule_range=rng,
                        xc_description="Conditional logic — review manually for XC match criteria",
                        note="Complex condition could not be automatically mapped",
                        diagnostic_code="XC203",
                    )
                )

        # pool command
        case IRCall(command="pool", args=args, range=rng) if args:
            pool_name = args[0]
            ctx.add_origin_pool(pool_name, source_range=rng)
            ctx.routes.append(
                XCRoute(
                    path_match=enclosing.path,
                    host_match=enclosing.host,
                    method_match=enclosing.method,
                    header_matches=enclosing.header_matches,
                    query_match=enclosing.query_match,
                    cookie_matches=enclosing.cookie_matches,
                    origin_pool=XCOriginPoolRef(name=pool_name, source_range=rng),
                    source_range=rng,
                )
            )

        # HTTP::redirect
        case IRCall(command="HTTP::redirect", args=args, range=rng) if args:
            url = args[0]
            code = 302
            if len(args) >= 2:
                try:
                    code = int(args[1])
                except (ValueError, IndexError):
                    pass
            ctx.routes.append(
                XCRoute(
                    path_match=enclosing.path,
                    host_match=enclosing.host,
                    method_match=enclosing.method,
                    header_matches=enclosing.header_matches,
                    query_match=enclosing.query_match,
                    cookie_matches=enclosing.cookie_matches,
                    redirect=XCRedirectAction(url=url, response_code=code, source_range=rng),
                    source_range=rng,
                )
            )
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.TRANSLATED,
                    kind=XCConstructKind.ROUTE,
                    irule_command=f"HTTP::redirect {url}",
                    irule_range=rng,
                    xc_description="XC redirect route",
                    diagnostic_code="XC101",
                )
            )

        # HTTP::respond
        case IRCall(command="HTTP::respond", args=args, range=rng) if args:
            status_code = 200
            try:
                status_code = int(args[0])
            except (ValueError, IndexError):
                pass
            body = ""
            for i, arg in enumerate(args):
                if arg == "content" and i + 1 < len(args):
                    body = args[i + 1]
                    break
            if status_code in (403, 401, 429):
                ctx.policy_rules.append(
                    XCServicePolicyRule(
                        name=ctx.next_rule_name(),
                        action="deny",
                        path_match=enclosing.path,
                        host_match=enclosing.host,
                        method_match=enclosing.method,
                        header_matches=enclosing.header_matches,
                        query_match=enclosing.query_match,
                        cookie_matches=enclosing.cookie_matches,
                        ip_prefix_list=enclosing.ip_prefix_list,
                        description=f"Deny with {status_code} (from HTTP::respond)",
                        source_range=rng,
                    )
                )
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.SERVICE_POLICY_RULE,
                        irule_command=f"HTTP::respond {status_code}",
                        irule_range=rng,
                        xc_description=f"XC service policy deny rule ({status_code})",
                        diagnostic_code="XC102",
                    )
                )
            else:
                ctx.routes.append(
                    XCRoute(
                        path_match=enclosing.path,
                        host_match=enclosing.host,
                        method_match=enclosing.method,
                        header_matches=enclosing.header_matches,
                        query_match=enclosing.query_match,
                        cookie_matches=enclosing.cookie_matches,
                        direct_response=XCDirectResponseAction(
                            status_code=status_code, body=body, source_range=rng
                        ),
                        source_range=rng,
                    )
                )
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.ROUTE,
                        irule_command=f"HTTP::respond {status_code}",
                        irule_range=rng,
                        xc_description=f"XC direct response route ({status_code})",
                        diagnostic_code="XC101",
                    )
                )

        # HTTP::header insert/replace/remove
        case IRCall(command="HTTP::header", args=args, range=rng) if args:
            subcommand = args[0].lower() if args else ""
            operation = HEADER_SUBCOMMAND_MAP.get(subcommand)
            if operation and len(args) >= 2:
                header_name = args[1]
                header_value = args[2] if len(args) >= 3 else ""
                target = "response" if ctx._current_event == "HTTP_RESPONSE" else "request"
                action = XCHeaderAction(
                    name=header_name,
                    value=header_value,
                    operation=operation,
                    target=target,
                    source_range=rng,
                )
                ctx.header_actions.append(action)
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=XCConstructKind.HEADER_ACTION,
                        irule_command=f"HTTP::header {subcommand} {header_name}",
                        irule_range=rng,
                        xc_description=f"XC {target} header {operation}: {header_name}",
                        diagnostic_code="XC103",
                    )
                )

        # class match → service policy
        case IRCall(command="class", args=args, range=rng) if args and args[0] == "match":
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.TRANSLATED,
                    kind=XCConstructKind.SERVICE_POLICY_RULE,
                    irule_command=f"class match {' '.join(args[1:3])}",
                    irule_range=rng,
                    xc_description="XC service policy rule (data-group matching)",
                    note="Each data-group entry may need a separate XC rule",
                    diagnostic_code="XC105",
                )
            )

        # ASM::disable → WAF exclusion rule
        case IRCall(command="ASM::disable", range=rng):
            ctx.waf_exclusion_rules.append(
                XCWafExclusionRule(
                    name=ctx.next_rule_name(),
                    path_match=enclosing.path,
                    ip_prefix_set=enclosing.ip_prefix_set,
                    description="Disable WAF (from ASM::disable)",
                    source_range=rng,
                )
            )
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.TRANSLATED,
                    kind=XCConstructKind.WAF_EXCLUSION,
                    irule_command="ASM::disable",
                    irule_range=rng,
                    xc_description="XC WAF exclusion rule (disable App Firewall)",
                    diagnostic_code="XC106",
                )
            )

        # ASM::enable — no-op (WAF enabled by default in XC)
        case IRCall(command="ASM::enable", range=rng):
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.TRANSLATED,
                    kind=XCConstructKind.NOTE,
                    irule_command="ASM::enable",
                    irule_range=rng,
                    xc_description="No action needed — XC App Firewall is enabled by default",
                    diagnostic_code="XC107",
                )
            )

        # Barrier commands (eval, uplevel, etc.)
        case IRBarrier(command=command, range=rng):
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.UNTRANSLATABLE,
                    kind=XCConstructKind.NOTE,
                    irule_command=command,
                    irule_range=rng,
                    xc_description="Dynamic construct — cannot be statically translated",
                    note=f"'{command}' creates dynamic behaviour with no XC equivalent. "
                    "Consider App Stack for this logic.",
                    diagnostic_code="XC300",
                )
            )

        # return
        case IRReturn(range=rng):
            pass  # return itself doesn't need translation

        # loops
        case IRFor(range=rng) | IRWhile(range=rng) | IRForeach(range=rng):
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.UNTRANSLATABLE,
                    kind=XCConstructKind.NOTE,
                    irule_command="loop",
                    irule_range=rng,
                    xc_description="Loop construct — no XC equivalent",
                    note="XC policies are declarative, not procedural. "
                    "Consider App Stack for iterative logic.",
                    diagnostic_code="XC300",
                )
            )
            body = getattr(stmt, "body", None)
            if body is not None:
                _walk_script(body, ctx, depth + 1)

        # catch / try
        case IRCatch(body=body, range=rng):
            _walk_script(body, ctx, depth + 1)

        case IRTry(body=body, range=rng):
            _walk_script(body, ctx, depth + 1)

        # Other IRCall commands
        case IRCall(command=command, range=rng):
            if REGISTRY.is_xc_never_translatable(command):
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.UNTRANSLATABLE,
                        kind=XCConstructKind.NOTE,
                        irule_command=command,
                        irule_range=rng,
                        xc_description=f"'{command}' has no XC equivalent",
                        note="Consider App Stack for this logic.",
                        diagnostic_code="XC300",
                    )
                )
            elif any(command.startswith(p) for p in UNTRANSLATABLE_PREFIXES):
                if not REGISTRY.is_xc_translatable_override(command):
                    ctx.items.append(
                        TranslationItem(
                            status=TranslateStatus.UNTRANSLATABLE,
                            kind=XCConstructKind.NOTE,
                            irule_command=command,
                            irule_range=rng,
                            xc_description=f"'{command}' has no XC equivalent",
                            note="L4/protocol-specific command. Consider App Stack.",
                            diagnostic_code="XC301",
                        )
                    )
            elif command in COMMAND_XC_MAP:
                mapping = COMMAND_XC_MAP[command]
                ctx.items.append(
                    TranslationItem(
                        status=TranslateStatus.TRANSLATED,
                        kind=mapping.kind,
                        irule_command=command,
                        irule_range=rng,
                        xc_description=mapping.xc_description,
                        note=mapping.note,
                        diagnostic_code="XC100",
                    )
                )

        # Assignments — ignore for translation
        case IRAssignConst() | IRAssignExpr() | IRAssignValue():
            pass


def _walk_script(
    script: IRScript,
    ctx: _TranslationContext,
    depth: int,
    *,
    enclosing: _EnclosingContext = _EMPTY_ENCLOSING,
) -> None:
    """Walk all statements in an IRScript."""
    for stmt in script.statements:
        _walk_statement(stmt, ctx, depth, enclosing=enclosing)


# Public API


def translate_irule(source: str) -> XCTranslationResult:
    """Translate an iRule to F5 XC configuration.

    Analyses the iRule source, walks the IR, and returns a
    :class:`XCTranslationResult` with routes, service policies,
    origin pools, header actions, and a coverage report.
    """
    ir_module = lower_to_ir(source)

    # Separate event handlers from regular procedures
    event_procs: dict[str, IRProcedure] = {}
    regular_procs: dict[str, IRProcedure] = {}
    for key, proc in ir_module.procedures.items():
        if key.startswith("::when::"):
            event_procs[key] = proc
        else:
            regular_procs[key] = proc

    ctx = _TranslationContext()

    # Walk each event handler
    for qualified_name, proc in event_procs.items():
        event_name = proc.name
        ctx._current_event = event_name

        if event_name in TRANSLATABLE_EVENTS:
            _walk_script(proc.body, ctx, depth=0)
        elif event_name in UNTRANSLATABLE_EVENTS:
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.UNTRANSLATABLE,
                    kind=XCConstructKind.NOTE,
                    irule_command=f"when {event_name}",
                    irule_range=proc.range,
                    xc_description=UNTRANSLATABLE_EVENTS[event_name],
                    note="This entire event handler has no XC equivalent.",
                    diagnostic_code="XC201",
                )
            )
        elif event_name in ADVISORY_EVENTS:
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.ADVISORY,
                    kind=XCConstructKind.NOTE,
                    irule_command=f"when {event_name}",
                    irule_range=proc.range,
                    xc_description=ADVISORY_EVENTS[event_name],
                    note="This event maps to a separate XC feature.",
                    diagnostic_code="XC250",
                )
            )
        else:
            # Unknown event — flag as untranslatable
            ctx.items.append(
                TranslationItem(
                    status=TranslateStatus.UNTRANSLATABLE,
                    kind=XCConstructKind.NOTE,
                    irule_command=f"when {event_name}",
                    irule_range=proc.range,
                    xc_description=f"Event '{event_name}' has no known XC equivalent.",
                    diagnostic_code="XC201",
                )
            )

    # Also walk regular procs that may be called from event handlers
    for proc in regular_procs.values():
        ctx._current_event = ""
        _walk_script(proc.body, ctx, depth=0)

    # Build service policy from collected rules
    service_policies: tuple[XCServicePolicy, ...] = ()
    if ctx.policy_rules:
        service_policies = (
            XCServicePolicy(
                name="translated-policy",
                rules=tuple(ctx.policy_rules),
            ),
        )

    # Calculate coverage
    total = len(ctx.items)
    if total == 0:
        coverage = 100.0
    else:
        translated = sum(
            1
            for i in ctx.items
            if i.status in (TranslateStatus.TRANSLATED, TranslateStatus.ADVISORY)
        )
        partial = sum(1 for i in ctx.items if i.status == TranslateStatus.PARTIAL)
        coverage = ((translated + partial * 0.5) / total) * 100.0

    return XCTranslationResult(
        routes=tuple(ctx.routes),
        service_policies=service_policies,
        origin_pools=tuple(ctx.origin_pools.values()),
        header_actions=tuple(ctx.header_actions),
        waf_exclusion_rules=tuple(ctx.waf_exclusion_rules),
        items=tuple(ctx.items),
        coverage_pct=round(coverage, 1),
    )
