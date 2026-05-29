"""Bridge from the existing IR/CFG to the unified Place model (Phase 8A).

Resolves each IR statement's *def* and *read* targets to canonical
:class:`compiler.place.Place`s on demand, so the dataflow consumers (dead-store,
unused, read-before-set) can switch from name-string equality to the sound
:func:`compiler.place.overlap` relation — without re-stamping every construction
site in lowering.  The per-function :class:`compiler.var_resolve.ResolveContext`
is built once by scanning the CFG for ``global`` / ``variable`` / ``upvar`` /
``trace`` declarations (reusing the ``compiler.var_scoping`` grammar that
already feeds memory-SSA and the LSP declaration provider).
"""

from __future__ import annotations

from shared.naming import normalise_var_name

from .cfg import CFGBranch, CFGFunction, CFGReturn
from .expr_ast import command_texts_in_expr_node, vars_in_expr_node
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRExprEval,
    IRIncr,
    IRReturn,
    IRStatement,
)
from .parsing.command_segmenter import segment_commands
from .place import Place, unknown
from .registry.runtime import ArgRole, BodyKind, arg_indices_for_role, body_kind_for_command
from .var_refs import scan_var_ref_forms
from .var_resolve import ResolveContext, resolve_place
from .var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)

# Commands whose VAR_READ-role argument names the *whole* array, not a scalar
# (so a write to any element is observed).  Conservative — extra members only
# widen overlap (sound).
_WHOLE_ARRAY_COMMANDS = frozenset({"::array", "::parray"})
_WHOLE_ARRAY_BARE = frozenset({"array", "parray"})


def build_resolve_context(cfg: CFGFunction, *, namespace: str | None = None) -> ResolveContext:
    """Scan *cfg* for the in-scope variable declarations and build the context
    used to resolve places within the function.

    When *namespace* is omitted it is derived from the function's qualified name
    (``cfg.name``): a ``variable v`` inside ``::ns::p`` must resolve to the
    namespace variable ``::ns::v``, not the global ``::v``.
    """
    if namespace is None:
        parent = cfg.name.rpartition("::")[0]
        namespace = parent or "::"
    globals_: set[str] = set()
    ns_vars: set[str] = set()
    upvar_aliases: dict[str, str] = {}
    traced: set[str] = set()

    for block in cfg.blocks.values():
        for stmt in block.statements:
            if not isinstance(stmt, (IRCall, IRBarrier)):
                continue
            cmd = stmt.canonical_command
            args = list(stmt.args)
            if cmd == "::global":
                for i in global_declaration_indices(args):
                    if 0 <= i < len(args):
                        globals_.add(normalise_var_name(args[i]))
            elif cmd == "::variable":
                for i in variable_declaration_indices(args):
                    if 0 <= i < len(args):
                        ns_vars.add(normalise_var_name(args[i]))
            elif cmd == "::trace":
                t = _trace_target(args)
                if t:
                    traced.add(normalise_var_name(t))
            # `upvar` / `namespace upvar` — recognised structurally by the shared
            # grammar (the IR command for `namespace upvar` is `namespace`, so we
            # must NOT gate on `is_scope_alias_command`, which only knows the bare
            # `upvar` command).  Mirrors `_escaping_var_names`'s single-source scan.
            _collect_upvar_aliases(stmt.command, args, upvar_aliases, namespace=namespace)

    return ResolveContext(
        namespace=namespace,
        globals=frozenset(globals_),
        ns_vars=frozenset(ns_vars),
        upvar_aliases=upvar_aliases,
        traced=frozenset(traced),
    )


def _qualify_namespace(ns_arg: str, current: str) -> str:
    """Fully-qualify a ``namespace upvar`` namespace argument.

    Absolute args (``::a::b``) are returned as-is; a relative arg (``inner``) is
    resolved against the enclosing function's namespace *current* (so inside
    ``::outer::p`` it becomes ``::outer::inner``), matching how Tcl resolves it
    at runtime.  This makes ``namespace upvar inner x`` and
    ``namespace upvar ::outer::inner x`` produce the same owner."""
    if ns_arg.startswith("::"):
        return ns_arg
    if current in ("", "::"):
        return "::" + ns_arg
    return f"{current}::{ns_arg}"


def _collect_upvar_aliases(
    command: str, args: list[str], out: dict[str, str], *, namespace: str = "::"
) -> None:
    """Record local-alias → caller-target pairs for an ``upvar`` form.

    The local-alias indices come from the shared grammar; the caller target is
    the immediately-preceding arg (``"" `` ⇒ dynamic/unresolvable).  For
    ``namespace upvar NS src alias`` the owner is qualified with the
    fully-resolved ``NS`` (relative args resolved against *namespace*) so that
    ``namespace upvar ::ns1 x`` / ``namespace upvar ::ns2 x`` stay distinct and
    ``namespace upvar inner x`` / ``namespace upvar ::outer::inner x`` (inside
    ``::outer``) correctly overlap."""
    # The namespace argument of a `namespace upvar` form (None for plain upvar).
    ns_arg: str | None = None
    if command == "namespace" and args and args[0] == "upvar":
        ns_arg = args[1] if len(args) > 1 else None
    elif command == "namespace upvar":
        ns_arg = args[0] if args else None

    for local_i in upvar_local_declaration_indices(command, args, allow_dynamic_target=True):
        if not (0 <= local_i < len(args)):
            continue
        alias = normalise_var_name(args[local_i])
        target_arg = args[local_i - 1] if local_i - 1 >= 0 else ""
        # A target with any embedded substitution (``$x`` / ``[…]`` anywhere,
        # not only as a prefix — e.g. ``upvar 1 prefix_$x var``) names a runtime-
        # computed caller var, so it is unresolvable: record it as dynamic ("").
        dynamic = not target_arg or "$" in target_arg or "[" in target_arg
        if dynamic:
            target = ""
        elif ns_arg is not None:
            # namespace upvar: qualify the owner with the fully-resolved
            # namespace.  A dynamic namespace is itself unresolvable → dynamic.
            if "$" in ns_arg or "[" in ns_arg or not ns_arg:
                target = ""
            else:
                ns_fq = _qualify_namespace(ns_arg, namespace)
                target = f"{ns_fq.rstrip(':')}::{target_arg.lstrip(':')}"
        else:
            target = target_arg
        if alias:
            out[alias] = target


def _trace_target(args: list[str]) -> str | None:
    if len(args) >= 3 and args[0] == "add" and args[1] == "variable":
        return args[2]
    if len(args) >= 2 and args[0] == "variable":
        return args[1]
    return None


def def_places(stmt: IRStatement, ctx: ResolveContext) -> tuple[Place, ...]:
    """The places *stmt* writes (element-granular)."""
    if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
        return (resolve_place(stmt.name, ctx),)
    if isinstance(stmt, IRCall):
        return tuple(resolve_place(d, ctx) for d in stmt.defs)
    return ()


def _structural_body_indices(command: str, args: list[str], tokens) -> set[int]:
    """Arg indices that carry a *structural* body — a proc/method/test/snit body
    that runs in its own scope and is analysed separately, so its reads must NOT
    count as enclosing-scope reads.  Mirrors :func:`compiler.ssa._structural_body_indices`
    (only braced literal bodies are excluded; substituted ``-body $script`` words
    stay ordinary dataflow)."""
    if body_kind_for_command(command, args) is not BodyKind.STRUCTURAL:
        return set()
    out: set[int] = set()
    for idx in arg_indices_for_role(command, args, ArgRole.BODY):
        if not (0 <= idx < len(args)):
            continue
        if tokens is None:
            out.add(idx)
            continue
        from shared.tokens import TokenType

        tok_index = idx + 1
        if tok_index >= len(tokens.argv) or tokens.argv[tok_index].type is TokenType.STR:
            out.add(idx)
    return out


def _command_reads(script_text: str, ctx: ResolveContext, out: list[Place]) -> None:
    """Resolve the reads of every command in *script_text* — VAR_READ-role
    name args (``array get arr`` reads whole array ``arr``; ``info exists x``
    reads ``x``) plus a recursive descent of each arg word (for ``$``-refs and
    nested ``[...]`` substitutions)."""
    for cmd in segment_commands(script_text):
        texts = cmd.texts
        if not texts:
            continue
        name, args = texts[0], list(texts[1:])
        whole = name in _WHOLE_ARRAY_BARE or ("::" + name) in _WHOLE_ARRAY_COMMANDS
        for i in arg_indices_for_role(name, args, ArgRole.VAR_READ):
            if 0 <= i < len(args):
                if not args[i].startswith(("$", "[")):
                    out.append(resolve_place(args[i], ctx, whole_array=whole))
                else:
                    # Dynamic VAR_READ name (``array get $arr_name``): the read
                    # target is computed, so we can't say *which* var/array is
                    # read.  Emit UNKNOWN (overlaps everything) so a write to any
                    # array is not falsely flagged dead — the literal `$arr_name`
                    # scalar read is still recovered by ``_word_reads`` below.
                    out.append(unknown())
        for arg in (name, *args):
            _word_reads(arg, ctx, out)


def _word_reads(text: str, ctx: ResolveContext, out: list[Place]) -> None:
    """Resolve reads in a single word: ``$``-refs (precise, with array index)
    at the top level, and any ``[...]`` command substitution recursively."""
    if not text:
        return
    if "$" in text:
        for ref in scan_var_ref_forms(text):
            out.append(resolve_place(ref, ctx))
    if "[" in text:
        from shared.tokens import TokenType

        from .parsing.green_tree import tokenise

        toks, _ = tokenise(text, 0, 0, 0)
        for tok in toks:
            if tok.type is TokenType.CMD and tok.text:
                _command_reads(tok.text, ctx, out)


def read_places(stmt: IRStatement, ctx: ResolveContext) -> tuple[Place, ...]:
    """The places *stmt* reads (sound over-approximation).

    Covers ``$``-substitution reads (element-granular for arrays), VAR_READ-role
    name args at every nesting level (``[array get arr]`` reads the whole array),
    and the name/index variables read to *form* a dynamic target/ref (``set $X``
    reads ``X``; ``$arr($i)`` reads ``i``) via ``places_read_to_form``.
    """
    from .place import places_read_to_form

    out: list[Place] = []

    def add_refs(text: str) -> None:
        _word_reads(text, ctx, out)

    if isinstance(stmt, IRAssignValue):
        add_refs(stmt.value)
    elif isinstance(stmt, IRIncr):
        # `incr x` reads its own target; the amount may read more.
        out.append(resolve_place(stmt.name, ctx))
        if stmt.amount:
            add_refs(stmt.amount)
    elif isinstance(stmt, (IRAssignExpr, IRExprEval)):
        for nm in vars_in_expr_node(stmt.expr):
            out.append(resolve_place(nm, ctx))
        for cmd_text in command_texts_in_expr_node(stmt.expr):
            add_refs(cmd_text)
    elif isinstance(stmt, IRReturn):
        if stmt.value:
            add_refs(stmt.value)
        if stmt.expr is not None:
            for nm in vars_in_expr_node(stmt.expr):
                out.append(resolve_place(nm, ctx))
    elif isinstance(stmt, (IRCall, IRBarrier)):
        cmd = stmt.canonical_command
        args = list(stmt.args)
        whole = cmd in _WHOLE_ARRAY_COMMANDS
        read_idx = set(arg_indices_for_role(stmt.command, args, ArgRole.VAR_READ))
        # A structural body (proc/method/test/snit) runs in its own scope and is
        # analysed separately — its reads must not leak into this scope.
        structural = _structural_body_indices(stmt.command, args, getattr(stmt, "tokens", None))
        for i, arg in enumerate(args):
            if i in structural:
                continue
            if i in read_idx and not arg.startswith(("$", "[")):
                # A name-valued read arg (`info exists x`, `array get a`).
                out.append(resolve_place(arg, ctx, whole_array=whole))
            else:
                if i in read_idx:
                    # Dynamic VAR_READ name (`array get $arr_name`): the read
                    # target is computed — emit UNKNOWN (overlaps everything) so
                    # a write to any array isn't falsely flagged dead.
                    out.append(unknown())
                # ``add_refs`` keeps brace-suppression intact (a braced data word
                # like ``set msg {$unused}`` is correctly *not* a read); a body
                # script kept as an arg is scanned brace-aware as a whole word.
                add_refs(arg)
        add_refs(stmt.command)
    elif isinstance(stmt, IRBlock):
        # ``eval``/``namespace eval`` body is opaque to the CFG but its reads
        # run in (or escape to) the enclosing scope — recover them so the value
        # feeding a ``$x`` inside ``eval {puts $x}`` isn't seen as dead.
        for inner in stmt.body.statements:
            out.extend(read_places(inner, ctx))

    # The vars read to *form* any dynamic def target (set $X / set ${tok}(k) /
    # set a($i)) AND any dynamic read ref ($arr($i) reads i) — the genuine
    # name/index reads the dead-store/unused checks need.
    for dp in def_places(stmt, ctx):
        out.extend(places_read_to_form(dp))
    for rp in list(out):
        out.extend(places_read_to_form(rp))

    return tuple(out)


def terminator_read_places(term: object, ctx: ResolveContext) -> tuple[Place, ...]:
    """The places a block *terminator* reads — an ``if``/``while``/``for``
    branch condition or a ``return`` value/expr.  Covers the direct ``$``-refs
    plus any variables hidden in a command substitution within the condition
    (``if {[catch {f $x} e]} …`` reads ``x``)."""
    out: list[Place] = []
    if isinstance(term, CFGBranch):
        for nm in vars_in_expr_node(term.condition):
            out.append(resolve_place(nm, ctx))
        for cmd_text in command_texts_in_expr_node(term.condition):
            _word_reads(cmd_text, ctx, out)
    elif isinstance(term, CFGReturn) and not term.braced:
        # A braced ``return {…}`` is a literal — its ``$``-refs are not reads
        # (the IR strips the braces but flags ``braced``), so skip it entirely.
        if term.value:
            _word_reads(term.value, ctx, out)
        if term.expr is not None:
            for nm in vars_in_expr_node(term.expr):
                out.append(resolve_place(nm, ctx))
            for cmd_text in command_texts_in_expr_node(term.expr):
                _word_reads(cmd_text, ctx, out)
    return tuple(out)


__all__ = ["build_resolve_context", "def_places", "read_places", "terminator_read_places"]
