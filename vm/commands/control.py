"""Control flow commands: eval, source, catch, if, for, while, foreach, switch, break, continue."""

from __future__ import annotations

import fnmatch
import os
from typing import TYPE_CHECKING

from ..machine import _list_escape, _split_list
from ..types import ReturnCode, TclBreak, TclContinue, TclError, TclResult, TclReturn

if TYPE_CHECKING:
    from ..interp import TclInterp


def _format_eval_cmd_text(args: list[str]) -> str:
    """Build the command text for eval's 'invoked from within' frame.

    Reconstructs ``eval arg ...`` with quoting for args that contain
    spaces, and truncates to ~150 chars like Tcl.
    """
    parts = ["eval"]
    for a in args:
        if " " in a or "\t" in a or "\n" in a or not a:
            parts.append(f'"{a}"')
        else:
            parts.append(a)
    text = " ".join(parts)
    if len(text) > 150:
        text = text[:150] + "..."
    return text


def _cmd_eval(interp: TclInterp, args: list[str]) -> TclResult:
    """eval arg ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "eval arg ?arg ...?"')
    # Capture the original (pre-substitution) command text before eval
    # overwrites it.  This is used for the "invoked from within" frame.
    saved_cmd_text = interp._error_cmd_text
    script = " ".join(args) if len(args) > 1 else args[0]
    try:
        return interp.eval(script)
    except TclError as e:
        # Add ("eval" body line 1) frame to errorInfo, matching C Tcl.
        if not e.error_info:
            e.error_info = [str(e)]
        e.error_info.append('    ("eval" body line 1)')
        # Add "invoked from within" frame with the original source text
        # of the eval command (before argument substitution).
        if saved_cmd_text:
            cmd_text = saved_cmd_text
            if len(cmd_text) > 150:
                cmd_text = cmd_text[:150] + "..."
        else:
            cmd_text = _format_eval_cmd_text(args)
        e.error_info.append(f'    invoked from within\n"{cmd_text}"')
        raise


def _cmd_source(interp: TclInterp, args: list[str]) -> TclResult:
    """source ?-encoding name? ?-nopkg? fileName"""
    remaining = list(args)
    while remaining and remaining[0].startswith("-"):
        if remaining[0] == "-encoding" and len(remaining) > 1:
            remaining = remaining[2:]
        elif remaining[0] == "-nopkg":
            remaining = remaining[1:]
        elif remaining[0] == "--":
            remaining = remaining[1:]
            break
        else:
            break
    if not remaining:
        raise TclError('wrong # args: should be "source ?-encoding name? fileName"')
    filename = remaining[0]
    try:
        with open(filename) as f:
            script = f.read()
    except OSError as e:
        raise TclError(f'couldn\'t read file "{filename}": {e}') from e

    old_script = interp.script_file
    interp.script_file = os.path.abspath(filename)
    try:
        return interp.eval(script)
    except TclReturn as ret:
        # In real Tcl, ``return`` in a sourced file terminates the
        # source, not the caller.  A plain ``return`` (level 1,
        # code OK) becomes an OK result.  ``return -code <X>``
        # propagates the special code.
        if ret.level <= 1 and ret.code == 0:
            return TclResult(value=ret.value)
        if ret.level > 1:
            raise TclReturn(
                value=ret.value,
                code=ret.code,
                level=ret.level - 1,
                error_info=ret.error_info,
                error_code=ret.error_code,
            )
        # ret.code != 0: re-raise as the appropriate exception type
        if ret.code == ReturnCode.ERROR:
            raise TclError(ret.value) from None
        if ret.code == ReturnCode.BREAK:
            raise TclReturn(
                value=ret.value,
                code=ReturnCode.BREAK,
                level=0,
            )
        if ret.code == ReturnCode.CONTINUE:
            raise TclReturn(
                value=ret.value,
                code=ReturnCode.CONTINUE,
                level=0,
            )
        # For arbitrary integer codes, re-raise as TclReturn
        raise TclReturn(
            value=ret.value,
            code=ret.code,
            level=0,
            error_info=ret.error_info,
            error_code=ret.error_code,
        )
    finally:
        interp.script_file = old_script


def _cmd_catch(interp: TclInterp, args: list[str]) -> TclResult:
    """catch script ?resultVarName? ?optionsVarName?"""
    if not args:
        raise TclError('wrong # args: should be "catch script ?resultVarName? ?optionsVarName?"')
    script = args[0]
    result_var = args[1] if len(args) > 1 else None
    options_var = args[2] if len(args) > 2 else None

    error_info_str = ""
    error_code = "NONE"
    try:
        result = interp.eval(script)
        code = result.code
        value = result.value
    except TclError as e:
        code = ReturnCode.ERROR
        value = e.message
        # Build errorInfo from accumulated trace frames
        if e.error_info:
            error_info_str = "\n".join(e.error_info)
        else:
            error_info_str = e.message
        error_code = e.error_code if e.error_code else "NONE"
    except TclReturn as ret:
        code = ReturnCode.RETURN
        value = ret.value
    except TclBreak:
        code = ReturnCode.BREAK
        value = ""
        error_info_str = 'invoked "break" outside of a loop'
    except TclContinue:
        code = ReturnCode.CONTINUE
        value = ""
        error_info_str = 'invoked "continue" outside of a loop'

    # Set $::errorInfo and $::errorCode when an error was caught
    if code == ReturnCode.ERROR:
        interp.global_frame.set_var("errorInfo", error_info_str)
        interp.global_frame.set_var("errorCode", error_code)

    if result_var:
        interp.current_frame.set_var(result_var, value)
    if options_var:
        from ..machine import _list_escape

        opts = (
            f"-code {int(code)} -level 0"
            f" -errorcode {_list_escape(error_code)}"
            f" -errorinfo {_list_escape(error_info_str)}"
        )
        interp.current_frame.set_var(options_var, opts)

    return TclResult(value=str(int(code)))


def _cmd_try(interp: TclInterp, args: list[str]) -> TclResult:
    """try body ?on code varList body ...? ?finally body?"""
    if not args:
        raise TclError('wrong # args: should be "try body ?handler ...? ?finally body?"')

    body = args[0]
    i = 1
    handlers: list[tuple[str, str, str, str]] = []  # (kind, code, varList, handlerBody)
    finally_body: str | None = None

    while i < len(args):
        kw = args[i]
        if kw == "finally" and i + 1 < len(args):
            finally_body = args[i + 1]
            i += 2
        elif kw in ("on", "trap") and i + 3 < len(args):
            handlers.append((kw, args[i + 1], args[i + 2], args[i + 3]))
            i += 4
        else:
            i += 1

    # Execute the body
    error_info_str = ""
    error_code = "NONE"
    try:
        result = interp.eval(body)
        code = ReturnCode.OK
        value = result.value
        message = ""
    except TclError as e:
        code = ReturnCode.ERROR
        value = e.message
        message = e.message
        if e.error_info:
            error_info_str = "\n".join(e.error_info)
        else:
            error_info_str = e.message
        error_code = e.error_code if e.error_code else "NONE"
    except TclReturn as ret:
        code = ret.code
        value = ret.value
        message = ""
    except TclBreak:
        code = ReturnCode.BREAK
        value = ""
        message = ""
    except TclContinue:
        code = ReturnCode.CONTINUE
        value = ""
        message = ""

    # Set $::errorInfo and $::errorCode when an error was caught
    if code == ReturnCode.ERROR:
        interp.global_frame.set_var("errorInfo", error_info_str)
        interp.global_frame.set_var("errorCode", error_code)

    # Match handlers
    handler_result: TclResult | None = None
    for kind, match_code, var_list, handler_body in handlers:
        if kind == "on":
            code_map = {"ok": 0, "error": 1, "return": 2, "break": 3, "continue": 4}
            if match_code in code_map:
                target_code = code_map[match_code]
            else:
                try:
                    target_code = int(match_code)
                except ValueError:
                    continue
            if int(code) == target_code:
                vars_list = _split_list(var_list)
                if vars_list:
                    interp.current_frame.set_var(vars_list[0], value)
                if len(vars_list) > 1:
                    opts = (
                        f"-code {int(code)} -level 0"
                        f" -errorcode {_list_escape(error_code)}"
                        f" -errorinfo {_list_escape(error_info_str)}"
                    )
                    interp.current_frame.set_var(vars_list[1], opts)
                handler_result = interp.eval(handler_body)
                break
        elif kind == "trap":
            if code == ReturnCode.ERROR:
                # Match error code pattern
                vars_list = _split_list(var_list)
                if vars_list:
                    interp.current_frame.set_var(vars_list[0], message)
                if len(vars_list) > 1:
                    opts = (
                        f"-code {int(code)} -level 0"
                        f" -errorcode {_list_escape(error_code)}"
                        f" -errorinfo {_list_escape(error_info_str)}"
                    )
                    interp.current_frame.set_var(vars_list[1], opts)
                handler_result = interp.eval(handler_body)
                break

    # Execute finally
    if finally_body is not None:
        interp.eval(finally_body)

    if handler_result is not None:
        return handler_result

    # Re-raise if no handler matched
    if code == ReturnCode.ERROR:
        raise TclError(value)
    if code == ReturnCode.BREAK:
        raise TclBreak()
    if code == ReturnCode.CONTINUE:
        raise TclContinue()

    return TclResult(value=value)


def _cmd_if(interp: TclInterp, args: list[str]) -> TclResult:
    """if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?"""
    if not args:
        raise TclError(
            'wrong # args: should be "if expr1 ?then? body1 elseif expr2 ?then? body2 ... ?else? ?bodyN?"'
        )

    # Pre-validate: if "else" appears, it must be followed by exactly one
    # argument (the body) with nothing after it.  Tcl checks this before
    # evaluating any branch.
    for k, a in enumerate(args):
        if a == "else":
            if k + 1 >= len(args):
                raise TclError('wrong # args: no script following "else" argument')
            if k + 2 < len(args):
                raise TclError('wrong # args: extra words after "else" clause in "if" command')
            break

    i = 0
    while i < len(args):
        if args[i] == "elseif":
            i += 1
            continue

        if args[i] == "else":
            return interp.eval(args[i + 1])

        # Evaluate condition
        cond_str = args[i]
        cond_result = interp.eval_expr(cond_str)
        i += 1

        if i < len(args) and args[i] == "then":
            i += 1

        if i >= len(args):
            raise TclError("wrong # args: no script following expression")

        if _tcl_bool(cond_result):
            return interp.eval(args[i])
        i += 1

    return TclResult()


def _cmd_for(interp: TclInterp, args: list[str]) -> TclResult:
    """for start test next body"""
    if len(args) != 4:
        raise TclError('wrong # args: should be "for start test next command"')
    start, test, next_cmd, body = args

    interp.eval(start)
    while True:
        cond = interp.eval_expr(test)
        if not _tcl_bool(cond):
            break
        try:
            interp.eval(body)
        except TclBreak:
            break
        except TclContinue:
            pass
        # break in step clause terminates loop;
        # continue in step clause propagates out (C Tcl semantics)
        try:
            interp.eval(next_cmd)
        except TclBreak:
            break
    return TclResult()


def _cmd_while(interp: TclInterp, args: list[str]) -> TclResult:
    """while test body"""
    if len(args) != 2:
        raise TclError('wrong # args: should be "while test command"')
    test, body = args

    while True:
        cond = interp.eval_expr(test)
        if not _tcl_bool(cond):
            break
        try:
            interp.eval(body)
        except TclBreak:
            break
        except TclContinue:
            pass
    return TclResult()


def _cmd_foreach(interp: TclInterp, args: list[str]) -> TclResult:
    """foreach varList list ?varList list ...? command"""
    if len(args) < 3 or len(args) % 2 == 0:
        raise TclError('wrong # args: should be "foreach varList list ?varList list ...? command"')

    body = args[-1]
    iterators: list[tuple[list[str], list[str]]] = []
    for i in range(0, len(args) - 1, 2):
        var_names = _split_list(args[i])
        list_values = _split_list(args[i + 1])
        iterators.append((var_names, list_values))

    # Tcl requires each varlist to be non-empty.
    for var_names, _vals in iterators:
        if not var_names:
            raise TclError("foreach varlist is empty")

    max_iters = (
        max((len(vals) + len(var_names) - 1) // len(var_names) for var_names, vals in iterators)
        if iterators
        else 0
    )

    for iteration in range(max_iters):
        for var_names, list_values in iterators:
            for j, vn in enumerate(var_names):
                idx = iteration * len(var_names) + j
                if idx < len(list_values):
                    interp.current_frame.set_var(vn, list_values[idx])
                else:
                    interp.current_frame.set_var(vn, "")
        try:
            interp.eval(body)
        except TclBreak:
            break
        except TclContinue:
            pass
    return TclResult()


def _cmd_lmap(interp: TclInterp, args: list[str]) -> TclResult:
    """lmap varList list ?varList list ...? command

    Like foreach, but collects each body result into a list.
    """
    if len(args) < 3 or len(args) % 2 == 0:
        raise TclError('wrong # args: should be "lmap varList list ?varList list ...? command"')

    body = args[-1]
    iterators: list[tuple[list[str], list[str]]] = []
    for i in range(0, len(args) - 1, 2):
        var_names = _split_list(args[i])
        list_values = _split_list(args[i + 1])
        iterators.append((var_names, list_values))

    # Tcl requires each varlist to be non-empty.
    for var_names, _vals in iterators:
        if not var_names:
            raise TclError("lmap varlist is empty")

    max_iters = (
        max((len(vals) + len(var_names) - 1) // len(var_names) for var_names, vals in iterators)
        if iterators
        else 0
    )

    collected: list[str] = []
    for iteration in range(max_iters):
        for var_names, list_values in iterators:
            for j, vn in enumerate(var_names):
                idx = iteration * len(var_names) + j
                if idx < len(list_values):
                    interp.current_frame.set_var(vn, list_values[idx])
                else:
                    interp.current_frame.set_var(vn, "")
        try:
            result = interp.eval(body)
            collected.append(result.value)
        except TclBreak:
            break
        except TclContinue:
            pass
    return TclResult(value=" ".join(_list_escape(v) for v in collected))


def _cmd_switch(interp: TclInterp, args: list[str]) -> TclResult:
    """switch ?options? string pattern body ?pattern body ...?
    switch ?options? string {pattern body ?pattern body ...?}
    """
    if not args:
        raise TclError(
            'wrong # args: should be "switch ?switches? string pattern body ... ?default body?"'
        )

    remaining = list(args)
    mode = "exact"  # exact | glob | regexp
    nocase = False

    while remaining and remaining[0].startswith("-"):
        opt = remaining.pop(0)
        if opt == "--":
            break
        elif "-exact".startswith(opt) and len(opt) >= 2:
            mode = "exact"
        elif "-glob".startswith(opt) and len(opt) >= 2:
            mode = "glob"
        elif "-regexp".startswith(opt) and len(opt) >= 2:
            mode = "regexp"
        elif "-nocase".startswith(opt) and len(opt) >= 2:
            nocase = True
        elif opt in ("-indexvar", "-matchvar"):
            # Consume the next argument (variable name)
            if remaining:
                remaining.pop(0)
        else:
            raise TclError(
                f'bad option "{opt}": must be -exact, -glob, -indexvar, '
                f"-matchvar, -nocase, -regexp, or --"
            )

    if not remaining:
        raise TclError(
            'wrong # args: should be "switch ?switches? string pattern body ... ?default body?"'
        )

    subject = remaining[0]
    remaining = remaining[1:]

    # Single-arg form: {pattern body pattern body ...}
    if len(remaining) == 1:
        pairs_list = _split_list(remaining[0])
        remaining = pairs_list

    if len(remaining) % 2 != 0:
        raise TclError("extra switch pattern with no body")

    # The last body must not be a fall-through
    if remaining and remaining[-1] == "-":
        raise TclError('no body specified for pattern "' + remaining[-2] + '"')

    # Process pattern/body pairs
    i = 0
    while i + 1 < len(remaining):
        pattern = remaining[i]
        body = remaining[i + 1]

        matched = False
        # "default" is only special as the last pattern
        if pattern == "default" and i + 2 >= len(remaining):
            matched = True
        elif mode == "exact":
            if nocase:
                matched = subject.lower() == pattern.lower()
            else:
                matched = subject == pattern
        elif mode == "glob":
            if nocase:
                matched = fnmatch.fnmatch(subject.lower(), pattern.lower())
            else:
                matched = fnmatch.fnmatch(subject, pattern)
        elif mode == "regexp":
            import re

            flags = re.IGNORECASE if nocase else 0
            matched = re.search(pattern, subject, flags) is not None

        if matched:
            # Skip past any fall-through (-) bodies to the real body
            while body == "-" and i + 3 < len(remaining):
                i += 2
                body = remaining[i + 1]
            if body != "-":
                return interp.eval(body)
        i += 2

    return TclResult()


def _cmd_break(interp: TclInterp, args: list[str]) -> TclResult:
    """break"""
    raise TclBreak()


def _cmd_continue(interp: TclInterp, args: list[str]) -> TclResult:
    """continue"""
    raise TclContinue()


def _cmd_after(interp: TclInterp, args: list[str]) -> TclResult:
    """after ms ?script?"""
    if not args:
        raise TclError('wrong # args: should be "after option ?arg ...?"')
    if args[0] == "idle":
        if len(args) > 1:
            return interp.eval(" ".join(args[1:]))
        return TclResult()
    # Simple synchronous sleep for now
    import time

    try:
        ms = int(args[0])
        time.sleep(ms / 1000.0)
    except ValueError:
        pass
    if len(args) > 1:
        return interp.eval(" ".join(args[1:]))
    return TclResult()


def _cmd_update(interp: TclInterp, args: list[str]) -> TclResult:
    """update ?idletasks?  — stub (no-op, no event loop)."""
    return TclResult()


def _cmd_exit(interp: TclInterp, args: list[str]) -> TclResult:
    """exit ?returnCode?"""
    code = int(args[0]) if args else 0
    raise SystemExit(code)


def _cmd_time(interp: TclInterp, args: list[str]) -> TclResult:
    """time script ?count?"""
    if not args:
        raise TclError('wrong # args: should be "time command ?count?"')
    script = args[0]
    count = int(args[1]) if len(args) > 1 else 1

    import time

    start = time.perf_counter_ns()
    for _ in range(count):
        interp.eval(script)
    elapsed_ns = time.perf_counter_ns() - start
    avg_us = elapsed_ns / count / 1000
    return TclResult(value=f"{avg_us:.0f} microseconds per iteration")


def _tcl_bool(s: str) -> bool:
    """Parse a Tcl boolean value."""
    s = s.strip().lower()
    if s in ("1", "true", "yes", "on"):
        return True
    if s in ("0", "false", "no", "off"):
        return False
    try:
        return float(s) != 0
    except ValueError:
        raise TclError(f'expected boolean value but got "{s}"') from None


def register() -> None:
    """Register control flow commands."""
    from core.commands.registry import REGISTRY

    REGISTRY.register_handler("eval", _cmd_eval)
    REGISTRY.register_handler("source", _cmd_source)
    REGISTRY.register_handler("catch", _cmd_catch)
    REGISTRY.register_handler("try", _cmd_try)
    REGISTRY.register_handler("if", _cmd_if)
    REGISTRY.register_handler("for", _cmd_for)
    REGISTRY.register_handler("while", _cmd_while)
    REGISTRY.register_handler("foreach", _cmd_foreach)
    REGISTRY.register_handler("lmap", _cmd_lmap)
    REGISTRY.register_handler("switch", _cmd_switch)
    REGISTRY.register_handler("break", _cmd_break)
    REGISTRY.register_handler("continue", _cmd_continue)
    REGISTRY.register_handler("after", _cmd_after)
    REGISTRY.register_handler("update", _cmd_update)
    REGISTRY.register_handler("exit", _cmd_exit)
    REGISTRY.register_handler("time", _cmd_time)
