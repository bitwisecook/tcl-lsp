"""Semantic token provider for Tcl source.

Encodes tokens in the LSP 5-integer delta format:
    [deltaLine, deltaStartChar, length, tokenType, tokenModifiers]

The provider recurses into body arguments (proc bodies, if/while/for
bodies, namespace eval bodies, etc.) so that tokens inside braces get
proper keyword/variable/function highlighting instead of being emitted
as one large string token.
"""

from __future__ import annotations

import re

from core.analysis.semantic_model import AnalysisResult
from core.bigip.iapp_extract import find_embedded_iapp_sections
from core.bigip.irules_refs import extract_irules_object_references
from core.bigip.rule_extract import find_embedded_rules
from core.commands.registry.command_registry import REGISTRY
from core.commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    SubcommandSig,
    arg_indices_for_role,
    options_with_value,
    regexp_pattern_index,
    skip_options,
)
from core.common.dialect import active_dialect
from core.common.ranges import position_from_relative
from core.common.source_map import SourceMap
from core.parsing.expr_lexer import ExprTokenType, tokenise_expr
from core.parsing.known_commands import known_command_names
from core.parsing.lexer import TclLexer
from core.parsing.recovery import compute_virtual_insertions
from core.parsing.token_positions import token_content_base, token_content_shift
from core.parsing.tokens import SourcePosition, Token, TokenType

# Short names: t = Token or token-position tuple.

# Token type legend (indices into this list are used in the encoding)
SEMANTIC_TOKEN_TYPES = [
    "keyword",  # 0
    "function",  # 1
    "variable",  # 2
    "string",  # 3
    "comment",  # 4
    "number",  # 5
    "operator",  # 6
    "parameter",  # 7
    "namespace",  # 8
    "regexp",  # 9
    "event",  # 10
    "decorator",  # 11
    "escape",  # 12 – backslash escape sequences inside strings
    "object",  # 13 – BIG-IP object names
    "fqdn",  # 14 – BIG-IP hostnames/FQDNs
    "ipAddress",  # 15 – BIG-IP IP literals
    "port",  # 16 – BIG-IP port literals
    "routeDomain",  # 17 – BIG-IP route-domain IDs (%N)
    "partition",  # 18 – BIG-IP partitions (/Common, /foo, ...)
    "username",  # 19 – BIG-IP user names
    "encrypted",  # 20 – encrypted/hashed secret values
    "pool",  # 21 – BIG-IP pool names
    "monitor",  # 22 – BIG-IP monitor names
    "profile",  # 23 – BIG-IP profile names
    "vlan",  # 24 – BIG-IP VLAN names
    "interface",  # 25 – BIG-IP interface names (e.g. 1.1, mgmt)
    # ARE regex sub-token types
    "regexpGroup",  # 26 – groups & flags: ( ) (?:) (?imsx)
    "regexpCharClass",  # 27 – char classes: [...] \d \w \s . and negated
    "regexpQuantifier",  # 28 – quantifiers: * + ? {n,m} and lazy variants
    "regexpAnchor",  # 29 – anchors: ^ $ \A \Z \b \B \m \M \y \Y
    "regexpEscape",  # 30 – escape sequences: \n \t \xHH \uHHHH \\meta
    "regexpBackref",  # 31 – backreferences: \0–\9
    "regexpAlternation",  # 32 – alternation pipe: |
    # binary scan/format sub-token types
    "binarySpec",  # 33 – format specifier letter (a A b B c s i w …)
    "binaryCount",  # 34 – repeat count (numeric)
    "binaryFlag",  # 35 – modifier: u s * (signed/unsigned/consume-all)
    # format/scan (sprintf) sub-token types
    "formatPercent",  # 36 – the % introducer and $ position separator
    "formatSpec",  # 37 – conversion type letter (d s f x …)
    "formatFlag",  # 38 – flags (- + 0 # space) and length modifier (h l)
    "formatWidth",  # 39 – numeric width/precision values
    # clock format sub-token types
    "clockPercent",  # 40 – the % introducer
    "clockSpec",  # 41 – specifier letter (Y m d H M S …)
    "clockModifier",  # 42 – locale modifier (E O)
]

SEMANTIC_TOKEN_MODIFIERS = [
    "declaration",  # 0
    "definition",  # 1
    "readonly",  # 2
    "defaultLibrary",  # 3
]

_TYPE_INDEX = {name: i for i, name in enumerate(SEMANTIC_TOKEN_TYPES)}
_MOD_INDEX = {name: i for i, name in enumerate(SEMANTIC_TOKEN_MODIFIERS)}

# Commands recognised as keywords for highlighting.
# Built from the registry at import time, plus language keywords and TclOO
# definition-context words that aren't standalone commands.
_KEYWORDS = frozenset(REGISTRY.command_names()) | frozenset(
    {
        # Control-flow words that appear as arguments, not standalone commands
        "else",
        "elseif",
        # TclOO definition-context keywords (valid inside oo::define bodies)
        "constructor",
        "destructor",
        "method",
        "my",
        "next",
        "self",
        "forward",
        "mixin",
        "filter",
        "superclass",
        "renamemethod",
        "deletemethod",
        "export",
        "unexport",
    }
)

# Prefix math/comparison operators
_OPERATORS = frozenset({"+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="})
_BINARY_FORMAT_SPECIFIERS = frozenset(
    {
        "a",
        "A",
        "b",
        "B",
        "h",
        "H",
        "c",
        "s",
        "S",
        "i",
        "I",
        "n",
        "w",
        "W",
        "m",
        "r",
        "R",
        "f",
        "d",
        "x",
        "X",
        "@",
        "t",
    }
)
# Integer specifiers that accept signed/unsigned modifiers (Tcl 8.5+).
_BINARY_INT_SPECIFIERS = frozenset({"c", "s", "S", "i", "I", "n", "t", "w", "W", "m", "r", "R"})

# ALLCAPS identifier pattern for iRule event names (e.g. HTTP_REQUEST).
_EVENT_RE = re.compile(r"^[A-Z][A-Z0-9_]+$")

# Tcl backslash escape sequences in source text.
_ESCAPE_RE = re.compile(
    r"\\(?:[abfnrtv\\\"\{\}\[\]\$ ]"
    r"|x[0-9a-fA-F]{1,2}"
    r"|u[0-9a-fA-F]{1,4}"
    r"|U[0-9a-fA-F]{1,8}"
    r"|[0-7]{1,3}"
    r"|\n[ \t]*"
    r"|."  # any other \char (Tcl treats unknown escapes as the char itself)
    r")"
)

# BIG-IP semantic highlighting patterns
_BIGIP_OBJECT_PATH_RE = re.compile(r"/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.:%-]+)+")
_BIGIP_IPV4_RE = re.compile(
    r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)"
    r"(?:%\d+)?(?:/\d{1,2})?\b"
)
_BIGIP_PORT_VALUE_RE = re.compile(r":(\d{1,5})\b")
_BIGIP_PORT_KEY_RE = re.compile(r"^\s*[\w-]*port[\w-]*\s+(\d{1,5})\b")
_BIGIP_ROUTE_DOMAIN_RE = re.compile(r"%\d+")
_BIGIP_PARTITION_DECL_RE = re.compile(r"^\s*auth\s+partition\s+([^\s{]+)")
_BIGIP_USERNAME_DECL_RE = re.compile(r"^\s*auth\s+user\s+([^\s{]+)")
_BIGIP_USERNAME_KEY_RE = re.compile(r"^\s*(?:user|username)\s+([^\s{]+)")
_BIGIP_SECRET_KEY_RE = re.compile(r"^\s*(secret|passphrase|password|encrypted-password)\s+([^\s]+)")
_BIGIP_FQDN_RE = re.compile(r"\b[A-Za-z0-9][A-Za-z0-9-]*(?:\.[A-Za-z0-9][A-Za-z0-9-]*)+\b")
_BIGIP_KEYED_OBJECT_REF_RE = re.compile(
    r"^\s*(pool|last-hop-pool|monitor|profile|rule|snatpool|traffic-group|"
    r"route-domain|default-route-domain|virtual-address|destination|"
    r"vlan|vlan-group|policy|fw-enforced-policy|fw-staged-policy|service-policy|"
    r"security-nat-policy|ha-group|device-group|gateway|gw"
    r")\s+([^\s{]+)"
)
_BIGIP_FS_PREFIXES = ("/config/", "/usr/", "/var/", "/shared/")

# Map BIG-IP keywords to specific semantic token types.
_BIGIP_KEYWORD_TYPE: dict[str, str] = {
    "pool": "pool",
    "last-hop-pool": "pool",
    "snatpool": "pool",
    "monitor": "monitor",
    "profile": "profile",
    "vlan": "vlan",
    "vlan-group": "vlan",
}

# Top-level BIG-IP object declarations:  ltm pool /Common/x {
_BIGIP_TOP_DECL_RE = re.compile(
    r"^\s*(?:ltm|gtm|net|sys|auth|security|wom|pem|apm|asm|ilx)\s+"
    r"(pool|virtual(?:-address)?|monitor\s+\S+|profile\s+\S+|rule|"
    r"snatpool|snat-translation|node|data-group|persistence|ifile|"
    r"dns|wideip|server|prober-pool|topology|"
    r"vlan|trunk|interface|self|route(?:-domain)?|"
    r"policy(?:-strategy)?)"
    r"\s+(/[^\s{]+)"
)

# Map declaration object types to semantic token types.
_BIGIP_DECL_TYPE: dict[str, str] = {
    "pool": "pool",
    "snatpool": "pool",
    "monitor": "monitor",
    "profile": "profile",
    "vlan": "vlan",
    "trunk": "vlan",
    "interface": "interface",
}

# BIG-IP interface names: slot.port (e.g. 1.1, 2.3) or mgmt
_BIGIP_INTERFACE_LINE_RE = re.compile(r"^\s*(?:net\s+)?interface\s+([\d]+\.[\d]+|mgmt)\b")
# Bare interface name on its own line (inside an interfaces { } block)
_BIGIP_BARE_INTERFACE_RE = re.compile(r"^\s+([\d]+\.[\d]+|mgmt)\s*\{?\s*$")


def _normalise_bigip_atom(text: str) -> str:
    """Strip common delimiters around a BIG-IP value atom."""
    return text.strip("{}\"'")


def _strip_bigip_port(host: str) -> str:
    """Strip a trailing ``:port`` suffix when present."""
    if host.count(":") == 1:
        left, right = host.rsplit(":", 1)
        if right.isdigit():
            return left
    return host


def _looks_like_fqdn(text: str) -> bool:
    """Heuristic FQDN detector for BIG-IP value atoms."""
    atom = _normalise_bigip_atom(text).rstrip(".")
    if "." not in atom:
        return False
    if not any(ch.isalpha() for ch in atom):
        return False
    labels = atom.split(".")
    if len(labels) < 2:
        return False
    for label in labels:
        if not label:
            return False
        if label.startswith("-") or label.endswith("-"):
            return False
        if not re.fullmatch(r"[A-Za-z0-9-]+", label):
            return False
    return True


def _append_bigip_token(
    out: list[tuple[int, int, int, int, int]],
    seen: set[tuple[int, int, int, int, int]],
    *,
    line: int,
    char: int,
    length: int,
    type_name: str,
) -> None:
    """Append a BIG-IP token once (deduplicated by exact span + type)."""
    if length <= 0:
        return
    type_idx = _TYPE_INDEX.get(type_name)
    if type_idx is None:
        return
    token = (line, char, length, type_idx, 0)
    if token in seen:
        return
    seen.add(token)
    out.append(token)


def _emit_bigip_path_tokens(
    out: list[tuple[int, int, int, int, int]],
    seen: set[tuple[int, int, int, int, int]],
    *,
    line_no: int,
    start_char: int,
    path: str,
    tail_type: str = "object",
) -> None:
    """Emit partition/object/IP/FQDN/port/route-domain tokens for one path."""
    if path.startswith(_BIGIP_FS_PREFIXES):
        return

    parts = path[1:].split("/")
    if len(parts) >= 2 and parts[0]:
        _append_bigip_token(
            out,
            seen,
            line=line_no,
            char=start_char + 1,
            length=len(parts[0]),
            type_name="partition",
        )

    tail = parts[-1] if parts else ""
    if tail:
        tail_start = start_char + len(path) - len(tail)
        _append_bigip_token(
            out,
            seen,
            line=line_no,
            char=tail_start,
            length=len(tail),
            type_name=tail_type,
        )

    for rd in _BIGIP_ROUTE_DOMAIN_RE.finditer(path):
        _append_bigip_token(
            out,
            seen,
            line=line_no,
            char=start_char + rd.start(),
            length=rd.end() - rd.start(),
            type_name="routeDomain",
        )

    for ip in _BIGIP_IPV4_RE.finditer(path):
        _append_bigip_token(
            out,
            seen,
            line=line_no,
            char=start_char + ip.start(),
            length=ip.end() - ip.start(),
            type_name="ipAddress",
        )

    for pm in _BIGIP_PORT_VALUE_RE.finditer(path):
        port_text = pm.group(1)
        try:
            port = int(port_text)
        except ValueError:
            continue
        if not (0 <= port <= 65535):
            continue
        _append_bigip_token(
            out,
            seen,
            line=line_no,
            char=start_char + pm.start(1),
            length=len(port_text),
            type_name="port",
        )

    cursor = 1
    for segment in parts:
        seg_start = start_char + cursor
        cursor += len(segment) + 1
        base = _strip_bigip_port(segment.split("%", 1)[0])
        if _looks_like_fqdn(base):
            _append_bigip_token(
                out,
                seen,
                line=line_no,
                char=seg_start,
                length=len(base),
                type_name="fqdn",
            )


def _collect_bigip_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
    *,
    lines: list[str] | None = None,
) -> None:
    """Collect BIG-IP-specific semantic tokens from config-like text."""
    seen: set[tuple[int, int, int, int, int]] = set()
    if lines is None:
        lines = source.split("\n")

    for line_no, line in enumerate(lines):
        stripped = line.lstrip()
        if not stripped or stripped.startswith("#"):
            continue

        # Determine the object type from line context (declaration or keyed ref).
        line_type = "object"
        top_decl = _BIGIP_TOP_DECL_RE.match(line)
        if top_decl:
            decl_kind = top_decl.group(1).split()[0]
            line_type = _BIGIP_DECL_TYPE.get(decl_kind, "object")
        else:
            keyed_ref = _BIGIP_KEYED_OBJECT_REF_RE.match(line)
            if keyed_ref:
                kw = keyed_ref.group(1)
                line_type = _BIGIP_KEYWORD_TYPE.get(kw, "object")

        for path_m in _BIGIP_OBJECT_PATH_RE.finditer(line):
            _emit_bigip_path_tokens(
                tokens,
                seen,
                line_no=line_no,
                start_char=path_m.start(),
                path=path_m.group(),
                tail_type=line_type,
            )

        for ip_m in _BIGIP_IPV4_RE.finditer(line):
            _append_bigip_token(
                tokens,
                seen,
                line=line_no,
                char=ip_m.start(),
                length=ip_m.end() - ip_m.start(),
                type_name="ipAddress",
            )

        for rd_m in _BIGIP_ROUTE_DOMAIN_RE.finditer(line):
            _append_bigip_token(
                tokens,
                seen,
                line=line_no,
                char=rd_m.start(),
                length=rd_m.end() - rd_m.start(),
                type_name="routeDomain",
            )

        port_key = _BIGIP_PORT_KEY_RE.match(line)
        if port_key:
            port_text = port_key.group(1)
            try:
                port = int(port_text)
            except ValueError:
                port = -1
            if 0 <= port <= 65535:
                _append_bigip_token(
                    tokens,
                    seen,
                    line=line_no,
                    char=port_key.start(1),
                    length=len(port_text),
                    type_name="port",
                )

        keyed_ref = _BIGIP_KEYED_OBJECT_REF_RE.match(line)
        if keyed_ref:
            kw = keyed_ref.group(1)
            ref = _normalise_bigip_atom(keyed_ref.group(2))
            if ref and not ref.startswith("/"):
                _append_bigip_token(
                    tokens,
                    seen,
                    line=line_no,
                    char=keyed_ref.start(2),
                    length=len(keyed_ref.group(2)),
                    type_name=_BIGIP_KEYWORD_TYPE.get(kw, "object"),
                )

        # Interface names: "net interface 1.1" or bare "1.1 {" inside blocks
        iface_m = _BIGIP_INTERFACE_LINE_RE.match(line)
        if not iface_m:
            iface_m = _BIGIP_BARE_INTERFACE_RE.match(line)
        if iface_m:
            _append_bigip_token(
                tokens,
                seen,
                line=line_no,
                char=iface_m.start(1),
                length=len(iface_m.group(1)),
                type_name="interface",
            )

        part_decl = _BIGIP_PARTITION_DECL_RE.match(line)
        if part_decl:
            _append_bigip_token(
                tokens,
                seen,
                line=line_no,
                char=part_decl.start(1),
                length=len(part_decl.group(1)),
                type_name="partition",
            )

        user_decl = _BIGIP_USERNAME_DECL_RE.match(line)
        if user_decl:
            _append_bigip_token(
                tokens,
                seen,
                line=line_no,
                char=user_decl.start(1),
                length=len(user_decl.group(1)),
                type_name="username",
            )

        user_key = _BIGIP_USERNAME_KEY_RE.match(line)
        if user_key:
            user_value = _normalise_bigip_atom(user_key.group(1))
            if user_value.lower() not in {"none", "nobody"}:
                _append_bigip_token(
                    tokens,
                    seen,
                    line=line_no,
                    char=user_key.start(1),
                    length=len(user_key.group(1)),
                    type_name="username",
                )

        secret_match = _BIGIP_SECRET_KEY_RE.match(line)
        if secret_match:
            key_name = secret_match.group(1).lower()
            raw_value = secret_match.group(2)
            normalised = _normalise_bigip_atom(raw_value)
            if "encrypted" in key_name or normalised.startswith("$"):
                val_start = secret_match.start(2)
                val_end = secret_match.end(2)
                while val_start < val_end and line[val_start] in "\"'{":
                    val_start += 1
                while val_end > val_start and line[val_end - 1] in "\"'}":
                    val_end -= 1
                _append_bigip_token(
                    tokens,
                    seen,
                    line=line_no,
                    char=val_start,
                    length=val_end - val_start,
                    type_name="encrypted",
                )

        for fqdn_m in _BIGIP_FQDN_RE.finditer(line):
            fqdn = fqdn_m.group(0)
            if _looks_like_fqdn(fqdn):
                _append_bigip_token(
                    tokens,
                    seen,
                    line=line_no,
                    char=fqdn_m.start(),
                    length=len(fqdn),
                    type_name="fqdn",
                )


def _collect_irules_object_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
) -> None:
    """Collect semantic tokens for object references in iRules source."""
    seen: set[tuple[int, int, int, int, int]] = set()
    for ref in extract_irules_object_references(source):
        start = ref.range.start
        end = ref.range.end
        if start.line != end.line:
            continue
        _append_bigip_token(
            tokens,
            seen,
            line=start.line,
            char=start.character,
            length=(end.character - start.character + 1),
            type_name="object",
        )


def _collect_bigip_embedded_irules_object_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
) -> None:
    """Collect semantic tokens for object refs inside embedded ``ltm rule`` bodies."""
    seen: set[tuple[int, int, int, int, int]] = set()
    source_map = SourceMap(source)
    for rule in find_embedded_rules(source):
        rule_module = "gtm" if rule.header.startswith("gtm ") else "ltm"
        body_start = source_map.offset_to_position(rule.body_start_offset - 1)
        body_end = source_map.offset_to_position(
            max(rule.body_end_offset - 1, rule.body_start_offset - 1)
        )
        for ref in extract_irules_object_references(
            rule.body,
            rule_module=rule_module,
            body_token=Token(
                type=TokenType.STR,
                text=rule.body,
                start=SourcePosition(
                    line=body_start.line,
                    character=body_start.character,
                    offset=rule.body_start_offset - 1,
                ),
                end=SourcePosition(
                    line=body_end.line,
                    character=body_end.character,
                    offset=max(rule.body_end_offset - 1, rule.body_start_offset - 1),
                ),
            ),
        ):
            start = ref.range.start
            end = ref.range.end
            if start.line != end.line:
                continue
            _append_bigip_token(
                tokens,
                seen,
                line=start.line,
                char=start.character,
                length=(end.character - start.character + 1),
                type_name="object",
            )


def _collect_embedded_tcl_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
) -> list[tuple[int, int]]:
    """Collect full Tcl tokens for embedded iRule and iApp bodies in bigip.conf.

    Finds all ``ltm rule`` / ``gtm rule`` bodies and ``implementation`` /
    ``presentation`` sections in iApp templates, then runs the full Tcl
    token collector on each body so that keywords, variables, events,
    strings, etc. receive proper semantic highlighting.

    Returns the list of (body_start_line, body_end_line) ranges that were
    tokenised so the caller can filter overlapping tokens from the
    whole-file pass.
    """
    source_map = SourceMap(source)
    body_ranges: list[tuple[int, int]] = []

    # --- Embedded iRules (ltm rule / gtm rule) ---
    for rule in find_embedded_rules(source):
        body_start = source_map.offset_to_position(rule.body_start_offset - 1)
        body_end = source_map.offset_to_position(
            max(rule.body_end_offset - 1, rule.body_start_offset - 1)
        )
        body_ranges.append((body_start.line, body_end.line))
        body_token = Token(
            type=TokenType.STR,
            text=rule.body,
            start=SourcePosition(
                line=body_start.line,
                character=body_start.character,
                offset=rule.body_start_offset - 1,
            ),
            end=SourcePosition(
                line=body_end.line,
                character=body_end.character,
                offset=max(rule.body_end_offset - 1, rule.body_start_offset - 1),
            ),
        )
        _collect_tokens(tokens, rule.body, body_token=body_token, regex_positions=regex_positions)

    # --- Embedded iApp sections (implementation / presentation) ---
    for section in find_embedded_iapp_sections(source):
        sec_start = source_map.offset_to_position(section.body_start_offset - 1)
        sec_end = source_map.offset_to_position(
            max(section.body_end_offset - 1, section.body_start_offset - 1)
        )
        body_ranges.append((sec_start.line, sec_end.line))
        body_token = Token(
            type=TokenType.STR,
            text=section.body,
            start=SourcePosition(
                line=sec_start.line,
                character=sec_start.character,
                offset=section.body_start_offset - 1,
            ),
            end=SourcePosition(
                line=sec_end.line,
                character=sec_end.character,
                offset=max(section.body_end_offset - 1, section.body_start_offset - 1),
            ),
        )
        _collect_tokens(
            tokens, section.body, body_token=body_token, regex_positions=regex_positions
        )

    return body_ranges


def _classify_token(tok_type: TokenType, text: str, *, is_command_name: bool) -> int | None:
    """Return semantic token type index, or None to skip this token."""
    match tok_type:
        case TokenType.VAR:
            return _TYPE_INDEX["variable"]
        case TokenType.CMD:
            return None  # command contents handled by recursive tokenisation
        case TokenType.STR:
            return _TYPE_INDEX["string"]
        case TokenType.COMMENT:
            return _TYPE_INDEX["comment"]
        case TokenType.ESC:
            if is_command_name:
                if text in _KEYWORDS:
                    return _TYPE_INDEX["keyword"]
                if text in _OPERATORS:
                    return _TYPE_INDEX["operator"]
                return _TYPE_INDEX["function"]
            # Check if it's a number
            try:
                int(text)
                return _TYPE_INDEX["number"]
            except ValueError:
                pass
            try:
                float(text)
                return _TYPE_INDEX["number"]
            except ValueError:
                pass
            return _TYPE_INDEX["string"]
        case _:
            return None


def _arg_indices(cmd_name: str, argv_texts: list[str], role: ArgRole) -> set[int]:
    """Return argument indices (0-based, after command name) for a given role."""
    return arg_indices_for_role(cmd_name, argv_texts, role)


def _classify_expr_token(tok_type: ExprTokenType, text: str) -> int | None:
    """Return semantic token type index for expression tokens."""
    match tok_type:
        case ExprTokenType.VARIABLE:
            return _TYPE_INDEX["variable"]
        case ExprTokenType.NUMBER:
            return _TYPE_INDEX["number"]
        case ExprTokenType.OPERATOR | ExprTokenType.PAREN_OPEN | ExprTokenType.PAREN_CLOSE:
            return _TYPE_INDEX["operator"]
        case ExprTokenType.TERNARY_Q | ExprTokenType.TERNARY_C | ExprTokenType.COMMA:
            return _TYPE_INDEX["operator"]
        case ExprTokenType.FUNCTION:
            return _TYPE_INDEX["function"]
        case ExprTokenType.BOOL:
            return _TYPE_INDEX["keyword"]
        case ExprTokenType.STRING:
            return _TYPE_INDEX["string"]
        case _:
            return None


def _append_token(
    out: list[tuple[int, int, int, int, int]],
    *,
    line: int,
    char: int,
    length: int,
    type_idx: int,
    modifiers: int = 0,
) -> None:
    """Append a token if it has a positive length."""
    if length <= 0:
        return
    out.append((line, char, length, type_idx, modifiers))


def _append_text_token(
    out: list[tuple[int, int, int, int, int]],
    *,
    start: SourcePosition,
    text: str,
    type_idx: int,
    modifiers: int = 0,
) -> None:
    """Append semantic token segments, splitting safely across lines."""
    if not text:
        return

    line = start.line
    char = start.character
    parts = text.split("\n")
    for i, part in enumerate(parts):
        _append_token(
            out,
            line=line,
            char=char,
            length=len(part),
            type_idx=type_idx,
            modifiers=modifiers,
        )
        if i < len(parts) - 1:
            line += 1
            char = 0


def _emit_namespace_qualified(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
    type_idx: int,
    modifiers: int = 0,
) -> None:
    """Split ``NS::cmd`` into a namespace token and a command token."""
    text = tok.text
    idx = text.rfind("::")
    ns_part = text[: idx + 2]
    cmd_part = text[idx + 2 :]

    base_offset, base_line, base_col = token_content_base(tok)
    ns_start = position_from_relative(
        text,
        0,
        base_line=base_line,
        base_col=base_col,
        base_offset=base_offset,
    )
    _append_text_token(
        out,
        start=ns_start,
        text=ns_part,
        type_idx=_TYPE_INDEX["namespace"],
        modifiers=modifiers,
    )
    if cmd_part:
        cmd_start = position_from_relative(
            text,
            idx + 2,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=cmd_start,
            text=cmd_part,
            type_idx=type_idx,
            modifiers=modifiers,
        )


def _emit_string_with_escapes(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
) -> bool:
    """Sub-tokenize an ESC token to highlight backslash escape sequences.

    Returns True when at least one escape was emitted, False otherwise.
    """
    text = tok.text
    matches = list(_ESCAPE_RE.finditer(text))
    if not matches:
        return False

    base_offset, base_line, base_col = token_content_base(tok)
    pos = 0

    for match in matches:
        if match.start() > pos:
            before_start = position_from_relative(
                text,
                pos,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )
        esc_start = position_from_relative(
            text,
            match.start(),
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=esc_start,
            text=match.group(),
            type_idx=_TYPE_INDEX["escape"],
        )
        pos = match.end()

    if pos < len(text):
        rest_start = position_from_relative(
            text,
            pos,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos:],
            type_idx=_TYPE_INDEX["string"],
        )
    return True


def _collect_expression_tokens(
    out: list[tuple[int, int, int, int, int]],
    expr_text: str,
    owner_token: Token,
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
) -> None:
    """Collect semantic tokens from an expression argument."""
    if owner_token.type is TokenType.STR:
        base_offset = owner_token.start.offset + 1
        base_line = owner_token.start.line
        base_col = owner_token.start.character + 1
    else:
        base_offset = owner_token.start.offset
        base_line = owner_token.start.line
        base_col = owner_token.start.character

    prev_op_text = ""
    for expr_tok in tokenise_expr(expr_text, dialect=active_dialect()):
        if expr_tok.type is ExprTokenType.WHITESPACE:
            continue

        if expr_tok.type is ExprTokenType.COMMAND and len(expr_tok.text) >= 2:
            cmd_text = expr_tok.text[1:-1]
            cmd_start = position_from_relative(
                expr_text,
                expr_tok.start,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            cmd_end = position_from_relative(
                expr_text,
                expr_tok.end,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            synthetic = Token(type=TokenType.CMD, text=cmd_text, start=cmd_start, end=cmd_end)
            _collect_tokens(out, cmd_text, body_token=synthetic, regex_positions=regex_positions)
            prev_op_text = ""
            continue

        # iRules matches_glob / matches_regex: highlight the RHS string
        if expr_tok.type is ExprTokenType.STRING and prev_op_text in (
            "matches_glob",
            "matches_regex",
        ):
            str_text = expr_tok.text
            # Strip surrounding delimiters (braces or quotes)
            if len(str_text) >= 2 and str_text[0] == "{" and str_text[-1] == "}":
                inner = str_text[1:-1]
                inner_offset = expr_tok.start + 1
            elif len(str_text) >= 2 and str_text[0] == '"' and str_text[-1] == '"':
                inner = str_text[1:-1]
                inner_offset = expr_tok.start + 1
            else:
                inner = str_text
                inner_offset = expr_tok.start

            inner_start = position_from_relative(
                expr_text,
                inner_offset,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            inner_end = position_from_relative(
                expr_text,
                inner_offset + len(inner) - 1,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            synthetic = Token(
                type=TokenType.STR,
                text=inner,
                start=inner_start,
                end=inner_end,
            )
            if prev_op_text == "matches_glob":
                if not _collect_glob_pattern_tokens(out, synthetic):
                    _append_text_token(
                        out,
                        start=inner_start,
                        text=inner,
                        type_idx=_TYPE_INDEX["string"],
                    )
            else:
                _emit_regex_token(out, synthetic)
            # Emit surrounding delimiters (quotes only; braces are syntax)
            if len(str_text) >= 2 and str_text[0] == '"':
                quote_start = position_from_relative(
                    expr_text,
                    expr_tok.start,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                _append_text_token(
                    out,
                    start=quote_start,
                    text='"',
                    type_idx=_TYPE_INDEX["string"],
                )
                end_quote_start = position_from_relative(
                    expr_text,
                    expr_tok.start + len(str_text) - 1,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                _append_text_token(
                    out,
                    start=end_quote_start,
                    text='"',
                    type_idx=_TYPE_INDEX["string"],
                )
            prev_op_text = ""
            continue

        if expr_tok.type is ExprTokenType.OPERATOR:
            prev_op_text = expr_tok.text
        else:
            prev_op_text = ""

        type_idx = _classify_expr_token(expr_tok.type, expr_tok.text)
        if type_idx is None:
            continue
        modifiers = 0
        if expr_tok.type is ExprTokenType.FUNCTION:
            modifiers = 1 << _MOD_INDEX["defaultLibrary"]
        start = position_from_relative(
            expr_text,
            expr_tok.start,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=start,
            text=expr_tok.text,
            type_idx=type_idx,
            modifiers=modifiers,
        )


def _split_words(
    source: str,
    *,
    base_offset: int = 0,
    base_line: int = 0,
    base_col: int = 0,
) -> tuple[list[str], list[Token]]:
    """Split Tcl words and return (word_texts, first_token_per_word)."""
    lexer = TclLexer(source, base_offset=base_offset, base_line=base_line, base_col=base_col)
    words: list[str] = []
    word_tokens: list[Token] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type in (TokenType.SEP, TokenType.EOL):
            prev_type = tok.type
            continue

        if prev_type in (TokenType.SEP, TokenType.EOL):
            words.append(tok.text)
            word_tokens.append(tok)
        elif words:
            words[-1] += tok.text
        else:
            words.append(tok.text)
            word_tokens.append(tok)
        prev_type = tok.type

    return words, word_tokens


def _collect_param_list_tokens(
    out: list[tuple[int, int, int, int, int]],
    param_token: Token,
) -> bool:
    """Emit semantic parameter tokens from a Tcl parameter-list argument."""
    if param_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    base_offset, base_line, base_col = token_content_base(param_token)
    words, word_tokens = _split_words(
        param_token.text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )

    emitted = False
    for word, tok in zip(words, word_tokens):
        param_name = word
        param_start = tok.start

        # Braced parameter form: {name ?default?}
        if tok.type is TokenType.STR:
            inner_offset, inner_line, inner_col = token_content_base(tok)
            inner_words, inner_tokens = _split_words(
                tok.text,
                base_offset=inner_offset,
                base_line=inner_line,
                base_col=inner_col,
            )
            if not inner_words or not inner_tokens:
                continue
            param_name = inner_words[0]
            param_start = inner_tokens[0].start

        if not param_name:
            continue

        _append_text_token(
            out,
            start=param_start,
            text=param_name,
            type_idx=_TYPE_INDEX["parameter"],
            modifiers=1 << _MOD_INDEX["declaration"],
        )
        emitted = True

    return emitted


def _proc_param_list_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a formal parameter list, if known."""
    if cmd_name in ("proc", "method"):
        return 2
    if cmd_name == "constructor":
        return 1
    if cmd_name == "self" and len(argv_texts) >= 2:
        if argv_texts[1] == "method":
            return 3
        if argv_texts[1] == "constructor":
            return 2
    return None


# Tcl ARE/ERE/BRE regex components for sub-tokenization.
# Matches metacharacters, character classes, escape sequences, quantifiers,
# groups, anchors, and backreferences.
_REGEX_PART_RE = re.compile(
    r"(?:"
    r"\(\?[imnsxwpq]*(?:[-imnsxwpq]*)?\)"  # (?flags) embedded flags
    r"|\(\?(?:[:=!>])"  # non-capturing / lookahead / lookbehind group open
    r"|\("  # group open
    r"|\)"  # group close
    r"|\[(?:\^)?\]?(?:[^\]\\]|\\.)*\]"  # character class [...]
    r"|\\[AbBdDmMsSwWyYZ]"  # ARE class shortcuts
    r"|\\[0-9]"  # backreference
    r"|\\[.*+?(){}\[\]|^$\\]"  # escaped metachar
    r"|\\[aefnrtv]"  # escape sequences
    r"|\\x[0-9a-fA-F]{1,2}"  # hex escape
    r"|\\u[0-9a-fA-F]{1,4}"  # unicode escape
    r"|\\U[0-9a-fA-F]{1,8}"  # wide unicode escape
    r"|[*+?](?:\?)?|\\{\\d+(?:,\\d*)?\\}"  # quantifiers
    r"|\{(?:\d+)(?:,\d*)?\}"  # bounded quantifier {n,m}
    r"|[|]"  # alternation
    r"|[\^$]"  # anchors
    r"|[.]"  # any-char
    r")"
)


def _collect_regex_pattern_tokens(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
) -> bool:
    """Sub-tokenize a regex pattern into its components.

    Returns True when at least one sub-token was emitted.
    """
    if tok.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = tok.text
    matches = list(_REGEX_PART_RE.finditer(text))
    if not matches:
        # No metacharacters — just a literal. Emit as single regexp token.
        return False

    base_offset, base_line, base_col = token_content_base(tok)
    pos_in_text = 0

    for match in matches:
        # Literal text before this metacharacter
        if match.start() > pos_in_text:
            before_start = position_from_relative(
                text,
                pos_in_text,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["regexp"],
            )

        matched = match.group()
        meta_start = position_from_relative(
            text,
            match.start(),
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )

        # Classify the component using dedicated ARE token types
        if matched.startswith("["):
            # Character class [...]
            type_idx = _TYPE_INDEX["regexpCharClass"]
        elif matched.startswith("\\") and len(matched) >= 2:
            ch = matched[1]
            if ch.isdigit():
                # Backreference \0–\9
                type_idx = _TYPE_INDEX["regexpBackref"]
            elif ch in "aefnrtv" or ch == "x" or ch == "u" or ch == "U":
                # Escape sequence \n \t \xHH etc.
                type_idx = _TYPE_INDEX["regexpEscape"]
            elif ch in "dDsSwW":
                # ARE class shortcut (\d, \s, \w and negated)
                type_idx = _TYPE_INDEX["regexpCharClass"]
            elif ch in "bBmMyYAZ":
                # ARE anchors (\b, \B, \m, \M, \y, \Y, \A, \Z)
                type_idx = _TYPE_INDEX["regexpAnchor"]
            else:
                # Escaped metachar
                type_idx = _TYPE_INDEX["regexpEscape"]
        elif matched in ("^", "$"):
            type_idx = _TYPE_INDEX["regexpAnchor"]
        elif matched in ("(", ")") or matched.startswith("(?"):
            type_idx = _TYPE_INDEX["regexpGroup"]
        elif matched in ("|",):
            type_idx = _TYPE_INDEX["regexpAlternation"]
        elif matched == ".":
            # Any-char dot is a character class
            type_idx = _TYPE_INDEX["regexpCharClass"]
        elif matched in ("*", "+", "?", "*?", "+?", "??"):
            type_idx = _TYPE_INDEX["regexpQuantifier"]
        elif matched.startswith("{") and matched.endswith("}"):
            # Bounded quantifier {n,m}
            type_idx = _TYPE_INDEX["regexpQuantifier"]
        else:
            type_idx = _TYPE_INDEX["regexpQuantifier"]

        _append_text_token(
            out,
            start=meta_start,
            text=matched,
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    # Remaining literal text
    if pos_in_text < len(text):
        rest_start = position_from_relative(
            text,
            pos_in_text,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["regexp"],
        )

    return True


def _emit_regex_token(
    out: list[tuple[int, int, int, int, int]],
    tok: Token,
) -> None:
    """Emit semantic tokens for a regex pattern, with sub-tokenization."""
    if _collect_regex_pattern_tokens(out, tok):
        return
    # Fallback: emit as a single regexp token
    rendered = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
    _append_text_token(
        out,
        start=tok.start,
        text=rendered,
        type_idx=_TYPE_INDEX["regexp"],
    )


def _collect_switch_case_bodies(
    out: list[tuple[int, int, int, int, int]],
    args: list[str],
    arg_tokens: list[Token],
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
) -> set[int]:
    """Collect body tokens for switch braced case-list form.

    Returns argument indices (0-based after command name) whose generic BODY
    recursion should be skipped because they are handled here.

    When ``-regexp`` is among the option switches, pattern elements are
    emitted as ``regexp`` semantic tokens.
    """
    is_regexp = False
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "-regexp":
            is_regexp = True
        if args[i] == "--":
            i += 1
            break
        i += 1

    if i >= len(args):
        return set()
    i += 1  # switch value/pattern source
    if i >= len(args):
        return set()

    # Non-braced form: pattern/body pairs as separate arguments.
    if i != len(args) - 1:
        # Emit regex tokens for patterns in inline form.
        # VAR tokens are skipped here — the analysis-driven regex_positions
        # override in the main token loop handles variable patterns.
        if is_regexp:
            j = i
            while j + 1 < len(args):
                if args[j] != "default" and j < len(arg_tokens):
                    tok = arg_tokens[j]
                    if tok.type is not TokenType.VAR:
                        _emit_regex_token(out, tok)
                j += 2
        return set()

    if i >= len(arg_tokens):
        return {i}

    case_list_tok = arg_tokens[i]
    if case_list_tok.type is not TokenType.STR:
        return set()

    case_offset, case_line, case_col = token_content_base(case_list_tok)
    elements, element_tokens = _split_words(
        args[i],
        base_offset=case_offset,
        base_line=case_line,
        base_col=case_col,
    )

    idx = 0
    while idx + 1 < len(elements):
        # Emit regex token for pattern in braced case list
        if is_regexp and elements[idx] != "default":
            if idx < len(element_tokens):
                _emit_regex_token(out, element_tokens[idx])
        body = elements[idx + 1]
        body_tok = element_tokens[idx + 1]
        if body != "-" and body_tok.type is TokenType.STR and body.strip():
            _collect_tokens(out, body, body_token=body_tok, regex_positions=regex_positions)
        idx += 2

    return {i}


def _procedure_name_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a procedure/method name definition."""
    if cmd_name == "proc" and len(argv_texts) >= 2:
        return 1
    if cmd_name == "method" and len(argv_texts) >= 2:
        return 1
    if cmd_name in ("oo::define", "oo::objdefine") and len(argv_texts) >= 4:
        if argv_texts[2] == "method":
            return 3
        if argv_texts[2] == "self" and len(argv_texts) >= 5 and argv_texts[3] == "method":
            return 4
    if cmd_name == "self" and len(argv_texts) >= 3 and argv_texts[1] == "method":
        return 2
    return None


def _binary_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a binary format string argument."""
    if cmd_name != "binary" or len(argv_texts) < 3:
        return None
    if argv_texts[1] == "format":
        return 2
    if argv_texts[1] == "scan" and len(argv_texts) >= 4:
        return 3
    return None


def _string_map_mapping_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the string map mapping list."""
    if cmd_name != "string" or len(argv_texts) < 4:
        return None
    if argv_texts[1] != "map":
        return None
    i = 2
    if i < len(argv_texts) and argv_texts[i] == "-nocase":
        i += 1
    return i if i < len(argv_texts) else None


def _collect_string_map_pairs_tokens(
    out: list[tuple[int, int, int, int, int]],
    mapping_token: Token,
) -> bool:
    """Tokenise string map mapping list with alternating pair colours.

    Each key-value pair gets a distinct colour that alternates between
    two token types so the user can visually associate keys with their
    replacement values.
    """
    if mapping_token.type is not TokenType.STR:
        return False

    base_offset, base_line, base_col = token_content_base(mapping_token)
    elements, element_tokens = _split_words(
        mapping_token.text,
        base_offset=base_offset,
        base_line=base_line,
        base_col=base_col,
    )

    if len(elements) < 2:
        return False

    pair_types = [_TYPE_INDEX["string"], _TYPE_INDEX["parameter"]]

    for idx, (elem, tok) in enumerate(zip(elements, element_tokens)):
        pair_num = idx // 2
        type_idx = pair_types[pair_num % 2]
        rendered = f"{{{elem}}}" if tok.type is TokenType.STR else elem
        _append_text_token(
            out,
            start=tok.start,
            text=rendered,
            type_idx=type_idx,
        )

    return True


def _subcommand_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a known subcommand token."""
    sig = SIGNATURES.get(cmd_name)
    if not isinstance(sig, SubcommandSig):
        return None
    if len(argv_texts) < 2:
        return None
    return 1


def _is_known_subcommand(cmd_name: str, sub_name: str) -> bool:
    """Return True when *sub_name* is a known subcommand for *cmd_name*."""
    sig = SIGNATURES.get(cmd_name)
    if not isinstance(sig, SubcommandSig):
        return False
    return sub_name in sig.subcommands or sig.allow_unknown


def _sprintf_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a format/scan format string argument."""
    if cmd_name == "format" and len(argv_texts) >= 2:
        return 1
    if cmd_name == "scan" and len(argv_texts) >= 3:
        return 2
    return None


# Tcl format/scan specifier sequence (simplified)
# %[position$][flags][width][.precision][length_modifier]type
# The \\?\$ handles both raw $ and Tcl-escaped \$ in double-quoted strings.
_SPRINTF_RE = re.compile(
    r"%(?:(?P<position>\d+)\\?\$)?(?P<flags>[-+ 0#]*)?(?P<width>\*|\d+)?(?:.(?P<precision>\*|\d+))?(?P<length>[hlLzq])?(?P<type>[aAbBcdieEfgGosuxX%])"
)


def _collect_sprintf_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
) -> bool:
    """Tokenise format/scan specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_SPRINTF_RE.finditer(text))
    if not matches:
        return False

    base_offset, base_line, base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_relative(
                text, pos_in_text, base_line=base_line, base_col=base_col, base_offset=base_offset
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        pos = match.start()

        def emit_part(end: int, tidx: int) -> None:
            nonlocal pos
            if end > pos:
                part_pos = position_from_relative(
                    text, pos, base_line=base_line, base_col=base_col, base_offset=base_offset
                )
                _append_text_token(out, start=part_pos, text=text[pos:end], type_idx=tidx)
                pos = end

        # The '%'. It's always 1 character
        emit_part(match.start() + 1, _TYPE_INDEX["formatPercent"])

        # position (digits)
        if match.span("position")[0] != -1:
            emit_part(match.end("position"), _TYPE_INDEX["formatWidth"])
            # The '$' follows the position
            emit_part(match.end("position") + 1, _TYPE_INDEX["formatPercent"])

        # flags (+- 0#)
        if match.span("flags")[0] != -1:
            emit_part(match.end("flags"), _TYPE_INDEX["formatFlag"])

        # width (digits or *)
        if match.span("width")[0] != -1:
            tid = (
                _TYPE_INDEX["formatWidth"]
                if text[match.start("width")].isdigit()
                else _TYPE_INDEX["formatFlag"]
            )
            emit_part(match.end("width"), tid)

        # precision (starts with . then digits or *)
        if match.span("precision")[0] != -1:
            # The '.' is a flag/punctuation
            emit_part(match.start("precision"), _TYPE_INDEX["formatFlag"])
            tid = (
                _TYPE_INDEX["formatWidth"]
                if text[match.start("precision")].isdigit()
                else _TYPE_INDEX["formatFlag"]
            )
            emit_part(match.end("precision"), tid)

        # length modifier (h l L z q)
        if match.span("length")[0] != -1:
            emit_part(match.end("length"), _TYPE_INDEX["formatFlag"])

        # type specifier (s d f etc.)
        if match.span("type")[0] != -1:
            emit_part(match.end("type"), _TYPE_INDEX["formatSpec"])

        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_relative(
            text, pos_in_text, base_line=base_line, base_col=base_col, base_offset=base_offset
        )
        _append_text_token(
            out, start=rest_start, text=text[pos_in_text:], type_idx=_TYPE_INDEX["string"]
        )

    return True


def _clock_format_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing a clock format string argument.

    The format string is the VALUE of the ``-format`` option in
    ``clock format $t -format "%Y-%m-%d"`` or ``clock scan $s -format "%Y-%m-%d"``.
    """
    if cmd_name != "clock" or len(argv_texts) < 3:
        return None
    if argv_texts[1] not in ("format", "scan"):
        return None
    for i in range(2, len(argv_texts)):
        if argv_texts[i] == "-format" and i + 1 < len(argv_texts):
            return i + 1
    return None


# Tcl clock format specifiers: %[E|O]<letter> or %%
_CLOCK_FORMAT_RE = re.compile(r"%(?:[EO])?[aAbBcCdDeEgGhHIjJklmMNOpPqQsSuUVwWxXyYzZ%]")


def _collect_clock_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
) -> bool:
    """Tokenise clock format specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_CLOCK_FORMAT_RE.finditer(text))
    if not matches:
        return False

    base_offset, base_line, base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_relative(
                text,
                pos_in_text,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        # Emit the % as clockPercent
        pct_pos = position_from_relative(
            text,
            match.start(),
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=pct_pos,
            text="%",
            type_idx=_TYPE_INDEX["clockPercent"],
        )

        # After % there may be an E/O locale modifier then the specifier letter
        spec_text = match.group()[1:]  # strip leading %
        if spec_text:
            off = match.start() + 1
            if spec_text[0] in ("E", "O") and len(spec_text) > 1:
                # Emit locale modifier separately
                mod_start = position_from_relative(
                    text,
                    off,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                _append_text_token(
                    out,
                    start=mod_start,
                    text=spec_text[0],
                    type_idx=_TYPE_INDEX["clockModifier"],
                )
                off += 1
                spec_text = spec_text[1:]
            # Emit the specifier letter
            if spec_text:
                spec_start = position_from_relative(
                    text,
                    off,
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                _append_text_token(
                    out,
                    start=spec_start,
                    text=spec_text,
                    type_idx=_TYPE_INDEX["clockSpec"],
                )

        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_relative(
            text,
            pos_in_text,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _regsub_subspec_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the regsub substitution spec.

    ``regsub ?switches? exp string subSpec ?varName?``
    """
    if cmd_name != "regsub" or len(argv_texts) < 4:
        return None
    pat_idx = regexp_pattern_index(argv_texts[1:])
    if pat_idx is None:
        return None
    # pattern is at pat_idx+1 in argv; subspec is pattern+2
    subspec_idx = pat_idx + 1 + 2
    if subspec_idx < len(argv_texts):
        return subspec_idx
    return None


def _regex_pattern_arg_index(cmd_name: str, argv_texts: list[str]) -> int | None:
    """Return argv index containing the regex pattern argument.

    Works for both ``regexp`` and ``regsub``:
        regexp ?switches? exp string ?matchVar ...?
        regsub ?switches? exp string subSpec ?varName?
    The pattern (*exp*) is the first positional arg after option switches.
    """
    if cmd_name not in ("regexp", "regsub"):
        return None
    if len(argv_texts) < 3:
        return None
    pat_idx = regexp_pattern_index(argv_texts[1:])
    if pat_idx is None:
        return None
    return pat_idx + 1


# Regsub substitution backreferences: \0-\9, \&
_REGSUB_BACKREF_RE = re.compile(r"\\([0-9&])")


def _collect_regsub_subspec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
) -> bool:
    """Tokenise regsub substitution spec backreferences."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_REGSUB_BACKREF_RE.finditer(text))
    if not matches:
        return False

    base_offset, base_line, base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_relative(
                text,
                pos_in_text,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        ref_start = position_from_relative(
            text,
            match.start(),
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        ref_char = match.group(1)
        # \& and \0 = whole match → operator; \1-\9 = capture group → number
        type_idx = _TYPE_INDEX["operator"] if ref_char == "&" else _TYPE_INDEX["number"]
        _append_text_token(
            out,
            start=ref_start,
            text=match.group(),
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_relative(
            text,
            pos_in_text,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _glob_pattern_arg_indices(cmd_name: str, argv_texts: list[str]) -> set[int]:
    """Return argv indices (absolute) containing glob pattern arguments.

    Handles ``string match``, ``glob``, and ``lsearch`` (default/glob mode).
    """
    if cmd_name == "string" and len(argv_texts) >= 4 and argv_texts[1] == "match":
        i = 2
        while i < len(argv_texts) and argv_texts[i].startswith("-"):
            if argv_texts[i] == "--":
                i += 1
                break
            i += 1
        return {i} if i < len(argv_texts) else set()

    if cmd_name == "glob":
        i = skip_options(argv_texts[1:], options_with_value("glob")) + 1
        return set(range(i, len(argv_texts)))

    if cmd_name == "lsearch":
        has_regexp = any(a == "-regexp" for a in argv_texts)
        has_exact = any(a == "-exact" for a in argv_texts)
        if has_regexp or has_exact:
            return set()
        if len(argv_texts) >= 3:
            return {len(argv_texts) - 1}
        return set()

    return set()


# Glob metacharacters: *, ?, [...], \x
_GLOB_META_RE = re.compile(
    r"\\."  # escaped character
    r"|\[[^\]]*\]"  # character class [...]
    r"|\*"  # match any string
    r"|\?"  # match any single character
)


def _collect_glob_pattern_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
) -> bool:
    """Tokenise glob pattern metacharacters."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    text = spec_token.text
    matches = list(_GLOB_META_RE.finditer(text))
    if not matches:
        return False

    base_offset, base_line, base_col = token_content_base(spec_token)
    pos_in_text = 0

    for match in matches:
        if match.start() > pos_in_text:
            before_start = position_from_relative(
                text,
                pos_in_text,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=before_start,
                text=text[pos_in_text : match.start()],
                type_idx=_TYPE_INDEX["string"],
            )

        meta_start = position_from_relative(
            text,
            match.start(),
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        matched = match.group()
        if matched.startswith("\\"):
            type_idx = _TYPE_INDEX["escape"]
        elif matched.startswith("["):
            type_idx = _TYPE_INDEX["regexp"]
        else:
            type_idx = _TYPE_INDEX["operator"]
        _append_text_token(
            out,
            start=meta_start,
            text=matched,
            type_idx=type_idx,
        )
        pos_in_text = match.end()

    if pos_in_text < len(text):
        rest_start = position_from_relative(
            text,
            pos_in_text,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=rest_start,
            text=text[pos_in_text:],
            type_idx=_TYPE_INDEX["string"],
        )

    return True


def _option_arg_indices(cmd_name: str, argv_texts: list[str]) -> set[int]:
    """Return arg indices (0-based after command name) that are option flags.

    Uses the command's ``option_terminator_profiles`` to identify where
    options start, walking args that begin with ``-`` until ``--`` or
    the first positional argument.
    """
    profiles = REGISTRY.option_terminator_profiles(cmd_name)
    if not profiles:
        return set()
    # Pick the best-matching profile (subcommand-aware).
    profile = None
    for p in profiles:
        if p.subcommand is not None and argv_texts and p.subcommand == argv_texts[0]:
            profile = p
            break
    if profile is None:
        for p in profiles:
            if p.subcommand is None:
                profile = p
                break
    if profile is None:
        return set()

    result: set[int] = set()
    i = profile.scan_start
    while i < len(argv_texts):
        arg = argv_texts[i]
        if arg == "--":
            break
        if not arg.startswith("-"):
            break
        result.add(i)
        if arg in profile.options_with_values and i + 1 < len(argv_texts):
            i += 1  # skip the option's value argument
        i += 1
    return result


def _collect_binary_format_spec_tokens(
    out: list[tuple[int, int, int, int, int]],
    spec_token: Token,
) -> bool:
    """Tokenise binary format/scan specifiers inside a format word."""
    if spec_token.type not in (TokenType.STR, TokenType.ESC):
        return False

    base_offset, base_line, base_col = token_content_base(spec_token)
    text = spec_token.text
    i = 0
    emitted = False

    while i < len(text):
        ch = text[i]
        if ch in " \t\r\n":
            i += 1
            continue

        count_start = i
        while i < len(text) and text[i].isdigit():
            i += 1
        if i > count_start:
            count_pos = position_from_relative(
                text,
                count_start,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=count_pos,
                text=text[count_start:i],
                type_idx=_TYPE_INDEX["binaryCount"],
            )
            emitted = True

        if i >= len(text):
            break

        spec = text[i]
        if spec not in _BINARY_FORMAT_SPECIFIERS:
            i += 1
            continue

        spec_pos = position_from_relative(
            text,
            i,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        _append_text_token(
            out,
            start=spec_pos,
            text=spec,
            type_idx=_TYPE_INDEX["binarySpec"],
        )
        emitted = True
        i += 1

        # Signed/unsigned modifier suffix (Tcl 8.5+): e.g. su, iu
        if (
            i < len(text)
            and text[i] in ("u", "s")
            and spec in _BINARY_INT_SPECIFIERS
            and active_dialect() not in ("tcl8.4", "f5")
        ):
            mod_pos = position_from_relative(
                text,
                i,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=mod_pos,
                text=text[i],
                type_idx=_TYPE_INDEX["binaryFlag"],
            )
            emitted = True
            i += 1

        if i < len(text) and text[i] == "*":
            star_pos = position_from_relative(
                text,
                i,
                base_line=base_line,
                base_col=base_col,
                base_offset=base_offset,
            )
            _append_text_token(
                out,
                start=star_pos,
                text="*",
                type_idx=_TYPE_INDEX["binaryFlag"],
            )
            emitted = True
            i += 1

    return emitted


def _recover_stray_close_bracket_in_flush(
    argv: list[Token],
    argv_texts: list[str],
    all_tokens_buf: list[Token],
    source: str,
    body_token: Token | None,
) -> None:
    """Merge tokens around a stray ``]`` into a virtual CMD for recovery.

    Mirrors the analyser's ``_recover_stray_close_bracket`` so that the
    semantic token provider sees the same argument structure.
    """
    base_off = (body_token.start.offset + 1) if body_token else 0

    # Step 1: Find an ESC token in argv containing ']' at its end.
    bracket_argv_idx = -1
    bracket_char_idx = -1
    for i, tok in enumerate(argv):
        if tok.type is not TokenType.ESC:
            continue
        idx = tok.text.find("]")
        if idx >= 0 and idx == len(tok.text) - 1:
            bracket_argv_idx = i
            bracket_char_idx = idx
            break
    if bracket_argv_idx <= 0:
        return  # not found, or is command name

    bracket_tok = argv[bracket_argv_idx]

    # Step 2: Scan backward through argv for a known command name.
    known = _known_commands_set()
    prefix = bracket_tok.text[:bracket_char_idx] if bracket_char_idx > 0 else ""

    cmd_start_argv_idx: int | None = None
    if prefix in known:
        cmd_start_argv_idx = bracket_argv_idx
    else:
        for i in range(bracket_argv_idx - 1, 0, -1):
            if argv[i].type is TokenType.ESC and argv[i].text in known:
                cmd_start_argv_idx = i
                break

    # Arity-based fallback: if the enclosing command has bounded max
    # arity and the argument count exceeds it, the missing [ should
    # go before the last expected argument position.
    if cmd_start_argv_idx is None or cmd_start_argv_idx <= 0:
        cmd_name = argv_texts[0] if argv_texts else ""
        validation = REGISTRY.validation(cmd_name)
        if validation is not None and not validation.arity.is_unlimited:
            max_args = validation.arity.max
            nargs = len(argv) - 1  # exclude command name
            if nargs > max_args >= 1:
                target_argv_idx = max_args  # argv[max_args] = last expected arg
                if target_argv_idx < len(argv):
                    target_tok = argv[target_argv_idx]
                    if target_tok.start.offset < bracket_tok.start.offset:
                        cmd_start_argv_idx = target_argv_idx

    if cmd_start_argv_idx is None or cmd_start_argv_idx <= 0:
        return

    # Step 3: Extract virtual CMD text from body source.
    start_tok = argv[cmd_start_argv_idx]
    local_src_start = start_tok.start.offset - base_off
    local_src_end = bracket_tok.start.offset + bracket_char_idx - base_off
    if local_src_start < 0 or local_src_end > len(source):
        return
    virtual_cmd_text = source[local_src_start:local_src_end]

    # Step 4: Build virtual CMD token.
    src_start = start_tok.start.offset
    virtual_cmd = Token(
        type=TokenType.CMD,
        text=virtual_cmd_text,
        start=SourcePosition(
            line=start_tok.start.line,
            character=max(start_tok.start.character - 1, 0),
            offset=src_start - 1,
        ),
        end=SourcePosition(
            line=bracket_tok.start.line,
            character=bracket_tok.start.character + bracket_char_idx - 1,
            offset=bracket_tok.start.offset + bracket_char_idx - 1,
        ),
    )

    # Step 5: Splice argv / argv_texts.
    argv[cmd_start_argv_idx : bracket_argv_idx + 1] = [virtual_cmd]
    argv_texts[cmd_start_argv_idx : bracket_argv_idx + 1] = [f"[{virtual_cmd_text}]"]

    # Step 6: Splice all_tokens_buf — find the matching range,
    # accounting for SEP tokens between the merged argv entries.
    start_all_idx: int | None = None
    end_all_idx: int | None = None
    for j, t in enumerate(all_tokens_buf):
        if start_all_idx is None and t.start.offset == start_tok.start.offset:
            start_all_idx = j
        if t.start.offset == bracket_tok.start.offset:
            end_all_idx = j

    if start_all_idx is not None and end_all_idx is not None:
        all_tokens_buf[start_all_idx : end_all_idx + 1] = [virtual_cmd]


# E101 recovery helpers for orphaned switch cases

_KNOWN_COMMANDS: frozenset[str] | None = None


def _known_commands_set() -> frozenset[str]:
    """Lazily build the set of known command names."""
    global _KNOWN_COMMANDS
    if _KNOWN_COMMANDS is None:
        _KNOWN_COMMANDS = known_command_names()
    return _KNOWN_COMMANDS


def _switch_is_form1_incomplete(
    argv_texts: list[str],
    argv: list[Token],
) -> bool:
    """Return True if the switch command is Form 1 (not compact Form 2).

    Form 2 is when the last non-option arg is a single STR token
    containing all pattern/body pairs.  Everything else is Form 1.
    """
    args = argv_texts[1:]
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "--":
            i += 1
            break
        if args[i] in ("-matchvar", "-indexvar"):
            i += 2
            continue
        i += 1
    # i is now the index of the string arg in args (0-based after cmd)
    i += 1  # skip string arg
    remaining = len(args) - i
    if remaining <= 0:
        # Switch has no body at all — definitely incomplete
        return True
    if remaining == 1:
        # Check if it's a STR (Form 2 compact body)
        tok_idx = i + 1  # +1 for cmd name in argv
        if tok_idx < len(argv) and argv[tok_idx].type is TokenType.STR:
            return False  # Form 2 — complete
    # Form 1 with explicit pairs — may have orphaned cases
    return True


def _looks_like_orphaned_switch_case(
    argv_texts: list[str],
    argv: list[Token],
) -> bool:
    """Return True if a command looks like an orphaned switch case.

    An orphaned case has 2 words: pattern + brace-body (or ``-`` for
    fall-through), and its "name" is not a known Tcl command.
    """
    if len(argv_texts) != 2:
        return False
    known = _known_commands_set()
    if argv_texts[0] in known:
        return False
    if len(argv) >= 2 and argv[-1].type is TokenType.STR:
        return True
    if argv_texts[-1] == "-":
        return True
    return False


def _emit_orphaned_switch_case(
    out: list[tuple[int, int, int, int, int]],
    argv: list[Token],
    all_tokens_buf: list[Token],
    regex_positions: frozenset[tuple[int, int]],
) -> None:
    """Emit semantic tokens for an orphaned switch case command.

    The pattern is emitted as a string token and the body is recursed
    into (instead of being emitted as a single string token).
    """
    pattern_emitted = False
    for tok in all_tokens_buf:
        if tok.type is TokenType.SEP:
            continue
        if not pattern_emitted:
            # First non-SEP token is the pattern — emit as string.
            if tok.type is TokenType.ESC and token_content_shift(tok) > 0:
                rendered = '"' + tok.text + '"'
            elif tok.type is TokenType.VAR:
                rendered = "$" + tok.text
            else:
                rendered = tok.text
            _append_text_token(
                out,
                start=tok.start,
                text=rendered,
                type_idx=_TYPE_INDEX["string"],
            )
            pattern_emitted = True
            continue
        # Subsequent tokens: body (STR → recurse) or other types.
        if tok.type is TokenType.STR and tok.text.strip():
            _collect_tokens(out, tok.text, body_token=tok, regex_positions=regex_positions)
        elif tok.type is TokenType.CMD:
            _collect_tokens(out, tok.text, body_token=tok, regex_positions=regex_positions)
        elif tok.type is TokenType.VAR:
            span = tok.end.offset - tok.start.offset + 1
            if span > len(tok.text) + 1:
                rendered = "${" + tok.text + "}"
            else:
                rendered = "$" + tok.text
            _append_text_token(
                out,
                start=tok.start,
                text=rendered,
                type_idx=_TYPE_INDEX["variable"],
            )


def _collect_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
    body_token: Token | None = None,
    regex_positions: frozenset[tuple[int, int]] = frozenset(),
) -> None:
    """Collect semantic tokens from *source* into *tokens*.

    Each entry is (line, char, length, type_idx, modifiers) -- absolute positions.
    These are sorted and delta-encoded by the caller.

    *regex_positions* is a frozenset of ``(line, character)`` tuples where the
    analyser has determined a token holds a regex pattern (e.g. a variable whose
    constant value flows into a ``regexp`` call).

    Recurses into CMD tokens (``[...]``) and into BODY arguments (braced
    bodies of proc, if, while, for, foreach, namespace eval, etc.).
    """
    # Compute virtual token insertions for error recovery (e.g. missing ]).
    # This injects zero-width virtual characters into the lexer so that
    # unterminated CMD tokens are properly terminated and downstream
    # token classification sees the correct argument structure.
    vi = compute_virtual_insertions(source, body_token) or None
    if body_token is not None:
        # Token start points to the delimiter ({ or [); the content
        # starts one character later, so add 1 to offset and col.
        lexer = TclLexer(
            source,
            base_offset=body_token.start.offset + 1,
            base_line=body_token.start.line,
            base_col=body_token.start.character + 1,
            virtual_insertions=vi,
        )
    else:
        lexer = TclLexer(source, virtual_insertions=vi)

    # We need to track commands so we can identify BODY arguments.
    # Collect tokens per command, then emit them.
    argv: list[Token] = []  # first token per argument
    argv_texts: list[str] = []  # concatenated text per argument
    all_tokens_buf: list[Token] = []  # all tokens in current command
    prev_type = TokenType.EOL

    # E101 recovery: when a switch is flushed in Form 1 (explicit
    # pattern/body pairs), subsequent commands that look like orphaned
    # cases should be emitted as switch-case tokens, not standalone
    # commands.
    switch_recovery_active = [False]

    def _flush_command() -> None:
        """Process a complete command's tokens with BODY awareness."""
        if not argv:
            return
        # Recovery: merge tokens around stray ']' (missing '[') into
        # a virtual CMD so downstream handlers see correct arg structure.
        _recover_stray_close_bracket_in_flush(
            argv,
            argv_texts,
            all_tokens_buf,
            source,
            body_token,
        )
        cmd_name = argv_texts[0]

        # E101 recovery: if a switch was just flushed (Form 1) and
        # this command looks like an orphaned case, emit as switch
        # case tokens (pattern = string, body = recurse) and stay in
        # recovery mode for further cases.
        if switch_recovery_active[0]:
            if _looks_like_orphaned_switch_case(argv_texts, argv):
                _emit_orphaned_switch_case(
                    tokens,
                    argv,
                    all_tokens_buf,
                    regex_positions,
                )
                return  # stay in switch recovery for next command
            switch_recovery_active[0] = False
        body_indices = _arg_indices(cmd_name, argv_texts[1:], ArgRole.BODY)
        expr_indices = _arg_indices(cmd_name, argv_texts[1:], ArgRole.EXPR)
        varname_indices = _arg_indices(cmd_name, argv_texts[1:], ArgRole.VAR_NAME) | _arg_indices(
            cmd_name, argv_texts[1:], ArgRole.VAR_READ
        )
        pattern_indices = _arg_indices(cmd_name, argv_texts[1:], ArgRole.PATTERN)
        param_arg_idx = _proc_param_list_arg_index(cmd_name, argv_texts)
        proc_name_arg_idx = _procedure_name_arg_index(cmd_name, argv_texts)
        binary_format_arg_idx = _binary_format_arg_index(cmd_name, argv_texts)
        subcommand_arg_idx = _subcommand_arg_index(cmd_name, argv_texts)
        sprintf_format_arg_idx = _sprintf_format_arg_index(cmd_name, argv_texts)
        clock_format_arg_idx = _clock_format_arg_index(cmd_name, argv_texts)
        regsub_subspec_arg_idx = _regsub_subspec_arg_index(cmd_name, argv_texts)
        string_map_arg_idx = _string_map_mapping_arg_index(cmd_name, argv_texts)
        glob_pattern_indices = _glob_pattern_arg_indices(cmd_name, argv_texts)
        option_indices = _option_arg_indices(cmd_name, argv_texts[1:])
        skip_body_indices: set[int] = set()
        if cmd_name == "switch":
            skip_body_indices = _collect_switch_case_bodies(
                tokens, argv_texts[1:], argv[1:], regex_positions=regex_positions
            )
            # Activate E101 recovery if the switch parsed as Form 1
            # (explicit pattern/body pairs, not a single braced body).
            # Subsequent case-like commands are likely orphaned.
            switch_recovery_active[0] = _switch_is_form1_incomplete(
                argv_texts,
                argv,
            )

        # Walk all tokens in this command and emit/recurse
        arg_idx = -1  # -1 = command name position, then 0, 1, 2...
        prev = TokenType.EOL
        for tok in all_tokens_buf:
            if tok.type is TokenType.SEP:
                prev = tok.type
                continue

            # Determine argument index
            if prev in (TokenType.SEP, TokenType.EOL):
                arg_idx += 1
            prev = tok.type

            is_cmd_name = arg_idx == 0

            # Check if this token is part of a BODY argument
            is_body = (arg_idx - 1) in body_indices and arg_idx > 0
            is_expr = (arg_idx - 1) in expr_indices and arg_idx > 0
            is_varname = (arg_idx - 1) in varname_indices and arg_idx > 0
            is_pattern = (arg_idx - 1) in pattern_indices and arg_idx > 0
            is_option = (arg_idx - 1) in option_indices and arg_idx > 0

            if tok.type is TokenType.CMD:
                # Always recurse into command substitutions
                _collect_tokens(tokens, tok.text, body_token=tok, regex_positions=regex_positions)
                continue

            # Skip tokens already processed by _collect_switch_case_bodies.
            if tok.type is TokenType.STR and arg_idx > 0 and (arg_idx - 1) in skip_body_indices:
                continue

            if tok.type is TokenType.STR and is_body and tok.text.strip():
                # This is a body argument -- recurse instead of emitting as string
                _collect_tokens(tokens, tok.text, body_token=tok, regex_positions=regex_positions)
                continue

            if tok.type is TokenType.STR and is_expr and tok.text.strip():
                _collect_expression_tokens(tokens, tok.text, tok, regex_positions=regex_positions)
                continue

            if (
                proc_name_arg_idx is not None
                and arg_idx == proc_name_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
            ):
                rendered_name = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=rendered_name,
                    type_idx=_TYPE_INDEX["function"],
                    modifiers=1 << _MOD_INDEX["definition"],
                )
                continue

            if (
                subcommand_arg_idx is not None
                and arg_idx == subcommand_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
                and _is_known_subcommand(cmd_name, argv_texts[arg_idx])
            ):
                rendered_sub = f"{{{tok.text}}}" if tok.type is TokenType.STR else tok.text
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=rendered_sub,
                    type_idx=_TYPE_INDEX["keyword"],
                    modifiers=1 << _MOD_INDEX["defaultLibrary"],
                )
                continue

            if (
                param_arg_idx is not None
                and arg_idx == param_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_param_list_tokens(tokens, tok):
                    continue

            if (
                binary_format_arg_idx is not None
                and arg_idx == binary_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_binary_format_spec_tokens(tokens, tok):
                    continue

            if (
                sprintf_format_arg_idx is not None
                and arg_idx == sprintf_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_sprintf_format_spec_tokens(tokens, tok):
                    continue

            if (
                clock_format_arg_idx is not None
                and arg_idx == clock_format_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_clock_format_spec_tokens(tokens, tok):
                    continue

            if (
                regsub_subspec_arg_idx is not None
                and arg_idx == regsub_subspec_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
            ):
                if _collect_regsub_subspec_tokens(tokens, tok):
                    continue

            # string map mapping list: alternate pair colours.
            if (
                string_map_arg_idx is not None
                and arg_idx == string_map_arg_idx
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type is TokenType.STR
            ):
                if _collect_string_map_pairs_tokens(tokens, tok):
                    continue

            # Glob pattern arguments (string match, glob, lsearch)
            # Must be checked before the generic PATTERN handler.
            if (
                arg_idx in glob_pattern_indices
                and arg_idx < len(argv)
                and tok is argv[arg_idx]
                and tok.type in (TokenType.ESC, TokenType.STR)
            ):
                if _collect_glob_pattern_tokens(tokens, tok):
                    continue

            # Variable name arguments (e.g. the "a" in "set a foo") get
            # highlighted as variables with the declaration modifier.
            if is_varname and tok.type is TokenType.ESC:
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["variable"],
                    modifiers=1 << _MOD_INDEX["declaration"],
                )
                continue

            # Regex pattern arguments (e.g. the pattern in "regexp {pat} str")
            # get highlighted as the "regexp" semantic token type.
            if is_pattern and tok.type in (TokenType.ESC, TokenType.STR):
                _emit_regex_token(tokens, tok)
                continue

            # Analysis-driven regex override: when the analyser has
            # determined that a token (typically a variable reference or
            # string literal) holds a regex pattern, highlight it as
            # ``regexp``.  The analyser records positions for both the
            # ``set`` value and the ``$var`` use-site.
            if regex_positions and (tok.start.line, tok.start.character) in regex_positions:
                # But don't override positional variables inside a format string!
                if not (sprintf_format_arg_idx is not None and arg_idx == sprintf_format_arg_idx):
                    rendered = "$" + tok.text if tok.type is TokenType.VAR else tok.text
                    if tok.type is TokenType.STR:
                        rendered = f"{{{tok.text}}}"
                    _append_text_token(
                        tokens,
                        start=tok.start,
                        text=rendered,
                        type_idx=_TYPE_INDEX["regexp"],
                    )
                    continue

            # iRule event name in ``when EVENT { body }``
            if (
                cmd_name == "when"
                and arg_idx == 1
                and tok.type is TokenType.ESC
                and _EVENT_RE.match(tok.text)
            ):
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["event"],
                )
                continue

            # Command options/flags known to the registry
            if is_option and tok.type is TokenType.ESC and tok.text.startswith("-"):
                _append_text_token(
                    tokens,
                    start=tok.start,
                    text=tok.text,
                    type_idx=_TYPE_INDEX["decorator"],
                )
                continue

            type_idx = _classify_token(tok.type, tok.text, is_command_name=is_cmd_name)
            if type_idx is None:
                continue

            # defaultLibrary modifier for built-in command keywords
            modifiers = 0
            if is_cmd_name and type_idx == _TYPE_INDEX["keyword"]:
                modifiers = 1 << _MOD_INDEX["defaultLibrary"]

            # Reconstruct the full source representation so
            # len(rendered) matches the source span.
            if tok.type is TokenType.VAR:
                span = tok.end.offset - tok.start.offset + 1
                if span > len(tok.text) + 1:  # +1 for '$'
                    rendered = "${" + tok.text + "}"  # brace-delimited
                else:
                    rendered = "$" + tok.text
            elif tok.type is TokenType.ESC and token_content_shift(tok) > 0:
                # Leading quote is always present.  A trailing quote only
                # exists when the token is NOT still inside a quoted string
                # (i.e. the token completed the quoted word).
                if tok.in_quote:
                    rendered = '"' + tok.text
                else:
                    rendered = '"' + tok.text + '"'
            else:
                rendered = tok.text

            # Namespace-qualified command names: split into namespace + command
            # (but never split comment tokens — they should remain atomic)
            if is_cmd_name and "::" in tok.text and tok.type is not TokenType.COMMENT:
                _emit_namespace_qualified(tokens, tok, type_idx, modifiers)
                continue

            # Escape sequences in string-classified ESC tokens
            if type_idx == _TYPE_INDEX["string"] and tok.type is TokenType.ESC and "\\" in tok.text:
                if _emit_string_with_escapes(tokens, tok):
                    continue

            _append_text_token(
                tokens,
                start=tok.start,
                text=rendered,
                type_idx=type_idx,
                modifiers=modifiers,
            )

    while True:
        tok = lexer.get_token()
        if tok is None:
            break

        match tok.type:
            case TokenType.SEP:
                all_tokens_buf.append(tok)
                prev_type = tok.type
                continue
            case TokenType.EOL:
                _flush_command()
                argv = []
                argv_texts = []
                all_tokens_buf = []
                prev_type = tok.type
                continue
            case _:
                pass

        all_tokens_buf.append(tok)

        # Build argv for command identification
        text = tok.text
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
            argv_texts.append(text)
        else:
            if argv_texts:
                argv_texts[-1] += text
            else:
                argv.append(tok)
                argv_texts.append(text)

        prev_type = tok.type

    # Handle trailing command without final EOL
    _flush_command()


def semantic_tokens_full(
    source: str,
    analysis: AnalysisResult | None = None,
    *,
    is_bigip_conf: bool = False,
    is_irules: bool = False,
    chunk_token_cache: list[list[tuple[int, int, int, int, int]] | None] | None = None,
    chunk_line_ranges: list[tuple[int, int, int, int]] | None = None,
) -> list[int]:
    """Produce the flat list of 5-int encoded semantic tokens for the source.

    When *analysis* is provided, regex variable positions identified by the
    analyser are highlighted as ``regexp`` tokens instead of their normal type.

    When *chunk_token_cache* and *chunk_line_ranges* are provided, cached
    absolute tokens are reused for chunks that have them, and only dirty
    chunks are extracted from a full token computation.  The cache entries
    are updated in-place so that future calls benefit from the cache.
    """
    regex_positions: frozenset[tuple[int, int]] = frozenset()
    if analysis is not None:
        regex_positions = frozenset(
            (rp.range.start.line, rp.range.start.character) for rp in analysis.regex_patterns
        )

    # Check if we have a full cache hit.
    if chunk_token_cache is not None and chunk_line_ranges is not None:
        if all(entry is not None for entry in chunk_token_cache):
            # Full cache hit — assemble from cached absolute tokens.
            raw_tokens: list[tuple[int, int, int, int, int]] = []
            for entry in chunk_token_cache:
                assert entry is not None
                raw_tokens.extend(entry)
            return _delta_encode(raw_tokens)

    # Collect all tokens with absolute positions
    raw_tokens: list[tuple[int, int, int, int, int]] = []
    base_tokens: list[tuple[int, int, int, int, int]] = []
    _collect_tokens(base_tokens, source, regex_positions=regex_positions)
    if is_bigip_conf:
        # Run full Tcl tokenisation on embedded iRule/iApp bodies.
        embedded_tokens: list[tuple[int, int, int, int, int]] = []
        body_ranges = _collect_embedded_tcl_tokens(
            embedded_tokens, source, regex_positions=regex_positions
        )
        if body_ranges:
            # Remove tokens from the whole-file Tcl pass that overlap
            # with embedded body ranges; the body-specific tokens are
            # richer.  BIG-IP overlay tokens are added separately.
            base_tokens = [
                tok
                for tok in base_tokens
                if not any(start <= tok[0] <= end for start, end in body_ranges)
            ]
        raw_tokens.extend(base_tokens)
        raw_tokens.extend(embedded_tokens)
        _collect_bigip_tokens(raw_tokens, source)
        _collect_bigip_embedded_irules_object_tokens(raw_tokens, source)
    else:
        raw_tokens.extend(base_tokens)
    if is_irules:
        _collect_irules_object_tokens(raw_tokens, source)

    # Sort by position (line, then character) for correct delta encoding
    raw_tokens.sort(key=lambda t: (t[0], t[1]))

    # Populate chunk cache if provided.
    # Use (line, col) boundaries so chunks sharing a line get non-overlapping
    # token sets (e.g. semicolon-separated commands on the same line).
    if chunk_token_cache is not None and chunk_line_ranges is not None:
        for i, (sl, sc, el, ec) in enumerate(chunk_line_ranges):
            if i < len(chunk_token_cache) and chunk_token_cache[i] is None:
                chunk_token_cache[i] = [
                    tok for tok in raw_tokens if (sl, sc) <= (tok[0], tok[1]) < (el, ec)
                ]

    return _delta_encode(raw_tokens)


def _delta_encode(raw_tokens: list[tuple[int, int, int, int, int]]) -> list[int]:
    """Convert absolute-position tokens to LSP delta-encoded format."""
    # Sort by position (line, then character) for correct delta encoding
    raw_tokens.sort(key=lambda t: (t[0], t[1]))

    data: list[int] = []
    prev_line = 0
    prev_char = 0

    for line, char, length, type_idx, modifiers in raw_tokens:
        delta_line = line - prev_line
        delta_char = char - prev_char if delta_line == 0 else char

        data.extend([delta_line, delta_char, length, type_idx, modifiers])
        prev_line = line
        prev_char = char

    return data
