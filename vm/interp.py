"""Tcl interpreter — ties together compiler, VM, commands, and scoping."""

from __future__ import annotations

import math
import os
import re
import struct
import sys
from collections import OrderedDict
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TextIO

from core.compiler.codegen import FunctionAsm
from core.compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from core.parsing.expr_parser import parse_expr
from core.parsing.lexer import TclParseError

from .commands import CommandHandler, register_builtins
from .compiler import compile_script
from .machine import BytecodeVM
from .scope import CallFrame, Namespace, ensure_namespace, resolve_namespace
from .types import ReturnCode, TclBreak, TclContinue, TclError, TclResult, TclReturn


@dataclass
class ProcDef:
    """Definition of a user-defined procedure."""

    name: str
    param_names: list[str]
    params_with_defaults: list[tuple[str, str | None]]
    body: str
    has_args: bool = False
    compiled_asm: FunctionAsm | None = None
    def_namespace: str = "::"
    body_start_line: int = 0  # 1-based line of first body statement (for errorInfo)


# Matches ``::tcl::<base>::<sub>`` — FQ ensemble command names emitted
# by the bytecode compiler (e.g. ``::tcl::dict::values``).
_FQ_ENSEMBLE_RE = re.compile(r"^::tcl::(\w+)::(\w+)$")


def _format_cmd_text(cmd_name: str, args: list[str]) -> str:
    """Reconstruct a Tcl command string for errorInfo display.

    Arguments containing spaces or special characters are wrapped in
    double quotes to approximate the original source text.
    """
    parts = [cmd_name]
    for a in args:
        if " " in a or "\t" in a or "\n" in a or not a:
            parts.append(f'"{a}"')
        else:
            parts.append(a)
    text = " ".join(parts)
    if len(text) > 150:
        text = text[:150] + "..."
    return text


class TclInterp:
    """The main Tcl interpreter.

    Owns the global call frame, command registry, and procedure table.
    Compiles Tcl source to bytecode via the ``core/`` compiler suite
    and executes it on the ``BytecodeVM`` stack machine.
    """

    # Default path to the bundled Tcl library
    _TCL_LIBRARY = str(Path(__file__).resolve().parent.parent / "tmp" / "tcl9.0.3" / "library")

    def __init__(
        self,
        *,
        optimise: bool = False,
        source_init: bool = False,
        tcl_library: str | None = None,
        debug_hook: Any | None = None,
    ) -> None:
        self.optimise = optimise
        self._debug_hook = debug_hook

        # Namespace tree — the root namespace is ::
        self.root_namespace = Namespace("", parent=None)
        self.current_namespace = self.root_namespace

        self.global_frame = CallFrame(level=0, namespace=self.root_namespace, interp=self)
        self.root_namespace._frame = self.global_frame
        self.current_frame = self.global_frame
        # Per-interpreter runtime command table for dynamically registered
        # commands (rename, namespace ensemble, tcltest, etc.).  Built-in
        # command handlers live on the global REGISTRY instead.
        self._runtime_commands: dict[str, CommandHandler] = {}
        register_builtins()
        self.procedures: dict[str, ProcDef] = {}
        self.script_file: str | None = None
        self.cmd_count = 0
        # Cache compiled bytecode for short, repeatedly-evaluated scripts.
        # This is especially important for tight loops (e.g. foreach/lmap)
        # that invoke ``eval`` with the same body many times.
        self._eval_cache: OrderedDict[str, tuple[Any, Any]] = OrderedDict()
        self._eval_cache_max_entries = 512
        self._eval_cache_max_source_len = 512

        # 1-based source line of the most recent INVOKE instruction
        # (used by _call_proc to build errorInfo procedure frames).
        self._error_line: int = 0

        # Original (pre-substitution) command text of the most recent
        # INVOKE instruction, for accurate errorInfo display.
        self._error_cmd_text: str = ""

        # Package registry: name -> {version, loaded, ifneeded}
        self.packages: dict[str, dict[str, str | bool | dict[str, str] | None]] = {}
        self._package_unknown: str | None = None

        # Variable traces: var_name -> list of (ops, script)
        self.variable_traces: dict[str, list[tuple[list[str], str]]] = {}

        # I/O channels
        self.channels: dict[str, TextIO] = {
            "stdout": sys.stdout,
            "stderr": sys.stderr,
            "stdin": sys.stdin,
        }

        # Resolve the Tcl library path
        self._tcl_library = tcl_library or self._TCL_LIBRARY

        # Initialise common Tcl variables
        self.global_frame.set_var("tcl_version", "9.0")
        self.global_frame.set_var("tcl_patchLevel", "9.0.3")
        self.global_frame.set_var("tcl_library", self._tcl_library)
        self.global_frame.set_var("tcl_pkgPath", "")
        self.global_frame.set_var("tcl_platform(os)", sys.platform)
        self.global_frame.set_var("tcl_platform(platform)", "unix")
        self.global_frame.set_var("tcl_platform(machine)", "x86_64")
        self.global_frame.set_var("tcl_platform(pathSeparator)", ":")
        self.global_frame.set_var("tcl_platform(wordSize)", str(struct.calcsize("P")))
        self.global_frame.set_var("tcl_platform(byteOrder)", sys.byteorder)
        self.global_frame.set_var("tcl_platform(pointerSize)", str(struct.calcsize("P")))
        self.global_frame.set_var("tcl_interactive", "0")
        self.global_frame.set_var("errorInfo", "")
        self.global_frame.set_var("errorCode", "NONE")
        self.global_frame.set_var("argc", "0")
        self.global_frame.set_var("argv", "")
        self.global_frame.set_var("argv0", "")

        # Pre-provide the Tcl package so init.tcl's `package require -exact tcl 9.0.3` succeeds.
        # Register under both "Tcl" (canonical) and "tcl" (init.tcl uses lowercase).
        _tcl_pkg: dict[str, str | bool | dict[str, str] | None] = {
            "version": "9.0.3",
            "loaded": True,
            "ifneeded": {},
        }
        self.packages["Tcl"] = _tcl_pkg
        self.packages["tcl"] = _tcl_pkg

        # Register the tcltest package so ``package require tcltest`` works
        from .commands.tcltest_cmds import setup_tcltest

        setup_tcltest(self)

        if source_init:
            self._source_init_tcl()

    # Init.tcl sourcing

    def _source_init_tcl(self) -> None:
        """Source the bundled Tcl init.tcl to set up package loading."""
        init_tcl = os.path.join(self._tcl_library, "init.tcl")
        if not os.path.exists(init_tcl):
            return  # Gracefully degrade if not found

        # Pre-create namespaces that init.tcl expects to exist
        from .scope import ensure_namespace

        # ::tcl::clock — needed by the clock ensemble setup
        ensure_namespace(self.root_namespace, "::tcl::clock")

        # ::tcl::unsupported::clock::configure — called by init.tcl
        unsup_ns = ensure_namespace(self.root_namespace, "::tcl::unsupported::clock")
        unsup_ns.register_command("configure", lambda interp, args: TclResult())

        old_script = self.script_file
        self.script_file = init_tcl
        try:
            with open(init_tcl) as f:
                script = f.read()
            self.eval(script)
        except Exception:
            pass  # Log but don't crash — the VM should work even without init.tcl
        finally:
            self.script_file = old_script

    # Evaluation

    @staticmethod
    def _collapse_continuations(source: str) -> str:
        """Collapse ``\\<newline><whitespace>`` sequences to a single space.

        Unlike the Tcl parser (which preserves ``\\<newline>`` inside
        braces), we collapse at ALL brace depths because our compiler
        processes the full source — including proc bodies — at parse
        time.  When Tcl encounters a proc body at runtime it re-parses
        the text and collapses continuations then; we must do the
        equivalent here.
        """
        result: list[str] = []
        i = 0
        n = len(source)
        while i < n:
            c = source[i]
            if c == "\\" and i + 1 < n and source[i + 1] == "\n":
                # Backslash-newline continuation: collapse
                i += 2
                while i < n and source[i] in " \t":
                    i += 1
                result.append(" ")
                continue
            if c == "\\" and i + 1 < n:
                # Other backslash — keep both chars
                result.append(c)
                result.append(source[i + 1])
                i += 2
                continue
            result.append(c)
            i += 1
        return "".join(result)

    def eval(self, source: str) -> TclResult:
        """Compile and execute Tcl *source*."""
        if not source.strip():
            return TclResult()

        # Collapse backslash-newline continuations at the script level
        # (the Tcl tokeniser does this; our lexer does not)
        if "\\\n" in source:
            source = self._collapse_continuations(source)

        from core.parsing.lexer import TclLexer

        cacheable = len(source) <= self._eval_cache_max_source_len
        cached = self._eval_cache.get(source) if cacheable else None

        if cached is not None:
            module_asm, ir_module = cached
            self._eval_cache.move_to_end(source)
        else:
            TclLexer.strict_quoting = True
            try:
                module_asm, ir_module = compile_script(source, optimise=self.optimise)
            except TclParseError as e:
                err = TclError(str(e))
                # Add "while executing" frame with the source text
                # (truncated to ~150 chars like Tcl does).
                display = source
                if len(display) > 150:
                    display = display[:150] + "..."
                err.error_info = [str(e), f'    while executing\n"{display}"']
                raise err from None
            finally:
                TclLexer.strict_quoting = False

            if cacheable:
                self._eval_cache[source] = (module_asm, ir_module)
                self._eval_cache.move_to_end(source)
                while len(self._eval_cache) > self._eval_cache_max_entries:
                    self._eval_cache.popitem(last=False)

        # Register procedures discovered during compilation (with compiled ASM)
        for qname, ir_proc in ir_module.procedures.items():
            compiled_asm = module_asm.procedures.get(qname)
            self._register_ir_proc(ir_proc.name, ir_proc, compiled_asm, source=source)

        # Execute the top-level bytecode
        vm = BytecodeVM(self, debug_hook=self._debug_hook)
        try:
            return vm.execute(module_asm.top_level)
        except TclReturn as ret:
            if ret.level <= 0:
                if ret.code == ReturnCode.ERROR:
                    err = TclError(ret.value)
                    if ret.error_info is not None:
                        err.error_info = [ret.error_info]
                    if ret.error_code is not None:
                        err.error_code = ret.error_code
                    raise err from None
                return TclResult(code=ReturnCode(ret.code), value=ret.value)
            # Propagate with decremented level
            raise TclReturn(
                value=ret.value,
                code=ret.code,
                level=ret.level - 1,
                error_info=ret.error_info,
                error_code=ret.error_code,
            ) from None

    def _register_ir_proc(
        self,
        short_name: str,
        ir_proc: object,
        compiled_asm: FunctionAsm | None,
        *,
        source: str = "",
    ) -> None:
        """Extract procedure definition from IR and register it."""
        from core.compiler.ir import IRProcedure

        if not isinstance(ir_proc, IRProcedure):
            return

        # Use params_raw for proper default-value parsing
        from .machine import _split_list

        params_with_defaults: list[tuple[str, str | None]] = []
        param_names: list[str] = []
        has_args = False

        raw_params = _split_list(ir_proc.params_raw) if ir_proc.params_raw else []
        if raw_params:
            for p in raw_params:
                parts = _split_list(p)
                if len(parts) == 1:
                    if parts[0] == "args":
                        has_args = True
                    param_names.append(parts[0])
                    params_with_defaults.append((parts[0], None))
                elif len(parts) >= 2:
                    param_names.append(parts[0])
                    params_with_defaults.append((parts[0], parts[1]))
        else:
            # Fallback to IR params (names only, no defaults)
            for p in ir_proc.params:
                if p == "args":
                    has_args = True
                param_names.append(p)
                params_with_defaults.append((p, None))

        # Extract body text from source if available
        body_text = ""
        if source and hasattr(ir_proc, "body"):
            stmts = ir_proc.body.statements if hasattr(ir_proc.body, "statements") else ()
            if stmts:
                first_range = stmts[0].range
                last_range = stmts[-1].range
                body_text = source[first_range.start.offset : last_range.end.offset + 1]

        # Determine the defining namespace.  The IR qualified name may
        # track the correct namespace when coming from the outer compilation
        # (e.g. ``::ns::foo`` from a ``namespace eval ns { proc foo ... }``
        # block), but an inner re-compilation of the body produces ``::foo``
        # since the compiler always starts at root.  In that case the
        # runtime current_namespace is the authoritative source.
        from .scope import ensure_namespace, namespace_qualifiers, namespace_tail

        ir_qname = getattr(ir_proc, "qualified_name", short_name)
        ir_ns = namespace_qualifiers(ir_qname) if "::" in ir_qname else ""
        cur_ns = self.current_namespace.qualname

        # If the IR says root but we're executing in a child namespace,
        # trust the runtime namespace.
        if (ir_ns == "" or ir_ns == "::") and cur_ns != "::":
            def_ns = cur_ns
            qname = f"{cur_ns}::{short_name}"
        elif "::" in ir_qname:
            def_ns = ir_ns if ir_ns else "::"
            qname = ir_qname
        elif "::" in short_name:
            ns_part = namespace_qualifiers(short_name)
            def_ns = ns_part if ns_part else "::"
            qname = short_name
        else:
            def_ns = cur_ns
            qname = short_name

        # The 0-based source line of the proc command (for errorInfo line calculation).
        proc_source_line = ir_proc.range.start.line if hasattr(ir_proc.range, "start") else 0

        proc_def = ProcDef(
            name=short_name,
            param_names=param_names,
            params_with_defaults=params_with_defaults,
            body=body_text,
            has_args=has_args,
            compiled_asm=compiled_asm,
            def_namespace=def_ns,
            body_start_line=proc_source_line,
        )
        # Store in flat table using the qualified name if available.
        # Also store under the short name only if this is a global proc
        # to avoid clobbering existing global entries.
        if "::" in qname:
            self.procedures[qname] = proc_def
            tail = namespace_tail(qname)
            ns = ensure_namespace(self.root_namespace, def_ns)
            ns.register_proc(tail, proc_def)
        elif def_ns == "::":
            self.procedures[short_name] = proc_def
        else:
            # Namespace proc with simple name — register in namespace table
            # but don't clobber the flat table if a global proc exists.
            if short_name not in self.procedures:
                self.procedures[short_name] = proc_def
            target_ns = ensure_namespace(self.root_namespace, def_ns)
            target_ns.register_proc(short_name, proc_def)

    # Runtime command table

    def register_command(self, name: str, handler: CommandHandler) -> None:
        """Register a runtime command (rename, ensemble, tcltest, etc.)."""
        self._runtime_commands[name] = handler

    def unregister_command(self, name: str) -> None:
        """Remove a runtime command.

        If the command also exists as a built-in on REGISTRY, a ``None``
        sentinel is stored so that ``lookup_command`` won't fall through
        to the global handler (e.g. after ``rename expr old_expr``).
        """
        from core.commands.registry import REGISTRY

        if REGISTRY.lookup_handler(name) is not None:
            self._runtime_commands[name] = None  # type: ignore[assignment]
        else:
            self._runtime_commands.pop(name, None)

    def lookup_command(self, name: str) -> CommandHandler | None:
        """Look up a command handler — runtime table first, then REGISTRY."""
        if name in self._runtime_commands:
            return self._runtime_commands[name]  # may be None (shadowed)
        from core.commands.registry import REGISTRY

        return REGISTRY.lookup_handler(name)

    def command_names(self) -> list[str]:
        """Return sorted list of all available command names."""
        from core.commands.registry import REGISTRY

        names: set[str] = set()
        for n, h in self._runtime_commands.items():
            if h is not None:
                names.add(n)
        for name in REGISTRY.specs_by_name:
            if name not in self._runtime_commands and REGISTRY.lookup_handler(name) is not None:
                names.add(name)
        return sorted(names)

    # Command dispatch

    def invoke(self, cmd_name: str, args: list[str]) -> TclResult:
        """Dispatch a command by name, annotating errors with errorInfo."""
        try:
            return self._invoke_inner(cmd_name, args)
        except RecursionError:
            msg = "too many nested evaluations (infinite loop?)"
            cmd_text = _format_cmd_text(cmd_name, args)
            raise TclError(msg, error_info=[msg, f'    while executing\n"{cmd_text}"']) from None
        except TclError as e:
            if not e.error_info:
                if self._error_cmd_text:
                    cmd_text = self._error_cmd_text
                    if len(cmd_text) > 150:
                        cmd_text = cmd_text[:150] + "..."
                else:
                    cmd_text = _format_cmd_text(cmd_name, args)
                e.error_info = [e.message, f'    while executing\n"{cmd_text}"']
            raise

    def _invoke_inner(self, cmd_name: str, args: list[str]) -> TclResult:
        """Dispatch a command by name (inner implementation)."""
        self.cmd_count += 1

        # Fully-qualified names (::foo::bar) resolve from root
        if "::" in cmd_name:
            # Try namespace-registered procs/commands
            ns_part = cmd_name[: cmd_name.rfind("::")]
            tail = cmd_name[cmd_name.rfind("::") + 2 :]
            ns = resolve_namespace(self.root_namespace, ns_part if ns_part else "::")
            if ns is not None:
                proc = ns.lookup_proc(tail)
                if proc is not None:
                    return self._call_proc(proc, args)
                handler = ns.lookup_command(tail)
                if handler is not None:
                    return handler(self, args)

            # Resolve ::tcl::<base>::<sub> ensemble names emitted
            # by the bytecode compiler (e.g. ::tcl::dict::append).
            _fq = _FQ_ENSEMBLE_RE.match(cmd_name)
            if _fq:
                return self.invoke(_fq.group(1), [_fq.group(2)] + args)

            # For fully-qualified names, also check the global flat tables
            # (some procs are registered in the flat procedures dict)
            proc = self.procedures.get(cmd_name)
            if proc is not None:
                return self._call_proc(proc, args)

            # Also try the tail as a built-in command (e.g. ::set → set)
            handler = self.lookup_command(tail)
            if handler is not None:
                return handler(self, args)

            # Also try the tail as a user-defined procedure
            proc = self.procedures.get(tail)
            if proc is not None:
                return self._call_proc(proc, args)

        # Current namespace procs (checked before global so namespace-local
        # procs shadow global ones, matching Tcl lookup order)
        if self.current_namespace is not self.root_namespace:
            ns_proc = self.current_namespace.lookup_proc(cmd_name)
            if isinstance(ns_proc, ProcDef):
                return self._call_proc(ns_proc, args)

            # Current namespace commands (e.g. from ``namespace import``)
            ns_handler = self.current_namespace.lookup_command(cmd_name)
            if callable(ns_handler):
                return ns_handler(self, args)

        # Built-in commands (global registry)
        handler = self.lookup_command(cmd_name)
        if handler is not None:
            return handler(self, args)

        # User-defined procedures (flat table)
        proc = self.procedures.get(cmd_name)
        if proc is not None:
            return self._call_proc(proc, args)

        # Root namespace procs / commands (global namespace fallback).
        # In Tcl, non-qualified command resolution goes:
        # current namespace → global namespace → unknown handler.
        ns_proc = self.root_namespace.lookup_proc(cmd_name)
        if ns_proc is not None:
            return self._call_proc(ns_proc, args)

        ns_handler = self.root_namespace.lookup_command(cmd_name)
        if ns_handler is not None:
            return ns_handler(self, args)

        # Try unknown handler (for auto-loading).  User-defined ``unknown``
        # procs may be stored under the qualified key ``::unknown``.
        unknown_proc = self.procedures.get("unknown") or self.procedures.get("::unknown")
        if unknown_proc is None:
            unknown_proc = self.root_namespace.lookup_proc("unknown")
        if isinstance(unknown_proc, ProcDef):
            return self._call_proc(unknown_proc, [cmd_name] + args)

        unknown = self.lookup_command("unknown")
        if unknown is not None:
            return unknown(self, [cmd_name] + args)

        raise TclError(f'invalid command name "{cmd_name}"')

    def call_proc_by_name(self, name: str, args: list[str]) -> TclResult:
        """Call a user-defined procedure by name."""
        proc = self.procedures.get(name)
        if proc is None:
            raise TclError(f'invalid command name "{name}"')
        return self._call_proc(proc, args)

    def _call_proc(self, proc: ProcDef, args: list[str]) -> TclResult:
        """Execute a user-defined procedure with the given arguments."""
        # Determine the namespace for this proc's execution context.
        # Each proc remembers the namespace where it was defined.
        proc_ns = resolve_namespace(self.root_namespace, proc.def_namespace)
        if proc_ns is None:
            proc_ns = self.root_namespace

        frame = CallFrame(
            level=self.current_frame.level + 1,
            proc_name=proc.name,
            parent=self.current_frame,
            namespace=proc_ns,
            interp=self,
            call_args=args,
        )

        # Bind parameters — Tcl binds sequentially; only trailing
        # params with defaults may be omitted.
        all_params = [(n, d) for n, d in proc.params_with_defaults if n != "args"]
        trailing_optional = 0
        for _n, _d in reversed(all_params):
            if _d is not None:
                trailing_optional += 1
            else:
                break
        min_args = len(all_params) - trailing_optional

        if len(args) < min_args or (not proc.has_args and len(args) > len(all_params)):
            # Build usage string with ?param? for optional and ?arg ...? for args
            parts: list[str] = []
            for n, d in proc.params_with_defaults:
                if n == "args":
                    parts.append("?arg ...?")
                elif d is not None:
                    parts.append(f"?{n}?")
                else:
                    parts.append(n)
            usage = " ".join(parts)
            if usage:
                usage = " " + usage
            raise TclError(f'wrong # args: should be "{proc.name}{usage}"')

        # Sequential assignment: fill params left-to-right with args,
        # then use defaults for any remaining trailing params.
        arg_idx = 0
        for name, default in all_params:
            if arg_idx < len(args):
                frame.set_var(name, args[arg_idx])
                arg_idx += 1
            elif default is not None:
                frame.set_var(name, default)

        if proc.has_args:
            from .machine import _list_escape

            remaining = args[arg_idx:]
            frame.set_var("args", " ".join(_list_escape(a) for a in remaining))

        # Execute the body in the new frame
        saved_frame = self.current_frame
        saved_ns = self.current_namespace
        self.current_frame = frame
        self.current_namespace = proc_ns
        try:
            if proc.compiled_asm is not None:
                vm = BytecodeVM(self, debug_hook=self._debug_hook)
                result = vm.execute(proc.compiled_asm)
            else:
                result = self.eval(proc.body)
            return TclResult(value=result.value)
        except TclError as e:
            # Add "(procedure X line N) / invoked from within" frame
            self._annotate_proc_error(e, proc, args)
            raise
        except TclBreak:
            # break escaped the proc body — convert to an error.
            # Tcl reports line 1 for break/continue outside a loop.
            saved_el = self._error_line
            self._error_line = 0
            e = TclError('invoked "break" outside of a loop')
            e.error_info = [e.message]
            self._annotate_proc_error(e, proc, args, line_fallback=1)
            self._error_line = saved_el
            raise e from None
        except TclContinue:
            # continue escaped the proc body — convert to an error.
            saved_el = self._error_line
            self._error_line = 0
            e = TclError('invoked "continue" outside of a loop')
            e.error_info = [e.message]
            self._annotate_proc_error(e, proc, args, line_fallback=1)
            self._error_line = saved_el
            raise e from None
        except TclReturn as ret:
            if ret.level > 1:
                raise TclReturn(value=ret.value, code=ret.code, level=ret.level - 1) from None
            # Level <= 1: the proc absorbs the return
            val = ret.value or ""
            code = ret.code
            if code == ReturnCode.OK or code == 0:
                return TclResult(value=val)
            if code == ReturnCode.ERROR or code == 1:
                err = TclError(val)
                if ret.error_code is not None:
                    err.error_code = ret.error_code
                # return -code error generates errorInfo at the call site
                cmd_text = _format_cmd_text(proc.name, args)
                if ret.error_info is not None:
                    err.error_info = [ret.error_info]
                else:
                    err.error_info = [val]
                err.error_info.append(
                    f'    while executing\n"{cmd_text}"',
                )
                raise err from None
            # For RETURN, BREAK, CONTINUE, and custom codes: re-raise
            # with level=0 so eval()/catch() can see the completion code.
            raise TclReturn(
                value=val,
                code=code,
                level=0,
                error_info=ret.error_info,
                error_code=ret.error_code,
            ) from None
        finally:
            self.current_frame = saved_frame
            self.current_namespace = saved_ns

    def _annotate_proc_error(
        self,
        e: TclError,
        proc: ProcDef,
        args: list[str],
        *,
        line_fallback: int = 1,
    ) -> None:
        """Add procedure context frame to a TclError's error_info.

        If the error has no existing errorInfo (e.g. from a compiled
        ``error`` command via RETURN_IMM), adds the initial message.
        Then appends the ``(procedure "X" line N)`` and
        ``invoked from within`` frames.
        """
        # Ensure the error message is the first entry
        if not e.error_info:
            e.error_info = [e.message]

        # Compute relative line within the proc body.
        # When compiled_asm is used, _error_line is in the outer script's
        # coordinate system, so we subtract body_start_line.  When the body
        # is eval'd as a standalone script, _error_line is already relative
        # to the body text and should be used directly.
        rel_line = line_fallback
        if self._error_line > 0:
            if proc.compiled_asm is not None:
                computed = self._error_line - proc.body_start_line
            else:
                computed = self._error_line
            if computed > 0:
                rel_line = computed

        cmd_text = _format_cmd_text(proc.name, args)
        e.error_info.append(f'    (procedure "{proc.name}" line {rel_line})')
        e.error_info.append(f'    invoked from within\n"{cmd_text}"')

    # Procedure definition

    def define_proc(self, name: str, param_str: str, body: str) -> None:
        """Define a new procedure."""
        from .machine import _split_list

        raw_params = _split_list(param_str)
        params_with_defaults: list[tuple[str, str | None]] = []
        param_names: list[str] = []
        has_args = False

        for p in raw_params:
            parts = _split_list(p)
            if len(parts) == 1:
                pname = parts[0]
                if not pname:
                    raise TclError("argument with no name")
                if "(" in pname and pname.endswith(")"):
                    raise TclError(f'formal parameter "{pname}" is an array element')
                if "::" in pname:
                    raise TclError(f'formal parameter "{pname}" is not a simple name')
                if pname == "args":
                    has_args = True
                param_names.append(pname)
                params_with_defaults.append((pname, None))
            elif len(parts) == 2:
                pname = parts[0]
                if not pname:
                    raise TclError("argument with no name")
                if "(" in pname and pname.endswith(")"):
                    raise TclError(f'formal parameter "{pname}" is an array element')
                if "::" in pname:
                    raise TclError(f'formal parameter "{pname}" is not a simple name')
                param_names.append(pname)
                params_with_defaults.append((pname, parts[1]))
            elif len(parts) > 2:
                raise TclError(f'too many fields in argument specifier "{p}"')
            else:
                raise TclError("argument with no name")

        # Qualify name with current namespace when not already qualified
        if name.startswith("::"):
            qualified = name
        elif "::" in name:
            # Namespace-relative name like "tcltest::test" → "::tcltest::test"
            qualified = "::" + name
        elif self.current_namespace is not self.root_namespace:
            qualified = self.current_namespace.qualname + "::" + name
        else:
            qualified = name

        # Preserve compiled bytecode if the proc was already registered
        # from the IR pass (which has compiled ASM) AND the body matches.
        existing = self.procedures.get(qualified) or self.procedures.get(name)
        compiled_asm = None
        if existing is not None and existing.compiled_asm is not None and existing.body == body:
            compiled_asm = existing.compiled_asm

        # Determine the defining namespace for proc execution context
        if "::" in qualified:
            ns_part = qualified[: qualified.rfind("::")]
            def_ns = ns_part if ns_part else "::"
        else:
            def_ns = self.current_namespace.qualname

        # Preserve body_start_line from the existing IR-registered proc,
        # or fall back to the current _error_line (the proc command's line).
        bsl = 0
        if existing is not None and existing.body_start_line:
            bsl = existing.body_start_line
        elif self._error_line:
            bsl = self._error_line - 1  # convert 1-based to 0-based

        proc_def = ProcDef(
            name=qualified,
            param_names=param_names,
            params_with_defaults=params_with_defaults,
            body=body,
            has_args=has_args,
            compiled_asm=compiled_asm,
            def_namespace=def_ns,
            body_start_line=bsl,
        )
        self.procedures[qualified] = proc_def
        # If we qualified the name, update the unqualified entry only
        # if it was pre-registered by the IR pass for this SAME namespace.
        # Do not clobber a global proc with the same simple name.
        if qualified != name and name in self.procedures:
            existing_simple = self.procedures[name]
            if existing_simple.def_namespace == def_ns:
                self.procedures[name] = proc_def
        # Register in the appropriate namespace's proc table
        if "::" in qualified:
            # Fully-qualified name — register in the target namespace
            tail = qualified[qualified.rfind("::") + 2 :]
            target_ns = ensure_namespace(self.root_namespace, def_ns)
            target_ns.register_proc(tail, proc_def)
        elif self.current_namespace is not self.root_namespace:
            self.current_namespace.register_proc(name, proc_def)

    # Expression evaluation

    def eval_expr(self, expr_str: str) -> str:
        """Parse and evaluate a Tcl expression string."""
        node = parse_expr(expr_str)
        if isinstance(node, ExprRaw):
            # ExprRaw means the parser could not parse the expression.
            # Empty string is a valid edge case (returns "").
            raw = expr_str.strip()
            if not raw:
                # Truly empty string: return empty (matches expr {})
                # Whitespace-only: syntax error (matches Tcl_ExprString(" "))
                if expr_str:
                    raise TclError(f'syntax error in expression "{expr_str}"')
                return expr_str
            # Try to interpret as a simple numeric or boolean literal
            try:
                return str(int(raw, 0))
            except (ValueError, TypeError):
                pass
            try:
                import math as _math

                from .machine import _format_number

                fv = float(raw)
                # NaN/Inf are not valid expression literals in Tcl —
                # they can only be produced by operations.
                if _math.isnan(fv):
                    raise TclError(
                        "domain error: argument not in valid range",
                        error_code="ARITH DOMAIN {domain error: argument not in valid range}",
                    )
                return _format_number(fv)
            except (ValueError, TypeError):
                pass
            low = raw.lower()
            if low in ("true", "yes", "on"):
                return "1"
            if low in ("false", "no", "off"):
                return "0"
            # Not a valid literal — raise a syntax error
            # Truncate long expressions in the error message
            display = expr_str if len(expr_str) <= 60 else expr_str[:60] + "..."
            raise TclError(f'syntax error in expression "{display}"')
        result = self._eval_expr_node(node)
        return self._try_cvt_to_numeric(result)

    @staticmethod
    def _try_cvt_to_numeric(val: str) -> str:
        """Canonicalize a string to its numeric form if it looks numeric.

        Mirrors the TRY_CVT_TO_NUMERIC opcode: 0x1234→4660, 0o123→83,
        trailing-zero floats 3.1200000→3.12, leading zeros 00005→5, etc.
        """
        stripped = val.strip()
        if not stripped:
            return val
        try:
            return str(int(stripped, 0))
        except (ValueError, TypeError):
            pass
        # Tcl 9.0 doesn't use 0-prefix for octal (only 0o), so
        # try decimal for leading zeros like 000005
        try:
            return str(int(stripped, 10))
        except (ValueError, TypeError):
            pass
        try:
            import math

            fv = float(stripped)
            if math.isinf(fv):
                return "-Inf" if fv < 0 else "Inf"
            if math.isnan(fv):
                return "NaN"
            return repr(fv)
        except (ValueError, TypeError):
            pass
        return val

    @staticmethod
    def _validate_numeric_literal(text: str) -> None:
        """Raise TclError for invalid prefix literals like 0o289."""
        import re

        s = text.strip().lstrip("+-")
        if re.match(r"0[oO]", s):
            try:
                int(text.strip(), 0)
            except ValueError:
                raise TclError(
                    f'invalid octal number "{text}"',
                    error_code=f"TCL VALUE NUMBER {{{text}}}",
                ) from None
        elif re.match(r"0[xX]", s):
            try:
                int(text.strip(), 0)
            except ValueError:
                raise TclError(
                    f'invalid hexadecimal number "{text}"',
                    error_code=f"TCL VALUE NUMBER {{{text}}}",
                ) from None
        elif re.match(r"0[bB]", s):
            try:
                int(text.strip(), 0)
            except ValueError:
                raise TclError(
                    f'invalid binary number "{text}"',
                    error_code=f"TCL VALUE NUMBER {{{text}}}",
                ) from None

    def _eval_expr_node(self, node: ExprNode) -> str:
        """Walk an expression AST, resolving variables and commands."""
        match node:
            case ExprLiteral(text=text):
                # Validate numeric literals — reject invalid prefix literals
                # like 0o289 (invalid octal digit 8, 9)
                self._validate_numeric_literal(text)
                return text

            case ExprString(text=text):
                # Strip surrounding delimiters — the parser keeps them
                if len(text) >= 2 and text[0] == '"' and text[-1] == '"':
                    return text[1:-1]
                if len(text) >= 2 and text[0] == "{" and text[-1] == "}":
                    content = text[1:-1]
                    # Collapse backslash-newline continuations inside braces
                    if "\\\n" in content:
                        from core.parsing.substitution import backslash_subst

                        content = backslash_subst(content)
                    return content
                return text

            case ExprVar(text=text, name=name):
                # text includes $ and optional array index: $arr(key)
                var_ref = text.lstrip("$") if "(" in text else name
                return self.current_frame.get_var(var_ref)

            case ExprBinary(op=op, left=left, right=right):
                return self._eval_binary(op, left, right)

            case ExprUnary(op=op, operand=operand):
                return self._eval_unary(op, operand)

            case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
                cond_val = self._eval_expr_node(cond)
                if _tcl_bool(cond_val):
                    return self._eval_expr_node(tb)
                return self._eval_expr_node(fb)

            case ExprCall(function=func, args=call_args):
                return self._eval_math_func(func, call_args)

            case ExprCommand(text=text):
                # Strip surrounding brackets — ExprCommand.text includes them
                cmd_text = text
                if cmd_text.startswith("[") and cmd_text.endswith("]"):
                    cmd_text = cmd_text[1:-1]
                result = self.eval(cmd_text)
                return result.value

            case _:
                return str(node)

    def _eval_binary(self, op: BinOp, left: ExprNode, right: ExprNode) -> str:
        """Evaluate a binary expression."""
        # Short-circuit for && and ||
        if op == BinOp.AND:
            lv = self._eval_expr_node(left)
            if not _tcl_bool(lv):
                return "0"
            rv = self._eval_expr_node(right)
            return "1" if _tcl_bool(rv) else "0"
        if op == BinOp.OR:
            lv = self._eval_expr_node(left)
            if _tcl_bool(lv):
                return "1"
            rv = self._eval_expr_node(right)
            return "1" if _tcl_bool(rv) else "0"

        lv = self._eval_expr_node(left)
        rv = self._eval_expr_node(right)

        # String comparison operators
        if op in (
            BinOp.STR_EQ,
            BinOp.STR_NE,
            BinOp.STR_LT,
            BinOp.STR_GT,
            BinOp.STR_LE,
            BinOp.STR_GE,
        ):
            match op:
                case BinOp.STR_EQ:
                    return "1" if lv == rv else "0"
                case BinOp.STR_NE:
                    return "1" if lv != rv else "0"
                case BinOp.STR_LT:
                    return "1" if lv < rv else "0"
                case BinOp.STR_GT:
                    return "1" if lv > rv else "0"
                case BinOp.STR_LE:
                    return "1" if lv <= rv else "0"
                case BinOp.STR_GE:
                    return "1" if lv >= rv else "0"
            return "0"

        # in / ni operators
        if op == BinOp.IN:
            from .machine import _split_list

            elements = _split_list(rv)
            return "1" if lv in elements else "0"
        if op == BinOp.NI:
            from .machine import _split_list

            elements = _split_list(rv)
            return "1" if lv not in elements else "0"

        # Numeric operations
        from .machine import _arith_binary, _bitwise_binary, _compare

        match op:
            case BinOp.ADD:
                return _arith_binary(lv, rv, "add")
            case BinOp.SUB:
                return _arith_binary(lv, rv, "sub")
            case BinOp.MUL:
                return _arith_binary(lv, rv, "mult")
            case BinOp.DIV:
                return _arith_binary(lv, rv, "div")
            case BinOp.MOD:
                return _arith_binary(lv, rv, "mod")
            case BinOp.POW:
                return _arith_binary(lv, rv, "expon")
            case BinOp.LSHIFT:
                return _bitwise_binary(lv, rv, "lshift")
            case BinOp.RSHIFT:
                return _bitwise_binary(lv, rv, "rshift")
            case BinOp.BIT_AND:
                return _bitwise_binary(lv, rv, "bitand")
            case BinOp.BIT_OR:
                return _bitwise_binary(lv, rv, "bitor")
            case BinOp.BIT_XOR:
                return _bitwise_binary(lv, rv, "bitxor")
            case BinOp.EQ:
                return _compare(lv, rv, "eq")
            case BinOp.NE:
                return _compare(lv, rv, "neq")
            case BinOp.LT:
                return _compare(lv, rv, "lt")
            case BinOp.GT:
                return _compare(lv, rv, "gt")
            case BinOp.LE:
                return _compare(lv, rv, "le")
            case BinOp.GE:
                return _compare(lv, rv, "ge")
            case _:
                raise TclError(f"unsupported binary op: {op}")

    def _eval_unary(self, op: UnaryOp, operand: ExprNode) -> str:
        """Evaluate a unary expression."""
        from .machine import _format_number, _parse_int_unary, _parse_number_unary

        val = self._eval_expr_node(operand)
        match op:
            case UnaryOp.NEG:
                return _format_number(-_parse_number_unary(val, "uminus"))
            case UnaryOp.POS:
                return _format_number(_parse_number_unary(val, "uplus"))
            case UnaryOp.BIT_NOT:
                return str(~_parse_int_unary(val, "bitnot"))
            case UnaryOp.NOT:
                return "0" if _tcl_bool(val) else "1"
            case _:
                raise TclError(f"unsupported unary op: {op}")

    def _eval_math_func(self, func: str, args: list[ExprNode]) -> str:
        """Evaluate a math function call."""
        from .machine import _format_number, _parse_number

        vals = [self._eval_expr_node(a) for a in args]

        try:
            return self._dispatch_math_func(func, vals, _format_number, _parse_number)
        except TclError:
            raise
        except (ValueError, OverflowError, ZeroDivisionError) as exc:
            from .types import _arith_error_from_exc

            raise _arith_error_from_exc(exc) from None
        except (TypeError, IndexError) as exc:
            raise TclError(str(exc)) from None

    def _dispatch_math_func(
        self,
        func: str,
        vals: list[str],
        _format_number: Callable[[int | float], str],
        _parse_number: Callable[[str], int | float],
    ) -> str:
        """Dispatch to the appropriate math function implementation."""
        # Arity checking (matches C Tcl error messages)
        _ARITY: dict[str, tuple[int, int]] = {
            # func: (min_args, max_args)  — -1 means variadic
            "abs": (1, 1),
            "acos": (1, 1),
            "asin": (1, 1),
            "atan": (1, 1),
            "atan2": (2, 2),
            "bool": (1, 1),
            "ceil": (1, 1),
            "cos": (1, 1),
            "cosh": (1, 1),
            "double": (1, 1),
            "entier": (1, 1),
            "exp": (1, 1),
            "floor": (1, 1),
            "fmod": (2, 2),
            "hypot": (2, 2),
            "int": (1, 1),
            "isqrt": (1, 1),
            "log": (1, 1),
            "log10": (1, 1),
            "max": (1, -1),
            "min": (1, -1),
            "pow": (2, 2),
            "rand": (0, 0),
            "round": (1, 1),
            "sin": (1, 1),
            "sinh": (1, 1),
            "sqrt": (1, 1),
            "srand": (1, 1),
            "tan": (1, 1),
            "tanh": (1, 1),
            "wide": (1, 1),
        }
        if func in _ARITY:
            lo, hi = _ARITY[func]
            nargs = len(vals)
            if nargs < lo:
                raise TclError(f'not enough arguments for math function "{func}"')
            if hi != -1 and nargs > hi:
                raise TclError(f'too many arguments for math function "{func}"')

        match func:
            case "abs":
                return _format_number(abs(_parse_number(vals[0])))
            case "acos":
                v = float(vals[0])
                if math.isnan(v):
                    raise ValueError("domain error: argument not in valid range")
                return _format_number(math.acos(v))
            case "asin":
                return _format_number(math.asin(float(vals[0])))
            case "atan":
                return _format_number(math.atan(float(vals[0])))
            case "atan2":
                return _format_number(math.atan2(float(vals[0]), float(vals[1])))
            case "bool":
                return "1" if _tcl_bool(vals[0]) else "0"
            case "ceil":
                return _format_number(int(math.ceil(float(vals[0]))))
            case "cos":
                return _format_number(math.cos(float(vals[0])))
            case "cosh":
                return _format_number(math.cosh(float(vals[0])))
            case "double":
                return _format_number(float(vals[0]))
            case "entier":
                return str(int(float(vals[0])))
            case "exp":
                try:
                    return _format_number(math.exp(float(vals[0])))
                except OverflowError:
                    return "Inf" if float(vals[0]) > 0 else "0.0"
            case "floor":
                return _format_number(int(math.floor(float(vals[0]))))
            case "fmod":
                return _format_number(math.fmod(float(vals[0]), float(vals[1])))
            case "hypot":
                try:
                    return _format_number(math.hypot(float(vals[0]), float(vals[1])))
                except ValueError:
                    for v in vals:
                        try:
                            float(v)
                        except (ValueError, TypeError):
                            if " " in v.strip():
                                raise TclError(
                                    "expected floating-point number but got a list"
                                ) from None
                            raise TclError(
                                f'expected floating-point number but got "{v}"'
                            ) from None
                    raise
            case "int":
                return str(int(float(vals[0])))
            case "isqrt":
                return str(math.isqrt(int(vals[0])))
            case "log":
                return _format_number(math.log(float(vals[0])))
            case "log10":
                return _format_number(math.log10(float(vals[0])))
            case "max":
                nums = [_parse_number(v) for v in vals]
                result = max(nums)
                if isinstance(result, int):
                    return str(result)
                return _format_number(result)
            case "min":
                nums = [_parse_number(v) for v in vals]
                result = min(nums)
                if isinstance(result, int):
                    return str(result)
                return _format_number(result)
            case "pow":
                try:
                    return _format_number(math.pow(float(vals[0]), float(vals[1])))
                except OverflowError:
                    base = float(vals[0])
                    exp = float(vals[1])
                    if base < 0 and exp == int(exp) and int(exp) % 2 == 1:
                        return "-Inf"
                    return "Inf"
            case "rand":
                import random

                return _format_number(random.random())
            case "round":
                # Tcl round ties away from zero
                v = float(vals[0])
                if v >= 0:
                    return str(int(math.floor(v + 0.5)))
                return str(int(math.ceil(v - 0.5)))
            case "sin":
                return _format_number(math.sin(float(vals[0])))
            case "sinh":
                return _format_number(math.sinh(float(vals[0])))
            case "sqrt":
                return _format_number(math.sqrt(float(vals[0])))
            case "srand":
                import random

                random.seed(int(vals[0]))
                return _format_number(random.random())
            case "tan":
                return _format_number(math.tan(float(vals[0])))
            case "tanh":
                return _format_number(math.tanh(float(vals[0])))
            case "wide":
                v = vals[0].strip()
                try:
                    return str(int(v, 0))
                except (ValueError, TypeError):
                    pass
                try:
                    return str(int(v, 10))
                except (ValueError, TypeError):
                    pass
                fv = float(v)
                try:
                    iv = int(fv)
                    if iv < -(2**63) or iv >= 2**63:
                        iv = iv % (2**64)
                        if iv >= 2**63:
                            iv -= 2**64
                    return str(iv)
                except (ValueError, OverflowError):
                    raise ValueError(f'expected integer but got "{vals[0]}"') from None
            case _:
                raise TclError(f'unknown math function "{func}"')

    # Frame navigation

    def frame_at_level(self, level: int, display: str | None = None) -> CallFrame:
        """Return the call frame at absolute *level*.

        *display* is the original level string for error messages
        (e.g. ``"#2"``); if omitted, the numeric *level* is shown.
        """
        frame = self.current_frame
        while frame is not None:
            if frame.level == level:
                return frame
            frame = frame.parent
        label = display if display is not None else str(level)
        raise TclError(f'bad level "{label}"')

    def frame_at_relative(self, rel: int, display: str | None = None) -> CallFrame:
        """Return the call frame *rel* levels up from current.

        *display* is the original level string for error messages;
        if omitted, the numeric *rel* is shown.
        """
        label = display if display is not None else str(rel)
        if rel < 0:
            raise TclError(f'bad level "{label}"')
        frame = self.current_frame
        for _ in range(rel):
            if frame.parent is None:
                raise TclError(f'bad level "{label}"')
            frame = frame.parent
        return frame

    # Completeness check (for REPL)

    def is_complete(self, source: str) -> bool:
        """Return True if *source* is a complete Tcl script.

        Implements Tcl's ``Tcl_CommandComplete`` semantics: the script
        has no unclosed braces, brackets, quotes, or backslash-newline
        continuations.  Comments (``#`` at command position) suppress
        brace/bracket counting until end-of-line, and backslash-newline
        at end-of-line (including inside comments) makes the script
        incomplete.
        """
        return _is_complete_scan(source, 0, len(source), 0) is not None


# is_complete helper functions
#
# These implement a recursive-descent scanner that mirrors Tcl's
# Tcl_CommandComplete / Tcl_ParseCommand behaviour.  Each function
# returns the position *after* the consumed content, or ``None`` if
# the script is incomplete (unclosed delimiter or trailing continuation).

_WORD_TERM = frozenset(" \t\r\n;")


def _is_complete_scan(src: str, i: int, n: int, bracket_depth: int) -> int | None:
    """Scan commands from *i* to *n*.  Return end position or ``None``."""
    while i < n:
        # Skip horizontal whitespace
        while i < n and src[i] in " \t\r":
            i += 1
        if i >= n:
            break

        c = src[i]

        # ']' closes a bracket context
        if c == "]":
            if bracket_depth > 0:
                return i + 1
            # Stray ']' at top level — literal
            i += 1
            continue

        # Newline — command separator
        if c == "\n":
            i += 1
            continue

        # Semicolon — command separator
        if c == ";":
            i += 1
            continue

        # Comment ('#' at command position)
        if c == "#":
            result = _scan_comment(src, i, n)
            if result is None:
                return None
            i = result
            continue

        # Scan one command (sequence of words)
        result = _scan_cmd_words(src, i, n, bracket_depth)
        if result is None:
            return None
        i = result

    if bracket_depth > 0:
        return None  # unclosed bracket
    return i


def _scan_comment(src: str, i: int, n: int) -> int | None:
    """Scan a comment starting at ``#``.  Return position after, or ``None``."""
    i += 1  # skip '#'
    while i < n:
        c = src[i]
        if c == "\n":
            return i + 1
        if c == "\\":
            if i + 1 >= n:
                return i + 1  # trailing backslash in comment is OK
            if src[i + 1] == "\n":
                # Backslash-newline continues the comment
                i += 2
                if i >= n:
                    return None  # continuation at EOF → incomplete
                continue
            i += 2  # skip escaped character
            continue
        i += 1
    return i  # EOF ends comment (complete)


def _scan_cmd_words(src: str, i: int, n: int, bracket_depth: int) -> int | None:
    """Scan words of one command.  Return position after, or ``None``."""
    while i < n:
        # Skip horizontal whitespace
        while i < n and src[i] in " \t\r":
            i += 1
        if i >= n:
            break

        c = src[i]

        # Command separators — return without consuming
        if c == "\n" or c == ";":
            return i

        # ']' at bracket depth — return without consuming
        if c == "]" and bracket_depth > 0:
            return i

        # Scan one word
        result = _scan_word(src, i, n, bracket_depth)
        if result is None:
            return None
        i = result

    return i


def _scan_word(src: str, i: int, n: int, bracket_depth: int) -> int | None:
    """Scan one word.  Dispatches to quoted/braced/bare scanners."""
    c = src[i]

    if c == '"':
        quoted_i = _scan_quoted(src, i, n, bracket_depth)
        if quoted_i is None:
            return None
        i = quoted_i
    elif c == "{":
        braced_i = _scan_braced(src, i, n)
        if braced_i is None:
            return None
        i = braced_i

    # Continue as bare word (handles extra chars after close-brace/quote,
    # or starts a bare word from scratch if neither branch above fired).
    return _scan_bare_word(src, i, n, bracket_depth)


def _scan_quoted(src: str, i: int, n: int, bracket_depth: int) -> int | None:
    """Scan a quoted string ``"..."``.  Return position after closing ``"``."""
    i += 1  # skip opening '"'
    while i < n:
        c = src[i]
        if c == '"':
            return i + 1
        if c == "\\":
            if i + 1 >= n:
                return None  # unclosed quote with trailing backslash
            if src[i + 1] == "\n":
                if i + 2 >= n:
                    return None  # continuation at EOF
                i += 2
                # Skip whitespace after continuation
                while i < n and src[i] in " \t":
                    i += 1
                continue
            i += 2
            continue
        if c == "[":
            # Command substitution inside quotes — recurse
            result = _is_complete_scan(src, i + 1, n, bracket_depth + 1)
            if result is None:
                return None
            i = result
            continue
        i += 1
    return None  # EOF without closing quote


def _scan_braced(src: str, i: int, n: int) -> int | None:
    """Scan a braced word ``{...}``.  Return position after closing ``}``."""
    depth = 1
    i += 1  # skip opening '{'
    while i < n and depth > 0:
        c = src[i]
        if c == "\\":
            # Escaped character — skip (prevents \{ or \} from counting)
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        i += 1
    if depth > 0:
        return None  # unclosed brace
    return i


def _scan_bare_word(src: str, i: int, n: int, bracket_depth: int) -> int | None:
    """Scan a bare (unquoted, unbraced) word or trailing chars after quote/brace."""
    while i < n:
        c = src[i]

        # Word terminators
        if c in _WORD_TERM:
            break
        if c == "]" and bracket_depth > 0:
            break

        # Backslash
        if c == "\\":
            if i + 1 >= n:
                # Trailing backslash — literal, word ends
                return i + 1
            if src[i + 1] == "\n":
                if i + 2 >= n:
                    return None  # continuation at EOF → incomplete
                # Continuation ends this word (acts as whitespace)
                i += 2
                while i < n and src[i] in " \t":
                    i += 1
                return i
            # Skip escaped character
            i += 2
            continue

        # Command substitution
        if c == "[":
            result = _is_complete_scan(src, i + 1, n, bracket_depth + 1)
            if result is None:
                return None
            i = result
            continue

        # Variable reference — check for $name( array subscript
        if c == "$":
            i += 1
            if i < n and src[i] == "{":
                # ${name} form — find closing brace
                end = src.find("}", i + 1)
                if end == -1:
                    return None  # unclosed ${
                i = end + 1
                continue
            # $name or $name(index) — scan identifier chars
            var_start = i
            while i < n and (src[i].isalnum() or src[i] in "_:"):
                i += 1
            if i > var_start and i < n and src[i] == "(":
                # Array subscript — find matching ')'
                paren_depth = 1
                i += 1
                while i < n and paren_depth > 0:
                    pc = src[i]
                    if pc == "(":
                        paren_depth += 1
                    elif pc == ")":
                        paren_depth -= 1
                    elif pc == "\\":
                        i += 1
                    i += 1
                if paren_depth > 0:
                    return None  # unclosed array subscript
            continue

        # Ordinary character (including mid-word `"`, `{`, `}`)
        i += 1

    return i


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
