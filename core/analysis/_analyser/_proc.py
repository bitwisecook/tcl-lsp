from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from ._core import _AnalyserBase as _Base
else:
    _Base = object

from ...commands.registry import REGISTRY
from ...commands.registry.signatures import Arity
from ...common.dialect import active_dialect
from ...common.naming import (
    normalise_qualified_name as _normalise_qualified_name,
)
from ...common.ranges import range_from_token
from ...parsing.lexer import TclLexer
from ...parsing.tokens import Token, TokenType
from ..proc_arg_traits import infer_param_traits
from ..semantic_model import (
    Diagnostic,
    ProcDef,
    RegexPattern,
    Scope,
    Severity,
    VarDef,
)
from ._utils import parse_param_list

log = logging.getLogger(__name__)


class _AnalyserProcMixin(_Base):
    """Proc definition, resolution, call-arity checks, and low-level handlers."""

    if TYPE_CHECKING:
        # From _AnalyserScopeMixin
        def _define_var(self, *a: Any, **kw: Any) -> Any: ...
        def _record_var_read(self, *a: Any, **kw: Any) -> None: ...
        def _lookup_const_string(self, *a: Any, **kw: Any) -> Any: ...
        def _record_defining_set_as_regex(self, *a: Any, **kw: Any) -> None: ...
        def _namespace_from_scope(self, *a: Any, **kw: Any) -> str: ...
        # From _AnalyserOOMixin
        def _extract_unknown_proc_info(self, *a: Any, **kw: Any) -> Any: ...

    def _handle_proc(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle a proc definition."""
        proc_name = args[0]
        param_str = args[1] if len(args) > 1 else ""
        body = args[2] if len(args) > 2 else ""

        params = parse_param_list(param_str)

        # Determine qualified name. Use ``_namespace_from_scope`` so a
        # nested ``namespace eval outer { namespace eval inner { proc foo
        # ... } }`` produces ``::outer::inner::foo`` (the bare ``scope.name``
        # would drop everything outside the innermost namespace).
        # ``normalise_qualified_name`` collapses ``::`` runs so a proc
        # already declared as ``::ns::foo`` doesn't pick up a stray
        # leading ``::``.
        ns = self._namespace_from_scope(scope) if scope.kind == "namespace" else "::"
        qualified = _normalise_qualified_name(f"{ns}::{proc_name}")
        # The ``name`` field carries the bare proc name even when the
        # source declared it qualified (``proc ::ns::foo``); the bare
        # tail is what consumers display in hover / outline UIs.
        bare_name = proc_name.rsplit("::", 1)[-1] or proc_name

        if not arg_tokens:
            return
        name_range = range_from_token(arg_tokens[0])
        body_range = range_from_token(arg_tokens[2]) if len(arg_tokens) > 2 else name_range

        # W113: warn when proc name shadows a built-in command.
        dialect = active_dialect()
        if self._builtin_names is None or self._builtin_dialect != dialect:
            self._builtin_names = frozenset(REGISTRY.command_names(dialect))
            self._builtin_dialect = dialect
        normalised_proc = proc_name.lstrip(":")
        normalised_qual = qualified.lstrip(":")
        shadow_name = normalised_proc if normalised_proc in self._builtin_names else None
        if shadow_name is None and normalised_qual in self._builtin_names:
            shadow_name = normalised_qual
        if shadow_name is not None:
            dialect_label = f" ({self._builtin_dialect})" if self._builtin_dialect else ""
            self.result.diagnostics.append(
                Diagnostic(
                    range=name_range,
                    message=f"Procedure '{proc_name}' shadows built-in command{dialect_label}",
                    severity=Severity.WARNING,
                    code="W113",
                )
            )

        preceding_doc = self._last_comment
        self._last_comment = ""

        # Extract the first comment block from the proc body as a
        # fallback docstring when there is no preceding comment.
        body_doc = ""
        if not preceding_doc and body:
            from core.formatting.docstring import extract_body_docstring

            body_doc = extract_body_docstring(body)

        proc_def = ProcDef(
            name=bare_name,
            qualified_name=qualified,
            params=params,
            name_range=name_range,
            body_range=body_range,
            doc=preceding_doc or body_doc,
        )

        # ``scope.procs`` is keyed by the bare proc name so per-scope
        # shadowing works whether the source declared the proc as
        # ``foo`` or ``::ns::foo``.  ``all_procs`` keeps the full
        # qualified key.
        scope.procs[bare_name] = proc_def
        self.result.all_procs[qualified] = proc_def

        # Analyse the body in a new proc scope. The scope name carries
        # the *raw* declared name (``::ns::foo`` form preserved) — this
        # matches the Rust port's choice in ``analyser/handlers.rs`` at
        # the ``Scope::new(ScopeKind::Proc, raw_name.clone())`` site, and
        # is what the per-scope ``all_variables`` key prefix relies on.
        proc_scope = Scope(kind="proc", name=proc_name, parent=scope, body_range=body_range)
        scope.children.append(proc_scope)

        # Define parameters as variables in the proc scope
        for p in params:
            proc_scope.variables[p.name] = VarDef(
                name=p.name,
                definition_range=name_range,
                warn_if_unused=False,
            )

        # Save/restore _last_comment around body analysis so that
        # comments inside the body do not bleed to the next proc.
        saved_comment = self._last_comment
        body_tok = arg_tokens[2] if len(arg_tokens) > 2 else None
        self._analyse_body(body, proc_scope, body_token=body_tok)
        self._last_comment = saved_comment

        # Infer proc argument traits from body usage.
        if params and body:
            param_names = tuple(p.name for p in params)
            try:
                proc_def.param_traits = infer_param_traits(param_names, body)
            except Exception:
                pass  # trait inference is best-effort

        # Detect user-defined ``unknown`` proc and extract dispatch info.
        # Handle both bare ``unknown`` and qualified forms like ``::tcl::unknown``.
        norm_qualified = qualified.lstrip(":")
        if proc_name == "unknown" or norm_qualified in ("unknown", "tcl::unknown"):
            self._extract_unknown_proc_info(body, params)

    # ------------------------------------------------------------------
    # TclOO class extraction
    # ------------------------------------------------------------------

    _OO_METACLASSES = frozenset({"oo::class", "oo::configurable", "oo::abstract", "oo::singleton"})

    _OO_DEFINE_SUBCMDS = frozenset(
        {
            "method",
            "classmethod",
            "constructor",
            "destructor",
            "superclass",
            "mixin",
            "variable",
            "filter",
            "forward",
            "export",
            "unexport",
            "property",
            "private",
            "initialise",
            "initialize",
            "definitionnamespace",
            "deletemethod",
            "renamemethod",
            "self",
        }
    )

    def _handle_set(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle a set command -- defines/references a variable."""
        var_name = args[0]
        if len(args) >= 2:
            # This is a definition/assignment
            self._define_var(var_name, arg_tokens[0], scope, warn_if_unused=True)
        else:
            # set with one arg is a read -- record reference
            self._record_var_read(var_name, range_from_token(arg_tokens[0]), scope)

    def _handle_switch(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle switch command -- analyse pattern/body pairs.

        Switch has two forms:
          1. switch string pattern body ?pattern body ...?
          2. switch string { pattern body ?pattern body ...? }
        In form 2, the entire pattern/body block is a single braced string.

        When ``-regexp`` is among the option switches, pattern arguments are
        recorded as :class:`RegexPattern` instances in the analysis result.
        """
        # Scan options, detecting -regexp
        is_regexp = False
        i = 0
        while i < len(args) and args[i].startswith("-"):
            if args[i] == "-regexp":
                is_regexp = True
            if args[i] == "--":
                i += 1
                break
            i += 1
        # Skip the string argument
        i += 1

        if i < len(args) and i == len(args) - 1:
            # Form 2: single braced body containing all pattern/body pairs
            # Re-lex the braced body to extract pairs
            body_text = args[i]
            body_tok = arg_tokens[i] if i < len(arg_tokens) else None
            self._parse_switch_body(body_text, body_tok, scope, is_regexp=is_regexp)
        else:
            # Form 1: remaining args are pattern/body pairs
            while i + 1 < len(args):
                # Record regex patterns from switch -regexp
                if is_regexp and args[i] != "default" and i < len(arg_tokens):
                    pat_tok = arg_tokens[i]
                    if pat_tok.type is TokenType.VAR:
                        const_val = self._lookup_const_string(pat_tok.text, scope)
                        if const_val is not None:
                            self.result.regex_patterns.append(
                                RegexPattern(
                                    range=range_from_token(pat_tok),
                                    pattern=const_val,
                                    command="switch",
                                )
                            )
                            self._regex_vars.add((id(scope), pat_tok.text))
                            self._record_defining_set_as_regex(pat_tok.text, scope, "switch")
                    else:
                        self.result.regex_patterns.append(
                            RegexPattern(
                                range=range_from_token(pat_tok),
                                pattern=args[i],
                                command="switch",
                            )
                        )
                body = args[i + 1]
                if body != "-":  # '-' means fall through
                    body_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                    self._analyse_body(body, scope, body_token=body_tok)
                i += 2

    def _parse_switch_body(
        self,
        body_text: str,
        outer_body_token: Token | None,
        scope: Scope,
        *,
        is_regexp: bool = False,
    ) -> None:
        """Parse the braced body of a switch command to extract pattern/body pairs."""
        # Create lexer with base offsets if we have the outer body token
        if outer_body_token is not None:
            lexer = TclLexer(
                body_text,
                base_offset=outer_body_token.start.offset + 1,
                base_line=outer_body_token.start.line,
                base_col=outer_body_token.start.character + 1,
            )
        else:
            lexer = TclLexer(body_text)
        elements: list[str] = []
        element_tokens: list[Token] = []
        prev_type = TokenType.EOL

        while True:
            tok = lexer.get_token()
            if tok is None:
                break
            if tok.type in (TokenType.SEP, TokenType.EOL):
                prev_type = tok.type
                continue

            if prev_type in (TokenType.SEP, TokenType.EOL):
                elements.append(tok.text)
                element_tokens.append(tok)
            else:
                if elements:
                    elements[-1] += tok.text
                else:
                    elements.append(tok.text)
                    element_tokens.append(tok)
            prev_type = tok.type

        # elements should be alternating pattern/body pairs
        i = 0
        while i + 1 < len(elements):
            # Record regex patterns from switch -regexp
            if is_regexp and elements[i] != "default" and i < len(element_tokens):
                pat_tok = element_tokens[i]
                if pat_tok.type is TokenType.VAR:
                    const_val = self._lookup_const_string(pat_tok.text, scope)
                    if const_val is not None:
                        self.result.regex_patterns.append(
                            RegexPattern(
                                range=range_from_token(pat_tok),
                                pattern=const_val,
                                command="switch",
                            )
                        )
                        self._regex_vars.add((id(scope), pat_tok.text))
                        self._record_defining_set_as_regex(pat_tok.text, scope, "switch")
                else:
                    self.result.regex_patterns.append(
                        RegexPattern(
                            range=range_from_token(pat_tok),
                            pattern=elements[i],
                            command="switch",
                        )
                    )
            body = elements[i + 1]
            if body != "-":  # '-' means fall through
                body_tok = element_tokens[i + 1] if i + 1 < len(element_tokens) else None
                self._analyse_body(body, scope, body_token=body_tok)
            i += 2

    def _handle_try(
        self,
        args: list[str],
        arg_tokens: list[Token],
        scope: Scope,
    ) -> None:
        """Handle try command -- analyse body and handler bodies."""
        if not args:
            return
        # First arg is the try body
        body_tok = arg_tokens[0] if arg_tokens else None
        self._analyse_body(args[0], scope, body_token=body_tok)
        # Scan for 'on', 'trap', 'finally' keywords
        i = 1
        while i < len(args):
            kw = args[i]
            if kw == "finally" and i + 1 < len(args):
                finally_tok = arg_tokens[i + 1] if i + 1 < len(arg_tokens) else None
                self._analyse_body(args[i + 1], scope, body_token=finally_tok)
                i += 2
            elif kw in ("on", "trap") and i + 3 < len(args):
                # on code varList body  /  trap pattern varList body
                handler_tok = arg_tokens[i + 3] if i + 3 < len(arg_tokens) else None
                self._analyse_body(args[i + 3], scope, body_token=handler_tok)
                i += 4
            else:
                i += 1

    def _resolve_proc_call(self, cmd_name: str, scope: Scope) -> ProcDef | None:
        """Resolve a command name to a known proc definition, if any."""
        if not cmd_name:
            return None

        candidates: list[str] = []
        seen: set[str] = set()

        def add_candidate(name: str) -> None:
            qname = _normalise_qualified_name(name)
            if not qname or qname in seen:
                return
            seen.add(qname)
            candidates.append(qname)

        if cmd_name.startswith("::"):
            add_candidate(cmd_name)
        elif "::" in cmd_name:
            add_candidate(f"::{cmd_name}")
        else:
            current: Scope | None = scope
            while current is not None:
                if current.kind == "namespace":
                    add_candidate(f"{current.name}::{cmd_name}")
                current = current.parent
            add_candidate(f"::{cmd_name}")

        for qname in candidates:
            proc = self.result.all_procs.get(qname)
            if proc is not None:
                return proc
        return None

    def _resolve_expansion_count(
        self,
        tok: Token,
        single_token: bool,
        scope: Scope,
    ) -> int | None:
        """Return the static element count of an expanded ({*}-prefixed) word.

        Inspects the token for the word after the ``{*}`` prefix:

        - Braced literal ``{a b c}`` → the segmenter has already stripped
          the outer braces; the token is a :class:`TokenType.STR` whose
          text is the inner list (``"a b c"``), which we split.  Empty
          braces ``{*}{}`` resolve to zero elements.
        - Pure variable reference ``$x`` → the token is a :class:`TokenType.VAR`;
          if the variable has a known constant value in the current
          scope chain, split that value.
        - Anything else (command substitution, interpolated/concatenated
          word, dynamic value) → ``None``: the count is not statically
          known.

        Refinement is only attempted when ``single_token`` is true:
        for concatenated words like ``{*}$x$y`` or ``{*}{a b}$suffix``
        the segmenter exposes only the *first* token in ``argv[i]``,
        which would otherwise be misinterpreted as a pure literal or
        pure var ref.

        Returns ``None`` when the count cannot be determined.  The caller
        must then treat the expansion as contributing an unknown number
        of runtime arguments (0..∞), matching Tcl's runtime behaviour
        where ``{*}`` coerces the value to a list via
        ``Tcl_ListObjGetElements``.
        """
        from ...compiler.tcl_expr_eval import _split_tcl_list

        if not single_token:
            return None
        if tok.type is TokenType.STR:
            try:
                return len(_split_tcl_list(tok.text))
            except Exception:
                return None
        if tok.type is TokenType.VAR:
            const_val = self._lookup_const_string(tok.text, scope)
            if const_val is None:
                return None
            try:
                return len(_split_tcl_list(const_val))
            except Exception:
                return None
        return None

    def _check_proc_call_arity(
        self,
        proc_def: ProcDef,
        args: list[str],
        cmd_token: Token,
        arg_expand: list[bool] | None = None,
        arg_tokens: list[Token] | None = None,
        arg_single_token: list[bool] | None = None,
        scope: Scope | None = None,
    ) -> None:
        """Check a proc call against the proc's parameter list.

        ``arg_expand`` is a list parallel to ``args`` whose entries are
        ``True`` for arguments preceded by the ``{*}`` expansion prefix.
        Each expanded argument may yield zero or more runtime arguments.

        When ``arg_tokens``, ``arg_single_token`` and ``scope`` are
        provided, each expansion is resolved to a static element count
        where possible (single-token literal lists, single-token
        variables with known constant values).  When an expansion cannot
        be resolved, it contributes an unknown number of arguments
        (0..∞) and E002 is suppressed for that call; E003 only fires
        when the minimum statically-known argument count still exceeds
        the max.
        """
        required = 0
        variadic = False
        for i, param in enumerate(proc_def.params):
            if i == len(proc_def.params) - 1 and param.name == "args" and not param.has_default:
                variadic = True
                continue
            if not param.has_default:
                required += 1

        arity = Arity(required, Arity.ANY if variadic else len(proc_def.params))
        nargs_min, nargs_max = self._arg_count_bounds(
            args, arg_expand, arg_tokens, arg_single_token, scope
        )
        display_name = proc_def.qualified_name
        if nargs_max < arity.min:
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(cmd_token),
                    message=f"Too few arguments for '{display_name}': expected at least {arity.min}, got {nargs_max}",
                    severity=Severity.ERROR,
                    code="E002",
                )
            )
        elif nargs_min > arity.max:
            self.result.diagnostics.append(
                Diagnostic(
                    range=range_from_token(cmd_token),
                    message=f"Too many arguments for '{display_name}': expected at most {arity.max}, got {nargs_min}",
                    severity=Severity.ERROR,
                    code="E003",
                )
            )

    def _arg_count_bounds(
        self,
        args: list[str],
        arg_expand: list[bool] | None,
        arg_tokens: list[Token] | None,
        arg_single_token: list[bool] | None,
        scope: Scope | None,
    ) -> tuple[int, int]:
        """Return the static (min, max) runtime argument count for a call.

        For each positional argument word, if it is not ``{*}``-expanded
        it contributes exactly one runtime argument.  If it is expanded,
        :meth:`_resolve_expansion_count` is consulted: a known count
        adds to both min and max; an unknown count adds 0 to min and
        leaves max unbounded (``Arity.ANY``).
        """
        if not arg_expand or not any(arg_expand):
            return len(args), len(args)
        nargs_min = 0
        nargs_max = 0
        unbounded = False
        for i, _ in enumerate(args):
            expanded = i < len(arg_expand) and arg_expand[i]
            if not expanded:
                nargs_min += 1
                nargs_max += 1
                continue
            tok = arg_tokens[i] if arg_tokens and i < len(arg_tokens) else None
            single = (
                arg_single_token[i] if arg_single_token and i < len(arg_single_token) else False
            )
            count: int | None = None
            if tok is not None and scope is not None:
                count = self._resolve_expansion_count(tok, single, scope)
            if count is None:
                unbounded = True
                continue
            nargs_min += count
            nargs_max += count
        if unbounded:
            nargs_max = Arity.ANY
        return nargs_min, nargs_max
