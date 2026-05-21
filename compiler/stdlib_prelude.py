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
from collections import deque
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
            # The ``[file join $dir ...]`` tail may carry several path
            # segments (subdirectories), e.g. ``[file join $dir sub
            # foo.tcl]``.  Join them into a relative path rather than
            # treating the whole tail as one segment.
            segments = m.group(2).split()
            if segments:
                mapping[m.group(1)] = os.path.join(*segments)
    return mapping


def _script_command_names(script_text: str, out: set[str]) -> None:
    """Collect the command name of each command in *script_text* and
    recurse into any ``[...]`` substitutions it contains.

    ``script_text`` is treated as a Tcl *script*: the first word of each
    command (after a command boundary) is a command name.  Used for the
    bodies of command substitutions, where the leading word genuinely is
    a command.
    """
    from compiler.parsing.lexer import TclLexer
    from shared.tokens import TokenType

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
            if at_cmd_start:
                if ttype in (TokenType.ESC, TokenType.STR):
                    out.add(tok.text)
                elif ttype in (TokenType.VAR, TokenType.CMD):
                    # The command name itself is computed at run time
                    # (``[$cmd ...]`` / ``[[set c foo] ...]``) — record a
                    # non-literal marker so the dynamic-dispatch gate
                    # trips and pruning stays conservative.
                    out.add("${dyn}")
            at_cmd_start = False
            if ttype == TokenType.CMD:
                _script_command_names(tok.text, out)


def _subst_commands(word_text: str, out: set[str]) -> None:
    """Collect command names from ``[...]`` substitutions inside a *word*.

    The word's own bare text is NOT a command (e.g. ``lappend l foo``'s
    ``foo`` is data) — only the contents of command substitutions are.
    """
    from compiler.parsing.lexer import TclLexer
    from shared.tokens import TokenType

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


# Commands that run a script or invoke a command by a name the static
# call graph can't see — their script/command argument is an ordinary
# word (not a ``[...]`` substitution the scanner walks), so any proc
# they reach is invisible.  ``eval`` / ``uplevel`` / dynamic
# ``namespace eval`` / dynamic ``catch`` already lower to
# :class:`IRBarrier`; this set covers the rest that stay plain
# :class:`IRCall`.  If any appears, pruning is unsafe.
_DISPATCH_COMMANDS: frozenset[str] = frozenset(
    {
        "apply",
        "source",
        "subst",
        "interp",
        "after",
        "trace",
        "coroutine",
        "tailcall",
        "vwait",
        "uplevel",
        "eval",
        "namespace",
        "unknown",
    }
)


def _name_is_dynamic(name: str) -> bool:
    """A command name that isn't a compile-time literal (it's computed at
    run time via variable / command substitution, so it could resolve to
    any proc)."""
    return "$" in name or "[" in name or "{" in name


def _has_barrier(script: IRScript) -> bool:
    """True if *script* (recursively) contains an :class:`IRBarrier` —
    eval / uplevel / dynamic ``namespace eval`` / dynamic ``catch`` and
    friends, which run scripts the static analysis can't see into."""
    from .ir import IRBarrier

    for stmt in script.statements:
        if isinstance(stmt, IRBarrier):
            return True
        # Reuse the body-recursion shape; collect into a throwaway set
        # is overkill, so walk bodies directly here.
        if _stmt_bodies_have_barrier(stmt):
            return True
    return False


def _stmt_bodies_have_barrier(stmt: IRStatement) -> bool:
    if isinstance(stmt, IRIf):
        return any(_has_barrier(c.body) for c in stmt.clauses) or (
            stmt.else_body is not None and _has_barrier(stmt.else_body)
        )
    if isinstance(stmt, IRFor):
        return _has_barrier(stmt.init) or _has_barrier(stmt.body) or _has_barrier(stmt.next)
    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRBlock, IRUpFrame)):
        return _has_barrier(stmt.body)
    if isinstance(stmt, IRTry):
        return (
            _has_barrier(stmt.body)
            or any(_has_barrier(h.body) for h in stmt.handlers)
            or (stmt.finally_body is not None and _has_barrier(stmt.finally_body))
        )
    if isinstance(stmt, IRSwitch):
        return any(arm.body is not None and _has_barrier(arm.body) for arm in stmt.arms) or (
            stmt.default_body is not None and _has_barrier(stmt.default_body)
        )
    return False


def _module_has_dynamic_dispatch(module: IRModule) -> bool:
    """True when the module contains *any* construct that could invoke a
    command by a name the static call graph can't enumerate — an
    :class:`IRBarrier`, a non-literal command name, or a call to a
    :data:`_DISPATCH_COMMANDS` script-runner.  Pruning bundled procs is
    only sound when this is False."""
    if _has_barrier(module.top_level):
        return True
    for proc in module.procedures.values():
        if _has_barrier(proc.body):
            return True
    refs: set[str] = set()
    _collect_commands(module.top_level, refs)
    for proc in module.procedures.values():
        _collect_commands(proc.body, refs)
    for name in refs:
        if _name_is_dynamic(name):
            return True
        if name.lstrip(":") in _DISPATCH_COMMANDS:
            return True
    return False


def _prune_unused_bundled(module: IRModule, bundled_qnames: set[str]) -> None:
    """Remove bundled procs unreachable from the user's code.

    Roots are the top level plus every *non-bundled* proc (any of which
    may run).  A bundled proc survives only if statically reachable from
    a root, transitively through other bundled procs.  Skipped entirely
    when the module has any dynamic dispatch (:func:`_module_has_dynamic
    _dispatch`) — then we cannot prove what a runtime-resolved name
    invokes, so nothing is pruned.
    """
    if not bundled_qnames:
        return
    if _module_has_dynamic_dispatch(module):
        return

    def _norm(n: str) -> str:
        return n.lstrip(":")

    bundled_by_norm: dict[str, str] = {_norm(q): q for q in bundled_qnames}

    # Seed reachability with commands referenced by the roots.
    worklist: set[str] = set()
    _collect_commands(module.top_level, worklist)
    for qname, proc in module.procedures.items():
        if qname not in bundled_qnames:
            _collect_commands(proc.body, worklist)

    live: set[str] = set()
    while worklist:
        name = worklist.pop()
        qn = bundled_by_norm.get(_norm(name))
        if qn is None or qn in live:
            continue
        live.add(qn)
        # Pull in the commands this now-live bundled proc references.
        _collect_commands(module.procedures[qn].body, worklist)

    for qname in bundled_qnames - live:
        module.procedures.pop(qname, None)


def _is_proc_definition(stmt: IRStatement) -> bool:
    """True if *stmt* is a top-level ``proc NAME ARGS BODY`` definition —
    redundant once the proc is in ``module.procedures``."""
    return isinstance(stmt, IRCall) and (
        stmt.command == "proc" or stmt.canonical_command == "::proc"
    )


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
    bundled_qnames: set[str] = set()
    seen_commands: set[str] = set()

    # Seed the worklist with commands referenced by the user's module,
    # gated by the allowlist (transitive deps added later are not gated).
    # The worklist is processed FIFO with new commands added in sorted
    # order so bundling — and the merged setup ordering below — is
    # deterministic / reproducible regardless of set iteration order.
    seed: set[str] = set()
    _collect_commands(module.top_level, seed)
    for proc in list(module.procedures.values()):
        _collect_commands(proc.body, seed)
    if allowlist is not None:
        seed = {n for n in seed if n in allowlist or n.lstrip(":") in allowlist}
    queue: deque[str] = deque(sorted(seed))

    # Load-time setup from each bundled file, accumulated in bundling
    # order and prepended once after the traversal (deterministic).
    setup: list[IRStatement] = []

    from .lowering import lower_to_ir

    while queue and len(bundled_files) < max_files:
        name = queue.popleft()
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
                bundled_qnames.add(qname)
        # Collect the file's load-time top-level setup (``namespace eval``
        # / ``variable`` declarations the procs depend on).  Drop the
        # file's ``proc`` *definition* statements: the procs are already
        # merged into ``module.procedures`` (whence codegen compiles
        # them), so the top-level ``proc`` calls are redundant — and
        # scanning their body arguments would otherwise pollute the prune
        # root set with commands the bundled procs reference internally.
        setup.extend(s for s in sub.top_level.statements if not _is_proc_definition(s))
        if sub.namespace_imports:
            module.namespace_imports = module.namespace_imports + sub.namespace_imports
        if sub.namespace_exports:
            module.namespace_exports = module.namespace_exports + sub.namespace_exports

        # Transitive: queue commands the freshly-bundled procs reference,
        # in sorted order for deterministic traversal.
        new_cmds: set[str] = set()
        for proc in sub.procedures.values():
            _collect_commands(proc.body, new_cmds)
        _collect_commands(sub.top_level, new_cmds)
        queue.extend(sorted(new_cmds - seen_commands))

    # Prepend all bundled setup ahead of the user's code so it runs
    # first, once, in deterministic bundling order.
    if setup:
        module.top_level = IRScript(statements=tuple(setup) + tuple(module.top_level.statements))

    # Drop bundled procs the program can't statically reach — but only
    # when it's provably safe (no eval/source/dynamic dispatch).
    _prune_unused_bundled(module, bundled_qnames)

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
