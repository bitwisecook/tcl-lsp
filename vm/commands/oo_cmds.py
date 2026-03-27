"""TclOO commands: oo::class, oo::define, oo::object, self, my, next, nextto."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..oo import OORuntime, TclOOClass, TclOOMethod
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

    # Split body into commands (simple line-based parsing)
    # Use the interpreter's own parser for robustness
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
                cls.mixins = parts[1:]
            case "variable":
                cls.variables.extend(parts[1:])
            case "filter":
                cls.filters.extend(parts[1:])
            case "forward":
                pass  # forward handled at dispatch time
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
            obj_name = oo.create_object(
                interp, qualified, obj_name=cmd_args[1], args=cmd_args[2:]
            )
            return TclResult(value=obj_name)
        raise TclError(f'unknown method "{method}": must be create or new')

    interp._runtime_commands[qualified] = _class_cmd
    # Also register short name
    if name != qualified:
        interp._runtime_commands[name] = _class_cmd

    return TclResult(value=qualified)


def _cmd_oo_define(interp: TclInterp, args: list[str]) -> TclResult:
    """oo::define className body"""
    if len(args) < 2:
        raise TclError('wrong # args: should be "oo::define className body"')

    class_name = args[0]
    oo = _get_oo_runtime(interp)

    # Resolve class name
    cls = oo.classes.get(class_name)
    if cls is None:
        qualified = f"::{class_name}" if not class_name.startswith("::") else class_name
        cls = oo.classes.get(qualified)
    if cls is None:
        raise TclError(f'unknown class "{class_name}"')

    _parse_class_body(interp, cls, args[1])
    cls.invalidate_mro()
    return TclResult()


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
    # For now, nextto is a simplified version — just calls next
    oo = _get_oo_runtime(interp)
    return oo.next_dispatch(interp, args[1:])


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
    REGISTRY.register_handler("oo::configurable", _cmd_oo_configurable)
    REGISTRY.register_handler("oo::abstract", _cmd_oo_abstract)
    REGISTRY.register_handler("oo::singleton", _cmd_oo_singleton)
    REGISTRY.register_handler("self", _cmd_self)
    REGISTRY.register_handler("my", _cmd_my)
    REGISTRY.register_handler("next", _cmd_next)
    REGISTRY.register_handler("nextto", _cmd_nextto)
