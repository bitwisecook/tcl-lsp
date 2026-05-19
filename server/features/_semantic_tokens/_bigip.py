from __future__ import annotations

import logging
import re
from bisect import bisect_right

from dialects.f5.bigip.apl_parser import AplTokenKind, tokenise_apl
from dialects.f5.bigip.iapp_extract import find_embedded_iapp_sections
from dialects.f5.bigip.irules_refs import extract_irules_object_references
from dialects.f5.bigip.parser._helpers import (
    _extract_blocks,
    _parse_generic_header,
    _parse_properties_with_spans,
)
from dialects.f5.bigip.registry import references_via_spec
from dialects.f5.bigip.registry.pilot import pilot_property_spec_for
from dialects.f5.bigip.rule_extract import find_embedded_rules
from shared.document_buffer import DocumentBuffer
from shared.tokens import SourcePosition, Token, TokenType

from ._collect import _collect_tokens
from ._constants import (
    _TYPE_INDEX,
)

log = logging.getLogger(__name__)

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


def _collect_registry_property_ref_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
) -> None:
    """Emit ``object``-type semantic tokens for every reference the
    registry's value-spec dispatch surfaces — exact byte spans on
    each property's value (no per-line regex), including nested
    references inside keyed-block lists (firewall rule destination
    address-lists, cert-key-chain cert refs, profile attachments,
    monitor expression entries, ...).

    Falls through harmlessly for unmigrated properties: the legacy
    line-based ``_collect_bigip_tokens`` collector still runs and
    covers them, with the ``seen`` dedup set making sure overlapping
    coverage doesn't double-emit tokens.
    """
    seen: set[tuple[int, int, int, int, int]] = set()
    buffer = DocumentBuffer.from_source(source)
    for block in _extract_blocks(source):
        generic = _parse_generic_header(block.header)
        if generic is None:
            continue
        module, object_type, identifier = generic
        body_base = block.start_offset + 1
        prop_map = _parse_properties_with_spans(block.body)
        for key, prop in prop_map.items():
            if pilot_property_spec_for(module, object_type, key) is None:
                continue
            if prop.value_start is None:
                continue
            base = body_base + prop.value_start
            refs = references_via_spec(
                module=module,
                object_type=object_type,
                property_name=key,
                value=prop.value,
                owner_path=identifier,
                base_offset=base,
                source_text=source,
            )
            for ref in refs or ():
                if ref.range is None:
                    continue
                # ``ref.range`` is half-open ``[start, end)``;
                # ``range_from_offsets`` is inclusive on the end so
                # convert at the boundary.  Token length is then the
                # inclusive-character difference (no extra ``+1``)
                # because both endpoints are inclusive in ``rng``.
                rng = buffer.range_from_offsets(ref.range.start, ref.range.end - 1)
                if rng.start.line != rng.end.line:
                    continue
                _append_bigip_token(
                    tokens,
                    seen,
                    line=rng.start.line,
                    char=rng.start.character,
                    length=rng.end.character - rng.start.character + 1,
                    type_name="object",
                )


def _collect_bigip_embedded_irules_object_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
) -> None:
    """Collect semantic tokens for object refs inside embedded ``ltm rule`` bodies."""
    seen: set[tuple[int, int, int, int, int]] = set()
    buf = DocumentBuffer.from_source(source)
    for rule in find_embedded_rules(source):
        rule_module = "gtm" if rule.header.startswith("gtm ") else "ltm"
        body_start = buf.offset_to_position(rule.body_start_offset - 1)
        body_end = buf.offset_to_position(max(rule.body_end_offset - 1, rule.body_start_offset - 1))
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


# Map APL token kinds to semantic token type names.
_APL_KIND_TO_TYPE: dict[AplTokenKind, str] = {
    AplTokenKind.COMMENT: "comment",
    AplTokenKind.DIRECTIVE: "aplDirective",
    AplTokenKind.SECTION_KW: "aplSection",
    AplTokenKind.FIELD_TYPE: "aplFieldType",
    AplTokenKind.DEFINE: "aplDefine",
    AplTokenKind.DEFINE_NAME: "aplDefineName",
    AplTokenKind.OPTIONAL: "aplOptional",
    AplTokenKind.ATTRIBUTE: "aplAttribute",
    AplTokenKind.SECTION_NAME: "aplSectionName",
    AplTokenKind.FIELD_NAME: "aplFieldName",
    AplTokenKind.VARIABLE: "variable",
    AplTokenKind.STRING: "string",
    AplTokenKind.NUMBER: "number",
    AplTokenKind.OPERATOR: "operator",
    AplTokenKind.ESCAPE: "escape",
    AplTokenKind.VALIDATOR_VALUE: "aplValidator",
}


def _find_apl_embedded_tcl(source: str) -> list[tuple[int, int, int, str]]:
    """Find ``[...]`` embedded Tcl regions in APL source.

    Returns a list of ``(start_line, start_char, end_offset, body)`` tuples
    for each top-level bracket expression found outside comments and strings.

    Handles multi-line ``[...]`` expressions that span across lines.
    """
    regions: list[tuple[int, int, int, str]] = []
    lines = source.split("\n")

    # Build a line-offset table for converting absolute offsets
    line_offsets: list[int] = []
    off = 0
    for ln in lines:
        line_offsets.append(off)
        off += len(ln) + 1

    # Scan character-by-character through the entire source
    pos = 0
    length = len(source)
    in_comment = False
    in_string = False

    while pos < length:
        ch = source[pos]

        # Track newlines — check if new line is a comment
        if ch == "\n":
            in_comment = False
            pos += 1
            # Check if next line is a comment
            if pos < length:
                rest = source[pos:].lstrip(" \t")
                if (
                    rest.startswith("#")
                    and not rest.startswith("#include")
                    and not rest.startswith("#inline")
                ):
                    in_comment = True
            continue

        if in_comment:
            pos += 1
            continue

        # Handle escapes
        if ch == "\\" and pos + 1 < length:
            pos += 2
            continue

        # Handle strings
        if ch == '"':
            in_string = not in_string
            pos += 1
            continue

        if ch == "[" and not in_string:
            # Find matching ] (may span multiple lines)
            depth = 1
            start_pos = pos
            pos += 1
            while pos < length and depth > 0:
                c = source[pos]
                if c == "\\":
                    pos += 1  # skip escaped char
                elif c == "[":
                    depth += 1
                elif c == "]":
                    depth -= 1
                pos += 1
            if depth == 0:
                body = source[start_pos + 1 : pos - 1]
                if body.strip():
                    # Determine start line and column from absolute offset
                    start_line = bisect_right(line_offsets, start_pos) - 1
                    start_col = start_pos - line_offsets[start_line] + 1  # +1 past '['
                    regions.append((start_line, start_col, pos, body))
            continue

        pos += 1

    return regions


def _collect_apl_tokens(
    tokens: list[tuple[int, int, int, int, int]],
    source: str,
) -> None:
    """Collect APL-specific semantic tokens from presentation source."""
    seen: set[tuple[int, int, int, int, int]] = set()
    for apl_tok in tokenise_apl(source):
        type_name = _APL_KIND_TO_TYPE.get(apl_tok.kind)
        if type_name is None:
            continue
        _append_bigip_token(
            tokens,
            seen,
            line=apl_tok.line,
            char=apl_tok.char,
            length=apl_tok.length,
            type_name=type_name,
        )

    # Embedded Tcl inside [...] brackets (e.g. [tmsh::create ...])
    for region_line, region_char, _end_offset, body in _find_apl_embedded_tcl(source):
        body_token = Token(
            type=TokenType.STR,
            text=body,
            start=SourcePosition(line=region_line, character=region_char, offset=0),
            end=SourcePosition(
                line=region_line, character=region_char + len(body), offset=len(body)
            ),
        )
        _collect_tokens(tokens, body, body_token=body_token)


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
    buf = DocumentBuffer.from_source(source)
    body_ranges: list[tuple[int, int]] = []

    # Embedded iRules (ltm rule / gtm rule)
    for rule in find_embedded_rules(source):
        body_start = buf.offset_to_position(rule.body_start_offset - 1)
        body_end = buf.offset_to_position(max(rule.body_end_offset - 1, rule.body_start_offset - 1))
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

    # Embedded iApp sections (implementation / presentation)
    for section in find_embedded_iapp_sections(source):
        sec_start = buf.offset_to_position(section.body_start_offset - 1)
        sec_end = buf.offset_to_position(
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
