"""_WasmEmitterValuesMixin: string/value boxing and argument emission."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _WasmEmitterBase as _Base
else:
    _Base = object

from collections.abc import Callable

from .....parsing.substitution import backslash_subst as _tcl_backslash_subst
from .._encoding import (
    _tcl_list_quote,
)
from .._imports import (
    runtime_import_for,
    subcommand_runtime_import_for,
)
from .._ir import (
    ValType,
)
from .._ownership import Ownership
from ._ops import _is_end_relative_index


def _contains_expand_prefix(text: str) -> bool:
    """True when *text* contains a Tcl ``{*}`` argument-expansion prefix.

    The Tcl 8.5+ syntax requires ``{*}`` to sit at a word boundary
    (the byte before is whitespace, ``;``, or the start of the script)
    and the byte after must be non-whitespace — that's what tells the
    parser ``{*}`` is the magic prefix and not a literal three-char
    brace word.  Mirrors the lexer's check in
    :meth:`core.parsing.lexer.TclLexer._parse_string`.
    """
    n = len(text)
    i = 0
    while True:
        idx = text.find("{*}", i)
        if idx < 0:
            return False
        # Boundary check on the LEFT — start of script, or after
        # whitespace / ``;``.
        if idx > 0 and text[idx - 1] not in " \t\n\r\x0b\x0c;":
            i = idx + 1
            continue
        # Boundary check on the RIGHT — the next char must exist
        # (so the word can be expanded) and be non-whitespace.
        right = idx + 3
        if right < n and text[right] not in " \t\n\r\x0b\x0c;":
            return True
        i = idx + 1
        if i >= n:
            return False


def _looks_like_string_option(arg: str) -> bool:
    """Detect a leading ``-flag`` argument on a ``string`` sub-command.

    Used to bypass the fixed-param-count fast path for forms like
    ``string map -nocase``, ``string match -nocase``, and
    ``string compare -nocase`` which would otherwise pass ``-nocase``
    as the first positional argument and silently corrupt the result.
    """
    if not arg or len(arg) < 2 or arg[0] != "-":
        return False
    if arg in ("-", "--"):
        return False
    second = arg[1]
    return not (second.isdigit() or second == ".")


class _WasmEmitterValuesMixin(_Base):
    if TYPE_CHECKING:
        # From _WasmEmitterVarMixin
        def _emit_var_read_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_var_write_obj_keep(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_namespace_eval_bridge(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterStmtMixin
        def _emit_eval_fallback(self, *a: Any, **kw: Any) -> Any: ...
        def _resolve_proc_qname(self, *a: Any, **kw: Any) -> Any: ...
        def _resolve_proc(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterExprMixin
        def _emit_expr(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_expr_obj(self, *a: Any, **kw: Any) -> Any: ...
        def _split_command_subst(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCtrlMixin
        def _emit_catch_from_args(self, *a: Any, **kw: Any) -> Any: ...
        # From _WasmEmitterCmdMixin
        def _emit_cmd_uplevel(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_cmd_lassign(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_array_subcmd_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_clock_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_info_value(self, *a: Any, **kw: Any) -> Any: ...
        def _emit_list_value(self, *a: Any, **kw: Any) -> Any: ...

    def _emit_value(self, value: str, *, was_braced: bool = False) -> Ownership:
        """Emit an i32 TclObj pointer for *value* and return its
        :class:`Ownership` tag.

        Resolves ``$x`` and ``${x}`` variable references to local.get
        (already i32), detects ``[cmd ...]`` command substitution and
        dispatches through the proc-call mechanism, or creates a new
        TclObj for literals.

        For strings with embedded substitutions (e.g. ``"hello $x"`` or
        ``"fib(10) = [fib 10]"``), parses the string into chunks and
        emits a concat chain using the runtime ``tcl_append`` function.

        Return value (S2.1):

        - :data:`Ownership.BORROWED` when the value came from
          ``local.get`` of a slot — the caller must retain if it
          stores the value into another owning slot.
        - :data:`Ownership.OWNED` for everything else (literals,
          interpolated strings, command-substitution results) — the
          stack value carries a +1 the caller can transfer.

        *was_braced* is set by callers that know the IR value came from
        a braced ``{…}`` token (the lexer strips the outer braces before
        storing the value, so we otherwise can't tell).  In that case,
        Tcl semantics say NO substitution applies — backslash
        substitution and ``$`` / ``[`` interpolation must be suppressed
        so ``\\{`` stays as the two-char sequence ``\\{`` instead of
        collapsing to ``{``.
        """
        if not was_braced:
            var = self._resolve_var_name(value)
            if var is not None:
                # Intern on first reference so reads before assignment
                # get a proper local (default-initialised to 0) instead
                # of falling through and boxing "$var" as a string literal.
                # Aliased variables (upvar/variable) route through the
                # runtime global table via _emit_var_read_obj.
                self._emit_var_read_obj(var)
                return Ownership.BORROWED
            # Try direct local lookup (for bare names like in IRAssignValue).
            # Alias-aware: checks _aliases before local_index.
            if value in self._aliases:
                self._emit_var_read_obj(value)
                return Ownership.BORROWED
            idx = self._local_index.get(value)
            if idx is not None:
                self._emit_local_get(idx)
                return Ownership.BORROWED
        # Braced literal — outer braces suppress all substitution in Tcl.
        # _split_command_subst preserves {…} so downstream code can
        # distinguish a braced word (literal) from a quoted one (allows
        # substitution).  Strip the braces and emit the content as-is.
        if value.startswith("{") and value.endswith("}"):
            self._emit_obj_literal(value[1:-1])
            return Ownership.OWNED
        # Braced token whose outer braces the IR already stripped — emit
        # the raw content verbatim, no ``\\`` / ``$`` / ``[`` processing.
        if was_braced:
            self._emit_obj_literal(value)
            return Ownership.OWNED
        # Command substitution: [cmd arg1 arg2 ...]
        if value.startswith("[") and value.endswith("]"):
            self._emit_command_subst_value(value)
            return Ownership.OWNED
        # Interpolated string: contains embedded $var/${var}/[cmd]
        # mixed with literal text.  Emit a concat chain.
        if self._has_embedded_subst(value):
            self._emit_interpolated_value(value)
            return Ownership.OWNED
        # Apply Tcl backslash substitution for non-braced literals.  Braced
        # words (already handled above) suppress all substitution; plain words
        # and double-quoted words allow ``\\`` → ``\``, ``\n`` → newline, etc.
        if "\\" in value:
            value = _tcl_backslash_subst(value)
        self._emit_obj_literal(value)
        return Ownership.OWNED

    def _has_embedded_subst(self, value: str) -> bool:
        """Check if *value* contains embedded $var, ${var}, or [cmd] substitutions."""
        i = 0
        n = len(value)
        while i < n:
            c = value[i]
            if c == "\\" and i + 1 < n:
                i += 2
                continue
            if c == "$" and i + 1 < n:
                nxt = value[i + 1]
                if nxt == "{" or nxt.isalpha() or nxt == "_":
                    return True
            if c == "[":
                return True
            i += 1
        return False

    def _parse_interpolated_parts(self, value: str) -> list[tuple[str, str]]:
        """Parse an interpolated string into ("lit", text) / ("var", name) /
        ("cmd", body) tuples.

        The returned list represents the string as a sequence of parts to
        concatenate.  Backslash escapes are preserved in literal parts.
        """
        parts: list[tuple[str, str]] = []
        buf: list[str] = []

        def flush() -> None:
            if buf:
                parts.append(("lit", "".join(buf)))
                buf.clear()

        i = 0
        n = len(value)
        while i < n:
            c = value[i]
            if c == "\\" and i + 1 < n:
                esc = value[i + 1]
                buf.append({"n": "\n", "t": "\t", "r": "\r"}.get(esc, esc))
                i += 2
                continue
            if c == "$" and i + 1 < n:
                nxt = value[i + 1]
                if nxt == "{":
                    # ${name}
                    j = value.find("}", i + 2)
                    if j == -1:
                        buf.append(c)
                        i += 1
                        continue
                    flush()
                    parts.append(("var", value[i + 2 : j]))
                    i = j + 1
                    continue
                if nxt.isalpha() or nxt == "_" or nxt == ":":
                    # $name — accepts [A-Za-z0-9_] and ``::`` namespace
                    # separators so names like ``$::counter::secsPerMinute``
                    # parse as a single variable reference rather than
                    # stopping at the first colon.  When the name is
                    # immediately followed by ``(``, consume through
                    # the matching ``)`` so ``$arr(key)`` is emitted
                    # as a single ("var", "arr(key)") part — the
                    # downstream `_emit_var_read_obj` dispatcher then
                    # routes it through ``_parse_array_ref`` to the
                    # array-element reader.  Without this, ``$a(x)``
                    # would split into ``$a`` + literal ``(x)`` and
                    # the array lookup would never happen.
                    j = i + 1
                    while j < n:
                        ch = value[j]
                        if ch.isalnum() or ch == "_":
                            j += 1
                        elif ch == ":" and j + 1 < n and value[j + 1] == ":":
                            j += 2
                        else:
                            break
                    if j < n and value[j] == "(":
                        # Scan for matching ')' — nested ``(..)``
                        # aren't standard in ``$arr(key)`` syntax
                        # but embedded ``$var`` / ``[cmd]`` may
                        # appear as part of the key; we look for
                        # the first unescaped ``)`` at depth 0
                        # relative to ``(``/``)`` nesting.
                        depth = 1
                        k = j + 1
                        while k < n and depth > 0:
                            ck = value[k]
                            if ck == "\\" and k + 1 < n:
                                k += 2
                                continue
                            if ck == "(":
                                depth += 1
                            elif ck == ")":
                                depth -= 1
                                if depth == 0:
                                    break
                            k += 1
                        if k < n and value[k] == ")":
                            flush()
                            parts.append(("var", value[i + 1 : k + 1]))
                            i = k + 1
                            continue
                    flush()
                    parts.append(("var", value[i + 1 : j]))
                    i = j
                    continue
            if c == "[":
                # [cmd ...] — find matching ] with nesting
                depth = 1
                j = i + 1
                while j < n and depth > 0:
                    if value[j] == "[":
                        depth += 1
                    elif value[j] == "]":
                        depth -= 1
                    if depth > 0:
                        j += 1
                    else:
                        break
                if depth != 0:
                    buf.append(c)
                    i += 1
                    continue
                flush()
                parts.append(("cmd", value[i + 1 : j]))
                i = j + 1
                continue
            buf.append(c)
            i += 1
        flush()
        return parts

    def _emit_interpolated_value(self, value: str) -> None:
        """Emit a TclObj for an interpolated string with $var/[cmd] chunks.

        Parses the string and emits each part (literal, variable, or
        command substitution), then concatenates them via tcl_append.
        """
        parts = self._parse_interpolated_parts(value)
        if not parts:
            self._emit_obj_literal("")
            return
        append_idx = self._shared_imports.get("tcl_append")
        if append_idx is None:
            # No append available — fall back to literal
            self._emit_obj_literal(value)
            return

        # Emit the first part
        self._emit_part(parts[0])
        # Concat remaining parts
        for part in parts[1:]:
            self._emit_part(part)
            self._emit_call(append_idx)

    def _emit_part(self, part: tuple[str, str]) -> None:
        kind, data = part
        if kind == "lit":
            self._emit_obj_literal(data)
        elif kind == "var":
            # Route through _emit_var_read_obj so array references
            # (``arr(key)``), aliases (upvar/variable), and
            # ``::``-qualified globals resolve through the correct
            # runtime path rather than all being treated as simple
            # WASM locals — otherwise ``"v=$arr(x)"`` would intern
            # a local named ``arr(x)`` and read 0 instead of
            # dispatching to ``tcl_array_get``.
            self._emit_var_read_obj(data)
        elif kind == "cmd":
            self._emit_command_subst_value("[" + data + "]")

    def _emit_command_subst_value(self, text: str) -> None:
        """Emit an i32 TclObj for a command substitution in value context.

        Dispatches ``[cmd ...]`` to a known proc (keeping its i32 result)
        or to ``[expr {...}]`` (compiling the expression and boxing).
        Falls back to a null TclObj for unknown commands.
        """
        cmd_text = text[1:-1].strip()
        # ``{*}`` argument expansion — compile-time splitting can't
        # cope with run-time list shape (``{*}$args`` length /
        # contents only known at run time) and naive splitting
        # mistakes ``{*}`` for a brace-quoted literal token,
        # dispatching the callee with ``{*}`` and the unexpanded
        # word as two separate args.  Route every expansion-bearing
        # form through the eval-fallback so the runtime parser
        # handles ``{*}WORD`` uniformly: literal lists split into
        # elements, ``$var`` / ``[cmd]`` produce list values the
        # dispatcher expands at run time.
        if _contains_expand_prefix(cmd_text):
            self._emit_eval_fallback("", (), script_override=cmd_text)
            return
        if not cmd_text:
            self._emit_i32_const(0)
            return

        parts = self._split_command_subst(cmd_text)
        if not parts:
            self._emit_i32_const(0)
            return

        cmd_name = parts[0]
        cmd_args = parts[1:]

        # [expr {...}] — compile expression and leave TclObj on stack.
        # Strip outer braces from expr_arg ({...} kept by splitter).
        if cmd_name == "expr" and len(cmd_args) == 1:
            expr_arg = cmd_args[0]
            if expr_arg.startswith("{") and expr_arg.endswith("}"):
                expr_arg = expr_arg[1:-1]
            from .....parsing.expr_parser import parse_expr

            try:
                nested_expr = parse_expr(expr_arg)
                self._emit_expr_obj(nested_expr)
                return
            except Exception:
                pass

        # [catch {body} ?varName?] in value context — re-parse the body
        # and emit via the real catch codegen so the body runs in the
        # compiled frame (with locals visible to eval-fallbacks).  Going
        # through ``_emit_eval_fallback`` would rebuild the script as
        # ``catch $v1 $v2 ...`` and lose the body's word structure.
        if cmd_name == "catch" and cmd_args:
            self._emit_catch_from_args(tuple(cmd_args), defs=(), keep_on_stack=True)
            return

        # ``[set varname]`` — 1-arg read form.  Without this shortcut
        # the call falls through to the eval fallback, which rebuilds
        # the script and evaluates ``set`` against the global table —
        # but compiled-frame locals and array elements live in
        # different storage, so ``[set a(x)]`` would return empty.
        # Route reads through the same ``_emit_var_read_obj`` path
        # that handles array refs, aliases, and qualified globals.
        if cmd_name == "set" and len(cmd_args) == 1:
            self._emit_var_read_obj(cmd_args[0])
            return
        # ``[set varname value]`` — 2-arg write: evaluate the value,
        # store it, and leave it on the stack (set returns the new value).
        if cmd_name == "set" and len(cmd_args) == 2:
            self._emit_value(cmd_args[1])
            self._emit_var_write_obj_keep(cmd_args[0])
            return

        # ``[namespace origin <name>]`` — return the canonical qualified
        # name of a command.  The runtime has no proc registry visible
        # to ``namespace origin``, so resolve at compile time: if the
        # bare name is known in the current proc's namespace, return its
        # qualified form; otherwise prepend ``::`` for global scope.
        # The tcltest pattern ``[list [namespace origin Eval] ...]``
        # depends on this producing a non-null string so uplevel
        # dispatches the right proc.
        if cmd_name == "namespace" and len(cmd_args) >= 2 and cmd_args[0] == "origin":
            bare_name = cmd_args[1]
            qualified = self._resolve_proc_qname(bare_name) or bare_name
            if not qualified.startswith("::"):
                ns = self._proc_namespace or "::"
                qualified = f"{ns}::{bare_name}" if ns != "::" else f"::{bare_name}"
            self._emit_obj_literal(qualified)
            return

        # Proc call — result is already i32 TclObj
        proc_info = self._resolve_proc(cmd_name)
        if proc_info is not None:
            func_idx, n_params = proc_info
            qname = self._resolve_proc_qname(cmd_name)
            has_args_tail = qname is not None and qname in self._proc_args_tail
            # Stash the exact word the caller wrote so the callee's
            # prologue can report it as argv[0] via
            # ``take_pending_argv0``.  Must happen BEFORE args are
            # evaluated so inner compiled calls that own their own
            # pending slot don't clobber ours, and the push right
            # before ``call`` consumes it immediately.
            argv0_local = self._emit_prepare_pending_argv0(cmd_name)
            if has_args_tail and n_params > 0:
                # ``proc p {... args}``: pack all surplus call-site args
                # into a list TclObj for the trailing slot.  Without
                # this branch the value-context dispatcher emitted the
                # first argument into the ``args`` slot and silently
                # dropped the rest (so ``p 1 2 3`` arrived as
                # ``args == 1``).
                fixed = n_params - 1
                for i in range(min(fixed, len(cmd_args))):
                    self._emit_value(cmd_args[i])
                for _slot in range(len(cmd_args), fixed):
                    # Pad with null TclObj; the compiled-proc prologue
                    # substitutes the declared default (if any) and
                    # the unsubstituted null lets ``frame_set_argv``
                    # report an accurate argv for ``info level 0``.
                    self._emit_i32_const(0)
                self._emit_args_list(tuple(cmd_args[fixed:]))
                self._emit_push_pending_argv0(argv0_local)
                self._emit_call(func_idx)
                return
            for i in range(min(n_params, len(cmd_args))):
                self._emit_value(cmd_args[i])
            # Missing args: emit the declared default (as a string
            # literal so ``{foo bar}`` reaches the callee as the
            # literal string), or fall back to a boxed-int 0 sentinel
            # when no default was declared.  ``[outputChannel]`` with
            # no args relies on this — without proper default
            # handling, ``filename`` arrives as the integer TclObj 0
            # rather than the ``""`` the ``proc outputChannel
            # {{filename ""}}`` spec declares, making ``info level
            # 0`` report a 2-element list and the no-args early-
            # return check fail.
            for _slot in range(len(cmd_args), n_params):
                # Pad with null TclObj; the compiled-proc prologue
                # substitutes the declared default (if any) and
                # the unsubstituted null lets ``frame_set_argv``
                # report an accurate argv for ``info level 0``.
                self._emit_i32_const(0)
            self._emit_push_pending_argv0(argv0_local)
            self._emit_call(func_idx)
            return

        # dict sub-command in value context — returns i32 TclObj
        if cmd_name == "dict" and cmd_args:
            subcmd = cmd_args[0]
            # ``dict merge ?d1 d2 ...?`` — variadic.  Chain
            # pairwise merges via the runtime helper; source always
            # wins on duplicate keys.  Zero args → empty dict.
            if subcmd == "merge":
                sub_args = cmd_args[1:]
                merge_idx = self._shared_imports.get("tcl_dict_merge_pair")
                if not sub_args:
                    self._emit_obj_literal("")
                    return
                if merge_idx is None:
                    self._emit_value(sub_args[0])
                    return
                self._emit_value(sub_args[0])
                for rest in sub_args[1:]:
                    self._emit_value(rest)
                    self._emit_call(merge_idx)
                return
            # ``dict create`` — the runtime helper returns empty and
            # ignores its args, so build the k/v pair list at the
            # compiler level.  Pure literals fold to a compile-time
            # string; anything else chains ``tcl_lappend`` which
            # re-quotes each element canonically.  Using ``tcl_concat``
            # here was wrong — concat trims leading/trailing whitespace
            # from every argument, so ``dict create $k $v`` with
            # whitespace-containing values would silently discard the
            # outer spaces (and fail to brace-wrap list-valued keys).
            if subcmd == "create":
                kv = cmd_args[1:]
                if not kv:
                    self._emit_obj_literal("")
                    return
                if all(
                    not a.startswith("$")
                    and not a.startswith("[")
                    and not self._has_embedded_subst(a)
                    and a not in self._aliases
                    and a not in self._local_index
                    for a in kv
                ):
                    self._emit_obj_literal(" ".join(kv))
                    return
                lappend_idx = self._shared_imports.get("tcl_lappend")
                if lappend_idx is None:
                    self._emit_eval_fallback(cmd_name, tuple(cmd_args))
                    return
                # Start with empty list; lappend each k/v in order.
                self._emit_obj_literal("")
                for elem in kv:
                    self._emit_value(elem)
                    self._emit_call(lappend_idx)
                return
            sri = subcommand_runtime_import_for("dict", subcmd)
            if sri is not None and sri.import_key in self._shared_imports:
                func_idx = self._shared_imports[sri.import_key]
                param_count = len(sri.params)
                sub_args = cmd_args[1:]
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not sri.results:
                    self._emit_i32_const(0)
                return

        # string sub-command in value context
        if cmd_name == "string" and cmd_args:
            subcmd = cmd_args[0]
            # ``string cat`` — variadic, no-trim concat.  Fold pure
            # literals; chain ``tcl_append`` for mixed cases.
            if subcmd == "cat":
                sub_args = cmd_args[1:]
                if not sub_args:
                    self._emit_obj_literal("")
                    return
                if all(
                    not a.startswith("$")
                    and not a.startswith("[")
                    and not self._has_embedded_subst(a)
                    and a not in self._aliases
                    and a not in self._local_index
                    for a in sub_args
                ):
                    self._emit_obj_literal("".join(sub_args))
                    return
                # Non-literal path — chain ``tcl_append``.  The scan
                # pass registers the import whenever ``string cat``
                # appears, so the fall-through below is defensive.
                append_idx = self._shared_imports.get("tcl_append")
                if append_idx is None:
                    self._emit_eval_fallback("string", cmd_args)
                    return
                self._emit_value(sub_args[0])
                for rest in sub_args[1:]:
                    self._emit_value(rest)
                    self._emit_call(append_idx)
                return
            sri = subcommand_runtime_import_for("string", subcmd)
            if sri is not None and sri.import_key in self._shared_imports:
                sub_args = cmd_args[1:]
                # ``string map -nocase ...`` / ``string match -nocase ...``
                # / ``string compare -nocase ...`` — the fixed-param
                # runtime import would silently take ``-nocase`` as the
                # first positional argument and corrupt the result.
                # Fall through to eval so the runtime dispatcher
                # parses the option correctly.  Use ``script_override``
                # with the raw bracket body so braced args like
                # ``{HELLO WORLD}`` keep their original quoting (the
                # default eval-fallback path re-list-quotes them which
                # would double-brace and the runtime would treat them
                # as single-element lists).
                if sub_args and _looks_like_string_option(sub_args[0]):
                    self._emit_eval_fallback("string", cmd_args, script_override=cmd_text)
                    return
                func_idx = self._shared_imports[sri.import_key]
                param_count = len(sri.params)
                for i in range(min(param_count, len(sub_args))):
                    self._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not sri.results:
                    self._emit_i32_const(0)
                return

        # info sub-command in value context — leaves i32 TclObj on stack.
        if cmd_name == "info" and cmd_args:
            self._emit_info_value(tuple(cmd_args))
            return

        # lassign in value context — return leftover list.
        if cmd_name == "lassign" and cmd_args:
            self._emit_cmd_lassign(tuple(cmd_args), defs=(), keep_on_stack=True)
            return

        # clock in value context — i32 TclObj of the timer value.
        if cmd_name == "clock" and cmd_args:
            self._emit_clock_value(tuple(cmd_args))
            return

        # array subcommand in value context.
        if cmd_name == "array" and cmd_args:
            self._emit_array_subcmd_value(tuple(cmd_args))
            return

        # uplevel in value context.
        if cmd_name == "uplevel" and cmd_args:
            self._emit_cmd_uplevel(tuple(cmd_args))
            return

        # ``list`` in value context — variadic list builder, same
        # space-joined compile-time shape as the statement-context
        # path.
        if cmd_name == "list":
            self._emit_list_value(tuple(cmd_args))
            return

        # ``concat`` in value context — variadic, trim+join semantics.
        if cmd_name == "concat":
            concat_idx = self._shared_imports.get("tcl_concat")
            if concat_idx is not None:
                if not cmd_args:
                    self._emit_obj_literal("")
                elif len(cmd_args) == 1:
                    # Single-arg concat: trim whitespace (concat(x, "") returns trimmed x).
                    self._emit_value(cmd_args[0])
                    self._emit_obj_literal("")
                    self._emit_call(concat_idx)
                else:
                    self._emit_value(cmd_args[0])
                    for a in cmd_args[1:]:
                        self._emit_value(a)
                        self._emit_call(concat_idx)
                return

        # ``regexp`` / ``regsub`` with options or capture vars — the
        # 2-arg runtime fast path can't represent them, so fall through
        # to the eval path which dispatches to ``eval_regexp_cmd`` /
        # the regsub interpreter handler with full Tcl 9 option +
        # capture semantics.  Bare ``regexp PAT STR`` keeps the inline
        # fast path.  See cmds/regexp_.py for the matching statement-
        # context hook + the known compiled-top-level capture-var
        # readback caveat.
        if cmd_name in ("regexp", "regsub") and cmd_args:
            n_pos = 0
            uses_options = False
            for a in cmd_args:
                if a.startswith("-") and len(a) > 1:
                    uses_options = True
                    break
                n_pos += 1
            min_positional = 2 if cmd_name == "regexp" else 3
            if uses_options or n_pos > min_positional:
                # Pre-intern capture-var names so the proc's frame
                # readback after eval-fallback reloads them into
                # the wasm-local cache.  Without this, ``regexp
                # PAT STR a b c`` inside a compiled proc body
                # leaves ``$a`` / ``$b`` / ``$c`` reading stale
                # (empty) wasm-locals — see cmds/regexp_.py.
                from .cmds.regexp_ import _capture_vars_for as _cap_vars

                for vname in _cap_vars(cmd_name, tuple(cmd_args)):
                    self._intern_local(vname)
                # Use ``script_override`` with the original source text
                # so braced patterns like ``{a+}`` survive verbatim
                # rather than being re-list-quoted to ``{{a+}}`` (which
                # the runtime parser would unwrap once back to the
                # literal ``{a+}`` and pass to the regex engine —
                # matching nothing).
                self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
                return

        # Runtime command in value context (llength, lindex, etc.)
        rimp = runtime_import_for(cmd_name)
        if rimp is not None:
            func_idx = self._shared_imports.get(rimp.import_key)
            if func_idx is not None:
                param_count = len(rimp.params)
                if cmd_name == "linsert" and len(cmd_args) > param_count:
                    # Multi-value ``[linsert list idx v1 v2 …]`` — see
                    # ``_emit_cmd_runtime`` for the index-ordering
                    # rationale.  Value-context variant leaves the
                    # final running-result on the stack.
                    list_arg = cmd_args[0]
                    index_arg = cmd_args[1]
                    values = cmd_args[2:]
                    self._emit_value(list_arg)
                    iter_values = (
                        values if _is_end_relative_index(index_arg) else tuple(reversed(values))
                    )
                    for v in iter_values:
                        self._emit_value(index_arg)
                        self._emit_value(v)
                        self._emit_call(func_idx)
                    return
                if cmd_name == "lreplace" and len(cmd_args) > param_count:
                    # Multi-value ``[lreplace list first last v1 v2 …]``
                    # — see ``_emit_cmd_runtime`` for ordering rationale.
                    list_arg = cmd_args[0]
                    first_arg = cmd_args[1]
                    last_arg = cmd_args[2]
                    values = cmd_args[3:]
                    list_insert_idx = self._shared_imports.get("tcl_list_insert")
                    if list_insert_idx is None or not values:
                        self._emit_value(list_arg)
                        self._emit_value(first_arg)
                        self._emit_value(last_arg)
                        self._emit_value(values[0] if values else "")
                        self._emit_call(func_idx)
                    elif _is_end_relative_index(first_arg):
                        self._emit_value(list_arg)
                        self._emit_value(first_arg)
                        self._emit_value(last_arg)
                        self._emit_value(values[0])
                        self._emit_call(func_idx)
                        for v in values[1:]:
                            self._emit_value(first_arg)
                            self._emit_value(v)
                            self._emit_call(list_insert_idx)
                    else:
                        self._emit_value(list_arg)
                        self._emit_value(first_arg)
                        self._emit_value(last_arg)
                        self._emit_value(values[-1])
                        self._emit_call(func_idx)
                        for v in reversed(values[:-1]):
                            self._emit_value(first_arg)
                            self._emit_value(v)
                            self._emit_call(list_insert_idx)
                    return
                if cmd_name in ("lsort",) and len(cmd_args) > param_count:
                    # ``lsort ?-switches? list`` — runtime export is
                    # the no-switch form; grab the trailing positional
                    # list rather than treating ``-integer`` as the list
                    # itself and returning the single-element result.
                    self._emit_value(cmd_args[-1])
                    for _ in range(param_count - 1):
                        self._emit_i32_const(0)
                elif cmd_name == "apply":
                    # ``apply lambda ?arg ...?`` — the runtime export's
                    # second param is a Tcl *list* of every positional
                    # arg (it list-parses to recover individual words).
                    # The default ``param_count=2`` fast path passes the
                    # raw arg verbatim, which mis-fires when the arg
                    # contains whitespace (``apply LAM {a 1 c 2}`` →
                    # the lambda's first param sees only ``a``).  Pack
                    # the args into a single list TclObj.
                    #
                    # ``_split_command_subst`` keeps outer ``{…}`` on
                    # braced words; strip them before re-list-quoting
                    # so we don't double-brace.
                    def _strip_braces(s: str) -> str:
                        return (
                            s[1:-1]
                            if (len(s) >= 2 and s.startswith("{") and s.endswith("}"))
                            else s
                        )

                    self._emit_value(cmd_args[0] if cmd_args else "")
                    self._emit_args_list(tuple(_strip_braces(a) for a in cmd_args[1:]))
                else:
                    for i in range(min(param_count, len(cmd_args))):
                        self._emit_value(cmd_args[i])
                    for _ in range(param_count - len(cmd_args)):
                        self._emit_i32_const(0)
                self._emit_call(func_idx)
                if not rimp.results:
                    self._emit_i32_const(0)
                return

        # ``[namespace eval ns arg1 arg2 ...]`` with dynamic script args.
        # The eval-fallback path would embed the literal source text (e.g.
        # ``$CustomMatch($mode)``) and let the interpreter re-evaluate it.
        # But the interpreter can't see compiled-frame aliases (e.g.
        # ``CustomMatch`` aliased to ``::tcltest::CustomMatch`` via
        # ``variable CustomMatch``), so array-element references like
        # ``$CustomMatch($mode)`` resolve to the wrong value (or nothing).
        # Fix: evaluate each script arg through the WASM compiled path
        # (which honours aliases) to get the actual string values, join
        # them with spaces at runtime, then pass the assembled script to
        # ``tcl_eval``.
        if cmd_name == "namespace" and cmd_args and cmd_args[0] == "eval" and len(cmd_args) > 2:
            if self._emit_namespace_eval_bridge(
                cmd_args[2:], drop_result=False, ns_name=cmd_args[1]
            ):
                return
            # Required runtime imports missing — fall through to the
            # generic eval fallback below so we still produce something.
            self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)
            return

        # Unknown command in value context — fall back to interpreter.
        # Use the original command text (script_override) so braced
        # words like {[^a-z]+} are not re-interpreted as command
        # substitutions during fallback script reconstruction.
        self._emit_eval_fallback(cmd_name, cmd_args, script_override=cmd_text)

    def _emit_obj_literal(self, value: str) -> None:
        """Create a TclObj for a literal value — pushes i32 pointer.

        Integers are boxed via ``tcl_obj_new_int``; non-numeric strings
        are boxed via ``tcl_obj_new_string`` from the data segment.

        Requires TclObj lifecycle imports; raises if they are missing
        to avoid silently emitting raw i32 values that would be
        misinterpreted as TclObj pointers.
        """
        new_int_idx = self._shared_imports.get("tcl_obj_new_int")
        new_str_idx = self._shared_imports.get("tcl_obj_new_string")
        if new_int_idx is None or new_str_idx is None:
            msg = (
                "WASM TclObj literal emission requires shared imports "
                "'tcl_obj_new_int' and 'tcl_obj_new_string'"
            )
            raise RuntimeError(msg)

        try:
            int_val = int(value)
            # Tcl 9 BigInt literal — fall through to the string path
            # so the runtime preserves the source bytes.  ``i64.const``
            # would saturate (or be rejected by wasmtime entirely) and
            # silently lose precision on every BigInt arithmetic step.
            if -(1 << 63) <= int_val <= (1 << 63) - 1:
                # S6.4 — small integers fit in a tagged-immediate
                # handle.  Emit the encoded i32 directly instead of
                # calling ``tcl_obj_new_int`` — saves a function call
                # plus the heap allocation that the runtime would
                # otherwise have to short-circuit on.  The encoding
                # (``(value << 1) | 1``) matches the runtime's
                # ``immediate_box`` so readers transparently round-trip.
                if -(1 << 30) <= int_val <= (1 << 30) - 1:
                    tagged = ((int_val << 1) | 1) & 0xFFFFFFFF
                    # _emit_i32_const takes a signed value; reinterpret
                    # as signed via the standard 32-bit two's-complement
                    # mapping.
                    if tagged >= 0x80000000:
                        tagged -= 0x100000000
                    self._emit_i32_const(tagged)
                    return
                self._emit_i64_const(int_val)
                self._emit_call(new_int_idx)
                return
        except ValueError:
            pass
        offset = self._intern_string(value)
        encoded = value.encode("utf-8", errors="surrogatepass")
        # data_ptr = segment offset + 4 (skip length prefix)
        self._emit_i32_const(offset + 4)
        self._emit_i32_const(len(encoded))
        self._emit_call(new_str_idx)

    def _emit_box_int(self) -> None:
        """Convert i64 on stack to i32 TclObj pointer via tcl_obj_new_int."""
        self._emit_call(self._shared_imports["tcl_obj_new_int"])

    def _emit_unbox_int(self) -> None:
        """Convert i32 TclObj pointer on stack to i64 via tcl_obj_get_int."""
        self._emit_call(self._shared_imports["tcl_obj_get_int"])

    def _emit_default_arg(self) -> None:
        """Emit a null TclObj (``i32.const 0``) for a missing
        call-site argument.

        The compiled-proc prologue inspects each param slot: if
        it's null, substitutes the declared default (see the
        prologue in :meth:`generate`); if still null after
        substitution, the callee reads empty via ``var_resolve``.
        This approach keeps the "was the slot supplied?" bit
        recoverable so the prologue's argv-list capture
        (``frame_set_argv``) reports the caller's real word
        count — the invariant tcltest's
        ``[llength [info level 0]] == 1`` accessor pattern
        depends on.
        """
        self._emit_i32_const(0)

    def _emit_prepare_pending_argv0(self, invoked_name: str) -> int | None:
        """Stash *invoked_name* in a reserved local so the caller
        can push it onto the pending-argv0 slot right before a
        compiled ``call``.

        Why a reserved local rather than emitting the ``obj_literal
        → set_pending_argv0`` pair straight away?  Argument
        evaluation between the prepare and the call may itself
        invoke other compiled procs (``foo [bar 1]``), and those
        inner calls will consume their own pending-argv0 slot via
        ``take_pending_argv0``.  Stashing the TclObj pointer in a
        local keeps our outer caller's argv0 alive until its own
        call, so the inner bar/outer foo sequence reports the
        right invoked word for each callee.

        Returns the local index (i32) on success, or ``None`` when
        the pending-argv0 imports aren't available (either the
        scan layer didn't pull them in or this is a runtime that
        pre-dates the ABI).  Callers should pass the returned
        handle to :meth:`_emit_push_pending_argv0` immediately
        before ``self._emit_call(func_idx)``.
        """
        if "tcl_frame_set_pending_argv0" not in self._shared_imports:
            return None
        loc = self._add_extra_local(prefix="_pending_argv0", val_type=ValType.I32)
        # Evaluate the literal now (pushes TclObj pointer) and
        # stash it — the value is inert, so intervening code can
        # execute without affecting the stored word.
        self._emit_obj_literal(invoked_name)
        self._emit_local_set(loc)
        return loc

    def _emit_push_pending_argv0(self, saved_local: int | None) -> None:
        """Publish the stashed invoked word to the runtime's
        pending-argv0 slot.  Emit immediately before the
        corresponding compiled ``call`` — the callee's prologue
        consumes the slot on entry.  No-op when *saved_local* is
        ``None`` (the paired :meth:`_emit_prepare_pending_argv0`
        returned ``None``, typically because the runtime import
        isn't available).
        """
        if saved_local is None:
            return
        set_idx = self._shared_imports.get("tcl_frame_set_pending_argv0")
        if set_idx is None:
            return
        self._emit_local_get(saved_local)
        self._emit_call(set_idx)

    def _emit_args_list(
        self,
        tail_args: tuple[str, ...],
        *,
        was_braced_fn: "Callable[[int], bool] | None" = None,
    ) -> None:
        """Emit a Tcl list TclObj containing *tail_args*.

        Used when calling a proc whose last formal parameter is ``args``
        (Tcl's variadic catch-all): all surplus call-site arguments must
        be packed into a single list before being passed as that slot.

        If the list is empty (no surplus args) emit an empty string
        TclObj (Tcl's empty list).  If all elements are pure literals
        (no ``$var`` / ``[cmd]`` substitutions) build the list string
        at compile time and emit it as a single object literal.
        Otherwise build the list at runtime by starting with an empty
        list and calling ``tcl_cmd_lappend`` for each element.

        *was_braced_fn* tells us for each tail index whether the source
        token was a braced ``{…}`` word.  Braced content is protected
        from backslash substitution per Tcl semantics, so when the
        call-site word was braced the arg's bytes must pass through
        unchanged even if they contain ``\\{`` / ``\\n`` / ``\\$`` /
        etc.  Without this flag a test body like
        ``{set x "a\\{"; lappend x abc}`` would have its ``\\{`` folded
        to ``{`` before reaching the proc's ``args``, breaking later
        reparsing.
        """
        if not tail_args:
            # ``args`` formal receives the empty list ``{}``
            self._emit_obj_literal("")
            return

        def _was_braced(i: int) -> bool:
            return was_braced_fn(i) if was_braced_fn is not None else False

        # Check whether all elements are plain literals.
        def _is_literal(a: str) -> bool:
            return (
                not self._has_embedded_subst(a)
                and not a.startswith("$")
                and not a.startswith("[")
                and a not in self._aliases
                and a not in self._local_index
            )

        all_literals = all(_is_literal(a) for a in tail_args)
        if all_literals:
            # Build the list string at compile time.  IR values have outer
            # braces already stripped by the lexer, so treat brace-looking
            # values (e.g. "{}" from source "{{}}") as literal data and let
            # _tcl_list_quote encode them correctly.  Non-braced values may
            # have raw backslash sequences that need expansion first; braced
            # values carry their exact source bytes and must pass through
            # unchanged.
            def _prep(a: str, braced: bool) -> str:
                if braced:
                    return a
                if a.startswith("{") and a.endswith("}"):
                    return a  # brace chars are part of the value, not quoting
                return _tcl_backslash_subst(a) if "\\" in a else a

            list_str = " ".join(
                _tcl_list_quote(_prep(a, _was_braced(i)), first=(i == 0))
                for i, a in enumerate(tail_args)
            )
            self._emit_obj_literal(list_str)
            return

        # Runtime path: start with empty list, lappend each arg
        lappend_idx = self._shared_imports.get("tcl_lappend")
        if lappend_idx is not None:
            self._emit_obj_literal("")  # empty list seed
            for i, a in enumerate(tail_args):
                self._emit_value(a, was_braced=_was_braced(i))
                self._emit_call(lappend_idx)
        else:
            # No lappend available — fall back to compile-time join.
            # IR values are already de-braced by the lexer; _tcl_list_quote
            # handles proper list encoding.
            def _prep2(a: str, braced: bool) -> str:
                if braced:
                    return a
                return _tcl_backslash_subst(a) if "\\" in a else a

            list_str = " ".join(
                _tcl_list_quote(_prep2(a, _was_braced(i)), first=(i == 0))
                for i, a in enumerate(tail_args)
            )
            self._emit_obj_literal(list_str)

    def _intern_string(self, value: str) -> int:
        """Return the memory offset for a string constant.

        Uses ``surrogatepass`` so lone surrogates that the source
        reader left in a Python string survive as WTF-8 bytes.  The
        Zig runtime reads strings as opaque byte sequences — it does
        not validate UTF-8 — so this preserves the exact source
        bytes end-to-end.  Needed for test bundles that embed
        arbitrary binary data in ``test`` result strings (``expr.test``
        has a few); before the compile-time namespace-import
        resolution, those calls fell back to ``tcl_eval`` which
        stored the source verbatim, so the issue was latent.
        """
        if value in self._string_index:
            return self._string_index[value]
        offset = self._string_offset_ref[0]
        encoded = value.encode("utf-8", errors="surrogatepass")
        self._string_offset_ref[0] += len(encoded) + 4  # 4 bytes for length prefix
        self._strings.append((value, offset))
        self._string_index[value] = offset
        return offset
