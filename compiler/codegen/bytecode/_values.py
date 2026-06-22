"""Mixin: variable loading/storing, value emission."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from shared.sentinels import _BRACE_CLOSE, _BRACE_OPEN, _RAW_PREFIX
from shared.tcl_list import tcl_list_join

from ._helpers import _parse_subst_template, _split_list_simple
from .format import _esc
from .opcodes import Op

if TYPE_CHECKING:
    from ._emitter import _Emitter


# Cmd names that compile to a ``::tcl::<ensemble>::<sub>`` FQ-name
# push followed by ``invokeStk1`` even at top level.  When such a
# substitution is nested inside a strcat-style string interpolation
# tclsh wraps it in its own ``startCommand``; plain cmds (``lreverse``,
# ``puts``) inline without a wrap.  Restricted to ``dict`` for now —
# extend as more ensembles get the same treatment in the reference
# corpus.
_ENSEMBLE_INTERP_HEADS = frozenset({"dict"})


class _ValuesMixin:
    """Mixin: variable loading/storing, value emission."""

    # TYPE_CHECKING annotation to help IDEs:
    # self: _Emitter  — but not needed at runtime

    def _push_lit(self: _Emitter, value: str) -> None:
        idx = self._lit.intern(value)
        op = Op.PUSH1 if idx < 256 else Op.PUSH4
        self._emit(op, idx, comment=f'"{_esc(value)}"')

    _NO_DEDUP_TAG = " #nodedup"

    def _push_lit_no_dedup(self: _Emitter, value: str) -> None:
        """Push a literal using a fresh slot (no deduplication)."""
        idx = self._lit.register(value)
        op = Op.PUSH1 if idx < 256 else Op.PUSH4
        self._emit(op, idx, comment=f'"{_esc(value)}"{self._NO_DEDUP_TAG}')

    def _push_raw_lit(self: _Emitter, value: str) -> None:
        """Push a raw literal that must NOT be brace-stripped or substituted.

        Used for interpolation template parts that may contain ``{``, ``}``,
        ``$``, or ``[`` characters that should be literal.
        """
        idx = self._lit.intern(_RAW_PREFIX + value)
        op = Op.PUSH1 if idx < 256 else Op.PUSH4
        self._emit(op, idx, comment=f'"{_esc(value)}"')

    def _begin_command(self: _Emitter, *, count: int = 1) -> None:
        """Emit ``startCommand`` for non-first specialised commands.

        Must be paired with a ``_end_command()`` call after the
        command's own instructions are emitted.  The trailing ``pop``
        (if any) must come AFTER ``_end_command()``.
        """
        if self._cmd_index > 0:
            label = self._fresh_label("cmd_end")
            self._emit(Op.START_CMD, label, count)
            self._start_cmd_end_label = label
        else:
            self._start_cmd_end_label = None
        self._cmd_index += 1

    def _end_command(self: _Emitter) -> None:
        """Place the end label for the current ``startCommand``."""
        if self._start_cmd_end_label is not None:
            self._place_label(self._start_cmd_end_label)
            self._start_cmd_end_label = None

    @staticmethod
    def _split_array_ref(name: str) -> tuple[str, str] | None:
        """Split ``arr(key)`` into ``("arr", "key")``, or None."""
        if "(" in name and name.endswith(")"):
            idx = name.index("(")
            return name[:idx], name[idx + 1 : -1]
        return None

    def _push_array_key(self: _Emitter, elem: str, *, target: bool = False) -> None:
        """Push an array element key onto the stack.

        Handles nested variable references (``$var``, ``${var}``) and
        command substitutions (``[cmd ...]``).

        *target* is True when *elem* is the element key of a store **target**
        (``set arr(key) ...``).  In that position a bare *simple* ``$foo``
        (no index) is a brace-suppressed literal — ``set {arr($foo)} ...`` —
        because a live simple reference would have been normalised to
        ``${foo}`` by lowering.  In a read context (*target* False) a bare
        ``$foo`` is a live reference and is loaded.
        """
        if elem.startswith("${") and elem.endswith("}"):
            # Braced variable reference used as an array key, e.g.
            # ``outer(${a($i)})`` or the store target ``set outer(${a($i)}) 9``.
            # Braced spelling SUPPRESSES inner substitution, so a braced
            # array-shaped key whose inner name carries a *suppressed*
            # substitution (``${a($i)}`` — the ``$i`` is held literal) is a
            # WHOLE-name load: ``push "a($i)"; loadStk`` (the runtime
            # resolves the array element from the whole literal name),
            # matching tclsh 9.0.3.
            #
            # A braced key with a *static* index (``${a(1)}``) is special:
            # the command segmenter collapses BOTH the bare ``$a(1)`` and
            # braced ``${a(1)}`` static-index keys to the same ``${a(1)}``
            # spelling here, so the bare/braced distinction is already lost
            # by the time codegen runs.  The reference corpus exercises the
            # bare form (``set ::a($::a(1)) 3`` → SPLIT ``loadArrayStk``),
            # so the ambiguous static case must keep SPLITTING via
            # ``_load_var``; only the unambiguous dynamic-index case takes
            # the whole-name path.  A plain braced scalar ``${foo}`` also
            # loads via ``_load_var``.  Bare ``$...`` keys with a dynamic
            # index (``$a($i)``) survive as bare and are SPLIT by the
            # ``elif elem.startswith("$")`` branch below.
            whole = self._parse_whole_name_array_ref(elem)
            if whole is not None and ("$" in whole or "[" in whole):
                self._emit_whole_name_array_load(elem)
            else:
                inner = self._parse_simple_var_ref(elem)
                if inner is not None:
                    self._load_var(inner)
                else:
                    self._emit_value(elem)
        elif elem.startswith("$") and self._bare_var_ref_is_whole(elem):
            if target and self._split_array_ref(elem[1:]) is None:
                # Store target with a bare *simple* ``$foo`` element key — it
                # survived lowering un-normalised because it came from a
                # brace-suppressed word (``set {arr($foo)} ...``), so the
                # ``$`` is literal.  Push it raw (no substitution), matching
                # tclsh (``push "$foo"; storeArray``).
                self._push_raw_lit(elem)
            else:
                # A read, or a bare *array* reference whose index carries a
                # nested substitution (``$a($i)`` / ``$::a(1)``) kept
                # un-normalised by lowering: load the variable / element.
                self._load_var(elem[1:])
        elif "$" in elem or "[" in elem:
            # Pass interpolate=True so a mixed-substitution index like
            # ``[format x]\ y [format z]`` decomposes into push/cmd
            # pieces + strcat instead of being pushed as a literal.
            # ``_emit_value`` only kicks the interpolate path in for
            # multi-part templates, so a single ``[cmd]`` index needs
            # the explicit short-circuit below.
            if elem.startswith("[") and elem.endswith("]") and "[" not in elem[1:-1]:
                self._emit_inline_cmd_subst(elem)
            else:
                self._emit_value(elem, interpolate=True)
        else:
            self._push_lit(elem)

    def _push_var_ref(self: _Emitter, name: str) -> None:
        """Push the variable reference for a store/load.

        For scalars, pushes the name.  For arrays like ``arr(key)``,
        pushes array name then element separately (non-proc context),
        or just the element key (proc context with LVT array ops).
        When the element contains ``$var`` references (e.g.
        ``::a($::a(1))``), resolve them at runtime.
        """
        arr = self._split_array_ref(name)
        if arr is not None:
            if self._is_proc and not self._is_qualified(name):
                # Proc context: only push the key (LVT slot encodes array name)
                self._push_array_key(arr[1], target=True)
            else:
                self._push_lit(arr[0])
                self._push_array_key(arr[1], target=True)
        else:
            self._push_lit(name)

    def _is_array_ref(self: _Emitter, name: str) -> bool:
        """Return True if *name* is an array reference like ``arr(key)``."""
        return "(" in name and name.endswith(")")

    def _is_qualified(self: _Emitter, name: str) -> bool:
        """Return True for namespace-qualified variable names (``::foo``)."""
        return name.startswith("::")

    def _needs_stk_var_ref(self: _Emitter, name: str) -> bool:
        """True when a store needs the name/key pushed first.

        For proc context: True for qualified names and array refs (need key).
        For non-proc context: always True (name/arr+key pushed on stack).
        """
        if not self._is_proc:
            return True
        if self._is_qualified(name):
            return True
        # Proc context: array refs need the element key pushed
        return self._is_array_ref(name)

    def _load_var(self: _Emitter, name: str) -> None:
        if self._is_proc and not self._is_qualified(name):
            arr = self._split_array_ref(name)
            if arr is not None:
                base = arr[0]
                slot = self._lvt.intern(base)
                self._push_array_key(arr[1])
                self._emit(Op.LOAD_ARRAY1, slot, comment=f'var "{base}"')
            else:
                slot = self._lvt.intern(name)
                op = Op.LOAD_SCALAR1 if slot < 256 else Op.LOAD_SCALAR4
                self._emit(op, slot, comment=f'var "{name}"')
        else:
            arr = self._split_array_ref(name)
            if arr is not None:
                self._push_lit(arr[0])
                self._push_array_key(arr[1])
                self._emit(Op.LOAD_ARRAY_STK)
            else:
                self._push_lit(name)
                self._emit(Op.LOAD_STK)

    def _store_var(self: _Emitter, name: str) -> None:
        """Emit store instruction.  Caller must have pushed the value on TOS.

        For proc bodies, uses storeScalar.  For top-level scalars,
        caller must have pushed the name before the value (storeStk).
        For top-level arrays, caller must have pushed array name, element,
        then value (storeArrayStk).

        Namespace-qualified variables (``::foo``) always use stack-based
        access, even inside procs.
        """
        if self._is_proc and not self._is_qualified(name):
            arr = self._split_array_ref(name)
            if arr is not None:
                base = arr[0]
                slot = self._lvt.intern(base)
                self._emit(Op.STORE_ARRAY1, slot, comment=f'var "{base}"')
            else:
                slot = self._lvt.intern(name)
                op = Op.STORE_SCALAR1 if slot < 256 else Op.STORE_SCALAR4
                self._emit(op, slot, comment=f'var "{name}"')
        else:
            if self._is_array_ref(name):
                self._emit(Op.STORE_ARRAY_STK)
            else:
                self._emit(Op.STORE_STK)

    def _emit_incr(self: _Emitter, name: str, amount: str | None) -> None:
        """Emit incr bytecode, leaving the new value on TOS."""
        from compiler.parsing.expr_parser import parse_expr as _parse_expr_ast

        if self._is_proc and not self._is_qualified(name):
            slot = self._lvt.intern(name)
            if amount is None:
                self._emit(Op.INCR_SCALAR1_IMM, slot, 1, comment=f'var "{name}"')
            elif amount.lstrip("-").isdigit():
                imm = int(amount)
                if -128 <= imm <= 127:
                    self._emit(Op.INCR_SCALAR1_IMM, slot, imm, comment=f'var "{name}"')
                else:
                    self._push_lit(amount)
                    self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{name}"')
            elif amount.startswith("[expr ") and amount.endswith("]"):
                # Inline the expression: ``incr x [expr {-$y}]``
                inner = amount[1:-1]  # strip surrounding [ ]
                parts = inner.split(None, 1)
                if len(parts) == 2:
                    expr_body = parts[1]
                    # Strip outer braces: {-$i} → -$i
                    if expr_body.startswith("{") and expr_body.endswith("}"):
                        expr_body = expr_body[1:-1]
                    try:
                        node = _parse_expr_ast(expr_body)
                        self._emit_expr(node)
                        self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{name}"')
                    except Exception:
                        self._load_var(amount)
                        self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{name}"')
                else:
                    self._load_var(amount)
                    self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{name}"')
            else:
                # A braced whole-name amount ``incr x ${a($i)}`` loads the
                # WHOLE name (``push "a($i)"; loadStk``); bare ``$a($i)`` and
                # scalars go through the split ``_load_var`` path.
                if not self._emit_whole_name_array_load(amount):
                    var_ref = self._parse_simple_var_ref(amount)
                    self._load_var(var_ref if var_ref is not None else amount)
                self._emit(Op.INCR_SCALAR1, slot, comment=f'var "{name}"')
        else:
            arr = self._split_array_ref(name)
            if amount is None:
                if arr is not None:
                    self._push_lit(arr[0])
                    self._push_lit(arr[1])
                    self._emit(Op.INCR_ARRAY_STK_IMM, 1)
                else:
                    self._push_lit(name)
                    self._emit(Op.INCR_STK_IMM, 1)
            elif amount.lstrip("-").isdigit():
                imm = int(amount)
                if -128 <= imm <= 127:
                    if arr is not None:
                        self._push_lit(arr[0])
                        self._push_lit(arr[1])
                        self._emit(Op.INCR_ARRAY_STK_IMM, imm)
                    else:
                        self._push_lit(name)
                        self._emit(Op.INCR_STK_IMM, imm)
                else:
                    # Large increment — fall back to invokeStk
                    self._push_lit("incr")
                    self._push_lit(name)
                    self._push_lit(amount)
                    self._emit(Op.INVOKE_STK1, 3, comment="incr")
                    return
            else:
                # Variable amount — use incrStk (load amount from stack).
                # A braced whole-name amount ``incr x ${a($i)}`` loads the
                # WHOLE name (``push name; push "a($i)"; loadStk; incrStk``);
                # a scalar/array ``${var}`` amount loads via ``_load_var``.
                # Bare/other amounts fall back to the generic ``incr`` invoke.
                whole_amount = self._parse_whole_name_array_ref(amount)
                var_ref = self._parse_simple_var_ref(amount)
                if arr is None and whole_amount is not None:
                    self._push_lit(name)
                    self._push_lit(whole_amount)
                    self._emit(Op.LOAD_STK)
                    self._emit(Op.INCR_STK)
                elif arr is None and var_ref is not None:
                    self._push_lit(name)
                    self._load_var(var_ref)
                    self._emit(Op.INCR_STK)
                else:
                    self._push_lit("incr")
                    self._push_lit(name)
                    self._push_lit(amount)
                    self._emit(Op.INVOKE_STK1, 3, comment="incr")
                    return

    @staticmethod
    def _parse_simple_var_ref(value: str) -> str | None:
        """Extract variable name from a normalised ``${var}`` reference.

        The lowering pass normalises actual variable substitutions to
        ``${varname}``; bare ``$varname`` (from braced literals like
        ``{$x}``) is left as-is.  Only the ``${...}`` form is treated
        as a resolvable variable reference.

        Handles nested variable references like ``${::a(${::a(1)})}``
        where the inner ``${...}`` creates additional brace pairs.

        Returns the variable name, or ``None`` for anything else.
        """
        if not value.startswith("${") or not value.endswith("}"):
            return None
        # Verify the closing } matches the opening ${ by checking
        # balanced braces.  The opening { is at index 1.
        depth = 0
        for i, ch in enumerate(value):
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return value[2:-1] if i == len(value) - 1 else None
        return None

    @staticmethod
    def _parse_bare_array_ref(value: str) -> str | None:
        """Extract array reference from a bare ``$arr(idx)`` string.

        The lowering keeps bare form (no surrounding braces) when the
        index contains a nested ``$`` or ``[`` so the recursive
        substitution semantics survive.  Returns ``arr(idx)`` (without
        the leading ``$``), or ``None`` if *value* is not a bare
        array reference covering its entire span.

        ``$`` and ``[`` indices are both supported — ``_push_array_key``
        inlines the substitution at runtime so the index byte string
        seen by ``LOAD_ARRAY_STK`` is the resolved value.
        """
        if not value.startswith("$") or value.startswith("${"):
            return None
        if "(" not in value or not value.endswith(")"):
            return None
        # Find the first '(' — that splits the array name from the index.
        paren = value.index("(")
        # Reject if the array name (before '(') contains '$' or '[' —
        # we can only handle a literal array name here.
        if "$" in value[1:paren] or "[" in value[1:paren]:
            return None
        return value[1:]

    @staticmethod
    def _bare_var_ref_is_whole(elem: str) -> bool:
        """True if *elem* is a single bare ``$var`` / ``$arr(idx)`` reference
        spanning its whole length (no trailing literal, no second ref).

        ``$a`` and ``$::a(1)`` are whole; ``$a,hej`` and ``$a$b`` are
        templates that must go through the substitution path instead.
        Braced ``${...}`` forms are handled by the caller's earlier
        branch and always return False here.
        """
        if not elem.startswith("$") or elem.startswith("${"):
            return False
        i = 1
        n = len(elem)
        while i < n:
            ch = elem[i]
            if ch.isalnum() or ch == "_":
                i += 1
            elif ch == ":" and i + 1 < n and elem[i + 1] == ":":
                i += 2
            else:
                break
        if i == 1:
            return False  # bare "$" with no name (e.g. "$(...)")
        if i < n and elem[i] == "(":
            # ``$name(idx)`` — the index runs to the final ``)``.
            return elem.endswith(")")
        return i == n

    @staticmethod
    def _parse_whole_name_array_ref(value: str) -> str | None:
        """Extract the whole array-element name from a braced ``${a(1)}`` ref.

        tclsh 9 compiles braced ``${a(1)}`` by loading the WHOLE literal
        name ``a(1)`` — the runtime resolves it as array element ``a(1)``
        — via ``push "a(1)"; loadStk``.  This is distinct from bare
        ``$a(1)``, which SPLITS into array ``a`` element ``1``.

        Matches a braced reference ``${inner}`` whose *inner* name is
        array-shaped (contains ``(``, ends with ``)``) and has no ``}``
        inside it.  A plain braced scalar ``${foo}`` (inner has no
        ``(...)``) does NOT match — it must continue through the normal
        ``_parse_simple_var_ref`` → ``_load_var`` path (loadScalar in
        procs).  Returns *inner* for a whole-name load, or ``None``.
        """
        if not value.startswith("${") or not value.endswith("}"):
            return None
        inner = value[2:-1]
        if "}" in inner:
            return None
        if "(" in inner and inner.endswith(")"):
            return inner
        return None

    def _emit_whole_name_array_load(self: _Emitter, value: str) -> bool:
        """Emit a whole-name array load for a braced ``${a(...)}`` ref.

        If *value* is a braced whole-name array ref ``${a(...)}``, emit
        ``push <inner>; loadStk`` (the runtime resolves the array element
        from the whole literal name) and return True; otherwise return
        False.  Callers MUST try this BEFORE
        ``_parse_simple_var_ref``/``_load_var``, which would otherwise
        SPLIT ``${a($i)}`` into a substituted array read (``push "a";
        <load i>; loadArrayStk``), wrongly compiling braced ``${a($i)}``
        exactly like bare ``$a($i)``.
        """
        whole = self._parse_whole_name_array_ref(value)
        if whole is None:
            return False
        self._push_lit(whole)
        self._emit(Op.LOAD_STK)
        return True

    @staticmethod
    def _canonical_verbatim_ref(text: str) -> str:
        """Canonicalise a var-ref *text* for VERBATIM literal/comment emission.

        A few codegen sites (``dict set d k <val>``'s value push; the
        ``{*}``-expansion ``startCommand`` / ``invokeExpanded`` comments)
        surface the raw argument text rather than substituting it.  Bare
        array-shaped refs now carry their natural ``$a(1)`` spelling, but
        the canonical literal/comment form is the brace-wrapped
        ``${a(1)}`` (the form a scalar ref ``$foo`` already normalises to).
        Re-wrap a bare array ref so those verbatim sites stay stable;
        leave every other text (scalars, already-braced, plain literals,
        substituted-index refs) untouched.
        """
        if (
            text.startswith("$")
            and not text.startswith("${")
            and "(" in text
            and text.endswith(")")
            and "$" not in text[1:]
            and "[" not in text
        ):
            return "${" + text[1:] + "}"
        return text

    @staticmethod
    def _fold_cmd_args(value: str, prefix: str) -> str | None:
        """Constant-fold a command with all-literal args to a single string.

        *prefix* is the command text including trailing space, e.g.
        ``"list "`` or ``"dict create "``.  Returns ``None`` when the
        value doesn't match or contains substitutions.
        """
        full_prefix = f"[{prefix}"
        if not (value.startswith(full_prefix) and value.endswith("]")):
            return None
        inner = value[len(full_prefix) : -1]
        # Resolve backslash-newline continuations to a single space.
        inner = re.sub(r"\\\n\s*", " ", inner)
        # Cannot fold if UNBRACED arguments contain substitutions.
        # Content inside {braces} is literal and safe to fold.
        depth = 0
        for ch in inner:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            elif depth == 0 and ch in ("$", "["):
                return None
        # Parse the literal arguments and re-format as a canonical Tcl list.
        args = _split_list_simple(inner)
        return tcl_list_join(args)

    @staticmethod
    def _fold_list_cmd(value: str) -> str | None:
        """Constant-fold ``[list arg1 arg2 ...]`` to the result string."""
        return _ValuesMixin._fold_cmd_args(value, "list ")

    @staticmethod
    def _fold_dict_create_cmd(value: str) -> str | None:
        """Constant-fold ``[dict create k v ...]`` to the result string."""
        return _ValuesMixin._fold_cmd_args(value, "dict create ")

    _LIST_EXPAND_RE = re.compile(
        r"^\[list"
        r"((?:\s+\{\*\}\$\w+)+)"
        r"\s*\]$"
    )

    def _try_list_expand_concat(self: _Emitter, value: str) -> bool:
        """Compile ``[list {*}$a {*}$b]`` as ``load a; load b; listConcat``.

        Returns True if the pattern matched and was emitted.
        """
        m = self._LIST_EXPAND_RE.match(value)
        if m is None:
            return False
        # Extract the variable names from "{*}$varname" tokens.
        tokens = m.group(1).split()
        var_names = [t[4:] for t in tokens]  # strip "{*}$"
        if len(var_names) != 2:
            return False
        self._load_var(var_names[0])
        self._load_var(var_names[1])
        self._emit(Op.LIST_CONCAT)
        return True

    def _try_inline_list_with_break_continue(self: _Emitter, value: str) -> bool:
        """Compile ``[list arg ... [break] ...]`` or ``[list arg ... [continue] ...]`` inline.

        tclsh 9.0 compiles ``break``/``continue`` inside ``[list ...]``
        command substitutions as inline jumps with stack cleanup.  Values
        pushed before the break/continue are popped, then a ``jump4``
        goes to the break/continue target.  The remaining ``list N`` and
        any outer command instructions become dead code.
        """
        if not (value.startswith("[list ") and value.endswith("]")):
            return False
        parts = self._parse_cmd_parts(value)
        if not parts or parts[0][0] != "list":
            return False
        args = parts[1:]  # skip "list"
        has_bc = any(a in ("[break]", "[continue]") for a, *_ in args)
        if not has_bc:
            return False

        n_pushed = 0
        for arg, _braced, *_ in args:
            if arg == "[break]" and self._break_target is not None:
                end_label = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, end_label, 1)
                self._cmd_index += 1
                # Pop values already pushed for earlier list args.
                for _ in range(n_pushed):
                    self._emit(Op.POP)
                self._emit(Op.JUMP4, self._break_target, comment="break")
                self._place_label(end_label)
                n_pushed += 1  # dead-code placeholder
            elif arg == "[continue]" and self._continue_target is not None:
                end_label = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, end_label, 1)
                self._cmd_index += 1
                for _ in range(n_pushed):
                    self._emit(Op.POP)
                self._emit(Op.JUMP4, self._continue_target, comment="continue")
                self._place_label(end_label)
                n_pushed += 1
            else:
                self._emit_value(arg)
                n_pushed += 1
        self._emit(Op.LIST, len(args))
        return True

    def _emit_value(self: _Emitter, value: str, *, interpolate: bool = False) -> None:
        """Emit a value, resolving variable references.

        If *value* is a simple ``$var`` or ``${var}`` reference, emit a
        variable load.  When *interpolate* is True and the value contains
        embedded ``$var`` references mixed with literal text, decompose
        and emit ``push/load/strcat``.  Otherwise push the raw literal.
        """
        # Marker-wrapped values (braced strings from _process_literals)
        # must be pushed as-is — no variable resolution or interpolation.
        if value.startswith(_BRACE_OPEN) and value.endswith(_BRACE_CLOSE):
            self._push_lit(value)
            return
        # Braced array-shaped ref ${a(1)}: load the WHOLE name a(1) via
        # push + loadStk (the runtime resolves the array element), NOT a
        # split array load.  Must run before _parse_simple_var_ref, which
        # would otherwise route ${a(1)} through the split _load_var path.
        if self._emit_whole_name_array_load(value):
            return
        var_name = self._parse_simple_var_ref(value)
        if var_name is not None:
            self._load_var(var_name)
            return
        # Bare array reference with substituted index: ``$a($cmd)``.
        # The lowering preserves bare form when the index contains a
        # nested ``$`` or ``[`` so eval-fallback substitution still
        # works (cmdAH-1.4 / 1.5).  Emit it as an array load with the
        # index resolved at runtime.
        bare_arr = self._parse_bare_array_ref(value)
        if bare_arr is not None:
            self._load_var(bare_arr)
            return
        # Constant-fold [list arg1 arg2 ...] with all constant args.
        # Tcl 9.0 emits startCommand for the nested command substitution
        # even when the result is constant-folded, so wrap accordingly.
        folded = self._fold_list_cmd(value)
        if folded is not None:
            if not self._is_proc:
                end_label = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, end_label, 1)
                # Computed value — use no-dedup so later source literals
                # with the same string get their own pool entry.
                self._push_lit_no_dedup(folded)
                self._place_label(end_label)
                self._seen_generic_invoke = True
            else:
                self._push_lit_no_dedup(folded)
            return
        # Inline [list {*}$a {*}$b] → load a, load b, listConcat.
        # Tclsh 9.0 compiles this as a specialised two-list concatenation.
        if self._try_list_expand_concat(value):
            return
        # Inline [list arg ... [break] ...] or [list arg ... [continue] ...].
        # tclsh 9.0 compiles break/continue inside list as inline jumps.
        if self._try_inline_list_with_break_continue(value):
            return
        # Constant-fold [dict create k v ...] with all constant args.
        # Like list fold but also emits dup + verifyDict.
        folded_dict = self._fold_dict_create_cmd(value)
        if folded_dict is not None:
            if not self._is_proc:
                end_label = self._fresh_label("cmd_end")
                self._emit(Op.START_CMD, end_label, 1)
                self._push_lit(folded_dict)
                self._emit(Op.DUP)
                self._emit(Op.VERIFY_DICT)
                self._place_label(end_label)
                self._seen_generic_invoke = True
            else:
                self._push_lit(folded_dict)
                self._emit(Op.DUP)
                self._emit(Op.VERIFY_DICT)
            return
        # Try interpolated string: "Hello, $name!" or "Result: [expr {1+2}]"
        if interpolate and ("$" in value or "[" in value):
            parts = _parse_subst_template(value)
            if parts is not None:
                # A single-part subst (a bare ``[cmd]`` or ``$var``)
                # still needs unwrapping — without this, the value is
                # pushed as the literal source text instead of the
                # substituted value.
                if len(parts) == 1:
                    kind, part_val = parts[0]
                    if kind == "cmd":
                        self._emit_inline_cmd_subst(part_val)
                        return
                    if kind == "scalar":
                        self._push_lit(part_val)
                        self._emit(Op.LOAD_STK)
                        return
                    if kind == "var":
                        self._load_var(part_val)
                        return
                if len(parts) > 1:
                    for kind, part_val in parts:
                        if kind == "lit":
                            # Use raw push so braces/dollars in the literal
                            # part are not confused with brace-wrapping or
                            # runtime substitution markers.
                            self._push_raw_lit(part_val)
                        elif kind == "cmd":
                            # tclsh wraps the inline ``[cmd]`` inside
                            # a strcat-style interpolation in its own
                            # ``startCommand`` only when the resolved
                            # command name is a fully-qualified
                            # ensemble call (``::tcl::dict::merge``,
                            # ``::tcl::string::equal`` …).  Plain
                            # cmds (``[lreverse $lst]``) inline
                            # directly without an inner SC.  We
                            # cheaply detect the FQ-ensemble shape by
                            # parsing the head of *part_val*.
                            head = ""
                            inner_text = part_val.lstrip()
                            if inner_text.startswith("[") and inner_text.endswith("]"):
                                inner_inner = inner_text[1:-1].lstrip()
                                head = inner_inner.split(None, 1)[0] if inner_inner else ""
                            if head in _ENSEMBLE_INTERP_HEADS:
                                _prev_inline_used = self._used_inline_cmd_subst
                                sc_end = self._fresh_label("interp_cmd_end")
                                self._emit(Op.START_CMD, sc_end, 1)
                                self._emit_inline_cmd_subst(part_val)
                                self._place_label(sc_end)
                                self._used_inline_cmd_subst = _prev_inline_used
                            else:
                                self._emit_inline_cmd_subst(part_val)
                        elif kind == "scalar":
                            # Braced scalar: push name + loadStk.
                            self._push_lit(part_val)
                            self._emit(Op.LOAD_STK)
                        else:
                            self._load_var(part_val)
                    self._emit(Op.STR_CONCAT1, len(parts))
                    return
        # If the value looks like a braced string (starts/ends with {/}),
        # push with the raw prefix so the PUSH handler doesn't strip
        # another brace level — the parser already de-braced once.
        if value.startswith("{") and value.endswith("}") and "$" not in value and "[" not in value:
            self._push_raw_lit(value)
        else:
            self._push_lit(value)

    def _emit_list_arg(self: _Emitter, value: str) -> None:
        """Emit a ``list`` argument, inlining command substitutions.

        Like ``_emit_value`` but also handles ``[cmd arg ...]``
        command substitutions inline.  Used exclusively by the
        body-level ``list`` handler where the args are known to
        be ``list`` arguments (not arbitrary values that might be
        braced literals like regexp character classes).
        """
        # Brace-marked values (VM pipeline) → push as-is.
        if value.startswith(_BRACE_OPEN) and value.endswith(_BRACE_CLOSE):
            self._push_lit(value)
            return
        # Braced array-shaped ref ${a(1)} → load WHOLE name a(1) via
        # push + loadStk (runtime resolves the array element).
        if self._emit_whole_name_array_load(value):
            return
        # Variable reference → load var.
        var_name = self._parse_simple_var_ref(value)
        if var_name is not None:
            self._load_var(var_name)
            return
        # Bare array reference ``$a(1)`` / ``$a($i)`` → split array load
        # (loadArrayStk), the same as _emit_value.  Without this a bare array
        # arg falls through to a literal push (it used to be normalised to
        # ``${a(1)}`` and caught by _parse_simple_var_ref above).
        bare_arr = self._parse_bare_array_ref(value)
        if bare_arr is not None:
            self._load_var(bare_arr)
            return
        # [cmd arg ...] command substitution → compile inline.
        # Require whitespace (like ``[array exists a]``); skip
        # bare ``[x]`` or expansion ``{*}`` patterns.
        if (
            value.startswith("[")
            and value.endswith("]")
            and " " in value[1:-1]
            and "{*}" not in value
        ):
            self._emit_inline_cmd_subst(value)
            return
        # Fall through: push as literal (includes constant values
        # and braced values without markers).
        # If the value looks like a braced string (balanced {…}), push
        # with the raw prefix so the PUSH handler doesn't strip another
        # brace level — the parser already de-braced once.
        if value.startswith("{") and value.endswith("}") and "$" not in value and "[" not in value:
            self._push_raw_lit(value)
        else:
            self._push_lit(value)

    def _emit_return_value(self: _Emitter, value: str) -> None:
        """Emit the value for a ``return`` statement.

        When *value* is a ``[expr {...}]`` command substitution in a proc
        body, inline-compile the expression instead of pushing it as a
        literal string.  Otherwise fall back to ``_emit_value``.
        """
        if self._is_proc and value.startswith("[") and value.endswith("]"):
            self._emit_inline_cmd_subst(value)
            return
        self._emit_value(value, interpolate=True)

    @staticmethod
    def _try_format_fold(value: str) -> str | None:
        """Constant-fold ``[format "..." arg ...]`` → result string.

        Only handles simple ``%s`` and ``%d`` conversions with literal args.
        Returns ``None`` if the format cannot be folded.
        """
        if not value.startswith("[format ") or not value.endswith("]"):
            return None
        inner = value[1:-1]  # strip [ ]
        # Parse the command parts
        parts: list[str] = []
        i = len("format ")
        while i < len(inner):
            ch = inner[i]
            if ch in (" ", "\t"):
                i += 1
                continue
            if ch == '"':
                # Quoted string
                i += 1
                buf: list[str] = []
                while i < len(inner) and inner[i] != '"':
                    if inner[i] == "\\":
                        buf.append(inner[i : i + 2])
                        i += 2
                    else:
                        buf.append(inner[i])
                        i += 1
                if i < len(inner):
                    i += 1  # skip closing "
                parts.append("".join(buf))
            elif ch == "{":
                depth = 0
                start = i
                while i < len(inner):
                    if inner[i] == "{":
                        depth += 1
                    elif inner[i] == "}":
                        depth -= 1
                        if depth == 0:
                            i += 1
                            break
                    i += 1
                parts.append(inner[start + 1 : i - 1])
            else:
                start = i
                while i < len(inner) and inner[i] not in (" ", "\t"):
                    i += 1
                parts.append(inner[start:i])
        if len(parts) < 1:
            return None
        fmt = parts[0]
        args = parts[1:]
        # Simple format: only handle %s and %d
        result: list[str] = []
        ai = 0
        fi = 0
        while fi < len(fmt):
            if fmt[fi] == "%" and fi + 1 < len(fmt):
                spec = fmt[fi + 1]
                if spec == "s" and ai < len(args):
                    result.append(args[ai])
                    ai += 1
                    fi += 2
                elif spec == "d" and ai < len(args):
                    try:
                        result.append(str(int(args[ai])))
                    except ValueError:
                        return None
                    ai += 1
                    fi += 2
                elif spec == "%":
                    result.append("%")
                    fi += 2
                else:
                    return None  # unsupported format spec
            else:
                result.append(fmt[fi])
                fi += 1
        return "".join(result)
