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
            # Bare-leaf new values (no ``/`` at all) are resolved
            # against the existing object's partition + folder
            # context so ``.name = "X"`` (or ``.name |= with_name(.,
            # "X")``) keeps the object in the same partition without
            # the user having to remember to write a full path or
            # compose ``with_partition`` / ``with_folder``.  The
            # ``rename()`` builtin and ``f5 rename`` CLI always
            # supply a full path so this branch is a no-op for them.
            if "/" not in new_path and "/" in op.object_path:
                folder = op.object_path.rsplit("/", 1)[0]
                new_path = f"{folder}/{new_path}"
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


# Fields that ``+=`` / ``=`` can materialise as a fresh
# ``<field> { ... }`` block on a stanza that doesn't already have one.
# Restricted to *flat* list slots whose elements stringify cleanly to
# bare tokens — VS attachments by full-path.  Pool members are
# intentionally excluded: their list elements are sub-block-shaped
# objects (``/Common/n1:80 { address … port … monitor … }``) that the
# generic materialiser would flatten into a meaningless token sequence
# and produce invalid SCF.
#
# Several field names also appear on kinds whose list elements are
# compound sub-blocks (``ltm policy.rules`` is a tuple of named
# rule objects, not a list of paths).  The bare field name is not
# enough to know whether materialisation is safe, so we additionally
# pin to (object_kind, field_name) pairs and reject any list whose
# elements aren't flat scalars / PathRefs at apply time.
_MATERIALISABLE_KIND_FIELDS = frozenset(
    {
        ("ltm virtual", "rules"),
        ("ltm virtual", "profiles"),
        ("ltm virtual", "persist"),
        ("ltm virtual", "policies"),
    }
)


def _is_flat_list_element(value: Any) -> bool:
    """Return True iff *value* renders cleanly as a bare SCF token.

    The materialiser joins list items with spaces and emits the result
    verbatim, so we only accept types whose ``__str__`` produces
    syntactically valid SCF: bare scalars and :class:`PathRef`
    (which renders to its full-path).  ObjectRef / dict / list /
    nested containers are rejected — they need a typed encoder we
    don't have in v1.
    """
    return isinstance(value, (str, int, float, bool, PathRef)) or value is None


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
            new_text = _format_value(
                op.new_value,
                original_raw=op.field_slot.raw_text,
                field_name=op.field_name,
            )
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
    new_source = "".join(out_parts)

    # Defence-in-depth reparse — even with the SCF scalar encoder
    # filtering destination strings, the generic ``_format_value``
    # path could still hit a code path (or a future field type) that
    # produces malformed SCF.  ``rename_object`` and the prefix
    # rewrite path already do this; field edits skipped it.  Mirror
    # the rename behaviour: if the rewritten source no longer parses,
    # raise instead of letting ``--write`` / ``--in-place`` persist a
    # broken config.
    try:
        parse_bigip_conf(new_source)
    except Exception as exc:  # noqa: BLE001
        raise EditError(f"field edit produced invalid SCF in {uri}: {exc}") from exc

    return new_source


def _format_value(value: Any, *, original_raw: str, field_name: str = "") -> str:
    """Render *value* for splicing back into source text.

    Strings are encoded through :func:`_encode_tmsh_scalar`, which
    bare-emits identifier-shaped tokens, double-quotes anything that
    contains whitespace / quotes / semicolons / comment markers (with
    backslash-escapes for embedded ``"`` and ``\\``), and rejects
    values whose characters can't be safely represented in SCF at all
    (newlines, braces, NUL / control chars).  Lists become brace-
    delimited groups, mirroring the spacing of the original value.
    Other scalars are converted with ``str``.
    """
    if isinstance(value, PathRef):
        return value.full_path
    if isinstance(value, list):
        items = [_format_value(v, original_raw="", field_name=field_name) for v in value]
        # Preserve a brace wrapper if the original value used one.
        if original_raw.startswith("{"):
            return "{ " + " ".join(items) + " }"
        return " ".join(items)
    if value is None:
        return "none"
    if isinstance(value, bool):
        return "enabled" if value else "disabled"
    if isinstance(value, str):
        return _encode_tmsh_scalar(value, field_name=field_name)
    return str(value)


# Characters whose presence in a scalar value forces double-quoting.
# Whitespace and the SCF-significant punctuation (``"`` / ``;`` /
# ``#`` / ``[`` / ``]``) all need quoting; bare tokens are restricted
# to identifier-shaped sequences.
_TMSH_REQUIRES_QUOTING = re.compile(r'[\s"\[\];#]')

# Characters that have *no* safe textual representation inside an SCF
# scalar — they break the brace-balanced structure or the line-oriented
# parser, and the right answer is to refuse the edit.  Newlines are
# the most important entry: a value with ``\n`` in it would split the
# stanza body and inject arbitrary content into the surrounding scope
# (the CVE-shaped concern that motivated the encoder).
_TMSH_FORBIDDEN_IN_VALUE = re.compile(r"[\n\r\x00-\x08\x0b\x0c\x0e-\x1f{}]")


def _encode_tmsh_scalar(value: str, *, field_name: str = "") -> str:
    """Encode *value* as an SCF scalar token.

    Three cases:

    - ``""`` (empty string) → emit ``""`` so the slot stays
      syntactically valid.
    - Identifier-shaped value (no whitespace / no SCF punctuation) →
      emit bare.
    - Anything else that can be quoted → wrap in ``"..."`` with
      backslash-escapes for embedded ``\\`` and ``"``.

    Values containing characters from
    :data:`_TMSH_FORBIDDEN_IN_VALUE` (newlines / braces / NUL / other
    control chars) raise :class:`EditError`: the only way to "represent"
    such a value in SCF would be to break the brace-balanced structure
    or open a new scope, both of which are stanza-injection vectors.
    The caller can either pre-sanitise its input or refuse to write
    that field.
    """
    forbidden = _TMSH_FORBIDDEN_IN_VALUE.search(value)
    if forbidden is not None:
        bad = forbidden.group(0)
        ctx = f" for field {field_name!r}" if field_name else ""
        raise EditError(
            f"value contains character {bad!r} that cannot be safely "
            f"represented in SCF / TMSH{ctx}: newlines, braces, and "
            f"control characters break the brace-balanced format and "
            f"would corrupt the surrounding stanza"
        )
    if value == "":
        return '""'
    if _TMSH_REQUIRES_QUOTING.search(value):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return value


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

    - The op's ``(object_kind, field_name)`` pair is in
      :data:`_MATERIALISABLE_KIND_FIELDS`.  A bare field name isn't
      enough: ``rules`` on ``ltm virtual`` is a flat list of paths,
      but ``rules`` on ``ltm policy`` is a tuple of compound
      sub-block objects — materialising the latter would flatten
      ``BigipPolicyRule`` reprs into the source and produce invalid
      SCF.  Pool ``members`` is also intentionally absent: its
      elements are sub-block-shaped.
    - Every element of ``new_value`` is a flat scalar or PathRef
      (see :func:`_is_flat_list_element`).  Anything else is
      rejected loudly rather than silently corrupting the stanza.
    - The op has a ``stanza_slot`` so we know where to insert.
    - The op's ``new_value`` is a non-empty list (an empty list would
      produce ``<field> { }`` — no-op, skipped).
    - The operator is ``+=`` or ``=``.  ``-=`` against an absent
      block is a no-op (nothing to remove); we leave it to the
      existing error path.
    """
    if (op.object_kind, op.field_name) not in _MATERIALISABLE_KIND_FIELDS:
        return None
    if op.stanza_slot is None:
        return None
    if op.operator not in ("+=", "="):
        return None
    new_value = op.new_value
    if not isinstance(new_value, list) or not new_value:
        return None
    bad = next((v for v in new_value if not _is_flat_list_element(v)), None)
    if bad is not None:
        bad_kind = type(bad).__name__
        raise EditError(
            f"cannot materialise {op.field_name!r} on {op.object_path!r}: "
            f"list element of type {bad_kind} is not a flat SCF token "
            f"(materialisation is restricted to scalars and PathRefs)"
        )

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

    items_text = " ".join(
        _format_value(v, original_raw="", field_name=op.field_name) for v in new_value
    )

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
