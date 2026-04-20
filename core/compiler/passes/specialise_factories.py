"""Option-shape factory proc specialisation (Phase 8).

Drives the tcltest ``Option`` pattern:

    proc Configure {name default description} {
        variable Option(\$name) \$default
        variable OptionDesc(\$name) \$description
        proc \$name {{value {}}} [subst -nocommands {
            variable Option(\$name)
            if {[string length \$value] > 0} {
                set Option(\$name) \$value
            }
            return \$Option(\$name)
        }]
    }

    Configure verbose 0 "verbose flag"
    Configure skip     {} "patterns to skip"

Each call to ``Configure`` with literal args creates a dedicated
helper proc (``verbose``, ``skip``, …).  P6/P7 already lower the
*factory itself* well — the inner ``proc \$name`` substitutes when
its name and body template are const-known *from inside the
factory*.  But the call sites at top level still fire the factory
through interpreted dispatch on every invocation, which is what
this pass fixes.

P8.1 is the detector.  It recognises factory procs and records the
shape needed by P8.2 (the call-site rewriter): which param becomes
the child proc's name, the literal params-spec, and the template
string ready to feed through
:func:`core.parsing.subst_nocommands.subst_nocommands` with a
``param -> literal-arg`` map.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...parsing.command_segmenter import segment_commands
from ...parsing.tokens import TokenType
from ..ir import IRBarrier, IRProcedure


@dataclass(frozen=True, slots=True)
class FactoryShape:
    """The extracted specialisation recipe for an Option-shape factory.

    Populated by :func:`detect_factory_shape`.  Consumed by the
    call-site rewriter in P8.2 to synthesise per-call helper procs.
    """

    qualified_name: str
    """The factory proc's qualified name (e.g. ``::Configure``)."""

    params: tuple[str, ...]
    """All parameters of the factory, in declaration order.  Used by
    the rewriter to build the ``param -> literal-arg`` map from a
    call-site's positional args."""

    name_param: str
    """Name of the factory parameter whose value becomes the
    synthesised child proc's command name (the ``\\$foo`` inside
    ``proc \\$foo …``)."""

    child_params: str
    """Literal params-spec of the child proc (second argument to the
    inner ``proc``).  Passed straight through to the synthesised
    ``IRProcedure``."""

    child_body_template: str
    """The ``subst -nocommands`` template body.  Feed this plus a
    ``param -> literal-arg`` const-map into ``subst_nocommands``
    to get the concrete child proc body for a given call site."""


def detect_factory_shape(proc: IRProcedure) -> FactoryShape | None:
    """Recognise the Option-shape factory pattern in *proc*.

    Returns a populated :class:`FactoryShape` when the proc ends in
    a single ``proc \\$param {literal_params} [subst -nocommands
    {template}]`` call that matches the gates below, or ``None``
    otherwise.

    Gates:

    * The factory body must be **exactly one statement**, and that
      statement must be the inner ``proc`` barrier.  Earlier
      statements (``variable Option(\\$name) \\$default`` and
      similar prologue) would need to run at every call site; P8.2
      replaces call sites with a no-op + compiled-proc
      registration, which would silently drop those side effects.
      A future PR can lift this restriction by symbolically
      replaying the prologue per call site.
    * The inner ``proc`` name must be a bare ``\\$var`` VAR token
      referring to one of the factory's parameters (otherwise the
      rewriter has no way to pin it to a call-site arg).
    * The inner ``proc`` params-spec must be a single literal
      token (ESC bareword or STR braced).  Dynamic params shapes
      would need to be specialised too and aren't worth the
      complexity today.
    * The body must be a CMD token whose content is ``subst
      -nocommands {template}`` (same shape P7.3 recognises inline,
      extracted here instead of inlined).
    * The template must be a single literal STR token (i.e. the
      ``{…}``-braced form).  This plus the ``-nocommands`` flag is
      what lets the rewriter precompute the body byte-for-byte.
    """
    stmts = proc.body.statements
    if len(stmts) != 1:
        return None
    last = stmts[0]
    if not isinstance(last, IRBarrier):
        return None
    if last.reason != "dynamic proc name":
        return None
    if last.command != "proc":
        return None

    tokens = last.tokens
    if tokens is None:
        return None
    # Expect exactly four words: ``proc \$name {params} [subst …]``.
    if len(tokens.argv) != 4:
        return None
    if not all(tokens.single_token_word[:4]):
        return None

    name_tok = tokens.argv[1]
    params_tok = tokens.argv[2]
    body_tok = tokens.argv[3]

    if name_tok.type is not TokenType.VAR:
        return None
    name_param = name_tok.text
    if name_param not in proc.params:
        return None

    # Child params must be a literal — either a STR (braced) or an
    # ESC bareword with no substitutions.
    if params_tok.type is TokenType.STR:
        child_params = params_tok.text
    elif (
        params_tok.type is TokenType.ESC
        and "$" not in params_tok.text
        and "[" not in params_tok.text
    ):
        child_params = params_tok.text
    else:
        return None

    # Body is a CMD token.  Re-parse to extract the subst call and
    # verify the -nocommands + single-template shape.
    if body_tok.type is not TokenType.CMD:
        return None
    template = _extract_subst_nocommands_template(body_tok.text)
    if template is None:
        return None

    return FactoryShape(
        qualified_name=proc.qualified_name,
        params=proc.params,
        name_param=name_param,
        child_params=child_params,
        child_body_template=template,
    )


def _extract_subst_nocommands_template(cmd_text: str) -> str | None:
    """Return the braced template from a ``subst -nocommands
    {template}`` command substitution, or ``None`` if the shape
    doesn't match exactly.  Mirrors the gate in
    :meth:`core.compiler.lowering._Lowerer._eval_subst_nocommands_body`
    but returns the raw template rather than the evaluated string —
    the factory rewriter applies different const-maps per call
    site, so we defer the evaluation.
    """
    try:
        inner = segment_commands(cmd_text, None)
    except Exception:
        return None
    if len(inner) != 1:
        return None
    call = inner[0]
    if not call.texts or call.texts[0] != "subst":
        return None
    saw_nocommands = False
    template: str | None = None
    for i, tok in enumerate(call.argv[1:], start=1):
        text = call.texts[i]
        if text == "-nocommands":
            saw_nocommands = True
            continue
        if text in ("-nobackslashes", "-novariables"):
            return None
        if text.startswith("-"):
            return None
        if not call.single_token_word[i]:
            return None
        if tok.type is not TokenType.STR:
            return None
        if template is not None:
            return None
        template = text
    if not saw_nocommands or template is None:
        return None
    return template
