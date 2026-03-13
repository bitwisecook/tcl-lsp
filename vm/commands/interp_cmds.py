"""The ``interp`` command — child interpreter management.

Supports ``interp create``, ``interp eval``, ``interp delete``,
``interp hide``, ``interp expose``, ``interp alias``, and
``interp bgerror``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp

# Child interpreter registry

# Global map of child interpreter name → TclInterp instance.
# Each TclInterp that creates children stores them here.
_child_interps: dict[str, "TclInterp"] = {}

# Hidden commands: interp_name → {hidden_name → (handler, original_name)}
_hidden_commands: dict[str, dict[str, tuple[object, str]]] = {}

# Interp aliases: interp_name → {alias_name → (target_interp, target_cmd, extra_args)}
_interp_aliases: dict[str, dict[str, tuple[str, str, list[str]]]] = {}

# Background error handlers: interp_name → handler_command
_bgerror_handlers: dict[str, str] = {}


def _get_interp_name(interp: TclInterp) -> str:
    """Return a canonical name for this interpreter (for hidden-cmd tracking)."""
    return getattr(interp, "_interp_name", "")


def _set_interp_name(interp: TclInterp, name: str) -> None:
    """Tag an interpreter with its canonical name."""
    interp._interp_name = name  # type: ignore[attr-defined]


def _resolve_interp(caller: TclInterp, path: str) -> "TclInterp":
    """Resolve an interpreter name to its TclInterp instance.

    An empty string ``""`` or ``{}`` refers to the caller itself.
    """
    if not path or path == "{}":
        return caller
    child = _child_interps.get(path)
    if child is None:
        raise TclError(f'could not find interpreter "{path}"')
    return child


# Main command


def _cmd_interp(interp: TclInterp, args: list[str]) -> TclResult:
    """interp subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "interp cmd ?arg ...?"')

    match args[0]:
        case "issafe":
            return TclResult(value="0")
        case "create":
            return _interp_create(interp, args[1:])
        case "eval":
            return _interp_eval(interp, args[1:])
        case "delete":
            return _interp_delete(interp, args[1:])
        case "hide":
            return _interp_hide(interp, args[1:])
        case "expose":
            return _interp_expose(interp, args[1:])
        case "alias":
            return _interp_alias(interp, args[1:])
        case "bgerror":
            return _interp_bgerror(interp, args[1:])
        case "exists":
            return _interp_exists(interp, args[1:])
        case "slaves" | "children":
            return _interp_children(interp, args[1:])
        case _:
            raise TclError(
                f'bad option "{args[0]}": must be alias, bgerror, children, '
                f"create, delete, eval, exists, expose, hide, issafe, or slaves"
            )


# Subcommands


def _interp_create(caller: TclInterp, args: list[str]) -> TclResult:
    """interp create ?-safe? ?--? ?name?"""
    from ..interp import TclInterp

    # Parse flags
    remaining = list(args)
    _safe = False
    while remaining and remaining[0].startswith("-"):
        if remaining[0] == "-safe":
            _safe = True
            remaining = remaining[1:]
        elif remaining[0] == "--":
            remaining = remaining[1:]
            break
        else:
            remaining = remaining[1:]

    if remaining:
        name = remaining[0]
    else:
        # Auto-generate name
        idx = len(_child_interps)
        name = f"interp{idx}"

    if name in _child_interps:
        raise TclError(f'interpreter named "{name}" already exists')

    child = TclInterp(source_init=False)
    _set_interp_name(child, name)
    _child_interps[name] = child
    _hidden_commands[name] = {}
    _interp_aliases[name] = {}

    # Register the child name as a command in the parent so
    # ``child_name subcommand ...`` works (Tcl's interp ensemble).
    def _child_ensemble(parent: TclInterp, cmd_args: list[str]) -> TclResult:
        if not cmd_args:
            raise TclError(f'wrong # args: should be "{name} cmd ?arg ...?"')
        subcmd = cmd_args[0]
        rest = cmd_args[1:]
        # The child's "alias" subcommand has different arg mapping:
        #   childName alias srcCmd targetCmd ?arg...?
        # maps to:
        #   interp alias childPath srcCmd "" targetCmd ?arg...?
        if subcmd == "alias" and len(rest) >= 2:
            # rest[0] = srcCmd, rest[1:] = targetCmd ?arg...?
            # Insert "" as targetPath between srcCmd and targetCmd
            return _cmd_interp(parent, ["alias", name, rest[0], ""] + rest[1:])
        # Default: interp <subcmd> <childPath> <rest...>
        return _cmd_interp(parent, [subcmd, name] + rest)

    caller.register_command(name, _child_ensemble)

    return TclResult(value=name)


def _interp_eval(caller: TclInterp, args: list[str]) -> TclResult:
    """interp eval path arg ?arg ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "interp eval path arg ?arg ...?"')

    path = args[0]
    child = _resolve_interp(caller, path)

    if len(args) == 2:
        script = args[1]
    else:
        script = " ".join(args[1:])

    return child.eval(script)


def _interp_delete(caller: TclInterp, args: list[str]) -> TclResult:
    """interp delete ?path ...?"""
    for path in args:
        if path in _child_interps:
            del _child_interps[path]
            _hidden_commands.pop(path, None)
            _interp_aliases.pop(path, None)
            # Remove the child ensemble command from the parent
            if caller.lookup_command(path):
                caller.unregister_command(path)
    return TclResult()


def _interp_hide(caller: TclInterp, args: list[str]) -> TclResult:
    """interp hide path cmdName ?hiddenCmdName?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "interp hide path cmdName ?hiddenCmdName?"')

    path = args[0]
    cmd_name = args[1]
    hidden_name = args[2] if len(args) > 2 else cmd_name

    # Validate: no namespace qualifiers in the command name (it must be global)
    if "::" in cmd_name:
        raise TclError("can only hide global namespace commands (use rename then hide)")
    if "::" in hidden_name:
        raise TclError("cannot use namespace qualifiers in hidden command token (rename)")

    target = _resolve_interp(caller, path)
    name_key = _get_interp_name(target) or path

    # Find the command
    handler = target.lookup_command(cmd_name)
    proc = target.procedures.get(cmd_name)
    if handler is None and proc is None:
        raise TclError(f'unknown command "{cmd_name}"')

    # Move to hidden
    if name_key not in _hidden_commands:
        _hidden_commands[name_key] = {}
    _hidden_commands[name_key][hidden_name] = (handler or proc, cmd_name)

    # Remove from visible
    if handler:
        target.unregister_command(cmd_name)
    if proc:
        target.procedures.pop(cmd_name, None)
        # Also remove from the root namespace proc table
        target.root_namespace._procs.pop(cmd_name, None)

    # Invalidate any aliases pointing to this command
    aliases = _interp_aliases.get(name_key, {})
    for alias_name, (target_path, target_cmd, _extra) in list(aliases.items()):
        if target_cmd == cmd_name:
            # Mark alias as broken — it will fail when called
            pass

    return TclResult()


def _interp_expose(caller: TclInterp, args: list[str]) -> TclResult:
    """interp expose path hiddenCmdName ?cmdName?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "interp expose path hiddenCmdName ?cmdName?"')

    path = args[0]
    hidden_name = args[1]
    cmd_name = args[2] if len(args) > 2 else hidden_name

    if "::" in cmd_name:
        raise TclError("cannot expose to a namespace (use expose to toplevel, then rename)")

    target = _resolve_interp(caller, path)
    name_key = _get_interp_name(target) or path

    hidden = _hidden_commands.get(name_key, {})
    if hidden_name not in hidden:
        raise TclError(f'unknown hidden command "{hidden_name}"')

    handler, _original_name = hidden.pop(hidden_name)

    # Restore to visible commands
    from ..interp import ProcDef

    if isinstance(handler, ProcDef):
        target.procedures[cmd_name] = handler
        # Also restore to the root namespace proc table
        target.root_namespace.register_proc(cmd_name, handler)
    elif handler is not None:
        target.register_command(cmd_name, handler)  # type: ignore[arg-type]

    return TclResult()


def _interp_alias(caller: TclInterp, args: list[str]) -> TclResult:
    """interp alias srcPath srcToken targetPath targetCmd ?arg ...?
    interp alias srcPath srcToken {}  — delete alias
    interp alias srcPath srcToken     — query alias
    interp alias srcPath {}           — list aliases
    """
    if len(args) < 2:
        raise TclError('wrong # args: should be "interp alias ..."')

    src_path = args[0]
    src_token = args[1]

    # List all aliases: interp alias srcPath {}
    if not src_token or src_token == "{}":
        source = _resolve_interp(caller, src_path)
        name_key = _get_interp_name(source) or src_path
        aliases = _interp_aliases.get(name_key, {})
        return TclResult(value=" ".join(sorted(aliases.keys())))

    if len(args) == 2:
        # Query: interp alias srcPath srcToken
        source = _resolve_interp(caller, src_path)
        name_key = _get_interp_name(source) or src_path
        aliases = _interp_aliases.get(name_key, {})
        if src_token in aliases:
            _tgt_path, tgt_cmd, extra = aliases[src_token]
            parts = [tgt_cmd] + extra
            return TclResult(value=" ".join(parts))
        raise TclError(f'alias "{src_token}" in path "{src_path}" not found')

    target_path = args[2]

    # Delete alias: interp alias srcPath srcToken {} (exactly 3 args — no targetCmd)
    if len(args) == 3 and (not target_path or target_path == "{}"):
        source = _resolve_interp(caller, src_path)
        name_key = _get_interp_name(source) or src_path
        aliases = _interp_aliases.get(name_key, {})
        aliases.pop(src_token, None)
        source.unregister_command(src_token)
        source.procedures.pop(src_token, None)
        return TclResult()

    if len(args) < 4:
        raise TclError(
            'wrong # args: should be "interp alias srcPath srcToken targetPath targetCmd ?arg ...?"'
        )

    target_cmd = args[3]
    extra_args = args[4:]

    source = _resolve_interp(caller, src_path)
    target = _resolve_interp(caller, target_path)
    name_key = _get_interp_name(source) or src_path

    if name_key not in _interp_aliases:
        _interp_aliases[name_key] = {}
    _interp_aliases[name_key][src_token] = (target_path, target_cmd, extra_args)

    # Register the alias as a command in the source interpreter
    def _alias_handler(
        interp: TclInterp,
        cmd_args: list[str],
        _target: TclInterp = target,
        _target_cmd: str = target_cmd,
        _extra: list[str] = extra_args,
    ) -> TclResult:
        return _target.invoke(_target_cmd, _extra + cmd_args)

    source.register_command(src_token, _alias_handler)

    return TclResult(value=src_token)


def _interp_bgerror(caller: TclInterp, args: list[str]) -> TclResult:
    """interp bgerror path ?cmdPrefix?"""
    if not args:
        raise TclError('wrong # args: should be "interp bgerror path ?cmdPrefix?"')
    path = args[0]
    target = _resolve_interp(caller, path)
    name_key = _get_interp_name(target) or path

    if len(args) < 2:
        handler = _bgerror_handlers.get(name_key, "::tcl::Bgerror")
        return TclResult(value=handler)

    _bgerror_handlers[name_key] = args[1]
    return TclResult()


def _interp_exists(caller: TclInterp, args: list[str]) -> TclResult:
    """interp exists path"""
    if not args:
        raise TclError('wrong # args: should be "interp exists path"')
    return TclResult(value="1" if args[0] in _child_interps else "0")


def _interp_children(caller: TclInterp, args: list[str]) -> TclResult:
    """interp children ?path?"""
    # For simplicity return all known child names
    names = sorted(_child_interps.keys())
    return TclResult(value=" ".join(names))


# Cleanup hook


def reset_interp_state() -> None:
    """Reset global interp state (for test isolation)."""
    _child_interps.clear()
    _hidden_commands.clear()
    _interp_aliases.clear()
    _bgerror_handlers.clear()


def register() -> None:
    """Register interp command."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("interp", _cmd_interp)
