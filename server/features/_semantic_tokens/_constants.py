from __future__ import annotations

import logging
import re

from compiler.registry.command_registry import REGISTRY

log = logging.getLogger(__name__)

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
    # APL (iApp presentation language) token types
    "aplSection",  # 43 – section/text/table/row keywords
    "aplFieldType",  # 44 – field type keywords (string, choice, …)
    "aplAttribute",  # 45 – attributes (default, display, required, validator)
    "aplSectionName",  # 46 – name following section/text/table/row
    "aplFieldName",  # 47 – name following a field-type keyword
    "aplDefine",  # 48 – define keyword
    "aplDefineName",  # 49 – name after define
    "aplDirective",  # 50 – #include, #inline
    "aplOptional",  # 51 – optional keyword
    "aplValidator",  # 52 – validator value (IpAddress, PortNumber, …)
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
# Bare tail names (e.g. "test" from "tcltest::test") are included for
# packages that are conventionally imported unqualified via
# ``namespace import ::pkg::*``.  Only packages in this allowlist get
# tail-name promotion to avoid false-positive keyword highlights.
_REGISTRY_NAMES = frozenset(REGISTRY.command_names())
_IMPORTED_UNQUALIFIED_NAMESPACES = frozenset({"tcltest", "msgcat"})
_KEYWORD_TAILS = frozenset(
    name.rsplit("::", 1)[-1]
    for name in _REGISTRY_NAMES
    if "::" in name and name.rsplit("::", 1)[0] in _IMPORTED_UNQUALIFIED_NAMESPACES
)

# All known built-in commands (registry + imported tails).
# Used to decide whether a command name gets the ``defaultLibrary`` modifier.
_BUILTIN_COMMANDS = _REGISTRY_NAMES | _KEYWORD_TAILS

# Language keywords — control-flow, definition syntax, and TclOO framework
# words.  These are the commands the TextMate grammar classifies as
# ``keyword.control.tcl`` or ``keyword.other.tcl``.  Everything else in
# the registry is a built-in *function* (``support.function.tcl`` in the
# TM grammar) and should get ``function`` + ``defaultLibrary`` from the LSP.
#
# Built from ``is_language_keyword`` on CommandSpec, plus sub-keywords that
# aren't standalone commands (else, elseif, on, trap, finally, TclOO
# definition-context words like method/constructor/destructor/forward etc.).
_LANGUAGE_KEYWORD_SUB_KEYWORDS = frozenset(
    {
        # Sub-keywords of if/try/switch — not standalone commands
        "else",
        "elseif",
        "on",
        "trap",
        "finally",
        # TclOO definition-context keywords without standalone CommandSpec
        "method",
        "constructor",
        "destructor",
        "forward",
        "mixin",
        "filter",
        "superclass",
        "renamemethod",
        "deletemethod",
        "export",
        "unexport",
        # TclOO definition-context keywords (9.0+)
        "classmethod",
        "definitionnamespace",
        "initialise",
        "initialize",
        "private",
        "property",
        # TclOO method-body keywords without standalone CommandSpec
        "callback",
        "mymethod",
        "link",
    }
)
_LANGUAGE_KEYWORDS = REGISTRY.language_keyword_commands() | _LANGUAGE_KEYWORD_SUB_KEYWORDS

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
