"""Static resolution of ``source FILE`` at compile time.

The Zig runtime's ``source`` builtin reads from the WASI-preopened
filesystem at run time — robust but it costs every embedder a
preopen mount, surfaces ``source: file not found`` traps when a
test fixture isn't staged, and forces the runtime to re-parse
the script on every call.  For ``source`` calls whose target
filename is *known at compile time* we can do better: read the
file off the host filesystem once, lower it to IR, and splice its
top-level statements directly in place of the ``source`` call.
The compiled WASM ends up self-contained — the sourced file's
contents live in the same module as the caller's, with no
runtime fs dependency.

The inliner runs after :func:`core.compiler.lowering.lower_to_ir`
and before :func:`core.compiler.cfg.build_cfg`.  It walks every
``IRCall(command="source", …)`` node in the top-level script and
proc bodies; for the patterns listed below it resolves the
target, reads the file, lowers it to IR, and merges the
result.  Recursive ``source`` chains follow up to
``max_depth`` levels deep, with a cycle guard on the resolved
absolute paths.

Resolvable patterns (everything else falls through to the
runtime ``source`` builtin unchanged):

* ``source LITERAL`` — plain literal filename.  Resolved
  relative to the calling file's directory, then against each
  ``search_paths`` entry.
* ``source [file join [file dirname [info script]] LITERAL]`` —
  the standard tcltest pattern for sourcing a sibling helper
  next to the test file.  ``[info script]`` is treated as the
  caller's filename so the join resolves to a sibling.
"""

# canonicalisation: audited #246


from __future__ import annotations

import re
from pathlib import Path

from .ir import (
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)

# ``[file join [file dirname [info script]] NAME]`` — match the
# inner expression text (we receive the whole bracket expression
# from the IR as a single arg string).  Allow flexible whitespace
# and the optional surrounding ``::`` prefixes.  NAME must not
# contain ``$`` / ``[`` / spaces — we only accept literals.
_FILE_JOIN_INFO_SCRIPT_RE = re.compile(
    r"""\s*
    (?:::)? file \s+ join \s+
    \[ \s* (?:::)? file \s+ dirname \s+ \[ \s* (?:::)? info \s+ script \s* \] \s* \] \s+
    ([^\s\[\]\$]+)
    \s*""",
    re.VERBOSE,
)


def _resolve_source_arg(arg: str) -> str | None:
    """Try to extract a literal filename from a ``source`` arg string.

    Returns the resolved filename (relative or absolute), or
    ``None`` when the arg is genuinely dynamic.
    """
    if not arg:
        return None
    # Bare literal — no leading ``$`` or ``[``.
    if not arg.startswith("$") and not arg.startswith("["):
        return arg
    # ``[file join [file dirname [info script]] LITERAL]``
    if arg.startswith("[") and arg.endswith("]"):
        inner = arg[1:-1]
        m = _FILE_JOIN_INFO_SCRIPT_RE.fullmatch(inner)
        if m:
            return m.group(1)
    return None


def _resolve_path(
    target: str,
    base_dir: Path | None,
    search_paths: tuple[Path, ...],
) -> Path | None:
    p = Path(target)
    if p.is_absolute():
        return p if p.is_file() else None
    if base_dir is not None:
        candidate = base_dir / target
        if candidate.is_file():
            return candidate.resolve()
    for sp in search_paths:
        candidate = sp / target
        if candidate.is_file():
            return candidate.resolve()
    return None


def _is_static_source_call(stmt: IRStatement) -> tuple[bool, str | None]:
    """Return ``(is_match, target_arg)`` for a static ``source`` IRCall.

    ``is_match`` is True only when the IRCall is ``source`` (with
    no options, or only ``-encoding NAME``) and the target is a
    statically-resolvable literal.
    """
    if not isinstance(stmt, IRCall):
        return False, None
    if stmt.canonical_command != "::source":
        return False, None
    args = list(stmt.args)
    while args and args[0].startswith("-"):
        flag = args.pop(0)
        if flag == "-encoding" and args:
            args.pop(0)
        elif flag == "--":
            break
        # Unknown option — let the runtime handle it.
        else:
            return False, None
    if len(args) != 1:
        return False, None
    target = _resolve_source_arg(args[0])
    return target is not None, target


def _inline_in_script(
    script: IRScript,
    base_dir: Path | None,
    search_paths: tuple[Path, ...],
    resolved: set[Path],
    procedures: dict[str, IRProcedure],
    redefined: set[str],
    namespace_imports: list[tuple[str, str]],
    namespace_exports: list[tuple[str, str]],
    depth: int,
    max_depth: int,
) -> IRScript:
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        is_static, target = _is_static_source_call(stmt)
        if is_static and target is not None and depth < max_depth:
            spliced = _try_inline(
                target,
                base_dir,
                search_paths,
                resolved,
                procedures,
                redefined,
                namespace_imports,
                namespace_exports,
                depth,
                max_depth,
            )
            if spliced is not None:
                changed = True
                new_stmts.extend(spliced.statements)
                continue
        # Recurse into nested control-flow.
        new_stmt = _walk_stmt(
            stmt,
            base_dir,
            search_paths,
            resolved,
            procedures,
            redefined,
            namespace_imports,
            namespace_exports,
            depth,
            max_depth,
        )
        if new_stmt is not stmt:
            changed = True
        new_stmts.append(new_stmt)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _walk_stmt(
    stmt: IRStatement,
    base_dir: Path | None,
    search_paths: tuple[Path, ...],
    resolved: set[Path],
    procedures: dict[str, IRProcedure],
    redefined: set[str],
    namespace_imports: list[tuple[str, str]],
    namespace_exports: list[tuple[str, str]],
    depth: int,
    max_depth: int,
) -> IRStatement:
    args = (
        base_dir,
        search_paths,
        resolved,
        procedures,
        redefined,
        namespace_imports,
        namespace_exports,
        depth,
        max_depth,
    )
    if isinstance(stmt, IRIf):
        new_clauses = []
        clauses_changed = False
        from dataclasses import replace

        for clause in stmt.clauses:
            new_body = _inline_in_script(clause.body, *args)
            if new_body is not clause.body:
                clauses_changed = True
                new_clauses.append(replace(clause, body=new_body))
            else:
                new_clauses.append(clause)
        new_else = stmt.else_body
        if stmt.else_body is not None:
            ne = _inline_in_script(stmt.else_body, *args)
            if ne is not stmt.else_body:
                new_else = ne
                clauses_changed = True
        if not clauses_changed:
            return stmt
        from dataclasses import replace

        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else)
    if isinstance(stmt, IRFor):
        from dataclasses import replace

        new_init = _inline_in_script(stmt.init, *args)
        new_body = _inline_in_script(stmt.body, *args)
        new_next = _inline_in_script(stmt.next, *args)
        if new_init is stmt.init and new_body is stmt.body and new_next is stmt.next:
            return stmt
        return replace(stmt, init=new_init, body=new_body, next=new_next)
    if isinstance(stmt, IRWhile):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRForeach):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRCatch):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRTry):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        new_finally = stmt.finally_body
        if stmt.finally_body is not None:
            new_finally = _inline_in_script(stmt.finally_body, *args)
        if new_body is stmt.body and new_finally is stmt.finally_body:
            return stmt
        return replace(stmt, body=new_body, finally_body=new_finally)
    if isinstance(stmt, IRSwitch):
        from dataclasses import replace

        new_arms = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            nb = _inline_in_script(arm.body, *args)
            if nb is not arm.body:
                arms_changed = True
                new_arms.append(replace(arm, body=nb))
            else:
                new_arms.append(arm)
        new_default = stmt.default_body
        if stmt.default_body is not None:
            nd = _inline_in_script(stmt.default_body, *args)
            if nd is not stmt.default_body:
                new_default = nd
                arms_changed = True
        if not arms_changed:
            return stmt
        return replace(stmt, arms=tuple(new_arms), default_body=new_default)
    if isinstance(stmt, IRBlock):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRUpFrame):
        from dataclasses import replace

        new_body = _inline_in_script(stmt.body, *args)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    return stmt


def _try_inline(
    target: str,
    base_dir: Path | None,
    search_paths: tuple[Path, ...],
    resolved: set[Path],
    procedures: dict[str, IRProcedure],
    redefined: set[str],
    namespace_imports: list[tuple[str, str]],
    namespace_exports: list[tuple[str, str]],
    depth: int,
    max_depth: int,
) -> IRScript | None:
    """Resolve *target*, lower the file, and return its top-level IR.

    Returns ``None`` when the file can't be found or the IR can't
    be produced — caller leaves the original ``source`` IRCall in
    place so the runtime gets a shot at it.
    """
    path = _resolve_path(target, base_dir, search_paths)
    if path is None:
        return None
    if path in resolved:
        # Cycle / re-source — Tcl semantics evaluate the file
        # again on every ``source``, but for static inlining the
        # second hit is almost always intended as a guard ("only
        # source once").  Skip silently to avoid runaway IR
        # growth on accidental cycles.
        return IRScript()
    resolved.add(path)
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    # Late import — ``lowering`` imports nothing from this module
    # but importing at top level would create a cycle through
    # ``inline_uplevel``'s post-lower pass which references ``ir``.
    from .lowering import lower_to_ir

    sub_module = lower_to_ir(text)
    # Recursively inline sources in the just-loaded module BEFORE
    # we splice it into the parent.  The sub-module's ``source``
    # calls resolve relative to its own directory.
    sub_module = inline_static_sources(
        sub_module,
        source_dir=path.parent,
        search_paths=search_paths,
        max_depth=max_depth,
        _resolved=resolved,
        _depth=depth + 1,
    )
    # Merge procedures into the parent's table.
    for qname, proc in sub_module.procedures.items():
        if qname in procedures:
            redefined.add(qname)
        procedures[qname] = proc
    # Merge namespace imports / exports.
    namespace_imports.extend(sub_module.namespace_imports)
    namespace_exports.extend(sub_module.namespace_exports)
    return sub_module.top_level


def inline_static_sources(
    module: IRModule,
    *,
    source_dir: Path | None = None,
    search_paths: tuple[Path, ...] = (),
    max_depth: int = 5,
    _resolved: set[Path] | None = None,
    _depth: int = 0,
) -> IRModule:
    """Splice statically-resolvable ``source`` calls into the IR.

    Mutates *module* in place and returns it.  Procedures defined
    in inlined files are merged into ``module.procedures``;
    namespace imports/exports are appended to the corresponding
    tuples on the parent module.

    Args:
        module: The IR module to inline into.
        source_dir: The directory of the file *module* was lowered
            from.  Used to resolve relative ``source`` targets.
            ``None`` disables relative resolution; absolute paths
            and ``search_paths`` lookups still work.
        search_paths: Additional directories to consult when a
            relative target isn't found alongside *source_dir*.
        max_depth: Depth cap on recursive ``source`` chains.
    """
    if _resolved is None:
        _resolved = set()
    procedures = dict(module.procedures)
    redefined = set(module.redefined_procedures)
    namespace_imports = list(module.namespace_imports)
    namespace_exports = list(module.namespace_exports)
    new_top = _inline_in_script(
        module.top_level,
        source_dir,
        search_paths,
        _resolved,
        procedures,
        redefined,
        namespace_imports,
        namespace_exports,
        _depth,
        max_depth,
    )
    # Also inline inside existing procedure bodies — a proc body
    # might contain a ``source`` call (e.g. helper-loading at proc
    # invocation time).  Each procedure body is its own IRScript.
    new_procs: dict[str, IRProcedure] = {}
    procs_changed = False
    for qname, proc in procedures.items():
        new_body = _inline_in_script(
            proc.body,
            source_dir,
            search_paths,
            _resolved,
            procedures,
            redefined,
            namespace_imports,
            namespace_exports,
            _depth,
            max_depth,
        )
        if new_body is not proc.body:
            procs_changed = True
            from dataclasses import replace

            new_procs[qname] = replace(proc, body=new_body)
        else:
            new_procs[qname] = proc

    if (
        new_top is module.top_level
        and not procs_changed
        and namespace_imports == list(module.namespace_imports)
        and namespace_exports == list(module.namespace_exports)
        and redefined == module.redefined_procedures
        and procedures.keys() == module.procedures.keys()
    ):
        return module

    from dataclasses import replace as _replace

    return _replace(
        module,
        top_level=new_top,
        procedures=new_procs,
        redefined_procedures=redefined,
        namespace_imports=tuple(namespace_imports),
        namespace_exports=tuple(namespace_exports),
    )
