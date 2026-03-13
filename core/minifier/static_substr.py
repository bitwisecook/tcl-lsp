"""Static substring optimisation using IR/CFG/SSA/SCCP and taint analysis.

This module analyses quoted strings in the source and determines which
ones can be fully evaluated at compile time.  The analysis handles:

- **Variable substitutions** (``$var``) — folded when SCCP proves the
  variable holds a compile-time constant.
- **Pure command substitutions** (``[string length ...]``,
  ``[expr {...}]``, ``[format ...]``, etc.) — folded when the command
  is pure, deterministic, and all its arguments are themselves constant.
- **Interprocedural constants** (stub — not yet implemented) — when a
  ``$var`` is a parameter of a procedure that is only ever called with
  constant arguments, the interprocedural analysis could provide the
  constant value.

The analysis uses the full compiler pipeline:

- **IR** — structured intermediate representation with source ranges
- **CFG** — control flow graph with basic blocks
- **SSA** — static single-assignment form (variable versioning)
- **SCCP** — sparse conditional constant propagation (lattice values)
- **Taint** — rejects folding when tainted values lack sanitisation
  colours that prove a fixed form (e.g. ``IP_ADDRESS``, ``PORT``)

After folding, a **dead-set elimination** pass removes ``set`` commands
whose only purpose was to define a variable that is now folded away.

The public entry point is :func:`fold_static_substrings`.
"""

from __future__ import annotations

import logging
import re
from typing import TYPE_CHECKING

from core.common.text_edits import apply_edits

if TYPE_CHECKING:
    from core.compiler.compilation_unit import FunctionUnit
    from core.compiler.core_analyses import LatticeValue
    from core.compiler.interprocedural import InterproceduralAnalysis
    from core.compiler.ssa import SSAValueKey
    from core.compiler.taint import TaintLattice

log = logging.getLogger(__name__)

# Taint colours that indicate a fixed-form value safe for folding even
# when the underlying value is tainted.  These prove the value has a
# constrained shape (e.g. a validated IP address, port number, FQDN)
# and will not vary unpredictably between requests.
_SAFE_TAINT_COLOURS: int = 0  # populated at import time below

try:
    from core.commands.registry.taint_hints import TaintColour

    _SAFE_TAINT_COLOURS = (
        TaintColour.IP_ADDRESS
        | TaintColour.PORT
        | TaintColour.FQDN
        | TaintColour.LIST_CANONICAL
        | TaintColour.REGEX_LITERAL
    ).value
except Exception:
    pass


def fold_static_substrings(source: str) -> tuple[str, int, dict[str, str]]:
    """Replace dynamic quoted strings with their static values where provable.

    Analyses each quoted-string argument containing ``$var`` or ``[cmd]``
    substitutions.  When SCCP proves every substitution resolves to a
    constant and no unsanitised tainted values are involved, the string
    is replaced with its fully-evaluated literal form.

    After folding, dead ``set`` commands (whose variable is no longer
    referenced) are removed.

    Returns ``(modified_source, fold_count, fold_map)`` where
    *fold_count* is the number of strings folded and *fold_map* maps
    original dynamic content to the folded static value.
    """
    from core.compiler.compilation_unit import compile_source

    try:
        cu = compile_source(source)
    except Exception:
        log.debug("static_substr: compilation failed; skipping static folding")
        return source, 0, {}

    edits: list[tuple[int, int, str]] = []
    fold_map: dict[str, str] = {}
    folded_vars: set[str] = set()  # Variables consumed by folds.

    # Process each function scope (top-level + procedures).
    scopes: list[tuple[str, FunctionUnit]] = [("::top", cu.top_level)]
    for qname, fu in cu.procedures.items():
        scopes.append((qname, fu))

    for _scope_name, fu in scopes:
        _collect_folds_for_scope(source, fu, cu.interproc, edits, fold_map, folded_vars)

    if not edits:
        return source, 0, {}

    # Deduplicate edits by (offset, length).
    seen: set[tuple[int, int]] = set()
    unique_edits: list[tuple[int, int, str]] = []
    for offset, length, new_text in edits:
        key = (offset, length)
        if key not in seen:
            seen.add(key)
            unique_edits.append((offset, length, new_text))

    result = apply_edits(source, unique_edits)

    # Dead-set elimination: remove `set var value` commands where $var
    # was only used in strings that have now been folded away.
    result, dead_count = _eliminate_dead_sets(result, folded_vars)
    fold_count = len(unique_edits)

    return result, fold_count + dead_count, fold_map


def _collect_folds_for_scope(
    source: str,
    fu: FunctionUnit,
    interproc: InterproceduralAnalysis,
    edits: list[tuple[int, int, str]],
    fold_map: dict[str, str],
    folded_vars: set[str],
) -> None:
    """Collect static-fold edits for a single function scope."""
    from core.compiler.ir import IRAssignValue, IRCall

    cfg = fu.cfg
    ssa = fu.ssa
    analysis = fu.analysis
    values = analysis.values
    taints = analysis.taints

    for block_name, block in cfg.blocks.items():
        ssa_block = ssa.blocks.get(block_name)
        if ssa_block is None:
            continue

        if block_name in analysis.unreachable_blocks:
            continue

        # Build a combined uses dict: SSA entry versions + accumulated
        # definitions.  This is needed because [expr {$x}] inside a
        # quoted string doesn't register $x in the statement's uses
        # (it's evaluated by expr, not by string interpolation), but
        # we still need to look up x's SCCP value.
        block_vars: dict[str, int] = dict(ssa_block.entry_versions)

        for stmt_idx, stmt in enumerate(block.statements):
            if stmt_idx >= len(ssa_block.statements):
                continue

            ssa_stmt = ssa_block.statements[stmt_idx]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue

            # Merge statement's uses with block-level variable versions.
            combined_uses = {**block_vars, **ssa_stmt.uses}

            # Case 1: IRAssignValue with interpolation — "set x <value>"
            # where value contains $var refs that SCCP can resolve.
            if isinstance(stmt, IRAssignValue) and stmt.value_needs_backsubst:
                _try_fold_assign_value(
                    source,
                    stmt,
                    combined_uses,
                    values,
                    taints,
                    interproc,
                    edits,
                    fold_map,
                    folded_vars,
                )

            # Case 2: IRCall with CommandTokens — check each argument.
            if isinstance(stmt, IRCall) and stmt.tokens is not None:
                _try_fold_call_args(
                    source,
                    stmt,
                    combined_uses,
                    values,
                    taints,
                    interproc,
                    edits,
                    fold_map,
                    folded_vars,
                )

            # Update block_vars with definitions from this statement.
            for name, ver in ssa_stmt.defs.items():
                block_vars[name] = ver


def _try_fold_assign_value(
    source: str,
    stmt,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
    taints: dict[SSAValueKey, TaintLattice],
    interproc: InterproceduralAnalysis,
    edits: list[tuple[int, int, str]],
    fold_map: dict[str, str],
    folded_vars: set[str],
) -> None:
    """Try to fold an IRAssignValue's interpolated value to a constant."""
    value = stmt.value
    if "$" not in value and "[" not in value:
        return

    folded = _fold_string_via_sccp(value, uses, values, interproc)
    if folded is None:
        return

    if _has_unsafe_tainted_inputs(value, uses, taints):
        return

    # Record which variables were consumed by this fold.
    _record_consumed_vars(value, folded_vars)

    # Find the quoted string in the source text and replace it.
    r = stmt.range
    _emit_fold_edits_for_region(
        source, r.start.offset, r.end.offset + 1, value, folded, edits, fold_map
    )


def _try_fold_call_args(
    source: str,
    stmt,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
    taints: dict[SSAValueKey, TaintLattice],
    interproc: InterproceduralAnalysis,
    edits: list[tuple[int, int, str]],
    fold_map: dict[str, str],
    folded_vars: set[str],
) -> None:
    """Try to fold quoted-string arguments in an IRCall."""
    tokens = stmt.tokens
    if tokens is None:
        return

    # Walk each argument (skip command name at index 0).
    for arg_idx in range(1, len(tokens.argv)):
        arg_tok = tokens.argv[arg_idx]
        arg_text = tokens.argv_texts[arg_idx]

        # Only fold quoted strings (multi-token words with $var refs).
        # tokens.argv offsets point at the opening " for quoted strings.
        arg_offset = arg_tok.start.offset
        if arg_offset >= len(source):
            continue

        # Check if this is a quoted string in the source.
        if source[arg_offset] != '"':
            continue

        content = arg_text
        if "$" not in content and "[" not in content:
            continue

        folded = _fold_string_via_sccp(content, uses, values, interproc)
        if folded is None:
            continue

        if _has_unsafe_tainted_inputs(content, uses, taints):
            continue

        # Find the closing quote.
        close = _find_close_quote(source, arg_offset + 1)
        if close is None:
            continue

        # Record consumed variables.
        _record_consumed_vars(content, folded_vars)

        # Replace "dynamic_content" with the static equivalent.
        replacement = _build_replacement(folded)
        edits.append((arg_offset, close - arg_offset + 1, replacement))
        fold_map[content] = folded


# ---------------------------------------------------------------------------
# Dead-set elimination
# ---------------------------------------------------------------------------

# Matches `set varname value` as a full command (with possible leading whitespace).
# Handles braced {…} and quoted "…" values that may contain semicolons.
_SET_CMD_RE = re.compile(
    r"""(?:^|(?<=;))             # start of string or after semicolon
    (                            # group 1: entire set command body
      [ \t]*set[ \t]+
      (\S+)                      # group 2: variable name
      [ \t]+
      (?:                        # value: one or more word fragments
        \{[^{}]*\}               # braced word (no nesting)
        | "(?:[^"\\]|\\.)*"      # quoted word
        | [^\s;\n]+              # bare word
      )
      (?:                        # optional additional fragments
        [ \t]+
        (?:\{[^{}]*\}|"(?:[^"\\]|\\.)*"|[^\s;\n]+)
      )*
    )
    (?:;|\n|$)                   # terminator
    """,
    re.MULTILINE | re.VERBOSE,
)


def _eliminate_dead_sets(source: str, folded_vars: set[str]) -> tuple[str, int]:
    """Remove ``set var value`` commands whose variable was folded away.

    Only removes a ``set`` if the variable name is in *folded_vars* AND
    there are no remaining ``$var`` references in the source after
    folding.  Returns ``(modified_source, removal_count)``.
    """
    if not folded_vars:
        return source, 0

    removals = 0
    for var_name in folded_vars:
        # Check if $var is still referenced anywhere ($ form).
        if f"${var_name}" in source or f"${{{var_name}}}" in source:
            # Still referenced — keep the set.  This can happen when only
            # some uses were folded.
            continue

        # Also check for non-$ reads: [set var], incr var, unset var, etc.
        # These reference the variable by name without a $ prefix.
        if re.search(
            r"\b(?:set|incr|append|lappend|unset|info\s+exists)\s+"
            + re.escape(var_name)
            + r"(?:\s|;|\]|$)",
            source,
        ):
            continue

        # Find and remove `set varname ...` commands.
        def _remove_set(m: re.Match) -> str:
            if m.group(2) == var_name:
                nonlocal removals
                removals += 1
                return ""
            return m.group(0)

        source = _SET_CMD_RE.sub(_remove_set, source)

    # Clean up stray semicolons and blank lines left by removals.
    if removals:
        source = re.sub(r";;+", ";", source)
        source = re.sub(r"^\s*;\s*$", "", source, flags=re.MULTILINE)
        source = re.sub(r"\n{3,}", "\n\n", source)
        source = source.strip("\n") + "\n" if source.strip() else source

    return source, removals


# ---------------------------------------------------------------------------
# Pure command evaluation
# ---------------------------------------------------------------------------

# Supported pure commands that we can evaluate at compile time.
_PURE_STRING_SUBCMDS = frozenset(
    {
        "length",
        "cat",
        "toupper",
        "tolower",
        "trim",
        "trimleft",
        "trimright",
        "reverse",
        "repeat",
        "index",
        "range",
        "first",
        "last",
        "replace",
        "totitle",
        "map",
    }
)


def _try_eval_pure_cmd(cmd_text: str, uses: dict[str, int], values: dict) -> str | None:
    """Try to evaluate a pure command substitution to a constant.

    Handles ``[expr {...}]``, ``[string length ...]``, ``[format ...]``,
    ``[llength ...]``, ``[lindex ...]``, ``[join ...]``, ``[concat ...]``,
    and other pure commands when all arguments are constant.
    """
    from core.compiler.tcl_expr_eval import eval_tcl_expr_str, format_tcl_value

    # Strip the surrounding [ ].
    if not cmd_text.startswith("[") or not cmd_text.endswith("]"):
        return None
    inner = cmd_text[1:-1].strip()
    if not inner:
        return None

    parts = _split_simple_command(inner)
    if not parts:
        return None

    cmd_name = parts[0]
    cmd_args = parts[1:]

    # Resolve any $var references in the arguments.
    resolved_args: list[str] = []
    for arg in cmd_args:
        resolved = _resolve_arg_constants(arg, uses, values)
        if resolved is None:
            return None
        resolved_args.append(resolved)

    # expr — evaluate the expression directly.
    if cmd_name == "expr" and len(resolved_args) == 1:
        expr_text = resolved_args[0]
        # Strip braces if present.
        if expr_text.startswith("{") and expr_text.endswith("}"):
            expr_text = expr_text[1:-1]
        result = eval_tcl_expr_str(expr_text)
        if result is not None:
            return format_tcl_value(result)
        return None

    # string subcommand — evaluate common pure subcommands.
    if cmd_name == "string" and resolved_args:
        subcmd = resolved_args[0]
        str_args = resolved_args[1:]
        return _eval_string_subcmd(subcmd, str_args)

    # format — evaluate format string.
    if cmd_name == "format" and resolved_args:
        return _eval_format(resolved_args)

    # llength — list length.
    if cmd_name == "llength" and len(resolved_args) == 1:
        return str(len(_tcl_list_split(resolved_args[0])))

    # lindex — list index.
    if cmd_name == "lindex" and len(resolved_args) == 2:
        elements = _tcl_list_split(resolved_args[0])
        try:
            idx = int(resolved_args[1])
            if 0 <= idx < len(elements):
                return elements[idx]
        except (ValueError, IndexError):
            pass
        return None

    # join — join list with separator.
    if cmd_name == "join" and 1 <= len(resolved_args) <= 2:
        elements = _tcl_list_split(resolved_args[0])
        sep = resolved_args[1] if len(resolved_args) > 1 else " "
        return sep.join(elements)

    # concat — concatenate words.
    if cmd_name == "concat":
        return " ".join(resolved_args)

    # list — make a canonical Tcl list.
    if cmd_name == "list":
        return " ".join(_tcl_list_element(a) for a in resolved_args)

    return None


def _split_simple_command(text: str) -> list[str]:
    """Split a simple Tcl command into words (no nested substitutions)."""
    words: list[str] = []
    pos = 0
    n = len(text)

    while pos < n:
        # Skip whitespace.
        while pos < n and text[pos] in " \t":
            pos += 1
        if pos >= n:
            break

        ch = text[pos]
        if ch == '"':
            # Quoted word — find closing quote.
            end = pos + 1
            while end < n and text[end] != '"':
                if text[end] == "\\":
                    end += 1
                end += 1
            if end < n:
                end += 1  # include closing quote
            words.append(text[pos + 1 : end - 1])
            pos = end
        elif ch == "{":
            # Braced word — find matching close brace.
            depth = 1
            end = pos + 1
            while end < n and depth > 0:
                if text[end] == "{":
                    depth += 1
                elif text[end] == "}":
                    depth -= 1
                elif text[end] == "\\":
                    end += 1
                end += 1
            words.append(text[pos + 1 : end - 1])
            pos = end
        else:
            # Bare word.
            end = pos
            while end < n and text[end] not in " \t\n;":
                if text[end] == "\\":
                    end += 1
                end += 1
            words.append(text[pos:end])
            pos = end

    return words


def _resolve_arg_constants(
    arg: str,
    uses: dict[str, int],
    values: dict,
) -> str | None:
    """Resolve $var references in *arg* to their constant values.

    Returns the fully-resolved string, or ``None`` if any variable
    is not a known constant.
    """
    from core.compiler.core_analyses import LatticeKind

    if "$" not in arg:
        return arg

    pieces: list[str] = []
    pos = 0
    n = len(arg)

    while pos < n:
        if arg[pos] == "$":
            # Parse variable reference.
            var_end, var_name = _parse_var_ref(arg, pos)
            if var_name is None:
                return None
            ver = uses.get(var_name, 0)
            if ver <= 0:
                return None
            lv = values.get((var_name, ver))
            if lv is None or lv.kind is not LatticeKind.CONST:
                return None
            pieces.append(str(lv.value))
            pos = var_end
        elif arg[pos] == "\\":
            if pos + 1 < n:
                pieces.append(arg[pos + 1])
                pos += 2
            else:
                pieces.append("\\")
                pos += 1
        else:
            pieces.append(arg[pos])
            pos += 1

    return "".join(pieces)


def _parse_var_ref(text: str, pos: int) -> tuple[int, str | None]:
    """Parse a $var or ${var} reference starting at *pos*.

    Returns ``(end_pos, var_name)`` or ``(pos+1, None)`` on failure.
    """
    if pos >= len(text) or text[pos] != "$":
        return pos + 1, None

    start = pos + 1
    if start >= len(text):
        return start, None

    if text[start] == "{":
        # ${varname}
        close = text.find("}", start + 1)
        if close < 0:
            return start, None
        return close + 1, text[start + 1 : close]

    # $varname — word characters only.
    end = start
    while end < len(text) and (text[end].isalnum() or text[end] == "_"):
        end += 1
    if end == start:
        return start, None
    # Reject array element forms ($arr(idx)) — we cannot safely fold these.
    # Also reject namespace-qualified forms ($ns::var) — detected by `::` after the name.
    if end < len(text):
        if text[end] == "(":
            return start, None
        if text[end] == ":" and end + 1 < len(text) and text[end + 1] == ":":
            return start, None
    return end, text[start:end]


# ---------------------------------------------------------------------------
# Pure command evaluators
# ---------------------------------------------------------------------------


def _eval_string_subcmd(subcmd: str, args: list[str]) -> str | None:
    """Evaluate a pure ``string`` subcommand with constant arguments."""
    if subcmd not in _PURE_STRING_SUBCMDS:
        return None

    try:
        if subcmd == "length" and len(args) == 1:
            return str(len(args[0]))
        if subcmd == "cat":
            return "".join(args)
        if subcmd == "toupper" and len(args) == 1:
            return args[0].upper()
        if subcmd == "tolower" and len(args) == 1:
            return args[0].lower()
        if subcmd == "totitle" and len(args) == 1:
            return args[0].title()
        if subcmd == "trim" and 1 <= len(args) <= 2:
            chars = args[1] if len(args) > 1 else " \t\n"
            return args[0].strip(chars)
        if subcmd == "trimleft" and 1 <= len(args) <= 2:
            chars = args[1] if len(args) > 1 else " \t\n"
            return args[0].lstrip(chars)
        if subcmd == "trimright" and 1 <= len(args) <= 2:
            chars = args[1] if len(args) > 1 else " \t\n"
            return args[0].rstrip(chars)
        if subcmd == "reverse" and len(args) == 1:
            return args[0][::-1]
        if subcmd == "repeat" and len(args) == 2:
            count = int(args[1])
            if count < 0 or count > 10000:
                return None
            return args[0] * count
        if subcmd == "index" and len(args) == 2:
            idx = int(args[1])
            s = args[0]
            if 0 <= idx < len(s):
                return s[idx]
            return ""
        if subcmd == "range" and len(args) == 3:
            s = args[0]
            first = int(args[1])
            last = int(args[2])
            if first < 0:
                first = 0
            if last >= len(s):
                last = len(s) - 1
            return s[first : last + 1]
        if subcmd == "first" and len(args) >= 2:
            needle = args[0]
            haystack = args[1]
            start = int(args[2]) if len(args) > 2 else 0
            return str(haystack.find(needle, start))
        if subcmd == "last" and len(args) >= 2:
            needle = args[0]
            haystack = args[1]
            return str(haystack.rfind(needle))
        if subcmd == "replace" and len(args) == 4:
            s = args[0]
            first = int(args[1])
            last = int(args[2])
            new_str = args[3]
            return s[:first] + new_str + s[last + 1 :]
        if subcmd == "map" and len(args) == 2:
            # string map {old new old2 new2} string
            mapping_list = _tcl_list_split(args[0])
            if len(mapping_list) % 2 != 0:
                return None
            result = args[1]
            for i in range(0, len(mapping_list), 2):
                result = result.replace(mapping_list[i], mapping_list[i + 1])
            return result
    except (ValueError, IndexError):
        return None

    return None


def _eval_format(args: list[str]) -> str | None:
    """Evaluate ``format fmt ?args...?`` with constant arguments."""
    if not args:
        return None
    fmt = args[0]
    fmt_args = args[1:]

    # Simple format string handling: %s, %d, %x, %o, %f, %%
    pieces: list[str] = []
    arg_idx = 0
    pos = 0
    while pos < len(fmt):
        if fmt[pos] == "%" and pos + 1 < len(fmt):
            spec = fmt[pos + 1]
            if spec == "%":
                pieces.append("%")
                pos += 2
                continue
            if arg_idx >= len(fmt_args):
                return None
            val = fmt_args[arg_idx]
            arg_idx += 1
            try:
                if spec == "s":
                    pieces.append(val)
                elif spec == "d":
                    pieces.append(str(int(val)))
                elif spec == "x":
                    pieces.append(hex(int(val))[2:])
                elif spec == "o":
                    pieces.append(oct(int(val))[2:])
                elif spec == "f":
                    pieces.append(f"{float(val):f}")
                else:
                    return None  # Unsupported format spec.
            except (ValueError, TypeError):
                return None
            pos += 2
        else:
            pieces.append(fmt[pos])
            pos += 1

    # Tcl raises an error when extra arguments are not consumed.
    if arg_idx != len(fmt_args):
        return None

    return "".join(pieces)


def _tcl_list_split(text: str) -> list[str]:
    """Simple Tcl list splitting (handles braces and quotes)."""
    return _split_simple_command(text.strip())


# Characters that require quoting when producing a Tcl list element.
_TCL_LIST_SPECIAL = frozenset(' \t\n"{}[]$;\\')


def _tcl_list_element(value: str) -> str:
    """Quote *value* as a canonical Tcl list element.

    Bare words are returned as-is.  Values containing Tcl special
    characters are brace-quoted when safe, otherwise backslash-escaped.
    """
    if not value:
        return "{}"
    if not any(ch in _TCL_LIST_SPECIAL for ch in value):
        return value
    # Brace-quoting is safe when braces are properly nested and no backslash.
    if "\\" not in value and _braces_balanced(value):
        return "{" + value + "}"
    # Fallback: backslash-escape special characters.
    out: list[str] = []
    for ch in value:
        if ch in _TCL_LIST_SPECIAL:
            out.append("\\" + ch)
        else:
            out.append(ch)
    return "".join(out)


# ---------------------------------------------------------------------------
# String folding core
# ---------------------------------------------------------------------------


def _fold_string_via_sccp(
    content: str,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
    interproc: InterproceduralAnalysis | None = None,
) -> str | None:
    """Try to fold a string body to a static value via SCCP.

    Handles both ``$var`` substitutions (resolved via SCCP lattice)
    and pure ``[cmd]`` substitutions (evaluated when all args are
    constant).  Returns the folded string if everything resolves,
    or ``None`` if any part is overdefined/unknown.
    """
    from core.compiler.core_analyses import LatticeKind

    pieces: list[str] = []
    has_dynamic = False
    pos = 0
    n = len(content)

    while pos < n:
        ch = content[pos]

        if ch == "$":
            # Variable reference.
            var_end, var_name = _parse_var_ref(content, pos)
            if var_name is None:
                return None
            ver = uses.get(var_name, 0)
            if ver <= 0:
                # Try interprocedural constants.
                if interproc is not None:
                    const_val = _interproc_var_constant(var_name, interproc)
                    if const_val is not None:
                        pieces.append(str(const_val))
                        has_dynamic = True
                        pos = var_end
                        continue
                return None
            lv = values.get((var_name, ver))
            if lv is None or lv.kind is not LatticeKind.CONST:
                return None
            pieces.append(str(lv.value))
            has_dynamic = True
            pos = var_end

        elif ch == "[":
            # Command substitution — try to evaluate pure commands.
            cmd_end = _find_cmd_end(content, pos)
            if cmd_end is None:
                return None
            cmd_text = content[pos : cmd_end + 1]
            result = _try_eval_pure_cmd(cmd_text, uses, values)
            if result is None:
                return None
            pieces.append(result)
            has_dynamic = True
            pos = cmd_end + 1

        elif ch == "\\":
            # Backslash substitution.
            if pos + 1 < n:
                next_ch = content[pos + 1]
                if next_ch == "n":
                    pieces.append("\n")
                elif next_ch == "t":
                    pieces.append("\t")
                elif next_ch == "r":
                    pieces.append("\r")
                else:
                    pieces.append(next_ch)
                pos += 2
            else:
                pieces.append("\\")
                pos += 1

        else:
            pieces.append(ch)
            pos += 1

    if not has_dynamic:
        return None  # No substitutions to fold — already static.

    return "".join(pieces)


def _find_cmd_end(content: str, start: int) -> int | None:
    """Find the closing ``]`` for a command starting with ``[`` at *start*."""
    if start >= len(content) or content[start] != "[":
        return None
    depth = 1
    pos = start + 1
    while pos < len(content) and depth > 0:
        ch = content[pos]
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth -= 1
        elif ch == "\\":
            pos += 1  # Skip escaped char.
        pos += 1
    if depth != 0:
        return None
    return pos - 1  # Position of the closing ].


# ---------------------------------------------------------------------------
# Interprocedural constant lookup
# ---------------------------------------------------------------------------


def _interproc_var_constant(
    var_name: str,
    interproc: InterproceduralAnalysis,
) -> int | float | bool | str | None:
    """Check if *var_name* is a proc parameter with a known constant value.

    This is a conservative lookup: we only return a constant if the
    interprocedural analysis proves the parameter is always called
    with the same constant value.
    """
    for _qname, summary in interproc.procedures.items():
        if var_name in summary.params and summary.can_fold_static_calls:
            # The parameter is constant if the proc is only called with
            # a single constant value.  The interprocedural analysis
            # tracks this via returns_constant / constant_return, but
            # individual parameter constants require re-analysis which
            # is too expensive here.  Skip for now.
            pass
    return None


# ---------------------------------------------------------------------------
# Taint gating
# ---------------------------------------------------------------------------


def _has_unsafe_tainted_inputs(
    content: str,
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
) -> bool:
    """Check whether any variable reference has unsafe taint.

    Returns ``True`` if a variable is tainted AND lacks any of the
    safe taint colours (IP_ADDRESS, PORT, FQDN, LIST_CANONICAL,
    REGEX_LITERAL) that prove a fixed-form value.

    A value with safe colours has a constrained format even though
    it originates from external input — it's safe to fold because
    its shape is predictable.
    """
    pos = 0
    n = len(content)
    while pos < n:
        if content[pos] == "$":
            var_end, var_name = _parse_var_ref(content, pos)
            if var_name is not None:
                ver = uses.get(var_name, 0)
                if ver > 0:
                    taint = taints.get((var_name, ver))
                    if taint is not None and taint.tainted:
                        # Check if any safe colour is present.
                        if not (taint.colour.value & _SAFE_TAINT_COLOURS):
                            return True
            pos = var_end if var_name is not None else pos + 1
        elif content[pos] == "[":
            # Skip command substitution — taint of cmd results is
            # checked separately via the command's return taint.
            cmd_end = _find_cmd_end(content, pos)
            pos = (cmd_end + 1) if cmd_end is not None else pos + 1
        else:
            pos += 1
    return False


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _braces_balanced(text: str) -> bool:
    """Return True if braces in *text* are properly nested (depth never goes negative)."""
    depth = 0
    for ch in text:
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


def _record_consumed_vars(content: str, folded_vars: set[str]) -> None:
    """Record variable names referenced in *content* for dead-set detection."""
    pos = 0
    n = len(content)
    while pos < n:
        if content[pos] == "$":
            var_end, var_name = _parse_var_ref(content, pos)
            if var_name is not None:
                folded_vars.add(var_name)
            pos = var_end if var_name is not None else pos + 1
        else:
            pos += 1


def _emit_fold_edits_for_region(
    source: str,
    start: int,
    end: int,
    original_value: str,
    folded: str,
    edits: list[tuple[int, int, str]],
    fold_map: dict[str, str],
) -> None:
    """Find and replace a quoted string within a source region."""
    region = source[start:end]
    # Look for a quoted string containing the original value.
    q_start = region.find('"')
    if q_start < 0:
        return

    abs_start = start + q_start
    close = _find_close_quote(source, abs_start + 1)
    if close is None:
        return

    inner = source[abs_start + 1 : close]
    if "$" not in inner and "[" not in inner:
        return

    replacement = _build_replacement(folded)
    edits.append((abs_start, close - abs_start + 1, replacement))
    fold_map[inner] = folded


def _find_close_quote(source: str, start: int) -> int | None:
    """Find the closing double-quote starting from *start*."""
    pos = start
    while pos < len(source):
        ch = source[pos]
        if ch == '"':
            return pos
        if ch == "\\":
            pos += 2
            continue
        if ch == "[":
            depth = 1
            pos += 1
            while pos < len(source) and depth > 0:
                if source[pos] == "[":
                    depth += 1
                elif source[pos] == "]":
                    depth -= 1
                elif source[pos] == "\\":
                    pos += 1
                pos += 1
            continue
        pos += 1
    return None


def _build_replacement(folded: str) -> str:
    """Build a replacement token for a folded static string.

    Prefers brace quoting when safe.  Falls back to double-quote form
    with proper escaping of Tcl metacharacters.
    """
    needs_quoting = not folded or any(
        ch in folded for ch in (" ", "\t", "\n", '"', "{", "}", "[", "]", "$", "\\", ";")
    )

    if not needs_quoting:
        return folded

    # Brace-quoting: safe when braces are properly nested and no backslash.
    if "\\" not in folded and _braces_balanced(folded):
        return "{" + folded + "}"

    # Double-quote form with escaping.
    escaped = folded.replace("\\", "\\\\").replace('"', '\\"')
    escaped = escaped.replace("$", "\\$").replace("[", "\\[")
    return '"' + escaped + '"'
