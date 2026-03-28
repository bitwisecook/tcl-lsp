"""TclOO commands: oo::class, oo::define, oo::objdefine, oo::object, oo::copy, self, my, next, nextto."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..oo import OORuntime, TclOOClass, TclOOMethod, TclOOObject
from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _parse_method_params(
    param_str: str,
) -> tuple[list[tuple[str, str | None]], list[str], bool]:
    """Parse a method parameter list into (params_with_defaults, param_names, has_args)."""
    from ..machine import _split_list

    raw_params = _split_list(param_str)
    params: list[tuple[str, str | None]] = []
    names: list[str] = []
    has_args = False
    for p in raw_params:
        parts = _split_list(p)
        if len(parts) == 1:
            pname = parts[0]
            if pname == "args":
                has_args = True
            names.append(pname)
            params.append((pname, None))
        elif len(parts) == 2:
            pname = parts[0]
            names.append(pname)
            params.append((pname, parts[1]))
        else:
            raise TclError(f'too many fields in argument specifier "{p}"')
    return params, names, has_args


_CLASS_BODY_SUBCMDS = (
    "classmethod",
    "constructor",
    "definitionnamespace",
    "deletemethod",
    "destructor",
    "export",
    "filter",
    "forward",
    "method",
    "mixin",
    "private",
    "renamemethod",
    "self",
    "superclass",
    "unexport",
    "variable",
)

_OBJDEFINE_SUBCMDS = (
    "class",
    "deletemethod",
    "export",
    "filter",
    "forward",
    "method",
    "mixin",
    "private",
    "renamemethod",
    "unexport",
    "variable",
)


def _abbreviate(word: str, options: tuple[str, ...]) -> str:
    """Resolve a possibly-abbreviated subcommand.

    Returns the full subcommand name if *word* is an unambiguous prefix
    of exactly one option.  Returns *word* unchanged otherwise (the
    caller's ``match/case`` will fall through to the default branch).
    """
    matches = [o for o in options if o.startswith(word)]
    if len(matches) == 1:
        return matches[0]
    return word


def _default_method_visibility(name: str) -> str:
    """Return the default visibility for a method name.

    In C Tcl, methods whose names start with a lowercase ASCII letter
    are exported (public) by default; all others are unexported.
    """
    if name and name[0].islower():
        return "public"
    return "unexported"


def _split_body_lines(body: str) -> list[str]:
    """Split a definition body into individual command lines.

    Handles semicolons as command separators (like Tcl), but only at the
    top level (not inside braces or quotes).
    """
    raw_lines = body.strip().split("\n")
    lines: list[str] = []
    for raw in raw_lines:
        depth = 0
        in_quote = False
        current: list[str] = []
        i_c = 0
        while i_c < len(raw):
            ch = raw[i_c]
            if ch == "\\" and not in_quote:
                current.append(ch)
                i_c += 1
                if i_c < len(raw):
                    current.append(raw[i_c])
                i_c += 1
                continue
            if ch == '"':
                in_quote = not in_quote
            elif not in_quote:
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                elif ch == ";" and depth == 0:
                    lines.append("".join(current))
                    current = []
                    i_c += 1
                    continue
            current.append(ch)
            i_c += 1
        lines.append("".join(current))
    return lines


def _define_method(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::method — define a method on the current class or object."""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if cls is None and obj is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    if len(args) < 3:
        raise TclError('wrong # args: should be "method name ?option? args body"')
    name = args[0]
    # Check for -export/-unexport/-private flag
    flag_vis = None
    if args[1].startswith("-") and len(args) >= 4:
        flag = args[1]
        if flag == "-export":
            flag_vis = "public"
        elif flag == "-unexport":
            flag_vis = "unexported"
        elif flag == "-private":
            flag_vis = "private"
        else:
            raise TclError(
                f'bad export flag "{flag}": must be -export, -private, or -unexport'
            )
        param_str, method_body = args[2], args[3]
    else:
        param_str, method_body = args[1], args[2]
    params, param_names, has_args = _parse_method_params(param_str)
    if flag_vis is not None:
        vis = flag_vis
    elif getattr(interp, "_oo_private_mode", False):
        vis = "private"
    else:
        vis = _default_method_visibility(name)
    m = TclOOMethod(
        name=name,
        params=params,
        param_names=param_names,
        body=method_body,
        has_args=has_args,
        visibility=vis,
    )
    if cls is not None:
        cls.exported_methods.discard(name)
        cls.unexported_methods.discard(name)
        cls.methods[name] = m
    elif obj is not None:
        obj.exported_methods.discard(name)
        obj.unexported_methods.discard(name)
        obj.instance_methods[name] = m
    return TclResult()


def _define_classmethod(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::classmethod"""
    cls = getattr(interp, "_defining_class", None)
    if cls is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    if len(args) < 3:
        raise TclError('wrong # args: should be "classmethod name args body"')
    name, param_str, method_body = args[0], args[1], args[2]
    params, param_names, has_args = _parse_method_params(param_str)
    cls.class_methods[name] = TclOOMethod(
        name=name,
        params=params,
        param_names=param_names,
        body=method_body,
        has_args=has_args,
    )
    return TclResult()


def _define_constructor(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::constructor"""
    cls = getattr(interp, "_defining_class", None)
    if cls is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    if len(args) < 2:
        raise TclError('wrong # args: should be "constructor args body"')
    param_str, ctor_body = args[0], args[1]
    params, param_names, has_args = _parse_method_params(param_str)
    cls.constructor = TclOOMethod(
        name="<constructor>",
        params=params,
        param_names=param_names,
        body=ctor_body,
        has_args=has_args,
    )
    return TclResult()


def _define_destructor(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::destructor"""
    cls = getattr(interp, "_defining_class", None)
    if cls is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    if len(args) < 1:
        raise TclError('wrong # args: should be "destructor body"')
    cls.destructor = TclOOMethod(
        name="<destructor>",
        params=[],
        param_names=[],
        body=args[0],
    )
    return TclResult()


def _define_superclass(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::superclass"""
    cls = getattr(interp, "_defining_class", None)
    if cls is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    if not args:
        # No args resets to default superclass
        # Metaclasses (subclasses of oo::class) default to ::oo::class
        # Regular classes default to ::oo::object
        # oo::object itself has no superclass
        oo = _get_oo_runtime(interp)
        if cls.qualified_name == "::oo::object":
            cls.superclasses = []
        elif oo._is_metaclass(cls):
            cls.superclasses = ["::oo::class"]
        else:
            cls.superclasses = ["::oo::object"]
    else:
        # Handle -append flag
        if args[0] == "-append":
            supers = list(args[1:])
            for s in supers:
                if s not in cls.superclasses:
                    cls.superclasses.append(s)
        else:
            cls.superclasses = list(args)
    return TclResult()


def _define_mixin(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::mixin"""
    cls = getattr(interp, "_defining_class", None)
    if cls is None:
        raise TclError("this command may only be called from within the body of an oo::define command")
    mixin_args = list(args)
    if mixin_args and mixin_args[0] == "-append":
        new_mixins = mixin_args[1:]
    else:
        new_mixins = list(mixin_args)
        cls.mixins = []
    oo = _get_oo_runtime(interp)
    for m in new_mixins:
        qn = m if m.startswith("::") else f"::{m}"
        if qn == cls.qualified_name:
            raise TclError("may not mix a class into itself")
        if m in cls.mixins or qn in cls.mixins:
            raise TclError("class should only be a direct mixin once")
        cls.mixins.append(m)
    return TclResult()


def _define_variable(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::variable"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if cls is not None:
        cls.variables.extend(args)
    elif obj is not None:
        obj.instance_variables.extend(args)
    return TclResult()


def _define_filter(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::filter"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if cls is not None:
        cls.filters.extend(args)
    elif obj is not None:
        obj.instance_filters.extend(args)
    return TclResult()


def _define_forward(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::forward"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if len(args) < 2:
        raise TclError('wrong # args: should be "forward name cmdName ?arg ...?"')
    fwd_name = args[0]
    fwd_target = args[1:]
    m = TclOOMethod(
        name=fwd_name,
        params=[("args", None)],
        param_names=["args"],
        body="",
        has_args=True,
        forward_target=fwd_target,
    )
    if cls is not None:
        cls.methods[fwd_name] = m
    elif obj is not None:
        obj.instance_methods[fwd_name] = m
    return TclResult()


def _define_deletemethod(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::deletemethod"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    for m in args:
        if cls is not None:
            cls.methods.pop(m, None)
        elif obj is not None:
            obj.instance_methods.pop(m, None)
    return TclResult()


def _define_renamemethod(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::renamemethod"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "renamemethod oldName newName"')
    old_name, new_name = args[0], args[1]
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if cls is not None:
        method = cls.methods.pop(old_name, None)
        if method is not None:
            method.name = new_name
            cls.methods[new_name] = method
    elif obj is not None:
        method = obj.instance_methods.pop(old_name, None)
        if method is not None:
            method.name = new_name
            obj.instance_methods[new_name] = method
    return TclResult()


def _define_export(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::export"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    for m in args:
        if cls is not None:
            cls.exported_methods.add(m)
            cls.unexported_methods.discard(m)
            method = cls.methods.get(m)
            if method:
                method.visibility = "public"
        elif obj is not None:
            obj.exported_methods.add(m)
            obj.unexported_methods.discard(m)
    return TclResult()


def _define_unexport(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::unexport"""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    for m in args:
        if cls is not None:
            cls.unexported_methods.add(m)
            cls.exported_methods.discard(m)
            method = cls.methods.get(m)
            if method:
                method.visibility = "unexported"
        elif obj is not None:
            obj.unexported_methods.add(m)
            obj.exported_methods.discard(m)
    return TclResult()


def _define_private(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::private — TIP 500 private method/variable."""
    cls = getattr(interp, "_defining_class", None)
    obj = getattr(interp, "_defining_object", None)
    if cls is None and obj is None:
        raise TclError(
            "this command may only be called from within the context of"
            " an ::oo::define or ::oo::objdefine command"
        )
    if not args:
        raise TclError('wrong # args: should be "private cmd ?arg ...?"')
    subcmd = args[0]
    if subcmd == "method":
        cls = getattr(interp, "_defining_class", None)
        obj = getattr(interp, "_defining_object", None)
        if len(args) < 4:
            raise TclError('wrong # args: should be "private method name args body"')
        name, param_str, method_body = args[1], args[2], args[3]
        params, param_names, has_args = _parse_method_params(param_str)
        m = TclOOMethod(
            name=name,
            params=params,
            param_names=param_names,
            body=method_body,
            has_args=has_args,
            visibility="private",
        )
        if cls is not None:
            cls.methods[name] = m
        elif obj is not None:
            obj.instance_methods[name] = m
    elif subcmd == "variable":
        cls = getattr(interp, "_defining_class", None)
        obj = getattr(interp, "_defining_object", None)
        if cls is not None:
            cls.variables.extend(args[1:])
        elif obj is not None:
            obj.instance_variables.extend(args[1:])
    elif subcmd == "forward":
        return _define_forward(interp, args[1:])
    elif len(args) == 1:
        # Single braced block: evaluate as definition script
        # Set private mode so methods defined inside are private
        saved_private = getattr(interp, "_oo_private_mode", False)
        interp._oo_private_mode = True
        try:
            interp.eval(args[0])
        finally:
            interp._oo_private_mode = saved_private
    else:
        # Multiple args: evaluate as Tcl command in current context
        saved_private = getattr(interp, "_oo_private_mode", False)
        interp._oo_private_mode = True
        try:
            from ..machine import _list_escape
            cmd = " ".join(_list_escape(a) for a in args)
            interp.eval(cmd)
        finally:
            interp._oo_private_mode = saved_private
    return TclResult()


def _define_self(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::self — apply definitions to the class object itself."""
    cls = getattr(interp, "_defining_class", None)
    defining_frame = getattr(interp, "_defining_frame", None)
    if cls is None or (defining_frame is not None and interp.current_frame is not defining_frame):
        raise TclError(
            "this command may only be called from within the context of "
            "an ::oo::define or ::oo::objdefine command"
        )
    oo = _get_oo_runtime(interp)
    obj = oo.objects.get(cls.qualified_name)
    if obj is None:
        return TclResult()
    if not args:
        return TclResult(value=cls.qualified_name)
    if len(args) == 1:
        _parse_objdefine_body(interp, obj, args[0])
    else:
        from ..machine import _list_escape

        reconstructed = " ".join(_list_escape(a) for a in args)
        _parse_objdefine_body(interp, obj, reconstructed)
    return TclResult()


def _define_definitionnamespace(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::define::definitionnamespace — not yet implemented."""
    return TclResult()


def _define_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """Unknown handler for ::oo::define namespace — supports abbreviation."""
    if not args:
        raise TclError("wrong # args")
    cmd_name = args[0]
    # Try abbreviation
    full = _abbreviate(cmd_name, _CLASS_BODY_SUBCMDS)
    if full != cmd_name:
        fq = f"::oo::define::{full}"
        handler = interp._runtime_commands.get(fq)
        if handler is not None:
            return handler(interp, args[1:])
    raise TclError(f'invalid command name "{cmd_name}"')


def _objdefine_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """Unknown handler for ::oo::objdefine namespace — supports abbreviation."""
    if not args:
        raise TclError("wrong # args")
    cmd_name = args[0]
    full = _abbreviate(cmd_name, _OBJDEFINE_SUBCMDS)
    if full != cmd_name:
        fq = f"::oo::objdefine::{full}"
        handler = interp._runtime_commands.get(fq)
        if handler is not None:
            return handler(interp, args[1:])
    raise TclError(f'invalid command name "{cmd_name}"')


def _ensure_define_commands(interp: TclInterp) -> None:
    """Register ::oo::define::* commands if not already present."""
    if getattr(interp, "_oo_define_commands_registered", False):
        return
    interp._oo_define_commands_registered = True

    cmds = {
        "::oo::define::method": _define_method,
        "::oo::define::classmethod": _define_classmethod,
        "::oo::define::constructor": _define_constructor,
        "::oo::define::destructor": _define_destructor,
        "::oo::define::superclass": _define_superclass,
        "::oo::define::mixin": _define_mixin,
        "::oo::define::variable": _define_variable,
        "::oo::define::filter": _define_filter,
        "::oo::define::forward": _define_forward,
        "::oo::define::deletemethod": _define_deletemethod,
        "::oo::define::renamemethod": _define_renamemethod,
        "::oo::define::export": _define_export,
        "::oo::define::unexport": _define_unexport,
        "::oo::define::private": _define_private,
        "::oo::define::self": _define_self,
        "::oo::define::definitionnamespace": _define_definitionnamespace,
    }
    for name, handler in cmds.items():
        interp._runtime_commands[name] = handler

    # Register unknown handler for abbreviation support
    from ..scope import ensure_namespace

    interp._runtime_commands["::oo::define::__unknown__"] = _define_unknown
    oo_define_ns = ensure_namespace(interp.root_namespace, "::oo::define")
    oo_define_ns._unknown_handler = "::oo::define::__unknown__"


def _ensure_objdefine_commands(interp: TclInterp) -> None:
    """Register ::oo::objdefine::* commands if not already present."""
    if getattr(interp, "_oo_objdefine_commands_registered", False):
        return
    interp._oo_objdefine_commands_registered = True

    # objdefine shares many commands with define, but uses the object context
    cmds = {
        "::oo::objdefine::method": _define_method,
        "::oo::objdefine::variable": _define_variable,
        "::oo::objdefine::filter": _define_filter,
        "::oo::objdefine::forward": _define_forward,
        "::oo::objdefine::deletemethod": _define_deletemethod,
        "::oo::objdefine::renamemethod": _define_renamemethod,
        "::oo::objdefine::export": _define_export,
        "::oo::objdefine::unexport": _define_unexport,
        "::oo::objdefine::mixin": _objdefine_mixin,
        "::oo::objdefine::private": _define_private,
        "::oo::objdefine::class": _objdefine_class,
    }
    for name, handler in cmds.items():
        interp._runtime_commands[name] = handler

    # Register unknown handler for abbreviation
    from ..scope import ensure_namespace

    interp._runtime_commands["::oo::objdefine::__unknown__"] = _objdefine_unknown
    objdefine_ns = ensure_namespace(interp.root_namespace, "::oo::objdefine")
    objdefine_ns._unknown_handler = "::oo::objdefine::__unknown__"


def _parse_class_body(
    interp: TclInterp,
    cls: TclOOClass,
    body: str,
) -> TclResult:
    """Parse a class definition body by evaluating it in the ::oo::define namespace.

    In C Tcl, the body of ``oo::define`` is a Tcl script executed in
    the ``::oo::define`` namespace where definition commands (method,
    constructor, etc.) are registered as real commands.  We replicate
    this by registering transient handlers then running
    ``interp.eval(body)`` in that namespace.
    """
    from ..scope import ensure_namespace

    _ensure_define_commands(interp)

    oo_define_ns = ensure_namespace(interp.root_namespace, "::oo::define")
    saved_ns = interp.current_namespace
    saved_frame_ns = interp.current_frame.namespace
    saved_cls = getattr(interp, "_defining_class", None)
    saved_obj = getattr(interp, "_defining_object", None)
    saved_define_frame = getattr(interp, "_defining_frame", None)
    interp._defining_class = cls
    interp._defining_object = None
    interp._defining_frame = interp.current_frame
    interp.current_namespace = oo_define_ns
    interp.current_frame.namespace = oo_define_ns
    try:
        result = interp.eval(body)
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame.namespace = saved_frame_ns
        interp._defining_class = saved_cls
        interp._defining_object = saved_obj
        interp._defining_frame = saved_define_frame
    return result


def _objdefine_mixin(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::objdefine::mixin"""
    obj = getattr(interp, "_defining_object", None)
    if obj is None:
        raise TclError("this command may only be called from within the body of an oo::objdefine command")
    mixin_args = list(args)
    if mixin_args and mixin_args[0] == "-append":
        new_mixins = mixin_args[1:]
    else:
        new_mixins = list(mixin_args)
        obj.instance_mixins = []
    for m in new_mixins:
        qn = m if m.startswith("::") else f"::{m}"
        for existing in obj.instance_mixins:
            existing_qn = existing if existing.startswith("::") else f"::{existing}"
            if qn == existing_qn:
                raise TclError("class should only be a direct mixin once")
        obj.instance_mixins.append(m)
    return TclResult()


def _objdefine_class(interp: TclInterp, args: list[str]) -> TclResult:
    """::oo::objdefine::class"""
    obj = getattr(interp, "_defining_object", None)
    if obj is None:
        raise TclError("this command may only be called from within the body of an oo::objdefine command")
    if len(args) < 1:
        raise TclError('wrong # args: should be "class className"')
    new_class = args[0]
    oo = _get_oo_runtime(interp)
    cls = oo.classes.get(new_class)
    if cls is None:
        qn = f"::{new_class}" if not new_class.startswith("::") else new_class
        cls = oo.classes.get(qn)
    if cls is None:
        raise TclError(f'"{new_class}" is not a class')
    if cls.qualified_name == obj.name:
        raise TclError("may not change classes into an instance of themselves")
    obj.class_name = cls.qualified_name
    if oo._is_metaclass(cls) and obj.name not in oo.classes:
        new_cls_obj = TclOOClass(
            name=obj.name.rsplit("::", 1)[-1],
            qualified_name=obj.name,
        )
        oo.register_class(new_cls_obj)
        _register_class_command(interp, oo, obj.name, new_cls_obj)
    oo.invalidate_all_mro()
    return TclResult()


def _parse_objdefine_body(
    interp: TclInterp,
    obj: TclOOObject,
    body: str,
) -> TclResult:
    """Parse an oo::objdefine body by evaluating it in the ::oo::objdefine namespace."""
    from ..scope import ensure_namespace

    _ensure_define_commands(interp)
    _ensure_objdefine_commands(interp)

    # Register objdefine-only commands
    if "::oo::objdefine::class" not in interp._runtime_commands:
        interp._runtime_commands["::oo::objdefine::class"] = _objdefine_class
        interp._runtime_commands["::oo::objdefine::mixin"] = _objdefine_mixin

    objdefine_ns = ensure_namespace(interp.root_namespace, "::oo::objdefine")
    saved_ns = interp.current_namespace
    saved_frame_ns = interp.current_frame.namespace
    saved_cls = getattr(interp, "_defining_class", None)
    saved_obj = getattr(interp, "_defining_object", None)
    saved_define_frame = getattr(interp, "_defining_frame", None)
    interp._defining_class = None
    interp._defining_object = obj
    interp._defining_frame = interp.current_frame
    interp.current_namespace = objdefine_ns
    interp.current_frame.namespace = objdefine_ns
    try:
        result = interp.eval(body)
    finally:
        interp.current_namespace = saved_ns
        interp.current_frame.namespace = saved_frame_ns
        interp._defining_class = saved_cls
        interp._defining_object = saved_obj
        interp._defining_frame = saved_define_frame
    return result


def _get_oo_runtime(interp: TclInterp) -> OORuntime:
    """Get or create the OO runtime on the interpreter."""
    if not hasattr(interp, "_oo_runtime"):
        interp._oo_runtime = OORuntime()
        # Set ::oo:: namespace variables that Tcl test files expect.
        from ..scope import ensure_namespace

        oo_ns = ensure_namespace(interp.root_namespace, "::oo")
        oo_frame = oo_ns.get_frame(interp)
        oo_frame.set_var("patchlevel", "1.3.1")
        oo_frame.set_var("version", "1.3.1")
        # Register tcl::oo as a provided package
        interp.eval('package provide tcl::oo 1.3.1')
        # Eagerly register define/objdefine subcommands so they are
        # accessible by FQ name (e.g. oo::define::private) from any context.
        _ensure_define_commands(interp)
        _ensure_objdefine_commands(interp)
    return interp._oo_runtime


def _resolve_class(interp: TclInterp, oo: OORuntime, name: str) -> TclOOClass:
    """Resolve a class name, checking current namespace if not qualified."""
    cls = oo.classes.get(name)
    if cls is None and not name.startswith("::"):
        cls = oo.classes.get(f"::{name}")
    if cls is None and not name.startswith("::"):
        ns = interp.current_namespace.qualname
        ns_qualified = f"{ns}::{name}" if ns != "::" else f"::{name}"
        cls = oo.classes.get(ns_qualified)
    if cls is None:
        raise TclError(f'unknown class "{name}"')
    return cls


def _cmd_oo_class(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::class create className ?body?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "oo::class create name ?definition?"')
    subcmd = args[0]
    if subcmd != "create":
        raise TclError(f'unknown method "{subcmd}": must be create')

    name = args[1]
    # Qualify the name
    if not name.startswith("::"):
        ns = interp.current_namespace.qualname
        if ns == "::":
            qualified = f"::{name}"
        else:
            qualified = f"{ns}::{name}"
    else:
        qualified = name
        name = qualified.rsplit("::", 1)[-1]

    oo = _get_oo_runtime(interp)
    cls = TclOOClass(name=name, qualified_name=qualified)

    # Default superclass if none specified — set before registration so that
    # register_class sees the correct hierarchy from the start.
    # _parse_class_body may override this if a superclass is specified.
    if qualified != "::oo::object":
        cls.superclasses = ["::oo::object"]

    # Register early so that `self { ... }` blocks in the class body
    # can find the class object via oo.objects.
    oo.register_class(cls)

    if len(args) >= 3:
        try:
            _parse_class_body(interp, cls, args[2])
        except TclError as e:
            # Add definition script context to errorInfo
            from ..machine import _list_escape

            full_cmd = f"oo::class create {name} {_list_escape(args[2])}"
            ctx = f'    (in definition script for class "{qualified}" line 1)'
            inv = f'    invoked from within\n"{full_cmd}"'
            info = list(e.error_info) if e.error_info else [e.message]
            info.append(ctx)
            info.append(inv)
            raise TclError(e.message, error_info=info) from None

    _register_class_command(interp, oo, qualified, cls)

    return TclResult(value=qualified)


def _register_class_command(
    interp: TclInterp, oo: OORuntime, qualified: str, cls: TclOOClass
) -> None:
    """Register the class command for `ClassName new` / `ClassName create`."""
    name = cls.name

    def _class_cmd(interp: TclInterp, cmd_args: list[str]) -> TclResult:
        from ..oo import _format_method_list

        if not cmd_args:
            raise TclError(f'wrong # args: should be "{qualified} method ?arg ...?"')
        method = cmd_args[0]

        # Check if method is unexported on the class object
        obj = oo.objects.get(qualified)
        is_unexported = obj is not None and method in obj.unexported_methods

        if method == "new" and not is_unexported:
            obj_name = oo.create_object(interp, qualified, args=cmd_args[1:])
            return TclResult(value=obj_name)
        if method == "create" and not is_unexported:
            if len(cmd_args) < 2:
                raise TclError(f'wrong # args: should be "{qualified} create name ?arg ...?"')
            create_name = cmd_args[1]
            if not create_name:
                raise TclError("object name must not be empty")
            if not create_name.startswith("::"):
                ns = interp.current_namespace.qualname
                if ns == "::":
                    create_name = f"::{create_name}"
                else:
                    create_name = f"{ns}::{create_name}"
            # Normalize repeated :: separators (e.g. ::::foo → ::foo)
            while "::::" in create_name:
                create_name = create_name.replace("::::", "::")
            obj_name = oo.create_object(
                interp, qualified, obj_name=create_name, args=cmd_args[2:],
                display_name=cmd_args[1],
            )
            return TclResult(value=obj_name)
        if method == "destroy":
            # Destroy this class (and cascade to instances)
            if obj is not None:
                return oo._destroy_object(interp, obj)
            return TclResult()
        # Dispatch to the class's OO method dispatch (class objects have methods too)
        if obj is not None and not is_unexported:
            m, defining_class = oo.resolve_method(obj, method)
            if m is not None:
                return oo._invoke_method(
                    interp, obj, m, cmd_args[1:], defining_class=defining_class
                )
        # Build dynamic available method list for error message
        available: list[str] = ["destroy"]
        if obj is not None:
            # Add create/new if not unexported
            for builtin in ("create", "new"):
                if builtin not in obj.unexported_methods:
                    available.append(builtin)
            # Add instance methods (from self block / objdefine)
            for m_name, m_def in obj.instance_methods.items():
                if m_name in obj.exported_methods:
                    available.append(m_name)
                elif m_name in obj.unexported_methods:
                    continue
                elif m_def.visibility == "public":
                    available.append(m_name)
            # Add methods from MRO
            for class_qname in oo._effective_mro(obj):
                ancestor = oo.classes.get(class_qname)
                if ancestor:
                    for m_name, m_def in ancestor.methods.items():
                        if m_name in obj.exported_methods:
                            available.append(m_name)
                        elif m_name in obj.unexported_methods:
                            continue
                        elif m_def.visibility == "public":
                            available.append(m_name)
        else:
            available.extend(["create", "new"])
        available = sorted(set(available))
        raise TclError(
            f'unknown method "{method}": must be '
            + _format_method_list(available)
        )

    interp._runtime_commands[qualified] = _class_cmd
    # Also register short name
    if name != qualified:
        interp._runtime_commands[name] = _class_cmd


def _cmd_oo_define(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::define className body  OR  oo::define className subcommand ?arg ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "oo::define className defScript"')

    class_name = args[0]
    oo = _get_oo_runtime(interp)
    cls = _resolve_class(interp, oo, class_name)

    body_result = TclResult()
    if len(args) == 2:
        # Body form: oo::define Dog { method bark {} { ... } }
        # Special case: "self" alone returns the class name (TIP #470)
        if args[1].strip() == "self":
            return TclResult(value=cls.qualified_name)
        try:
            body_result = _parse_class_body(interp, cls, args[1])
        except TclError as e:
            # Add definition script context frame to errorInfo
            from ..machine import _list_escape

            full_cmd = f"oo::define {class_name} {_list_escape(args[1])}"
            ctx = f'    (in definition script for class "{cls.qualified_name}" line 1)'
            inv = f'    invoked from within\n"{full_cmd}"'
            info = list(e.error_info) if e.error_info else [e.message]
            info.append(ctx)
            info.append(inv)
            raise TclError(e.message, error_info=info) from None
    else:
        # Single-subcommand form: oo::define Dog superclass Animal
        # Special case: "self" alone returns the class name (TIP #470)
        if args[1] == "self" and len(args) == 2:
            return TclResult(value=cls.qualified_name)
        from ..machine import _list_escape

        reconstructed = " ".join(_list_escape(a) for a in args[1:])
        try:
            body_result = _parse_class_body(interp, cls, reconstructed)
        except TclError as e:
            # Rewrite "wrong # args" to include oo::define prefix
            msg = e.message
            if msg.startswith("wrong # args: should be \""):
                inner = msg[len("wrong # args: should be \""):-1]
                msg = f'wrong # args: should be "oo::define {class_name} {inner}"'
            # Build proper error_info with full command text
            full_cmd = "oo::define " + " ".join(args)
            error_info = [msg, f'    while executing\n"{full_cmd}"']
            raise TclError(msg, error_info=error_info) from None

    # Invalidate ALL classes — adding a mixin/superclass to one class
    # can change the MRO of any subclass
    oo.invalidate_all_mro()
    return body_result


def _cmd_oo_objdefine(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::objdefine objectName body  OR  oo::objdefine objectName subcommand ?arg ...?"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "oo::objdefine objectName defScript"')

    obj_name = args[0]
    oo = _get_oo_runtime(interp)

    # Resolve object name
    obj = oo.objects.get(obj_name)
    if obj is None:
        qualified = f"::{obj_name}" if not obj_name.startswith("::") else obj_name
        obj = oo.objects.get(qualified)
    if obj is None:
        raise TclError(f'"{obj_name}" does not refer to an object')

    if len(args) == 2:
        body_result = _parse_objdefine_body(interp, obj, args[1])
    else:
        # Single-subcommand form: oo::objdefine obj method foo {} {body}
        # Reconstruct with braces to preserve structure
        from ..machine import _list_escape

        reconstructed = " ".join(_list_escape(a) for a in args[1:])
        try:
            body_result = _parse_objdefine_body(interp, obj, reconstructed)
        except TclError as e:
            msg = e.message
            if msg.startswith("wrong # args: should be \""):
                inner = msg[len("wrong # args: should be \""):-1]
                msg = f'wrong # args: should be "oo::objdefine {obj_name} {inner}"'
            full_cmd = "oo::objdefine " + " ".join(args)
            error_info = [msg, f'    while executing\n"{full_cmd}"']
            raise TclError(msg, error_info=error_info) from None

    # Re-register the object command so new methods are visible
    # But skip if the object is now a class (class command was already registered)
    if obj.name in oo.classes:
        _register_class_command(interp, oo, obj.name, oo.classes[obj.name])
    else:
        oo._register_object_command(interp, obj)
    return body_result


def _cmd_oo_object(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::object create/new — create plain objects (not from a user class)."""
    if not args:
        raise TclError('wrong # args: should be "oo::object method ?arg ...?"')

    oo = _get_oo_runtime(interp)
    subcmd = args[0]

    if subcmd == "new":
        obj_name = oo.create_object(interp, "::oo::object", args=args[1:])
        return TclResult(value=obj_name)
    elif subcmd == "create":
        if len(args) < 2:
            raise TclError('wrong # args: should be "oo::object create name ?arg ...?"')
        name = args[1]
        if not name:
            raise TclError("object name must not be empty")
        if not name.startswith("::"):
            ns = interp.current_namespace.qualname
            if ns == "::":
                name = f"::{name}"
            else:
                name = f"{ns}::{name}"
        obj_name = oo.create_object(interp, "::oo::object", obj_name=name, args=args[2:], display_name=args[1])
        return TclResult(value=obj_name)
    elif subcmd == "destroy":
        # oo::object itself cannot be destroyed in normal usage
        raise TclError("cannot destroy the root object")
    else:
        # Fall through to OO dispatch for methods defined on oo::object
        obj = oo.objects.get("::oo::object")
        if obj is not None:
            m, defining_class = oo.resolve_method(obj, subcmd)
            if m is not None:
                return oo._invoke_method(interp, obj, m, args[1:], defining_class=defining_class)
        raise TclError(f'unknown method "{subcmd}": must be create, destroy or new')


def _cmd_oo_copy(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::copy sourceName ?targetName? ?targetNamespace?"""
    if not args:
        raise TclError(
            'wrong # args: should be "oo::copy sourceName ?targetName? ?targetNamespace?"'
        )

    oo = _get_oo_runtime(interp)
    src_name = args[0]

    # Resolve source object
    src = oo.objects.get(src_name)
    if src is None:
        qualified = f"::{src_name}" if not src_name.startswith("::") else src_name
        src = oo.objects.get(qualified)
    if src is None:
        raise TclError(f'"{src_name}" does not refer to an object')

    # Determine target name and optional target namespace
    oo._next_obj_id += 1
    target_namespace = args[2] if len(args) >= 3 else None
    if len(args) >= 2:
        tgt_name = args[1]
        if not tgt_name.startswith("::"):
            ns = interp.current_namespace.qualname
            if ns == "::":
                tgt_name = f"::{tgt_name}"
            else:
                tgt_name = f"{ns}::{tgt_name}"
    else:
        tgt_name = f"::oo::Obj{oo._next_obj_id}"

    import copy

    from ..oo import TclOOClass, TclOOObject
    from ..scope import ensure_namespace

    # If source is a class, create a new class copy
    src_cls = oo.classes.get(src.name)
    if src_cls is not None:
        new_cls = TclOOClass(
            name=tgt_name.rsplit("::", 1)[-1],
            qualified_name=tgt_name,
            superclasses=list(src_cls.superclasses),
            methods={k: copy.copy(v) for k, v in src_cls.methods.items()},
            class_methods={k: copy.copy(v) for k, v in src_cls.class_methods.items()},
            constructor=copy.copy(src_cls.constructor) if src_cls.constructor else None,
            destructor=copy.copy(src_cls.destructor) if src_cls.destructor else None,
            mixins=list(src_cls.mixins),
            filters=list(src_cls.filters),
            variables=list(src_cls.variables),
        )
        oo.register_class(new_cls)
        # Register the class command
        _register_class_command(interp, oo, tgt_name, new_cls)
        # Copy instance methods from the source object to the target
        tgt = oo.objects.get(tgt_name)
        if tgt is not None:
            tgt.instance_methods = {k: copy.copy(v) for k, v in src.instance_methods.items()}
            tgt.instance_mixins = list(src.instance_mixins)
            tgt.instance_filters = list(src.instance_filters)
            tgt.instance_variables = list(src.instance_variables)
            tgt.exported_methods = set(src.exported_methods)
            tgt.unexported_methods = set(src.unexported_methods)
            tgt._vars = dict(src._vars)
            tgt._arrays = {k: dict(v) for k, v in src._arrays.items()}
            if target_namespace:
                tgt.namespace = target_namespace
                ensure_namespace(interp.root_namespace, target_namespace)
        return TclResult(value=tgt_name)

    tgt_ns = target_namespace or f"::oo::Obj{oo._next_obj_id}"
    if target_namespace:
        ensure_namespace(interp.root_namespace, target_namespace)
    tgt = TclOOObject(
        name=tgt_name,
        class_name=src.class_name,
        namespace=tgt_ns,
        instance_methods={k: copy.copy(v) for k, v in src.instance_methods.items()},
        instance_mixins=list(src.instance_mixins),
        instance_filters=list(src.instance_filters),
        instance_variables=list(src.instance_variables),
        exported_methods=set(src.exported_methods),
        unexported_methods=set(src.unexported_methods),
        _vars=dict(src._vars),
        _arrays={k: dict(v) for k, v in src._arrays.items()},
    )
    oo.objects[tgt_name] = tgt
    oo._register_object_command(interp, tgt)
    return TclResult(value=tgt_name)


def _cmd_self(interp: TclInterp, args: list[str]) -> TclResult:
    """self ?subcommand?"""
    oo = _get_oo_runtime(interp)

    # TIP #470: inside oo::define / oo::objdefine (not in a method),
    # ``self`` takes no subcommands — it just returns the class/object name.
    # Only works when called directly in the define body frame (not from
    # a nested proc), matching C Tcl behaviour.
    frame = interp.current_frame
    in_method = getattr(frame, "_oo_self", None) is not None
    if not in_method:
        defining_cls = getattr(interp, "_defining_class", None)
        defining_obj = getattr(interp, "_defining_object", None)
        defining_frame = getattr(interp, "_defining_frame", None)
        in_define = (defining_cls is not None or defining_obj is not None)
        if in_define and frame is defining_frame:
            if args:
                raise TclError('wrong # args: should be "self"')
            return TclResult(value=oo.self_name(interp))
        if in_define and frame is not defining_frame:
            # Inside define context but in a nested proc — self is not available
            raise TclError(
                "this command may only be called from within the context of "
                "an ::oo::define or ::oo::objdefine command"
            )

    if not args:
        return TclResult(value=oo.self_name(interp))
    subcmd = args[0]
    if subcmd in ("object", "self"):
        return TclResult(value=oo.self_name(interp))
    if subcmd == "class":
        frame = interp.current_frame
        class_name = getattr(frame, "_oo_class", None)
        if class_name is None:
            raise TclError('"self class" may only be invoked from within a method')
        if class_name.startswith("__instance__"):
            raise TclError("method not defined by a class")
        return TclResult(value=class_name)
    if subcmd == "method":
        frame = interp.current_frame
        method_name = getattr(frame, "_oo_method", None)
        if method_name is None:
            raise TclError('"self method" may only be invoked from within a method')
        return TclResult(value=method_name)
    if subcmd == "namespace":
        obj_name = oo.self_name(interp)
        obj = oo.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')
        return TclResult(value=obj.namespace)
    if subcmd == "caller":
        # Return {definingClass methodName} of the calling method
        frame = interp.current_frame
        parent = getattr(frame, "parent", None)
        if parent is None:
            raise TclError("caller is not an object")
        caller_class = getattr(parent, "_oo_class", None)
        caller_method = getattr(parent, "_oo_method", None)
        if caller_class is None or caller_method is None:
            raise TclError("caller is not an object")
        return TclResult(value=f"{caller_class} {caller_method}")
    if subcmd == "filter":
        # Return the current filter name if in a filter
        frame = interp.current_frame
        chain = getattr(frame, "_oo_filter_chain", None)
        if chain is None:
            raise TclError("not inside a filter")
        idx = getattr(frame, "_oo_filter_index", 0)
        if idx < len(chain):
            return TclResult(value=chain[idx][0].name)
        raise TclError("not inside a filter")
    if subcmd == "target":
        # Return the target method and defining class when inside a filter
        frame = interp.current_frame
        target = getattr(frame, "_oo_filter_target", None)
        if target is None:
            raise TclError("not inside a filter")
        target_method, target_class = target
        class_name = target_class if target_class else ""
        method_name = getattr(frame, "_oo_filter_method_name", "")
        return TclResult(value=f"{class_name} {method_name}")
    if subcmd == "call":
        # Return {callChain currentIndex} — the full call chain for the
        # current method and the position of this method within it.
        frame = interp.current_frame
        obj_name = getattr(frame, "_oo_self", None)
        method_name = getattr(frame, "_oo_method", None)
        if obj_name is None or method_name is None:
            raise TclError('"self call" may only be invoked from within a method')
        obj = oo.objects.get(obj_name)
        if obj is None:
            raise TclError(f'object "{obj_name}" has been destroyed')
        chain = oo.build_object_call_chain(obj, method_name)
        # Format the chain
        from vm.machine import _list_escape
        chain_parts = []
        for call_type, mname, cname, impl_type in chain:
            chain_parts.append(f"{{{call_type} {mname} {cname} {impl_type}}}")
        chain_str = " ".join(chain_parts)
        # Find current position in chain
        current_class = getattr(frame, "_oo_class", None)
        position = 0
        for i, (call_type, mname, cname, impl_type) in enumerate(chain):
            if cname == current_class:
                position = i
                break
            elif cname == "object" and current_class is None:
                position = i
                break
        return TclResult(value=f"{{{chain_str}}} {position}")
    raise TclError(
        f'unknown method "{subcmd}": must be call, caller, class, filter, method, namespace, object, or target'
    )


def _cmd_my(interp: TclInterp, args: list[str]) -> TclResult:
    """my method ?arg ...?"""
    oo = _get_oo_runtime(interp)
    return oo.my_dispatch(interp, args)


def _cmd_next(interp: TclInterp, args: list[str]) -> TclResult:
    """next ?arg ...?"""
    oo = _get_oo_runtime(interp)
    return oo.next_dispatch(interp, args)


def _cmd_nextto(interp: TclInterp, args: list[str]) -> TclResult:
    """nextto className ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "nextto class ?arg...?"')
    oo = _get_oo_runtime(interp)
    return oo.nextto_dispatch(interp, args[0], args[1:])


# Metaclass variants
def _cmd_oo_configurable(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::configurable create className ?body?"""
    return _cmd_oo_class(interp, args)


def _cmd_oo_abstract(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::abstract create className ?body?"""
    return _cmd_oo_class(interp, args)


def _cmd_oo_singleton(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::singleton create className ?body?"""
    return _cmd_oo_class(interp, args)


def register() -> None:
    """Register OO commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("oo::class", _cmd_oo_class)
    REGISTRY.register_handler("oo::define", _cmd_oo_define)
    REGISTRY.register_handler("oo::objdefine", _cmd_oo_objdefine)
    REGISTRY.register_handler("oo::object", _cmd_oo_object)
    REGISTRY.register_handler("oo::copy", _cmd_oo_copy)
    REGISTRY.register_handler("oo::configurable", _cmd_oo_configurable)
    REGISTRY.register_handler("oo::abstract", _cmd_oo_abstract)
    REGISTRY.register_handler("oo::singleton", _cmd_oo_singleton)
    REGISTRY.register_handler("self", _cmd_self)
    REGISTRY.register_handler("my", _cmd_my)
    REGISTRY.register_handler("next", _cmd_next)
    REGISTRY.register_handler("nextto", _cmd_nextto)
