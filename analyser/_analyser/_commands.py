# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from compiler.registry import REGISTRY
from compiler.registry.dialect import active_dialect
from compiler.registry.runtime import (
    SIGNATURES,
    ArgRole,
    CommandSig,
    SubcommandSig,
    arg_indices_for_role,
    body_arg_implicit_args_for_command,
    command_is_disabled_in_active_dialect,
    iter_body_arguments,
)
from shared.ranges import range_from_token
from shared.tokens import Token, TokenType

from ..semantic_model import (
    AutoPathEntry,
    CodeFix,
    CommandInvocation,
    Diagnostic,
    NamespaceImport,
    PackageProvide,
    PackageRequire,
    RegexPattern,
    Scope,
    Severity,
    SourceTarget,
)
from ._utils import (
    _argv_with_word_spans,
    _irules_top_level_only,
)

log = logging.getLogger(__name__)


class _AnalyserCommandsMixin(_Base):
    """Main command dispatch: _process_command and special-case routing."""

    if TYPE_CHECKING:
        # From _AnalyserProcMixin
        def _resolve_proc_call(self, *a: Any, **kw: Any) -> Any: ...
        def _check_proc_call_arity(self, *a: Any, **kw: Any) -> None: ...
        # From _AnalyserHandlersMixin
        def _resolve_alias(self, *a: Any, **kw: Any) -> Any: ...
        def _handle_proc_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_oo_class_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_oo_define_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_snit_type_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_oo_objdefine(self, *a: Any, **kw: Any) -> None: ...
        def _handle_set_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_var_declaration_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_namespace_eval_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_foreach_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_for_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_switch_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_catch_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_try_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_incr_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_upvar_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_namespace_upvar_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_uplevel_command(self, *a: Any, **kw: Any) -> bool: ...
        def _handle_dict_var_command(self, *a: Any, **kw: Any) -> None: ...
        def _handle_interp_alias(self, *a: Any, **kw: Any) -> None: ...
        def _handle_namespace_ensemble(self, *a: Any, **kw: Any) -> None: ...
        def _handle_trace_command(self, *a: Any, **kw: Any) -> None: ...
        # From _AnalyserScopeMixin
        def _namespace_from_scope(self, *a: Any, **kw: Any) -> str: ...
        def _define_var(self, *a: Any, **kw: Any) -> Any: ...
        def _lookup_const_string(self, *a: Any, **kw: Any) -> Any: ...
        def _record_defining_set_as_regex(self, *a: Any, **kw: Any) -> None: ...

    def _signature_for_command(self, cmd_name: str) -> CommandSig | SubcommandSig | None:
        """Resolve signature metadata from the command registry."""
        dialect = active_dialect()
        spec = REGISTRY.get(cmd_name, dialect)
        if spec is not None and spec.subcommands:
            dialect_subs = spec.subcommands_for_dialect(dialect)
            return SubcommandSig(
                subcommands={
                    name: CommandSig(
                        arity=sub.arity,
                        arg_roles=dict(sub.arg_roles) if sub.arg_roles else {},
                    )
                    for name, sub in dialect_subs.items()
                },
                allow_unknown=spec.allow_unknown_subcommands,
            )
        validation = REGISTRY.validation(cmd_name, dialect)
        if validation is not None:
            return CommandSig(arity=validation.arity)
        return SIGNATURES.get(cmd_name)

    @staticmethod
    def _scope_is_method(scope: Scope) -> bool:
        """Return True if *scope* or any ancestor is a TclOO method body."""
        s: Scope | None = scope
        while s is not None:
            if s.kind == "method":
                return True
            s = s.parent
        return False

    # Keywords that are only valid as arguments within a parent command.
    # When they appear as standalone commands it almost always means a
    # misplaced newline split the parent command (e.g. ``}\nelse {`` instead
    # of ``} else {``).  Maps keyword → parent command name.
    _ORPHANED_KEYWORDS: dict[str, str] = {
        "else": "if",
        "elseif": "if",
        "then": "if",
        "on": "try",
        "trap": "try",
        "finally": "try",
    }

    def _process_command(
        self,
        argv: list[Token],
        argv_texts: list[str],
        all_tokens: list[Token],
        scope: Scope,
        source: str,
        expand_word: list[bool] | None = None,
        single_token_word: list[bool] | None = None,
    ) -> None:
        """Process a single command (argv[0] is the command name).

        ``expand_word`` (when provided) is a list parallel to ``argv`` whose
        entries are ``True`` for words that were preceded by the Tcl 8.5+
        ``{*}`` expansion prefix.  Arity checks must treat expanded words
        as an unknown number of runtime arguments rather than counting
        them as one.

        ``single_token_word`` (when provided) is a list parallel to
        ``argv`` whose entries are ``True`` when the word consists of a
        single atomic token.  Used to gate constant-folding of expanded
        words: a multi-token word like ``$x$y`` exposes only its first
        token in ``argv``, so refinement based on that single token
        would be incorrect.
        """
        if not argv:
            return

        argv = _argv_with_word_spans(argv, all_tokens)
        cmd_name = argv_texts[0]
        args = argv_texts[1:]
        arg_tokens = argv[1:]
        arg_expand = list(expand_word[1:]) if expand_word else None
        arg_single_token = list(single_token_word[1:]) if single_token_word else None
        resolved_proc = self._resolve_proc_call(cmd_name, scope)
        self.result.command_invocations.append(
            CommandInvocation(
                name=cmd_name,
                range=range_from_token(argv[0]),
                resolved_qualified_name=resolved_proc.qualified_name if resolved_proc else None,
                enclosing_namespace=self._namespace_from_scope(scope),
            )
        )

        # W125 — orphaned control-flow keyword used as standalone command.
        # Suppressed when a user proc with the same name is in scope.
        if (
            cmd_name in self._ORPHANED_KEYWORDS
            and resolved_proc is None
            and REGISTRY.get(cmd_name, active_dialect()) is None
        ):
            parent = self._ORPHANED_KEYWORDS[cmd_name]
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(argv[0]),
                    message=(
                        f'"{cmd_name}" used as standalone command'
                        f' — should be part of "{parent}"'
                        f" (check for misplaced newline)"
                    ),
                    severity=Severity.WARNING,
                    code="W125",
                )
            )

        # Record variable-as-command sites for post-analysis method resolution.
        # When ``$obj method ...`` is used, the type lattice post-pass checks
        # whether ``obj`` holds a TclOO object and validates the method name.
        # Use cmd_tok.text (raw var name "obj") not cmd_name ("${obj}").
        cmd_tok = all_tokens[0] if all_tokens else None
        # The command word is a single atomic token only when ``argv[0]``
        # was flagged in ``single_token_word``.  Composite words like
        # ``${cmd}x`` start with a VAR/CMD token but dispatch to the
        # concatenated string, not the substitution's value.
        cmd_word_single = bool(single_token_word[0]) if single_token_word else False
        if cmd_tok is not None and cmd_tok.type is TokenType.VAR:
            method_name = args[0] if args else None
            in_method = self._scope_is_method(scope)
            self._var_command_sites.append(
                (
                    cmd_tok.text,
                    method_name,
                    range_from_token(cmd_tok),
                    in_method,
                    cmd_word_single,
                    len(args),  # positional_arg_count -- see _core.py docstring
                )
            )
        elif cmd_tok is not None and cmd_tok.type is TokenType.CMD:
            method_name = args[0] if args else None
            in_method = self._scope_is_method(scope)
            self._cmd_command_sites.append(
                (
                    cmd_tok.text,
                    method_name,
                    range_from_token(cmd_tok),
                    in_method,
                    cmd_word_single,
                )
            )

        # IRULE5005: direct proc invocation without ``call`` in iRules.
        # In iRules, procs must be invoked via ``call proc_name``, not
        # directly as ``proc_name arg...``.
        if (
            resolved_proc is not None
            and cmd_name != "call"
            and self._current_event is not None
            and active_dialect() == "f5-irules"
        ):
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(argv[0]),
                    message=(
                        f"iRules procs must be invoked with 'call': "
                        f"call {cmd_name}" + ((" " + " ".join(args)) if args else "")
                    ),
                    severity=Severity.ERROR,
                    code="IRULE5005",
                    fixes=(
                        CodeFix(
                            range=range_from_token(argv[0]),
                            new_text=f"call {cmd_name}",
                            description=f"Use 'call {cmd_name}'",
                        ),
                    ),
                )
            )

        # iRules ``call proc_name arg...`` — record an additional
        # CommandInvocation for the target proc so that references,
        # rename, and call-hierarchy see through the indirection.
        if cmd_name == "call" and args and arg_tokens and active_dialect() == "f5-irules":
            call_target_name = args[0]
            call_target_proc = self._resolve_proc_call(call_target_name, scope)
            self.result.command_invocations.append(
                CommandInvocation(
                    name=call_target_name,
                    range=range_from_token(arg_tokens[0]),
                    resolved_qualified_name=(
                        call_target_proc.qualified_name if call_target_proc else None
                    ),
                    enclosing_namespace=self._namespace_from_scope(scope),
                )
            )

        # IRULE5006: top-level-only commands used inside a nested body.
        if (
            self._body_depth > 0
            and active_dialect() == "f5-irules"
            and cmd_name in _irules_top_level_only()
        ):
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(argv[0]),
                    message=f"'{cmd_name}' is only valid at the top level of an iRule.",
                    severity=Severity.WARNING,
                    code="IRULE5006",
                )
            )

        # IRULE5007: event-context command used at top level outside a when block.
        if (
            self._body_depth == 0
            and self._current_event is None
            and active_dialect() == "f5-irules"
            and cmd_name not in _irules_top_level_only()
        ):
            spec = REGISTRY.get(cmd_name, "f5-irules")
            if spec is not None and spec.event_requires is not None:
                self.result.diagnostics.append(
                    Diagnostic(
                        range=range_from_token(argv[0]),
                        message=f"'{cmd_name}' requires an event context — use it inside a `when` block.",
                        severity=Severity.WARNING,
                        code="IRULE5007",
                    )
                )

        # Record package require/provide statements for cross-file resolution.
        if cmd_name == "package" and args:
            if args[0] == "require":
                pkg_name = args[1] if len(args) >= 2 else None
                if pkg_name:
                    # Strip leading -exact flag if present
                    pkg_version: str | None = None
                    if pkg_name == "-exact" and len(args) >= 3:
                        pkg_name = args[2]
                        pkg_version = args[3] if len(args) >= 4 else None
                    else:
                        pkg_version = args[2] if len(args) >= 3 else None
                    # Detect dynamic package names (variable or command
                    # substitution).  When the name is not a static
                    # literal we cannot know which packages are loaded,
                    # so mark the file as having dynamic providers.
                    pkg_arg_idx = 2 if args[0] == "-exact" else 1
                    if pkg_arg_idx < len(arg_tokens):
                        _pt = arg_tokens[pkg_arg_idx]
                        if (
                            _pt.type in (TokenType.VAR, TokenType.CMD)
                            or "$" in pkg_name
                            or "[" in pkg_name
                        ):
                            self.result.has_dynamic_providers = True
                    self.result.package_requires.append(
                        PackageRequire(
                            name=pkg_name,
                            version=pkg_version,
                            range=range_from_token(argv[0]),
                            conditional=self._conditional_depth > 0,
                        )
                    )
            elif args[0] == "provide":
                pkg_name = args[1] if len(args) >= 2 else None
                if pkg_name:
                    pkg_version = args[2] if len(args) >= 3 else None
                    self.result.package_provides.append(
                        PackageProvide(
                            name=pkg_name,
                            version=pkg_version,
                            range=range_from_token(argv[0]),
                        )
                    )

        # Detect dynamic providers: commands that create or load commands
        # at runtime, making static command resolution unreliable.
        if cmd_name == "load":
            self.result.has_dynamic_providers = True
        elif cmd_name in ("set", "lappend") and args and args[0] == "auto_path":
            self.result.has_dynamic_providers = True
            # Record each path argument so ``_load_packages_if_needed`` can
            # extend the package resolver's search paths for this document.
            for i in range(1, len(args)):
                if i >= len(arg_tokens):
                    break
                self.result.auto_path_entries.append(
                    AutoPathEntry(
                        resolved_path=None,
                        raw=args[i],
                        range=range_from_token(arg_tokens[i]),
                    )
                )
        elif cmd_name == "rename":
            self.result.has_dynamic_providers = True
        elif cmd_name == "namespace" and len(args) >= 2 and args[0] == "import":
            self.result.has_dynamic_providers = True
            # ``scope.name`` is only the *local* namespace segment, so
            # walk the scope chain to get the fully-qualified prefix —
            # otherwise nested ``namespace eval outer { namespace eval
            # inner { namespace import … } }`` would record the import
            # under ``::inner`` instead of ``::outer::inner``.
            importing_ns = self._namespace_from_scope(scope)
            i = 1
            while i < len(args):
                pattern = args[i]
                tok = arg_tokens[i] if i < len(arg_tokens) else None
                if pattern == "-force":
                    i += 1
                    continue
                # Ignore non-qualified or substituted patterns — we
                # cannot statically resolve them to a source namespace.
                if "::" not in pattern or "$" in pattern or "[" in pattern:
                    i += 1
                    continue
                # A pattern without a leading ``::`` is resolved
                # relative to the *current* namespace, not the global
                # one — Tcl's own rule for ``namespace import``.  So
                # ``namespace eval foo { namespace import bar::* }``
                # imports ``::foo::bar::*``, not ``::bar::*``.
                if not pattern.startswith("::"):
                    if importing_ns == "::":
                        pattern = f"::{pattern}"
                    else:
                        pattern = f"{importing_ns}::{pattern}"
                if tok is not None:
                    self.result.namespace_imports.append(
                        NamespaceImport(
                            ns=importing_ns,
                            pattern=pattern,
                            range=range_from_token(tok),
                        )
                    )
                i += 1
        elif cmd_name.endswith("::import") and len(args) == 1:
            # tcllib-style ``some::ns::import <alias>`` wrapper: the proc
            # body typically ``uplevel``s ``namespace import some::ns::*``
            # into the given alias namespace.  Infer the import as a
            # conjecture so ``alias::foo`` resolves to ``some::ns::foo``.
            alias = args[0]
            if alias and "$" not in alias and "[" not in alias:
                source_ns = cmd_name[: -len("::import")]
                if not source_ns.startswith("::"):
                    source_ns = f"::{source_ns}"
                # Relative aliases live under the current namespace —
                # ``namespace eval outer { some::ns::import vt }``
                # creates ``::outer::vt``, not ``::vt``.
                current_ns = self._namespace_from_scope(scope)
                if alias.startswith("::"):
                    alias_ns = alias
                elif current_ns == "::":
                    alias_ns = f"::{alias}"
                else:
                    alias_ns = f"{current_ns}::{alias}"
                self.result.namespace_imports.append(
                    NamespaceImport(
                        ns=alias_ns,
                        pattern=f"{source_ns}::*",
                        range=range_from_token(argv[0]),
                        conjectured=True,
                    )
                )

        # Record source command targets for cross-file dependency tracking.
        if cmd_name == "source" and args:
            # Skip -encoding flag: source ?-encoding enc? fileName
            file_idx = 2 if args[0] == "-encoding" and len(args) >= 3 else 0
            if file_idx < len(args) and file_idx < len(arg_tokens):
                _st = arg_tokens[file_idx]
                _path = args[file_idx]
                _is_lit = (
                    _st.type not in (TokenType.VAR, TokenType.CMD)
                    and "$" not in _path
                    and "[" not in _path
                )
                self.result.source_targets.append(
                    SourceTarget(
                        raw_path=_path,
                        range=range_from_token(_st),
                        is_literal=_is_lit,
                    )
                )

        # W002 and built-in arity (E001-E003, W001) are now emitted by
        # the IR-based _arity_checks in compiler_checks.py.

        if self._run_command_special_cases(cmd_name, args, arg_tokens, argv[0], scope):
            return

        # Resolve command aliases for role-based argument analysis.
        # For ``interp alias {} = {} expr``, calling ``= {$x+1}`` should
        # analyse the argument as an expression (ArgRole.EXPR).
        role_cmd, role_args = self._resolve_alias(cmd_name, args, scope)
        prepend_n = len(role_args) - len(args)

        resolved_sig = self._signature_for_command(cmd_name)
        # If the command itself has no signature, try the alias target.
        if resolved_sig is None and role_cmd != cmd_name:
            resolved_sig = self._signature_for_command(role_cmd)

        # Arity checking for user-defined procs (needs ProcDef.params
        # with has_default info, which the IR doesn't carry).
        if resolved_sig is None:
            proc_def = self._resolve_proc_call(cmd_name, scope)
            if proc_def is not None:
                self._check_proc_call_arity(
                    proc_def,
                    args,
                    argv[0],
                    arg_expand=arg_expand,
                    arg_tokens=arg_tokens,
                    arg_single_token=arg_single_token,
                    scope=scope,
                )

        # iRules ``call``: arity-check the target proc against the
        # arguments *after* the proc name (args[1:], not args).
        if cmd_name == "call" and args and arg_tokens and active_dialect() == "f5-irules":
            call_target_proc = self._resolve_proc_call(args[0], scope)
            if call_target_proc is not None:
                self._check_proc_call_arity(
                    call_target_proc,
                    args[1:],
                    arg_tokens[0],
                    arg_expand=arg_expand[1:] if arg_expand else None,
                    arg_tokens=arg_tokens[1:],
                    arg_single_token=arg_single_token[1:] if arg_single_token else None,
                    scope=scope,
                )

        # Use the resolved alias target for role-based lookups, then map
        # indices back to the real args by subtracting the prepend offset.
        for virtual_idx in sorted(arg_indices_for_role(role_cmd, role_args, ArgRole.EXPR)):
            idx = virtual_idx - prepend_n
            if idx < 0 or idx >= len(args):
                continue
            expr_tok = arg_tokens[idx] if idx < len(arg_tokens) else None
            self._analyse_expr(args[idx], scope, expr_token=expr_tok)

        # Recurse into body arguments and capture var-name args.
        # For ``when EVENT { body }``, set event context while analysing.
        # For ``if``, bodies are conditional so increment depth for package
        # confidence tracking.
        prev_event = self._current_event
        if cmd_name == "when" and args:
            self._current_event = args[0]
        is_conditional = cmd_name in ("if", "try")
        if is_conditional:
            self._conditional_depth += 1
        # A foreign-dialect builtin's BODY arg-role must not be applied.
        # ``when`` is an iRules-only builtin: under plain Tcl it is an unknown
        # would-be user command whose braced argument is opaque data, not a
        # handler script — recursing into it produces spurious findings
        # (W123/W210) on what Tcl never parses as a script.  Only *disabled*
        # builtins are skipped; commands unknown in every dialect (user procs,
        # TclOO method/constructor bodies, recovery artefacts) still recurse.
        # Aliases resolve to a real target, so gating on ``role_cmd`` is right.
        body_implicit_args = body_arg_implicit_args_for_command(role_cmd, role_args)
        if not command_is_disabled_in_active_dialect(role_cmd):
            for body in iter_body_arguments(role_cmd, role_args, arg_tokens, prepend_n=prepend_n):
                self._analyse_body(
                    body.text,
                    scope,
                    body_token=body.token,
                    implicit_appended_args=body_implicit_args,
                )
        if is_conditional:
            self._conditional_depth -= 1
        if cmd_name == "when":
            self._current_event = prev_event

        if cmd_name not in ("set", "variable", "global", "incr"):
            for virtual_idx in sorted(arg_indices_for_role(role_cmd, role_args, ArgRole.VAR_WRITE)):
                idx = virtual_idx - prepend_n
                if idx < 0 or idx >= len(args):
                    continue
                tok = arg_tokens[idx] if idx < len(arg_tokens) else argv[0]
                self._define_var(args[idx], tok, scope, warn_if_unused=False)

        # Record regex pattern arguments (regexp, regsub).
        # When the pattern position is a variable reference, look up its
        # constant value and also mark the defining `set` as a regex pattern.
        for virtual_idx in sorted(arg_indices_for_role(role_cmd, role_args, ArgRole.PATTERN)):
            idx = virtual_idx - prepend_n
            if idx < 0 or idx >= len(arg_tokens):
                continue
            pat_tok = arg_tokens[idx]
            if pat_tok.type is TokenType.VAR:
                # Pattern is a variable — resolve to constant value if possible
                var_name = pat_tok.text
                const_val = self._lookup_const_string(var_name, scope)
                if const_val is not None:
                    self.result.regex_patterns.append(
                        RegexPattern(
                            range=range_from_token(pat_tok),
                            pattern=const_val,
                            command=cmd_name,
                        )
                    )
                    self._regex_vars.add((id(scope), var_name))
                    # Also mark the original definition site as a regex pattern
                    self._record_defining_set_as_regex(var_name, scope, cmd_name)
            else:
                self.result.regex_patterns.append(
                    RegexPattern(
                        range=range_from_token(pat_tok),
                        pattern=args[idx],
                        command=cmd_name,
                    )
                )

    def _run_command_special_cases(
        self,
        cmd_name: str,
        args: list[str],
        arg_tokens: list[Token],
        cmd_token: Token,
        scope: Scope,
    ) -> bool:
        """Apply command-specific analyser behaviour.

        Returns True when the command was fully handled and generic role-based
        processing should stop.
        """
        if self._handle_proc_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_oo_class_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_oo_define_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_snit_type_command(cmd_name, args, arg_tokens, scope):
            return True
        self._handle_oo_objdefine(cmd_name, args)

        self._handle_set_command(cmd_name, args, arg_tokens, scope)
        self._handle_var_declaration_command(cmd_name, args, arg_tokens, scope)

        if self._handle_namespace_eval_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_foreach_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_for_command(cmd_name, args, arg_tokens, scope):
            return True
        if self._handle_switch_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True
        if self._handle_catch_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True
        if self._handle_try_command(cmd_name, args, arg_tokens, cmd_token, scope):
            return True
        if self._handle_uplevel_command(cmd_name, args, arg_tokens, scope):
            return True

        self._handle_incr_command(cmd_name, args, arg_tokens, scope)
        self._handle_upvar_command(cmd_name, args, arg_tokens, scope)
        self._handle_namespace_upvar_command(cmd_name, args, arg_tokens, scope)
        self._handle_dict_var_command(cmd_name, args, arg_tokens, scope)
        self._handle_interp_alias(cmd_name, args)
        self._handle_namespace_ensemble(cmd_name, args, scope)
        self._handle_trace_command(cmd_name, args, scope)
        return False
