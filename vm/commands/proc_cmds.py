"""Procedure-related commands: proc, return, error, rename, unknown."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import ReturnCode, TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_proc(interp: TclInterp, args: list[str]) -> TclResult:
    """proc name argList body"""
    if len(args) != 3:
        raise TclError('wrong # args: should be "proc name args body"')
    name, param_str, body = args
    interp.define_proc(name, param_str, body)
    return TclResult()


def _cmd_return(interp: TclInterp, args: list[str]) -> TclResult:
    """return ?-code code? ?-level level? ?-errorcode errorcode? ?-errorinfo info? ?result?"""
    code = ReturnCode.OK
    level = 1
    value = ""
    ret_error_info: str | None = None
    ret_error_code: str | None = None
    i = 0
    while i < len(args):
        if args[i] == "-code" and i + 1 < len(args):
            code_str = args[i + 1]
            _code_map = {"ok": 0, "error": 1, "return": 2, "break": 3, "continue": 4}
            if code_str in _code_map:
                code = _code_map[code_str]
            else:
                try:
                    code = int(code_str)
                except ValueError:
                    raise TclError(
                        f'bad completion code "{code_str}": must be'
                        f" ok, error, return, break, continue, or an integer"
                    ) from None
            i += 2
        elif args[i] == "-level" and i + 1 < len(args):
            level = int(args[i + 1])
            i += 2
        elif args[i] == "-errorcode" and i + 1 < len(args):
            ret_error_code = args[i + 1]
            i += 2
        elif args[i] == "-errorinfo" and i + 1 < len(args):
            ret_error_info = args[i + 1]
            i += 2
        elif args[i] == "-options" and i + 1 < len(args):
            # Parse options dict: -code N -level N -errorcode ... -errorinfo ...
            from ..machine import _split_list

            opts = _split_list(args[i + 1])
            j = 0
            while j < len(opts):
                if opts[j] == "-code" and j + 1 < len(opts):
                    code_str_opt = opts[j + 1]
                    match code_str_opt:
                        case "ok" | "0":
                            code = ReturnCode.OK
                        case "error" | "1":
                            code = ReturnCode.ERROR
                        case "return" | "2":
                            code = ReturnCode.RETURN
                        case "break" | "3":
                            code = ReturnCode.BREAK
                        case "continue" | "4":
                            code = ReturnCode.CONTINUE
                        case _:
                            try:
                                code = int(code_str_opt)
                            except ValueError:
                                pass
                    j += 2
                elif opts[j] == "-level" and j + 1 < len(opts):
                    level = int(opts[j + 1])
                    j += 2
                elif opts[j] == "-errorinfo" and j + 1 < len(opts):
                    ret_error_info = opts[j + 1]
                    j += 2
                elif opts[j] == "-errorcode" and j + 1 < len(opts):
                    ret_error_code = opts[j + 1]
                    j += 2
                else:
                    j += 2  # skip unknown option pairs
            i += 2
        elif args[i].startswith("-") and i + 1 < len(args):
            i += 2  # skip unknown options
        else:
            value = args[i]
            i += 1

    raise TclReturn(
        value=value,
        code=code,
        level=level,
        error_info=ret_error_info,
        error_code=ret_error_code,
    )


def _cmd_error(interp: TclInterp, args: list[str]) -> TclResult:
    """error message ?info? ?code?"""
    if not args:
        raise TclError('wrong # args: should be "error message ?info? ?code?"')
    message = args[0]
    error_code = args[2] if len(args) > 2 else "NONE"
    error_info = [args[1]] if len(args) > 1 else []
    raise TclError(message, error_code=error_code, error_info=error_info)


def _cleanup_proc(interp: TclInterp, old_name: str, new_name: str) -> None:
    """Move or remove any proc entries for *old_name* during a rename."""
    proc = interp.procedures.pop(old_name, None)
    qualified_proc = interp.procedures.pop(f"::{old_name}", None)
    p = proc or qualified_proc
    if p is not None and new_name:
        interp.procedures[new_name] = p
    interp.root_namespace._procs.pop(old_name, None)


def _target_exists(interp: TclInterp, name: str) -> bool:
    """Return *True* if *name* already exists as a command or proc."""
    if interp.lookup_command(name) is not None:
        return True
    if interp.procedures.get(name) is not None:
        return True
    if interp.root_namespace.lookup_proc(name) is not None:
        return True
    return False


def _cmd_rename(interp: TclInterp, args: list[str]) -> TclResult:
    """rename oldName newName"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "rename oldName newName"')
    old_name, new_name = args

    # Reject rename to an existing command (Tcl semantics)
    if new_name and _target_exists(interp, new_name):
        raise TclError(f'can\'t rename to "{new_name}": command already exists')

    # Check flat command registry
    handler = interp.lookup_command(old_name)
    if handler is not None:
        if new_name:
            interp.register_command(new_name, handler)
        interp.unregister_command(old_name)
        # In Tcl, procs and commands share the same name space.
        # Clean up any pre-registered proc with the same name so that
        # ``rename unknown unknown.old`` also removes the proc.
        _cleanup_proc(interp, old_name, new_name)
        return TclResult()

    # Check flat user procs
    proc = interp.procedures.get(old_name)
    if proc is None:
        # Also check qualified form
        proc = interp.procedures.get(f"::{old_name}")
    if proc is not None:
        if new_name:
            interp.procedures[new_name] = proc
            # If renaming into a namespace, also register in the namespace's
            # proc table so namespace-scoped lookups find it.
            if "::" in new_name:
                from ..scope import ensure_namespace

                new_ns_part = new_name[: new_name.rfind("::")]
                new_tail = new_name[new_name.rfind("::") + 2 :]
                new_ns = ensure_namespace(
                    interp.root_namespace, new_ns_part if new_ns_part else "::"
                )
                # Update the proc's def_namespace so it runs in the new namespace
                proc.def_namespace = new_ns.qualname
                new_ns.register_proc(new_tail, proc)
        interp.procedures.pop(old_name, None)
        interp.procedures.pop(f"::{old_name}", None)
        # Also remove from namespace proc tables (root or defining namespace).
        # Namespace tables store procs by their tail name only.
        from ..scope import resolve_namespace

        if "::" in old_name:
            old_tail = old_name[old_name.rfind("::") + 2 :]
        else:
            old_tail = old_name
        def_ns = resolve_namespace(interp.root_namespace, proc.def_namespace)
        if def_ns is not None:
            def_ns._procs.pop(old_tail, None)
        interp.root_namespace._procs.pop(old_tail, None)
        return TclResult()

    # Check namespace-qualified names: look in the target namespace
    if "::" in old_name:
        from ..scope import ensure_namespace, resolve_namespace

        ns_part = old_name[: old_name.rfind("::")]
        tail = old_name[old_name.rfind("::") + 2 :]
        ns = resolve_namespace(interp.root_namespace, ns_part if ns_part else "::")
        if ns is not None:
            ns_proc = ns.lookup_proc(tail)
            ns_cmd = ns.lookup_command(tail)
            if ns_proc is not None:
                ns.remove_proc(tail)
                # Clean up flat table entries that point to this specific proc.
                # Only remove the unqualified tail entry if it references the
                # same object — a global proc with the same simple name must
                # be preserved.
                qualified_old = ns.qualname + "::" + tail
                interp.procedures.pop(old_name, None)
                interp.procedures.pop(qualified_old, None)
                if interp.procedures.get(tail) is ns_proc:
                    interp.procedures.pop(tail, None)
                if new_name:
                    interp.procedures[new_name] = ns_proc
                    if "::" in new_name:
                        new_ns_part = new_name[: new_name.rfind("::")]
                        new_tail = new_name[new_name.rfind("::") + 2 :]
                        new_ns = ensure_namespace(
                            interp.root_namespace, new_ns_part if new_ns_part else "::"
                        )
                        new_ns.register_proc(new_tail, ns_proc)
                return TclResult()
            if ns_cmd is not None:
                ns._commands.pop(tail, None)
                interp.unregister_command(old_name)
                if new_name and ns_cmd is not None:
                    interp.register_command(new_name, ns_cmd)
                return TclResult()

    # Use "can't delete" when new_name is empty (Tcl convention)
    verb = "delete" if not new_name else "rename"
    raise TclError(f"can't {verb} \"{old_name}\": command doesn't exist")


def _cmd_unknown(interp: TclInterp, args: list[str]) -> TclResult:
    """Default unknown command handler.

    Checks ``::auto_index`` for an auto-loading script.  If found,
    evaluates it (which should define the command) and then retries.
    """
    if not args:
        raise TclError('invalid command name ""')
    cmd_name = args[0]
    cmd_args = args[1:]

    # Check ::auto_index for auto-loading
    try:
        script = interp.current_frame.get_var(f"::auto_index({cmd_name})")
    except TclError:
        script = None

    if script:
        interp.eval(script)
        return interp.invoke(cmd_name, cmd_args)

    raise TclError(f'invalid command name "{cmd_name}"')


def _cmd_throw(interp: TclInterp, args: list[str]) -> TclResult:
    """throw type message"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "throw type message"')
    raise TclError(args[1], error_code=args[0])


def _cmd_apply(interp: TclInterp, args: list[str]) -> TclResult:
    """apply func ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "apply lambdaExpr ?arg ...?"')
    from ..machine import _split_list

    lambda_parts = _split_list(args[0])
    if len(lambda_parts) < 2:
        raise TclError(f'can\'t interpret "{args[0]}" as a lambda expression')
    param_str = lambda_parts[0]
    body = lambda_parts[1]
    # namespace = lambda_parts[2] if len(lambda_parts) > 2 else "::"

    interp.define_proc("::apply_lambda", param_str, body)
    result = interp.call_proc_by_name("::apply_lambda", args[1:])
    interp.procedures.pop("::apply_lambda", None)
    return result


def register() -> None:
    """Register procedure commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("proc", _cmd_proc)
    REGISTRY.register_handler("return", _cmd_return)
    REGISTRY.register_handler("error", _cmd_error)
    REGISTRY.register_handler("rename", _cmd_rename)
    REGISTRY.register_handler("unknown", _cmd_unknown)
    REGISTRY.register_handler("throw", _cmd_throw)
    REGISTRY.register_handler("apply", _cmd_apply)
