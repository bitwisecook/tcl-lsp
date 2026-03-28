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


def _parse_class_body(
    interp: TclInterp,
    cls: TclOOClass,
    body: str,
) -> None:
    """Parse a class definition body and populate the class."""
    from ..machine import _split_list

    lines = _split_body_lines(body)

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if not line or line.startswith("#"):
            i += 1
            continue

        # Collect multi-line commands (brace balancing)
        while line.count("{") > line.count("}") and i + 1 < len(lines):
            i += 1
            line += "\n" + lines[i]

        parts = _split_list(line)
        if not parts:
            i += 1
            continue

        subcmd = _abbreviate(parts[0], _CLASS_BODY_SUBCMDS)
        match subcmd:
            case "method":
                if len(parts) < 4:
                    raise TclError('wrong # args: should be "method name args body"')
                name = parts[1]
                # Check for -export/-unexport/-private flag
                flag_vis = None
                if parts[2].startswith("-") and len(parts) >= 5:
                    flag = parts[2]
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
                    param_str, method_body = parts[3], parts[4]
                else:
                    param_str, method_body = parts[2], parts[3]
                params, param_names, has_args = _parse_method_params(param_str)
                vis = flag_vis if flag_vis is not None else _default_method_visibility(name)
                # Defining a method clears class-level export/unexport for it
                cls.exported_methods.discard(name)
                cls.unexported_methods.discard(name)
                cls.methods[name] = TclOOMethod(
                    name=name,
                    params=params,
                    param_names=param_names,
                    body=method_body,
                    has_args=has_args,
                    visibility=vis,
                )
            case "classmethod":
                if len(parts) < 4:
                    raise TclError('wrong # args: should be "classmethod name args body"')
                name, param_str, method_body = parts[1], parts[2], parts[3]
                params, param_names, has_args = _parse_method_params(param_str)
                cls.class_methods[name] = TclOOMethod(
                    name=name,
                    params=params,
                    param_names=param_names,
                    body=method_body,
                    has_args=has_args,
                )
            case "constructor":
                if len(parts) < 3:
                    raise TclError('wrong # args: should be "constructor args body"')
                param_str, ctor_body = parts[1], parts[2]
                params, param_names, has_args = _parse_method_params(param_str)
                cls.constructor = TclOOMethod(
                    name="<constructor>",
                    params=params,
                    param_names=param_names,
                    body=ctor_body,
                    has_args=has_args,
                )
            case "destructor":
                if len(parts) < 2:
                    raise TclError('wrong # args: should be "destructor body"')
                cls.destructor = TclOOMethod(
                    name="<destructor>",
                    params=[],
                    param_names=[],
                    body=parts[1],
                )
            case "superclass":
                cls.superclasses = parts[1:]
            case "mixin":
                mixin_args = parts[1:]
                if mixin_args and mixin_args[0] == "-append":
                    new_mixins = mixin_args[1:]
                else:
                    new_mixins = list(mixin_args)
                    cls.mixins = []
                # Validate: no self-mixin, no duplicates
                oo = _get_oo_runtime(interp)
                for m in new_mixins:
                    qn = m if m.startswith("::") else f"::{m}"
                    if qn == cls.qualified_name:
                        raise TclError("may not mix a class into itself")
                    if m in cls.mixins or qn in cls.mixins:
                        raise TclError("class should only be a direct mixin once")
                    cls.mixins.append(m)
            case "variable":
                cls.variables.extend(parts[1:])
            case "filter":
                cls.filters.extend(parts[1:])
            case "forward":
                if len(parts) < 3:
                    raise TclError('wrong # args: should be "forward name cmdName ?arg ...?"')
                fwd_name = parts[1]
                fwd_target = parts[2:]
                cls.methods[fwd_name] = TclOOMethod(
                    name=fwd_name,
                    params=[("args", None)],
                    param_names=["args"],
                    body="",
                    has_args=True,
                    forward_target=fwd_target,
                )
            case "deletemethod":
                for m in parts[1:]:
                    cls.methods.pop(m, None)
            case "renamemethod":
                if len(parts) < 3:
                    raise TclError('wrong # args: should be "renamemethod oldName newName"')
                old_name, new_name = parts[1], parts[2]
                method = cls.methods.pop(old_name, None)
                if method is not None:
                    method.name = new_name
                    cls.methods[new_name] = method
            case "export":
                for m in parts[1:]:
                    cls.exported_methods.add(m)
                    cls.unexported_methods.discard(m)
                    method = cls.methods.get(m)
                    if method:
                        method.visibility = "public"
            case "unexport":
                for m in parts[1:]:
                    cls.unexported_methods.add(m)
                    cls.exported_methods.discard(m)
                    method = cls.methods.get(m)
                    if method:
                        method.visibility = "unexported"
            case "private":
                # TIP 500: private method — only accessible via my from
                # within the defining class.
                # private method name args body
                if len(parts) >= 5 and parts[1] == "method":
                    name, param_str, method_body = parts[2], parts[3], parts[4]
                    params, param_names, has_args = _parse_method_params(param_str)
                    cls.methods[name] = TclOOMethod(
                        name=name,
                        params=params,
                        param_names=param_names,
                        body=method_body,
                        has_args=has_args,
                        visibility="private",
                    )
                elif len(parts) >= 2 and parts[1] == "variable":
                    # private variable — same as variable but in private context
                    cls.variables.extend(parts[2:])
                elif len(parts) == 2:
                    # private {block} — evaluate block as Tcl
                    interp.eval(parts[1])
                elif len(parts) >= 2:
                    # private someCommand args... — evaluate as Tcl
                    from ..machine import _list_escape

                    cmd = " ".join(_list_escape(a) for a in parts[1:])
                    interp.eval(cmd)
            case "self":
                # oo::define ClassName { self { method foo {} {...} } }
                # Applies definitions to the class object itself.
                if len(parts) >= 2:
                    oo = _get_oo_runtime(interp)
                    obj = oo.objects.get(cls.qualified_name)
                    if obj is not None:
                        # If a single block, parse as objdefine body
                        # If multiple args, treat as single subcommand
                        if len(parts) == 2:
                            _parse_objdefine_body(interp, obj, parts[1])
                        else:
                            from ..machine import _list_escape

                            reconstructed = " ".join(_list_escape(a) for a in parts[1:])
                            _parse_objdefine_body(interp, obj, reconstructed)
            case "definitionnamespace":
                pass  # not yet implemented
            case _:
                # In C Tcl, oo::define evaluates the body as a Tcl script
                # in the ::oo::define namespace.  Non-definition commands
                # are evaluated normally.
                from ..scope import ensure_namespace

                oo_define_ns = ensure_namespace(interp.root_namespace, "::oo::define")
                saved_ns = interp.current_namespace
                interp.current_namespace = oo_define_ns
                try:
                    interp.eval(line)
                finally:
                    interp.current_namespace = saved_ns

        i += 1


def _parse_objdefine_body(
    interp: TclInterp,
    obj: TclOOObject,
    body: str,
) -> None:
    """Parse an oo::objdefine body and populate the object's instance methods."""
    from ..machine import _split_list

    lines = _split_body_lines(body)
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if not line or line.startswith("#"):
            i += 1
            continue

        while line.count("{") > line.count("}") and i + 1 < len(lines):
            i += 1
            line += "\n" + lines[i]

        parts = _split_list(line)
        if not parts:
            i += 1
            continue

        subcmd = _abbreviate(parts[0], _OBJDEFINE_SUBCMDS)
        match subcmd:
            case "method":
                if len(parts) < 4:
                    raise TclError('wrong # args: should be "method name args body"')
                name = parts[1]
                # Check for -export/-unexport/-private flag
                flag_vis = None
                if parts[2].startswith("-") and len(parts) >= 5:
                    flag = parts[2]
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
                    param_str, method_body = parts[3], parts[4]
                else:
                    param_str, method_body = parts[2], parts[3]
                params, param_names, has_args = _parse_method_params(param_str)
                vis = flag_vis if flag_vis is not None else _default_method_visibility(name)
                obj.instance_methods[name] = TclOOMethod(
                    name=name,
                    params=params,
                    param_names=param_names,
                    body=method_body,
                    has_args=has_args,
                    visibility=vis,
                )
            case "forward":
                if len(parts) < 3:
                    raise TclError('wrong # args: should be "forward name cmdName ?arg ...?"')
                fwd_name = parts[1]
                fwd_target = parts[2:]
                obj.instance_methods[fwd_name] = TclOOMethod(
                    name=fwd_name,
                    params=[("args", None)],
                    param_names=["args"],
                    body="",
                    has_args=True,
                    forward_target=fwd_target,
                    visibility=_default_method_visibility(fwd_name),
                )
            case "deletemethod":
                for m in parts[1:]:
                    obj.instance_methods.pop(m, None)
            case "renamemethod":
                if len(parts) < 3:
                    raise TclError('wrong # args: should be "renamemethod oldName newName"')
                old_name, new_name = parts[1], parts[2]
                method = obj.instance_methods.pop(old_name, None)
                if method is not None:
                    method.name = new_name
                    obj.instance_methods[new_name] = method
            case "mixin":
                mixin_args = parts[1:]
                if mixin_args and mixin_args[0] == "-append":
                    new_mixins = mixin_args[1:]
                else:
                    new_mixins = list(mixin_args)
                    obj.instance_mixins = []
                for m in new_mixins:
                    qn = m if m.startswith("::") else f"::{m}"
                    # Check for duplicates
                    for existing in obj.instance_mixins:
                        existing_qn = existing if existing.startswith("::") else f"::{existing}"
                        if qn == existing_qn:
                            raise TclError("class should only be a direct mixin once")
                    obj.instance_mixins.append(m)
            case "variable":
                obj.instance_variables.extend(parts[1:])
            case "filter":
                obj.instance_filters.extend(parts[1:])
            case "export":
                for m in parts[1:]:
                    obj.exported_methods.add(m)
                    obj.unexported_methods.discard(m)
                    method = obj.instance_methods.get(m)
                    if method:
                        method.visibility = "public"
            case "unexport":
                for m in parts[1:]:
                    obj.unexported_methods.add(m)
                    obj.exported_methods.discard(m)
                    method = obj.instance_methods.get(m)
                    if method:
                        method.visibility = "unexported"
            case "class":
                if len(parts) < 2:
                    raise TclError('wrong # args: should be "class className"')
                new_class = parts[1]
                oo = _get_oo_runtime(interp)
                # Resolve class name
                cls = oo.classes.get(new_class)
                if cls is None:
                    qn = f"::{new_class}" if not new_class.startswith("::") else new_class
                    cls = oo.classes.get(qn)
                if cls is None:
                    raise TclError(f'"{new_class}" is not a class')
                # Check: cannot make a class an instance of itself
                if cls.qualified_name == obj.name:
                    raise TclError(
                        "may not change classes into an instance of themselves"
                    )
                # Change the object's class
                obj.class_name = cls.qualified_name
                # If switching to oo::class, register as a class
                if oo._is_metaclass(cls) and obj.name not in oo.classes:
                    new_cls_obj = TclOOClass(
                        name=obj.name.rsplit("::", 1)[-1],
                        qualified_name=obj.name,
                    )
                    oo.register_class(new_cls_obj)
                    _register_class_command(interp, oo, obj.name, new_cls_obj)
                oo.invalidate_all_mro()
            case "private":
                # TIP 500: private instance method
                if len(parts) >= 5 and parts[1] == "method":
                    name, param_str, method_body = parts[2], parts[3], parts[4]
                    params, param_names, has_args = _parse_method_params(param_str)
                    obj.instance_methods[name] = TclOOMethod(
                        name=name,
                        params=params,
                        param_names=param_names,
                        body=method_body,
                        has_args=has_args,
                        visibility="private",
                    )
                elif len(parts) >= 2 and parts[1] == "variable":
                    obj.instance_variables.extend(parts[2:])
                elif len(parts) == 2:
                    # private {block} — evaluate block as Tcl
                    interp.eval(parts[1])
                elif len(parts) >= 2:
                    # private someCommand args... — evaluate as Tcl
                    from ..machine import _list_escape

                    cmd = " ".join(_list_escape(a) for a in parts[1:])
                    interp.eval(cmd)
            case _:
                pass

        i += 1


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
    return interp._oo_runtime


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
        _parse_class_body(interp, cls, args[2])

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

    # Resolve class name
    cls = oo.classes.get(class_name)
    if cls is None:
        qualified = f"::{class_name}" if not class_name.startswith("::") else class_name
        cls = oo.classes.get(qualified)
    if cls is None:
        raise TclError(f'unknown class "{class_name}"')

    if len(args) == 2:
        # Body form: oo::define Dog { method bark {} { ... } }
        # Special case: "self" alone returns the class name (TIP #470)
        if args[1].strip() == "self":
            return TclResult(value=cls.qualified_name)
        _parse_class_body(interp, cls, args[1])
    else:
        # Single-subcommand form: oo::define Dog superclass Animal
        # Special case: "self" alone returns the class name (TIP #470)
        if args[1] == "self" and len(args) == 2:
            return TclResult(value=cls.qualified_name)
        from ..machine import _list_escape

        reconstructed = " ".join(_list_escape(a) for a in args[1:])
        _parse_class_body(interp, cls, reconstructed)

    # Invalidate ALL classes — adding a mixin/superclass to one class
    # can change the MRO of any subclass
    oo.invalidate_all_mro()
    return TclResult()


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
        _parse_objdefine_body(interp, obj, args[1])
    else:
        # Single-subcommand form: oo::objdefine obj method foo {} {body}
        # Reconstruct with braces to preserve structure
        from ..machine import _list_escape

        reconstructed = " ".join(_list_escape(a) for a in args[1:])
        _parse_objdefine_body(interp, obj, reconstructed)

    # Re-register the object command so new methods are visible
    # But skip if the object is now a class (class command was already registered)
    if obj.name in oo.classes:
        _register_class_command(interp, oo, obj.name, oo.classes[obj.name])
    else:
        oo._register_object_command(interp, obj)
    return TclResult()


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

    # Determine target name
    oo._next_obj_id += 1
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
        return TclResult(value=tgt_name)

    tgt_ns = f"::oo::Obj{oo._next_obj_id}"
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
    if not args:
        return TclResult(value=oo.self_name(interp))
    subcmd = args[0]
    if subcmd == "object":
        return TclResult(value=oo.self_name(interp))
    if subcmd == "class":
        frame = interp.current_frame
        class_name = getattr(frame, "_oo_class", None)
        if class_name is None:
            raise TclError('"self class" may only be invoked from within a method')
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
    raise TclError(
        f'unknown method "{subcmd}": must be caller, class, filter, method, namespace, object, or target'
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
