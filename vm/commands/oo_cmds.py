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


def _parse_class_body(
    interp: TclInterp,
    cls: TclOOClass,
    body: str,
) -> None:
    """Parse a class definition body and populate the class."""
    from ..machine import _split_list

    # Split body into commands via simple line-based parsing with
    # brace-balancing for multi-line commands.  This is a lightweight
    # approach — it does not use the full Tcl parser.
    lines = body.strip().split("\n")
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

        subcmd = parts[0]
        match subcmd:
            case "method":
                if len(parts) < 4:
                    raise TclError('wrong # args: should be "method name args body"')
                name, param_str, method_body = parts[1], parts[2], parts[3]
                params, param_names, has_args = _parse_method_params(param_str)
                cls.methods[name] = TclOOMethod(
                    name=name,
                    params=params,
                    param_names=param_names,
                    body=method_body,
                    has_args=has_args,
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
                    cls.mixins.extend(mixin_args[1:])
                else:
                    cls.mixins = list(mixin_args)
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
                    method = cls.methods.get(m)
                    if method:
                        method.visibility = "public"
            case "unexport":
                for m in parts[1:]:
                    method = cls.methods.get(m)
                    if method:
                        method.visibility = "unexported"
            case _:
                pass  # ignore unknown subcommands

        i += 1


def _parse_objdefine_body(
    interp: TclInterp,
    obj: TclOOObject,
    body: str,
) -> None:
    """Parse an oo::objdefine body and populate the object's instance methods."""
    from ..machine import _split_list

    lines = body.strip().split("\n")
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

        subcmd = parts[0]
        match subcmd:
            case "method":
                if len(parts) < 4:
                    raise TclError('wrong # args: should be "method name args body"')
                name, param_str, method_body = parts[1], parts[2], parts[3]
                params, param_names, has_args = _parse_method_params(param_str)
                obj.instance_methods[name] = TclOOMethod(
                    name=name,
                    params=params,
                    param_names=param_names,
                    body=method_body,
                    has_args=has_args,
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
                if not hasattr(obj, "instance_mixins"):
                    obj.instance_mixins = []
                if mixin_args and mixin_args[0] == "-append":
                    obj.instance_mixins.extend(mixin_args[1:])
                else:
                    obj.instance_mixins = list(mixin_args)
            case "variable":
                if not hasattr(obj, "instance_variables"):
                    obj.instance_variables = []
                obj.instance_variables.extend(parts[1:])
            case "filter":
                if not hasattr(obj, "instance_filters"):
                    obj.instance_filters = []
                obj.instance_filters.extend(parts[1:])
            case "export":
                for m in parts[1:]:
                    method = obj.instance_methods.get(m)
                    if method:
                        method.visibility = "public"
            case "unexport":
                for m in parts[1:]:
                    method = obj.instance_methods.get(m)
                    if method:
                        method.visibility = "unexported"
            case _:
                pass

        i += 1


def _get_oo_runtime(interp: TclInterp) -> OORuntime:
    """Get or create the OO runtime on the interpreter."""
    if not hasattr(interp, "_oo_runtime"):
        interp._oo_runtime = OORuntime()
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

    if len(args) >= 3:
        _parse_class_body(interp, cls, args[2])

    # Default superclass if none specified
    if not cls.superclasses and qualified != "::oo::object":
        cls.superclasses = ["::oo::object"]

    oo.register_class(cls)

    # Register class as a command for `ClassName new` / `ClassName create`
    def _class_cmd(interp: TclInterp, cmd_args: list[str]) -> TclResult:
        if not cmd_args:
            raise TclError(f'wrong # args: should be "{qualified} method ?arg ...?"')
        method = cmd_args[0]
        if method == "new":
            obj_name = oo.create_object(interp, qualified, args=cmd_args[1:])
            return TclResult(value=obj_name)
        if method == "create":
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
            obj_name = oo.create_object(interp, qualified, obj_name=create_name, args=cmd_args[2:])
            return TclResult(value=obj_name)
        raise TclError(f'unknown method "{method}": must be create or new')

    interp._runtime_commands[qualified] = _class_cmd
    # Also register short name
    if name != qualified:
        interp._runtime_commands[name] = _class_cmd

    return TclResult(value=qualified)


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
        _parse_class_body(interp, cls, args[1])
    else:
        # Single-subcommand form: oo::define Dog superclass Animal
        # Reconstruct with braces to preserve structure
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
        obj_name = oo.create_object(interp, "::oo::object", obj_name=name, args=args[2:])
        return TclResult(value=obj_name)
    elif subcmd == "destroy":
        # oo::object itself cannot be destroyed in normal usage
        raise TclError("cannot destroy the root object")
    else:
        raise TclError(f'unknown method "{subcmd}": must be create, destroy or new')


def _cmd_oo_copy(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::copy sourceObject ?targetObject? ?targetNamespace?"""
    if not args:
        raise TclError(
            'wrong # args: should be "oo::copy sourceObject ?targetObject? ?targetNamespace?"'
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
    if len(args) >= 2:
        tgt_name = args[1]
        if not tgt_name.startswith("::"):
            ns = interp.current_namespace.qualname
            if ns == "::":
                tgt_name = f"::{tgt_name}"
            else:
                tgt_name = f"{ns}::{tgt_name}"
    else:
        oo._next_obj_id += 1
        tgt_name = f"::oo::Obj{oo._next_obj_id}"

    import copy

    from ..oo import TclOOObject

    tgt = TclOOObject(
        name=tgt_name,
        class_name=src.class_name,
        namespace=tgt_name,
        instance_methods={k: copy.copy(v) for k, v in src.instance_methods.items()},
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
    raise TclError(f'unknown method "{subcmd}": must be class, method, namespace, or object')


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
        raise TclError('wrong # args: should be "nextto class ?arg ...?"')
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
