"""Parser for F5 iApp APL (Application Presentation Language) files.

APL is a domain-specific language used to define the presentation layer
(user-facing forms) of iApp templates on BIG-IP.  It describes sections,
field types (string, choice, password, etc.), text labels, tables, and
reusable ``define`` blocks.

This module provides lightweight tokenisation for semantic highlighting
rather than a full AST — matching the approach used for bigip.conf.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum, auto


class AplTokenKind(Enum):
    """Semantic classification of an APL source token."""

    COMMENT = auto()
    DIRECTIVE = auto()  # #include, #inline
    SECTION_KW = auto()  # section, text, table, row
    FIELD_TYPE = auto()  # string, choice, editchoice, password, …
    DEFINE = auto()  # define keyword
    DEFINE_NAME = auto()  # name after define
    OPTIONAL = auto()  # optional keyword
    ATTRIBUTE = auto()  # default, display, required, validator
    SECTION_NAME = auto()  # name following section/text/table/row
    FIELD_NAME = auto()  # name following a field-type keyword
    VARIABLE = auto()  # $var, ${var}
    STRING = auto()  # "…" quoted string
    NUMBER = auto()  # numeric literal
    OPERATOR = auto()  # =>, arithmetic/logical ops
    ESCAPE = auto()  # backslash escape inside string
    VALIDATOR_VALUE = auto()  # validator name inside quotes


@dataclass(frozen=True, slots=True)
class AplToken:
    """A single semantic token from APL source."""

    line: int
    char: int
    length: int
    kind: AplTokenKind


# APL control keywords that introduce named blocks.
_SECTION_KEYWORDS = frozenset({"section", "text", "table", "row"})

# APL field-type keywords.
_FIELD_TYPE_KEYWORDS = frozenset(
    {
        "string",
        "choice",
        "editchoice",
        "multichoice",
        "message",
        "password",
        "yesno",
        "noyes",
        "enadis",
        "enadisdry",
        "disena",
        "indefint",
        "falsetrue",
        "truefalse",
        "tcpprof",
        "addrport",
    }
)

# APL attribute keywords (modifiers on field declarations).
_ATTRIBUTE_KEYWORDS = frozenset(
    {
        "default",
        "display",
        "required",
        "validator",
    }
)

# Known validator names.
_VALIDATOR_NAMES = frozenset(
    {
        "Number",
        "NonNegativeNumber",
        "IpAddress",
        "PortNumber",
        "IpOrFqdn",
        "FQDN",
    }
)

# Regex patterns
_COMMENT_RE = re.compile(r"(?:^|(?<=;))\s*#.*")
_DIRECTIVE_RE = re.compile(r"(?:^|(?<=[\[{;]))\s*(#include|#inline)\b")
_DEFINE_RE = re.compile(r"^\s*(define)\s+(\S+)")
_OPTIONAL_RE = re.compile(r"\b(optional)\s*\(")
_SECTION_KW_RE = re.compile(r"(?:^|(?<=[\s{;]))(section|text|table|row)\s+(\S+)")
_FIELD_TYPE_RE = re.compile(
    r"(?:^|(?<=[\s{;]))"
    r"(" + "|".join(sorted(_FIELD_TYPE_KEYWORDS)) + r")"
    r"\s+(\S+)"
)
_ATTRIBUTE_RE = re.compile(r"\b(default|display|required|validator)\b")
_VARIABLE_RE = re.compile(r"\$(?:[a-zA-Z0-9_]|::)+(?:\([^)]+\))?|\$\{[^}]*\}")
_STRING_RE = re.compile(r'"(?:[^"\\]|\\.)*"')
_NUMBER_RE = re.compile(r"(?<![a-zA-Z])(?:0x[0-9a-fA-F]+|[+-]?(?:[0-9]*\.)?[0-9]+f?)(?![.a-zA-Z])")
_OPERATOR_RE = re.compile(r"=>")
_ESCAPE_IN_STRING_RE = re.compile(r"\\(?:[0-7]{1,3}|x[a-fA-F0-9]+|u[a-fA-F0-9]{1,4}|.|\n)")
_VALIDATOR_VALUE_RE = re.compile(
    r'\bvalidator\s+"(' + "|".join(re.escape(v) for v in _VALIDATOR_NAMES) + r')"'
)


def tokenise_apl(source: str) -> list[AplToken]:
    """Tokenise APL source into semantic tokens for highlighting."""
    tokens: list[AplToken] = []
    lines = source.split("\n")

    for line_no, line in enumerate(lines):
        stripped = line.lstrip()
        if not stripped:
            continue

        # Comments: lines starting with #  (but not #include / #inline)
        if stripped.startswith("#"):
            if not stripped.startswith("#include") and not stripped.startswith("#inline"):
                indent = len(line) - len(stripped)
                tokens.append(AplToken(line_no, indent, len(stripped), AplTokenKind.COMMENT))
                continue

        # Directives: #include, #inline
        for m in _DIRECTIVE_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(1), len(m.group(1)), AplTokenKind.DIRECTIVE))

        # define <name>
        m = _DEFINE_RE.match(line)
        if m:
            tokens.append(AplToken(line_no, m.start(1), len(m.group(1)), AplTokenKind.DEFINE))
            tokens.append(AplToken(line_no, m.start(2), len(m.group(2)), AplTokenKind.DEFINE_NAME))

        # optional ( ... )
        for m in _OPTIONAL_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(1), len(m.group(1)), AplTokenKind.OPTIONAL))

        # Section keywords: section, text, table, row  + name
        for m in _SECTION_KW_RE.finditer(line):
            kw = m.group(1)
            name = m.group(2)
            tokens.append(AplToken(line_no, m.start(1), len(kw), AplTokenKind.SECTION_KW))
            # Don't emit name token for text if name is a quoted locale string
            if name and not name.startswith("{"):
                tokens.append(
                    AplToken(
                        line_no,
                        m.start(2),
                        len(name),
                        AplTokenKind.SECTION_NAME,
                    )
                )

        # Field types: string, choice, etc.  + name
        for m in _FIELD_TYPE_RE.finditer(line):
            kw = m.group(1)
            name = m.group(2)
            tokens.append(AplToken(line_no, m.start(1), len(kw), AplTokenKind.FIELD_TYPE))
            # Name following the field type (skip if it's a known attribute)
            if name and name not in _ATTRIBUTE_KEYWORDS and not name.startswith("{"):
                tokens.append(
                    AplToken(
                        line_no,
                        m.start(2),
                        len(name),
                        AplTokenKind.FIELD_NAME,
                    )
                )

        # Attributes: default, display, required, validator
        for m in _ATTRIBUTE_RE.finditer(line):
            # Avoid double-matching if the attribute was already captured
            # as part of a field-type match
            tokens.append(AplToken(line_no, m.start(1), len(m.group(1)), AplTokenKind.ATTRIBUTE))

        # Validator values
        for m in _VALIDATOR_VALUE_RE.finditer(line):
            tokens.append(
                AplToken(
                    line_no,
                    m.start(1),
                    len(m.group(1)),
                    AplTokenKind.VALIDATOR_VALUE,
                )
            )

        # Operator: =>
        for m in _OPERATOR_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(), m.end() - m.start(), AplTokenKind.OPERATOR))

        # Variables: $var, ${var}
        for m in _VARIABLE_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(), m.end() - m.start(), AplTokenKind.VARIABLE))

        # Strings (with embedded escapes)
        for m in _STRING_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(), m.end() - m.start(), AplTokenKind.STRING))
            # Find escape sequences within the string
            for esc in _ESCAPE_IN_STRING_RE.finditer(m.group()):
                tokens.append(
                    AplToken(
                        line_no,
                        m.start() + esc.start(),
                        esc.end() - esc.start(),
                        AplTokenKind.ESCAPE,
                    )
                )

        # Numbers
        for m in _NUMBER_RE.finditer(line):
            tokens.append(AplToken(line_no, m.start(), m.end() - m.start(), AplTokenKind.NUMBER))

    return tokens
