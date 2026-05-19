# canonicalisation: audited #246
r"""Option-shape factory proc specialisation (Phase 8).

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
:func:`compiler.parsing.subst_nocommands.subst_nocommands` with a
``param -> literal-arg`` map.
"""

from __future__ import annotations

from dataclasses import dataclass

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.subst_nocommands import subst_nocommands
from compiler.parsing.tokens import TokenType

from ..ir import (
    IRBarrier,
    IRBlock,
    IRCall,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
)


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
    if last.canonical_command != "::proc":
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


_DEFAULT_FACTORY_CAP = 64
"""P8.4 — per-factory specialisation cap.

An upper bound on how many call sites of a single factory we will
specialise.  Each specialisation synthesises a full ``IRProcedure``
(body, params, tokens, range), so an unchecked factory with a
thousand call sites would bloat bytecode without a matching
runtime benefit (the same factory call-site dispatch cost dominates
either way).  A cap of 64 covers tcltest (which defines ~20
options) plus a comfortable margin, and bails cleanly on pathological
inputs.

Callers that want to override the default can pass ``cap`` to
:func:`specialise_factories`.
"""


def specialise_factories(module: IRModule, *, cap: int = _DEFAULT_FACTORY_CAP) -> None:
    """Mutate *module* in place, replacing each literal-args call to
    an Option-shape factory with a synthesised child ``IRProcedure``
    and a no-op ``IRBlock`` in the caller's script.

    See :func:`detect_factory_shape` for what qualifies as a factory.
    Call sites qualify when:

    * The target resolves to a detected factory (unqualified name
      resolves through ``::``; qualified name must match a factory
      qname directly).
    * The call has exactly ``len(factory.params)`` args.
    * Every arg token is a literal (``ESC`` bareword with no
      ``\\$`` / ``[``, or ``STR`` braced).
    * The materialised template produced by
      :func:`compiler.parsing.subst_nocommands.subst_nocommands` is
      non-None (no unresolved ``\\$`` references, no refused
      constructs).
    * The per-factory specialisation count (tracked in
      *counts* internally) hasn't reached *cap*.  Once the cap is
      hit, subsequent call sites of the same factory stay on the
      runtime dispatch path — matches the bytecode-bloat guard the
      design doc calls for.

    Synthesised procs use the ``name_param`` arg's value as their
    unqualified name, mangled to ``::<name>`` at module scope.  A
    call-site that produces a duplicate name (same factory called
    twice with the same name) overwrites the earlier synthesis —
    matches Tcl's last-``proc``-wins semantics.
    """
    factories: dict[str, FactoryShape] = {}
    for qname, proc in module.procedures.items():
        if qname in module.redefined_procedures:
            continue
        shape = detect_factory_shape(proc)
        if shape is not None:
            factories[qname] = shape
    if not factories:
        return

    # Deferred imports — pulling ``lower_to_ir`` at module load would
    # cycle through ``compilation_unit.py``.
    from ..lowering import lower_to_ir as _lower

    # Per-factory specialisation counter.  Bumped on each successful
    # rewrite; once a factory's count reaches *cap* the rewriter
    # leaves subsequent call sites alone.
    counts: dict[str, int] = dict.fromkeys(factories.keys(), 0)

    module.top_level = _rewrite_script(
        module.top_level,
        factories,
        module,
        namespace="::",
        lower=_lower,
        counts=counts,
        cap=cap,
    )
    for qname, proc in list(module.procedures.items()):
        caller_ns = _namespace_of(qname)
        new_body = _rewrite_script(
            proc.body,
            factories,
            module,
            namespace=caller_ns,
            lower=_lower,
            counts=counts,
            cap=cap,
        )
        if new_body is not proc.body:
            module.procedures[qname] = IRProcedure(
                name=proc.name,
                qualified_name=proc.qualified_name,
                params=proc.params,
                range=proc.range,
                body=new_body,
                params_raw=proc.params_raw,
                body_source=proc.body_source,
                namespace_scoped=proc.namespace_scoped,
                base_priority=proc.base_priority,
            )


def _namespace_of(qname: str) -> str:
    idx = qname.rfind("::")
    if idx <= 0:
        return "::"
    return qname[:idx] or "::"


def _resolve_target(command: str, caller_ns: str, factories: dict[str, FactoryShape]) -> str | None:
    """Resolve ``command`` to a factory qname, or ``None`` if no match."""
    if command.startswith("::"):
        return command if command in factories else None
    # Unqualified: try caller-ns-qualified first (factories live in
    # the ns they're defined in), then root.
    if caller_ns != "::":
        candidate = f"{caller_ns}::{command}"
        if candidate in factories:
            return candidate
    candidate_root = f"::{command}"
    if candidate_root in factories:
        return candidate_root
    return None


def _literal_arg(stmt: IRCall, i: int) -> str | None:
    """Return the literal value of ``stmt``'s i-th argument, or
    ``None`` if it isn't a single-token literal.  ESC barewords are
    accepted as long as they have no ``$`` / ``[`` substitutions;
    STR (braced) tokens are always accepted.  Dynamic args fail the
    gate."""
    tokens = stmt.tokens
    if tokens is None:
        return None
    # tokens.argv[0] is the command name; argv[1:] are the args.
    if i + 1 >= len(tokens.argv):
        return None
    if not tokens.single_token_word[i + 1]:
        return None
    if tokens.expand_word and tokens.expand_word[i + 1]:
        return None
    tok = tokens.argv[i + 1]
    if tok.type is TokenType.STR:
        return stmt.args[i]
    if tok.type is TokenType.ESC:
        text = stmt.args[i]
        if "$" in text or "[" in text:
            return None
        return text
    return None


def _rewrite_script(
    script: IRScript,
    factories: dict[str, FactoryShape],
    module: IRModule,
    *,
    namespace: str,
    lower,
    counts: dict[str, int],
    cap: int,
) -> IRScript:
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        rewritten = _rewrite_stmt(
            stmt,
            factories,
            module,
            namespace=namespace,
            lower=lower,
            counts=counts,
            cap=cap,
        )
        if rewritten is not stmt:
            changed = True
        new_stmts.append(rewritten)
    if changed:
        return IRScript(statements=tuple(new_stmts))
    return script


def _rewrite_stmt(
    stmt: IRStatement,
    factories: dict[str, FactoryShape],
    module: IRModule,
    *,
    namespace: str,
    lower,
    counts: dict[str, int],
    cap: int,
) -> IRStatement:
    # Only two shapes qualify for rewriting in this first wave:
    # direct ``IRCall`` to a factory, and ``IRBlock`` containers
    # that might hold such a call.  Structured IR (``if``, ``for``,
    # …) is left alone — factory calls inside control-flow branches
    # are uncommon (tcltest's ``Configure`` calls sit at module
    # scope) and the recursion cost isn't justified yet.
    if isinstance(stmt, IRCall):
        return _maybe_specialise_call(
            stmt,
            factories,
            module,
            namespace=namespace,
            lower=lower,
            counts=counts,
            cap=cap,
        )
    if isinstance(stmt, IRBlock):
        inner_ns = stmt.namespace or namespace
        new_body = _rewrite_script(
            stmt.body,
            factories,
            module,
            namespace=inner_ns,
            lower=lower,
            counts=counts,
            cap=cap,
        )
        if new_body is not stmt.body:
            return IRBlock(
                range=stmt.range,
                body=new_body,
                namespace=stmt.namespace,
                source_args=stmt.source_args,
                source_tokens=stmt.source_tokens,
            )
        return stmt
    return stmt


def _maybe_specialise_call(
    stmt: IRCall,
    factories: dict[str, FactoryShape],
    module: IRModule,
    *,
    namespace: str,
    lower,
    counts: dict[str, int],
    cap: int,
) -> IRStatement:
    if not stmt.command:
        return stmt
    target = _resolve_target(stmt.command, namespace, factories)
    if target is None:
        return stmt
    # P8.4: per-factory cap.  Keeps pathological call-site counts
    # from bloating bytecode — hit-the-cap means "leave further
    # calls to runtime dispatch".
    if counts.get(target, 0) >= cap:
        return stmt
    factory = factories[target]
    if len(stmt.args) != len(factory.params):
        return stmt
    # Build the const-map from (param-name → literal-arg).  Bail on
    # any non-literal arg.
    const_map: dict[str, str] = {}
    for i, pname in enumerate(factory.params):
        lit = _literal_arg(stmt, i)
        if lit is None:
            return stmt
        const_map[pname] = lit

    materialised = subst_nocommands(factory.child_body_template, const_map)
    if materialised is None:
        return stmt
    child_name = const_map[factory.name_param]
    # Refuse names that contain ``::`` — the synthesised proc would
    # need to live inside some namespace we may not have a path to,
    # and mixing factory-namespace inference with top-level specs is
    # more than this first wave wants to tackle.
    if "::" in child_name or not child_name:
        return stmt

    qualified_child = f"::{child_name}"
    child_body = lower(materialised).top_level
    module.procedures[qualified_child] = IRProcedure(
        name=child_name,
        qualified_name=qualified_child,
        params=_parse_child_params(factory.child_params),
        range=stmt.range,
        body=child_body,
        params_raw=factory.child_params,
        body_source=materialised,
        namespace_scoped=False,
    )
    counts[target] = counts.get(target, 0) + 1
    # Replace the original call with an empty IRBlock — the
    # factory's side effects are captured entirely by the
    # synthesised child proc (see the single-statement gate in
    # ``detect_factory_shape``).  Keep the tokens so downstream
    # passes that want the original source can still find it.
    return IRBlock(
        range=stmt.range,
        body=IRScript(),
        namespace=namespace,
        source_args=stmt.args,
        source_tokens=stmt.tokens,
    )


def _parse_child_params(params_raw: str) -> tuple[str, ...]:
    """Parse a params-spec string into a tuple of parameter names.
    Handles the single-token shapes we accept: bareword or braced
    list.  ``{value {}}``-style default-value list elements are
    reduced to just the name."""
    text = params_raw.strip()
    if not text:
        return ()
    # Very small-scope parser: split on whitespace / braces.  A
    # full Tcl list parser would be ideal but the child params our
    # factory-shape gate accepts are simple enough that a
    # hand-rolled split handles them cleanly.
    names: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        if text[i].isspace():
            i += 1
            continue
        if text[i] == "{":
            # ``{name default}`` element — extract the name only.
            depth = 1
            j = i + 1
            while j < n and depth > 0:
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                j += 1
            inner = text[i + 1 : j - 1].strip()
            # Name is up to first space.
            sp = inner.find(" ")
            names.append(inner if sp < 0 else inner[:sp])
            i = j
            continue
        # Bareword: read to next whitespace.
        j = i
        while j < n and not text[j].isspace():
            j += 1
        names.append(text[i:j])
        i = j
    return tuple(names)


def _extract_subst_nocommands_template(cmd_text: str) -> str | None:
    """Return the braced template from a ``subst -nocommands
    {template}`` command substitution, or ``None`` if the shape
    doesn't match exactly.  Mirrors the gate in
    :meth:`compiler.lowering._Lowerer._eval_subst_nocommands_body`
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
