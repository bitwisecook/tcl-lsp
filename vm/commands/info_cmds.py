"""The ``info`` command and its subcommands."""

from __future__ import annotations

import os
import sys
from typing import TYPE_CHECKING

from ..machine import _list_escape
from ..oo import _format_method_list as _format_or_list
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_info(interp: TclInterp, args: list[str]) -> TclResult:
    """info subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "info subcommand ?arg ...?"')
    sub = args[0]
    rest = args[1:]

    # Resolve unique subcommand abbreviation — must match C Tcl 9.0's list
    # exactly so that error messages and abbreviation resolution are conformant.
    _INFO_SUBCMDS = (
        "args",
        "body",
        "class",
        "cmdcount",
        "cmdtype",
        "commands",
        "complete",
        "constant",
        "consts",
        "coroutine",
        "default",
        "errorstack",
        "exists",
        "frame",
        "functions",
        "globals",
        "hostname",
        "level",
        "library",
        "loaded",
        "locals",
        "nameofexecutable",
        "object",
        "patchlevel",
        "procs",
        "script",
        "sharedlibextension",
        "tclversion",
        "vars",
    )
    matches = [s for s in _INFO_SUBCMDS if s.startswith(sub)]
    if len(matches) == 1:
        sub = matches[0]
    elif len(matches) > 1 and sub not in _INFO_SUBCMDS:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_INFO_SUBCMDS[:-1])
            + ", or "
            + _INFO_SUBCMDS[-1]
        )

    match sub:
        case "exists":
            if not rest:
                raise TclError('wrong # args: should be "info exists varName"')
            return TclResult(value="1" if interp.current_frame.exists(rest[0]) else "0")

        case "commands":
            pattern = rest[0] if rest else "*"
            import fnmatch

            from ..scope import resolve_namespace

            names: list[str] = list(interp.command_names())
            names.extend(interp.procedures.keys())

            # If the pattern has namespace qualifiers, also search that namespace
            if "::" in pattern:
                # Normalise relative patterns to fully-qualified so fnmatch
                # works against the ``::ns::name`` entries we collect.
                if not pattern.startswith("::"):
                    cur = interp.current_namespace.qualname
                    if cur == "::":
                        pattern = "::" + pattern
                    else:
                        pattern = cur + "::" + pattern

                ns_part = pattern[: pattern.rfind("::")]
                ns = resolve_namespace(interp.root_namespace, ns_part if ns_part else "::")
                if ns is not None:
                    # Add namespace procs with fully-qualified names
                    for pname in ns.proc_names():
                        fq = ns.qualname + "::" + pname
                        names.append(fq)
                    # Add namespace commands with fully-qualified names
                    for cname in ns.command_names():
                        fq = ns.qualname + "::" + cname
                        names.append(fq)

            matched = [n for n in sorted(set(names)) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "procs":
            pattern = rest[0] if rest else "*"
            import fnmatch

            matched = [n for n in sorted(interp.procedures.keys()) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "vars":
            pattern = rest[0] if rest else "*"
            import fnmatch

            from ..scope import resolve_namespace

            if "::" in pattern:
                # Qualified pattern — search the target namespace's frame
                if not pattern.startswith("::"):
                    cur = interp.current_namespace.qualname
                    if cur == "::":
                        pattern = "::" + pattern
                    else:
                        pattern = cur + "::" + pattern
                ns_part = pattern[: pattern.rfind("::")]
                ns = resolve_namespace(interp.root_namespace, ns_part if ns_part else "::")
                if ns is not None:
                    ns_frame = ns.get_frame(interp)
                    prefix = ns.qualname + "::" if ns.qualname != "::" else "::"
                    names = [prefix + n for n in ns_frame.var_names()]
                else:
                    names = []
            else:
                names = interp.current_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "globals":
            pattern = rest[0] if rest else "*"
            import fnmatch

            names = interp.global_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "locals":
            pattern = rest[0] if rest else "*"
            import fnmatch

            names = interp.current_frame.var_names()
            matched = [n for n in sorted(names) if fnmatch.fnmatch(n, pattern)]
            return TclResult(value=" ".join(_list_escape(n) for n in matched))

        case "body":
            if not rest:
                raise TclError('wrong # args: should be "info body procname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            return TclResult(value=proc.body)

        case "args":
            if not rest:
                raise TclError('wrong # args: should be "info args procname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            return TclResult(value=" ".join(_list_escape(p) for p in proc.param_names))

        case "default":
            if len(rest) < 3:
                raise TclError('wrong # args: should be "info default procname arg varname"')
            proc = interp.procedures.get(rest[0])
            if proc is None:
                raise TclError(f'"{rest[0]}" isn\'t a procedure')
            param_name = rest[1]
            var_name = rest[2]
            for pn, default in proc.params_with_defaults:
                if pn == param_name:
                    if default is not None:
                        interp.current_frame.set_var(var_name, default)
                        return TclResult(value="1")
                    interp.current_frame.set_var(var_name, "")
                    return TclResult(value="0")
            raise TclError(f'procedure "{rest[0]}" doesn\'t have an argument "{param_name}"')

        case "level":
            if not rest:
                return TclResult(value=str(interp.current_frame.level))
            level = int(rest[0])
            # Non-positive levels are relative: 0 = current, -1 = one up
            if level <= 0:
                frame = interp.frame_at_relative(-level)
            else:
                frame = interp.frame_at_level(level)
            if frame.proc_name:
                # Return the full invocation: command name + args
                parts = [_list_escape(frame.proc_name)]
                if frame.call_args:
                    parts.extend(_list_escape(a) for a in frame.call_args)
                return TclResult(value=" ".join(parts))
            return TclResult(value="")

        case "script":
            return TclResult(value=interp.script_file or "")

        case "nameofexecutable":
            return TclResult(value=sys.executable)

        case "patchlevel":
            return TclResult(value=interp.global_frame.get_var("tcl_patchLevel", default="9.0.3"))

        case "tclversion":
            return TclResult(value=interp.global_frame.get_var("tcl_version", default="9.0"))

        case "hostname":
            import socket

            return TclResult(value=socket.gethostname())

        case "library":
            return TclResult(value=interp.global_frame.get_var("tcl_library", default=""))

        case "sharedlibextension":
            if sys.platform == "darwin":
                return TclResult(value=".dylib")
            if sys.platform == "win32":
                return TclResult(value=".dll")
            return TclResult(value=".so")

        case "loaded":
            return TclResult(value="")

        case "cmdcount":
            return TclResult(value=str(interp.cmd_count))

        case "frame":
            if not rest:
                # Return current frame depth, using _info_parent for
                # true call chain depth (next-chained methods count).
                depth = 0
                f = interp.current_frame
                while f is not None:
                    depth += 1
                    f = getattr(f, "_info_parent", None) or getattr(f, "parent", None)
                return TclResult(value=str(depth))
            try:
                frame_level = int(rest[0])
            except ValueError:
                raise TclError(f'expected integer but got "{rest[0]}"')
            return _info_frame(interp, frame_level)

        case "complete":
            if not rest:
                raise TclError('wrong # args: should be "info complete command"')
            return TclResult(value="1" if interp.is_complete(rest[0]) else "0")

        case "object":
            return _info_object(interp, rest)

        case "class":
            return _info_class(interp, rest)

        case "coroutine":
            return TclResult(value="")

        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{sub}": must be '
                + ", ".join(_INFO_SUBCMDS[:-1])
                + ", or "
                + _INFO_SUBCMDS[-1]
            )


def _info_frame(interp: TclInterp, level: int) -> TclResult:
    """Return dict describing a call frame.

    ``info frame 0`` is the current frame.  Positive *level* is an
    absolute frame number (1 = outermost).  Negative *level* is
    relative to the current frame.
    """
    from ..machine import _list_escape

    # Build frame stack using _info_parent for true call depth
    frames: list[object] = []
    f = interp.current_frame
    while f is not None:
        frames.append(f)
        f = getattr(f, "_info_parent", None) or getattr(f, "parent", None)
    frames.reverse()  # frames[0] = outermost, frames[-1] = current

    if level == 0:
        target = interp.current_frame
    elif level > 0:
        if level > len(frames):
            raise TclError(f"bad level \"{level}\"")
        target = frames[level - 1]
    else:
        idx = len(frames) + level
        if idx < 0:
            raise TclError(f"bad level \"{level}\"")
        target = frames[idx]

    # Build result dict as key-value pairs
    parts: list[str] = []
    parts.append("type")
    parts.append(getattr(target, "_frame_type", "eval"))

    # cmd: the currently executing command text.  When the target is the
    # current frame, this is always "info frame <level>".
    cmd = getattr(target, "_current_cmd", None)
    if cmd:
        parts.append("cmd")
        parts.append(cmd)
    elif target is interp.current_frame:
        parts.append("cmd")
        parts.append(f"info frame {level}")

    # proc: only for non-OO procedures
    oo_method = getattr(target, "_oo_method", None)
    if oo_method is None:
        proc_name = getattr(target, "proc_name", None)
        if proc_name:
            parts.append("proc")
            parts.append(proc_name)

    # OO method context
    if oo_method and oo_method not in ("<constructor>", "<destructor>"):
        parts.append("method")
        parts.append(oo_method)
    oo_class = getattr(target, "_oo_class", None)
    oo_self = getattr(target, "_oo_self", None)
    if oo_class and not oo_class.startswith("__instance__"):
        # Class-defined method: show class, not object
        parts.append("class")
        parts.append(oo_class)
    elif oo_self:
        # Instance (per-object) method: show object, not class
        parts.append("object")
        parts.append(oo_self)

    # level: 0 for current frame context
    parts.append("level")
    parts.append("0")

    return TclResult(value=" ".join(_list_escape(p) for p in parts))


def _get_oo(interp: TclInterp):
    """Get OO runtime, initialising if needed."""
    from .oo_cmds import _get_oo_runtime

    return _get_oo_runtime(interp)


def _resolve_object(interp: TclInterp, name: str):
    """Resolve an object by name, raising TclError if not found."""
    oo = _get_oo(interp)
    if oo is None:
        raise TclError(f"{name} does not refer to an object")
    obj = oo.objects.get(name)
    if obj is None and not name.startswith("::"):
        obj = oo.objects.get(f"::{name}")
    # Try resolving relative to current namespace
    if obj is None and not name.startswith("::"):
        ns = interp.current_namespace.qualname
        ns_qn = f"{ns}::{name}" if ns != "::" else f"::{name}"
        obj = oo.objects.get(ns_qn)
    if obj is None:
        raise TclError(f"{name} does not refer to an object")
    return oo, obj


def _resolve_class(interp: TclInterp, name: str):
    """Resolve a class by name, raising TclError if not a class."""
    oo = _get_oo(interp)
    if oo is None:
        raise TclError(f"{name} does not refer to an object")
    # First check if it's a known object
    obj = oo.objects.get(name) or oo.objects.get(f"::{name}" if not name.startswith("::") else name)
    cls = oo.classes.get(name)
    if cls is None and not name.startswith("::"):
        cls = oo.classes.get(f"::{name}")
    if cls is None:
        # Check if the name is an object but not a class
        if obj is not None:
            raise TclError(f'"{name}" is not a class')
        raise TclError(f"{name} does not refer to an object")
    return oo, cls


def _info_object(interp: TclInterp, args: list[str]) -> TclResult:
    """info object subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "info object subcommand ?arg ...?"')

    sub = args[0]
    rest = args[1:]

    _OBJ_SUBCMDS = (
        "call",
        "class",
        "creationid",
        "definition",
        "filters",
        "forward",
        "isa",
        "methods",
        "methodtype",
        "mixins",
        "namespace",
        "properties",
        "variables",
        "vars",
    )
    matches = [s for s in _OBJ_SUBCMDS if s.startswith(sub)]
    if len(matches) == 1:
        sub = matches[0]
    elif len(matches) > 1 and sub not in _OBJ_SUBCMDS:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_OBJ_SUBCMDS[:-1])
            + ", or "
            + _OBJ_SUBCMDS[-1]
        )
    elif not matches:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_OBJ_SUBCMDS[:-1])
            + ", or "
            + _OBJ_SUBCMDS[-1]
        )

    match sub:
        case "class":
            if not rest:
                raise TclError('wrong # args: should be "info object class objName ?className?"')
            oo, obj = _resolve_object(interp, rest[0])
            if len(rest) >= 2:
                # Two-arg form: check if object is of given class
                check_class = rest[1]
                qn = check_class if check_class.startswith("::") else f"::{check_class}"
                cls = oo.classes.get(obj.class_name)
                if cls is None:
                    return TclResult(value="0")
                mro = cls.mro(oo.classes)
                return TclResult(value="1" if qn in mro or check_class in mro else "0")
            return TclResult(value=obj.class_name)

        case "isa":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "info object isa category objName ?arg ...?"'
                )
            category = rest[0]
            obj_name = rest[1]
            oo = _get_oo(interp)

            if category == "object":
                # Check if the name refers to a known object or class
                if oo is None:
                    return TclResult(value="0")
                found = (
                    obj_name in oo.objects
                    or f"::{obj_name}" in oo.objects
                    or obj_name in oo.classes
                    or f"::{obj_name}" in oo.classes
                )
                return TclResult(value="1" if found else "0")

            elif category == "class":
                if oo is None:
                    return TclResult(value="0")
                qn = obj_name if obj_name.startswith("::") else f"::{obj_name}"
                found = qn in oo.classes or obj_name in oo.classes
                return TclResult(value="1" if found else "0")

            elif category == "metaclass":
                if oo is None:
                    return TclResult(value="0")
                qn = obj_name if obj_name.startswith("::") else f"::{obj_name}"
                cls = oo.classes.get(qn) or oo.classes.get(obj_name)
                if cls is None:
                    return TclResult(value="0")
                # A metaclass is a class whose instances are also classes
                # In Tcl, oo::class is a metaclass, and subclasses of oo::class are metaclasses
                mro = cls.mro(oo.classes)
                is_meta = "::oo::class" in mro
                return TclResult(value="1" if is_meta else "0")

            elif category == "typeof":
                if len(rest) < 3:
                    raise TclError(
                        'wrong # args: should be "info object isa typeof objName className"'
                    )
                class_name = rest[2]
                if oo is None:
                    return TclResult(value="0")
                qn_obj = obj_name if obj_name.startswith("::") else f"::{obj_name}"
                obj = oo.objects.get(obj_name) or oo.objects.get(qn_obj)
                if obj is None:
                    return TclResult(value="0")
                cls = oo.classes.get(obj.class_name)
                if cls is None:
                    return TclResult(value="0")
                qn = class_name if class_name.startswith("::") else f"::{class_name}"
                mro = cls.mro(oo.classes)
                return TclResult(value="1" if qn in mro or class_name in mro else "0")

            elif category == "mixin":
                if len(rest) < 3:
                    raise TclError(
                        'wrong # args: should be "info object isa mixin objName className"'
                    )
                class_name = rest[2]
                if oo is None:
                    return TclResult(value="0")
                qn_obj = obj_name if obj_name.startswith("::") else f"::{obj_name}"
                obj = oo.objects.get(obj_name) or oo.objects.get(qn_obj)
                if obj is None:
                    return TclResult(value="0")
                cls = oo.classes.get(obj.class_name)
                qn = class_name if class_name.startswith("::") else f"::{class_name}"
                # Check both instance-level and class-level mixins
                all_mixins = list(obj.instance_mixins)
                if cls is not None:
                    all_mixins.extend(cls.mixins)
                is_mixin = any(qn == m or class_name == m or f"::{m}" == qn for m in all_mixins)
                return TclResult(value="1" if is_mixin else "0")

            else:
                raise TclError(
                    f'unknown category "{category}": must be class, metaclass, mixin, object, or typeof'
                )

        case "methods":
            if not rest:
                raise TclError(
                    'wrong # args: should be "info object methods objName ?options ...?"'
                )
            oo, obj = _resolve_object(interp, rest[0])
            import fnmatch

            # Parse options: -all, -private, -scope scope, pattern
            all_flag = False
            private_flag = False
            scope_filter: str | None = None
            pattern = "*"
            i = 1
            while i < len(rest):
                if rest[i] == "-all":
                    all_flag = True
                    i += 1
                elif rest[i] == "-private":
                    private_flag = True
                    i += 1
                elif rest[i] == "-scope" and i + 1 < len(rest):
                    scope_filter = rest[i + 1]
                    i += 2
                else:
                    pattern = rest[i]
                    i += 1

            methods: dict[str, str] = {}  # name -> visibility
            # Instance methods
            for mname, m in obj.instance_methods.items():
                vis = getattr(m, "visibility", "public")
                if mname in obj.exported_methods:
                    vis = "public"
                elif mname in obj.unexported_methods:
                    vis = "unexported"
                methods[mname] = vis
            if all_flag:
                # Include class methods from MRO
                cls = oo.classes.get(obj.class_name)
                if cls:
                    for cqn in cls.mro(oo.classes):
                        ancestor = oo.classes.get(cqn)
                        if ancestor:
                            for mname, m in ancestor.methods.items():
                                if mname not in methods:
                                    methods[mname] = getattr(m, "visibility", "public")
                # Add built-in methods, respecting unexport from MRO
                _BUILTIN_METHODS = {
                    "destroy": "public",
                    "<cloned>": "private",
                    "eval": "private",
                    "unknown": "private",
                    "variable": "private",
                    "varname": "private",
                }
                # Collect unexported methods from entire MRO
                all_unexported: set[str] = set(getattr(obj, "unexported_methods", set()))
                if cls:
                    for cqn in cls.mro(oo.classes):
                        ancestor = oo.classes.get(cqn)
                        if ancestor:
                            all_unexported |= ancestor.unexported_methods
                for bm, bv in _BUILTIN_METHODS.items():
                    if bm not in methods:
                        if bm in all_unexported:
                            methods[bm] = "unexported"
                        else:
                            methods[bm] = bv
            if scope_filter is not None:
                methods = {k: v for k, v in methods.items() if v == scope_filter}
            elif all_flag and private_flag:
                # -all -private: show everything
                pass
            elif private_flag:
                # -private without -all: show public + unexported (not private)
                methods = {k: v for k, v in methods.items() if v in ("public", "exported", "unexported")}
            elif all_flag:
                # -all without -private: show public methods (inherited + built-in public)
                methods = {k: v for k, v in methods.items() if v in ("public", "exported")}
            else:
                # Default: show only public/exported instance methods
                methods = {k: v for k, v in methods.items() if v in ("public", "exported")}
            matched = sorted(m for m in methods if fnmatch.fnmatch(m, pattern))
            return TclResult(value=" ".join(_list_escape(m) for m in matched))

        case "namespace":
            if not rest:
                raise TclError('wrong # args: should be "info object namespace objName"')
            oo, obj = _resolve_object(interp, rest[0])
            return TclResult(value=obj.namespace)

        case "creationid":
            if len(rest) != 1:
                raise TclError(
                    'wrong # args: should be "info object creationid objName"'
                )
            oo, obj = _resolve_object(interp, rest[0])
            return TclResult(value=str(obj.creation_id))

        case "mixins":
            if not rest:
                raise TclError('wrong # args: should be "info object mixins objName"')
            oo, obj = _resolve_object(interp, rest[0])
            instance_mixins = []
            for m in obj.instance_mixins:
                if not m.startswith("::"):
                    qm = f"::{m}" if f"::{m}" in oo.classes else m
                else:
                    qm = m
                instance_mixins.append(qm)
            return TclResult(value=" ".join(_list_escape(m) for m in instance_mixins))

        case "variables":
            if not rest:
                raise TclError('wrong # args: should be "info object variables objName"')
            oo, obj = _resolve_object(interp, rest[0])
            if "-private" in rest[1:]:
                pvars = getattr(obj, "private_instance_variables", [])
                return TclResult(value=" ".join(_list_escape(v) for v in pvars))
            instance_vars = obj.instance_variables
            return TclResult(value=" ".join(_list_escape(v) for v in instance_vars))

        case "vars":
            if not rest:
                raise TclError('wrong # args: should be "info object vars objName"')
            oo, obj = _resolve_object(interp, rest[0])
            import fnmatch

            pattern = rest[1] if len(rest) > 1 else "*"
            matched = sorted(v for v in obj._vars.keys() if fnmatch.fnmatch(v, pattern))
            return TclResult(value=" ".join(_list_escape(v) for v in matched))

        case "filters":
            if not rest:
                raise TclError('wrong # args: should be "info object filters objName"')
            oo, obj = _resolve_object(interp, rest[0])
            instance_filters = obj.instance_filters
            return TclResult(value=" ".join(_list_escape(f) for f in instance_filters))

        case "definition":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "info object definition objName methodName"'
                )
            oo, obj = _resolve_object(interp, rest[0])
            method_name = rest[1]
            method = obj.instance_methods.get(method_name)
            if method is None:
                raise TclError(f'unknown method "{method_name}"')
            param_parts = [f"{{{n} {d}}}" if d is not None else _list_escape(n) for n, d in method.params]
            param_str = " ".join(param_parts)
            return TclResult(value=f"{{{param_str}}} {{{method.body}}}")

        case "methodtype":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "info object methodtype objName methodName"'
                )
            oo, obj = _resolve_object(interp, rest[0])
            method_name = rest[1]
            method = obj.instance_methods.get(method_name)
            if method is None:
                raise TclError(f'unknown method "{method_name}"')
            if method.forward_target is not None:
                return TclResult(value="forward")
            return TclResult(value="method")

        case "forward":
            if len(rest) < 2:
                raise TclError('wrong # args: should be "info object forward objName methodName"')
            oo, obj = _resolve_object(interp, rest[0])
            method_name = rest[1]
            method = obj.instance_methods.get(method_name)
            if method is None or method.forward_target is None:
                raise TclError(f'unknown forward method "{method_name}"')
            return TclResult(value=" ".join(_list_escape(t) for t in method.forward_target))

        case "call":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "info object call objName methodName"')
            oo, obj = _resolve_object(interp, rest[0])
            method_name = rest[1]
            chain = oo.build_object_call_chain(obj, method_name)
            parts = []
            for call_type, mname, cname, impl_type in chain:
                parts.append(f"{{{call_type} {mname} {cname} {impl_type}}}")
            return TclResult(value=" ".join(parts))

        case _:
            return TclResult()


def _info_class(interp: TclInterp, args: list[str]) -> TclResult:
    """info class subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "info class subcommand ?arg ...?"')

    sub = args[0]
    rest = args[1:]

    _CLS_SUBCMDS = (
        "call",
        "constructor",
        "definition",
        "definitionnamespace",
        "destructor",
        "filters",
        "forward",
        "instances",
        "methods",
        "methodtype",
        "mixins",
        "properties",
        "subclasses",
        "superclasses",
        "variables",
    )
    matches = [s for s in _CLS_SUBCMDS if s.startswith(sub)]
    if len(matches) == 1:
        sub = matches[0]
    elif len(matches) > 1 and sub not in _CLS_SUBCMDS:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_CLS_SUBCMDS[:-1])
            + ", or "
            + _CLS_SUBCMDS[-1]
        )
    elif not matches:
        raise TclError(
            f'unknown or ambiguous subcommand "{sub}": must be '
            + ", ".join(_CLS_SUBCMDS[:-1])
            + ", or "
            + _CLS_SUBCMDS[-1]
        )

    match sub:
        case "superclasses":
            if not rest:
                raise TclError('wrong # args: should be "info class superclasses className"')
            oo, cls = _resolve_class(interp, rest[0])
            supers = cls.superclasses if cls.superclasses else []
            # Qualify names
            result = []
            for s in supers:
                if s.startswith("::"):
                    result.append(s)
                elif f"::{s}" in oo.classes:
                    result.append(f"::{s}")
                else:
                    result.append(s)
            return TclResult(value=" ".join(_list_escape(s) for s in result))

        case "subclasses":
            if not rest:
                raise TclError('wrong # args: should be "info class subclasses className"')
            oo, cls = _resolve_class(interp, rest[0])
            import fnmatch

            pattern = rest[1] if len(rest) > 1 else "*"
            subs = []
            for cqn, c in oo.classes.items():
                if cls.qualified_name in c.superclasses or (
                    not cls.qualified_name.startswith("::") and cls.qualified_name in c.superclasses
                ):
                    subs.append(cqn)
                # Also check normalised forms
                for s in c.superclasses:
                    norm = s if s.startswith("::") else f"::{s}"
                    if norm == cls.qualified_name and cqn not in subs:
                        subs.append(cqn)
            matched = sorted(s for s in subs if fnmatch.fnmatch(s, pattern))
            return TclResult(value=" ".join(_list_escape(s) for s in matched))

        case "mixins":
            if not rest:
                raise TclError('wrong # args: should be "info class mixins className"')
            oo, cls = _resolve_class(interp, rest[0])
            return TclResult(value=" ".join(_list_escape(m) for m in cls.mixins))

        case "methods":
            if not rest:
                raise TclError('wrong # args: should be "info class methods className"')
            oo, cls = _resolve_class(interp, rest[0])
            import fnmatch

            pattern = "*"
            all_flag = False
            private_flag = False
            scope_filter: str | None = None
            i = 1
            while i < len(rest):
                if rest[i] == "-all":
                    all_flag = True
                    i += 1
                elif rest[i] == "-private":
                    private_flag = True
                    i += 1
                elif rest[i] == "-scope" and i + 1 < len(rest):
                    scope_filter = rest[i + 1]
                    i += 2
                else:
                    pattern = rest[i]
                    i += 1
            methods_dict: dict[str, str] = {}  # name -> visibility
            for mname, m in cls.methods.items():
                vis = getattr(m, "visibility", "public")
                # export/unexport override visibility
                if mname in cls.exported_methods:
                    vis = "public"
                elif mname in cls.unexported_methods:
                    vis = "unexported"
                methods_dict[mname] = vis
            if all_flag:
                for cqn in cls.mro(oo.classes):
                    ancestor = oo.classes.get(cqn)
                    if ancestor and ancestor is not cls:
                        for mname, m in ancestor.methods.items():
                            if mname not in methods_dict:
                                methods_dict[mname] = getattr(m, "visibility", "public")
                # Add built-in methods
                _BUILTIN_CLASS_METHODS = {
                    "destroy": "public",
                    "<cloned>": "private",
                    "eval": "private",
                    "unknown": "private",
                    "variable": "private",
                    "varname": "private",
                }
                # Check class-level unexport
                all_unexported: set[str] = set()
                for cqn in cls.mro(oo.classes):
                    ancestor = oo.classes.get(cqn)
                    if ancestor:
                        all_unexported |= ancestor.unexported_methods
                for bm, bv in _BUILTIN_CLASS_METHODS.items():
                    if bm not in methods_dict:
                        if bm in all_unexported:
                            methods_dict[bm] = "unexported"
                        else:
                            methods_dict[bm] = bv
            if scope_filter is not None:
                # -scope X: show only methods with that visibility
                methods = [k for k, v in methods_dict.items() if v == scope_filter]
            elif all_flag and private_flag:
                # -all -private: show everything
                methods = list(methods_dict.keys())
            elif private_flag:
                # -private without -all: show public + unexported (not private)
                methods = [k for k, v in methods_dict.items() if v in ("public", "exported", "unexported")]
            elif all_flag:
                # -all without -private: show public methods (inherited + built-in public)
                methods = [k for k, v in methods_dict.items() if v in ("public", "exported")]
            else:
                # Default: show only public/exported
                methods = [k for k, v in methods_dict.items() if v in ("public", "exported")]
            matched = sorted(m for m in methods if fnmatch.fnmatch(m, pattern))
            return TclResult(value=" ".join(_list_escape(m) for m in matched))

        case "instances":
            if not rest:
                raise TclError('wrong # args: should be "info class instances className"')
            oo, cls = _resolve_class(interp, rest[0])
            import fnmatch

            pattern = rest[1] if len(rest) > 1 else "*"
            instances = sorted(
                name
                for name, obj in oo.objects.items()
                if obj.class_name == cls.qualified_name and fnmatch.fnmatch(name, pattern)
            )
            return TclResult(value=" ".join(_list_escape(i) for i in instances))

        case "definition":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "info class definition className methodName"'
                )
            oo, cls = _resolve_class(interp, rest[0])
            method_name = rest[1]
            method = cls.methods.get(method_name)
            if method is None:
                raise TclError(
                    f'unknown method "{method_name}": must be {_format_or_list(sorted(cls.methods.keys()))}'
                )
            param_parts = [f"{{{n} {d}}}" if d is not None else _list_escape(n) for n, d in method.params]
            param_str = " ".join(param_parts)
            return TclResult(value=f"{{{param_str}}} {{{method.body}}}")

        case "constructor":
            if not rest:
                raise TclError('wrong # args: should be "info class constructor className"')
            oo, cls = _resolve_class(interp, rest[0])
            if cls.constructor is None:
                return TclResult(value="")
            ctor = cls.constructor
            param_str = " ".join(f"{{{n} {d}}}" if d is not None else n for n, d in ctor.params)
            return TclResult(value=f"{param_str} {{{ctor.body}}}")

        case "destructor":
            if not rest:
                raise TclError('wrong # args: should be "info class destructor className"')
            oo, cls = _resolve_class(interp, rest[0])
            if cls.destructor is None:
                return TclResult(value="")
            return TclResult(value=cls.destructor.body)

        case "variables":
            if not rest:
                raise TclError('wrong # args: should be "info class variables className"')
            oo, cls = _resolve_class(interp, rest[0])
            if "-private" in rest[1:]:
                pvars = getattr(cls, "private_variables", [])
                return TclResult(value=" ".join(_list_escape(v) for v in pvars))
            return TclResult(value=" ".join(_list_escape(v) for v in cls.variables))

        case "filters":
            if not rest:
                raise TclError('wrong # args: should be "info class filters className"')
            oo, cls = _resolve_class(interp, rest[0])
            return TclResult(value=" ".join(_list_escape(f) for f in cls.filters))

        case "methodtype":
            if len(rest) < 2:
                raise TclError(
                    'wrong # args: should be "info class methodtype className methodName"'
                )
            oo, cls = _resolve_class(interp, rest[0])
            method_name = rest[1]
            method = cls.methods.get(method_name)
            if method is None:
                raise TclError(f'unknown method "{method_name}"')
            if method.forward_target is not None:
                return TclResult(value="forward")
            return TclResult(value="method")

        case "forward":
            if len(rest) < 2:
                raise TclError('wrong # args: should be "info class forward className methodName"')
            oo, cls = _resolve_class(interp, rest[0])
            method_name = rest[1]
            method = cls.methods.get(method_name)
            if method is None or method.forward_target is None:
                raise TclError(f'unknown forward method "{method_name}"')
            return TclResult(value=" ".join(_list_escape(t) for t in method.forward_target))

        case "call":
            if len(rest) != 2:
                raise TclError('wrong # args: should be "info class call className methodName"')
            oo, cls = _resolve_class(interp, rest[0])
            method_name = rest[1]
            chain = oo.build_class_call_chain(cls.qualified_name, method_name)
            parts = []
            for call_type, mname, cname, impl_type in chain:
                parts.append(f"{{{call_type} {mname} {cname} {impl_type}}}")
            return TclResult(value=" ".join(parts))

        case "definitionnamespace":
            if not rest:
                raise TclError(
                    'wrong # args: should be "info class definitionnamespace className ?kind?"'
                )
            if len(rest) > 2:
                raise TclError(
                    'wrong # args: should be "info class definitionnamespace className ?kind?"'
                )
            oo, cls = _resolve_class(interp, rest[0])
            kind = rest[1] if len(rest) >= 2 else "-class"
            if kind == "-class":
                defns = getattr(cls, "definition_namespace", None) or ""
                return TclResult(value=defns)
            elif kind == "-instance":
                defns = getattr(cls, "instance_definition_namespace", None) or ""
                return TclResult(value=defns)
            else:
                raise TclError(
                    f'bad kind "{kind}": must be -class or -instance'
                )

        case _:
            return TclResult()


def _cmd_pid(interp: TclInterp, args: list[str]) -> TclResult:
    """pid"""
    return TclResult(value=str(os.getpid()))


def _cmd_pwd(interp: TclInterp, args: list[str]) -> TclResult:
    """pwd"""
    return TclResult(value=os.getcwd())


def _cmd_cd(interp: TclInterp, args: list[str]) -> TclResult:
    """cd ?dirName?"""
    directory = args[0] if args else os.path.expanduser("~")
    try:
        os.chdir(directory)
    except OSError as e:
        raise TclError(f'couldn\'t change working directory to "{directory}": {e}') from e
    return TclResult()


def register() -> None:
    """Register info commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("info", _cmd_info)
    REGISTRY.register_handler("pid", _cmd_pid)
    REGISTRY.register_handler("pwd", _cmd_pwd)
    REGISTRY.register_handler("cd", _cmd_cd)
