"""iRules-specific command checks that require event context.

These checks run only when the analyser/compiler knows which ``when EVENT``
block the command is inside (via the ``event`` parameter from
``run_all_checks``).  All checks are gated on ``active_dialect() == "f5-irules"``.

Diagnostic codes
================

- **IRULE1001** (WARNING): command invalid/ineffective in this event
- **IRULE1003** (WARNING): deprecated iRules event
- **IRULE1004** (HINT): ``when`` block missing explicit ``priority``
- **IRULE2001** (WARNING): deprecated ``matchclass`` — use ``class match``
- **IRULE2101** (HINT): heavy regex in a hot event (HTTP_REQUEST/RESPONSE)
- **IRULE4001** (WARNING): write to ``static::`` variable outside RULE_INIT
- **IRULE4002** (HINT): generic ``static::`` variable name — collision likely
- **IRULE4003** (HINT): variable scoping concern across events
- **IRULE5001** (HINT): ungated ``log`` in a high-frequency event
- **IRULE6001** (WARNING): global namespace variable (``::var`` or implicit global in RULE_INIT) causes TMM pinning
"""

from __future__ import annotations

import re

from shared.codes import diag
from shared.dialect import active_dialect
from shared.ranges import range_from_token

from ..commands.registry import REGISTRY
from ..commands.registry.namespace_data import (
    deprecated_events,
    hot_events,
    missing_requirements_description,
    profile_info_description,
)
from ..commands.registry.namespace_models import EventRequires
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..commands.registry.runtime import variable_writing_commands
from ..parsing.tokens import Token
from .semantic_model import CodeFix, Diagnostic, Range, Severity

# Derived from registry EventProps metadata — not hardcoded.
_HOT_EVENTS: frozenset[str] = hot_events()


def _valid_events_for_command(cmd_name: str, dialect: str) -> list[str]:
    """Return events where *cmd_name* is legal, via the legality matrix."""
    legality = REGISTRY.command_legality(dialect)
    return legality.events_for_command(cmd_name)


def _raw_arg_text(source: str, tok: Token) -> str:
    """Return the raw source slice for an argument token.

    Widens the end offset by one when the next character is the
    matching closing delimiter of an opening ``"``/``{`` so quoted /
    braced args round-trip intact.  Used by quick-fix builders that
    must preserve variable and command substitutions in the original
    syntax (rather than re-emitting the post-substitution value via
    ``args[idx]``).
    """
    start = tok.start.offset
    end = tok.end.offset + 1
    if 0 <= start < len(source) and source[start] in ('"', "{"):
        close = '"' if source[start] == '"' else "}"
        if end < len(source) and source[end] == close:
            end += 1
    return source[start:end]


# IRULE1001: Command invalid/ineffective in this event


@diag("IRULE1001", "Command invalid or ineffective in this iRules event.", section="irules")
def check_command_event_validity(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
    file_profiles: frozenset[str] = frozenset(),
) -> list[Diagnostic]:
    """IRULE1001: Warn when a command is used in an event where it's invalid."""
    dialect = active_dialect()
    if dialect != "f5-irules":
        return []

    spec = REGISTRY.get(cmd_name, dialect)
    if spec is None:
        return []

    # Fast path: use the pre-computed legality matrix for O(1) lookup.
    # If the command is legal AND has no profile-info to report, skip
    # the more expensive per-spec checks entirely.
    legality = REGISTRY.command_legality(dialect)
    if legality.is_legal(event, cmd_name):
        # Still check for informational profile hints.
        if "::" in cmd_name:
            prefix = cmd_name.split("::", 1)[0]
            ns_spec = EVENT_REGISTRY.get_protocol_namespace(prefix)
            if ns_spec is not None and ns_spec.profiles:
                ns_profile_info = profile_info_description(
                    EventRequires(profiles=ns_spec.profiles),
                    file_profiles,
                )
                if ns_profile_info is not None:
                    return [
                        Diagnostic(
                            range=range_from_token(all_tokens[0]),
                            message=f"'{cmd_name}' {ns_profile_info}.",
                            severity=Severity.HINT,
                            code="IRULE1001",
                        )
                    ]
        if spec.event_requires is not None:
            profile_info = profile_info_description(spec.event_requires, file_profiles)
            if profile_info is not None:
                return [
                    Diagnostic(
                        range=range_from_token(all_tokens[0]),
                        message=f"'{cmd_name}' {profile_info}.",
                        severity=Severity.HINT,
                        code="IRULE1001",
                    )
                ]
        return []

    # Illegal — produce detailed diagnostic.

    # excluded_events: command is explicitly forbidden in this event.
    if spec.excluded_events and event in spec.excluded_events:
        valid_events = _valid_events_for_command(cmd_name, dialect)
        hint = ""
        if valid_events:
            hint = f" Available in: {', '.join(sorted(valid_events)[:5])}"
            if len(valid_events) > 5:
                hint += f" (+{len(valid_events) - 5} more)"
            hint += "."
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=f"'{cmd_name}' cannot be used in {event}.{hint}",
                severity=Severity.WARNING,
                code="IRULE1001",
            )
        ]

    # Layer-based requirements: produce detailed reason.
    if spec.event_requires is not None:
        event_props = EVENT_REGISTRY.get_props(event)
        if event_props is None:
            # Unknown event — legality matrix already flagged it as
            # illegal; emit a generic warning.
            return [
                Diagnostic(
                    range=range_from_token(all_tokens[0]),
                    message=(f"'{cmd_name}' may not work in {event}."),
                    severity=Severity.WARNING,
                    code="IRULE1001",
                )
            ]

        requires = spec.event_requires
        desc = missing_requirements_description(event, event_props, requires)
        return [
            Diagnostic(
                range=range_from_token(all_tokens[0]),
                message=(f"'{cmd_name}' may not work in {event}: {desc}."),
                severity=Severity.WARNING,
                code="IRULE1001",
            )
        ]

    return []


# IRULE2001: Deprecated matchclass → class match


@diag(
    code="IRULE2001",
    description="Deprecated `matchclass` — use `class match` instead.",
    section="irules",
    ai_category="style",
    conversion_label="Deprecated matchclass -> class match",
)
def check_matchclass(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE2001: Deprecated ``matchclass`` — suggest ``class match``."""
    if cmd_name != "matchclass":
        return []
    if active_dialect() != "f5-irules":
        return []

    fixes: tuple[CodeFix, ...] = ()
    # Auto-fix: matchclass <item> <class> → class match <item> equals <class>
    if len(args) >= 2 and len(all_tokens) >= 3 and len(arg_tokens) >= 2:
        first_tok = all_tokens[0]
        last_tok = all_tokens[-1]
        # Token end offsets point at the last *content* character, so a
        # quoted/braced final argument leaves the closing ``"``/``}``
        # outside the fix range.  Widen the end by one when the next
        # source char is the matching delimiter — otherwise applying
        # the fix leaves a stray delimiter trailing the rewrite
        # (Copilot review, PR #438).
        fix_end = last_tok.end
        next_off = last_tok.end.offset + 1
        if 0 <= last_tok.start.offset < len(source) and source[last_tok.start.offset] in (
            '"',
            "{",
        ):
            close = '"' if source[last_tok.start.offset] == '"' else "}"
            if next_off < len(source) and source[next_off] == close:
                fix_end = type(last_tok.end)(
                    line=last_tok.end.line,
                    character=last_tok.end.character + 1,
                    offset=next_off,
                )
        fix_range = Range(
            start=first_tok.start,
            end=fix_end,
        )
        # ``args[idx]`` holds the *substituted* value of each word.  For
        # ``matchclass $url ::lib`` that gives ``"url"`` instead of
        # ``"$url"`` and the replacement would silently drop the
        # variable reference (IRULE2001 quick-fix audit, 2026-05).
        # Pull the raw source slice via the arg token offsets so the
        # rewrite preserves variable/command substitutions verbatim.
        item = _raw_arg_text(source, arg_tokens[0])
        cls = _raw_arg_text(source, arg_tokens[1])
        fixes = (
            CodeFix(
                range=fix_range,
                new_text=f"class match {item} equals {cls}",
                description="Replace with 'class match'",
            ),
        )

    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                "'matchclass' is deprecated since BIG-IP v10. "
                "Use 'class match <item> <operator> <class>' instead."
            ),
            severity=Severity.WARNING,
            code="IRULE2001",
            fixes=fixes,
        )
    ]


# IRULE2101: Heavy regex in hot event


@diag(
    code="IRULE2101",
    description="Heavy `regexp` in a high-frequency event — consider `string match` or data-group.",
    section="irules",
    ai_category="performance",
)
def check_heavy_regex_in_hot_event(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE2101: Hint when ``regexp`` is used in a hot event."""
    if cmd_name != "regexp":
        return []
    if active_dialect() != "f5-irules":
        return []
    if event not in _HOT_EVENTS:
        return []
    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                f"'regexp' in {event} may be expensive at high traffic volumes. "
                "Consider 'string match', 'switch -glob', or a data-group lookup."
            ),
            severity=Severity.HINT,
            code="IRULE2101",
        )
    ]


# IRULE5001: Ungated log in hot event


@diag(
    code="IRULE5001",
    description="Ungated `log` in a high-frequency event.",
    section="irules",
    ai_category="performance",
    conversion_label="Ungated log in hot event -> add debug gating",
)
def check_ungated_log(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE5001: Hint when ``log`` is used in a hot event without a guard."""
    if cmd_name != "log":
        return []
    if active_dialect() != "f5-irules":
        return []
    if event not in _HOT_EVENTS:
        return []
    # If the log command is inside an if guard we'd need the analyser's
    # nesting context to detect that.  For the first pass, always warn.
    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                f"'log' in {event} fires on every request. "
                "Set a debug flag in CLIENT_ACCEPTED "
                "(e.g. set debug 0) and gate with "
                "if {$debug} {...}."
            ),
            severity=Severity.HINT,
            code="IRULE5001",
        )
    ]


# IRULE4001: Write to static:: outside RULE_INIT
# IRULE4002: Generic static:: variable name — collision likely


def _static_var_from_set(
    cmd_name: str,
    args: list[str],
) -> str | None:
    """Return the ``static::`` variable name being written, or None."""
    if cmd_name == "set" and args and args[0].startswith("static::"):
        return args[0]
    if (
        cmd_name == "array"
        and len(args) >= 2
        and args[0] == "set"
        and args[1].startswith("static::")
    ):
        return args[1]
    return None


@diag(
    "IRULE4001",
    "Write to `static::` variable outside `RULE_INIT`.",
    section="irules_variable",
)
def check_static_write_outside_rule_init(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE4001: Warn when a ``static::`` variable is written outside RULE_INIT."""
    if active_dialect() != "f5-irules":
        return []
    if event == "RULE_INIT":
        return []
    var_name = _static_var_from_set(cmd_name, args)
    if var_name is None:
        return []
    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=(
                f"Writing to '{var_name}' outside RULE_INIT is dangerous. "
                "static:: variables are shared across all connections; "
                "concurrent writes can cause race conditions."
            ),
            severity=Severity.WARNING,
            code="IRULE4001",
        )
    ]


# Default regex patterns for generic ``static::`` / global variable names
# that are likely to collide across iRules.  Each pattern is matched
# case-insensitively against the bare name (after stripping ``static::``).
# Users can override these via ``tclLsp.genericVariablePatterns`` in
# editor settings.
DEFAULT_GENERIC_VARIABLE_PATTERNS: tuple[str, ...] = (
    # Debug / logging
    r"^debug(_level|_enabled)?$",
    r"^dbg$",
    r"^log_(level|server|enabled)$",
    r"^logging$",
    r"^verbose$",
    r"^trace$",
    # Configuration
    r"^(response_)?timeout$",
    r"^(max_)?retr(y|ies)$",
    r"^config$",
    r"^(enabled|disabled|active)$",
    r"^mode$",
    r"^(port|host|server|pool)$",
    # Counters / limits
    r"^count(er)?$",
    r"^(limit|max_connections|threshold|rate|interval)$",
    # Generic state
    r"^(flag|level|status|state|version|name|value|data|result|test|init|default)$",
)

# Compiled patterns (lazily rebuilt when the user changes configuration).
_compiled_generic_patterns: list[re.Pattern[str]] | None = None
_raw_generic_patterns: tuple[str, ...] | None = None


def _get_generic_patterns(
    patterns: list[str] | None = None,
) -> list[re.Pattern[str]]:
    """Return compiled regex patterns for generic variable name detection.

    Uses a module-level cache; recompiles when *patterns* changes.
    """
    global _compiled_generic_patterns, _raw_generic_patterns
    if patterns is None:
        patterns = list(DEFAULT_GENERIC_VARIABLE_PATTERNS)
    key = tuple(patterns)
    if _compiled_generic_patterns is not None and _raw_generic_patterns == key:
        return _compiled_generic_patterns
    _raw_generic_patterns = key
    _compiled_generic_patterns = []
    for pat in patterns:
        try:
            _compiled_generic_patterns.append(re.compile(pat, re.IGNORECASE))
        except re.error:
            pass  # skip invalid patterns from user config
    return _compiled_generic_patterns


def _is_generic_static_name(
    var_name: str,
    patterns: list[str] | None = None,
) -> bool:
    """Return True if *var_name* (including ``static::`` prefix) is generic."""
    bare = var_name.removeprefix("static::").lower()
    for pat in _get_generic_patterns(patterns):
        if pat.search(bare):
            return True
    return False


# IRULE4003: Variable scoping across events

_WHEN_BLOCK_RE = re.compile(r"\bwhen\s+([A-Z_][A-Z0-9_]*)\b")


def _scan_when_blocks(source: str) -> dict[str, list[str]]:
    """Return ``{event_name: [body_text, ...]}`` for all ``when`` blocks.

    Uses balanced-brace scanning to extract each body.
    """
    result: dict[str, list[str]] = {}
    for m in _WHEN_BLOCK_RE.finditer(source):
        event = m.group(1)
        pos = m.end()
        # Skip whitespace
        while pos < len(source) and source[pos] in " \t\n\r":
            pos += 1
        # Skip optional "priority N" / "timing enable|disable"
        while pos < len(source) and source[pos] != "{":
            if source[pos] in " \t\n\r":
                pos += 1
            elif source[pos : pos + 8] == "priority":
                pos += 8
                while pos < len(source) and source[pos] in " \t\n\r":
                    pos += 1
                while pos < len(source) and source[pos].isdigit():
                    pos += 1
            elif source[pos : pos + 6] == "timing":
                pos += 6
                while pos < len(source) and source[pos] in " \t\n\r":
                    pos += 1
                while pos < len(source) and source[pos].isalpha():
                    pos += 1
            else:
                break
        if pos >= len(source) or source[pos] != "{":
            continue
        # Balanced brace scan
        depth = 1
        start = pos + 1
        pos += 1
        while pos < len(source) and depth > 0:
            ch = source[pos]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            elif ch == "\\":
                pos += 1  # skip escaped char
            pos += 1
        body = source[start : pos - 1]
        result.setdefault(event, []).append(body)
    return result


def _var_referenced_in(var_name: str, body: str) -> bool:
    """Check if ``$var_name`` is referenced in *body* text."""
    # Match $varname (not followed by word char) or ${varname}
    escaped = re.escape(var_name)
    return bool(re.search(rf"\${escaped}(?!\w)|\$\{{{escaped}\}}", body))


@diag("IRULE4003", "Variable scoping concern across events.", section="irules_variable")
def check_variable_scope_across_events(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE4003: Hint about variable scoping concerns across events.

    When ``set varname value`` is used in event X (not RULE_INIT), scans
    other ``when`` blocks for ``$varname`` references and reports scoping
    concerns via :func:`variable_scope_note`.
    """
    if cmd_name != "set":
        return []
    if active_dialect() != "f5-irules":
        return []
    if event == "RULE_INIT":
        return []
    # Must be a write (set var value), not a read (set var)
    if len(args) < 2:
        return []
    var_name = args[0]
    # Skip static:: vars (handled by IRULE4001/4002)
    if var_name.startswith("static::"):
        return []
    # Skip global vars
    if var_name.startswith("::"):
        return []

    # Scan all when blocks for references to this variable
    blocks = _scan_when_blocks(source)
    concerns: list[str] = []
    for other_event, bodies in blocks.items():
        if other_event == event:
            continue
        for body in bodies:
            if _var_referenced_in(var_name, body):
                note = EVENT_REGISTRY.variable_scope_note(event, other_event)
                if note is not None:
                    concerns.append(note)
                break  # one match per event is enough

    if not concerns:
        return []

    # Build a single diagnostic combining all concerns
    msg = f"Variable '{var_name}': {concerns[0]}"
    if len(concerns) > 1:
        extras = "; ".join(concerns[1:])
        msg = f"{msg}; {extras}"

    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message=msg,
            severity=Severity.HINT,
            code="IRULE4003",
        )
    ]


# IRULE1003: Deprecated iRules event

# Derived from registry EventProps metadata — not hardcoded.
_DEPRECATED_EVENTS: frozenset[str] = deprecated_events()


@diag("IRULE1003", "Deprecated iRules event.", section="irules")
def check_deprecated_event(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str = "",
) -> list[Diagnostic]:
    """IRULE1003: Warn when ``when`` references a deprecated event."""
    if cmd_name != "when":
        return []
    if active_dialect() != "f5-irules":
        return []
    if not args or not arg_tokens:
        return []
    event_name = args[0]
    if event_name not in _DEPRECATED_EVENTS:
        return []
    return [
        Diagnostic(
            range=range_from_token(arg_tokens[0]),
            message=f"'{event_name}' event is deprecated.",
            severity=Severity.WARNING,
            code="IRULE1003",
        )
    ]


# IRULE1004: when block missing explicit priority


@diag("IRULE1004", "`when` block missing explicit `priority`.", section="irules")
def check_when_missing_priority(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str = "",
) -> list[Diagnostic]:
    """IRULE1004: Hint when a ``when`` block omits an explicit ``priority``."""
    if cmd_name != "when":
        return []
    if active_dialect() != "f5-irules":
        return []
    # when EVENT { body }            → args = [EVENT, body]   (no priority)
    # when EVENT priority N { body } → args = [EVENT, "priority", N, body]
    if len(args) >= 2 and args[1] == "priority":
        return []
    if not all_tokens:
        return []
    return [
        Diagnostic(
            range=range_from_token(all_tokens[0]),
            message="'when' missing an explicit priority. Add 'priority <N>' to control execution order.",
            severity=Severity.HINT,
            code="IRULE1004",
        )
    ]


# IRULE6001: Global namespace variable causes TMM pinning


def _global_var_from_command(
    cmd_name: str,
    args: list[str],
) -> str | None:
    """Return the global namespace variable name being accessed, or None.

    Detects ``set ::var``, ``array set ::var``, ``incr ::var``,
    ``append ::var``, ``lappend ::var``, and ``unset ::var``.
    """
    var_idx = variable_writing_commands().get(cmd_name)
    if var_idx is not None and var_idx < len(args):
        if args[var_idx].startswith("::"):
            return args[var_idx]
    return None


def _implicit_global_var_from_command(
    cmd_name: str,
    args: list[str],
) -> str | None:
    """Return a plain variable name that is implicitly global in RULE_INIT.

    In RULE_INIT, ``set var value`` (without ``::`` or ``static::`` prefix)
    creates a variable in the global namespace because RULE_INIT executes
    at global scope.  This helper detects such implicit globals.

    Only write forms are detected (``set var value``, ``incr var``, etc.)
    — a bare ``set var`` (read) in RULE_INIT is harmless if the variable
    was already created elsewhere.
    """
    var_idx = variable_writing_commands().get(cmd_name)
    if var_idx is None:
        return None
    # ``set var`` with 1 arg is a read, not a write — skip.
    if cmd_name == "set" and len(args) < 2:
        return None
    # ``unset`` destroys variables, it doesn't create implicit globals.
    if cmd_name == "unset":
        return None
    # ``array`` only creates implicit globals via ``array set``.
    if cmd_name == "array" and (not args or args[0] != "set"):
        return None
    if var_idx < len(args):
        var = args[var_idx]
        if not var.startswith("::") and not var.startswith("static::"):
            return var
    return None


@diag("IRULE6001", "iRules global variable usage.", section="irules", internal=True)
def check_global_namespace_usage(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
) -> list[Diagnostic]:
    """IRULE6001: Warn when a global namespace variable is used in an iRule.

    Global namespace variables (``::var``) force the virtual server into
    CMP compatibility mode, pinning all traffic to a single TMM.  Use
    ``static::var`` instead to keep TMM-local storage and preserve
    multi-TMM scalability.

    Also detects the ``global`` command which imports variables from the
    global namespace, and plain ``set var value`` in RULE_INIT which is
    implicitly global because RULE_INIT runs at the global namespace scope.
    """
    if active_dialect() != "f5-irules":
        return []

    # ``global varname`` — imports from the global namespace
    if cmd_name == "global" and args:
        var_name = args[0]
        static_name = f"static::{var_name}"
        fix_range = range_from_token(all_tokens[0])
        # Build the replacement: "set static::var" as a read, or keep existing value
        # The fix replaces the entire `global var` command with nothing useful on its
        # own — the user needs to also rename $var to $static::var.  We provide the
        # diagnostic; the code-action handler builds the edit.
        return [
            Diagnostic(
                range=fix_range,
                message=(
                    f"'global {var_name}' imports from the global namespace, "
                    "forcing CMP compatibility mode and pinning the virtual server "
                    f"to a single TMM. Use '{static_name}' instead."
                ),
                severity=Severity.WARNING,
                code="IRULE6001",
            )
        ]

    # ``set ::var``, ``incr ::var``, etc.
    var_name = _global_var_from_command(cmd_name, args)
    if var_name is not None:
        # Strip leading :: to get the bare name
        bare = var_name[2:]
        static_name = f"static::{bare}"
        fix_range = range_from_token(all_tokens[0])

        # Determine the token that holds the variable name for the fix range
        var_idx = variable_writing_commands().get(cmd_name)
        if var_idx is not None and var_idx < len(arg_tokens):
            fix_range = range_from_token(arg_tokens[var_idx])

        return [
            Diagnostic(
                range=fix_range,
                message=(
                    f"Global namespace variable '{var_name}' forces CMP compatibility "
                    "mode, pinning the virtual server to a single TMM. "
                    f"Use '{static_name}' instead."
                ),
                severity=Severity.WARNING,
                code="IRULE6001",
                fixes=(
                    CodeFix(
                        range=fix_range,
                        new_text=static_name,
                        description=f"Replace '{var_name}' with '{static_name}'",
                    ),
                ),
            )
        ]

    # Implicit globals in RULE_INIT: ``set var value`` (no :: prefix) is
    # global because RULE_INIT executes at the global namespace scope.
    if event == "RULE_INIT":
        bare = _implicit_global_var_from_command(cmd_name, args)
        if bare is not None:
            static_name = f"static::{bare}"
            fix_range = range_from_token(all_tokens[0])
            var_idx = variable_writing_commands().get(cmd_name)
            if var_idx is not None and var_idx < len(arg_tokens):
                fix_range = range_from_token(arg_tokens[var_idx])

            return [
                Diagnostic(
                    range=fix_range,
                    message=(
                        f"'{bare}' in RULE_INIT is implicitly global — RULE_INIT "
                        "runs at the global namespace scope. This forces CMP "
                        "compatibility mode, pinning the virtual server to a "
                        f"single TMM. Use '{static_name}' instead."
                    ),
                    severity=Severity.WARNING,
                    code="IRULE6001",
                    fixes=(
                        CodeFix(
                            range=fix_range,
                            new_text=static_name,
                            description=f"Replace '{bare}' with '{static_name}'",
                        ),
                    ),
                )
            ]

    return []


# All event-aware checks

_EVENT_CHECKS = [
    check_command_event_validity,
    check_matchclass,
    check_deprecated_event,
    check_when_missing_priority,
    check_heavy_regex_in_hot_event,
    check_ungated_log,
    check_static_write_outside_rule_init,
    check_variable_scope_across_events,
    check_global_namespace_usage,
]


def run_irules_event_checks(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str,
    file_profiles: frozenset[str] = frozenset(),
) -> list[Diagnostic]:
    """Run all iRules event-aware checks."""
    diagnostics: list[Diagnostic] = []
    for check in _EVENT_CHECKS:
        if check is check_command_event_validity:
            diagnostics.extend(
                check(
                    cmd_name,
                    args,
                    arg_tokens,
                    all_tokens,
                    source,
                    event=event,
                    file_profiles=file_profiles,
                )
            )
        else:
            diagnostics.extend(check(cmd_name, args, arg_tokens, all_tokens, source, event=event))
    return diagnostics
