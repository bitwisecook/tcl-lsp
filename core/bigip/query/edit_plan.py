"""Edit plan: collect, route, and apply query-driven source edits.

Every assignment produced by the evaluator turns into an
:class:`EditOp`.  The plan groups them by source URI, routes
identity-field writes through :func:`core.bigip.rewrite.rename_object`,
detects overlapping ranges, and applies the remaining edits
bottom-up so earlier offsets stay valid.

The applier is intentionally text-oriented: it never round-trips
through the parser, so comments, whitespace, key order, and unknown
stanzas all survive.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any

from ..parser import parse_bigip_conf
from ..rewrite import RenameReport, rename_object
from .errors import EditError
from .values import FieldSlot, PathRef

_IDENTITY_FIELDS = frozenset({"name", "full-path"})


@dataclass(frozen=True, slots=True)
class EditOp:
    """A single (object, field) → new-value edit recorded by the evaluator."""

    source_uri: str
    object_path: str
    object_kind: str
    field_name: str
    operator: str  # "=", "|=", "+=", "-="
    old_value: Any
    new_value: Any
    field_slot: FieldSlot | None
    stanza_slot: FieldSlot | None
    # When ``strict`` is True (the default) a zero-occurrence rename
    # raises :class:`EditError`.  Builtins that *intend* a tolerant
    # search-and-replace (e.g. the ``rename()`` builtin that backs
    # ``f5 rename``) set this to False; the applier then skips the op
    # silently and the CLI surfaces the no-match with a warning + an
    # exit-code-1 instead of an error.
    strict: bool = True


@dataclass(frozen=True, slots=True)
class PrefixRewrite:
    """A whole-source regex prefix substitution.

    Used by cascade operations such as ``rename_partition`` that need to
    rewrite *every* occurrence of a prefix — including ones embedded in
    compound values (destination addresses, pool-member names, iRule
    body literals) that are not standalone object identifiers and so
    fall outside ``rename_object``'s token-bounded match.

    The pattern is compiled with the same token boundaries
    ``rename_object`` uses, so prefix substitutions are still
    identifier-safe — they won't rewrite a longer name that happens to
    share a leading substring.
    """

    source_uri: str
    label: str  # human-readable LHS, for stderr summaries
    pattern: re.Pattern[str]
    replacement: str
    # Human-readable rendering of the *destination* for the stderr
    # summary.  ``replacement`` may carry regex backrefs (``\g<1>``)
    # that confuse users when surfaced in a "renamed X -> Y" line;
    # ``human_new`` lets the scheduler supply the user-visible form
    # (e.g. ``"auth partition Tenant_A"``) directly.  Defaults to
    # ``replacement`` when omitted.
    human_new: str = ""


@dataclass
class EditPlan:
    """Collected edits, applied once at the end of a query run."""

    ops: list[EditOp] = field(default_factory=list)
    prefix_rewrites: list[PrefixRewrite] = field(default_factory=list)

    def add(self, op: EditOp) -> None:
        self.ops.append(op)

    def add_prefix(self, rewrite: PrefixRewrite) -> None:
        self.prefix_rewrites.append(rewrite)

    def has_edits(self) -> bool:
        return bool(self.ops) or bool(self.prefix_rewrites)


@dataclass(frozen=True, slots=True)
class AppliedSource:
    """One source file's result after edits land."""

    uri: str
    original: str
    new_source: str
    rename_reports: tuple[RenameReport, ...]
    field_edits: int


def apply(plan: EditPlan, sources: dict[str, str]) -> dict[str, AppliedSource]:
    """Apply every op in *plan* to *sources*.

    Returns one :class:`AppliedSource` per touched URI.  The applier
    runs cascading prefix rewrites first (where present), then routes
    identity writes through ``rename_object``, and finally splices
    field edits.

    Mixing prefix rewrites with field edits in the same query is
    rejected: a prefix rewrite changes byte offsets across the source,
    so field-slot ranges captured against the original text would
    target the wrong span after the rewrite.  Run them in separate
    statements (``;``-separated) and they will be applied in order;
    inside a single statement, pick one mode.

    Overlapping field edits raise :class:`EditError`.
    """
    by_uri: dict[str, list[EditOp]] = {}
    for op in plan.ops:
        by_uri.setdefault(op.source_uri, []).append(op)
    prefix_by_uri: dict[str, list[PrefixRewrite]] = {}
    for pr in plan.prefix_rewrites:
        prefix_by_uri.setdefault(pr.source_uri, []).append(pr)

    out: dict[str, AppliedSource] = {}
    touched = set(by_uri) | set(prefix_by_uri)

    for uri in touched:
        ops = by_uri.get(uri, [])
        prefixes = prefix_by_uri.get(uri, [])

        if prefixes and any(op.field_name not in _IDENTITY_FIELDS for op in ops):
            raise EditError(
                "cannot mix prefix-cascade rewrites (e.g. rename_partition) "
                "with field edits in a single statement; split them with ';' "
                "and the runner will apply each statement against the post-"
                "rewrite source"
            )

        source = sources[uri]
        current = source
        rename_reports: list[RenameReport] = []

        # Apply every prefix rewrite first, building a synthetic
        # RenameReport per pattern so the CLI can surface the count.
        for pr in prefixes:
            new_text, count = pr.pattern.subn(pr.replacement, current)
            if count == 0:
                continue
            try:
                parse_bigip_conf(new_text)
            except Exception as exc:  # noqa: BLE001
                raise EditError(
                    f"prefix rewrite for {pr.label!r} produced invalid SCF: {exc}"
                ) from exc
            current = new_text
            # Synthetic report for the prefix rewrite.  The CLI only
            # reads ``old`` / ``new`` / ``occurrences`` for the stderr
            # summary; leaving ``new_source`` blank avoids retaining a
            # full-source copy per prefix rewrite (multi-step queries
            # can otherwise hold O(k * source-size) bytes).  The
            # canonical post-rewrite text is on ``AppliedSource``.
            # ``human_new`` carries the user-facing target string —
            # the raw ``replacement`` may contain regex backrefs
            # (``\g<1>...``) that would otherwise leak into the
            # "renamed X -> Y" line.
            rename_reports.append(
                RenameReport(
                    old=pr.label,
                    new=pr.human_new or pr.replacement,
                    occurrences=count,
                    new_source="",
                )
            )

        # Field/identity ops collected from regular assignments.

        identity_ops: list[EditOp] = []
        field_ops: list[EditOp] = []
        for op in ops:
            # ``|=`` on an identity field is sugar for ``= (rhs evaluated
            # against the current name)`` and routes through the same
            # rename machinery — the evaluator has already computed the
            # new value, we just have to admit it here.  ``+=`` / ``-=``
            # on identity fields stay rejected: arithmetic on a name is
            # nonsensical.
            if op.field_name in _IDENTITY_FIELDS:
                if op.operator in ("+=", "-="):
                    raise EditError(
                        f"assignment {op.operator} to identity field "
                        f"{op.field_name!r} is not supported"
                    )
                identity_ops.append(op)
            else:
                field_ops.append(op)

        for op in identity_ops:
            new_path = _stringify(op.new_value)
            if not new_path:
                raise EditError(f"rename target for {op.object_path!r} produced an empty value")
            # Pass ``object_kind`` as the rename scope so a rename
            # driven by ``.<kind>[X].name = Y`` doesn't accidentally
            # rewrite the *header* of a different-kind stanza that
            # happens to share the same full-path (e.g. a pool and a
            # virtual server both called ``/Common/shared``).  The
            # ``rename()`` builtin and ``f5 rename`` CLI leave
            # ``object_kind`` empty for legacy global behaviour.
            report = rename_object(current, op.object_path, new_path, kind_scope=op.object_kind)
            if report.occurrences == 0:
                if op.strict:
                    raise EditError(f"rename of {op.object_path!r} matched no source text")
                # Tolerant rename (e.g. via the ``rename()`` builtin): leave
                # the source unchanged and emit no RenameReport.  The CLI
                # detects the no-op via the post-apply diff and exits 1.
                continue
            current = report.new_source
            rename_reports.append(report)

        if field_ops:
            current = _splice_edits(current, field_ops, uri)
            field_edit_count = len(field_ops)
        else:
            field_edit_count = 0

        out[uri] = AppliedSource(
            uri=uri,
            original=source,
            new_source=current,
            rename_reports=tuple(rename_reports),
            field_edits=field_edit_count,
        )
    return out


# Fields where ``+=`` against a missing block should materialise a
# fresh ``<field> { ... }`` compound block inside the stanza.  These
# are the common BIG-IP list-shaped property slots; the value
# projected by the evaluator is already a Python ``list``, so the
# splice just wraps it in a brace block.
_MATERIALISABLE_LIST_FIELDS = frozenset({"rules", "profiles", "persist", "policies", "members"})


def _splice_edits(source: str, ops: list[EditOp], uri: str) -> str:
    """Apply non-identity edits to *source* bottom-up.

    Each op must either carry a :attr:`EditOp.field_slot` (an existing
    property to overwrite) or target a missing **list field** on a
    stanza we know how to extend.  In the materialise case we insert
    a fresh ``<field> { ... }`` block before the stanza's closing
    ``}``; this is how ``.ltm.virtual[].rules += "/Common/log"``
    works against VSes that didn't have a ``rules`` block before.

    Slots are checked for overlap before application so two writes
    against the same span fail loudly.
    """
    placed: list[tuple[int, int, str, EditOp]] = []
    for op in ops:
        if op.field_slot is not None:
            new_text = _format_value(op.new_value, original_raw=op.field_slot.raw_text)
            placed.append((op.field_slot.start, op.field_slot.end, new_text, op))
            continue
        # No field_slot — see if we can materialise a fresh compound block.
        insert = _materialise_compound_block(source, op)
        if insert is not None:
            placed.append(insert)
            continue
        raise EditError(
            f"cannot edit {op.field_name!r} on {op.object_path!r}: "
            "this field has no single-line slot in the source "
            "(compound values are not writable in v1)"
        )

    placed.sort(key=lambda t: (t[0], t[1]))
    for i in range(1, len(placed)):
        prev_start, prev_end, _, prev_op = placed[i - 1]
        start, end, _, op = placed[i]
        if start < prev_end:
            raise EditError(
                f"overlapping edits at {uri}: "
                f"{prev_op.object_path}.{prev_op.field_name} and "
                f"{op.object_path}.{op.field_name}"
            )

    out_parts: list[str] = []
    cursor = 0
    # Apply bottom-up by iterating in reverse and rebuilding around
    # each splice; a single forward pass is just as correct, easier to
    # reason about, and used here.
    for start, end, new_text, _ in placed:
        out_parts.append(source[cursor:start])
        out_parts.append(new_text)
        cursor = end
    out_parts.append(source[cursor:])
    return "".join(out_parts)


def _format_value(value: Any, *, original_raw: str) -> str:
    """Render *value* for splicing back into source text.

    Strings are emitted as-is (no quoting — TMSH values are bare
    tokens).  Lists become brace-delimited groups, mirroring the
    spacing of the original value.  Other scalars are converted with
    ``str``.
    """
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, list):
        items = [_format_value(v, original_raw="") for v in value]
        # Preserve a brace wrapper if the original value used one.
        if original_raw.startswith("{"):
            return "{ " + " ".join(items) + " }"
        return " ".join(items)
    if value is None:
        return "none"
    if isinstance(value, bool):
        return "enabled" if value else "disabled"
    return str(value)


def _materialise_compound_block(source: str, op: EditOp) -> tuple[int, int, str, EditOp] | None:
    """Return an edit that overwrites or materialises a ``<field> { ... }``
    compound block on the op's stanza, or ``None`` when the op isn't a
    candidate.

    When the stanza already has an existing ``<field> { ... }`` block,
    the returned edit overwrites that block's byte range.  When the
    block is absent, the edit inserts a fresh ``<field> { ... }``
    line before the stanza's closing brace.  Either way the indent
    is sniffed from the stanza body so the rewritten source stays
    consistent with the surrounding properties.

    Eligibility:

    - The op's ``field_name`` is one of the known
      :data:`_MATERIALISABLE_LIST_FIELDS` (``rules`` / ``profiles`` /
      ``persist`` / ``policies`` / ``members``).
    - The op has a ``stanza_slot`` so we know where to insert.
    - The op's ``new_value`` is a non-empty list (an empty list would
      produce ``<field> { }`` — no-op, skipped).
    - The operator is ``+=`` or ``=``.  ``-=`` against an absent
      block is a no-op (nothing to remove); we leave it to the
      existing error path.
    """
    if op.field_name not in _MATERIALISABLE_LIST_FIELDS:
        return None
    if op.stanza_slot is None:
        return None
    if op.operator not in ("+=", "="):
        return None
    new_value = op.new_value
    if not isinstance(new_value, list) or not new_value:
        return None

    stanza_start = op.stanza_slot.start
    stanza_end = op.stanza_slot.end
    closing_brace = source.rfind("}", stanza_start, stanza_end)
    if closing_brace == -1:
        return None
    body_text = source[stanza_start:stanza_end]

    # Sniff indent from the line immediately before the closing brace.
    line_before_close_start = source.rfind("\n", stanza_start, closing_brace) + 1
    if line_before_close_start > 0:
        prev_line_end = line_before_close_start - 1
        prev_line_start = source.rfind("\n", stanza_start, prev_line_end) + 1
        prev_line = source[prev_line_start:prev_line_end]
        indent = prev_line[: len(prev_line) - len(prev_line.lstrip(" \t"))]
        if not indent:
            indent = "    "
    else:
        indent = "    "

    items_text = " ".join(_format_value(v, original_raw="") for v in new_value)

    # Is there an existing ``<field> { ... }`` block to overwrite?
    # Search the body for a top-level ``<indent>?<field>\s*{`` and
    # walk to its matching closing brace.
    existing = _find_top_level_block(body_text, op.field_name)
    if existing is not None:
        start_in_body, end_in_body = existing
        abs_start = stanza_start + start_in_body
        abs_end = stanza_start + end_in_body
        # The replacement starts at the existing block's name token,
        # so the source's leading indent for the line is preserved
        # implicitly — no need to re-emit it here.
        new_text = f"{op.field_name} {{ {items_text} }}"
        return (abs_start, abs_end, new_text, op)

    # No existing block — insert before the closing brace.  Need a
    # leading indent and a trailing newline so the closing ``}``
    # stays on its own line.
    block = f"{indent}{op.field_name} {{ {items_text} }}\n"
    return (closing_brace, closing_brace, block, op)


def _find_top_level_block(body: str, name: str) -> tuple[int, int] | None:
    """Locate ``<name> { ... }`` at the top level of *body*.

    Top-level means brace-depth 0 outside the candidate block.  The
    returned span covers ``<name> { ... }`` exactly (the name token
    through the matching closing brace).  Returns ``None`` when no
    such block exists.
    """
    pattern = re.compile(rf"\b{re.escape(name)}\s*\{{")
    depth = 0
    i = 0
    n = len(body)
    while i < n:
        ch = body[i]
        if ch == "{":
            depth += 1
            i += 1
            continue
        if ch == "}":
            depth -= 1
            i += 1
            continue
        if ch == '"':
            i += 1
            while i < n and body[i] != '"':
                if body[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                i += 1
            i += 1
            continue
        if depth == 1 and ch == name[0]:
            match = pattern.match(body, i)
            if match is not None:
                # Walk to the matching closing brace.
                inner_depth = 1
                j = match.end()
                while j < n and inner_depth > 0:
                    if body[j] == "{":
                        inner_depth += 1
                    elif body[j] == "}":
                        inner_depth -= 1
                    elif body[j] == '"':
                        j += 1
                        while j < n and body[j] != '"':
                            if body[j] == "\\" and j + 1 < n:
                                j += 2
                                continue
                            j += 1
                    j += 1
                if inner_depth == 0:
                    return (match.start(), j)
                return None
        i += 1
    return None


def _stringify(value: Any) -> str:
    if isinstance(value, PathRef):
        return value.full_path
    return str(value)
