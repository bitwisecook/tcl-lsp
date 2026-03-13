"""Locate and extract ``ltm rule`` / ``gtm rule`` blocks in BIG-IP configs.

Used by the LSP code-action and VS Code command to open an embedded iRule
in a scratch editor with the ``f5-irules`` dialect, and to write changes
back to the original configuration file.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from ..analysis.semantic_model import Range
from ..common.source_map import SourceMap

# Match:  ltm rule /Common/name {  or  gtm rule /Common/name {
_RULE_HEADER_RE = re.compile(
    r"^((?:ltm|gtm)\s+rule\s+(/[\w/.-]+))\s*\{",
    re.MULTILINE,
)


@dataclass(frozen=True, slots=True)
class EmbeddedRule:
    """An iRule block found inside a BIG-IP configuration file."""

    full_path: str  # e.g. "/Common/my_irule"
    name: str  # e.g. "my_irule"
    header: str  # e.g. "ltm rule /Common/my_irule"
    body: str  # the Tcl source between the outermost { }
    body_start_offset: int  # offset of the first char after the opening {
    body_end_offset: int  # offset of the last char before the closing }
    block_start_offset: int  # offset of the opening {
    block_end_offset: int  # offset of the closing } + 1
    range: Range  # range covering the entire stanza


def find_embedded_rules(source: str) -> list[EmbeddedRule]:
    """Find all ``ltm rule`` and ``gtm rule`` blocks in *source*."""
    source_map = SourceMap(source)
    rules: list[EmbeddedRule] = []
    for m in _RULE_HEADER_RE.finditer(source):
        header = m.group(1)
        full_path = m.group(2)
        name = full_path.rsplit("/", 1)[-1]
        brace_pos = m.end() - 1  # position of '{'
        pos = brace_pos + 1
        depth = 1
        while pos < len(source) and depth > 0:
            ch = source[pos]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            elif ch == "\\":
                pos += 1  # skip escaped char
            pos += 1
        body = source[brace_pos + 1 : pos - 1]
        rules.append(
            EmbeddedRule(
                full_path=full_path,
                name=name,
                header=header,
                body=body,
                body_start_offset=brace_pos + 1,
                body_end_offset=pos - 1,
                block_start_offset=m.start(),
                block_end_offset=pos,
                range=Range(
                    start=source_map.offset_to_position(m.start()),
                    end=source_map.offset_to_position(pos - 1),
                ),
            )
        )
    return rules


def find_rule_at_offset(source: str, offset: int) -> EmbeddedRule | None:
    """Return the ``EmbeddedRule`` whose block contains *offset*, or ``None``."""
    for rule in find_embedded_rules(source):
        if rule.block_start_offset <= offset <= rule.block_end_offset:
            return rule
    return None


def replace_rule_body(source: str, rule: EmbeddedRule, new_body: str) -> str:
    """Return *source* with the body of *rule* replaced by *new_body*."""
    return source[: rule.body_start_offset] + new_body + source[rule.body_end_offset :]
