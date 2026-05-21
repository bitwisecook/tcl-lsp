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

import os
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

# Standard-library commands verified to bundle *and run* correctly when
# compiled into the WASM module (output matches reference tclsh 8.6/9.0).
# Only these seed compile-time bundling by default; everything else
# falls back to the runtime ``TCL_LIBRARY`` auto-loader.  "Compiles" is
# not enough — some library procs depend on init-time state set
# elsewhere in the bootstrap — so this list grows only as commands are
# differentially validated (see ``tests/test_wasm_autoload.py``).
# Transitive helpers a listed command pulls in (e.g. word.tcl's
# ``::tcl::UpdateWordBreakREs``) come along automatically and need not
# be listed.
DEFAULT_STDLIB_ALLOWLIST: frozenset[str] = frozenset(
    {
        "parray",
        "tcl_endOfWord",
        "tcl_startOfNextWord",
        "tcl_startOfPreviousWord",
        "tcl_wordBreakAfter",
        "tcl_wordBreakBefore",
    }
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


def _script_command_names(script_text: str, out: set[str]) -> None:
    """Collect the command name of each command in *script_text* and
    recurse into any ``[...]`` substitutions it contains.

    ``script_text`` is treated as a Tcl *script*: the first word of each
    command (after a command boundary) is a command name.  Used for the
    bodies of command substitutions, where the leading word genuinely is
    a command.
    """
    from core.parsing.lexer import TclLexer
    from core.parsing.tokens import TokenType

    at_cmd_start = True
    for tok in TclLexer(script_text).tokenise_all():
        ttype = tok.type
        if ttype in (TokenType.EOL,):
            at_cmd_start = True
        elif ttype == TokenType.SEP:
            # Leading whitespace before the command word doesn't end the
            # command-start state.
            continue
        elif ttype == TokenType.COMMENT:
            continue
        else:
            if at_cmd_start and ttype in (TokenType.ESC, TokenType.STR):
                out.add(tok.text)
            at_cmd_start = False
            if ttype == TokenType.CMD:
                _script_command_names(tok.text, out)


def _subst_commands(word_text: str, out: set[str]) -> None:
    """Collect command names from ``[...]`` substitutions inside a *word*.

    The word's own bare text is NOT a command (e.g. ``lappend l foo``'s
    ``foo`` is data) — only the contents of command substitutions are.
    """
    from core.parsing.lexer import TclLexer
    from core.parsing.tokens import TokenType

    for tok in TclLexer(word_text).tokenise_all():
        if tok.type == TokenType.CMD:
            _script_command_names(tok.text, out)


def _collect_commands(script: IRScript, out: set[str]) -> None:
    """Collect every command name invoked anywhere in *script*.

    Records statement-level commands (both the surface ``command`` and
    the resolved ``canonical_command`` so ``parray`` and ``::parray``
    both match), plus commands embedded in ``[...]`` substitutions inside
    call arguments and value assignments — the dominant case for
    library procs that return values (``set i [tcl_endOfWord ...]``).
    """
    for stmt in script.statements:
        if isinstance(stmt, IRCall):
            if stmt.command:
                out.add(stmt.command)
            if stmt.canonical_command:
                out.add(stmt.canonical_command)
            for arg in stmt.args:
                _subst_commands(arg, out)
        else:
            value = getattr(stmt, "value", None)
            if isinstance(value, str):
                _subst_commands(value, out)
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
    allowlist: frozenset[str] | None = DEFAULT_STDLIB_ALLOWLIST,
) -> IRModule:
    """Bundle the library files for stdlib commands *module* references.

    Mutates and returns *module*: procedures (and load-time top-level
    setup) from each referenced library file are merged in.  Resolution
    is transitive — a bundled proc that calls another library command
    pulls that file in too — to a fixpoint, capped at *max_files*.

    *allowlist* gates which *referenced* commands seed bundling: only
    commands in it (default :data:`DEFAULT_STDLIB_ALLOWLIST`, the
    differentially-validated set) are bundled from the user's module.
    Pass ``None`` to bundle every referenced command (power / validation
    use).  Transitive helpers a seeded command pulls in are never gated
    — they're part of that command's verified closure.

    A no-op (returns *module* unchanged) when *library_dir* has no
    ``tclIndex`` or references nothing bundleable, so callers can pass a
    missing path harmlessly.
    """
    lib = Path(library_dir)
    mapping = _parse_tcl_index(lib)
    if not mapping:
        return module

    bundled_files: set[str] = set()
    seen_commands: set[str] = set()

    # Seed the worklist with commands referenced by the user's module,
    # gated by the allowlist (transitive deps added later are not gated).
    pending: set[str] = set()
    _collect_commands(module.top_level, pending)
    for proc in list(module.procedures.values()):
        _collect_commands(proc.body, pending)
    if allowlist is not None:
        pending = {n for n in pending if n in allowlist or n.lstrip(":") in allowlist}

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


def apply_stdlib_prelude_auto(
    module: IRModule,
    library_dir: str | Path | None = None,
    *,
    allowlist: frozenset[str] | None = DEFAULT_STDLIB_ALLOWLIST,
) -> IRModule:
    """Apply :func:`apply_stdlib_prelude`, resolving the library directory
    from the compile-time ``TCL_LIBRARY`` env var when *library_dir* is
    not given.

    This is the default-on entry point: bundling happens whenever a
    standard library is discoverable (explicit path or ``TCL_LIBRARY``),
    and is a no-op otherwise.  Shared by the compile-time linker
    (:func:`core.compiler.codegen.wasm_link`) and the WASM CLI so both
    pick up the same behaviour.
    """
    effective = library_dir if library_dir is not None else os.environ.get("TCL_LIBRARY")
    if not effective:
        return module
    return apply_stdlib_prelude(module, effective, allowlist=allowlist)
