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
    label: str  # human-readable, for stderr summaries
    pattern: re.Pattern[str]
    replacement: str


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
            rename_reports.append(
                RenameReport(
                    old=pr.label,
                    new=pr.replacement,
                    occurrences=count,
                    new_source=current,
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
            report = rename_object(current, op.object_path, new_path)
            if report.occurrences == 0:
                raise EditError(f"rename of {op.object_path!r} matched no source text")
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


def _splice_edits(source: str, ops: list[EditOp], uri: str) -> str:
    """Apply non-identity edits to *source* bottom-up.

    Each op must carry a :attr:`EditOp.field_slot`; without one we
    cannot place the edit, so we reject the query with a clear error
    rather than guessing.  Slots are checked for overlap before
    application so two writes against the same span fail loudly.
    """
    placed: list[tuple[int, int, str, EditOp]] = []
    for op in ops:
        if op.field_slot is None:
            raise EditError(
                f"cannot edit {op.field_name!r} on {op.object_path!r}: "
                "this field has no single-line slot in the source "
                "(compound values are not writable in v1)"
            )
        new_text = _format_value(op.new_value, original_raw=op.field_slot.raw_text)
        placed.append((op.field_slot.start, op.field_slot.end, new_text, op))

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


def _stringify(value: Any) -> str:
    if isinstance(value, PathRef):
        return value.full_path
    return str(value)
