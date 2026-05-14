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
``ProfileAttachmentSpec``, ``SnatModeSpec``, ...) come in later phases
once the foundation is in place.

This file is **introduced without behaviour change** in Phase 1.  The
existing parser / projection / edit-plan paths continue to read the
older ``BigipPropertySpec`` data; Phase 2 onward starts routing each
stage through ``ValueSpec`` for the properties that have migrated.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
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
    selects the lexical shape (per the design doc's list taxonomy);
    Phase 1 only honours ``SPACE_SEPARATED`` / ``BRACED_SPACE_SEPARATED``
    / ``KEYED_BLOCK`` since those cover every list in the existing
    renderer, and adds the others in Phase 6.

    *list_operators* mirrors :attr:`BigipPropertySpec.list_operators`
    so the tmsh emitter can keep consulting the spec for the
    operator without having to thread two parallel registries.
    """

    item: ValueSpec | None = None
    syntax: str = "braced-space-separated"
    list_operators: frozenset[str] = frozenset(("add", "delete", "replace-all-with"))
    keyed: bool = False
    allow_empty_item_body: bool = True

    @property
    def is_structured(self) -> bool:
        return True


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

    def parse(self, raw: str, ctx: ParseContext) -> ParsedValue:  # noqa: ARG002
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
        return ParsedValue(value=parsed, raw=raw)

    def project(self, value: object, ctx: ProjectionContext) -> object:  # noqa: ARG002
        # Legacy parity: the existing projection layer surfaces the
        # monitor as its canonical string.  Phase 6+ can layer
        # ``.monitor.mode`` / ``.monitor.monitors[]`` /
        # ``.monitor.minimum`` structured children on top later; until
        # then we keep the string surface so existing queries against
        # ``.monitor`` still work.
        if value is None:
            return ""
        return str(value)

    def render(self, value: object, ctx: RenderContext) -> str:  # noqa: ARG002
        if value is None:
            return ""
        return str(value)

    def references(self, value: object, ctx: ReferenceContext) -> Iterable[Reference]:
        # The monitor expression already enumerates its own
        # references; the spec attributes each to the first
        # declared target kind so unresolved monitor refs at least
        # land on a single kind for graph traversal.  Phase 6 can
        # widen this (a ``ltm monitor http`` reference is also
        # findable as ``ltm monitor``, etc.) when the kind-resolver
        # API is in place.
        if value is None:
            return ()
        from ..types import MonitorExpression

        if not isinstance(value, MonitorExpression):
            return ()
        targets = self.target_kinds
        if not targets:
            return ()
        kind = targets[0]
        return tuple(Reference(target_kind=kind, target_path=path) for path in value.references())

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

        if value is None or not isinstance(value, LtmPolicyAction):
            return ()
        return tuple(
            Reference(target_kind=kind, target_path=path) for kind, path in value.references()
        )

    @property
    def is_structured(self) -> bool:
        return True


# Public re-exports.
__all__ = [
    "AddressSpec",
    "BoolSpec",
    "CertKeyChainSpec",
    "DataGroupRecordSpec",
    "DestinationSpec",
    "Diagnostic",
    "EnumSpec",
    "GtmRegionMemberSpec",
    "IntSpec",
    "ListSpec",
    "LtmPolicyActionSpec",
    "LtmPolicyConditionSpec",
    "MonitorExpressionSpec",
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
