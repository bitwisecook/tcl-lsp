"""High-level entry point — ties parser, evaluator, and edit planner together.

This is what the ``f5 query`` verb calls.  Splitting the orchestration
out here keeps the CLI module focused on argparse plumbing and makes
the runner reusable from tests, MCP tools, or future ad-hoc scripts.
"""

from __future__ import annotations

import contextlib
from contextvars import ContextVar
from dataclasses import dataclass, field
from typing import Any

from ..parser import parse_bigip_conf
from .edit_plan import AppliedSource, apply
from .errors import QueryError
from .evaluator import EvalContext, evaluate_statement
from .parser import parse_query
from .source_map import SourceMap
from .values import Root

# Active roots are looked up by URI by graph builtins (``refs`` /
# ``referenced_by``) which need the per-file context without it being
# threaded through every call.  Stored in a :class:`ContextVar` of an
# immutable dict so the runner is safe to call concurrently from an
# LSP server, MCP tool, async editor command, or any other host that
# might run two queries in flight at once — each ``set`` returns a
# token the matching ``reset`` undoes, and contexts never share their
# underlying mapping.  A bare module-level ``dict`` would race.
_ACTIVE_ROOTS: ContextVar[dict[str, Root]] = ContextVar("f5_query_active_roots")

# Set to ``True`` while a ``--merge`` query is evaluating.  Graph
# builtins (``refs`` / ``referenced_by``) consult this to decide
# whether to walk just the originating source (per-file default) or
# every bound root (merge mode).  Stored in a :class:`ContextVar` so
# concurrent merge / non-merge contexts stay isolated.
_MERGE_ACTIVE: ContextVar[bool] = ContextVar("f5_query_merge_active", default=False)


def _is_merge_active() -> bool:
    """Return True when the current query was invoked with ``--merge``.

    Graph builtins call this to decide whether to widen ``compute_grep``
    over every bound root or stay scoped to the originating source.
    """
    return _MERGE_ACTIVE.get()


def _lookup_active_roots() -> dict[str, Root]:
    """Return every root bound in the current evaluation context.

    Graph builtins use this in merge mode to widen reference resolution
    across files; per-file mode still resolves against a single entry.
    """
    try:
        return _ACTIVE_ROOTS.get()
    except LookupError:
        return {}


def _lookup_active_root(uri: str) -> Root | None:
    """Return the root currently bound for *uri*, or ``None``.

    Graph builtins (``refs`` / ``referenced_by``) call this to find
    the per-file context they're evaluating against.
    """
    try:
        return _ACTIVE_ROOTS.get().get(uri)
    except LookupError:
        return None


@dataclass
class QueryResult:
    """The combined output of a single ``f5 query`` invocation.

    ``has_mutation`` reports whether the query *attempted* a mutation
    (queued any edit op or prefix rewrite).  The actual textual diff
    may still be empty when the query targets nothing — the CLI uses
    ``has_mutation`` to pick between the diff-or-write code path and
    the value-rendering one, and then independently checks whether
    each :class:`AppliedSource` actually changed to decide the exit
    code.
    """

    values_per_file: dict[str, list[Any]] = field(default_factory=dict)
    edits_per_file: dict[str, AppliedSource] = field(default_factory=dict)
    has_mutation: bool = False


@contextlib.contextmanager
def _active_root(root: Root):
    """Bind *root* into the per-context active-root map for the
    duration of the ``with`` block.

    Uses copy-on-write on the underlying dict + ``ContextVar.reset``
    so nested invocations (and concurrent contexts) never see each
    other's bindings.
    """
    try:
        current = _ACTIVE_ROOTS.get()
    except LookupError:
        current = {}
    new = dict(current)
    new[root.uri] = root
    token = _ACTIVE_ROOTS.set(new)
    try:
        yield
    finally:
        _ACTIVE_ROOTS.reset(token)


def _build_root(uri: str, source: str) -> Root:
    return Root(
        uri=uri,
        source=source,
        config=parse_bigip_conf(source),
        source_map=SourceMap.build(source),
    )


def run_query(
    query: str,
    sources: dict[str, str],
    *,
    names: dict[str, str] | None = None,
    merge: bool = False,
) -> QueryResult:
    """Parse *query* and run it against each file in *sources*.

    *names* binds DSL variables: ``names = {"ltm": "/path/ltm.conf",
    "gtm": "/path/gtm.conf"}`` makes ``$ltm`` and ``$gtm`` resolve to
    each source's root.  If *names* is ``None`` the runner auto-derives
    a name from each URI's filename stem (so ``ltm.conf`` becomes
    ``$ltm`` without ceremony).  Auto-derived collisions fall back to
    the unmodified URI.

    *merge* picks the dispatch mode for the primary input (the value
    seen by ``.`` at the top of every pipeline):

    - ``merge=False`` (the default): the query runs **once per source**,
      with each source taking its turn as the primary input.  Variables
      are still bound to all sources so cross-references via ``$name``
      work, but graph builtins stay scoped to the per-file source.
      This preserves the historical single-file behaviour and avoids
      surprising joins when callers pass two unrelated configs.
    - ``merge=True``: the query runs **once** with every loaded source
      iterated as a single stream from the top.  ``.ltm.virtual[]``
      yields virtuals from every input, ``refs`` / ``referenced_by``
      walk across files, and edits route back to the originating
      source via :func:`apply`'s built-in URI partitioning.

    Statements separated by ``;`` are evaluated and applied in order:
    each statement's edits land before the next runs, so a
    ``rename_partition("Common","Tenant_A"); .ltm.virtual[].destination = "..."``
    chain sees a coherent post-rewrite source by the second statement.
    Inside a single statement the planner still rejects mixing a
    prefix-cascade rewrite with field edits — the byte ranges captured
    at projection time would otherwise be wrong after the rewrite.
    """
    program = parse_query(query)
    from ..rewrite import RenameReport

    result = QueryResult()
    if merge:
        return _run_query_merged(program, sources, names)

    # When every top-level statement is rooted at a ``$name``
    # variable rather than the bare identity ``.``, the query has no
    # per-primary semantics — running it once per source would only
    # duplicate the same result into every URI bucket.  Evaluate
    # once against the first source so the result lands under one
    # URI; the named-source binding map is built against all loaded
    # sources so ``$ltm`` / ``$gtm`` still resolve.  Edits route via
    # ``target.obj.config_uri`` so the rewritten source still lands
    # on the originating file.
    if sources and _is_purely_variable_rooted(program):
        return _run_query_variable_rooted(program, sources, names)

    # Per-file iteration (default).  Each source takes its turn as the
    # primary input; variables bind to every loaded source so ``$other``
    # remains reachable from inside each iteration.
    for uri, source in sources.items():
        named_roots = _build_named_roots(
            sources={u: (s if u != uri else source) for u, s in sources.items()},
            names=names,
        )
        # Replace the entry for the iterating source so it tracks
        # current_source as edits accumulate.
        current_source = source
        accumulated_renames: list[RenameReport] = []
        accumulated_field_edits = 0
        last_values: list[Any] = []

        attempted_mutation = False
        for index, stmt in enumerate(program.statements):
            root = _build_root(uri, current_source)
            # Rebuild the named-root for the iterating source against
            # the current_source so ``$self.x`` reads post-edit state
            # within multi-statement queries.
            named_roots_for_step = dict(named_roots)
            for nm, r in list(named_roots_for_step.items()):
                if r.uri == uri:
                    named_roots_for_step[nm] = root
            ctx = EvalContext(root=root, named_roots=named_roots_for_step, merge_mode=False)
            with _active_roots({root.uri: root}):
                values = evaluate_statement(stmt, ctx)
            if index == len(program.statements) - 1:
                last_values = values
            if ctx.edits.has_edits():
                attempted_mutation = True
                # Edits may target the iterating source or any named
                # source the query reached via ``$name``.  Apply across
                # every source — :func:`apply` partitions by URI.
                all_sources_now = dict(sources)
                all_sources_now[uri] = current_source
                applied_per = apply(ctx.edits, all_sources_now)
                self_applied = applied_per.get(uri)
                if self_applied is not None:
                    current_source = self_applied.new_source
                    for rep in self_applied.rename_reports:
                        accumulated_renames.append(
                            RenameReport(
                                old=rep.old,
                                new=rep.new,
                                occurrences=rep.occurrences,
                                new_source="",
                            )
                        )
                    accumulated_field_edits += self_applied.field_edits
                # Cross-file edits (``$other.x.y = ...`` from inside a
                # per-file iteration) land directly on result.edits_per_file —
                # they originated from this iteration but target a different
                # source, so we merge them in below.
                for other_uri, other_applied in applied_per.items():
                    if other_uri == uri:
                        continue
                    existing = result.edits_per_file.get(other_uri)
                    base_original = (
                        existing.original if existing is not None else sources[other_uri]
                    )
                    base_renames = existing.rename_reports if existing is not None else ()
                    base_edits = existing.field_edits if existing is not None else 0
                    result.edits_per_file[other_uri] = AppliedSource(
                        uri=other_uri,
                        original=base_original,
                        new_source=other_applied.new_source,
                        rename_reports=base_renames
                        + tuple(
                            RenameReport(
                                old=r.old,
                                new=r.new,
                                occurrences=r.occurrences,
                                new_source="",
                            )
                            for r in other_applied.rename_reports
                        ),
                        field_edits=base_edits + other_applied.field_edits,
                    )

        result.values_per_file[uri] = last_values
        if attempted_mutation:
            result.has_mutation = True
            existing = result.edits_per_file.get(uri)
            base_renames = existing.rename_reports if existing is not None else ()
            base_edits = existing.field_edits if existing is not None else 0
            result.edits_per_file[uri] = AppliedSource(
                uri=uri,
                original=source,
                new_source=current_source,
                rename_reports=base_renames + tuple(accumulated_renames),
                field_edits=base_edits + accumulated_field_edits,
            )
    return result


def _run_query_merged(
    program,
    sources: dict[str, str],
    names: dict[str, str] | None,
) -> "QueryResult":
    """Run *program* in --merge mode: one pass over every loaded source.

    ``.ltm.virtual[]`` yields virtuals from every input; graph builtins
    (``refs`` / ``referenced_by``) walk references across files; edits
    route back to their originating source via :func:`apply`'s URI
    partitioning.  Hard-errors if two sources define the same
    ``(kind, full-path)``: a downstream user can always rename or scope
    inputs first, but silently merging two distinct definitions would
    produce undefined query results.
    """
    from ..rewrite import RenameReport

    result = QueryResult()
    if not sources:
        return result

    # Build all roots once.
    roots: dict[str, Root] = {uri: _build_root(uri, src) for uri, src in sources.items()}
    _detect_collisions(roots)
    named_roots = _build_named_roots(sources=sources, names=names, prebuilt=roots)

    # ``current_sources`` tracks post-edit text per URI as multi-
    # statement queries chain.  The primary stream is the concatenation
    # of every root's top-level container.
    current_sources: dict[str, str] = dict(sources)
    accumulated_renames_per_uri: dict[str, list[RenameReport]] = {u: [] for u in sources}
    accumulated_field_edits_per_uri: dict[str, int] = {u: 0 for u in sources}

    merge_token = _MERGE_ACTIVE.set(True)
    try:
        return _run_merge_statements(
            program=program,
            sources=sources,
            names=names,
            roots=roots,
            named_roots=named_roots,
            current_sources=current_sources,
            accumulated_renames_per_uri=accumulated_renames_per_uri,
            accumulated_field_edits_per_uri=accumulated_field_edits_per_uri,
            result=result,
        )
    finally:
        _MERGE_ACTIVE.reset(merge_token)


def _run_merge_statements(
    *,
    program,
    sources: dict[str, str],
    names: dict[str, str] | None,
    roots: dict[str, Root],
    named_roots: dict[str, Root],
    current_sources: dict[str, str],
    accumulated_renames_per_uri: dict,
    accumulated_field_edits_per_uri: dict,
    result: "QueryResult",
) -> "QueryResult":
    """Inner loop of merge mode, lifted so the ContextVar token can wrap it."""
    from ..rewrite import RenameReport

    last_values: list[Any] = []
    attempted_mutation = False

    for index, stmt in enumerate(program.statements):
        # Rebuild roots against current_sources so each statement sees
        # the post-edit text of every preceding statement.
        step_roots = {uri: _build_root(uri, current_sources[uri]) for uri in sources}
        step_named_roots = _build_named_roots(
            sources=current_sources, names=names, prebuilt=step_roots
        )
        # In merge mode the evaluator's "primary input" is a virtual
        # stream of every root's top-level container.  Rather than
        # invent a new container kind, we evaluate the statement once
        # per root and concatenate, with merge_mode=True so graph
        # builtins know to cross files.
        stmt_values: list[Any] = []
        plan = None
        for root in step_roots.values():
            ctx = EvalContext(
                root=root,
                named_roots=step_named_roots,
                merge_mode=True,
            )
            with _active_roots(step_roots):
                stmt_values.extend(evaluate_statement(stmt, ctx))
            if ctx.edits.has_edits():
                if plan is None:
                    plan = ctx.edits
                else:
                    plan.ops.extend(ctx.edits.ops)
                    plan.prefix_rewrites.extend(ctx.edits.prefix_rewrites)
        if index == len(program.statements) - 1:
            last_values = stmt_values
        if plan is not None and plan.has_edits():
            attempted_mutation = True
            applied_per = apply(plan, current_sources)
            for uri, applied in applied_per.items():
                current_sources[uri] = applied.new_source
                for rep in applied.rename_reports:
                    accumulated_renames_per_uri[uri].append(
                        RenameReport(
                            old=rep.old,
                            new=rep.new,
                            occurrences=rep.occurrences,
                            new_source="",
                        )
                    )
                accumulated_field_edits_per_uri[uri] += applied.field_edits

    # In merge mode all values flow into a single bucket — but to keep
    # the CLI's per-file rendering path working without a special case,
    # attribute every value to the first source's URI.  Callers that
    # need a true per-source split can run without --merge.
    primary_uri = next(iter(sources))
    result.values_per_file[primary_uri] = last_values
    if attempted_mutation:
        result.has_mutation = True
        for uri in sources:
            if (
                current_sources[uri] == sources[uri]
                and not accumulated_renames_per_uri[uri]
                and accumulated_field_edits_per_uri[uri] == 0
            ):
                continue
            result.edits_per_file[uri] = AppliedSource(
                uri=uri,
                original=sources[uri],
                new_source=current_sources[uri],
                rename_reports=tuple(accumulated_renames_per_uri[uri]),
                field_edits=accumulated_field_edits_per_uri[uri],
            )
    return result


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _is_purely_variable_rooted(program) -> bool:
    """True when every statement reaches its input only through ``$name``.

    Walks the program AST looking for any reference to the bare
    identity (the implicit ``.`` input that ``.field`` /
    ``.x.y[]`` / bare-builtin calls flow through).  If every
    statement is rooted at a ``$name`` variable instead — directly
    or as the LHS of a pipe — the query has no per-primary
    semantics and the runner should evaluate it once rather than
    once per loaded source.

    Conservative: any unresolved reference (``Identity`` /
    ``PathExpr`` without a leading ``$name`` / bare-builtin call /
    object literal entry resolving against ``.``) makes the whole
    program "primary-dependent" and the runner falls back to the
    historical per-file loop.
    """
    from .ast import (
        Assignment,
        BinOp,
        Call,
        Identity,
        LetBinding,
        ListLiteral,
        Literal,
        ObjectLiteral,
        PathExpr,
        Pipe,
        UnaryOp,
        Variable,
    )

    def _uses_primary(node) -> bool:
        if isinstance(node, Variable):
            return False
        if isinstance(node, Literal):
            return False
        if isinstance(node, Identity):
            return True
        if isinstance(node, PathExpr):
            # A PathExpr with no preceding ``$name`` reads from ``.``.
            return True
        if isinstance(node, Pipe):
            # ``$ltm.x | <body>`` — body runs against the LHS, not
            # against ``.``.  Recurse only into the LHS.
            return _uses_primary(node.lhs)
        if isinstance(node, Call):
            # ``length(.x)`` uses primary; ``length($x)`` doesn't.
            # Bare-builtin form (no args) is short for ``f(.)`` so
            # it always reads primary.  ``with_ctx`` builtins —
            # ``rename`` / ``rename_partition`` / ``rename_prefix`` /
            # ``rename_folder`` / ``check_partition_visibility`` —
            # also operate on the active root by design, so a query
            # whose top-level call is one of these depends on the
            # primary source even when its args are literals.
            if not node.args:
                return True
            from . import builtins as _builtins

            spec = _builtins.lookup(node.name)
            if spec is not None and spec.with_ctx:
                return True
            return any(_uses_primary(a) for a in node.args)
        if isinstance(node, BinOp):
            return _uses_primary(node.lhs) or _uses_primary(node.rhs)
        if isinstance(node, UnaryOp):
            return _uses_primary(node.operand)
        if isinstance(node, ListLiteral):
            return node.inner is not None and _uses_primary(node.inner)
        if isinstance(node, ObjectLiteral):
            return any(_uses_primary(v) for _, v in node.entries)
        if isinstance(node, LetBinding):
            # The source is evaluated against ``.``; the body runs
            # in the let-binding's scope but ``.`` remains the outer
            # input, so a primary reference in either still counts.
            return _uses_primary(node.source) or _uses_primary(node.body)
        if isinstance(node, Assignment):
            # ``$x.path = expr`` — assignment LHS is rooted at $x
            # (parser sets ``source``); rhs may read primary.
            target_uses_primary = node.source is None
            return target_uses_primary or _uses_primary(node.rhs)
        return True  # unknown shape: be conservative

    return not any(_uses_primary(stmt) for stmt in program.statements)


def _run_query_variable_rooted(
    program, sources: dict[str, str], names: dict[str, str] | None
) -> "QueryResult":
    """Evaluate a fully ``$name``-rooted program once against all loaded sources.

    Result values are attributed to the first source URI by
    convention (the program never referenced ``.`` so there's no
    "primary" to pick); edits route via ``target.obj.config_uri``
    so the rewritten source lands on the file the named root came
    from regardless of which URI hosts the values bucket.

    Each statement still re-projects the named roots against the
    post-edit text so a ``$ltm.x = a; $ltm.x = b`` chain sees the
    intermediate state on the second statement.
    """
    from ..rewrite import RenameReport
    from .edit_plan import apply

    result = QueryResult()
    current_sources = dict(sources)
    primary_uri = next(iter(sources))
    accumulated_renames_per_uri: dict[str, list[RenameReport]] = {u: [] for u in sources}
    accumulated_field_edits_per_uri: dict[str, int] = {u: 0 for u in sources}
    last_values: list[Any] = []
    attempted_mutation = False

    for index, stmt in enumerate(program.statements):
        # Rebuild roots against the post-edit text so $name reads
        # see the same coherent state the per-file loop would.
        step_roots = {uri: _build_root(uri, current_sources[uri]) for uri in sources}
        step_named_roots = _build_named_roots(
            sources=current_sources, names=names, prebuilt=step_roots
        )
        primary_root = step_roots[primary_uri]
        ctx = EvalContext(
            root=primary_root,
            named_roots=step_named_roots,
        )
        with _active_roots(step_roots):
            values = evaluate_statement(stmt, ctx)
        if index == len(program.statements) - 1:
            last_values = values
        if ctx.edits.has_edits():
            attempted_mutation = True
            applied_per = apply(ctx.edits, current_sources)
            for uri, applied in applied_per.items():
                current_sources[uri] = applied.new_source
                for rep in applied.rename_reports:
                    accumulated_renames_per_uri[uri].append(
                        RenameReport(
                            old=rep.old,
                            new=rep.new,
                            occurrences=rep.occurrences,
                            new_source="",
                        )
                    )
                accumulated_field_edits_per_uri[uri] += applied.field_edits

    result.values_per_file[primary_uri] = last_values
    if attempted_mutation:
        result.has_mutation = True
        for uri in sources:
            if (
                current_sources[uri] == sources[uri]
                and not accumulated_renames_per_uri[uri]
                and accumulated_field_edits_per_uri[uri] == 0
            ):
                continue
            result.edits_per_file[uri] = AppliedSource(
                uri=uri,
                original=sources[uri],
                new_source=current_sources[uri],
                rename_reports=tuple(accumulated_renames_per_uri[uri]),
                field_edits=accumulated_field_edits_per_uri[uri],
            )
    return result


def _filename_stem(uri: str) -> str:
    """Derive a default $-name from a URI.

    Strips any directory prefix and the trailing extension so
    ``/path/to/bigip-ltm.conf`` becomes ``bigip-ltm``.  ``-`` (stdin)
    becomes ``stdin``.
    """
    if uri == "-":
        return "stdin"
    base = uri.rsplit("/", 1)[-1]
    if "." in base:
        base = base.rsplit(".", 1)[0]
    return base or uri


def _build_named_roots(
    *,
    sources: dict[str, str],
    names: dict[str, str] | None,
    prebuilt: dict[str, Root] | None = None,
) -> dict[str, Root]:
    """Build ``$name → Root`` bindings for every loaded source.

    Explicit *names* (``--name N=PATH``) win.  Remaining sources fall
    back to filename-stem auto-naming; when two stems collide the
    later source is bound under its full URI instead so the earlier
    name keeps working unambiguously.
    """
    bindings: dict[str, Root] = {}
    used: set[str] = set()
    if names:
        for nm, uri in names.items():
            if uri not in sources:
                continue
            root = (prebuilt or {}).get(uri) or _build_root(uri, sources[uri])
            bindings[nm] = root
            used.add(uri)
    for uri, src in sources.items():
        if uri in used:
            continue
        stem = _filename_stem(uri)
        if stem in bindings:
            # Collision: keep the earlier binding, also expose the
            # later source under its URI to keep it reachable.
            bindings[uri] = (prebuilt or {}).get(uri) or _build_root(uri, src)
            continue
        bindings[stem] = (prebuilt or {}).get(uri) or _build_root(uri, src)
    return bindings


def _detect_collisions(roots: dict[str, Root]) -> None:
    """Hard-error if two roots define the same ``(kind, full-path)``.

    Merge mode walks all configs as one logical namespace; identical
    object identities across files would otherwise yield ambiguous
    query results (which body wins under ``.ltm.pool["/Common/p1"]``?)
    and unsafe edits (renames touching one source could orphan
    references in another).  Forcing the user to redact, namespace,
    or partition the inputs is the only correct answer.

    Identical-bytes duplicates would be safe in principle but are
    just as confusing in practice — refuse them too so the error
    message is uniform.
    """
    from ..model._config import BigipConfig

    seen: dict[tuple[str, str], str] = {}
    collisions: list[tuple[str, str, str, str]] = []
    for uri, root in roots.items():
        cfg = root.config
        for attr in BigipConfig.__dataclass_fields__:
            container = getattr(cfg, attr, None)
            if not isinstance(container, dict):
                continue
            for full_path in container:
                key = (attr, full_path)
                prior = seen.get(key)
                if prior is None:
                    seen[key] = uri
                    continue
                collisions.append((attr, full_path, prior, uri))
    if not collisions:
        return
    head = collisions[:5]
    extra = len(collisions) - len(head)
    lines = [
        f"  {full_path} ({attr.replace('_', ' ')}) in {prior_uri} and {uri}"
        for attr, full_path, prior_uri, uri in head
    ]
    if extra > 0:
        lines.append(f"  ... and {extra} more")
    raise QueryError(
        "--merge: refusing to merge configs that define the same object in "
        "more than one source:\n" + "\n".join(lines)
    )


@contextlib.contextmanager
def _active_roots(roots: dict[str, Root]):
    """Bind multiple roots into the per-context active-root map.

    A convenience over :func:`_active_root` that takes the whole map at
    once; used by both per-file iteration (with a single root) and
    merge mode (with every loaded root).  Copy-on-write semantics carry
    over from the single-root form so concurrent contexts stay
    isolated.
    """
    try:
        current = _ACTIVE_ROOTS.get()
    except LookupError:
        current = {}
    new = dict(current)
    new.update(roots)
    token = _ACTIVE_ROOTS.set(new)
    try:
        yield
    finally:
        _ACTIVE_ROOTS.reset(token)
