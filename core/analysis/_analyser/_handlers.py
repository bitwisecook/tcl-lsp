from __future__ import annotations

import logging

from ...commands.registry import REGISTRY
from ...common.alias import detect_interp_alias, resolve_alias
from ...common.dialect import active_dialect
from ...common.ranges import range_from_token
from ...parsing.tokens import Token, TokenType
from ..semantic_model import (
    Scope,
)

log = logging.getLogger(__name__)


class _AnalyserHandlersMixin:
    """Special-case command handlers."""

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
                    self._define_var(arg_text, arg_tokens[i], scope, warn_if_unused=False)
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
