"""Namespace command implementation."""

from __future__ import annotations

import fnmatch
from typing import TYPE_CHECKING

from ..scope import (
    ensure_namespace,
    namespace_qualifiers,
    namespace_tail,
    resolve_namespace,
)
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..commands import CommandHandler
    from ..interp import EnsembleConfig, TclInterp


def _resolve_ns_name(interp: TclInterp, name: str) -> str:
    """Turn a potentially relative namespace name into a fully-qualified one."""
    if name.startswith("::"):
        return name
    cur = interp.current_namespace.qualname
    if cur == "::":
        return f"::{name}"
    return f"{cur}::{name}"


def _cmd_namespace(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "namespace subcommand ?arg ...?"')

    subcmd = args[0]
    rest = args[1:]

    match subcmd:
        case "eval":
            return _ns_eval(interp, rest)
        case "current":
            if rest:
                raise TclError('wrong # args: should be "namespace current"')
            return TclResult(value=interp.current_namespace.qualname)
        case "export":
            return _ns_export(interp, rest)
        case "import":
            return _ns_import(interp, rest)
        case "exists" | "exist":
            return _ns_exists(interp, rest)
        case "children":
            return _ns_children(interp, rest)
        case "parent":
            return _ns_parent(interp, rest)
        case "delete":
            return _ns_delete(interp, rest)
        case "qualifiers":
            if len(rest) != 1:
                raise TclError('wrong # args: should be "namespace qualifiers string"')
            return TclResult(value=namespace_qualifiers(rest[0]))
        case "tail":
            if len(rest) != 1:
                raise TclError('wrong # args: should be "namespace tail string"')
            return TclResult(value=namespace_tail(rest[0]))
        case "which":
            return _ns_which(interp, rest)
        case "code":
            return _ns_code(interp, rest)
        case "inscope":
            return _ns_inscope(interp, rest)
        case "path":
            return _ns_path(interp, rest)
        case "upvar":
            return _ns_upvar(interp, rest)
        case "unknown":
            return _ns_unknown(interp, rest)
        case "ensemble":
            return _ns_ensemble(interp, rest)
        case "origin":
            return _ns_origin(interp, rest)
        case _:
            raise TclError(
                f'bad option "{subcmd}": must be children, code, current, delete, '
                f"ensemble, eval, exists, export, import, inscope, origin, parent, "
                f"path, qualifiers, tail, unknown, upvar, or which"
            )


def _ns_eval(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace eval ns body ?body ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "namespace eval name arg ?arg ...?"')

    ns_name = _resolve_ns_name(interp, args[0])
    ns = ensure_namespace(interp.root_namespace, ns_name)

    # Concatenate remaining args as the body
    body = " ".join(args[1:]) if len(args) > 2 else args[1]

    # Execute in the namespace context.  Each namespace has its own
    # persistent CallFrame so that variables set during ``namespace eval``
    # are scoped to the namespace (matching real Tcl behaviour).  The
    # global namespace's frame is the interpreter's global frame.
    saved_ns = interp.current_namespace
    saved_frame = interp.current_frame
    interp.current_namespace = ns
    interp.current_frame = ns.get_frame(interp)
    try:
        result = interp.eval(body)
        return result
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame = saved_frame


def _ns_export(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace export ?-clear? ?pattern ...?"""
    ns = interp.current_namespace
    clear = False
    patterns = []
    for arg in args:
        if arg == "-clear":
            clear = True
        else:
            patterns.append(arg)

    if clear:
        ns._export_patterns.clear()

    if not patterns:
        # Return current export list
        return TclResult(value=" ".join(ns._export_patterns))

    for pat in patterns:
        ns.add_export(pat)
    return TclResult()


def _ns_import(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace import ?-force? ?pattern ...?"""
    force = False
    patterns = []
    for arg in args:
        if arg == "-force":
            force = True
        else:
            patterns.append(arg)

    for pattern in patterns:
        # Pattern like ::foo::bar::* or ::tcltest::*
        qual = namespace_qualifiers(pattern)
        tail_pat = namespace_tail(pattern)
        source_ns = resolve_namespace(interp.root_namespace, qual)
        if source_ns is None:
            raise TclError(f'unknown namespace in import pattern "{pattern}"')

        exported = source_ns.exported_commands()
        for cmd_name in exported:
            if not fnmatch.fnmatch(cmd_name, tail_pat):
                continue
            # Check if source has the command
            handler = source_ns.lookup_command(cmd_name)
            proc = source_ns.lookup_proc(cmd_name)
            if handler is not None:
                existing = interp.current_namespace.lookup_command(cmd_name)
                if existing is not None and not force:
                    # Real Tcl silently overwrites if the existing command
                    # is the same handler (re-import from same source).
                    if existing is not handler:
                        raise TclError(f'can\'t import command "{cmd_name}": already exists')
                interp.current_namespace.register_command(cmd_name, handler)
            elif proc is not None:
                existing = interp.current_namespace.lookup_proc(cmd_name)
                if existing is not None and not force:
                    if existing is not proc:
                        raise TclError(f'can\'t import command "{cmd_name}": already exists')
                interp.current_namespace.register_proc(cmd_name, proc)
                # Also register in flat procedures table for backwards compat
                interp.procedures[cmd_name] = proc

    return TclResult()


def _ns_exists(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace exists name"""
    if not args:
        raise TclError('wrong # args: should be "namespace exists name"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    return TclResult(value="1" if ns is not None else "0")


def _ns_children(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace children ?ns? ?pattern?"""
    if not args:
        ns = interp.current_namespace
    else:
        ns_name = _resolve_ns_name(interp, args[0])
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'namespace "{args[0]}" not found')

    pattern = args[1] if len(args) > 1 else None
    children = list(ns.children.values())

    if pattern:
        children = [c for c in children if fnmatch.fnmatch(c.qualname, pattern)]

    return TclResult(value=" ".join(c.qualname for c in children))


def _ns_parent(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace parent ?ns?"""
    if len(args) > 1:
        raise TclError('wrong # args: should be "namespace parent ?name?"')
    if not args:
        ns = interp.current_namespace
    else:
        ns_name = _resolve_ns_name(interp, args[0])
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'namespace "{ns_name}" not found in "::"')

    if ns.parent is None:
        return TclResult(value="")
    return TclResult(value=ns.parent.qualname)


def _ns_delete(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace delete ?name ...?"""
    for name in args:
        ns_name = _resolve_ns_name(interp, name)
        ns = resolve_namespace(interp.root_namespace, ns_name)
        if ns is None:
            raise TclError(f'unknown namespace "{ns_name}" in namespace delete command')
        if ns is interp.root_namespace:
            raise TclError("cannot delete the global namespace")
        # If this namespace belongs to an OO object, destroy the object
        oo_obj_name = getattr(ns, "_oo_object_name", None)
        if oo_obj_name is not None:
            oo = getattr(interp, "_oo_runtime", None)
            if oo is not None:
                obj = oo.objects.get(oo_obj_name)
                if obj is not None:
                    try:
                        oo._destroy_object(interp, obj)
                    except Exception:
                        pass  # ignore destruction errors
        # Clean up ensemble commands owned by this namespace
        _cleanup_ensembles_for_namespace(interp, ns_name)
        ns.delete()
    return TclResult()


def _cleanup_ensembles_for_namespace(interp: TclInterp, ns_name: str) -> None:
    """Remove ensemble metadata and commands when a namespace is deleted."""
    to_remove = [fqn for fqn, cfg in interp.ensembles.items() if cfg.namespace == ns_name]
    for fqn in to_remove:
        del interp.ensembles[fqn]
        # Remove the registered command (use tail name for unqualified commands)
        tail = namespace_tail(fqn)
        interp.unregister_command(tail)
        interp.unregister_command(fqn)


def _ns_which(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace which ?-command|-variable? name"""
    if not args or len(args) > 2:
        raise TclError('wrong # args: should be "namespace which ?-command? ?-variable? name"')

    which_type = "command"
    if len(args) == 2:
        if args[0] == "-command":
            which_type = "command"
        elif args[0] == "-variable":
            which_type = "variable"
        else:
            raise TclError('wrong # args: should be "namespace which ?-command? ?-variable? name"')
        name = args[1]
    else:
        # Single arg: always treated as the name (even if it looks like a flag)
        name = args[0]

    if which_type == "command":
        # Check current namespace
        if interp.current_namespace.lookup_command(name) is not None:
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        if interp.current_namespace.lookup_proc(name) is not None:
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        # Check global builtins
        if interp.lookup_command(name) is not None:
            return TclResult(value=f"::{name}")
        if name in interp.procedures:
            return TclResult(value=f"::{name}")
        return TclResult(value="")
    else:
        # variable — for fully-qualified names, check if the namespace exists
        if name.startswith("::"):
            last_sep = name.rfind("::")
            if last_sep > 0:
                ns_part = name[:last_sep] or "::"
                target_ns = resolve_namespace(interp.root_namespace, ns_part)
                if target_ns is not None:
                    return TclResult(value=name)
            # Global namespace variable
            return TclResult(value=name if interp.global_frame.exists(name.lstrip(":")) else "")
        # Relative name — check the namespace's own frame
        ns_frame = interp.current_namespace.get_frame(interp)
        if ns_frame.exists(name):
            if interp.current_namespace.qualname == "::":
                return TclResult(value=f"::{name}")
            return TclResult(value=f"{interp.current_namespace.qualname}::{name}")
        return TclResult(value="")


def _ns_code(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace code script"""
    if len(args) != 1:
        raise TclError('wrong # args: should be "namespace code arg"')
    script = args[0]
    # If already scoped, return as-is
    if script.startswith("::namespace inscope "):
        return TclResult(value=script)
    ns = interp.current_namespace.qualname
    return TclResult(value=f"::namespace inscope {ns} {script}")


def _ns_inscope(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace inscope ns arg ?arg ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "namespace inscope name arg ?arg ...?"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    if ns is None:
        raise TclError(f'unknown namespace "{ns_name}"')

    body = args[1]
    if len(args) > 2:
        # Extra args are appended to the script as arguments
        body = body + " " + " ".join(args[2:])

    saved_ns = interp.current_namespace
    saved_frame = interp.current_frame
    interp.current_namespace = ns
    interp.current_frame = ns.get_frame(interp)
    try:
        return interp.eval(body)
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame = saved_frame


def _ns_path(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace path ?nsList?"""
    ns = interp.current_namespace
    if not args:
        return TclResult(value=" ".join(n.qualname for n in ns._path))

    from ..machine import _split_list

    ns_names = _split_list(args[0])
    ns._path = []
    for name in ns_names:
        resolved = resolve_namespace(interp.root_namespace, _resolve_ns_name(interp, name))
        if resolved is None:
            raise TclError(f'namespace "{name}" not found in "namespace path"')
        ns._path.append(resolved)
    return TclResult()


def _ns_upvar(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace upvar ns ?otherVar myVar ...?"""
    if len(args) < 1 or (len(args) > 1 and not len(args) & 1):
        raise TclError('wrong # args: should be "namespace upvar ns ?otherVar myVar ...?"')
    ns_name = _resolve_ns_name(interp, args[0])
    ns = resolve_namespace(interp.root_namespace, ns_name)
    if ns is None:
        raise TclError(f'namespace "{ns_name}" not found')

    # Create aliases from current frame to the namespace's own frame
    ns_frame = ns.get_frame(interp)
    for i in range(1, len(args), 2):
        other_var = args[i]
        my_var = args[i + 1]
        interp.current_frame.add_alias(my_var, ns_frame, other_var)

    return TclResult()


def _ns_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace unknown ?script?"""
    ns = interp.current_namespace
    if not args:
        return TclResult(value=ns._unknown_handler or "")
    ns._unknown_handler = args[0]
    return TclResult()


def _ns_ensemble(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace ensemble create/configure/exists"""
    if not args:
        raise TclError('wrong # args: should be "namespace ensemble subcommand ?arg ...?"')

    subcmd = args[0]
    rest = args[1:]

    match subcmd:
        case "create":
            return _ns_ensemble_create(interp, rest)
        case "configure":
            return _ns_ensemble_configure(interp, rest)
        case "exists":
            if not rest:
                raise TclError('wrong # args: should be "namespace ensemble exists cmdname"')
            fqn = _resolve_ns_name(interp, rest[0])
            return TclResult(value="1" if fqn in interp.ensembles else "0")
        case _:
            raise TclError(f'bad subcommand "{subcmd}": must be configure, create, or exists')


def _parse_ensemble_options(
    interp: TclInterp, args: list[str]
) -> dict[str, str | list[str] | dict[str, str] | bool]:
    """Parse ensemble option-value pairs, returning a dict of option -> value."""
    from ..machine import _split_list

    if len(args) & 1:
        raise TclError('wrong # args: should be "namespace ensemble create ?option value ...?"')

    result: dict[str, str | list[str] | dict[str, str] | bool] = {}
    i = 0
    while i < len(args):
        opt = args[i]
        val = args[i + 1]
        match opt:
            case "-command":
                result["command"] = val
            case "-subcommands":
                result["subcommands"] = list(_split_list(val))
            case "-map":
                items = list(_split_list(val))
                if len(items) & 1:
                    raise TclError("missing value to go with key")
                m: dict[str, str] = {}
                for j in range(0, len(items), 2):
                    impl = items[j + 1]
                    if not impl:
                        raise TclError(
                            "ensemble subcommand implementations must be non-empty lists"
                        )
                    m[items[j]] = impl
                result["map"] = m
            case "-prefixes":
                result["prefixes"] = val not in ("0", "off", "false", "no")
            case "-unknown":
                result["unknown"] = val
            case "-parameters":
                result["parameters"] = list(_split_list(val))
            case _:
                raise TclError(f'unknown option "{opt}"')
        i += 2
    return result


def _ns_ensemble_create(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace ensemble create ?-option value ...?"""
    from ..interp import EnsembleConfig

    ns = interp.current_namespace
    opts = _parse_ensemble_options(interp, args)

    if "command" in opts:
        command_name = str(opts["command"])
        if command_name.startswith("::"):
            fqn = command_name
        else:
            fqn = f"::{command_name}"
    else:
        # Default: use the namespace tail as command name, FQN is the namespace
        command_name = namespace_tail(ns.qualname)
        fqn = ns.qualname if ns.qualname.startswith("::") else f"::{ns.qualname}"

    config = EnsembleConfig(
        namespace=ns.qualname,
        map=dict(opts["map"]) if "map" in opts else {},
        subcommands=list(opts["subcommands"]) if "subcommands" in opts else [],
        unknown=str(opts.get("unknown", "")),
        prefixes=bool(opts.get("prefixes", True)),
        parameters=list(opts["parameters"]) if "parameters" in opts else [],
    )

    # Qualify map values relative to the namespace
    if config.map:
        qualified_map: dict[str, str] = {}
        for sub, impl in config.map.items():
            if not impl.startswith("::"):
                impl = f"{ns.qualname}::{impl}" if ns.qualname != "::" else f"::{impl}"
            qualified_map[sub] = impl
        config.map = qualified_map

    interp.ensembles[fqn] = config
    handler = _make_ensemble_handler(fqn)
    interp.register_command(command_name, handler)

    # Register in the parent namespace's command table so that
    # namespace export/import can find the ensemble command.
    tail = namespace_tail(fqn)
    if ns.parent is not None:
        ns.parent.register_command(tail, handler)
    else:
        # Global namespace — register in root
        interp.root_namespace.register_command(tail, handler)

    return TclResult(value=fqn)


def _ns_ensemble_configure(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace ensemble configure cmdname ?-option? ?value? ..."""
    if not args:
        raise TclError(
            'wrong # args: should be "namespace ensemble configure cmdname ?-option? ?value ...?"'
        )

    cmd_name = args[0]
    # Resolve command name to FQN — try direct, then with :: prefix
    fqn = cmd_name if cmd_name.startswith("::") else f"::{cmd_name}"
    config = interp.ensembles.get(fqn)
    if config is None:
        # Also try resolving relative to current namespace
        fqn = _resolve_ns_name(interp, cmd_name)
        config = interp.ensembles.get(fqn)
    if config is None:
        raise TclError(f'"{cmd_name}" is not an ensemble command')

    rest = args[1:]
    if not rest:
        # Return all options as a flat list
        return TclResult(value=_ensemble_config_dict(config))

    if len(rest) == 1:
        # Query a single option
        return TclResult(value=_ensemble_config_get(config, rest[0]))

    # Set options
    from ..machine import _split_list

    if len(rest) & 1:
        raise TclError("wrong # args: must have even number of args after cmdname")

    i = 0
    while i < len(rest):
        opt = rest[i]
        val = rest[i + 1]
        match opt:
            case "-map":
                items = list(_split_list(val))
                if len(items) & 1:
                    raise TclError("missing value to go with key")
                m: dict[str, str] = {}
                for j in range(0, len(items), 2):
                    impl = items[j + 1]
                    if not impl:
                        raise TclError(
                            "ensemble subcommand implementations must be non-empty lists"
                        )
                    if not impl.startswith("::"):
                        ns_qual = config.namespace
                        impl = f"{ns_qual}::{impl}" if ns_qual != "::" else f"::{impl}"
                    m[items[j]] = impl
                config.map = m
            case "-subcommands":
                config.subcommands = list(_split_list(val))
            case "-unknown":
                config.unknown = val
            case "-prefixes":
                config.prefixes = val not in ("0", "off", "false", "no")
            case "-parameters":
                config.parameters = list(_split_list(val))
            case _:
                raise TclError(f'unknown option "{opt}"')
        i += 2

    return TclResult()


def _tcl_list_value(val: str) -> str:
    """Wrap a value for Tcl list output: empty strings become {}."""
    return val if val else "{}"


def _ensemble_config_dict(config: "EnsembleConfig") -> str:
    """Format ensemble config as a Tcl dict string."""
    map_str = _tcl_dict_str(config.map)
    params_str = " ".join(config.parameters) if config.parameters else ""
    subcmds_str = " ".join(config.subcommands) if config.subcommands else ""
    prefix_str = "1" if config.prefixes else "0"

    parts = [
        "-map",
        _tcl_list_value(map_str),
        "-namespace",
        config.namespace,
        "-parameters",
        _tcl_list_value(params_str),
        "-prefixes",
        prefix_str,
        "-subcommands",
        _tcl_list_value(subcmds_str),
        "-unknown",
        _tcl_list_value(config.unknown),
    ]
    return " ".join(parts)


def _ensemble_config_get(config: "EnsembleConfig", option: str) -> str:
    """Get a single ensemble config option value."""
    match option:
        case "-map":
            return _tcl_dict_str(config.map)
        case "-namespace":
            return config.namespace
        case "-parameters":
            return " ".join(config.parameters)
        case "-prefixes":
            return "1" if config.prefixes else "0"
        case "-subcommands":
            return " ".join(config.subcommands)
        case "-unknown":
            return config.unknown
        case _:
            raise TclError(f'unknown option "{option}"')


def _tcl_dict_str(d: dict[str, str]) -> str:
    """Format a dict as a Tcl dict string."""
    if not d:
        return ""
    parts: list[str] = []
    for k, v in d.items():
        parts.append(k)
        parts.append(v)
    return " ".join(parts)


def _get_ensemble_subcommands(interp: TclInterp, config: "EnsembleConfig") -> list[str]:
    """Get the effective subcommand names for an ensemble."""
    if config.subcommands:
        return config.subcommands
    if config.map:
        return sorted(config.map.keys())
    # Use exported commands from the namespace
    ns = resolve_namespace(interp.root_namespace, config.namespace)
    if ns is not None:
        return sorted(ns.exported_commands())
    return []


def _make_ensemble_handler(fqn: str) -> "CommandHandler":
    """Create a command handler closure for the ensemble at *fqn*."""
    # Use a mutable container so rename can update the FQN
    ref = [fqn]

    def ensemble_handler(interp_: TclInterp, handler_args: list[str]) -> TclResult:  # noqa: C901
        current_fqn = ref[0]
        config = interp_.ensembles.get(current_fqn)
        if config is None:
            raise TclError(f'invalid command name "{current_fqn}"')

        # Determine display name for error messages
        display = namespace_tail(current_fqn)

        # Handle -parameters: consume fixed leading args before subcommand
        params = config.parameters
        if params:
            if len(handler_args) < len(params) + 1:
                param_usage = " ".join(params)
                raise TclError(
                    f'wrong # args: should be "{display} {param_usage} subcommand ?arg ...?"'
                )
            # Skip parameter values, keep them for potential forwarding
            remaining = handler_args[len(params) :]
        else:
            remaining = handler_args

        if not remaining:
            raise TclError(f'wrong # args: should be "{display} subcommand ?arg ...?"')

        sub = remaining[0]
        sub_args = remaining[1:]

        # Resolve via map (including when -subcommands + -map both set)
        effective_map = config.map

        # When -subcommands is set, it restricts which subcommands are valid
        if config.subcommands:
            # Check if sub is in the allowed subcommands list
            if sub in config.subcommands:
                # If there's a map entry, use it; otherwise dispatch to namespace
                if sub in effective_map:
                    return _dispatch_ensemble_target(interp_, effective_map[sub], sub_args)
                qual = f"{config.namespace}::{sub}"
                return interp_.invoke(qual, sub_args)
            # Try prefix matching if enabled
            if config.prefixes:
                matches = [s for s in config.subcommands if s.startswith(sub)]
                if len(matches) == 1:
                    matched = matches[0]
                    if matched in effective_map:
                        return _dispatch_ensemble_target(interp_, effective_map[matched], sub_args)
                    qual = f"{config.namespace}::{matched}"
                    return interp_.invoke(qual, sub_args)
            # Fall through to unknown handler / error
        elif effective_map:
            # No -subcommands, dispatch from map
            if sub in effective_map:
                return _dispatch_ensemble_target(interp_, effective_map[sub], sub_args)
            if config.prefixes:
                matches = [s for s in effective_map if s.startswith(sub)]
                if len(matches) == 1:
                    return _dispatch_ensemble_target(interp_, effective_map[matches[0]], sub_args)
        else:
            # No map, no subcommands — use namespace exports
            ns = resolve_namespace(interp_.root_namespace, config.namespace)
            if ns is not None:
                exported = ns.exported_commands()
                if sub in exported:
                    qual = f"{config.namespace}::{sub}"
                    return interp_.invoke(qual, sub_args)
                if config.prefixes:
                    matches = [s for s in exported if s.startswith(sub)]
                    if len(matches) == 1:
                        qual = f"{config.namespace}::{matches[0]}"
                        return interp_.invoke(qual, sub_args)

        # Try unknown handler
        if config.unknown:
            try:
                # Unknown handler receives: ensemble_name subcommand args...
                all_args = [current_fqn, sub] + sub_args
                result = interp_.invoke(config.unknown, all_args)
                return result
            except TclError:
                raise

        # Error: unknown subcommand
        known = _get_ensemble_subcommands(interp_, config)
        if not known:
            raise TclError(
                f'unknown subcommand "{sub}": namespace {config.namespace} '
                f"does not export any commands"
            )

        # Format error message to match C Tcl
        if config.prefixes:
            prefix = "unknown or ambiguous subcommand"
        else:
            prefix = "unknown subcommand"

        if len(known) == 1:
            known_str = known[0]
        elif len(known) == 2:
            known_str = f"{known[0]}, or {known[1]}"
        else:
            known_str = ", ".join(known[:-1]) + f", or {known[-1]}"

        raise TclError(f'{prefix} "{sub}": must be {known_str}')

    ensemble_handler._ensemble_ref = ref  # type: ignore[attr-defined]
    return ensemble_handler


def _dispatch_ensemble_target(interp: TclInterp, target: str, args: list[str]) -> TclResult:
    """Dispatch to an ensemble target, handling multi-word map values."""
    from ..machine import _split_list

    parts = list(_split_list(target))
    if len(parts) == 1:
        return interp.invoke(parts[0], args)
    # Multi-word: first word is command, rest are prepended args
    return interp.invoke(parts[0], parts[1:] + args)


def _ns_origin(interp: TclInterp, args: list[str]) -> TclResult:
    """namespace origin command

    Return the fully-qualified name of the original command that
    *command* refers to.  If *command* is not an imported command,
    return its fully-qualified name directly.
    """
    if len(args) != 1:
        raise TclError('wrong # args: should be "namespace origin name"')

    name = args[0]

    # Fully-qualified name — look it up directly.
    if name.startswith("::"):
        tail = namespace_tail(name)
        qual = namespace_qualifiers(name)
        ns = resolve_namespace(interp.root_namespace, qual)
        if ns is not None:
            if ns.lookup_command(tail) is not None or ns.lookup_proc(tail) is not None:
                return TclResult(value=name)
        raise TclError(f'invalid command name "{name}"')

    # Unqualified name — search the current namespace, then global.
    cur = interp.current_namespace
    if cur.lookup_command(name) is not None or cur.lookup_proc(name) is not None:
        fqn = f"{cur.qualname}::{name}" if cur.qualname != "::" else f"::{name}"
        return TclResult(value=fqn)

    # Check global builtins / procs
    if interp.lookup_command(name) is not None or name in interp.procedures:
        return TclResult(value=f"::{name}")

    raise TclError(f'invalid command name "{name}"')


def register() -> None:
    """Register namespace commands."""
    from compiler.registry import REGISTRY

    REGISTRY.register_handler("namespace", _cmd_namespace)
