"""Tcl code minifier: strips comments, collapses whitespace, compacts names.

Pure function: source in, minified source out.  The minifier preserves
semantic equivalence while producing the smallest reasonable output by:

1. Stripping all comments.
2. Collapsing inter-command whitespace to semicolons.
3. Collapsing intra-command whitespace to single spaces.
4. Recursively minifying braced body arguments.
5. Preserving string literals and expressions verbatim.
6. Compressing whitespace inside ``expr`` bodies (``{$a + $b}`` → ``{$a+$b}``).
7. Applying De Morgan's laws and comparison inversion when they shorten expressions.
8. Replacing ``${var}`` with ``$var`` when safe (no adjacent word characters).
9. Optionally compacting local variable, proc, and parameter names to short
   identifiers (a, b, c, ..., aa, ab, ...).
10. Optionally compacting static array member names to short identifiers.
11. Optionally aliasing repeated namespace-qualified command names.
12. Optionally emitting a symbol map for debugging.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Literal, overload

from core.commands.registry.runtime import body_arg_indices, expr_arg_indices
from core.common.suffix_array import build_lcp_array, build_suffix_array
from core.common.text_edits import apply_edits, name_generator
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType

# Commands whose presence in a proc body makes variable renaming unsafe
# because they reference variables by name as strings at runtime.
_SCOPE_BARRIER_COMMANDS = frozenset(
    {
        "upvar",
        "uplevel",
        "global",
        "variable",
        "eval",
        "trace",
        "vwait",
        "tkwait",
    }
)


@dataclass
class SymbolMap:
    """Map of original names to compacted names, grouped by scope.

    Used for debugging minified output -- e.g. to translate a short
    variable name in a log message back to the original.
    """

    variables: dict[str, dict[str, str]] = field(default_factory=dict)
    procs: dict[str, str] = field(default_factory=dict)
    array_members: dict[str, dict[str, str]] = field(default_factory=dict)
    command_aliases: dict[str, str] = field(default_factory=dict)
    argument_aliases: dict[str, str] = field(default_factory=dict)
    string_aliases: dict[str, str] = field(default_factory=dict)
    static_folds: dict[str, str] = field(default_factory=dict)

    def format(self) -> str:
        """Return a human-readable symbol map string."""
        lines: list[str] = []
        if self.procs:
            lines.append("# Procs")
            for original, short in sorted(self.procs.items()):
                lines.append(f"  {short} <- {original}")
        if self.variables:
            for scope_name, var_map in sorted(self.variables.items()):
                lines.append(f"# Variables in {scope_name}")
                for original, short in sorted(var_map.items(), key=lambda x: x[1]):
                    lines.append(f"  {short} <- {original}")
        if self.array_members:
            for array_name, member_map in sorted(self.array_members.items()):
                lines.append(f"# Array members of {array_name}")
                for original, short in sorted(member_map.items(), key=lambda x: x[1]):
                    lines.append(f"  {short} <- {original}")
        if self.command_aliases:
            lines.append("# Command aliases")
            for original, alias_var in sorted(self.command_aliases.items()):
                lines.append(f"  ${alias_var} <- {original}")
        if self.argument_aliases:
            lines.append("# Argument aliases")
            for original, alias_var in sorted(self.argument_aliases.items()):
                lines.append(f"  ${alias_var} <- {original}")
        if self.string_aliases:
            lines.append("# String literal aliases")
            for original, alias_var in sorted(self.string_aliases.items(), key=lambda x: x[1]):
                lines.append(f"  ${alias_var} <- {original!r}")
        if self.static_folds:
            lines.append("# Static substring folds (SCCP)")
            for original, folded in sorted(self.static_folds.items()):
                lines.append(f"  {folded!r} <- {original!r}")
        return "\n".join(lines)

    def reverse(self) -> dict[str, str]:
        """Build a reverse lookup: compacted name -> original name.

        For variables and array members the scope-qualified short name
        (``scope:short``) is included as well as the bare short name.
        When the same short name appears in multiple scopes the first
        one wins for the bare entry — the scope-qualified entry is
        always unambiguous.
        """
        rev: dict[str, str] = {}
        # Proc names: short -> original
        for original, short in self.procs.items():
            rev.setdefault(short, original)
        # Variables: scope-qualified and bare
        for scope_name, var_map in self.variables.items():
            for original, short in var_map.items():
                rev[f"{scope_name}:{short}"] = f"{scope_name}:{original}"
                rev.setdefault(short, original)
        # Array members
        for array_name, member_map in self.array_members.items():
            for original, short in member_map.items():
                rev.setdefault(short, original)
        # Command aliases: alias_var -> original_cmd
        for original, alias_var in self.command_aliases.items():
            rev.setdefault(alias_var, original)
        # Argument aliases: alias_var -> original_arg
        for original, alias_var in self.argument_aliases.items():
            rev.setdefault(alias_var, original)
        # String literal aliases: alias_var -> original_literal
        for original, alias_var in self.string_aliases.items():
            rev.setdefault(alias_var, original)
        return rev

    @staticmethod
    def parse(text: str) -> "SymbolMap":
        """Parse a symbol map from the human-readable format produced by :meth:`format`.

        This is the inverse of :meth:`format` and is used by
        :func:`unminify_error` to reload a saved symbol map file.
        """
        sm = SymbolMap()
        section: str | None = None
        section_name: str = ""
        for line in text.splitlines():
            stripped = line.strip()
            if not stripped:
                continue
            if stripped.startswith("# Procs"):
                section = "procs"
                continue
            if stripped.startswith("# Variables in "):
                section = "variables"
                section_name = stripped[len("# Variables in ") :]
                continue
            if stripped.startswith("# Array members of "):
                section = "array_members"
                section_name = stripped[len("# Array members of ") :]
                continue
            if stripped.startswith("# Command aliases"):
                section = "command_aliases"
                continue
            if stripped.startswith("# Argument aliases"):
                section = "argument_aliases"
                continue
            if stripped.startswith("# String literal aliases"):
                section = "string_aliases"
                continue
            if stripped.startswith("#"):
                section = None
                continue
            # Parse "  short <- original" or "  $alias <- original"
            if " <- " not in stripped:
                continue
            left, right = stripped.split(" <- ", 1)
            left = left.strip().lstrip("$")
            right = right.strip()
            if section == "procs":
                sm.procs[right] = left
            elif section == "variables":
                sm.variables.setdefault(section_name, {})[right] = left
            elif section == "array_members":
                sm.array_members.setdefault(section_name, {})[right] = left
            elif section == "command_aliases":
                sm.command_aliases[right] = left
            elif section == "argument_aliases":
                sm.argument_aliases[right] = left
            elif section == "string_aliases":
                # right might be repr-quoted
                if right.startswith("'") and right.endswith("'"):
                    right = right[1:-1]
                elif right.startswith('"') and right.endswith('"'):
                    right = right[1:-1]
                sm.string_aliases[right] = left
        return sm


@dataclass
class MinifyResult:
    """Full result from aggressive minification."""

    source: str
    symbol_map: SymbolMap = field(default_factory=SymbolMap)
    optimisations_applied: int = 0
    original_length: int = 0

    @property
    def minified_length(self) -> int:
        return len(self.source)

    @property
    def savings_pct(self) -> float:
        if self.original_length == 0:
            return 0.0
        return (1 - len(self.source) / self.original_length) * 100


def _resolve_dialect(dialect: str | None) -> str:
    """Resolve dialect, falling back to the active profile or tcl8.6."""
    if dialect is not None:
        return dialect
    try:
        from core.common.dialect import active_dialect

        return active_dialect()
    except Exception:
        return "tcl8.6"


@overload
def minify_tcl(
    source: str,
    *,
    compact_names: Literal[False] = ...,
    aggressive: Literal[False] = ...,
    dialect: str | None = ...,
) -> str: ...


@overload
def minify_tcl(
    source: str,
    *,
    compact_names: Literal[True],
    aggressive: Literal[False] = ...,
    dialect: str | None = ...,
) -> tuple[str, SymbolMap]: ...


@overload
def minify_tcl(
    source: str,
    *,
    compact_names: bool = ...,
    aggressive: Literal[True] = ...,
    dialect: str | None = ...,
) -> MinifyResult: ...


@overload
def minify_tcl(
    source: str,
    *,
    compact_names: bool = ...,
    aggressive: bool = ...,
    dialect: str | None = ...,
) -> str | tuple[str, SymbolMap] | MinifyResult: ...


def minify_tcl(
    source: str,
    *,
    compact_names: bool = False,
    aggressive: bool = False,
    dialect: str | None = None,
) -> str | tuple[str, SymbolMap] | MinifyResult:
    """Minify a Tcl source string.

    Args:
        source: The Tcl source code to minify.
        compact_names: If True, also compact variable and proc names.
            When True, returns ``(minified_source, symbol_map)`` instead
            of just the minified source string.
        aggressive: If True, run all optimizer passes (constant propagation,
            dead code elimination, expression folding, etc.) before name
            compaction and whitespace minification.  Returns a
            :class:`MinifyResult` with full details.
        dialect: The target dialect (e.g. ``"f5-irules"``).  When set,
            enables dialect-specific transforms such as ensemble subcommand
            abbreviation.  If *None*, auto-detected from the active
            dialect.

    Returns:
        The minified source code, or ``(minified, symbol_map)`` when
        *compact_names* is True, or a :class:`MinifyResult` when
        *aggressive* is True.
    """
    if aggressive:
        return _aggressive_minify(source, dialect=_resolve_dialect(dialect))
    resolved_dialect = _resolve_dialect(dialect)
    if compact_names:
        renamed_source, symbol_map = _compact_names(source)
        minified = _minify_body(renamed_source, dialect=resolved_dialect)
        return minified, symbol_map
    return _minify_body(source, dialect=resolved_dialect)


def unminify_error(
    error_message: str,
    *,
    symbol_map: SymbolMap | str | None = None,
    minified_source: str | None = None,
    original_source: str | None = None,
) -> str:
    """Translate a Tcl or iRule error message from minified code back to original names.

    Takes an error message produced by running minified code (e.g. a Tcl
    ``can't read "a"`` error or an iRule ``/Common/rule:1:`` log line) and
    replaces compacted identifiers with their original names using the
    symbol map.

    When *original_source* and *minified_source* are both provided,
    character-offset references (like ``... (line 1)`` pointing into a
    single-line minified script) are mapped to the approximate original
    line by correlating semicolons in the minified body with newlines in
    the original.

    Args:
        error_message: The raw error text from ``tclsh``, an iRule log,
            or a BIG-IP ``/var/log/ltm`` entry.
        symbol_map: A :class:`SymbolMap` instance, or the formatted text
            string produced by :meth:`SymbolMap.format` (as saved to a
            ``--symbol-map`` file).
        minified_source: The minified Tcl source that was running when
            the error occurred.  Used together with *original_source*
            for character-offset → line mapping.
        original_source: The original (pre-minification) Tcl source.

    Returns:
        The error message with compacted names expanded to originals and
        character offsets mapped to original line numbers where possible.
    """
    if symbol_map is None and original_source is None:
        return error_message

    # Resolve symbol map
    sm: SymbolMap
    if isinstance(symbol_map, str):
        sm = SymbolMap.parse(symbol_map)
    elif symbol_map is not None:
        sm = symbol_map
    else:
        sm = SymbolMap()

    result = error_message
    rev = sm.reverse()

    if rev:
        # Build a regex that matches any compacted identifier.  We match
        # whole identifiers only — either preceded by ``$`` (variable
        # reference in error text) or bounded by non-word characters.
        # Sort by length descending so longer names match first.
        sorted_names = sorted(rev.keys(), key=len, reverse=True)
        escaped = [re.escape(str(n)) for n in sorted_names]

        # Replace $shortName with $originalName (variable references in error text)
        dollar_pat = re.compile(r"\$(" + "|".join(escaped) + r")(?=\b|[^a-zA-Z0-9_]|$)")
        result = dollar_pat.sub(lambda m: "$" + rev[m.group(1)], result)

        # Replace bare identifiers in quotes: "shortName" -> "originalName"
        # This covers  can't read "a", invalid command name "b", etc.
        quoted_pat = re.compile(r'"(' + "|".join(escaped) + r')"')
        result = quoted_pat.sub(lambda m: '"' + rev[m.group(1)] + '"', result)

    # Map character offsets / line references in minified single-line code
    # back to original source lines.
    if minified_source is not None and original_source is not None:
        result = _remap_line_references(result, minified_source, original_source)

    return result


# Patterns for line/char references in Tcl and iRule error messages.
# iRule log:  /Common/my_irule:1: error message   (always line 1 for minified)
_IRULE_LOG_RE = re.compile(r"(/[A-Za-z0-9_./-]+):(\d+):")
# Tcl errorInfo:  (procedure "X" line N)
_TCL_PROCLINE_RE = re.compile(r'\(procedure "([^"]+)" line (\d+)\)')
# Tcl generic "line N"
_TCL_LINE_RE = re.compile(r"\bline (\d+)\b")


def _remap_line_references(
    message: str,
    minified_source: str,
    original_source: str,
) -> str:
    """Remap line-number references from minified positions to original lines.

    Minified code is typically one long line so all Tcl errors say "line 1".
    We build a character-offset → original-line mapping from the two sources
    so a human can find the relevant code in the original script.
    """
    # Build offset map: for each semicolon-separated command in the minified
    # output, record which original line it corresponds to.
    orig_lines = original_source.splitlines()
    orig_line_count = len(orig_lines)

    # Simple heuristic: count non-empty original lines and map them
    # proportionally to the minified command count.
    min_commands = minified_source.count(";") + 1
    orig_non_empty = [
        i + 1
        for i, line in enumerate(orig_lines)
        if line.strip() and not line.strip().startswith("#")
    ]

    if not orig_non_empty:
        return message

    # For "line 1" in minified code, we can't determine the exact original
    # line without a character offset.  But we can note that the code came
    # from the original and add context.
    def _remap_tcl_line(m: re.Match) -> str:
        line_no = int(m.group(1))
        if line_no == 1 and min_commands > 1:
            # Line 1 of minified code = entire script; not useful to remap.
            return m.group(0)
        # For line N in a proc body that was minified, map proportionally.
        if orig_line_count > 0 and line_no <= min_commands:
            ratio = (line_no - 1) / max(min_commands - 1, 1)
            idx = min(int(ratio * (len(orig_non_empty) - 1)), len(orig_non_empty) - 1)
            orig_line = orig_non_empty[idx]
            return f"line {orig_line} (minified line {line_no})"
        return m.group(0)

    # Remap procedure line references
    def _remap_proc_line(m: re.Match) -> str:
        proc_name = m.group(1)
        line_no = int(m.group(2))
        if line_no <= min_commands and orig_line_count > 0:
            ratio = (line_no - 1) / max(min_commands - 1, 1)
            idx = min(int(ratio * (len(orig_non_empty) - 1)), len(orig_non_empty) - 1)
            orig_line = orig_non_empty[idx]
            return f'(procedure "{proc_name}" line {orig_line}, minified line {line_no})'
        return m.group(0)

    result = _TCL_PROCLINE_RE.sub(_remap_proc_line, message)
    result = _TCL_LINE_RE.sub(_remap_tcl_line, result)
    return result


def _aggressive_minify(source: str, *, dialect: str) -> MinifyResult:
    """Maximum compression: optimise → compact names → minify whitespace.

    Pipeline:
    1. Run all compiler optimisation passes (constant propagation,
       dead code elimination, expression folding, string chain folding,
       incr conversion, code sinking, unused proc removal, etc.)
    1.5. Fold dynamic quoted strings to static literals where SCCP proves
         all substitutions are constant and no tainted values are involved.
    2. Compact variable and proc names to short identifiers.
    3. Strip comments, collapse whitespace, compress expr bodies.
    """
    from core.compiler.optimiser._manager import apply_optimisations, find_optimisations

    from .static_substr import fold_static_substrings

    original_length = len(source)

    # Phase 1: Run all optimizer passes.
    optimisations = find_optimisations(source)
    optimised = apply_optimisations(source, optimisations)
    opt_count = sum(1 for o in optimisations if not o.hint_only)

    # Phase 1.5: Static substring folding — use IR/CFG/SSA/SCCP to find
    # quoted strings where all $var substitutions resolve to constants.
    # Replaces dynamic strings with pre-computed literals, eliminating
    # repeated runtime variable lookups and command evaluations.
    folded, fold_count, static_fold_map = fold_static_substrings(optimised)
    opt_count += fold_count

    # Phase 2: Compact names.
    renamed, symbol_map = _compact_names(folded)
    if static_fold_map:
        symbol_map.static_folds = static_fold_map

    # Phase 2.5: Command aliasing — replace repeated long command names
    # with short variable aliases (e.g. HTTP::uri → $a).
    aliased, cmd_aliases = _alias_repeated_commands(renamed)
    if cmd_aliases:
        symbol_map.command_aliases = cmd_aliases

    # Phase 2.6: Argument aliasing — replace repeated literal arguments
    # (e.g. -normalized) with short variable aliases.
    existing_alias_names = set(cmd_aliases.values())
    arg_aliased, arg_aliases = _alias_repeated_arguments(aliased, existing_alias_names)
    if arg_aliases:
        symbol_map.argument_aliases = arg_aliases

    # Phase 2.7: String-literal substring aliasing — replace repeated
    # substrings inside quoted strings with variable aliases.
    all_alias_names = existing_alias_names | set(arg_aliases.values())
    str_aliased, str_aliases = _alias_string_literals(arg_aliased, all_alias_names)
    if str_aliases:
        symbol_map.string_aliases = str_aliases

    # Phase 3: Whitespace minification.
    minified = _minify_body(str_aliased, dialect=dialect)

    return MinifyResult(
        source=minified,
        symbol_map=symbol_map,
        optimisations_applied=opt_count,
        original_length=original_length,
    )


# Name generation — delegated to core.common.text_edits
_name_generator = name_generator


# Name compaction using the semantic model


def _compact_names(source: str) -> tuple[str, SymbolMap]:
    """Compact variable and proc names using the analyser's semantic model.

    Returns (renamed_source, symbol_map).
    """
    from core.analysis.analyser import analyse
    from core.analysis.semantic_model import Scope
    from core.commands.registry import REGISTRY
    from lsp.features.references import find_proc_call_sites

    analysis = analyse(source)
    symbol_map = SymbolMap()

    # Collect all text edits as (offset, length, new_text) tuples.
    edits: list[tuple[int, int, str]] = []

    # Detect barrier commands to identify unsafe scopes.
    barrier_scopes = _find_barrier_scopes(analysis)

    builtin_names = set(REGISTRY.command_names())

    # --- Variable renaming (per-proc scope) ---

    def _process_scope(scope: Scope, scope_label: str) -> None:
        if scope.kind == "proc" and scope_label not in barrier_scopes:
            # Find the ProcDef for this scope to get param info.
            proc_def = None
            for pd in analysis.all_procs.values():
                if pd.name == scope.name:
                    proc_def = pd
                    break

            param_names = set()
            if proc_def:
                param_names = {p.name for p in proc_def.params}

            var_gen = _name_generator()
            var_map: dict[str, str] = {}
            existing_names = set(scope.variables.keys())

            for var_name, var_def in sorted(scope.variables.items()):
                # Skip single-char names -- already minimal.
                if len(var_name) <= 1:
                    continue
                # Skip namespace-qualified variables.
                if "::" in var_name:
                    continue

                short = _next_unused_name(var_gen, existing_names, var_map)
                if short is None or len(short) >= len(var_name):
                    continue
                var_map[var_name] = short

                is_param = var_name in param_names

                # Definition site: only for non-params (param defs point to proc name).
                if not is_param:
                    r = var_def.definition_range
                    actual = source[r.start.offset : r.end.offset + 1]
                    if actual == var_name:
                        edits.append((r.start.offset, len(actual), short))

                # Reference sites ($var): skip the $ prefix.
                for ref in var_def.references:
                    ref_text = source[ref.start.offset : ref.end.offset + 1]
                    if ref_text.startswith("$") and ref_text[1:] == var_name:
                        edits.append((ref.start.offset + 1, len(var_name), short))

            # Rename params in the param list.
            if proc_def and var_map:
                _rename_params_in_list(source, proc_def, var_map, edits)

            if var_map:
                symbol_map.variables[scope_label] = var_map

        for child in scope.children:
            child_label = (
                f"{scope_label}::{child.name}" if scope_label != "::" else f"::{child.name}"
            )
            _process_scope(child, child_label)

    _process_scope(analysis.global_scope, "::")

    # --- Proc renaming ---

    proc_name_gen = _name_generator()
    proc_map: dict[str, str] = {}
    used_proc_names: set[str] = set()

    for qname, proc_def in sorted(analysis.all_procs.items()):
        name = proc_def.name
        # Skip single-char names.
        if len(name) <= 1:
            continue
        # Skip namespace-qualified proc names for safety.
        if "::" in name:
            continue

        short = next(proc_name_gen)
        while short in builtin_names or short in used_proc_names:
            short = next(proc_name_gen)
        if len(short) >= len(name):
            continue
        proc_map[name] = short
        used_proc_names.add(short)

        # Rename definition site.
        r = proc_def.name_range
        actual = source[r.start.offset : r.end.offset + 1]
        if actual == name:
            edits.append((r.start.offset, len(actual), short))

        # Rename call sites.
        call_sites = find_proc_call_sites(name, proc_def.qualified_name, analysis)
        def_key = (r.start.offset, len(actual))
        for call_r in call_sites:
            call_text = source[call_r.start.offset : call_r.end.offset + 1]
            key = (call_r.start.offset, len(call_text))
            if key != def_key and call_text == name:
                edits.append((call_r.start.offset, len(call_text), short))

    if proc_map:
        symbol_map.procs = proc_map

    # --- Array member compaction ---

    array_member_map = _compact_array_members(source, barrier_scopes, analysis, edits)
    if array_member_map:
        symbol_map.array_members = array_member_map

    # --- Apply edits ---

    result = _apply_edits(source, edits)
    return result, symbol_map


def _rename_params_in_list(
    source: str,
    proc_def,
    var_map: dict[str, str],
    edits: list[tuple[int, int, str]],
) -> None:
    """Find and rename parameter names within the proc's parameter list."""
    # The param list sits between the proc name and the body: proc NAME {PARAMS} {BODY}
    search_start = proc_def.name_range.end.offset + 1
    search_end = proc_def.body_range.start.offset
    param_region = source[search_start:search_end]

    for param in proc_def.params:
        if param.name not in var_map:
            continue
        short = var_map[param.name]
        # Find the param name in the param region.
        # Use word-boundary matching to avoid partial matches.
        pattern = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(param.name) + r"(?![A-Za-z0-9_])")
        for m in pattern.finditer(param_region):
            abs_offset = search_start + m.start()
            edits.append((abs_offset, len(param.name), short))


# Array keys that look user-input-derived — skip the whole array.
# Only match when these terms appear as the full key or as a leading component
# (e.g. "uri", "request_body") but not as a trailing component like "database_host".
_UNSAFE_MEMBER_PATTERN = re.compile(
    r"^(uri|url|path|header|cookie|query|param|filename|request|input|form"
    r"|method|remote|client|addr|password|auth|token|session)(_|$)",
    re.IGNORECASE,
)

# Parse "arrayname(member)" from a token's text.
_ARRAY_MEMBER_RE = re.compile(r"^([\w:]+)\(([^)$\[]+)\)$")


def _scan_array_tokens(
    text: str,
    base_offset: int,
    array_uses: dict[str, dict[str, list[int]]],
    top_source: str,
) -> None:
    """Walk the token stream to find array member references.

    Descends into ``STR`` (braced body) and ``CMD`` (command substitution)
    tokens so that references inside proc bodies, if-blocks, etc. are found.
    Only ``VAR`` and ``ESC`` tokens are considered — string literals and
    comments are never matched.

    Uses an explicit work stack instead of recursion to avoid hitting
    Python's recursion limit on deeply nested iRules.

    Args:
        text: The Tcl source text to lex.
        base_offset: Absolute offset of *text* within the top-level source.
        array_uses: Accumulator — mutated in place.
        top_source: The original top-level source (used to check quoting context).
    """
    work_stack: list[tuple[str, int]] = [(text, base_offset)]

    while work_stack:
        cur_text, cur_base = work_stack.pop()
        lexer = TclLexer(cur_text)
        prev_type = TokenType.EOL
        in_quoted = False

        while True:
            tok = lexer.get_token()
            if tok is None:
                break

            if tok.type == TokenType.SEP:
                prev_type = tok.type
                in_quoted = False
                continue
            if tok.type == TokenType.EOL:
                prev_type = tok.type
                in_quoted = False
                continue

            if tok.type == TokenType.STR:
                # Only descend if content is long enough to hold an array
                # reference (minimum "a(b)" = 4 chars).  Also guards against
                # the lexer producing a STR token for a bare "$" which would
                # create an infinite work-stack loop.
                if len(tok.text) >= 4:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                prev_type = tok.type
                in_quoted = False
                continue

            if tok.type == TokenType.CMD:
                if len(tok.text) >= 4:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                prev_type = tok.type
                continue

            is_new_arg = prev_type in (TokenType.SEP, TokenType.EOL)
            if is_new_arg:
                abs_off = cur_base + tok.start.offset
                in_quoted = abs_off < len(top_source) and top_source[abs_off] == '"'

            prev_type = tok.type

            if in_quoted and tok.type == TokenType.ESC:
                continue

            if tok.type not in (TokenType.ESC, TokenType.VAR):
                continue

            m = _ARRAY_MEMBER_RE.match(tok.text)
            if not m:
                continue

            arr_name = m.group(1)
            member = m.group(2)

            if len(member) <= 1 or "::" in arr_name:
                continue

            if tok.type == TokenType.VAR:
                text_start = cur_base + tok.start.offset + 1
            else:
                text_start = cur_base + tok.start.offset

            member_offset = text_start + len(arr_name) + 1
            array_uses.setdefault(arr_name, {}).setdefault(member, []).append(member_offset)


def _compact_array_members(
    source: str,
    barrier_scopes: set[str],
    analysis,
    edits: list[tuple[int, int, str]],
) -> dict[str, dict[str, str]]:
    """Compact static array member names to short identifiers.

    Recursively walks the lexer token stream to find ``VAR`` and ``ESC``
    tokens with ``name(member)`` structure.  This only matches actual
    variable references — never string literals, comments, or other text
    that happens to contain parentheses.

    Only renames members that:
    - Are literal string keys (no variable interpolation or command subst).
    - Are longer than 1 character.
    - Don't appear to derive from user input (URI, path, header, etc.).

    Returns a map of ``{array_name: {original_member: short_member}}``.
    """
    # Recursively scan all tokens to collect array member uses.
    array_uses: dict[str, dict[str, list[int]]] = {}
    _scan_array_tokens(source, 0, array_uses, source)

    if not array_uses:
        return {}

    result_map: dict[str, dict[str, str]] = {}

    for arr_name, members in array_uses.items():
        # Skip if any member name looks user-input-derived.
        if any(_UNSAFE_MEMBER_PATTERN.search(m) for m in members):
            continue

        # Build a rename map for this array's members.
        member_gen = _name_generator()
        member_map: dict[str, str] = {}
        existing_members = set(members.keys())

        for member_name in sorted(members.keys()):
            short = _next_unused_name(member_gen, existing_members, member_map)
            if short is None or len(short) >= len(member_name):
                continue
            member_map[member_name] = short

            for offset in members[member_name]:
                edits.append((offset, len(member_name), short))

        if member_map:
            result_map[arr_name] = member_map

    return result_map


# ---------------------------------------------------------------------------
# Phase 2.8: Ensemble subcommand abbreviation
# ---------------------------------------------------------------------------

# Minimum unambiguous prefixes for Tcl 8.6 ensemble subcommands.
# Only used when the dialect guarantees a fixed ensemble (e.g. F5 iRules).
_SUBCMD_ABBREVIATIONS: dict[str, dict[str, str]] = {
    "string": {
        "bytelength": "b",
        "cat": "ca",
        "compare": "co",
        "equal": "e",
        "first": "f",
        "index": "in",
        "last": "la",
        "length": "le",
        "map": "map",  # "ma" matches both "map" and "match"
        "match": "mat",
        "range": "ra",
        "repeat": "repe",
        "replace": "repl",
        "reverse": "rev",
        "tolower": "tol",
        "totitle": "tot",
        "toupper": "tou",
        "trim": "trim",  # prefix of trimleft/trimright
        "trimleft": "triml",
        "trimright": "trimr",
        "wordend": "worde",
        "wordstart": "words",
    },
    "info": {
        "args": "a",
        "body": "b",
        "cmdcount": "cm",
        "commands": "comm",
        "complete": "comp",
        "default": "d",
        "exists": "e",
        "frame": "fr",
        "functions": "fu",
        "globals": "g",
        "hostname": "h",
        "level": "le",
        "library": "li",
        "loaded": "loa",
        "locals": "loc",
        "nameofexecutable": "n",
        "patchlevel": "pa",
        "procs": "pr",
        "script": "sc",
        "sharedlibextension": "sh",
        "tclversion": "t",
    },
    "clock": {
        "add": "a",
        "clicks": "c",
        "format": "f",
        "microseconds": "mic",
        "milliseconds": "mil",
        "scan": "sc",
        "seconds": "se",
    },
}

# Only include entries where the abbreviation is actually shorter.
for _cmd, _subs in _SUBCMD_ABBREVIATIONS.items():
    for _full, _abbr in list(_subs.items()):
        if len(_abbr) >= len(_full):
            del _subs[_full]

# Dialects where ensemble commands are fixed (no user-added subcommands).
_FIXED_ENSEMBLE_DIALECTS = frozenset({"f5-irules", "f5-iapps", "f5-bigip"})


def _abbreviated_subcommand(
    command_name: str,
    subcommand_name: str,
    dialect: str | None,
) -> str:
    """Return abbreviated subcommand text when safe for the active dialect."""
    if dialect not in _FIXED_ENSEMBLE_DIALECTS:
        return subcommand_name
    subs = _SUBCMD_ABBREVIATIONS.get(command_name)
    if not subs:
        return subcommand_name
    return subs.get(subcommand_name, subcommand_name)


def _alias_repeated_commands(source: str) -> tuple[str, dict[str, str]]:
    """Replace repeated long command names with short variable aliases.

    Uses the semantic model's ``command_invocations`` to find command call
    sites rather than regex matching on raw source text.

    For each command name used N times, replacing it with ``$alias`` saves
    bytes if ``N * len(name) > N * (len(alias) + 1) + len("set alias name\\n")``.

    Returns ``(modified_source, {original_name: alias_var})``.
    """
    from core.analysis.analyser import analyse

    try:
        analysis = analyse(source)
    except Exception:
        return source, {}

    # Collect all command invocations from the semantic model.
    # Skip Tcl builtins that are byte-compiled specially and would break if
    # invoked through a variable reference.
    _BUILTIN_SKIP = {
        "set",
        "unset",
        "proc",
        "if",
        "else",
        "elseif",
        "while",
        "for",
        "foreach",
        "switch",
        "return",
        "break",
        "continue",
        "expr",
        "catch",
        "try",
        "throw",
        "package",
        "namespace",
        "upvar",
        "uplevel",
        "variable",
        "global",
        "append",
        "lappend",
        "incr",
        "info",
        "string",
        "list",
        "llength",
        "lindex",
        "lrange",
        "lsort",
        "lsearch",
        "lreplace",
        "linsert",
        "dict",
        "array",
        "regexp",
        "regsub",
        "scan",
        "format",
        "open",
        "close",
        "read",
        "gets",
        "puts",
        "eof",
        "flush",
        "seek",
        "tell",
        "fconfigure",
        "fcopy",
        "fileevent",
        "socket",
        "after",
        "update",
        "vwait",
        "rename",
        "source",
        "eval",
        "apply",
        "tailcall",
        "error",
        "cd",
        "pwd",
        "file",
        "glob",
        "clock",
        "binary",
        "encoding",
        "interp",
        "load",
        "exit",
        "pid",
        "exec",
        "chan",
    }
    cmd_uses: dict[str, list[int]] = {}
    for inv in analysis.command_invocations:
        name = inv.name
        # Skip very short names and Tcl builtins.
        if len(name) <= 2 or name in _BUILTIN_SKIP:
            continue
        cmd_uses.setdefault(name, []).append(inv.range.start.offset)

    if not cmd_uses:
        return source, {}

    alias_gen = _name_generator()
    aliases: dict[str, str] = {}
    used_aliases: set[str] = set()

    # Process in order of most savings first.
    for name, offsets in sorted(cmd_uses.items(), key=lambda x: -len(x[1]) * len(x[0])):
        count = len(offsets)
        if count < 2:
            continue

        alias = next(alias_gen)
        while alias in used_aliases:
            alias = next(alias_gen)

        # Cost-benefit analysis.
        original_cost = count * len(name)
        preamble_cost = 4 + len(alias) + 1 + len(name) + 1  # "set a HTTP::uri\n"
        aliased_cost = preamble_cost + count * (len(alias) + 1)  # "$a" per use

        if aliased_cost >= original_cost:
            continue

        aliases[name] = alias
        used_aliases.add(alias)

    if not aliases:
        return source, {}

    # Build preamble and replacements using the exact offsets from the model.
    preamble_lines: list[str] = []
    all_replacements: list[tuple[int, int, str]] = []
    for name, alias in aliases.items():
        preamble_lines.append(f"set {alias} {name}")
        for offset in cmd_uses[name]:
            all_replacements.append((offset, len(name), f"${alias}"))

    result = _apply_edits(source, all_replacements)
    preamble = "\n".join(preamble_lines) + "\n"
    result = preamble + result

    return result, aliases


def _scan_argument_tokens(
    text: str,
    base_offset: int,
    arg_uses: dict[str, list[int]],
    top_source: str,
) -> None:
    """Walk the token stream to find repeated literal arguments.

    Collects ``ESC`` tokens in non-command position (i.e. not the first
    word after a newline/semicolon).  Descends into ``STR`` and ``CMD``
    tokens to cover proc bodies, if-blocks, etc.

    Uses an explicit work stack instead of recursion to avoid hitting
    Python's recursion limit on deeply nested iRules.

    Only considers tokens whose text is at least 3 characters long and
    doesn't contain characters that would need quoting when used as a
    ``set`` value.
    """
    work_stack: list[tuple[str, int]] = [(text, base_offset)]

    while work_stack:
        cur_text, cur_base = work_stack.pop()
        lexer = TclLexer(cur_text)
        is_command_word = True
        in_quoted = False

        while True:
            tok = lexer.get_token()
            if tok is None:
                break

            if tok.type == TokenType.EOL:
                is_command_word = True
                in_quoted = False
                continue
            if tok.type == TokenType.SEP:
                is_command_word = False
                in_quoted = False
                continue

            if tok.type == TokenType.STR:
                if len(tok.text) >= 3:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                is_command_word = False
                in_quoted = False
                continue

            if tok.type == TokenType.CMD:
                if len(tok.text) >= 3:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                is_command_word = False
                continue

            if is_command_word:
                is_command_word = False
                continue

            if tok.type != TokenType.ESC:
                in_quoted = False
                continue

            abs_off = cur_base + tok.start.offset
            if abs_off < len(top_source) and top_source[abs_off] == '"':
                in_quoted = True

            if in_quoted:
                continue

            val = tok.text
            # Skip short values and values with characters that complicate quoting.
            if len(val) < 3 or any(c in val for c in ' \t\n"{}[]$\\;'):
                continue

            arg_uses.setdefault(val, []).append(abs_off)


def _alias_repeated_arguments(
    source: str,
    existing_aliases: set[str],
) -> tuple[str, dict[str, str]]:
    """Replace repeated literal arguments with short variable aliases.

    Scans the token stream for literal ``ESC`` tokens in argument position
    that appear multiple times.  Applies the same cost-benefit analysis as
    command aliasing.

    *existing_aliases* contains alias variable names already claimed by
    command aliasing so we don't collide.

    Returns ``(modified_source, {original_value: alias_var})``.
    """
    arg_uses: dict[str, list[int]] = {}
    _scan_argument_tokens(source, 0, arg_uses, source)

    if not arg_uses:
        return source, {}

    alias_gen = _name_generator()
    aliases: dict[str, str] = {}
    used: set[str] = set(existing_aliases)

    for val, offsets in sorted(arg_uses.items(), key=lambda x: -len(x[1]) * len(x[0])):
        count = len(offsets)
        if count < 2:
            continue

        alias = next(alias_gen)
        while alias in used:
            alias = next(alias_gen)

        original_cost = count * len(val)
        preamble_cost = 4 + len(alias) + 1 + len(val) + 1  # "set a -normalized\n"
        aliased_cost = preamble_cost + count * (len(alias) + 1)  # "$a" per use

        if aliased_cost >= original_cost:
            continue

        aliases[val] = alias
        used.add(alias)

    if not aliases:
        return source, {}

    preamble_lines: list[str] = []
    all_replacements: list[tuple[int, int, str]] = []
    for val, alias in aliases.items():
        preamble_lines.append(f"set {alias} {val}")
        for offset in arg_uses[val]:
            all_replacements.append((offset, len(val), f"${alias}"))

    result = _apply_edits(source, all_replacements)
    preamble = "\n".join(preamble_lines) + "\n"
    result = preamble + result

    return result, aliases


# ---------------------------------------------------------------------------
# Phase 2.7 — String-literal substring aliasing
# ---------------------------------------------------------------------------


def _collect_string_literals(
    text: str,
    base_offset: int,
    top_source: str,
    segments: list[tuple[int, str]],
) -> None:
    """Collect literal text segments inside quoted strings.

    Each entry in *segments* is ``(abs_offset, text)`` for an ``ESC`` token
    that appears inside a double-quoted argument.  Descends into ``STR``
    and ``CMD`` tokens to cover proc bodies.

    Uses an explicit work stack instead of recursion to avoid hitting
    Python's recursion limit on deeply nested iRules.
    """
    work_stack: list[tuple[str, int]] = [(text, base_offset)]

    while work_stack:
        cur_text, cur_base = work_stack.pop()
        lexer = TclLexer(cur_text)
        is_command_word = True
        in_quoted = False

        while True:
            tok = lexer.get_token()
            if tok is None:
                break

            if tok.type == TokenType.EOL:
                is_command_word = True
                in_quoted = False
                continue
            if tok.type == TokenType.SEP:
                is_command_word = False
                in_quoted = False
                continue

            if tok.type == TokenType.STR:
                if len(tok.text) >= 2:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                is_command_word = False
                in_quoted = False
                continue

            if tok.type == TokenType.CMD:
                if len(tok.text) >= 2:
                    inner_offset = cur_base + tok.start.offset + 1
                    work_stack.append((tok.text, inner_offset))
                is_command_word = False
                continue

            abs_off = cur_base + tok.start.offset

            if is_command_word:
                is_command_word = False
                in_quoted = False
                continue

            if not in_quoted and abs_off < len(top_source) and top_source[abs_off] == '"':
                in_quoted = True
                abs_off += 1

            if in_quoted and tok.type == TokenType.ESC and len(tok.text) >= 2:
                segments.append((abs_off, tok.text))

            if tok.type not in (TokenType.ESC, TokenType.VAR, TokenType.CMD):
                in_quoted = False


# Suffix array / LCP — delegated to core.common.suffix_array
_build_suffix_array = build_suffix_array
_build_lcp_array = build_lcp_array


def _alias_string_literals(
    source: str,
    existing_aliases: set[str],
) -> tuple[str, dict[str, str]]:
    """Replace repeated substrings in quoted strings with variable aliases.

    Collects all literal text segments from inside double-quoted arguments,
    concatenates them (separated by a sentinel), builds a suffix array +
    LCP array to find repeated substrings, then greedily selects the most
    valuable candidates.

    The preamble uses braces (``set a {value}``) so the stored value is
    inert — no further ``$``, ``[``, or ``\\`` expansion.

    *existing_aliases* are alias names already claimed by prior phases.

    Returns ``(modified_source, {original_substring: alias_var})``.
    """
    # Step 1: collect literal segments.
    segments: list[tuple[int, str]] = []
    _collect_string_literals(source, 0, source, segments)

    if not segments:
        return source, {}

    # Step 2: build a concatenated text with sentinels between segments.
    # Map positions in the concatenated text back to source offsets.
    sentinel = "\x00"
    concat_parts: list[str] = []
    # concat_offset_to_source[i] = source offset for concat position i, or -1
    concat_to_src: list[int] = []
    # Track which segment each concat position belongs to.
    concat_to_seg: list[int] = []

    for seg_idx, (src_offset, seg_text) in enumerate(segments):
        concat_parts.append(seg_text)
        for j in range(len(seg_text)):
            concat_to_src.append(src_offset + j)
            concat_to_seg.append(seg_idx)
        # Add sentinel.
        concat_parts.append(sentinel)
        concat_to_src.append(-1)
        concat_to_seg.append(-1)

    concat = "".join(concat_parts)

    # Step 3: suffix array + LCP to find repeated substrings.
    sa = _build_suffix_array(concat)
    lcp = _build_lcp_array(concat, sa)

    # Step 4: find candidate substrings.
    # For each LCP value >= min_len, the shared prefix of adjacent suffixes
    # is a repeated substring.  Collect unique substrings with their count
    # and positions.
    min_len = 3  # Minimum substring length to consider.

    # candidate_text -> set of (seg_idx, offset_in_seg) tuples
    candidates: dict[str, set[int]] = {}  # text -> set of concat positions

    for i in range(len(sa) - 1):
        lcp_len = lcp[i]
        if lcp_len < min_len:
            continue
        # The repeated substring is concat[sa[i]:sa[i]+lcp_len].
        # Record the maximal length.
        pos1 = sa[i]
        pos2 = sa[i + 1]

        # Skip if either position is a sentinel or crosses a sentinel.
        if concat_to_src[pos1] < 0 or concat_to_src[pos2] < 0:
            continue

        # Find the actual shared length that doesn't cross a sentinel.
        actual_len = 0
        for k in range(lcp_len):
            if pos1 + k >= len(concat_to_src) or pos2 + k >= len(concat_to_src):
                break
            if concat_to_src[pos1 + k] < 0 or concat_to_src[pos2 + k] < 0:
                break
            actual_len = k + 1

        if actual_len < min_len:
            continue

        sub = concat[pos1 : pos1 + actual_len]
        # Don't alias substrings that contain sentinels.
        if sentinel in sub:
            continue

        candidates.setdefault(sub, set()).add(pos1)
        candidates.setdefault(sub, set()).add(pos2)

    if not candidates:
        return source, {}

    # Step 5: for each candidate, find ALL occurrences across all segments.
    # The suffix array only gave us adjacent pairs; now count true frequency.
    candidate_occurrences: dict[str, list[int]] = {}  # text -> [source_offsets]

    for sub_text in candidates:
        src_offsets: list[int] = []
        start = 0
        while True:
            idx = concat.find(sub_text, start)
            if idx < 0:
                break
            # Check this occurrence doesn't cross a sentinel.
            crosses = False
            for k in range(len(sub_text)):
                if concat_to_src[idx + k] < 0:
                    crosses = True
                    break
            if not crosses:
                src_offsets.append(concat_to_src[idx])
            start = idx + 1

        if len(src_offsets) >= 2:
            candidate_occurrences[sub_text] = src_offsets

    if not candidate_occurrences:
        return source, {}

    # Step 6: cost-benefit and greedy selection.
    # Sort by savings descending.
    alias_gen = _name_generator()
    used: set[str] = set(existing_aliases)
    aliases: dict[str, str] = {}  # substring -> alias
    alias_occurrences: dict[str, list[int]] = {}  # substring -> [source_offsets]

    # Track which source byte ranges are already claimed.
    claimed: set[int] = set()

    scored = []
    for sub_text, src_offsets in candidate_occurrences.items():
        count = len(src_offsets)
        original_cost = count * len(sub_text)
        # Preamble: "set a {value}\n" — braces add 2 chars.
        est_alias_len = 1  # Estimate alias length (1 char for first 26).
        preamble_cost = 4 + est_alias_len + 1 + 1 + len(sub_text) + 1 + 1  # set a {val}\n
        aliased_cost = preamble_cost + count * (est_alias_len + 1)  # "$a"
        savings = original_cost - aliased_cost
        if savings > 0:
            scored.append((savings, sub_text, src_offsets))

    scored.sort(key=lambda x: (-x[0], -len(x[1])))

    for savings, sub_text, src_offsets in scored:
        # Skip substrings with unbalanced braces — can't safely brace-quote.
        if sub_text.count("{") != sub_text.count("}"):
            continue

        # Filter out occurrences that overlap with already-claimed ranges.
        free_offsets = []
        for off in src_offsets:
            span = set(range(off, off + len(sub_text)))
            if not span & claimed:
                free_offsets.append(off)

        if len(free_offsets) < 2:
            continue

        # Recheck cost-benefit with the actual alias name.
        alias = next(alias_gen)
        while alias in used:
            alias = next(alias_gen)

        count = len(free_offsets)
        original_cost = count * len(sub_text)
        preamble_cost = 4 + len(alias) + 1 + 1 + len(sub_text) + 1 + 1  # set a {val}\n

        # Compute per-site replacement cost: "$a" (len+1) or "${a}" (len+3)
        # depending on whether the next char is a word character.
        aliased_cost = preamble_cost
        for off in free_offsets:
            end = off + len(sub_text)
            if end < len(source) and (source[end].isalnum() or source[end] == "_"):
                aliased_cost += len(alias) + 3  # "${alias}"
            else:
                aliased_cost += len(alias) + 1  # "$alias"

        if aliased_cost >= original_cost:
            continue

        aliases[sub_text] = alias
        alias_occurrences[sub_text] = free_offsets
        used.add(alias)

        # Claim the byte ranges.
        for off in free_offsets:
            claimed.update(range(off, off + len(sub_text)))

    if not aliases:
        return source, {}

    # Step 7: build preamble and edits.
    preamble_lines: list[str] = []
    all_replacements: list[tuple[int, int, str]] = []

    for sub_text, alias in aliases.items():
        # Use braces in preamble to prevent any substitution of the value.
        preamble_lines.append(f"set {alias} {{{sub_text}}}")
        for offset in alias_occurrences[sub_text]:
            # If the character immediately after the replacement is a word
            # character, use ${alias} so Tcl doesn't merge the alias name
            # with the following text (e.g. "$acompleted" vs "${a}completed").
            end = offset + len(sub_text)
            if end < len(source) and (source[end].isalnum() or source[end] == "_"):
                replacement = f"${{{alias}}}"
            else:
                replacement = f"${alias}"
            all_replacements.append((offset, len(sub_text), replacement))

    result = _apply_edits(source, all_replacements)
    preamble = "\n".join(preamble_lines) + "\n"
    result = preamble + result

    return result, aliases


def _next_unused_name(
    gen,
    existing_vars: set[str],
    already_mapped: dict[str, str],
) -> str | None:
    """Get the next short name that doesn't collide with existing variables."""
    used_shorts = set(already_mapped.values())
    for _ in range(1000):  # Safety limit.
        short = next(gen)
        if short not in existing_vars and short not in used_shorts:
            return short
    return None


def _find_barrier_scopes(analysis) -> set[str]:
    """Find proc scope labels that contain barrier commands."""
    barrier_scopes: set[str] = set()
    for invocation in analysis.command_invocations:
        if invocation.name in _SCOPE_BARRIER_COMMANDS:
            scope_label = _scope_label_at_line(analysis.global_scope, invocation.range.start.line)
            if scope_label:
                barrier_scopes.add(scope_label)
    return barrier_scopes


def _scope_label_at_line(scope, line: int, prefix: str = "::") -> str | None:
    """Find the deepest scope label containing the given line."""
    for child in scope.children:
        child_label = f"{prefix}::{child.name}" if prefix != "::" else f"::{child.name}"
        if child.body_range and child.body_range.start.line <= line <= child.body_range.end.line:
            deeper = _scope_label_at_line(child, line, child_label)
            if deeper:
                return deeper
            return child_label
    if scope.kind == "proc":
        return prefix
    return None


# _apply_edits — delegated to core.common.text_edits
_apply_edits = apply_edits


# Whitespace minification (original engine)


def _minify_body(source: str, *, dialect: str | None = None) -> str:
    """Minify a Tcl script body (top-level or inside braces)."""
    lexer = TclLexer(source)
    commands: list[list[_Arg]] = []
    current_args: list[_Arg] = []
    prev_type = TokenType.EOL

    while True:
        tok = lexer.get_token()
        if tok is None:
            break

        match tok.type:
            case TokenType.COMMENT:
                continue
            case TokenType.SEP:
                prev_type = tok.type
                continue
            case TokenType.EOL:
                if current_args:
                    commands.append(current_args)
                    current_args = []
                prev_type = tok.type
                continue
            case _:
                pass

        is_start_of_new_arg = prev_type in (TokenType.SEP, TokenType.EOL)

        # Detect quoted context from source.
        detected_quoted = False
        if is_start_of_new_arg and tok.start.offset < len(source):
            if source[tok.start.offset] == '"':
                detected_quoted = True

        if is_start_of_new_arg:
            current_args.append(
                _Arg(
                    tokens=[tok],
                    is_braced=tok.type == TokenType.STR,
                    is_quoted=detected_quoted,
                )
            )
        else:
            if current_args:
                current_args[-1].tokens.append(tok)
            else:
                current_args.append(
                    _Arg(
                        tokens=[tok],
                        is_braced=tok.type == TokenType.STR,
                        is_quoted=detected_quoted,
                    )
                )

        prev_type = tok.type

    if current_args:
        commands.append(current_args)

    if not commands:
        return ""

    # --- Render each arg to its string form. ---
    rendered_commands: list[list[str]] = []
    for cmd_args in commands:
        cmd_name = _token_text(cmd_args[0]) if cmd_args else ""
        body_indices = _get_body_indices(cmd_name, cmd_args)
        expr_indices = _get_expr_indices(cmd_name, cmd_args)

        arg_strs: list[str] = []
        for i, arg in enumerate(cmd_args):
            if i in body_indices and arg.is_braced and len(arg.tokens) == 1:
                inner = arg.tokens[0].text
                minified_inner = _minify_body(inner, dialect=dialect)
                arg_strs.append("{" + minified_inner + "}")
            elif i in expr_indices and arg.is_braced and len(arg.tokens) == 1:
                inner = arg.tokens[0].text
                compressed = _compress_expr(inner, dialect=dialect)
                arg_strs.append("{" + compressed + "}")
            else:
                arg_strs.append(_reconstruct_arg(arg, dialect=dialect))

        if len(arg_strs) >= 2:
            arg_strs[1] = _abbreviated_subcommand(arg_strs[0], arg_strs[1], dialect)

        rendered_commands.append(arg_strs)

    # --- Template deduplication (subst aliasing). ---
    # Find quoted args with dynamic content that repeat identically.
    # Replace with [subst $alias] and prepend set alias {content}.
    template_map, rendered_commands = _dedup_templates(rendered_commands)

    preamble_parts: list[str] = []
    for content, alias in template_map.items():
        preamble_parts.append(f"set {alias} {{{content}}}")

    parts: list[str] = preamble_parts
    is_irules = dialect == "f5-irules"
    for arg_strs in rendered_commands:
        if is_irules and len(arg_strs) > 1:
            # In iRules, }{ is a valid word boundary — omit spaces between
            # adjacent braced args to save bytes.
            pieces = [arg_strs[0]]
            for prev, cur in zip(arg_strs, arg_strs[1:]):
                if prev.endswith("}") and cur.startswith("{"):
                    pieces.append(cur)
                else:
                    pieces.append(" " + cur)
            parts.append("".join(pieces))
        else:
            parts.append(" ".join(arg_strs))

    return ";".join(parts)


def _dedup_templates(
    rendered_commands: list[list[str]],
) -> tuple[dict[str, str], list[list[str]]]:
    """Find repeated quoted string args and replace with [subst $alias].

    A quoted arg ``"content"`` is eligible when:

    - It contains at least one ``$`` or ``[`` (dynamic content — otherwise
      plain literal aliasing already handles it).
    - The content can be safely brace-quoted (balanced braces).
    - The cost-benefit works out: ``[subst $alias]`` per use must be cheaper
      than the original ``"content"`` per use, accounting for the preamble.

    ``[subst {content}]`` is semantically identical to ``"content"`` by Tcl
    spec — same three substitution types, same evaluation order.  Because
    both preamble and usage are in the same scope (inside ``_minify_body``),
    variable references resolve identically.

    Returns ``(template_map, modified_commands)`` where *template_map* maps
    ``{content: alias}`` for each deduplicated template.
    """
    # Collect all quoted args: content → list of (cmd_idx, arg_idx).
    quoted_uses: dict[str, list[tuple[int, int]]] = {}
    for cmd_idx, arg_strs in enumerate(rendered_commands):
        for arg_idx, arg_str in enumerate(arg_strs):
            if not arg_str.startswith('"') or not arg_str.endswith('"'):
                continue
            content = arg_str[1:-1]
            # Must have dynamic content.
            if "$" not in content and "[" not in content:
                continue
            # Must be long enough to benefit.
            if len(content) < 10:
                continue
            # Must have balanced braces for safe brace-quoting.
            if content.count("{") != content.count("}"):
                continue
            quoted_uses.setdefault(content, []).append((cmd_idx, arg_idx))

    if not quoted_uses:
        return {}, rendered_commands

    # Collect all names that already appear as $var references in the output
    # so template aliases don't shadow them.
    _used_var_re = re.compile(r"\$\{?(\w+)")
    used_names: set[str] = set()
    for arg_strs in rendered_commands:
        for arg_str in arg_strs:
            used_names.update(_used_var_re.findall(arg_str))

    alias_gen = _name_generator()
    template_map: dict[str, str] = {}

    for content, uses in sorted(quoted_uses.items(), key=lambda x: -len(x[1]) * len(x[0])):
        count = len(uses)
        if count < 2:
            continue

        alias = next(alias_gen)
        while alias in used_names:
            alias = next(alias_gen)

        original_cost = count * (len(content) + 2)  # "content" per use
        preamble_cost = 4 + len(alias) + 1 + 1 + len(content) + 1 + 1  # set a {content}\n
        subst_ref = f"[subst ${alias}]"
        aliased_cost = preamble_cost + count * len(subst_ref)

        if aliased_cost >= original_cost:
            continue

        # Safety: ensure the template doesn't reference $alias (circular).
        if f"${alias}" in content or f"${{{alias}}}" in content:
            continue

        template_map[content] = alias
        used_names.add(alias)

    if not template_map:
        return {}, rendered_commands

    # Apply replacements.
    result = [list(arg_strs) for arg_strs in rendered_commands]
    for content, alias in template_map.items():
        subst_ref = f"[subst ${alias}]"
        for cmd_idx, arg_idx in quoted_uses[content]:
            result[cmd_idx][arg_idx] = subst_ref

    return template_map, result


class _Arg:
    """Lightweight argument accumulator for minification."""

    __slots__ = ("tokens", "is_braced", "is_quoted")

    def __init__(
        self,
        tokens: list[Token],
        is_braced: bool,
        is_quoted: bool,
    ) -> None:
        self.tokens = tokens
        self.is_braced = is_braced
        self.is_quoted = is_quoted


def _token_text(arg: _Arg) -> str:
    """Get the plain text of the first token in an argument."""
    return arg.tokens[0].text if arg.tokens else ""


def _reconstruct_raw(
    tok: Token,
    *,
    next_tok: Token | None = None,
    dialect: str | None = None,
) -> str:
    """Rebuild source text from a single token, re-adding delimiters.

    When *next_tok* is supplied (inside quoted strings), ``$var`` is rendered
    as ``${var}`` if the next token starts with a word character — otherwise
    Tcl would parse them as one longer variable name.
    """
    match tok.type:
        case TokenType.STR:
            return "{" + tok.text + "}"
        case TokenType.CMD:
            return "[" + _minify_body(tok.text, dialect=dialect) + "]"
        case TokenType.VAR:
            if next_tok is not None:
                # What character will follow $var in the output?
                # VAR tokens render as "$text", CMD as "[text]", etc.
                first_rendered = _first_rendered_char(next_tok)
                if first_rendered and (first_rendered.isalnum() or first_rendered == "_"):
                    return "${" + tok.text + "}"
            return "$" + tok.text
        case TokenType.EXPAND:
            return "{*}"
        case _:
            return tok.text


def _first_rendered_char(tok: Token) -> str:
    """Return the first character that *tok* will produce when rendered."""
    match tok.type:
        case TokenType.STR:
            return "{"
        case TokenType.CMD:
            return "["
        case TokenType.VAR:
            return "$"
        case TokenType.EXPAND:
            return "{"
        case _:
            return tok.text[0:1] if tok.text else ""


# Characters that would change semantics if they appear unquoted.
# Note: { and } are handled specially (allowed inside ${var} syntax).
_NEEDS_QUOTING = frozenset(' \t\n\r\v\f;"\x00')

# Matches ${varname} — the braces here are safe unquoted.
_BRACED_VAR_RE = re.compile(r"\$\{[^}]+\}")


def _can_strip_quotes(raw: str, tokens: list) -> bool:
    """Check whether a quoted argument can safely drop its double quotes.

    The content *raw* is the already-reconstructed inner text.  Quotes
    can be removed when:
    - The content is non-empty.
    - It doesn't start with ``"`` or ``{`` (would start a new quoting
      context) or ``#`` (would be parsed as a comment if first arg on line,
      but we skip that edge case and keep quotes to be safe).
    - It contains no characters that are special in unquoted context:
      whitespace, ``;``, ``"``.
    - Any ``{`` or ``}`` characters are part of ``${var}`` syntax.
    - It doesn't consist solely of ``{*}`` (expansion syntax).
    """
    if not raw:
        return False
    if raw[0] in ('"', "{", "#"):
        return False
    if raw == "{*}":
        return False
    if _NEEDS_QUOTING.intersection(raw):
        return False
    # Check for bare braces outside of ${var} references.
    stripped = _BRACED_VAR_RE.sub("", raw)
    if "{" in stripped or "}" in stripped:
        return False
    return True


def _reconstruct_arg(arg: _Arg, *, dialect: str | None = None) -> str:
    """Rebuild the source text of an argument from its tokens."""
    parts: list[str] = []
    tokens = arg.tokens
    for idx, t in enumerate(tokens):
        next_t = tokens[idx + 1] if idx + 1 < len(tokens) else None
        parts.append(
            _reconstruct_raw(
                t,
                next_tok=next_t if arg.is_quoted else None,
                dialect=dialect,
            )
        )
    raw = "".join(parts)
    if arg.is_quoted:
        if _can_strip_quotes(raw, tokens):
            return raw
        return '"' + raw + '"'
    return raw


def _get_body_indices(cmd_name: str, args: list[_Arg]) -> frozenset[int]:
    """Return 0-based arg indices (including command name at 0) that are bodies."""
    if not cmd_name:
        return frozenset()
    arg_texts = [_token_text(a) for a in args[1:]]
    try:
        raw_indices = body_arg_indices(cmd_name, arg_texts)
    except Exception:
        return frozenset()
    if raw_indices is None:
        return frozenset()
    # body_arg_indices returns indices relative to args[1:], so offset by 1.
    return frozenset(i + 1 for i in raw_indices)


def _get_expr_indices(cmd_name: str, args: list[_Arg]) -> frozenset[int]:
    """Return 0-based arg indices (including command name at 0) that are expressions."""
    if not cmd_name:
        return frozenset()
    arg_texts = [_token_text(a) for a in args[1:]]
    try:
        raw_indices = expr_arg_indices(cmd_name, arg_texts)
    except Exception:
        return frozenset()
    if raw_indices is None:
        return frozenset()
    return frozenset(i + 1 for i in raw_indices)


# Tcl expr word-operators that need surrounding whitespace.
_WORD_OPS = re.compile(r"\b(eq|ne|in|ni)\b")

# Whitespace around tokens in expr that can be collapsed — everything except
# around word-operators and inside [...] command substitution or "..." strings.
_EXPR_TOKEN = re.compile(
    r"""
    "(?:[^"\\]|\\.)*"    # double-quoted string
    | \[(?:[^\[\]]|\[(?:[^\[\]])*\])*\]  # [...] command substitution (1 level)
    | \$[\w:]+            # variable reference
    | [\w.]+              # number or function name
    | [+\-*/%<>=!&|^?:~]+ # symbolic operator(s)
    | [()]                # parens
    | \s+                 # whitespace
    """,
    re.VERBOSE,
)


def _compress_expr(text: str, *, dialect: str | None = None) -> str:
    """Compress and shrink an ``expr`` body for minimum size.

    1. Strip whitespace around symbolic operators.
    2. Parse the expression AST and try De Morgan / comparison inversion
       variants, picking the shortest rendering.
    """
    compressed = _strip_expr_whitespace(text, dialect=dialect)
    # Try AST-based shrinking for further gains.
    shrunk = _shrink_expr_ast(compressed, dialect=dialect)
    return shrunk if len(shrunk) < len(compressed) else compressed


def _strip_expr_whitespace(text: str, *, dialect: str | None = None) -> str:
    """Remove unnecessary whitespace inside an ``expr`` body.

    Preserves required whitespace around word operators (``eq``, ``ne``,
    ``in``, ``ni``) and inside ``[...]`` command substitution and
    ``"..."`` strings.
    """
    tokens: list[str] = []
    for m in _EXPR_TOKEN.finditer(text):
        tok = m.group()
        if tok.isspace():
            # Keep whitespace only if the previous or next token is a word-op,
            # a word (identifier/number), or nothing yet.
            continue  # We'll re-insert spaces only where needed below.
        if tok.startswith("[") and tok.endswith("]") and len(tok) >= 2:
            inner = tok[1:-1]
            tok = "[" + _minify_body(inner, dialect=dialect) + "]"
        tokens.append(tok)

    if not tokens:
        return text

    # Rebuild with spaces only around word-operators and between adjacent
    # word tokens (e.g. "int ($x)" needs no space, "$a eq $b" does).
    parts: list[str] = [tokens[0]]
    for i in range(1, len(tokens)):
        prev = tokens[i - 1]
        curr = tokens[i]
        need_space = False
        # Space needed around word operators.
        if _WORD_OPS.fullmatch(prev) or _WORD_OPS.fullmatch(curr):
            need_space = True
        # Space between two word tokens (e.g. "double $x" function call).
        elif _is_word_token(prev) and _is_word_token(curr):
            need_space = True
        if need_space:
            parts.append(" ")
        parts.append(curr)
    return "".join(parts)


def _is_word_token(tok: str) -> bool:
    """Return True if token is a word (identifier, number, variable)."""
    if not tok:
        return False
    if tok[0] in ("$", '"') or tok[0].isalnum() or tok[0] == "_":
        return True
    if tok.startswith("["):
        return True
    return False


# Comparison inversion map: operator → its logical complement.
_COMPARISON_INVERSION: dict[str, str] = {
    "==": "!=",
    "!=": "==",
    "<": ">=",
    ">=": "<",
    ">": "<=",
    "<=": ">",
    "eq": "ne",
    "ne": "eq",
    "in": "ni",
    "ni": "in",
    "lt": "ge",
    "ge": "lt",
    "gt": "le",
    "le": "gt",
}


def _shrink_expr_ast(text: str, *, dialect: str | None = None) -> str:
    """Try AST-based expression transforms that reduce size.

    Transforms attempted:
    - Comparison inversion: ``!($a==$b)`` → ``$a!=$b``
    - De Morgan forward: ``!($a&&$b)`` → ``!$a||!$b``  (if shorter)
    - De Morgan reverse: ``!$a||!$b`` → ``!($a&&$b)``  (if shorter)
    - Double negation: ``!!$x`` → ``$x``
    """
    try:
        from core.compiler.expr_ast import (
            ExprRaw,
            render_expr,
        )
        from core.parsing.expr_parser import parse_expr
    except Exception:
        return text

    try:
        node = parse_expr(text)
    except Exception:
        return text
    if isinstance(node, ExprRaw):
        return text

    # Try shrinking the parsed AST.
    shrunk = _shrink_node(node)
    if shrunk is node:
        return text

    rendered = render_expr(shrunk)
    # Re-compress whitespace in the rendered output.
    compressed = _strip_expr_whitespace(rendered, dialect=dialect)
    return compressed


def _shrink_node(node):
    """Recursively try size-reducing transforms on an ExprNode."""
    from core.compiler.expr_ast import (  # noqa: F811
        BinOp,
        ExprBinary,
        ExprTernary,
        ExprUnary,
        UnaryOp,
        render_expr,
    )

    match node:
        case ExprUnary(op=UnaryOp.NOT, operand=ExprUnary(op=UnaryOp.NOT, operand=inner)):
            # Double negation: !!x → x
            return _shrink_node(inner)

        case ExprUnary(op=UnaryOp.NOT, operand=ExprBinary(op=cmp_op, left=left, right=right)) if (
            cmp_op.value in _COMPARISON_INVERSION
        ):
            # Comparison inversion: !($a == $b) → $a != $b
            inv_op_str = _COMPARISON_INVERSION[cmp_op.value]
            inv_op = BinOp(inv_op_str)
            inverted = ExprBinary(op=inv_op, left=_shrink_node(left), right=_shrink_node(right))
            original_text = render_expr(node)
            inverted_text = render_expr(inverted)
            return inverted if len(inverted_text) < len(original_text) else node

        case ExprUnary(
            op=UnaryOp.NOT,
            operand=ExprBinary(op=BinOp.AND | BinOp.WORD_AND, left=left, right=right),
        ):
            # De Morgan forward: !(a && b) → !a || !b
            neg_l = ExprUnary(op=UnaryOp.NOT, operand=_shrink_node(left))
            neg_r = ExprUnary(op=UnaryOp.NOT, operand=_shrink_node(right))
            dual_op = BinOp.OR if node.operand.op == BinOp.AND else BinOp.WORD_OR
            # Recursively shrink the negated children (may trigger comparison inversion)
            demorgan_shrunk = ExprBinary(
                op=dual_op,
                left=_shrink_node(neg_l),
                right=_shrink_node(neg_r),
            )
            original_text = render_expr(node)
            dm_text = render_expr(demorgan_shrunk)
            return demorgan_shrunk if len(dm_text) < len(original_text) else node

        case ExprUnary(
            op=UnaryOp.NOT, operand=ExprBinary(op=BinOp.OR | BinOp.WORD_OR, left=left, right=right)
        ):
            # De Morgan forward: !(a || b) → !a && !b
            neg_l = ExprUnary(op=UnaryOp.NOT, operand=_shrink_node(left))
            neg_r = ExprUnary(op=UnaryOp.NOT, operand=_shrink_node(right))
            dual_op = BinOp.AND if node.operand.op == BinOp.OR else BinOp.WORD_AND
            demorgan_shrunk = ExprBinary(
                op=dual_op,
                left=_shrink_node(neg_l),
                right=_shrink_node(neg_r),
            )
            original_text = render_expr(node)
            dm_text = render_expr(demorgan_shrunk)
            return demorgan_shrunk if len(dm_text) < len(original_text) else node

        case ExprBinary(
            op=BinOp.OR | BinOp.WORD_OR,
            left=ExprUnary(op=UnaryOp.NOT, operand=a),
            right=ExprUnary(op=UnaryOp.NOT, operand=b),
        ):
            # De Morgan reverse: !a || !b → !(a && b) (if shorter)
            dual_op = BinOp.AND if node.op == BinOp.OR else BinOp.WORD_AND
            combined = ExprUnary(
                op=UnaryOp.NOT,
                operand=ExprBinary(op=dual_op, left=_shrink_node(a), right=_shrink_node(b)),
            )
            original_text = render_expr(node)
            combined_text = render_expr(combined)
            return combined if len(combined_text) < len(original_text) else node

        case ExprBinary(
            op=BinOp.AND | BinOp.WORD_AND,
            left=ExprUnary(op=UnaryOp.NOT, operand=a),
            right=ExprUnary(op=UnaryOp.NOT, operand=b),
        ):
            # De Morgan reverse: !a && !b → !(a || b) (if shorter)
            dual_op = BinOp.OR if node.op == BinOp.AND else BinOp.WORD_OR
            combined = ExprUnary(
                op=UnaryOp.NOT,
                operand=ExprBinary(op=dual_op, left=_shrink_node(a), right=_shrink_node(b)),
            )
            original_text = render_expr(node)
            combined_text = render_expr(combined)
            return combined if len(combined_text) < len(original_text) else node

        case ExprBinary(op=op, left=left, right=right):
            # Recurse into children.
            new_left = _shrink_node(left)
            new_right = _shrink_node(right)
            if new_left is not left or new_right is not right:
                return ExprBinary(op=op, left=new_left, right=new_right)
            return node

        case ExprUnary(op=op, operand=operand):
            new_operand = _shrink_node(operand)
            if new_operand is not operand:
                return ExprUnary(op=op, operand=new_operand)
            return node

        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            new_cond = _shrink_node(cond)
            new_tb = _shrink_node(tb)
            new_fb = _shrink_node(fb)
            if new_cond is not cond or new_tb is not tb or new_fb is not fb:
                return ExprTernary(condition=new_cond, true_branch=new_tb, false_branch=new_fb)
            return node

        case _:
            return node
