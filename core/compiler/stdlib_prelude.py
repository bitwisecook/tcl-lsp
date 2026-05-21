"""Compile-time bundling of referenced standard-library procedures.

The runtime can auto-load stdlib procs from ``TCL_LIBRARY`` at run time
(see ``runtime/zig`` ``ensure_stdlib_index_loaded``), but that costs a
filesystem dependency and runs the loaded proc through the interpreter.
For commands a program references whose definitions live in the standard
library, we can do better: read the ``.tcl`` file off the host once,
lower it to IR, and splice its procedures into the module so they
compile to WASM functions and the bundle is self-contained.

This mirrors :mod:`core.compiler.source_inliner` (which inlines explicit
``source LITERAL`` calls) but is *reference-driven*: the trigger is a
call to a command that the library's ``tclIndex`` maps to a file.  Only
files for commands actually referenced (transitively) are pulled in, so
the bundle grows by the few KB a program uses rather than the whole
library.

Anything not statically resolvable — dynamic command names (``$cmd`` /
``eval``), or a library file that fails to lower — is left untouched and
falls back to the runtime ``TCL_LIBRARY`` auto-loader.  The pass runs
after :func:`core.compiler.lowering.lower_to_ir` and before
:func:`core.compiler.cfg.build_cfg`.
"""

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
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)

# ``set auto_index(NAME) [list <loader> [file join $dir FILE]]`` — the
# canonical tclIndex (version 2.0) entry shape.  ``NAME`` is the command
# (possibly namespace-qualified), ``FILE`` the relative file under the
# library directory.
_AUTO_INDEX_RE = re.compile(
    r"set\s+auto_index\(([^)]+)\)\s+\[list\s+\S+\s+\[file\s+join\s+\$dir\s+([^\]]+)\]\]"
)


def _parse_tcl_index(library_dir: Path) -> dict[str, str]:
    """Parse ``$library_dir/tclIndex`` into a ``command -> relative file`` map.

    Returns an empty map when the index is absent or unreadable (the
    caller then leaves the module untouched).
    """
    index_path = library_dir / "tclIndex"
    if not index_path.is_file():
        return {}
    mapping: dict[str, str] = {}
    try:
        text = index_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return {}
    for line in text.splitlines():
        m = _AUTO_INDEX_RE.search(line)
        if m:
            mapping[m.group(1)] = m.group(2).strip()
    return mapping


def _collect_commands(script: IRScript, out: set[str]) -> None:
    """Collect every command name invoked anywhere in *script*.

    Records both the surface ``command`` and the resolved
    ``canonical_command`` so a call written as ``parray`` and one written
    as ``::parray`` both match the index map.
    """
    for stmt in script.statements:
        if isinstance(stmt, IRCall):
            if stmt.command:
                out.add(stmt.command)
            if stmt.canonical_command:
                out.add(stmt.canonical_command)
        _recurse_bodies(stmt, out)


def _recurse_bodies(stmt: IRStatement, out: set[str]) -> None:
    if isinstance(stmt, IRIf):
        for clause in stmt.clauses:
            _collect_commands(clause.body, out)
        if stmt.else_body:
            _collect_commands(stmt.else_body, out)
    elif isinstance(stmt, IRFor):
        _collect_commands(stmt.init, out)
        _collect_commands(stmt.body, out)
        _collect_commands(stmt.next, out)
    elif isinstance(stmt, IRWhile):
        _collect_commands(stmt.body, out)
    elif isinstance(stmt, IRForeach):
        _collect_commands(stmt.body, out)
    elif isinstance(stmt, IRCatch):
        _collect_commands(stmt.body, out)
    elif isinstance(stmt, IRTry):
        _collect_commands(stmt.body, out)
        for handler in stmt.handlers:
            _collect_commands(handler.body, out)
        if stmt.finally_body:
            _collect_commands(stmt.finally_body, out)
    elif isinstance(stmt, IRSwitch):
        for arm in stmt.arms:
            if arm.body:
                _collect_commands(arm.body, out)
        if stmt.default_body:
            _collect_commands(stmt.default_body, out)
    elif isinstance(stmt, IRBlock):
        _collect_commands(stmt.body, out)
    elif isinstance(stmt, IRUpFrame):
        _collect_commands(stmt.body, out)


def _file_for_command(name: str, mapping: dict[str, str]) -> str | None:
    """Resolve a referenced command to its library file, tolerating the
    leading ``::`` that canonicalisation adds to global names."""
    if name in mapping:
        return mapping[name]
    if name.startswith("::") and name[2:] in mapping:
        return mapping[name[2:]]
    return None


def apply_stdlib_prelude(
    module: IRModule,
    library_dir: str | Path,
    *,
    max_files: int = 64,
) -> IRModule:
    """Bundle the library files for stdlib commands *module* references.

    Mutates and returns *module*: procedures (and load-time top-level
    setup) from each referenced library file are merged in.  Resolution
    is transitive — a bundled proc that calls another library command
    pulls that file in too — to a fixpoint, capped at *max_files*.

    A no-op (returns *module* unchanged) when *library_dir* has no
    ``tclIndex`` or references nothing autoloadable, so callers can pass
    a missing path harmlessly.
    """
    lib = Path(library_dir)
    mapping = _parse_tcl_index(lib)
    if not mapping:
        return module

    bundled_files: set[str] = set()
    seen_commands: set[str] = set()

    # Seed the worklist with commands referenced by the user's module.
    pending: set[str] = set()
    _collect_commands(module.top_level, pending)
    for proc in list(module.procedures.values()):
        _collect_commands(proc.body, pending)

    from .lowering import lower_to_ir

    while pending and len(bundled_files) < max_files:
        name = pending.pop()
        if name in seen_commands:
            continue
        seen_commands.add(name)

        rel_file = _file_for_command(name, mapping)
        if rel_file is None or rel_file in bundled_files:
            continue
        # A command the user already defines as a proc shadows the
        # library version — don't bundle it.
        if name in module.procedures or f"::{name}" in module.procedures:
            continue

        path = lib / rel_file
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
            sub = lower_to_ir(text)
        except Exception:  # noqa: BLE001 — a non-readable / non-lowerable
            # library file just falls back to the runtime auto-loader.
            continue

        bundled_files.add(rel_file)

        # Merge procedures (skip any the user already defined).
        for qname, proc in sub.procedures.items():
            if qname not in module.procedures:
                module.procedures[qname] = proc
        # Merge the file's load-time top-level setup (``namespace eval`` /
        # ``variable`` declarations the procs depend on) ahead of the
        # user's code so it runs first.  Most index files lift their
        # whole body into ``procedures`` and leave this empty.
        if sub.top_level.statements:
            module.top_level = IRScript(
                statements=tuple(sub.top_level.statements) + tuple(module.top_level.statements)
            )
        if sub.namespace_imports:
            module.namespace_imports = module.namespace_imports + sub.namespace_imports
        if sub.namespace_exports:
            module.namespace_exports = module.namespace_exports + sub.namespace_exports

        # Transitive: queue commands the freshly-bundled procs reference.
        new_cmds: set[str] = set()
        for proc in sub.procedures.values():
            _collect_commands(proc.body, new_cmds)
        _collect_commands(sub.top_level, new_cmds)
        pending |= new_cmds - seen_commands

    return module
