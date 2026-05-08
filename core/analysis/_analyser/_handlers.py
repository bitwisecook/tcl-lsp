from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...commands.registry import REGISTRY
from ...common.alias import detect_interp_alias, resolve_alias
from ...common.dialect import active_dialect
from ...common.ranges import range_from_token
from ...parsing.tokens import Token, TokenType
from ..semantic_model import (
    Scope,
)

log = logging.getLogger(__name__)


class _AnalyserHandlersMixin(_Base):
    """Special-case command handlers."""

    if TYPE_CHECKING:
        # From _AnalyserProcMixin
        def _handle_proc(self, *a: Any, **kw: Any) -> None: ...
        def _handle_set(self, *a: Any, **kw: Any) -> None: ...
        def _handle_switch(self, *a: Any, **kw: Any) -> None: ...
        def _handle_try(self, *a: Any, **kw: Any) -> None: ...
        # From _AnalyserScopeMixin
        def _set_const_string(self, *a: Any, **kw: Any) -> None: ...
        def _clear_const_string(self, *a: Any, **kw: Any) -> None: ...
        def _lookup_const_string(self, *a: Any, **kw: Any) -> Any: ...
        def _define_var(self, *a: Any, **kw: Any) -> Any: ...
        def _define_vars_from_list(self, *a: Any, **kw: Any) -> None: ...
        def _namespace_from_scope(self, *a: Any, **kw: Any) -> str: ...

    def _handle_proc_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "proc" or len(args) < 3:
            return False
        self._handle_proc(args, arg_tokens, scope)
        return True

    def _handle_set_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name != "set" or not args:
            return

        self._handle_set(args, arg_tokens, scope)

        # Track constant string values for regex variable propagation.
        if len(args) < 2 or len(arg_tokens) < 2:
            return
        value_token = arg_tokens[1]
        if value_token.type in (TokenType.ESC, TokenType.STR) and value_token.text == args[1]:
            self._set_const_string(args[0], args[1], range_from_token(value_token), scope)
            return
        self._clear_const_string(args[0], scope)

    def _handle_var_declaration_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name not in ("variable", "global") or not args:
            return

        if cmd_name == "global":
            for i, arg_text in enumerate(args):
                if i < len(arg_tokens):
                    # Tcl strips any namespace qualifiers from the alias
                    # name -- ``global ::foo`` creates a local variable
                    # called ``foo`` (not ``::foo``).
                    local_name = arg_text.rsplit("::", 1)[-1] or arg_text
                    self._define_var(local_name, arg_tokens[i], scope, warn_if_unused=False)
            return

        i = 0
        while i < len(args):
            if i < len(arg_tokens):
                self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
            if i + 1 < len(args):
                i += 2
            else:
                i += 1

    def _handle_namespace_eval_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "namespace" or len(args) < 2 or args[0] != "eval":
            return False

        ns_name = args[1]
        ns_scope = Scope(
            kind="namespace",
            name=ns_name,
            parent=scope,
            body_range=range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else None,
        )
        scope.children.append(ns_scope)
        if len(args) >= 3:
            body_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
            self._analyse_body(args[2], ns_scope, body_token=body_tok)
        return True

    def _handle_foreach_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name == "foreach_in_collection":
            # Only treat as a loop when enabled in the active dialect.
            if REGISTRY.get(cmd_name, active_dialect()) is None:
                return False
        elif cmd_name != "foreach":
            return False
        if len(args) < 3:
            return False

        tok = arg_tokens[0] if arg_tokens else None
        if tok:
            self._define_vars_from_list(args[0], tok, scope)

        body_tok = arg_tokens[-1] if arg_tokens else None
        self._analyse_body(args[-1], scope, body_token=body_tok)
        return True

    def _handle_for_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        if cmd_name != "for" or len(args) < 4:
            return False

        init_tok = arg_tokens[0] if len(arg_tokens) > 0 else None
        test_tok = arg_tokens[1] if len(arg_tokens) > 1 else None
        next_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
        body_tok = arg_tokens[3] if len(arg_tokens) > 3 else None
        self._analyse_body(args[0], scope, body_token=init_tok)
        self._analyse_expr(args[1], scope, expr_token=test_tok)
        self._analyse_body(args[2], scope, body_token=next_tok)
        self._analyse_body(args[3], scope, body_token=body_tok)
        return True

    def _handle_switch_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "switch" or len(args) < 2:
            return False

        self._handle_switch(args, arg_tokens, scope)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_catch_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "catch" or not args:
            return False

        catch_body_tok = arg_tokens[0] if arg_tokens else None
        self._conditional_depth += 1
        self._analyse_body(args[0], scope, body_token=catch_body_tok)
        self._conditional_depth -= 1
        for i in range(1, min(len(args), 3)):
            if i < len(arg_tokens):
                self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_try_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        if cmd_name != "try" or not args:
            return False

        self._handle_try(args, arg_tokens, scope)
        # Arity now checked by compiler_checks._arity_checks via IR.
        return True

    def _handle_incr_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        if cmd_name == "incr" and args and arg_tokens:
            self._define_var(args[0], arg_tokens[0], scope, warn_if_unused=True)

    def _handle_upvar_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Register the local-alias names from ``upvar`` so they show up in
        completion / hover / references even when the proc only reads them.

        ``upvar ?level? otherVar myVar ?otherVar myVar ...?`` -- the level
        is optional; when present it makes the arg count odd.  Each pair
        after the (optional) level binds ``myVar`` (the local alias) to
        ``otherVar`` (in another stack frame)."""
        if cmd_name != "upvar" or not args or not arg_tokens:
            return
        # If arg count is odd, the first arg is ``?level?``; pairs follow.
        pair_start = 1 if len(args) % 2 == 1 else 0
        i = pair_start + 1  # myVar position within the first pair
        while i < len(args) and i < len(arg_tokens):
            self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
            i += 2

    def _handle_namespace_upvar_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """``namespace upvar nsname otherVar myVar ?otherVar myVar ...?``
        -- each ``myVar`` is a local alias to a namespace variable."""
        if cmd_name != "namespace" or len(args) < 4 or args[0] != "upvar":
            return
        # args = ["upvar", nsname, otherVar1, myVar1, otherVar2, myVar2, ...].
        # myVar lives at indices 3, 5, 7, ...
        i = 3
        while i < len(args) and i < len(arg_tokens):
            self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
            i += 2

    def _handle_uplevel_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> bool:
        """``uplevel ?level? body ?body ...?`` runs the body in another
        stack frame.  We can model ``uplevel #0`` (absolute global frame)
        by analysing the body in a synthetic scope whose lexical parent
        in the tree is ``scope`` (so ``find_scope_at_line`` descends into
        it) but whose ``parent`` reference is the global scope (so
        variable lookup chains skip the calling proc's locals -- those
        aren't reachable from the global frame at runtime).  Other levels
        depend on the dynamic caller and aren't resolvable statically."""
        if cmd_name != "uplevel" or not args or not arg_tokens:
            return False
        if args[0] != "#0":
            return False
        global_scope = self.result.global_scope
        for i in range(1, len(args)):
            if i >= len(arg_tokens):
                continue
            body_tok = arg_tokens[i]
            synthetic = Scope(
                kind="uplevel",
                name="<uplevel#0>",
                parent=global_scope,
                body_range=range_from_token(body_tok),
            )
            scope.children.append(synthetic)
            self._analyse_body(args[i], synthetic, body_token=body_tok)
        return True

    def _handle_dict_var_command(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Register loop / alias variables introduced by ``dict for`` /
        ``dict update`` / ``dict with``.

        - ``dict for {keyVar valueVar} dictValue body`` -- keyVar/valueVar
          are loop locals visible in body.
        - ``dict update dictVar key1 var1 ?key2 var2 ...? body`` -- each
          ``var`` is a local alias to the matching key during body.
        - ``dict with dictVar ?key ...? body`` -- the dict's keys become
          local vars during body.  Best effort: when the dict was set
          from a constant literal we parse its keys; otherwise the body
          is analysed as before and references to dict keys won't have
          a definition site."""
        if cmd_name != "dict" or not args or not arg_tokens:
            return
        sub = args[0]
        if sub == "for" and len(args) >= 4 and len(arg_tokens) >= 2:
            self._define_vars_from_list(args[1], arg_tokens[1], scope)
            return
        if sub == "update" and len(args) >= 5 and (len(args) - 3) % 2 == 0:
            # args = ["update", dictVar, key1, var1, key2, var2, ..., body].
            # var positions: 3, 5, 7, ..., len-2.
            for i in range(3, len(args) - 1, 2):
                if i < len(arg_tokens):
                    self._define_var(args[i], arg_tokens[i], scope, warn_if_unused=False)
            return
        if sub == "with" and len(args) >= 3 and len(arg_tokens) >= 2:
            # ``dict with dictVar ?key ...? body`` -- only the no-path
            # case is statically resolvable.  Path keys are intermediate
            # navigation that we'd need to evaluate to know the leaf
            # dict's keys, so skip them.
            if len(args) != 3:
                return
            dict_var = args[1]
            const_val = self._lookup_const_string(dict_var, scope)
            if const_val is None:
                return
            from ...compiler.tcl_expr_eval import _split_tcl_list

            try:
                elements = _split_tcl_list(const_val)
            except Exception:
                return
            # Tcl dicts are flat key/value lists; even-indexed entries
            # are the keys.
            for i in range(0, len(elements), 2):
                key = elements[i]
                if not key:
                    continue
                self._define_var(key, arg_tokens[1], scope, warn_if_unused=False)

    def _handle_interp_alias(self, cmd_name: str, args: list[str]) -> None:
        """Detect ``interp alias {} srcToken {} targetCmd ?arg ...?``.

        Records the alias so that argument roles (EXPR, BODY, etc.) from
        the target command are applied when the alias is invoked.
        """
        detected = detect_interp_alias(cmd_name, args)
        if detected is None:
            return
        qualified, target_cmd, prepended = detected
        self._command_aliases[qualified] = (target_cmd, prepended)
        self.result.command_aliases[qualified] = (target_cmd, prepended)

    def _handle_oo_objdefine(self, cmd_name: str, args: list[str]) -> None:
        """Detect ``oo::objdefine $obj ...`` and record the object variable.

        Objects modified via ``oo::objdefine`` may have per-instance methods
        that are not in the class definition.  We suppress W308 for these
        objects to avoid false positives.
        """
        if cmd_name != "oo::objdefine" or not args:
            return
        obj_name = args[0].strip()
        # Strip $ prefix for variable references.
        if obj_name.startswith("$"):
            obj_name = obj_name.lstrip("$").strip("{}")
        if obj_name:
            self._objdefined_vars.add(obj_name)

    def _handle_trace_command(
        self,
        cmd_name: str,
        args: list[str],
        scope: Scope,
    ) -> None:
        """Detect ``trace`` callback registrations and mark target procs.

        Recognises:
          - ``trace add variable|command|execution name ops commandPrefix``
            (Tcl 8.4+)
          - ``trace variable name ops command`` (legacy form, deprecated
            but still in use)

        When the commandPrefix resolves to a known user proc, the
        ``ProcDef.is_trace_callback`` flag is set so that W214
        (unused parameter) is suppressed: trace callbacks must accept a
        fixed trailing signature (e.g. ``name1 name2 op``) regardless of
        whether the body uses those arguments.
        """
        if cmd_name not in ("trace", "::trace"):
            return

        prefix: str | None = None
        if (
            len(args) >= 5
            and args[0] == "add"
            and args[1]
            in (
                "variable",
                "command",
                "execution",
            )
        ):
            prefix = args[4]
        elif len(args) >= 4 and args[0] == "variable":
            prefix = args[3]

        if not prefix:
            return

        from ...compiler.tcl_expr_eval import _split_tcl_list

        try:
            elements = _split_tcl_list(prefix)
        except Exception:
            return
        if not elements:
            return
        callback_name = elements[0]
        if not callback_name:
            return

        # Defer resolution until finalisation: the proc may be defined
        # after the ``trace add`` site.  Pre-compute the candidate
        # qualified names so the snapshot does not need to retain a
        # scope reference.
        candidates = self._trace_callback_candidates(callback_name, scope)
        if candidates:
            self._pending_trace_callbacks.append(candidates)

    @staticmethod
    def _trace_callback_candidates(cmd_name: str, scope: Scope) -> tuple[str, ...]:
        """Enumerate qualified-name candidates for an unqualified callback.

        Tcl resolves the trace ``commandPrefix`` at trace-fire time using
        the namespace active at the variable-write site, which the
        analyser cannot know statically — it may be the registration
        namespace, the global namespace, or any caller's namespace.  We
        therefore enumerate every namespace ancestor plus the global
        namespace as plausible candidates and let the resolver mark all
        existing procs that match.  Storing a tuple instead of keeping a
        scope reference keeps the pending list snapshot-safe.
        """
        from ...common.naming import normalise_qualified_name as _norm

        seen: set[str] = set()
        result: list[str] = []

        def add(name: str) -> None:
            qname = _norm(name)
            if not qname or qname in seen:
                return
            seen.add(qname)
            result.append(qname)

        if cmd_name.startswith("::"):
            add(cmd_name)
        elif "::" in cmd_name:
            add(f"::{cmd_name}")
        else:
            current: Scope | None = scope
            while current is not None:
                if current.kind == "namespace":
                    add(f"{current.name}::{cmd_name}")
                current = current.parent
            add(f"::{cmd_name}")
        return tuple(result)

    def _resolve_pending_trace_callbacks(self) -> None:
        """Resolve deferred ``trace`` callback registrations.

        Marks every ``ProcDef`` matching any candidate qualified name
        with ``is_trace_callback = True``.  Marking *all* matches (rather
        than only the first) reflects Tcl's runtime resolution rules:
        the callback's actual target depends on the namespace active at
        the variable-write site, so any matching proc could legitimately
        receive the trace arguments.  Suppressing W214 on every plausible
        target avoids false positives at the small cost of suppressing
        a hint on a same-named proc that happens not to be the callback.
        """
        if not self._pending_trace_callbacks:
            return
        for candidates in self._pending_trace_callbacks:
            for qname in candidates:
                proc = self.result.all_procs.get(qname)
                if proc is not None:
                    proc.is_trace_callback = True
        self._pending_trace_callbacks.clear()

    def _handle_namespace_ensemble(
        self,
        cmd_name: str,
        args: list[str],
        scope: Scope,
    ) -> None:
        """Detect ``namespace ensemble create`` and record the namespace."""
        if cmd_name != "namespace" or len(args) < 2:
            return
        if args[0] != "ensemble" or args[1] != "create":
            return
        # The ensemble command name is derived from the current namespace.
        ns = self._namespace_from_scope(scope)
        if ns and ns != "::":
            self._ensemble_namespaces.add(ns)

    def _resolve_alias(
        self, cmd_name: str, args: list[str], scope: Scope | None = None
    ) -> tuple[str, list[str]]:
        """Resolve a command alias to (target_cmd, effective_args).

        If *cmd_name* is a known alias, returns the target command name
        and the effective argument list (prepended args + original args).
        Otherwise returns the original cmd_name and args unchanged.

        Delegates to the shared ``resolve_alias()`` utility with the
        namespace derived from the current scope chain.
        """
        ns = self._namespace_from_scope(scope) if scope is not None else "::"
        alias = resolve_alias(cmd_name, self._command_aliases, namespace=ns)
        if alias is not None:
            target_cmd, prepended = alias
            return target_cmd, list(prepended) + args
        return cmd_name, args
