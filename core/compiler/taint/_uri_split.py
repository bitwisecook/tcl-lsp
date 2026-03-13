"""IRULE3103 -- suggest ``*::path`` / ``*::query`` over ``*::uri``.

Detects patterns where a developer uses a ``*::uri`` getter and then
decomposes the result manually, when a purpose-built ``*::path`` or
``*::query`` command would be clearer, cheaper, and eligible for the
``-normalized`` flag.

Detected patterns (all at the IR / CFG / SCCP level):

* ``split $uri "?"`` / ``split $uri "&"``
* ``[HTTP::uri] starts_with "/path"`` (path-like operand, no ``?``/``&``)
* ``[HTTP::uri] ends_with ".html"``   (path-like suffix)
* ``[HTTP::uri] contains "&key="``    (query-like operand)
* ``[HTTP::uri] matches_glob "/api/*"`` (path-like pattern)
* ``[HTTP::uri] matches_glob "*&key=*"`` (query-like pattern)
* ``string match "/api/*" $uri``
* ``string first "?" $uri``

The pass also generalises to **any** ``*::uri`` command that has
sibling ``*::path`` and/or ``*::query`` commands in the registry.
"""

from __future__ import annotations

from functools import lru_cache

from ...commands.registry import REGISTRY
from ...common.dialect import active_dialect
from ...common.naming import normalise_var_name as _normalise_var_name
from ..cfg import CFGBranch, CFGFunction
from ..core_analyses import LatticeKind, LatticeValue
from ..expr_ast import (
    BinOp,
    ExprBinary,
    ExprCommand,
    ExprNode,
    ExprVar,
)
from ..ir import IRAssignExpr, IRAssignValue, IRCall, IRStatement
from ..ssa import SSAFunction, SSAPhi, SSAStatement, SSAValueKey
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._types import TaintWarning

# Maximum depth for backward SSA tracing (prevents infinite loops on
# pathological phi chains).
_MAX_TRACE_DEPTH = 12

# Expression operators that compare a URI against a literal operand.
_COMPARISON_OPS = frozenset(
    {
        BinOp.STARTS_WITH,
        BinOp.ENDS_WITH,
        BinOp.CONTAINS,
        BinOp.MATCHES_GLOB,
        BinOp.MATCHES_REGEX,
        BinOp.STR_EQ,
        BinOp.STR_EQUALS,
        BinOp.EQ,
    }
)


# -- URI family discovery ----------------------------------------------------


@lru_cache(maxsize=1)
def _uri_families() -> dict[str, tuple[str | None, str | None]]:
    """Return ``{uri_cmd: (path_cmd_or_None, query_cmd_or_None)}``.

    Scans the command registry for ``*::uri`` commands that have a
    sibling ``*::path`` and/or ``*::query``.
    """
    families: dict[str, tuple[str | None, str | None]] = {}
    all_names = set(REGISTRY.command_names(dialect="f5-irules"))
    uri_cmds = [n for n in all_names if n.endswith("::uri")]
    for uri in uri_cmds:
        ns = uri.rsplit("::", 1)[0]
        path_cmd = f"{ns}::path" if f"{ns}::path" in all_names else None
        query_cmd = f"{ns}::query" if f"{ns}::query" in all_names else None
        if path_cmd is not None or query_cmd is not None:
            families[uri] = (path_cmd, query_cmd)
    return families


def _is_uri_getter(command: str, args: tuple[str, ...]) -> bool:
    """Return True if *command* is a ``*::uri`` getter (zero-arg form)."""
    return command in _uri_families() and len(args) == 0


def _uri_siblings(command: str) -> tuple[str | None, str | None]:
    """Return ``(path_cmd, query_cmd)`` for a ``*::uri`` command."""
    return _uri_families().get(command, (None, None))


# -- Tcl quoting helper ------------------------------------------------------


def _strip_tcl_quotes(arg: str) -> str:
    """Strip surrounding Tcl quoting (double-quotes or braces) from a word.

    ``parse_command_substitution`` uses naive whitespace splitting, so
    arguments arrive with their outer quoting intact.
    """
    stripped = arg.strip()
    if stripped.startswith('"') and stripped.endswith('"') and len(stripped) >= 2:
        return stripped[1:-1]
    if stripped.startswith("{") and stripped.endswith("}") and len(stripped) >= 2:
        return stripped[1:-1]
    return stripped


# -- literal / SCCP resolution -----------------------------------------------


def _resolve_literal(
    arg: str,
    sccp_values: dict[SSAValueKey, LatticeValue] | None,
    uses: dict[str, int],
) -> str | None:
    """Return the resolved literal string, or ``None`` if unknown."""
    cleaned = _strip_tcl_quotes(arg)
    if "$" not in cleaned and "[" not in cleaned:
        return cleaned
    if sccp_values is not None and is_pure_var_ref(cleaned):
        var_name = _normalise_var_name(cleaned)
        if var_name:
            ver = uses.get(var_name, 0)
            lv = sccp_values.get((var_name, ver))
            if lv is not None and lv.kind is LatticeKind.CONST and isinstance(lv.value, str):
                return lv.value
    return None


# -- pattern classification --------------------------------------------------


def _is_path_like(operand: str) -> bool:
    """Return True when *operand* is unambiguously a path pattern.

    Path-like: starts with ``/`` and contains no ``?`` or ``&``.
    Also matches file extensions (e.g. ``.html``, ``.js``) or path
    segments that contain no query-string characters.
    """
    if "?" in operand or "&" in operand or "=" in operand:
        return False
    if operand.startswith("/"):
        return True
    # Bare file extension or path-like token (no query chars).
    if operand.startswith(".") and "/" not in operand:
        return True
    return False


def _is_query_like(operand: str) -> bool:
    """Return True when *operand* is unambiguously a query pattern.

    Query-like: contains ``&`` or ``=`` (like ``key=val``), or starts
    with ``?``.
    """
    if "&" in operand or "=" in operand:
        return True
    if operand.startswith("?"):
        return True
    return False


def _classify_glob_pattern(pattern: str) -> str | None:
    """Classify a glob pattern as ``"path"`` or ``"query"`` or ``None``.

    * ``/api/*`` (no ``&`` / ``=``) → ``"path"``
    * ``*&key=*`` → ``"query"``

    Note: ``?`` is a single-character wildcard in glob syntax, so it
    is **not** treated as a query-string delimiter here.
    """
    # Strip leading glob wildcard to inspect the meaningful prefix.
    meaningful = pattern.lstrip("*")
    if meaningful.startswith("/") and "&" not in pattern and "=" not in pattern:
        return "path"
    # Unambiguous query indicators: & or =
    if "&" in pattern or "=" in pattern:
        return "query"
    return None


def _classify_regex_pattern(pattern: str) -> str | None:
    """Classify a regex pattern as ``"path"`` or ``"query"`` or ``None``.

    Applies heuristics with common regex anchoring stripped
    (``^``, ``$``, ``\\A``, ``\\Z``).

    Note: bare ``?`` is a regex quantifier and is **not** treated as a
    query delimiter.  Escaped ``\\?`` is the literal form and *is* a
    positive signal for query matching.
    """
    stripped = pattern.lstrip("^").removeprefix("\\A")
    # Path-like: starts with / and has no query indicators.
    if (
        stripped.startswith("/")
        and "&" not in stripped
        and "=" not in stripped
        and "\\?" not in pattern
    ):
        return "path"
    # Escaped \? or \& → literal query delimiters → query-like.
    if "\\?" in pattern or "\\&" in pattern:
        return "query"
    # Unambiguous query indicators: & or =
    if "&" in stripped or "=" in stripped:
        return "query"
    return None


# -- backward SSA tracing ---------------------------------------------------


def _build_def_site_map(
    cfg: CFGFunction,
    ssa: SSAFunction,
) -> dict[SSAValueKey, tuple[str, int]]:
    """Map ``(var_name, version)`` → ``(block_name, statement_index)``."""
    result: dict[SSAValueKey, tuple[str, int]] = {}
    for bn, ssa_block in ssa.blocks.items():
        for idx, s in enumerate(ssa_block.statements):
            for var, ver in s.defs.items():
                result[(var, ver)] = (bn, idx)
    return result


def _build_phi_index(ssa: SSAFunction) -> dict[SSAValueKey, SSAPhi]:
    """Build a ``(var_name, version)`` → ``SSAPhi`` lookup table."""
    index: dict[SSAValueKey, SSAPhi] = {}
    for _bn, ssa_block in ssa.blocks.items():
        for phi in ssa_block.phis:
            index[(phi.name, phi.version)] = phi
    return index


def _trace_to_uri_family(
    var_name: str,
    version: int,
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
    depth: int = 0,
    phi_index: dict[SSAValueKey, SSAPhi] | None = None,
) -> str | None:
    """Walk backward through SSA defs to find the originating ``*::uri`` command.

    Returns the command name (e.g. ``"HTTP::uri"``) if the value
    provably flows from a single URI getter call with no intermediate
    transformation, or ``None`` otherwise.
    """
    if depth > _MAX_TRACE_DEPTH:
        return None

    if phi_index is None:
        phi_index = _build_phi_index(ssa)

    key: SSAValueKey = (var_name, version)
    site = def_sites.get(key)
    if site is None:
        # Check phi nodes via index (O(1) lookup).
        phi = phi_index.get(key)
        if phi is not None:
            # All incoming edges must agree on the same origin.
            origin: str | None = None
            for _pred, inc_ver in phi.incoming.items():
                if inc_ver <= 0:
                    continue
                candidate = _trace_to_uri_family(
                    var_name,
                    inc_ver,
                    cfg,
                    ssa,
                    def_sites,
                    depth + 1,
                    phi_index,
                )
                if candidate is None:
                    return None
                if origin is None:
                    origin = candidate
                elif origin != candidate:
                    return None
            return origin
        return None

    bn, idx = site
    block = cfg.blocks.get(bn)
    ssa_block = ssa.blocks.get(bn)
    if block is None or ssa_block is None or idx >= len(block.statements):
        return None

    stmt = block.statements[idx]
    ssa_stmt = ssa_block.statements[idx]

    if isinstance(stmt, IRCall) and stmt.defs:
        if _is_uri_getter(stmt.command, stmt.args):
            return stmt.command
        return None

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        parsed = parse_command_substitution(value)
        if parsed is not None:
            cmd_name, cmd_args = parsed
            if _is_uri_getter(cmd_name, cmd_args):
                return cmd_name
            return None

        if is_pure_var_ref(value):
            src_name = _normalise_var_name(value)
            if src_name:
                src_ver = ssa_stmt.uses.get(src_name, 0)
                if src_ver > 0:
                    return _trace_to_uri_family(
                        src_name,
                        src_ver,
                        cfg,
                        ssa,
                        def_sites,
                        depth + 1,
                    )

    return None


def _arg_traces_to_uri_family(
    input_arg: str,
    ssa_stmt: SSAStatement,
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
) -> str | None:
    """Return the ``*::uri`` command name if *input_arg* traces back to one."""
    if is_pure_var_ref(input_arg):
        var_name = _normalise_var_name(input_arg)
        if not var_name:
            return None
        ver = ssa_stmt.uses.get(var_name, 0)
        if ver <= 0:
            return None
        return _trace_to_uri_family(var_name, ver, cfg, ssa, def_sites)

    if input_arg.startswith("[") and input_arg.endswith("]"):
        parsed = parse_command_substitution(input_arg)
        if parsed is not None and _is_uri_getter(parsed[0], parsed[1]):
            return parsed[0]

    return None


# -- message builders --------------------------------------------------------


def _split_message(uri_cmd: str, sep: str) -> str:
    """Build IRULE3103 message for a ``split`` pattern."""
    path_cmd, query_cmd = _uri_siblings(uri_cmd)
    has_q = "?" in sep
    has_amp = "&" in sep

    if has_q and not has_amp:
        parts = []
        if path_cmd:
            parts.append(path_cmd)
        if query_cmd:
            parts.append(query_cmd)
        return (
            f"Splitting {uri_cmd} on '?' to extract path or query; "
            f"use {' and '.join(parts)} instead for clearer, "
            f"more efficient URI decomposition."
        )
    if has_amp and not has_q:
        if query_cmd:
            return (
                f"Splitting {uri_cmd} on '&' to parse query parameters; "
                f"use {query_cmd} to obtain the query string directly, "
                f"then split on '&' if needed."
            )
        return (
            f"Splitting {uri_cmd} on '&' to parse query parameters; "
            f"consider using the query-string accessor directly."
        )
    parts = []
    if path_cmd:
        parts.append(path_cmd)
    if query_cmd:
        parts.append(query_cmd)
    return (
        f"Splitting {uri_cmd} on '?' and '&' to decompose the request; "
        f"use {' and '.join(parts)} for clearer, more efficient "
        f"URI handling."
    )


def _comparison_message(
    uri_cmd: str,
    op_name: str,
    component: str,
) -> str:
    """Build IRULE3103 message for an expression-operator or string-match pattern."""
    path_cmd, query_cmd = _uri_siblings(uri_cmd)
    if component == "path" and path_cmd:
        return (
            f"{uri_cmd} used with {op_name} on a path-like pattern; "
            f"use {path_cmd} instead for clearer intent and to avoid "
            f"query-string interference."
        )
    if component == "query" and query_cmd:
        return (
            f"{uri_cmd} used with {op_name} on a query-like pattern; "
            f"use {query_cmd} instead for direct access to the query string."
        )
    # Fallback: at least one sibling exists.
    parts = []
    if path_cmd:
        parts.append(path_cmd)
    if query_cmd:
        parts.append(query_cmd)
    return (
        f"{uri_cmd} used with {op_name}; "
        f"consider using {' or '.join(parts)} for more precise matching."
    )


def _string_first_message(uri_cmd: str, needle: str) -> str:
    """Build IRULE3103 message for a ``string first`` pattern."""
    path_cmd, query_cmd = _uri_siblings(uri_cmd)
    parts = []
    if path_cmd:
        parts.append(path_cmd)
    if query_cmd:
        parts.append(query_cmd)
    return (
        f"Searching {uri_cmd} for '{needle}' to decompose the URI; "
        f"use {' and '.join(parts)} instead for direct access to "
        f"path and query components."
    )


def _expr_hit_message(uri_cmd: str, op_name: str, component: str) -> str:
    """Build the right IRULE3103 message for a hit from ``_walk_expr``.

    For ``string first`` and ``split``, *component* carries the
    needle / separator string so the specialised message builder can
    be used.  For everything else it is ``"path"`` or ``"query"``.
    """
    if op_name == "string first":
        return _string_first_message(uri_cmd, component)
    if op_name == "split":
        return _split_message(uri_cmd, component)
    return _comparison_message(uri_cmd, op_name, component)


# -- expression-level detection ----------------------------------------------


def _classify_operand_for_op(op: BinOp, operand: str) -> str | None:
    """Return ``"path"`` or ``"query"`` for a literal operand, or ``None``.

    The classification depends on the operator:
    - ``starts_with`` with ``/...`` (no ``?``/``&``) → path
    - ``ends_with`` with no ``?``/``&``/``=`` → path; with ``&``/``=`` → query
    - ``contains`` with ``?``/``&``/``=`` → query; with ``/...`` → path
    - ``matches_glob`` → delegate to glob classifier
    - ``matches_regex`` → delegate to regex classifier
    - ``eq``/``equals``/``==`` → path if no query chars, query if has query chars
    """
    if op is BinOp.STARTS_WITH:
        if _is_path_like(operand):
            return "path"
        if _is_query_like(operand):
            return "query"
        return None

    if op is BinOp.ENDS_WITH:
        if _is_query_like(operand):
            return "query"
        if _is_path_like(operand):
            return "path"
        # Simple suffixes without query chars are path-like.
        if "?" not in operand and "&" not in operand and "=" not in operand:
            return "path"
        return None

    if op is BinOp.CONTAINS:
        if _is_query_like(operand):
            return "query"
        if _is_path_like(operand):
            return "path"
        return None

    if op is BinOp.MATCHES_GLOB:
        return _classify_glob_pattern(operand)

    if op is BinOp.MATCHES_REGEX:
        return _classify_regex_pattern(operand)

    if op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.EQ):
        if _is_path_like(operand):
            return "path"
        if _is_query_like(operand):
            return "query"
        return None

    return None


def _expr_literal_text(node: ExprNode) -> str | None:
    """Return the unquoted literal text from an expression node, or ``None``."""
    from ..expr_ast import ExprLiteral, ExprString

    if isinstance(node, ExprString):
        return _strip_tcl_quotes(node.text)
    if isinstance(node, ExprLiteral):
        return node.text
    return None


def _expr_traces_to_uri(
    node: ExprNode,
    ssa_exit_versions: dict[str, int],
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
) -> str | None:
    """Return the ``*::uri`` command name if *node* traces back to one."""
    if isinstance(node, ExprCommand):
        parsed = parse_command_substitution(node.text)
        if parsed is not None and _is_uri_getter(parsed[0], parsed[1]):
            return parsed[0]
        return None
    if isinstance(node, ExprVar):
        var_name = node.name
        ver = ssa_exit_versions.get(var_name, 0)
        if ver <= 0:
            return None
        return _trace_to_uri_family(var_name, ver, cfg, ssa, def_sites)
    return None


def _check_expr_binary(
    node: ExprBinary,
    ssa_exit_versions: dict[str, int],
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
) -> tuple[str, str, str] | None:
    """Check an ``ExprBinary`` for URI-vs-path/query pattern.

    Returns ``(uri_cmd, op_name, component)`` or ``None``.
    """
    if node.op not in _COMPARISON_OPS:
        return None

    # Pattern: <uri_expr> op <literal>
    uri_cmd = _expr_traces_to_uri(node.left, ssa_exit_versions, cfg, ssa, def_sites)
    if uri_cmd is not None:
        lit = _expr_literal_text(node.right)
        if lit is not None:
            component = _classify_operand_for_op(node.op, lit)
            if component is not None:
                return (uri_cmd, node.op.value, component)

    # Reversed operand order (uncommon but possible with eq/equals).
    if node.op in (BinOp.STR_EQ, BinOp.STR_EQUALS, BinOp.EQ):
        uri_cmd = _expr_traces_to_uri(node.right, ssa_exit_versions, cfg, ssa, def_sites)
        if uri_cmd is not None:
            lit = _expr_literal_text(node.left)
            if lit is not None:
                component = _classify_operand_for_op(node.op, lit)
                if component is not None:
                    return (uri_cmd, node.op.value, component)

    return None


def _check_expr_command(
    node: ExprCommand,
    ssa_exit_versions: dict[str, int],
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
) -> tuple[str, str, str] | None:
    """Check an ``ExprCommand`` for ``[string match ...]`` / ``[string first ...]``
    / ``[split ...]`` patterns against a ``*::uri`` value.

    Returns ``(uri_cmd, op_name, component)`` or ``None``.
    """
    parsed = parse_command_substitution(node.text)
    if parsed is None:
        return None
    cmd_name, cmd_args = parsed

    if cmd_name == "string" and cmd_args:
        sub = cmd_args[0]
        if sub == "match":
            remaining = cmd_args[1:]
            if remaining and remaining[0] == "-nocase":
                remaining = remaining[1:]
            if len(remaining) < 2:
                return None
            pattern = _strip_tcl_quotes(remaining[0])
            input_arg = _strip_tcl_quotes(remaining[1])
            if is_pure_var_ref(input_arg):
                var_name = _normalise_var_name(input_arg)
                if var_name:
                    ver = ssa_exit_versions.get(var_name, 0)
                    if ver > 0:
                        uri_cmd = _trace_to_uri_family(
                            var_name,
                            ver,
                            cfg,
                            ssa,
                            def_sites,
                        )
                        if uri_cmd is not None:
                            component = _classify_glob_pattern(pattern)
                            if component is not None:
                                return (uri_cmd, "string match", component)
        elif sub == "first":
            if len(cmd_args) < 3:
                return None
            needle = _strip_tcl_quotes(cmd_args[1])
            input_arg = _strip_tcl_quotes(cmd_args[2])
            if is_pure_var_ref(input_arg):
                var_name = _normalise_var_name(input_arg)
                if var_name:
                    ver = ssa_exit_versions.get(var_name, 0)
                    if ver > 0:
                        uri_cmd = _trace_to_uri_family(
                            var_name,
                            ver,
                            cfg,
                            ssa,
                            def_sites,
                        )
                        if uri_cmd is not None and bool(_QUERY_CHARS & set(needle)):
                            path_cmd, _query_cmd = _uri_siblings(uri_cmd)
                            component = needle  # pass needle for message builder
                            return (uri_cmd, "string first", component)

    if cmd_name == "split" and len(cmd_args) >= 2:
        sep = _strip_tcl_quotes(cmd_args[1])
        if bool(_QUERY_CHARS & set(sep)):
            input_arg = _strip_tcl_quotes(cmd_args[0])
            if is_pure_var_ref(input_arg):
                var_name = _normalise_var_name(input_arg)
                if var_name:
                    ver = ssa_exit_versions.get(var_name, 0)
                    if ver > 0:
                        uri_cmd = _trace_to_uri_family(
                            var_name,
                            ver,
                            cfg,
                            ssa,
                            def_sites,
                        )
                        if uri_cmd is not None:
                            component = sep  # pass separator for message builder
                            return (uri_cmd, "split", component)

    return None


def _walk_expr(
    node: ExprNode,
    ssa_exit_versions: dict[str, int],
    cfg: CFGFunction,
    ssa: SSAFunction,
    def_sites: dict[SSAValueKey, tuple[str, int]],
) -> list[tuple[str, str, str]]:
    """Walk an expression AST and collect all URI-vs-path/query matches.

    Returns a list of ``(uri_cmd, op_name, component)``.
    """
    from ..expr_ast import ExprCall, ExprTernary, ExprUnary

    results: list[tuple[str, str, str]] = []

    if isinstance(node, ExprBinary):
        hit = _check_expr_binary(node, ssa_exit_versions, cfg, ssa, def_sites)
        if hit is not None:
            results.append(hit)
        results.extend(_walk_expr(node.left, ssa_exit_versions, cfg, ssa, def_sites))
        results.extend(_walk_expr(node.right, ssa_exit_versions, cfg, ssa, def_sites))
    elif isinstance(node, ExprUnary):
        results.extend(_walk_expr(node.operand, ssa_exit_versions, cfg, ssa, def_sites))
    elif isinstance(node, ExprTernary):
        results.extend(_walk_expr(node.condition, ssa_exit_versions, cfg, ssa, def_sites))
        results.extend(_walk_expr(node.true_branch, ssa_exit_versions, cfg, ssa, def_sites))
        results.extend(_walk_expr(node.false_branch, ssa_exit_versions, cfg, ssa, def_sites))
    elif isinstance(node, ExprCall):
        for arg in node.args:
            results.extend(_walk_expr(arg, ssa_exit_versions, cfg, ssa, def_sites))
    elif isinstance(node, ExprCommand):
        # Check for [string match ...] / [string first ...] / [split ...]
        # inside expression conditions (e.g. ``if { [string match ...] }``).
        hit = _check_expr_command(node, ssa_exit_versions, cfg, ssa, def_sites)
        if hit is not None:
            results.append(hit)

    return results


# -- split detection ---------------------------------------------------------


def _extract_split_info(
    stmt: IRStatement,
    ssa_stmt: SSAStatement,
    sccp_values: dict[SSAValueKey, LatticeValue] | None,
) -> tuple[str | None, str | None] | None:
    """Extract ``(input_arg, separator)`` from a split call.

    Handles both ``IRCall`` (standalone ``split``) and ``IRAssignValue``
    (``set var [split ...]``).  Returns ``None`` if not a split call.
    """
    if isinstance(stmt, IRCall) and stmt.command == "split":
        if len(stmt.args) < 2:
            return None
        sep = _resolve_literal(stmt.args[1], sccp_values, ssa_stmt.uses)
        return (stmt.args[0].strip(), sep)

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        parsed = parse_command_substitution(value)
        if parsed is not None and parsed[0] == "split":
            cmd_args = parsed[1]
            if len(cmd_args) < 2:
                return None
            sep = _resolve_literal(cmd_args[1], sccp_values, ssa_stmt.uses)
            return (_strip_tcl_quotes(cmd_args[0]), sep)

    return None


# -- string match / string first detection -----------------------------------


def _extract_string_match_info(
    stmt: IRStatement,
    ssa_stmt: SSAStatement,
    sccp_values: dict[SSAValueKey, LatticeValue] | None,
) -> tuple[str, str | None, str | None] | None:
    """Detect ``string match <pattern> $uri`` or ``string first <needle> $uri``.

    Returns ``(sub_command, pattern_or_needle, input_arg)`` or ``None``.
    """

    def _from_args(
        command: str,
        args: tuple[str, ...],
        uses: dict[str, int],
    ) -> tuple[str, str | None, str | None] | None:
        if command != "string":
            return None
        if not args:
            return None
        sub = args[0]
        if sub == "match":
            # string match ?-nocase? pattern string
            remaining = args[1:]
            if remaining and remaining[0] == "-nocase":
                remaining = remaining[1:]
            if len(remaining) < 2:
                return None
            pattern = _resolve_literal(remaining[0], sccp_values, uses)
            return ("string match", pattern, _strip_tcl_quotes(remaining[1]))
        if sub == "first":
            # string first needleString haystackString ?startIndex?
            if len(args) < 3:
                return None
            needle = _resolve_literal(args[1], sccp_values, uses)
            return ("string first", needle, _strip_tcl_quotes(args[2]))
        return None

    if isinstance(stmt, IRCall):
        return _from_args(stmt.command, stmt.args, ssa_stmt.uses)

    if isinstance(stmt, IRAssignValue):
        value = stmt.value.strip()
        parsed = parse_command_substitution(value)
        if parsed is not None:
            return _from_args(parsed[0], parsed[1], ssa_stmt.uses)

    return None


# -- main entry point -------------------------------------------------------


_QUERY_CHARS = frozenset("?&")


def _find_uri_split_suggestions(
    cfg: CFGFunction,
    ssa: SSAFunction,
    sccp_values: dict[SSAValueKey, LatticeValue] | None,
    executable_blocks: set[str],
) -> list[TaintWarning]:
    """Find patterns where ``*::uri`` should be ``*::path`` or ``*::query``.

    Returns IRULE3103 warnings.
    """
    if active_dialect() != "f5-irules":
        return []

    if not _uri_families():
        return []

    warnings: list[TaintWarning] = []
    def_sites = _build_def_site_map(cfg, ssa)

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue

        # -- Statement-level checks (split, string match, string first) ------
        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]

            # 1. split detection
            split_info = _extract_split_info(stmt, ssa_stmt, sccp_values)
            if split_info is not None:
                input_arg, sep = split_info
                if input_arg is not None and sep is not None and bool(_QUERY_CHARS & set(sep)):
                    uri_cmd = _arg_traces_to_uri_family(
                        input_arg,
                        ssa_stmt,
                        cfg,
                        ssa,
                        def_sites,
                    )
                    if uri_cmd is not None:
                        warnings.append(
                            TaintWarning(
                                range=stmt.range,
                                variable="",
                                sink_command="split",
                                code="IRULE3103",
                                message=_split_message(uri_cmd, sep),
                            )
                        )
                        continue

            # 2. string match / string first
            sm_info = _extract_string_match_info(stmt, ssa_stmt, sccp_values)
            if sm_info is not None:
                sub_cmd, pattern, input_arg = sm_info
                if pattern is not None and input_arg is not None:
                    uri_cmd = _arg_traces_to_uri_family(
                        input_arg,
                        ssa_stmt,
                        cfg,
                        ssa,
                        def_sites,
                    )
                    if uri_cmd is not None:
                        if sub_cmd == "string match":
                            component = _classify_glob_pattern(pattern)
                            if component is not None:
                                warnings.append(
                                    TaintWarning(
                                        range=stmt.range,
                                        variable="",
                                        sink_command="string match",
                                        code="IRULE3103",
                                        message=_comparison_message(
                                            uri_cmd,
                                            "string match",
                                            component,
                                        ),
                                    )
                                )
                        elif sub_cmd == "string first":
                            if bool(_QUERY_CHARS & set(pattern)):
                                warnings.append(
                                    TaintWarning(
                                        range=stmt.range,
                                        variable="",
                                        sink_command="string first",
                                        code="IRULE3103",
                                        message=_string_first_message(uri_cmd, pattern),
                                    )
                                )

            # 3. Expression-level checks in IRAssignExpr (set x [expr {...}])
            if isinstance(stmt, IRAssignExpr):
                for uri_cmd, op_name, component in _walk_expr(
                    stmt.expr,
                    ssa_stmt.uses,
                    cfg,
                    ssa,
                    def_sites,
                ):
                    warnings.append(
                        TaintWarning(
                            range=stmt.range,
                            variable="",
                            sink_command=op_name,
                            code="IRULE3103",
                            message=_expr_hit_message(uri_cmd, op_name, component),
                        )
                    )

        # -- Branch condition checks (if { ... starts_with ... }) -----------
        term = block.terminator
        if isinstance(term, CFGBranch):
            exit_vers = ssa_block.exit_versions
            for uri_cmd, op_name, component in _walk_expr(
                term.condition,
                exit_vers,
                cfg,
                ssa,
                def_sites,
            ):
                if term.range is not None:
                    warnings.append(
                        TaintWarning(
                            range=term.range,
                            variable="",
                            sink_command=op_name,
                            code="IRULE3103",
                            message=_expr_hit_message(uri_cmd, op_name, component),
                        )
                    )

    # Deduplicate warnings that share the same range, sink, and message
    # (can arise in complex boolean expressions with repeated checks).
    seen: set[tuple[object, str, str]] = set()
    deduped: list[TaintWarning] = []
    for w in warnings:
        key = (w.range, w.sink_command, w.message)
        if key not in seen:
            seen.add(key)
            deduped.append(w)
    return deduped
