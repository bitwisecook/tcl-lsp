"""Value-spec protocol and concrete value specs for the BIG-IP registry.

A *value spec* declares the exact shape of a single property's value:
how the parser should turn its raw textual form into a structured
value, how the query DSL should project it, how the edit planner
should render it back to text, and which references it carries for
the graph / LSP layers.

The protocol is intentionally narrow — one tiny method per stage:

    class ValueSpec:
        def parse(raw: str, ctx) -> ParsedValue: ...
        def project(value, ctx) -> object: ...
        def render(value, ctx) -> str: ...
        def references(value, ctx) -> Iterable[Reference]: ...

Concrete specs (``StringSpec``, ``IntSpec``, ``EnumSpec``,
``BoolSpec``, ``ObjectRefSpec``, ``ListSpec``, ``DestinationSpec``,
``NetworkSpec``, ``AddressSpec``, ``PortSpec``) plug into this
protocol.  Compound specs (``MonitorExpressionSpec``,
``ProfileAttachmentSpec``, ``PersistenceAttachmentSpec``,
``SnatModeSpec``, ``DataGroupRecordSpec``, ``GtmRegionMemberSpec``,
``CertKeyChainSpec``, ``LtmPolicyConditionSpec``,
``LtmPolicyActionSpec``, ``LtmPolicyRuleSpec``, ``FirewallRuleSpec``,
``NatRuleSpec``) round out the catalogue.

Phases 1–6 of the rearchitecture have all landed: every dispatch
stage (projection / edit rendering / parser / references) routes
migrated properties through ``ValueSpec``; the pilot migration
table in :mod:`core.bigip.registry.pilot` ships the property-level
opt-ins (destination, monitor expressions on every pool/node/GTM
kind, SNAT mode, ltm-virtual list attachments, data-group records,
GTM region rows, cert-key-chains, ltm policy rules, firewall
rule-list bodies, firewall policy rule-lists, address-list
nesting).  Unmigrated properties keep the legacy
``BigipPropertySpec`` path unchanged.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass, field
from typing import Protocol

# ---------------------------------------------------------------------------
# Context objects
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class SourceRange:
    """Half-open byte span within the source text.

    Carried on :class:`ParsedValue` so LSP links, rename, and the
    edit planner can point at the exact bytes that produced each
    structured value.  Equivalent to the existing
    :class:`core.bigip.query.values.FieldSlot` but lives in the
    registry layer so value specs don't import the query module.
    """

    start: int
    end: int


@dataclass(frozen=True, slots=True)
class Reference:
    """One outbound reference embedded in a parsed value.

    The graph layer / LSP consume this to compute links, document
    references, rename eligibility, and partition visibility.  The
    range is the byte span inside the originating source where the
    reference token lives so editor features can navigate cleanly.
    """

    target_kind: str
    target_path: str
    range: SourceRange | None = None


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """A non-fatal note attached to a parsed value.

    ``severity`` follows the LSP convention (``"error"`` /
    ``"warning"`` / ``"info"`` / ``"hint"``).  Diagnostics never
    abort parsing; the design rule from the rearchitecture doc is
    that a malformed property should still surface in query / LSP
    views, just with a diagnostic attached.
    """

    severity: str
    message: str
    range: SourceRange | None = None


@dataclass(frozen=True, slots=True)
class ParseContext:
    """Side data threaded into ``ValueSpec.parse``.

    Most specs are pure functions of *raw* text — they don't need
    the context — but parsers for compound values that resolve
    references back into the active config do need the surrounding
    object's full-path so they can resolve relative refs.
    """

    module: str = ""
    object_type: str = ""
    object_path: str = ""
    source_uri: str = ""
    source_text: str = ""
    # Absolute byte offset of *raw* in the original source text.
    # Lets a value spec lift its internal local-offset spans onto
    # absolute SourceRange entries the edit planner uses.
    base_offset: int = 0


@dataclass(frozen=True, slots=True)
class ProjectionContext:
    """Side data for ``ValueSpec.project``.

    Carries the active root URI and the owning :class:`ObjectRef` so
    a spec can synthesise :class:`PathRef` values into the right
    config, and so reference projection can attribute resolved targets
    to a specific source.
    """

    root_uri: str = ""
    owner_kind: str = ""
    owner_path: str = ""


@dataclass(frozen=True, slots=True)
class RenderContext:
    """Side data for ``ValueSpec.render``.

    The renderer takes a structured value and turns it back into the
    textual representation tmsh / SCF expects.  ``original_text``
    lets a render preserve the user's original spelling when the
    write happens to be a no-op (or a structurally identical value
    with a different canonical form).
    """

    original_text: str = ""
    target_format: str = "scf"  # "scf" | "tmsh"


@dataclass(frozen=True, slots=True)
class ReferenceContext:
    """Side data for ``ValueSpec.references``.

    The owning object's full-path lets reference-bearing specs
    attribute their outbound edges to the right source object
    without each spec carrying its own copy.
    """

    owner_kind: str = ""
    owner_path: str = ""
    source_uri: str = ""


# ---------------------------------------------------------------------------
# ParsedValue
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class ParsedValue:
    """The result of running ``ValueSpec.parse`` on raw text.

    Preserves both the structured value (for queries and transforms)
    and the original raw text (for no-op round trips).  The range,
    when present, points at the originating bytes in the source for
    LSP and edit-planner consumption.  ``diagnostics`` is the
    non-fatal note channel — a property whose raw text fails to
    parse cleanly should still produce a ``ParsedValue`` with a
    sensible fallback ``value`` (often ``None`` or the raw string)
    plus diagnostics explaining what went wrong.
    """

    value: object
    raw: str
    range: SourceRange | None = None
    diagnostics: tuple[Diagnostic, ...] = ()


# ---------------------------------------------------------------------------
# Protocol
# ---------------------------------------------------------------------------


class ValueSpec(Protocol):
    """The contract every value spec implements.

    Each method is optional in spirit (most callers exercise one
    stage) but the protocol declares all four so static analysers
    flag missing implementations early.  Concrete bases (below)
    provide sensible defaults so a new spec only overrides the
    methods it actually changes.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue: ...

    def project(self, value: object, ctx: ProjectionContext) -> object: ...

    def render(self, value: object, ctx: RenderContext) -> str: ...

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]: ...

    @property
    def is_structured(self) -> bool:
        """True when ``project`` returns a non-string structured value.

        The projection layer uses this in place of the old
        ``FieldSpec.typed`` flag.
        """
        ...


# ---------------------------------------------------------------------------
# Base implementation with sensible defaults
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class _BaseSpec:
    """Default implementations every concrete spec inherits.

    Concrete specs only override the methods they need.  The base
    treats the value as an opaque string: parse echoes the raw text,
    project returns the value verbatim, render stringifies, and
    references yields nothing.  Subclasses override piece-by-piece.
    """

    description: str = ""

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        return ParsedValue(value=raw, raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        return value

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(  # noqa: ARG002
        self, value: object, ctx: ReferenceContext
    ) -> Iterable[Reference]:
        return ()

    @property
    def is_structured(self) -> bool:
        return False


# ---------------------------------------------------------------------------
# Concrete value specs
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class StringSpec(_BaseSpec):
    """A free-form string value.

    The default behaviour is what every untyped TMSH property uses
    today: parse echoes the raw text, project keeps it as a string,
    render stringifies.  ``StringSpec`` exists primarily so the
    registry can declare an explicit value even for plain strings —
    making "this property is a free-form string" a positive
    declaration rather than the absence of a spec.
    """


@dataclass(frozen=True, slots=True)
class IntSpec(_BaseSpec):
    """An integer-valued property.

    *min_value* / *max_value* mark the inclusive bounds.  ``None``
    means "no bound on that side"; the parser still accepts the
    value but emits a diagnostic when it lands outside the declared
    range.  Numeric coercion preserves the original text on
    ``raw`` so rendering keeps the user's spelling (``"080"`` vs
    ``"80"``) intact when the value didn't change.
    """

    min_value: int | None = None
    max_value: int | None = None
    allow_none: bool = False

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        text = raw.strip()
        if not text:
            return (
                ParsedValue(value=None, raw=raw)
                if self.allow_none
                else ParsedValue(value=0, raw=raw)
            )
        if self.allow_none and text == "none":
            return ParsedValue(value=None, raw=raw)
        try:
            n = int(text)
        except ValueError:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(Diagnostic(severity="error", message=f"not an integer: {raw!r}"),),
            )
        diags: tuple[Diagnostic, ...] = ()
        if self.min_value is not None and n < self.min_value:
            diags = (Diagnostic(severity="error", message=f"value {n} below min {self.min_value}"),)
        elif self.max_value is not None and n > self.max_value:
            diags = (Diagnostic(severity="error", message=f"value {n} above max {self.max_value}"),)
        return ParsedValue(value=n, raw=raw, diagnostics=diags)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class BoolSpec(_BaseSpec):
    """A boolean property spelled as ``enabled`` / ``disabled``
    (or ``yes`` / ``no`` / ``true`` / ``false``).

    Two spellings are common in TMSH:

    - ``enabled`` / ``disabled`` — used by feature toggles.
    - ``yes`` / ``no`` — used by attribute flags inside sub-blocks.

    ``style`` picks the preferred render spelling so a round-trip
    write doesn't switch styles on a user.  The parser accepts both
    spellings regardless.
    """

    style: str = "enabled"  # "enabled" | "yes" | "true"

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        text = raw.strip().lower()
        truthy = {"enabled", "yes", "true", "on", "1"}
        falsy = {"disabled", "no", "false", "off", "0"}
        if text in truthy:
            return ParsedValue(value=True, raw=raw)
        if text in falsy:
            return ParsedValue(value=False, raw=raw)
        return ParsedValue(
            value=None,
            raw=raw,
            diagnostics=(Diagnostic(severity="error", message=f"not a boolean: {raw!r}"),),
        )

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        styles = {
            "enabled": ("enabled", "disabled"),
            "yes": ("yes", "no"),
            "true": ("true", "false"),
        }
        on, off = styles.get(self.style, styles["enabled"])
        return on if value else off

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class EnumSpec(_BaseSpec):
    """A fixed-vocabulary string.

    Replaces the existing ``BigipPropertySpec.enum_values`` rule with
    a first-class spec.  Values outside the enumeration emit a
    diagnostic on parse but still produce a structured value (the
    raw text) so query consumers can see what was actually written.
    """

    values: frozenset[str] = frozenset()
    allow_none: bool = False

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        text = raw.strip()
        if not text and self.allow_none:
            return ParsedValue(value=None, raw=raw)
        if text in self.values:
            return ParsedValue(value=text, raw=raw)
        if self.allow_none and text == "none":
            return ParsedValue(value=None, raw=raw)
        return ParsedValue(
            value=text,
            raw=raw,
            diagnostics=(
                Diagnostic(
                    severity="warning",
                    message=(
                        f"{text!r} is not in the declared enum {{{', '.join(sorted(self.values))}}}"
                    ),
                ),
            ),
        )


@dataclass(frozen=True, slots=True)
class ObjectRefSpec(_BaseSpec):
    """A reference to another BIG-IP object.

    *kind* is the TMSH module+type pair as one string (``"ltm pool"``,
    ``"net vlan"``, etc.).  Multiple permitted kinds (a pool member's
    monitor can be any ``ltm monitor *`` profile) are encoded by
    listing each kind explicitly in ``kinds``.

    Phase 1 keeps the projection / parse behaviour identical to the
    old ``FieldSpec(ref_kind=...)`` — it surfaces a ``PathRef`` and
    yields the reference for the graph.  Phase 5 wires the spec's
    ``references()`` into the LSP and query graph layers as the
    single source of truth.
    """

    kind: str = ""
    kinds: tuple[str, ...] = ()
    allow_none: bool = False
    require_visible_from_object: bool = False

    @property
    def target_kinds(self) -> tuple[str, ...]:
        if self.kinds:
            return self.kinds
        return (self.kind,) if self.kind else ()

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        text = raw.strip()
        if not text and self.allow_none:
            return ParsedValue(value=None, raw=raw)
        if text == "none" and self.allow_none:
            return ParsedValue(value=None, raw=raw)
        # Phase 1 keeps the parsed value as the string full-path —
        # later phases lift it to a structured ObjectRef value once
        # the graph layer migrates onto value specs.
        return ParsedValue(value=text, raw=raw)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        if not value:
            return ()
        targets = self.target_kinds
        if not targets:
            return ()
        return tuple(Reference(target_kind=k, target_path=str(value)) for k in targets[:1])

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class ListSpec(_BaseSpec):
    """A list-valued property.

    *item* is the value spec describing each list element.  ``syntax``
    selects the lexical shape (per the design doc's list taxonomy):

    - ``"space-separated"`` — unbraced scalar lists.
    - ``"braced-space-separated"`` — ``{ a b c }`` of scalar refs.
    - ``"keyed-block"`` — ``{ /Common/p { ... } /Common/q { ... } }``
      with per-item metadata bodies (profiles, persist, records,
      cert-key-chain, region-members, ...).
    - ``"named-rule-block"`` — ``{ name { action accept ... } }`` for
      structured rule bodies (firewall rules, policy rules).

    *list_operators* mirrors :attr:`BigipPropertySpec.list_operators`
    so the tmsh emitter can keep consulting the spec for the
    operator without having to thread two parallel registries.

    The spec owns parse / project / render / references: callers
    hand it raw text or a :class:`BigipList` and get back the
    appropriate shape for each pipeline stage.  Per-item references
    are enumerated via the inner ``item`` spec so a list of
    :class:`FirewallRule` bodies surfaces every nested address-list
    / port-list edge through one dispatch.
    """

    item: ValueSpec | None = None
    syntax: str = "braced-space-separated"
    list_operators: frozenset[str] = frozenset(("add", "delete", "replace-all-with"))
    keyed: bool = False
    allow_empty_item_body: bool = True

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:
        """Tokenise *raw* into a :class:`BigipList` of typed items.

        Discriminates on ``syntax`` (with a fallback derived from
        ``item.is_structured`` so unmigrated pilot entries that
        leave the default keep working): keyed-block / named-rule
        forms route through :func:`_parse_keyed_block_entries` and
        feed each ``(key, body)`` pair to the inner spec; flat list
        forms route through :func:`_parse_list_block` and feed each
        token to the inner spec.

        Local byte offsets are captured into ``ListItem.range`` /
        ``key_range`` / ``body_range`` so the LSP layer can compute
        document links and rename ranges directly from the parsed
        value.  Offsets are absolute when ``ctx.base_offset`` is
        non-zero (the parser threads the property value's offset
        into the wider source).
        """
        from ..parser._helpers import (
            _parse_keyed_block_entries_with_offsets,
            _parse_list_block_with_offsets,
        )
        from ..types import BigipList, ListItem, SourceSpan

        if raw is None:
            return ParsedValue(value=None, raw=raw)
        if not raw.strip():
            return ParsedValue(
                value=BigipList(syntax=self._effective_syntax(), raw=""),
                raw=raw,
            )
        syntax = self._effective_syntax()
        base = ctx.base_offset
        items: list[ListItem] = []
        diagnostics: list[Diagnostic] = []
        operator: str | None = None
        # Operator-prefixed shape: ``{ replace-all-with { /Common/a } }``
        # / ``{ add { /Common/b } }`` — peel the leading operator off
        # and parse the inner braced body via the same flat-list
        # token walker so the items come through cleanly.  Multi-op
        # bodies (``{ add { … } delete { … } }``) keep only the
        # first operator and merge subsequent items into the same
        # list; the rendered output reflects whichever single
        # operator the caller set via ``BigipList.operator``.
        if syntax == "operator-prefixed":
            operator, inner_raw, inner_offset = _peel_operator_prefix(raw)
            if operator is not None:
                tokens = _parse_list_block_with_offsets(inner_raw)
                for tok, tok_start, tok_end in tokens:
                    parsed_item = (
                        self.item.parse(tok, ctx)
                        if self.item is not None
                        else ParsedValue(value=tok, raw=tok)
                    )
                    if parsed_item.diagnostics:
                        diagnostics.extend(parsed_item.diagnostics)
                    items.append(
                        ListItem(
                            value=parsed_item.value,
                            raw=tok,
                            range=SourceSpan(
                                base + inner_offset + tok_start,
                                base + inner_offset + tok_end,
                            ),
                        )
                    )
                value = BigipList(
                    items=tuple(items),
                    syntax=syntax,
                    operator=operator,
                    raw=raw.strip(),
                    range=SourceSpan(start=base, end=base + len(raw)) if base else None,
                )
                return ParsedValue(value=value, raw=raw, diagnostics=tuple(diagnostics))
            # No recognised operator prefix — fall through to the
            # bare-list parse so a property declared as
            # ``operator-prefixed`` still produces a usable
            # BigipList when the input happens to be the
            # bare-braced shape.
        if syntax in ("keyed-block", "named-rule-block"):
            entries = _parse_keyed_block_entries_with_offsets(raw)
            for (
                key,
                body,
                key_start,
                key_end,
                body_start,
                body_end,
                item_start,
                item_end,
            ) in entries:
                parsed_item = (
                    self.item.parse(_keyed_item_text(key, body), ctx)
                    if self.item is not None
                    else ParsedValue(value=key, raw=key)
                )
                if parsed_item.diagnostics:
                    diagnostics.extend(parsed_item.diagnostics)
                items.append(
                    ListItem(
                        value=parsed_item.value,
                        key=key,
                        body=body,
                        raw=_keyed_item_text(key, body),
                        range=SourceSpan(base + item_start, base + item_end),
                        key_range=SourceSpan(base + key_start, base + key_end),
                        body_range=SourceSpan(base + body_start, base + body_end)
                        if body_end > body_start
                        else None,
                    )
                )
        else:
            tokens = _parse_list_block_with_offsets(raw)
            for tok, tok_start, tok_end in tokens:
                parsed_item = (
                    self.item.parse(tok, ctx)
                    if self.item is not None
                    else ParsedValue(value=tok, raw=tok)
                )
                if parsed_item.diagnostics:
                    diagnostics.extend(parsed_item.diagnostics)
                items.append(
                    ListItem(
                        value=parsed_item.value,
                        raw=tok,
                        range=SourceSpan(base + tok_start, base + tok_end),
                    )
                )
        value = BigipList(
            items=tuple(items),
            syntax=syntax,
            raw=raw.strip(),
            range=SourceSpan(start=base, end=base + len(raw)) if base else None,
        )
        return ParsedValue(value=value, raw=raw, diagnostics=tuple(diagnostics))

    def project(self, value: object, ctx: ProjectionContext) -> object:
        """Project as a :class:`BigipList` so DSL queries can ask
        ``.profiles[]`` and ``.profiles[] | select(.context ==
        "clientside")``.  Legacy ``tuple`` / ``list`` values
        coming off the model are wrapped: each element parses
        through the inner item spec, so the structured fields
        (``.context``, ``.default``, ``.cert``, ...) materialise
        even when the model still stores bare path strings.
        """
        from ..types import BigipList, ListItem

        if value is None:
            return ()
        if isinstance(value, BigipList):
            return value
        if isinstance(value, (tuple, list)):
            items: list[ListItem] = []
            for v in value:
                if isinstance(v, ListItem):
                    items.append(v)
                    continue
                if self.item is not None and isinstance(v, str):
                    parsed = self.item.parse(v, ParseContext())
                    items.append(ListItem(value=parsed.value, raw=v))
                else:
                    items.append(ListItem(value=v))
            return BigipList(items=tuple(items), syntax=self._effective_syntax())
        return value

    def render(self, value: object, ctx: RenderContext) -> str:
        """Render *value* back to TMSH text.

        Accepts :class:`BigipList`, tuples/lists, or strings (the
        latter being the original raw text passed through verbatim
        as a last resort).  The output respects the configured
        ``syntax`` so keyed-block lists keep their per-item brace
        bodies and flat lists keep their bare-token shape.
        """
        from ..types import BigipList

        if value is None:
            return ""
        if isinstance(value, BigipList):
            syntax = value.syntax or self._effective_syntax()
            if syntax in ("keyed-block", "named-rule-block"):
                parts = []
                for item in value.items:
                    body = (item.body or "").strip()
                    if body:
                        parts.append(f"{item.key} {{ {body} }}")
                    else:
                        parts.append(f"{item.key} {{ }}")
                inner = " ".join(parts)
                return f"{{ {inner} }}" if inner else "{ }"
            if syntax == "operator-prefixed" and value.operator:
                # ``{ <operator> { items } }`` — the tmsh-modify edit
                # form where the verb lives inside the property's
                # braced body.
                inner_items = " ".join(self._render_item(it.value, ctx) for it in value.items)
                inner_body = f"{{ {inner_items} }}" if inner_items else "{ }"
                return f"{{ {value.operator} {inner_body} }}"
            inner = " ".join(self._render_item(it.value, ctx) for it in value.items)
            return f"{{ {inner} }}" if inner else "{ }"
        if isinstance(value, (tuple, list)):
            inner = " ".join(self._render_item(v, ctx) for v in value)
            return f"{{ {inner} }}" if inner else "{ }"
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        """Walk every item and yield its references.

        Each yielded :class:`Reference` carries a span when the
        item carries one: the key range when the reference's target
        path matches the item's key (the common keyed-block shape
        where the path IS the key); otherwise the item range as the
        fallback granularity.  Inner item specs may also populate
        ``range`` directly — those values pass through unchanged so
        compound specs with body-internal offset tracking aren't
        clobbered."""
        from ..types import BigipList

        if value is None or self.item is None:
            return ()
        if isinstance(value, BigipList):
            iterable = value.items
        elif isinstance(value, (tuple, list)):
            iterable = [_LegacyListItem(item) if not _is_listitem(item) else item for item in value]
        else:
            return ()
        out: list[Reference] = []
        for li in iterable:
            v = li.value if hasattr(li, "value") else li
            for ref in self.item.references(v, ctx) or ():
                if ref.range is None:
                    fallback = _pick_item_range(li, ref.target_path)
                    if fallback is not None:
                        ref = Reference(
                            target_kind=ref.target_kind,
                            target_path=ref.target_path,
                            range=fallback,
                        )
                out.append(ref)
        return tuple(out)

    def _effective_syntax(self) -> str:
        """Resolve a concrete syntax string for parse/render.

        Honours an explicit ``syntax`` first; otherwise infers
        keyed-block when the inner item spec is structured (the
        compound specs that carry per-item bodies)."""
        if self.syntax and self.syntax != "braced-space-separated":
            return self.syntax
        if self.item is not None and getattr(self.item, "is_structured", False):
            # ObjectRefSpec is structured but produces scalar refs,
            # not keyed-block bodies — keep the braced-space-separated
            # default for it specifically.
            if isinstance(self.item, ObjectRefSpec):
                return "braced-space-separated"
            return "keyed-block"
        return "braced-space-separated"

    def _render_item(self, value: object, ctx: RenderContext) -> str:
        if self.item is None:
            return str(value)
        return self.item.render(value, ctx) or str(value)

    @property
    def is_structured(self) -> bool:
        return True


_LIST_OPERATOR_PREFIXES: tuple[str, ...] = (
    "replace-all-with",
    "add",
    "delete",
    "modify",
    "none",
)


def _peel_operator_prefix(raw: str) -> tuple[str | None, str, int]:
    """Recognise a leading tmsh operator inside a braced list body.

    Accepts ``{ replace-all-with { /Common/a } }`` /
    ``{ add { /Common/b } }`` shapes and returns
    ``(operator, inner_braced_text, inner_offset_in_raw)`` so the
    caller can re-parse *inner_braced_text* through the regular
    list tokeniser while keeping the leading operator name on the
    returned :class:`BigipList`.  Returns ``(None, raw, 0)`` when
    no operator prefix matches — caller falls back to the bare
    flat-list parse.

    *inner_offset_in_raw* is the byte offset of the inner ``{``
    inside *raw* so per-item :class:`SourceSpan`\\ s land at the
    correct absolute positions when the spec threads
    ``base_offset`` from :class:`ParseContext`.
    """
    text = raw.strip()
    leading_skipped = len(raw) - len(raw.lstrip())
    if not text.startswith("{") or not text.endswith("}"):
        return None, raw, 0
    inner = text[1:-1].strip()
    # Skip leading whitespace inside the outer braces.
    inner_leading = (len(text) - 1) - len(text[1:].lstrip())
    for op in _LIST_OPERATOR_PREFIXES:
        if inner.startswith(op):
            after = inner[len(op) :]
            if not after or after[0] not in " \t\n\r{":
                continue
            rest = after.lstrip()
            if not rest.startswith("{"):
                continue
            inner_open_in_inner = inner.index("{", len(op))
            inner_close_in_inner = inner.rfind("}")
            if inner_close_in_inner < inner_open_in_inner:
                continue
            inner_body = inner[inner_open_in_inner : inner_close_in_inner + 1]
            inner_offset = leading_skipped + 1 + inner_leading + inner_open_in_inner
            return op, inner_body, inner_offset
    return None, raw, 0


def _keyed_item_text(key: str, body: str) -> str:
    """Re-glue a ``(key, body)`` pair into the per-item source the
    inner spec's ``parse`` consumes (``"<key> { <body> }"``)."""
    body = (body or "").strip()
    if not body:
        return f"{key} {{ }}"
    return f"{key} {{ {body} }}"


class _LegacyListItem:
    """Lightweight stand-in so ``ListSpec.references`` can walk a
    plain tuple of typed values via the same code path it uses for
    :class:`ListItem` records.  Used during the migration window
    while the model still stores raw tuples for some properties."""

    __slots__ = ("value", "range")

    def __init__(self, value: object) -> None:
        self.value = value
        self.range = None


def _is_listitem(value: object) -> bool:
    from ..types import ListItem

    return isinstance(value, ListItem)


def _pick_item_range(item: object, target_path: str) -> SourceRange | None:
    """Pick the most specific :class:`SourceRange` for *target_path*
    against a :class:`ListItem` (or legacy adapter).

    When the item's ``key`` is the same path as the reference's
    target (the common ``profiles { /Common/clientssl { ... } }``
    shape), use the narrower ``key_range``.  For body-internal
    references (cert-key-chain ``cert`` / firewall rule
    ``address-lists`` / etc.) we fall back to the broader item
    range.  Returns ``None`` when no span info is available.
    """
    key = getattr(item, "key", "")
    key_range = getattr(item, "key_range", None)
    item_range = getattr(item, "range", None)
    if key and key == target_path and key_range is not None:
        return SourceRange(start=key_range.start, end=key_range.end)
    if item_range is not None:
        return SourceRange(start=item_range.start, end=item_range.end)
    return None


# ---------------------------------------------------------------------------
# Domain-specific specs (Phase 1 covers the foundations only)
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class DestinationSpec(_BaseSpec):
    """An ``ltm virtual`` / ``ltm pool member`` destination value.

    Mirrors the broad :class:`core.bigip.types.Destination` class but
    declares per-property constraints (which the broad class can't
    know about) so a virtual-server destination's grammar can differ
    from a pool-member destination's grammar while both share one
    parser implementation.

    Phase 1 wires through ``Destination.try_parse`` and exposes the
    spec; Phase 4 makes the parser use this directly.
    """

    address_families: frozenset[str] = frozenset(("ipv4", "ipv6", "fqdn"))
    require_port: bool = False
    allow_route_domain: bool = True
    allow_partition: bool = True
    allow_folder: bool = True
    allow_wildcard: bool = True
    allow_service_name_port: bool = False
    port_separator: str = "preserve"  # "preserve" | "colon" | "dot"

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import Destination

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = Destination.try_parse(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(Diagnostic(severity="error", message=f"not a destination: {raw!r}"),),
            )
        return ParsedValue(value=parsed, raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        # Legacy parity: typed values flow into the DSL as their
        # canonical string spelling so every existing query that
        # compares ``.destination`` against a literal keeps working.
        # Phase 6's compound-spec work will introduce a structured
        # container form (``.destination.host``, ``.destination.port``)
        # alongside the string surface; until then we preserve the
        # string projection the legacy ``typed=True`` branch returned.
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class NetworkSpec(_BaseSpec):
    """A CIDR or dotted-quad mask value (``net route.network``,
    ``net self.address``, ...)."""

    allow_default_keyword: bool = True
    allow_dotted_quad_mask: bool = True
    preserve_host_bits: bool = True

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import Network

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = Network.try_parse(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(Diagnostic(severity="error", message=f"not a network: {raw!r}"),),
            )
        return ParsedValue(value=parsed, raw=raw)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class AddressSpec(_BaseSpec):
    """An IP address or FQDN value (``ltm node.address`` /
    ``ltm pool member.address``)."""

    allow_ip: bool = True
    allow_fqdn: bool = True
    allow_cidr: bool = False
    allow_range: bool = False

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import try_parse_address

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = try_parse_address(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(Diagnostic(severity="error", message=f"not an address: {raw!r}"),),
            )
        return ParsedValue(value=parsed, raw=raw)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class PortSpec(_BaseSpec):
    """A single port or port range (``net port-list.ports`` items,
    ``security firewall port-list.ports``, ...)."""

    allow_range: bool = True
    allow_service_name: bool = True
    allow_any: bool = True

    @property
    def is_structured(self) -> bool:
        return True


# ---------------------------------------------------------------------------
# Phase 6: compound value specs
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class MonitorExpressionSpec(_BaseSpec):
    """A pool / node / GTM-server ``monitor`` expression.

    Wraps the :class:`core.bigip.types.MonitorExpression` typed value
    in a value spec so the registry can route monitor parsing,
    projection, edit rendering, and reference enumeration through
    one consistent surface.  ``ref_kind`` (or the wider ``ref_kinds``
    set, for properties that accept any of many monitor types) gives
    the LSP / graph layers the target kind so navigation, document
    links, and rename eligibility all see the same edges as the
    parsed expression's :meth:`MonitorExpression.references`.

    Phase 6 wires this spec; ``ltm pool.monitor`` and friends migrate
    to it as the rest of the rearchitecture catches up.
    """

    ref_kind: str = "ltm monitor"
    ref_kinds: tuple[str, ...] = ()

    @property
    def target_kinds(self) -> tuple[str, ...]:
        if self.ref_kinds:
            return self.ref_kinds
        return (self.ref_kind,) if self.ref_kind else ()

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:
        from dataclasses import replace as _dc_replace

        from ..types import MonitorExpression

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = MonitorExpression.try_parse(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(
                    Diagnostic(
                        severity="error",
                        message=f"not a valid monitor expression: {raw!r}",
                    ),
                ),
            )
        # Lift the per-monitor spans onto absolute offsets.  Local
        # offsets are relative to ``text`` (the stripped raw text);
        # ``ctx.base_offset`` lives at the START of *raw* (which
        # may have leading whitespace), so adjust by the leading
        # stripped length so callers get exact byte spans inside the
        # original source.
        if parsed.monitor_spans and (ctx.base_offset or raw != text):
            lead = len(raw) - len(raw.lstrip())
            shift = ctx.base_offset + lead
            parsed = _dc_replace(
                parsed,
                monitor_spans=tuple(
                    (start + shift, end + shift) for start, end in parsed.monitor_spans
                ),
            )
        diagnostics: list[Diagnostic] = []
        if parsed.mode == "min-of" and parsed.minimum is not None:
            total = len(parsed.monitors)
            if parsed.minimum < 1:
                diagnostics.append(
                    Diagnostic(
                        severity="error",
                        message=(
                            f"monitor ``min {parsed.minimum} of`` threshold "
                            "must be at least 1; BIG-IP rejects 0-of "
                            "expressions at validation"
                        ),
                    )
                )
            elif total > 0 and parsed.minimum > total:
                diagnostics.append(
                    Diagnostic(
                        severity="error",
                        message=(
                            f"monitor ``min {parsed.minimum} of`` threshold "
                            f"exceeds the {total} listed monitor"
                            f"{'' if total == 1 else 's'} — BIG-IP would "
                            "never satisfy the rule"
                        ),
                    )
                )
        return ParsedValue(value=parsed, raw=raw, diagnostics=tuple(diagnostics))

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        # Project as the structured :class:`MonitorExpression` so DSL
        # queries can ask ``.monitor.mode`` / ``.monitor.monitors[]``
        # / ``.monitor.minimum`` directly.  The typed value carries
        # ``.full_path`` / ``.name`` properties so the historical
        # ``.monitor.full-path`` queries against a single-monitor
        # expression keep returning the path (PathRef back-compat).
        from ..types import MonitorExpression

        if value is None:
            return ""
        if isinstance(value, MonitorExpression):
            return value
        if isinstance(value, str):
            if not value.strip():
                return ""
            parsed = MonitorExpression.try_parse(value)
            return parsed if parsed is not None else value
        return value

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        # The monitor expression already enumerates its own
        # references; the spec attributes each to the first declared
        # target kind so unresolved monitor refs at least land on a
        # single kind for graph traversal.  Per-monitor source spans
        # come from :class:`MonitorExpression.monitor_spans` (local
        # offsets in ``raw``); the ListSpec / call site adds the
        # absolute base offset when threading ``Reference.range``.
        if value is None:
            return ()
        from ..types import MonitorExpression

        if not isinstance(value, MonitorExpression):
            return ()
        targets = self.target_kinds
        if not targets:
            return ()
        kind = targets[0]
        out: list[Reference] = []
        spans = value.monitor_spans or ()
        for idx, path in enumerate(value.monitors):
            rng = None
            if idx < len(spans):
                start, end = spans[idx]
                rng = SourceRange(start=start, end=end)
            out.append(Reference(target_kind=kind, target_path=path, range=rng))
        return tuple(out)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class ProfileAttachmentSpec(_BaseSpec):
    """One ``ltm virtual.profiles[]`` attachment item.

    Profiles attach to a virtual server as a keyed-block list:

        profiles {
            /Common/clientssl { context clientside }
            /Common/serverssl { context serverside }
            /Common/http { }
        }

    The spec parses one attachment at a time — the surrounding list
    is handled by the keyed-block parser; this spec only sees the
    key (full-path) + body for each item.  ``ref_kind`` is the
    target object kind for the graph layer (``"ltm profile"`` by
    default; per-profile-type specs can narrow it).
    """

    ref_kind: str = "ltm profile"

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import ProfileAttachment

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        # *raw* arrives as ``<path> { body }`` from the keyed-block
        # parser.  Split on the opening brace to extract the
        # reference and the body separately.
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=ProfileAttachment(path=text), raw=raw)
        path = text[:brace].strip()
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=ProfileAttachment.from_raw(path=path, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        from ..types import ProfileAttachment

        if value is None or not isinstance(value, ProfileAttachment):
            return ""
        # Phase 6 keeps the legacy string projection for backwards
        # compatibility; structured ``.context`` / ``.path`` /
        # ``.name`` accessors come in when the projection layer
        # exposes attachment containers.
        return value.path

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        from ..types import ProfileAttachment

        if value is None or not isinstance(value, ProfileAttachment):
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import ProfileAttachment

        if value is None or not isinstance(value, ProfileAttachment) or not value.path:
            return ()
        return (Reference(target_kind=self.ref_kind, target_path=value.path),)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class PersistenceAttachmentSpec(_BaseSpec):
    """One ``ltm virtual.persist[]`` attachment item.

    Persistence attachments share the keyed-block shape with
    profiles but carry a different optional flag (``default yes``
    instead of ``context``).  Modelled separately so the structured
    surface (``.default``) is clear and downstream queries don't
    have to look at a wrapper field.
    """

    ref_kind: str = "ltm persistence"

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import PersistenceAttachment

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=PersistenceAttachment(path=text), raw=raw)
        path = text[:brace].strip()
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=PersistenceAttachment.from_raw(path=path, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        from ..types import PersistenceAttachment

        if value is None or not isinstance(value, PersistenceAttachment):
            return ""
        return value.path

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        from ..types import PersistenceAttachment

        if value is None or not isinstance(value, PersistenceAttachment):
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import PersistenceAttachment

        if value is None or not isinstance(value, PersistenceAttachment) or not value.path:
            return ()
        return (Reference(target_kind=self.ref_kind, target_path=value.path),)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class SnatModeSpec(_BaseSpec):
    """An ltm virtual's ``source-address-translation`` value.

    Sum type covering ``none`` / ``automap`` / ``snat <pool>`` so
    queries can filter virtuals by SNAT mode (``select(.snat_mode.is_automap)``
    or ``select(.snat_mode.kind == "snat")``) and the rewrite layer
    can swap modes safely.

    The graph reference layer surfaces the ``snat pool`` reference
    when the mode is ``snat`` so ``references_to /Common/snatpool_x``
    finds every virtual that uses it.
    """

    ref_kind: str = "ltm snatpool"

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import SnatMode

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = SnatMode.try_parse(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(Diagnostic(severity="error", message=f"not a SNAT mode: {raw!r}"),),
            )
        return ParsedValue(value=parsed, raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import SnatMode

        if value is None or not isinstance(value, SnatMode):
            return ()
        return tuple(
            Reference(target_kind=self.ref_kind, target_path=path) for path in value.references()
        )

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class DataGroupRecordSpec(_BaseSpec):
    """One record inside a data-group's records list.

    Records are keyed by their lookup name (a string, CIDR, or
    integer depending on the data-group's ``type``).  The spec
    parses the key + body shape; the broader records container is
    handled by the keyed-block list parser.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import DataGroupRecord

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=DataGroupRecord(key=text), raw=raw)
        key = text[:brace].strip()
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=DataGroupRecord.from_raw(key=key, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        from ..types import DataGroupRecord

        if value is None or not isinstance(value, DataGroupRecord):
            return ""
        return value.key

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        from ..types import DataGroupRecord

        if value is None or not isinstance(value, DataGroupRecord):
            return ""
        return str(value)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class GtmRegionMemberSpec(_BaseSpec):
    """One row inside a GTM region's ``region-members`` list.

    Surfaces the parsed (kind, value, negated) triple so topology
    queries can filter by clause type and negation without re-
    parsing the row text.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import GtmRegionMember

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        parsed = GtmRegionMember.try_parse(text)
        if parsed is None:
            return ParsedValue(
                value=None,
                raw=raw,
                diagnostics=(
                    Diagnostic(
                        severity="error",
                        message=f"not a valid GTM region member: {raw!r}",
                    ),
                ),
            )
        return ParsedValue(value=parsed, raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class CertKeyChainSpec(_BaseSpec):
    """One ``cert-key-chain`` entry on a client/server SSL profile.

    Each entry carries up to three reference sub-fields (cert, key,
    chain) plus an optional passphrase.  The spec exposes all three
    references through ``ValueSpec.references()`` so the graph
    layer sees every SSL cert / key / CA bundle the profile
    depends on.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import CertKeyChain

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=CertKeyChain(name=text), raw=raw)
        name = text[:brace].strip()
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=CertKeyChain.from_raw(name=name, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        from ..types import CertKeyChain

        if value is None or not isinstance(value, CertKeyChain):
            return ""
        return value.name

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        from ..types import CertKeyChain

        if value is None or not isinstance(value, CertKeyChain):
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import CertKeyChain

        if value is None or not isinstance(value, CertKeyChain):
            return ()
        return tuple(
            Reference(target_kind=kind, target_path=path) for kind, path in value.references()
        )

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class LtmPolicyConditionSpec(_BaseSpec):
    """One clause inside a policy rule's ``conditions { ... }`` block.

    Phase 6 lifts the legacy "raw token tuple" representation into a
    structured value: queries can now ask things like

        .ltm.policy[].rules[].conditions[]
          | select(.operand == "http-host" and not .negate)
          | .values

    and the spec exposes a single structured value the rewrite
    layer can update safely (changing one operator, adding one
    value) without re-tokenising the whole body.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import LtmPolicyCondition

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        # ``raw`` arrives as ``<index> { body }`` from the keyed-
        # block parser; split on the brace to pull each side.
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=LtmPolicyCondition(), raw=raw)
        try:
            index = int(text[:brace].strip())
        except ValueError:
            index = 0
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=LtmPolicyCondition.from_raw(index=index, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class LtmPolicyActionSpec(_BaseSpec):
    """One clause inside a policy rule's ``actions { ... }`` block.

    The action's structured form exposes verb / target / select /
    parameter slots so queries can filter policies by what they
    actually do.  References (``forward select pool <path>``)
    surface through :meth:`ValueSpec.references` so the graph
    layer finds every policy that forwards to a given pool.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import LtmPolicyAction

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=LtmPolicyAction(), raw=raw)
        try:
            index = int(text[:brace].strip())
        except ValueError:
            index = 0
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=LtmPolicyAction.from_raw(index=index, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import LtmPolicyAction

        if value is None:
            return ()
        # Accept both the new ``LtmPolicyAction`` typed value and the
        # legacy ``BigipPolicyAction`` model dataclass — they overlap
        # heavily on the reference-bearing fields (``verb`` /
        # ``pool``), and bridging both lets the registry migration
        # consume the legacy model without rewriting it.  A policy
        # action that forwards to a pool yields the same edge in
        # either shape.
        if isinstance(value, LtmPolicyAction):
            edges = value.references()
        else:
            edges = _references_from_legacy_policy_action(value)
        return tuple(Reference(target_kind=kind, target_path=path) for kind, path in edges)


def _references_from_legacy_policy_action(value: object) -> tuple[tuple[str, str], ...]:
    """Adapt the legacy ``BigipPolicyAction`` dataclass to the
    reference-yielding contract.

    The legacy class lives in :mod:`core.bigip.model._ltm` and
    predates :class:`LtmPolicyAction`; field names match closely
    enough that we can read the same edges without bridging the
    dataclasses themselves.  Duck-typing on ``verb`` and ``pool``
    keeps this safe against unrelated objects (anything without
    both attributes yields nothing).
    """
    verb = getattr(value, "verb", "")
    pool = getattr(value, "pool", "")
    if verb == "forward" and pool:
        return (("ltm pool", pool),)
    return ()


@dataclass(frozen=True, slots=True)
class LtmPolicyRuleSpec(_BaseSpec):
    """One rule inside an ltm policy's ``rules { ... }`` block.

    The rule's value is itself a compound: ordered ``conditions``
    plus ordered ``actions``, each with its own structured shape.
    The spec walks both lists and yields the union of every
    reference each item carries — today that's pool refs from
    ``forward select pool <path>`` actions, with room for the
    rest of the action vocabulary as future migrations land
    (``insert ... profile X``, ``rewrite uri /Common/policy_X``,
    etc.).

    Accepts both the new typed value shape (a ``BigipPolicyRule``
    from the model, which carries tuples of ``BigipPolicyAction``
    /``BigipPolicyCondition``) and any duck-compatible object
    exposing ``actions`` / ``conditions`` attributes — so a
    callsite passing one rule's structured projection still flows
    through the same path.
    """

    action_spec: LtmPolicyActionSpec = field(default_factory=lambda: LtmPolicyActionSpec())
    condition_spec: LtmPolicyConditionSpec = field(default_factory=lambda: LtmPolicyConditionSpec())

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        if value is None:
            return ()
        out: list[Reference] = []
        for action in getattr(value, "actions", ()) or ():
            out.extend(self.action_spec.references(action, ctx))
        for condition in getattr(value, "conditions", ()) or ():
            out.extend(self.condition_spec.references(condition, ctx))
        return tuple(out)

    @property
    def is_structured(self) -> bool:
        return True


@dataclass(frozen=True, slots=True)
class FirewallRuleSpec(_BaseSpec):
    """One rule inside a security firewall rule-list / policy.

    The structured value exposes action / ip-protocol / source +
    destination endpoints / log / rule-list reference so safety
    queries can ask things like "which rules are any/any/accept?"
    or "what rules reference port-list X?" without rebuilding the
    body parser each time.  References surface every port-list /
    address-list / nested rule-list edge for the graph layer.
    """

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
        from ..types import FirewallRule

        text = raw.strip()
        if not text:
            return ParsedValue(value=None, raw=raw)
        brace = text.find("{")
        if brace < 0:
            return ParsedValue(value=FirewallRule(name=text), raw=raw)
        name = text[:brace].strip()
        body = text[brace + 1 :]
        if body.rstrip().endswith("}"):
            body = body.rsplit("}", 1)[0]
        return ParsedValue(value=FirewallRule.from_raw(name=name, body=body), raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        from ..types import FirewallRule

        if value is None or not isinstance(value, FirewallRule):
            return ""
        return value.name

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        from ..types import FirewallRule

        if value is None or not isinstance(value, FirewallRule):
            return ()
        return tuple(
            Reference(target_kind=kind, target_path=path) for kind, path in value.references()
        )

    @property
    def is_structured(self) -> bool:
        return True


# NAT rules reuse the firewall rule shape entirely; the spec is a
# thin alias so the registry's dispatch keeps them at the same
# layer even though the SCF stanza header differs.  Dedicated
# kind-specific behaviour can branch off here as it surfaces.
NatRuleSpec = FirewallRuleSpec


# Public re-exports.
__all__ = [
    "AddressSpec",
    "BoolSpec",
    "CertKeyChainSpec",
    "DataGroupRecordSpec",
    "DestinationSpec",
    "Diagnostic",
    "EnumSpec",
    "FirewallRuleSpec",
    "GtmRegionMemberSpec",
    "IntSpec",
    "ListSpec",
    "LtmPolicyActionSpec",
    "LtmPolicyConditionSpec",
    "LtmPolicyRuleSpec",
    "MonitorExpressionSpec",
    "NatRuleSpec",
    "NetworkSpec",
    "ObjectRefSpec",
    "ParseContext",
    "ParsedValue",
    "PersistenceAttachmentSpec",
    "PortSpec",
    "ProfileAttachmentSpec",
    "ProjectionContext",
    "Reference",
    "ReferenceContext",
    "RenderContext",
    "SnatModeSpec",
    "SourceRange",
    "StringSpec",
    "ValueSpec",
]
