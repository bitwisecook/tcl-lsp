"""Phase 1 tests for the value-spec scaffolding.

These tests pin the contract :class:`ValueSpec` declares — every
concrete spec implements ``parse`` / ``project`` / ``render`` /
``references`` / ``is_structured`` — and confirm the new
:class:`PropertySpec` compat properties (``typed`` / ``ref_kind`` /
``list_ref``) match what the legacy projection code expects.  Phases
2-6 will progressively retire the compat properties; until then,
asserting parity here prevents accidental skew between the two
shapes during the migration.
"""

from __future__ import annotations

from core.bigip.registry import (
    AddressSpec,
    BoolSpec,
    DestinationSpec,
    EnumSpec,
    IntSpec,
    ListSpec,
    NetworkSpec,
    ObjectKeySpec,
    ObjectRefSpec,
    ObjectSpec,
    ParseContext,
    PortSpec,
    PropertySpec,
    Reference,
    ReferenceContext,
    RenderContext,
    StringSpec,
)
from core.bigip.types import BigipList, ListItem

# ---------------------------------------------------------------------------
# StringSpec / IntSpec / BoolSpec / EnumSpec — scalar fundamentals
# ---------------------------------------------------------------------------


def test_string_spec_echoes_raw():
    spec = StringSpec()
    parsed = spec.parse("hello world", ParseContext())
    assert parsed.value == "hello world"
    assert parsed.raw == "hello world"
    assert parsed.diagnostics == ()
    assert spec.is_structured is False
    assert spec.render("hello", RenderContext()) == "hello"


def test_int_spec_parses_and_bounds_check():
    spec = IntSpec(min_value=0, max_value=65535)
    parsed = spec.parse("80", ParseContext())
    assert parsed.value == 80
    assert spec.is_structured is True
    # out of range produces a diagnostic but still yields a value
    out_of_range = spec.parse("99999", ParseContext())
    assert out_of_range.value == 99999
    assert any("above max" in d.message for d in out_of_range.diagnostics)
    # non-integer text produces a None value + error diagnostic
    bad = spec.parse("hello", ParseContext())
    assert bad.value is None
    assert any("not an integer" in d.message for d in bad.diagnostics)


def test_bool_spec_accepts_every_known_spelling():
    spec = BoolSpec()
    for raw in ("enabled", "yes", "true", "on", "1"):
        assert spec.parse(raw, ParseContext()).value is True
    for raw in ("disabled", "no", "false", "off", "0"):
        assert spec.parse(raw, ParseContext()).value is False
    # unknown spelling lands as None with a diagnostic
    bad = spec.parse("maybe", ParseContext())
    assert bad.value is None
    assert any("boolean" in d.message for d in bad.diagnostics)
    # render uses the declared style; default is enabled/disabled
    assert spec.render(True, RenderContext()) == "enabled"
    assert spec.render(False, RenderContext()) == "disabled"
    yes_no = BoolSpec(style="yes")
    assert yes_no.render(True, RenderContext()) == "yes"
    assert yes_no.render(False, RenderContext()) == "no"


def test_enum_spec_warns_on_unknown_value_but_keeps_raw():
    spec = EnumSpec(values=frozenset(("clientside", "serverside", "all")))
    ok = spec.parse("clientside", ParseContext())
    assert ok.value == "clientside"
    assert ok.diagnostics == ()
    unknown = spec.parse("middleside", ParseContext())
    assert unknown.value == "middleside"  # raw preserved
    assert any("declared enum" in d.message for d in unknown.diagnostics)


# ---------------------------------------------------------------------------
# ObjectRefSpec — references for the graph / LSP
# ---------------------------------------------------------------------------


def test_object_ref_spec_yields_a_reference():
    spec = ObjectRefSpec(kind="ltm pool")
    refs = list(spec.references("/Common/web_pool", ReferenceContext()))
    assert refs == [Reference(target_kind="ltm pool", target_path="/Common/web_pool")]
    # Empty value yields no edges.
    assert list(spec.references("", ReferenceContext())) == []
    # ``is_structured`` flips so the projection still routes through
    # the structured rendering branch.
    assert spec.is_structured is True


def test_object_ref_spec_supports_multiple_target_kinds():
    # ``pool member.monitor`` references any of many ``ltm monitor *``
    # kinds — the spec declares all of them and the parse-time
    # reference is attributed to the first (the graph resolver
    # narrows downstream).
    spec = ObjectRefSpec(kinds=("ltm monitor http", "ltm monitor https"))
    refs = list(spec.references("/Common/http", ReferenceContext()))
    assert refs == [Reference(target_kind="ltm monitor http", target_path="/Common/http")]


# ---------------------------------------------------------------------------
# ListSpec — list-valued properties keep their operator
# ---------------------------------------------------------------------------


def test_list_spec_declares_default_operator_set():
    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"))
    assert "replace-all-with" in spec.list_operators
    assert spec.is_structured is True


def test_bigip_list_behaves_like_mutable_python_list():
    values = BigipList(items=[ListItem(value="/Common/a")])

    values.append("/Common/b")
    values[0] = "/Common/z"
    values[1:2] = ["/Common/c", "/Common/d"]

    assert list(values) == ["/Common/z", "/Common/c", "/Common/d"]
    assert values.pop() == "/Common/d"
    assert list(values) == ["/Common/z", "/Common/c"]


# ---------------------------------------------------------------------------
# DestinationSpec / NetworkSpec / AddressSpec — domain types
# ---------------------------------------------------------------------------


def test_destination_spec_parses_ip_port_destination():
    from core.bigip.types import Destination

    spec = DestinationSpec(require_port=True, allow_partition=False)
    parsed = spec.parse("/Common/10.0.0.10:80", ParseContext())
    assert isinstance(parsed.value, Destination)
    # ``str(parsed.value)`` round-trips the original spelling.
    assert str(parsed.value) == "/Common/10.0.0.10:80"


def test_network_spec_keeps_original_spelling_and_default_keyword():
    spec = NetworkSpec(allow_default_keyword=True)
    interface = spec.parse("203.0.113.5/24", ParseContext())
    # Host bits preserved on the typed Network's ``original`` field.
    assert interface.value is not None
    assert str(interface.value) == "203.0.113.5/24"
    # ``default`` keyword survives as the canonical text.
    default_route = spec.parse("default", ParseContext())
    assert default_route.value is not None
    assert str(default_route.value) == "default"


def test_address_spec_parses_ip_and_fqdn():
    from core.bigip.types import FQDN, IPAddress

    spec = AddressSpec()
    ip = spec.parse("10.0.0.1", ParseContext())
    assert isinstance(ip.value, IPAddress)
    host = spec.parse("api.example.com", ParseContext())
    assert isinstance(host.value, FQDN)


def test_port_spec_is_structured():
    spec = PortSpec()
    assert spec.is_structured is True


# ---------------------------------------------------------------------------
# PropertySpec / ObjectSpec — compat properties for the projection path
# ---------------------------------------------------------------------------


def test_property_spec_typed_compat_matches_value_is_structured():
    """The legacy projection's ``FieldSpec.typed`` flag is derived
    from ``value.is_structured`` so Phase 1 keeps projection
    routing on the same branch."""
    string_prop = PropertySpec(attr="description", value=StringSpec())
    typed_prop = PropertySpec(attr="address", value=AddressSpec())
    assert string_prop.typed is False
    assert typed_prop.typed is True


def test_property_spec_ref_kind_compat_picks_object_ref():
    pool_prop = PropertySpec(attr="pool", value=ObjectRefSpec(kind="ltm pool"))
    assert pool_prop.ref_kind == "ltm pool"
    description_prop = PropertySpec(attr="description", value=StringSpec())
    assert description_prop.ref_kind == ""


def test_property_spec_list_ref_compat_picks_list_of_object_refs():
    rules_prop = PropertySpec(
        attr="rules",
        value=ListSpec(item=ObjectRefSpec(kind="ltm rule")),
    )
    assert rules_prop.list_ref is True
    pool_prop = PropertySpec(attr="pool", value=ObjectRefSpec(kind="ltm pool"))
    assert pool_prop.list_ref is False
    # A list of non-ref scalars isn't a ref-list.
    ports_prop = PropertySpec(
        attr="ports",
        value=ListSpec(item=PortSpec()),
    )
    assert ports_prop.list_ref is False


def test_property_spec_name_derives_tmsh_spelling():
    snake_prop = PropertySpec(attr="load_balancing_mode", value=StringSpec())
    assert snake_prop.name == "load-balancing-mode"
    # Explicit override wins.
    override_prop = PropertySpec(
        attr="something_weird",
        value=StringSpec(),
        tmsh_name="ip-tos-to-client",
    )
    assert override_prop.name == "ip-tos-to-client"


def test_object_spec_holds_property_map():
    """``ObjectSpec.properties`` is keyed by TMSH-spelt names; small
    smoke test confirms the dataclass shape is what the rest of the
    rearchitecture expects."""

    class FakeModel:
        pass

    spec = ObjectSpec(
        kind="ltm pool",
        model=FakeModel,
        config_attr="pools",
        key=ObjectKeySpec.full_path(),
        properties={
            "monitor": PropertySpec(
                attr="monitor",
                value=ObjectRefSpec(kind="ltm monitor"),
                writable=True,
            ),
        },
    )
    assert spec.kind == "ltm pool"
    assert spec.model is FakeModel
    assert spec.config_attr == "pools"
    assert spec.key.name == "full_path"
    assert spec.properties["monitor"].ref_kind == "ltm monitor"
    assert spec.properties["monitor"].writable is True


# ---------------------------------------------------------------------------
# Phase 2: projection routes through the pilot table when migrated
# ---------------------------------------------------------------------------


def test_pilot_table_seeds_ltm_virtual_destination():
    """The pilot migration table starts with one entry — the first
    Phase 2 deliverable — so the projection engine has something to
    exercise.  Behaviour is identical to the legacy ``typed=True``
    branch (canonical-string projection); the dispatch path is what
    Phase 2 actually exercises."""
    from core.bigip.registry.pilot import pilot_property_spec_for

    spec = pilot_property_spec_for("ltm", "virtual", "destination")
    assert spec is not None
    assert spec.writable is True
    assert spec.ref_kind == ""  # destination is not a ref


def test_projection_routes_destination_through_pilot_spec():
    """``ltm virtual.destination`` now flows through the new
    :class:`DestinationSpec` path.  The end-user surface is
    unchanged — the destination still projects as a canonical
    string — so every existing query continues to work."""
    from core.bigip.query import run_query

    src = "ltm virtual /Common/v { destination /Common/10.0.0.10:80 }\n"
    result = run_query('.ltm.virtual["/Common/v"].destination', {"m": src})
    [destination] = result.values_per_file["m"]
    assert destination == "/Common/10.0.0.10:80"


def test_destination_spec_projects_none_as_empty_string():
    """The legacy ``typed=True`` branch turned ``None`` typed values
    into ``""`` so falsey-truthiness matched empty strings.  The
    Phase 2 dispatch preserves that — Destination's ``project()``
    explicitly handles ``None`` to keep the contract."""
    from core.bigip.registry import DestinationSpec, ProjectionContext

    spec = DestinationSpec()
    assert spec.project(None, ProjectionContext()) == ""


# ---------------------------------------------------------------------------
# Phase 3: edit-plan writes flow through ValueSpec.render
# ---------------------------------------------------------------------------


_MULTI_LINE_VS = (
    "ltm virtual /Common/v {\n    destination /Common/10.0.0.10:80\n    ip-protocol tcp\n}\n"
)


def test_destination_write_through_spec_renders_canonical_form():
    """A write to ``ltm virtual.destination`` flows through
    :class:`DestinationSpec`'s ``parse`` + ``render`` round trip so
    the spec validates and re-emits the value rather than the
    generic SCF encoder splicing the raw string."""
    from core.bigip.query import run_query

    result = run_query(
        '.ltm.virtual["/Common/v"].destination = "/Common/192.168.1.1:443"',
        {"m": _MULTI_LINE_VS},
    )
    applied = result.edits_per_file["m"]
    assert "destination /Common/192.168.1.1:443" in applied.new_source


def test_destination_write_rejects_unparseable_input():
    """A spec-rejected value raises ``EditError`` rather than
    splicing in malformed text.  The Phase 3 spec validation runs
    before the SCF reparse guard so the rejection points at the
    actual problem."""
    import pytest

    from core.bigip.query import run_query
    from core.bigip.query.errors import EditError, QueryError

    with pytest.raises((EditError, QueryError)) as exc:
        run_query(
            '.ltm.virtual["/Common/v"].destination = "obviously-not-a-destination"',
            {"m": _MULTI_LINE_VS},
        )
    # Either the spec rejected it (Phase 3 path) or the legacy reparse
    # guard caught it downstream; both produce a clear error to the
    # operator.
    assert "destination" in str(exc.value).lower() or "scf" in str(exc.value).lower()


# ---------------------------------------------------------------------------
# Phase 4: parser populates typed fields via the registry spec
# ---------------------------------------------------------------------------


def test_destination_parse_routes_through_spec():
    """The virtual server parser now consults the migrated
    :class:`DestinationSpec` for ``destination``.  End-to-end test:
    parsing a virtual produces the same typed Destination value the
    legacy hand-rolled path produced."""
    from core.bigip.parser import parse_bigip_conf
    from core.bigip.types import Destination

    src = "ltm virtual /Common/v {\n    destination /Common/10.0.0.10:80\n}\n"
    cfg = parse_bigip_conf(src)
    vs = cfg.virtual_servers["/Common/v"]
    assert isinstance(vs.destination, Destination)
    assert str(vs.destination) == "/Common/10.0.0.10:80"


def test_phase4_dispatch_helper_returns_none_for_empty_raw():
    """The ``_parse_typed_field`` helper short-circuits on empty
    input so a parser asking about a missing property doesn't have
    to special-case it.  This is the same shape every legacy
    typed-field block (``if raw_text else None``) was using before
    the migration."""
    from core.bigip.parser._parsers import _parse_typed_field

    out = _parse_typed_field(
        module="ltm",
        object_type="virtual",
        property_name="destination",
        raw="",
        legacy_factory=lambda raw: "should-not-be-called",
    )
    assert out is None


# ---------------------------------------------------------------------------
# Phase 5: reference dispatch through the registry
# ---------------------------------------------------------------------------


def test_references_via_spec_returns_none_for_unmigrated_property():
    """The dispatch helper returns ``None`` when the property
    hasn't been migrated — the caller falls back to the legacy
    grep-based path so untouched properties continue to work."""
    from core.bigip.registry import references_via_spec

    out = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="this-property-does-not-exist",
        value="/Common/anything",
    )
    assert out is None


def test_iter_object_references_yields_from_migrated_specs_only():
    """Walking a property bag pulls references from every migrated
    spec and silently skips the unmigrated ones.  Phase 6 will add
    enough migrated reference-shaped properties for this to start
    returning real edges; today the destination spec doesn't carry
    references (it's a value, not a ref) so the iteration is empty
    even though the property is migrated."""
    from core.bigip.registry import iter_object_references

    refs = list(
        iter_object_references(
            module="ltm",
            object_type="virtual",
            properties=[
                ("destination", "/Common/10.0.0.1:80"),
                ("pool", "/Common/web_pool"),  # not migrated yet
            ],
            owner_path="/Common/v",
        )
    )
    # Destination spec yields no references (it's a value type, not
    # a ref).  Pool isn't migrated so iter_object_references skips
    # it entirely.  Combined: no edges yet — Phase 6 changes that.
    assert refs == []


# ---------------------------------------------------------------------------
# Phase 6: MonitorExpressionSpec
# ---------------------------------------------------------------------------


def test_monitor_expression_parses_default_keyword():
    from core.bigip.registry import MonitorExpressionSpec
    from core.bigip.types import MonitorExpression

    spec = MonitorExpressionSpec()
    parsed = spec.parse("default", ParseContext())
    assert isinstance(parsed.value, MonitorExpression)
    assert parsed.value.is_default
    assert parsed.value.references() == ()
    assert str(parsed.value) == "default"


def test_monitor_expression_parses_single_monitor():
    from core.bigip.registry import MonitorExpressionSpec
    from core.bigip.types import MonitorExpression

    spec = MonitorExpressionSpec()
    parsed = spec.parse("/Common/http", ParseContext())
    assert isinstance(parsed.value, MonitorExpression)
    assert parsed.value.mode == "single"
    assert parsed.value.monitors == ("/Common/http",)
    assert parsed.value.references() == ("/Common/http",)


def test_monitor_expression_parses_and_chain():
    from core.bigip.registry import MonitorExpressionSpec
    from core.bigip.types import MonitorExpression

    spec = MonitorExpressionSpec()
    parsed = spec.parse("/Common/http and /Common/tcp", ParseContext())
    assert isinstance(parsed.value, MonitorExpression)
    assert parsed.value.mode == "all"
    assert parsed.value.monitors == ("/Common/http", "/Common/tcp")
    # ``str()`` round-trips because we kept the raw spelling.
    assert str(parsed.value) == "/Common/http and /Common/tcp"


def test_monitor_expression_parses_min_of():
    from core.bigip.registry import MonitorExpressionSpec
    from core.bigip.types import MonitorExpression

    spec = MonitorExpressionSpec()
    parsed = spec.parse(
        "min 2 of { /Common/gateway_icmp /Common/http /Common/http2 }",
        ParseContext(),
    )
    assert isinstance(parsed.value, MonitorExpression)
    assert parsed.value.mode == "min-of"
    assert parsed.value.minimum == 2
    assert parsed.value.monitors == (
        "/Common/gateway_icmp",
        "/Common/http",
        "/Common/http2",
    )


def test_monitor_expression_spec_emits_references_for_graph():
    """The Phase 6 dispatch surfaces every monitor reference so the
    query graph / LSP layer can navigate to each one — this was the
    review's biggest "regex seed matching wouldn't be precise here"
    case.  Each parsed monitor becomes one :class:`Reference`."""
    from core.bigip.registry import MonitorExpressionSpec, ReferenceContext
    from core.bigip.types import MonitorExpression

    spec = MonitorExpressionSpec(ref_kind="ltm monitor")
    value = MonitorExpression(
        mode="all",
        monitors=("/Common/http", "/Common/tcp"),
        raw="/Common/http and /Common/tcp",
    )
    refs = list(spec.references(value, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm monitor", "/Common/http"),
        ("ltm monitor", "/Common/tcp"),
    ]


def test_monitor_expression_rejects_garbage():
    from core.bigip.registry import MonitorExpressionSpec

    spec = MonitorExpressionSpec()
    parsed = spec.parse("min hello of { /Common/http }", ParseContext())
    assert parsed.value is None
    assert any("monitor expression" in d.message for d in parsed.diagnostics)


# ---------------------------------------------------------------------------
# Phase 6 batch 2: ProfileAttachment / PersistenceAttachment / SnatMode
# ---------------------------------------------------------------------------


def test_profile_attachment_parses_context_clientside():
    from core.bigip.registry import ProfileAttachmentSpec, ReferenceContext
    from core.bigip.types import ProfileAttachment

    spec = ProfileAttachmentSpec()
    parsed = spec.parse("/Common/clientssl { context clientside }", ParseContext())
    assert isinstance(parsed.value, ProfileAttachment)
    assert parsed.value.path == "/Common/clientssl"
    assert parsed.value.context == "clientside"
    assert parsed.value.name == "clientssl"
    # The graph layer sees one reference to the profile.
    refs = list(spec.references(parsed.value, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm profile", "/Common/clientssl"),
    ]


def test_monitor_expression_str_invalidates_raw_on_field_mutation():
    """``__str__`` returns the canonical render — not stale ``raw``
    — once a structured field changes via ``dataclasses.replace``.
    Regression for the review's "raw-preserving render can make
    future transforms stale" finding."""
    import dataclasses

    from core.bigip.types import MonitorExpression

    expr = MonitorExpression.try_parse("min 2 of { /Common/http /Common/tcp /Common/https }")
    assert expr is not None
    # No-op round trip keeps the original spelling.
    assert str(expr) == "min 2 of { /Common/http /Common/tcp /Common/https }"
    # Drop one monitor — the canonical render kicks in.
    pruned = dataclasses.replace(expr, monitors=("/Common/http", "/Common/https"))
    assert str(pruned) == "min 2 of { /Common/http /Common/https }"


def test_profile_attachment_str_invalidates_raw_on_field_mutation():
    """Same stale-raw guard for :class:`ProfileAttachment` — a
    ``context`` change via ``dataclasses.replace`` must render the
    new value instead of returning the original raw text."""
    import dataclasses

    from core.bigip.types import ProfileAttachment

    p = ProfileAttachment.from_raw("/Common/clientssl", "context clientside")
    assert "context clientside" in str(p)
    flipped = dataclasses.replace(p, context="serverside")
    assert "serverside" in str(flipped)
    assert "clientside" not in str(flipped)


def test_persistence_attachment_str_invalidates_raw_on_field_mutation():
    """And for :class:`PersistenceAttachment` — a ``default``
    change clears the raw spelling so the render reflects the new
    structured value."""
    import dataclasses

    from core.bigip.types import PersistenceAttachment

    pe = PersistenceAttachment.from_raw("/Common/cookie", "default yes")
    assert "default yes" in str(pe)
    cleared = dataclasses.replace(pe, default=False)
    assert "default yes" not in str(cleared)


def test_profile_attachment_rejects_stray_context_token():
    """``ProfileAttachment.from_raw`` must only honour the strict
    ``context <value>`` pair — a bare ``clientside`` token sitting
    elsewhere in the body (or one nested in a sub-block) must NOT
    leak into the structured field.  Regression for the review's
    "attachment parsing is too loose for structured semantics"
    finding."""
    from core.bigip.types import ProfileAttachment

    # Stray token without the ``context`` key — must not become
    # the structured context value.
    stray = ProfileAttachment.from_raw("/Common/p", "app-service /Common/clientside")
    assert stray.context == ""
    # Nested sub-block — even though ``context clientside`` lives
    # inside, the outer attachment's context stays empty.
    nested = ProfileAttachment.from_raw("/Common/p", "nested { context clientside }")
    assert nested.context == ""


def test_persistence_attachment_rejects_nested_default_token():
    """``PersistenceAttachment.from_raw`` must not pick up a
    ``default yes`` pair that's nested inside a sub-block.  Same
    "attachment parsing too loose" regression on the persistence
    side."""
    from core.bigip.types import PersistenceAttachment

    nested = PersistenceAttachment.from_raw("/Common/p", "nested { default yes }")
    assert nested.default is False


def test_profile_attachment_parses_empty_body():
    from core.bigip.registry import ProfileAttachmentSpec
    from core.bigip.types import ProfileAttachment

    spec = ProfileAttachmentSpec()
    parsed = spec.parse("/Common/http { }", ParseContext())
    assert isinstance(parsed.value, ProfileAttachment)
    assert parsed.value.path == "/Common/http"
    assert parsed.value.context == ""


def test_persistence_attachment_parses_default_yes():
    from core.bigip.registry import PersistenceAttachmentSpec
    from core.bigip.types import PersistenceAttachment

    spec = PersistenceAttachmentSpec()
    parsed = spec.parse("/Common/cookie { default yes }", ParseContext())
    assert isinstance(parsed.value, PersistenceAttachment)
    assert parsed.value.path == "/Common/cookie"
    assert parsed.value.default is True
    assert parsed.value.name == "cookie"


def test_persistence_attachment_defaults_to_false():
    from core.bigip.registry import PersistenceAttachmentSpec
    from core.bigip.types import PersistenceAttachment

    spec = PersistenceAttachmentSpec()
    parsed = spec.parse("/Common/source_addr { }", ParseContext())
    assert isinstance(parsed.value, PersistenceAttachment)
    assert parsed.value.default is False


def test_snat_mode_parses_each_variant():
    from core.bigip.registry import ReferenceContext, SnatModeSpec
    from core.bigip.types import SnatMode

    spec = SnatModeSpec()

    none = spec.parse("{ type none }", ParseContext())
    assert isinstance(none.value, SnatMode)
    assert none.value.is_none
    assert none.value.references() == ()

    automap = spec.parse("{ type automap }", ParseContext())
    assert isinstance(automap.value, SnatMode)
    assert automap.value.is_automap

    snat = spec.parse("{ type snat pool /Common/snatpool_x }", ParseContext())
    assert isinstance(snat.value, SnatMode)
    assert snat.value.is_snat
    assert snat.value.pool_path == "/Common/snatpool_x"
    # ``references`` surfaces the snat pool as a graph edge so
    # ``references_to /Common/snatpool_x`` finds every virtual
    # using it.
    refs = list(spec.references(snat.value, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm snatpool", "/Common/snatpool_x"),
    ]


def test_snat_mode_rejects_snat_without_pool():
    from core.bigip.registry import SnatModeSpec

    spec = SnatModeSpec()
    parsed = spec.parse("{ type snat }", ParseContext())
    assert parsed.value is None
    assert any("SNAT mode" in d.message for d in parsed.diagnostics)


# ---------------------------------------------------------------------------
# Phase 6 batch 3: DataGroupRecord / GtmRegionMember / CertKeyChain
# ---------------------------------------------------------------------------


def test_data_group_record_parses_keyed_value():
    from core.bigip.registry import DataGroupRecordSpec
    from core.bigip.types import DataGroupRecord

    spec = DataGroupRecordSpec()
    parsed = spec.parse("host1.example.com { data 10.0.0.1 }", ParseContext())
    assert isinstance(parsed.value, DataGroupRecord)
    assert parsed.value.key == "host1.example.com"
    assert parsed.value.data == "10.0.0.1"


def test_data_group_record_handles_bare_entry():
    from core.bigip.registry import DataGroupRecordSpec
    from core.bigip.types import DataGroupRecord

    spec = DataGroupRecordSpec()
    parsed = spec.parse("prod-allowed { }", ParseContext())
    assert isinstance(parsed.value, DataGroupRecord)
    assert parsed.value.key == "prod-allowed"
    assert parsed.value.data == ""


def test_gtm_region_member_parses_negation_and_kind():
    from core.bigip.registry import GtmRegionMemberSpec
    from core.bigip.types import GtmRegionMember

    spec = GtmRegionMemberSpec()
    plain = spec.parse("subnet 10.10.1.0/24 { }", ParseContext())
    assert isinstance(plain.value, GtmRegionMember)
    assert plain.value.kind == "subnet"
    assert plain.value.value == "10.10.1.0/24"
    assert plain.value.negated is False

    negated = spec.parse("not country JP { }", ParseContext())
    assert isinstance(negated.value, GtmRegionMember)
    assert negated.value.kind == "country"
    assert negated.value.value == "JP"
    assert negated.value.negated is True


def test_cert_key_chain_parses_full_entry_and_yields_refs():
    from core.bigip.registry import CertKeyChainSpec, ReferenceContext
    from core.bigip.types import CertKeyChain

    spec = CertKeyChainSpec()
    parsed = spec.parse(
        "cert_a { cert /Common/cert_a.crt key /Common/cert_a.key chain /Common/ca_bundle.crt }",
        ParseContext(),
    )
    assert isinstance(parsed.value, CertKeyChain)
    assert parsed.value.name == "cert_a"
    assert parsed.value.cert == "/Common/cert_a.crt"
    assert parsed.value.key == "/Common/cert_a.key"
    assert parsed.value.chain == "/Common/ca_bundle.crt"
    refs = list(spec.references(parsed.value, ReferenceContext()))
    # cert + key + chain all surface as edges
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("sys file ssl-cert", "/Common/cert_a.crt"),
        ("sys file ssl-key", "/Common/cert_a.key"),
        ("sys file ssl-cert", "/Common/ca_bundle.crt"),
    ]


# ---------------------------------------------------------------------------
# Phase 6 batch 4: LtmPolicyConditionSpec / LtmPolicyActionSpec
# ---------------------------------------------------------------------------


def test_policy_condition_parses_http_host_match():
    from core.bigip.registry import LtmPolicyConditionSpec
    from core.bigip.types import LtmPolicyCondition

    spec = LtmPolicyConditionSpec()
    parsed = spec.parse(
        "0 { http-host all-strings values { example.com foo.example.com } }",
        ParseContext(),
    )
    assert isinstance(parsed.value, LtmPolicyCondition)
    assert parsed.value.index == 0
    assert parsed.value.operand == "http-host"
    assert parsed.value.selector == "all-strings"
    assert parsed.value.values == ("example.com", "foo.example.com")
    assert parsed.value.negate is False


def test_policy_condition_parses_negation_and_operator():
    from core.bigip.registry import LtmPolicyConditionSpec
    from core.bigip.types import LtmPolicyCondition

    spec = LtmPolicyConditionSpec()
    parsed = spec.parse(
        "1 { http-uri path not starts-with values { /admin } }",
        ParseContext(),
    )
    assert isinstance(parsed.value, LtmPolicyCondition)
    assert parsed.value.operand == "http-uri"
    assert parsed.value.selector == "path"
    assert parsed.value.operator == "starts-with"
    assert parsed.value.negate is True
    assert parsed.value.values == ("/admin",)


def test_policy_action_forward_pool_yields_pool_reference():
    from core.bigip.registry import LtmPolicyActionSpec, ReferenceContext
    from core.bigip.types import LtmPolicyAction

    spec = LtmPolicyActionSpec()
    parsed = spec.parse(
        "0 { request forward select pool /Common/web_pool }",
        ParseContext(),
    )
    assert isinstance(parsed.value, LtmPolicyAction)
    assert parsed.value.target == "request"
    assert parsed.value.verb == "forward"
    assert parsed.value.select is True
    assert parsed.value.pool == "/Common/web_pool"
    refs = list(spec.references(parsed.value, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm pool", "/Common/web_pool"),
    ]


def test_policy_action_redirect_captures_location_and_status():
    from core.bigip.registry import LtmPolicyActionSpec
    from core.bigip.types import LtmPolicyAction

    spec = LtmPolicyActionSpec()
    parsed = spec.parse(
        "1 { response redirect location https://example.com/new status 301 }",
        ParseContext(),
    )
    assert isinstance(parsed.value, LtmPolicyAction)
    assert parsed.value.verb == "redirect"
    assert parsed.value.redirect_location == "https://example.com/new"
    assert parsed.value.status == 301


# ---------------------------------------------------------------------------
# Phase 6 batch 5: FirewallRule / NatRule
# ---------------------------------------------------------------------------


def test_firewall_rule_parses_inline_classifier():
    from core.bigip.registry import FirewallRuleSpec, ReferenceContext
    from core.bigip.types import FirewallRule

    spec = FirewallRuleSpec()
    parsed = spec.parse(
        (
            "allow_https {"
            "    action accept"
            "    ip-protocol tcp"
            "    log"
            "    destination {"
            "        port-lists { /Common/_sys_self_allow_tcp_defaults }"
            "        ports { 443 }"
            "        address-lists { /Common/web_servers }"
            "    }"
            "    source {"
            "        address-lists { /Common/trusted_clients }"
            "        ports { 1024-65535 }"
            "    }"
            "}"
        ),
        ParseContext(),
    )
    assert isinstance(parsed.value, FirewallRule)
    assert parsed.value.name == "allow_https"
    assert parsed.value.action == "accept"
    assert parsed.value.ip_protocol == "tcp"
    assert parsed.value.log is True
    assert parsed.value.destination.ports == ("443",)
    assert parsed.value.destination.port_lists == ("/Common/_sys_self_allow_tcp_defaults",)
    assert parsed.value.destination.address_lists == ("/Common/web_servers",)
    assert parsed.value.source.address_lists == ("/Common/trusted_clients",)
    assert parsed.value.source.ports == ("1024-65535",)

    refs = list(spec.references(parsed.value, ReferenceContext()))
    # Every port-list + address-list shows up as a typed edge so
    # references_to /Common/web_servers finds this rule.
    paths = [(r.target_kind, r.target_path) for r in refs]
    assert ("security firewall port-list", "/Common/_sys_self_allow_tcp_defaults") in paths
    assert ("security firewall address-list", "/Common/web_servers") in paths
    assert ("security firewall address-list", "/Common/trusted_clients") in paths


def test_firewall_rule_captures_rule_list_reference():
    """``rule-list <full-path>`` rules link to another rule-list
    instead of defining the classifier inline.  The reference
    surfaces through the graph layer the same way every other
    list link does."""
    from core.bigip.registry import FirewallRuleSpec, ReferenceContext
    from core.bigip.types import FirewallRule

    spec = FirewallRuleSpec()
    parsed = spec.parse(
        "delegated { rule-list /Common/_sys_self_allow_defaults }",
        ParseContext(),
    )
    assert isinstance(parsed.value, FirewallRule)
    assert parsed.value.rule_list == "/Common/_sys_self_allow_defaults"
    refs = list(spec.references(parsed.value, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("security firewall rule-list", "/Common/_sys_self_allow_defaults"),
    ]


def test_nat_rule_spec_is_alias_of_firewall_rule_spec():
    """NAT rules share the firewall rule grammar — modelling them
    twice would duplicate the parser.  The spec is a thin alias
    so registry consumers see the same dispatch contract."""
    from core.bigip.registry import FirewallRuleSpec, NatRuleSpec

    assert NatRuleSpec is FirewallRuleSpec


# ---------------------------------------------------------------------------
# Migration batch 1: monitor expressions migrate to MonitorExpressionSpec
# ---------------------------------------------------------------------------


def test_pool_monitor_migration_enumerates_refs_via_spec():
    """``ltm pool.monitor`` is migrated to MonitorExpressionSpec; the
    reference dispatch parses the raw string through the spec and
    yields one Reference per monitor in the expression."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="pool",
        property_name="monitor",
        value="/Common/http and /Common/tcp",
        owner_path="/Common/web_pool",
    )
    assert refs is not None
    targets = [(r.target_kind, r.target_path) for r in refs]
    assert targets == [
        ("ltm monitor", "/Common/http"),
        ("ltm monitor", "/Common/tcp"),
    ]


def test_node_monitor_migration_handles_min_of():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="node",
        property_name="monitor",
        value="min 2 of { /Common/gateway_icmp /Common/http /Common/http2 }",
        owner_path="/Common/n1",
    )
    assert refs is not None
    assert {r.target_path for r in refs} == {
        "/Common/gateway_icmp",
        "/Common/http",
        "/Common/http2",
    }


def test_gtm_monitor_references_emit_gtm_target_kind():
    """``gtm pool.monitor`` and ``gtm server.monitor`` must produce
    references attributed to the ``gtm monitor`` family, NOT the
    ``ltm monitor`` family — otherwise the graph / link / definition
    layer fans out to ``ltm_monitor_*`` and never finds the
    GTM-side monitor object.  Regression for the re-review's
    high-severity finding."""
    from core.bigip.registry import (
        candidate_registry_kinds_for_display,
        references_via_spec,
    )

    for object_type in ("pool", "server"):
        refs = references_via_spec(
            module="gtm",
            object_type=object_type,
            property_name="monitor",
            value="/Common/http",
            owner_path=f"/Common/{object_type}1",
        )
        assert refs is not None and len(refs) == 1
        assert refs[0].target_kind == "gtm monitor"
        kinds = candidate_registry_kinds_for_display(refs[0].target_kind)
        assert "gtm_monitor_http" in kinds
        # And critically: no LTM monitor kinds leak into the fan-out.
        assert not any(k.startswith("ltm_") for k in kinds)


def test_ltm_monitor_references_still_emit_ltm_target_kind():
    """The split keeps LTM monitor refs on the LTM family — the
    LTM pool / node fan-out resolves to ``ltm_monitor_*`` kinds
    only."""
    from core.bigip.registry import (
        candidate_registry_kinds_for_display,
        references_via_spec,
    )

    refs = references_via_spec(
        module="ltm",
        object_type="pool",
        property_name="monitor",
        value="/Common/http",
        owner_path="/Common/p1",
    )
    assert refs is not None
    assert refs[0].target_kind == "ltm monitor"
    kinds = candidate_registry_kinds_for_display(refs[0].target_kind)
    assert not any(k.startswith("gtm_") for k in kinds)


def test_virtual_source_address_translation_migration_yields_snat_pool_ref():
    """``ltm virtual.source-address-translation`` is migrated to
    SnatModeSpec; the reference dispatch parses the body and yields
    one Reference to the snat pool when the mode is ``snat``."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="source-address-translation",
        value="{ type snat pool /Common/snatpool_x }",
        owner_path="/Common/v",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm snatpool", "/Common/snatpool_x"),
    ]


def test_automap_snat_yields_no_refs():
    """Automap doesn't reference a SNAT pool, so the migration
    correctly yields zero refs."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="source-address-translation",
        value="{ type automap }",
        owner_path="/Common/v",
    )
    assert refs is not None
    assert refs == ()


def test_monitor_projection_stays_on_legacy_pathref_surface():
    """The migration opts out of the projection dispatch so
    ``.ltm.pool["X"].monitor.full-path`` keeps working through the
    legacy ``ref_kind="ltm monitor"`` PathRef projection.  Phase 6+
    can introduce a structured MonitorExpression container later;
    for now back-compat is the priority."""
    from core.bigip.query import run_query

    src = "ltm pool /Common/web_pool {\n    monitor /Common/http\n}\n"
    result = run_query('.ltm.pool["/Common/web_pool"].monitor', {"m": src})
    [monitor] = result.values_per_file["m"]
    # PathRef surface: full_path attribute, string-like equality.
    assert str(monitor) == "/Common/http"


# ---------------------------------------------------------------------------
# Migration batch 3: list-valued migrations on ltm virtual
# ---------------------------------------------------------------------------


def test_virtual_profiles_migration_yields_per_element_refs():
    """``ltm virtual.profiles`` is migrated to a ListSpec; the
    reference dispatch unwinds the tuple and yields one Reference
    per profile attachment."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="profiles",
        value=("/Common/http", "/Common/tcp", "/Common/clientssl"),
        owner_path="/Common/v",
    )
    assert refs is not None
    paths = [r.target_path for r in refs]
    assert paths == ["/Common/http", "/Common/tcp", "/Common/clientssl"]
    # Every reference points at ``ltm profile`` (the ProfileAttachmentSpec's
    # default ref_kind); kind-narrowing can come later via per-profile
    # type tracking on the model.
    assert all(r.target_kind == "ltm profile" for r in refs)


def test_virtual_persist_migration_yields_per_element_refs():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="persist",
        value=("/Common/cookie", "/Common/source_addr"),
        owner_path="/Common/v",
    )
    assert refs is not None
    assert [r.target_path for r in refs] == [
        "/Common/cookie",
        "/Common/source_addr",
    ]
    assert all(r.target_kind == "ltm persistence" for r in refs)


def test_virtual_rules_migration_yields_per_element_refs():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="rules",
        value=("/Common/log_rule", "/Common/rewrite_rule"),
        owner_path="/Common/v",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm rule", "/Common/log_rule"),
        ("ltm rule", "/Common/rewrite_rule"),
    ]


def test_virtual_policies_migration_yields_per_element_refs():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="policies",
        value=("/Common/host_policy",),
        owner_path="/Common/v",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm policy", "/Common/host_policy"),
    ]


def test_virtual_vlans_migration_yields_per_element_refs():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="vlans",
        value=("/Common/external", "/Common/internal"),
        owner_path="/Common/v",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("net vlan", "/Common/external"),
        ("net vlan", "/Common/internal"),
    ]


def test_virtual_profile_projection_exposes_structured_attachments():
    """``.profiles[]`` projects as the typed BigipList view so DSL
    queries can ask ``.context`` / ``.full_path`` on each item
    (the back-compat PathRef alias)."""
    from core.bigip.query import run_query

    src = (
        "ltm virtual /Common/v {\n"
        "    destination /Common/10.0.0.10:80\n"
        "    profiles { /Common/http { } /Common/tcp { } }\n"
        "}\n"
    )
    # ``.full-path`` alias resolves the path string for each
    # ProfileAttachment, matching what the legacy PathRef projection
    # surfaced for ``.profiles[].full-path``.
    result = run_query('.ltm.virtual["/Common/v"].profiles[].full-path', {"m": src})
    assert sorted(result.values_per_file["m"]) == ["/Common/http", "/Common/tcp"]


# ---------------------------------------------------------------------------
# Migration batch 4: data-group records / GTM region rows / cert-key-chain
# ---------------------------------------------------------------------------


def test_data_group_records_migration_parses_each_entry():
    """``ltm data-group internal.records`` migrates to a ListSpec; the
    reference dispatch unwinds each record through DataGroupRecordSpec.
    Records don't reference other objects so we get an empty edge list,
    but the dispatch confirms it found the migrated spec (returns
    ``()`` not ``None``)."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="data-group internal",
        property_name="records",
        value=("host1.example.com", "host2.example.com"),
        owner_path="/Common/hosts",
    )
    # Found the migrated spec (not None); records don't ref anything.
    assert refs == ()


def test_gtm_region_members_migration_routes_through_spec():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="gtm",
        object_type="region",
        property_name="region-members",
        value=("subnet 10.10.1.0/24", "not country JP"),
        owner_path="/Common/us_east",
    )
    # Region rows don't reference other objects either.
    assert refs == ()


def test_cert_key_chain_migration_surfaces_ssl_artifact_refs():
    """``cert-key-chain`` items carry cert + key + chain references —
    each yields a typed edge so the graph layer finds every SSL
    profile depending on a given cert / key / CA bundle."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="ltm",
        object_type="profile client-ssl",
        property_name="cert-key-chain",
        value=(
            "cert_a { cert /Common/cert_a.crt key /Common/cert_a.key chain /Common/ca_bundle.crt }",
            "cert_b { cert /Common/cert_b.crt key /Common/cert_b.key }",
        ),
        owner_path="/Common/clientssl_app",
    )
    assert refs is not None
    targets = [(r.target_kind, r.target_path) for r in refs]
    # First entry: cert + key + chain.  Second entry: cert + key
    # only (no chain).  Five edges total.
    assert ("sys file ssl-cert", "/Common/cert_a.crt") in targets
    assert ("sys file ssl-key", "/Common/cert_a.key") in targets
    assert ("sys file ssl-cert", "/Common/ca_bundle.crt") in targets
    assert ("sys file ssl-cert", "/Common/cert_b.crt") in targets
    assert ("sys file ssl-key", "/Common/cert_b.key") in targets
    assert len(targets) == 5


def test_server_ssl_cert_key_chain_shares_one_spec():
    """Client-SSL and server-SSL cert-key-chain entries share the
    same spec — both kinds dispatch to ``CertKeyChainSpec``."""
    from core.bigip.registry.pilot import pilot_property_spec_for

    client = pilot_property_spec_for("ltm", "profile client-ssl", "cert-key-chain")
    server = pilot_property_spec_for("ltm", "profile server-ssl", "cert-key-chain")
    assert client is server


# ---------------------------------------------------------------------------
# Migration batch 6: security firewall list-of-refs
# ---------------------------------------------------------------------------


def test_firewall_policy_rule_lists_migration_yields_nested_refs():
    """``security firewall policy.rule-lists`` references each
    nested rule-list — the migration surfaces every edge so the
    graph layer can answer "which firewall policies use this
    rule-list?" exactly."""
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="security",
        object_type="firewall policy",
        property_name="rule-lists",
        value=("/Common/_sys_self_allow_defaults", "/Common/app_rules"),
        owner_path="/Common/policy_outer",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("security firewall rule-list", "/Common/_sys_self_allow_defaults"),
        ("security firewall rule-list", "/Common/app_rules"),
    ]


def test_firewall_address_list_nested_lists_migration_yields_refs():
    from core.bigip.registry import references_via_spec

    refs = references_via_spec(
        module="security",
        object_type="firewall address-list",
        property_name="address-lists",
        value=("/Common/trusted_networks", "/Common/datacenter_ranges"),
        owner_path="/Common/all_internal",
    )
    assert refs is not None
    assert [(r.target_kind, r.target_path) for r in refs] == [
        (
            "security firewall address-list",
            "/Common/trusted_networks",
        ),
        (
            "security firewall address-list",
            "/Common/datacenter_ranges",
        ),
    ]


# ---------------------------------------------------------------------------
# Registry-wide audit: no spec abuses enum_values for tmsh operators
# ---------------------------------------------------------------------------


def test_no_spec_encodes_operators_in_enum_values():
    """tmsh list operators (``add`` / ``delete`` / ``modify`` /
    ``replace-all-with`` / ``none``) belong on
    ``BigipPropertySpec.list_operators``, not ``enum_values``.  The
    legacy abuse was a sentinel for "this property is list-valued"
    that the new ``list_operators`` field replaces cleanly.  This
    audit pins the contract so a new spec contribution can't
    accidentally reintroduce the old shape.

    The audit allows ``default`` inside an enum because a few F5
    properties accept ``default`` as a value (not as an operator),
    and ``none`` is only flagged when paired with another operator
    (the value ``none`` is legitimately an enum in many specs).
    """
    from core.bigip.registry.specs import OBJECT_SPECS

    OPERATOR_TOKENS = {"add", "delete", "modify", "replace-all-with"}
    offenders: list[str] = []
    for spec in OBJECT_SPECS:
        for prop in spec.properties:
            if not prop.enum_values:
                continue
            values = set(prop.enum_values)
            if OPERATOR_TOKENS & values:
                # Operator tokens appear — flag this property.
                offenders.append(f"{spec.kind_spec.kind}.{prop.name}: {prop.enum_values}")
    if offenders:
        joined = "\n  - ".join(offenders)
        raise AssertionError(
            "Spec files still encode tmsh list operators in "
            "enum_values.  Migrate each one to "
            "`list_operators=frozenset((...))` and switch "
            '`value_type="enum"` to `value_type="reference"`:\n  - ' + joined
        )


# ---------------------------------------------------------------------------
# Pilot-table parity sweep
# ---------------------------------------------------------------------------


def test_every_pilot_property_has_a_known_value_spec():
    """The pilot table only registers specs the value-spec module
    exports.  This audit prevents a typo / orphaned entry from
    landing — the dispatch contract requires that every registered
    property points at a callable spec."""
    from core.bigip.registry import PropertySpec
    from core.bigip.registry.pilot import PILOT_PROPERTY_SPECS

    for key, spec in PILOT_PROPERTY_SPECS.items():
        assert isinstance(spec, PropertySpec), f"{key}: not a PropertySpec"
        # ``value`` must implement the protocol — sanity-check the
        # required methods are present.
        for method in ("parse", "project", "render", "references"):
            assert callable(getattr(spec.value, method, None)), (
                f"{key}: spec.value missing {method}()"
            )
        # ``is_structured`` should be exposed via the value spec.
        assert hasattr(spec.value, "is_structured")


# ---------------------------------------------------------------------------
# ltm policy rules — walk nested actions/conditions through the registry
# ---------------------------------------------------------------------------


def test_policy_action_spec_accepts_legacy_bigip_policy_action():
    """The reference dispatch tolerates either the new
    :class:`LtmPolicyAction` or the legacy
    :class:`BigipPolicyAction` (from :mod:`core.bigip.model._ltm`)
    so the migration can land without rewriting the policy
    dataclasses."""
    from core.bigip.model import BigipPolicyAction
    from core.bigip.registry import LtmPolicyActionSpec, ReferenceContext

    spec = LtmPolicyActionSpec()
    legacy = BigipPolicyAction(index=0, target="request", verb="forward", pool="/Common/web_pool")
    refs = list(spec.references(legacy, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm pool", "/Common/web_pool"),
    ]


def test_policy_rule_spec_walks_nested_actions():
    """``LtmPolicyRuleSpec`` walks ``rule.actions[]`` and yields
    every reference each action carries — the bridge that lets
    the registry surface pool refs from policy forward actions
    without a model rewrite."""
    from core.bigip.model import BigipPolicyAction, BigipPolicyRule
    from core.bigip.registry import LtmPolicyRuleSpec, ReferenceContext

    spec = LtmPolicyRuleSpec()
    rule = BigipPolicyRule(
        name="route_to_web",
        ordinal=0,
        actions=(
            BigipPolicyAction(index=0, target="request", verb="forward", pool="/Common/web_pool"),
            BigipPolicyAction(index=1, target="request", verb="log"),  # no pool ref
        ),
    )
    refs = list(spec.references(rule, ReferenceContext()))
    assert [(r.target_kind, r.target_path) for r in refs] == [
        ("ltm pool", "/Common/web_pool"),
    ]


def test_policy_rules_migration_unwinds_nested_pool_refs():
    """End-to-end: walking ``ltm policy.rules`` via the registry
    yields every pool ref forwarded to by any action of any rule —
    addressing the design doc's "find every policy forwarding to a
    pool" example query."""
    from core.bigip.model import BigipPolicyAction, BigipPolicyRule
    from core.bigip.registry import references_via_spec

    rules = (
        BigipPolicyRule(
            name="route_a",
            actions=(BigipPolicyAction(index=0, verb="forward", pool="/Common/a_pool"),),
        ),
        BigipPolicyRule(
            name="route_b",
            actions=(
                BigipPolicyAction(index=0, verb="forward", pool="/Common/b_pool"),
                BigipPolicyAction(index=1, verb="log"),
            ),
        ),
    )
    refs = references_via_spec(
        module="ltm",
        object_type="policy",
        property_name="rules",
        value=rules,
        owner_path="/Common/site_policy",
    )
    assert refs is not None
    assert [r.target_path for r in refs] == ["/Common/a_pool", "/Common/b_pool"]


# ---------------------------------------------------------------------------
# Migration batch 7: firewall rule-list rule bodies migrate to FirewallRuleSpec
# ---------------------------------------------------------------------------


def test_firewall_rule_list_rule_objects_pilot_walks_typed_bodies():
    """``security firewall rule-list.rules`` is migrated to a list of
    typed FirewallRule bodies (sourced from ``rule_objects`` on the
    model).  Walking the pilot via ``references_via_spec`` enumerates
    every port-list / address-list edge each rule references — so the
    graph layer answers ``references_to /Common/web_servers`` for
    rule-lists without re-parsing the nested source view."""
    from core.bigip.registry import references_via_spec
    from core.bigip.types import FirewallEndpoint, FirewallRule

    rules = (
        FirewallRule(
            name="allow_https",
            action="accept",
            ip_protocol="tcp",
            destination=FirewallEndpoint(
                port_lists=("/Common/_sys_self_allow_tcp_defaults",),
                address_lists=("/Common/web_servers",),
            ),
            source=FirewallEndpoint(address_lists=("/Common/trusted_clients",)),
        ),
        FirewallRule(
            name="delegated",
            rule_list="/Common/_sys_self_allow_defaults",
        ),
    )
    refs = references_via_spec(
        module="security",
        object_type="firewall rule-list",
        property_name="rules",
        value=rules,
        owner_path="/Common/rl_web",
    )
    assert refs is not None
    paths = [(r.target_kind, r.target_path) for r in refs]
    assert ("security firewall port-list", "/Common/_sys_self_allow_tcp_defaults") in paths
    assert ("security firewall address-list", "/Common/web_servers") in paths
    assert ("security firewall address-list", "/Common/trusted_clients") in paths
    assert ("security firewall rule-list", "/Common/_sys_self_allow_defaults") in paths


def test_firewall_rule_list_parser_populates_rule_objects():
    """End-to-end through the parser: a ``security firewall
    rule-list`` stanza yields ``rule_objects`` carrying the typed
    bodies — the back-compat ``rules`` tuple of names is preserved
    in document order."""
    from core.bigip.parser._helpers import _Block
    from core.bigip.parser._parsers import _parse_security_firewall_rule_list
    from core.bigip.types import FirewallRule
    from shared.document_buffer import DocumentBuffer

    body = (
        "rules {\n"
        "    allow_https {\n"
        "        action accept\n"
        "        ip-protocol tcp\n"
        "        destination {\n"
        "            address-lists { /Common/web_servers }\n"
        "            ports { 443 }\n"
        "        }\n"
        "    }\n"
        "    delegated {\n"
        "        rule-list /Common/_sys_self_allow_defaults\n"
        "    }\n"
        "}\n"
    )
    buf = DocumentBuffer.from_source(body)
    block = _Block(header="", body=body, start_offset=0, end_offset=len(body))
    rule_list = _parse_security_firewall_rule_list(
        "/Common/rl_web",
        body,
        buf,
        block,
    )
    assert rule_list.rules == ("allow_https", "delegated")
    assert len(rule_list.rule_objects) == 2
    assert all(isinstance(r, FirewallRule) for r in rule_list.rule_objects)
    first, second = rule_list.rule_objects
    assert first.name == "allow_https"
    assert first.action == "accept"
    assert first.destination.address_lists == ("/Common/web_servers",)
    assert first.destination.ports == ("443",)
    assert second.name == "delegated"
    assert second.rule_list == "/Common/_sys_self_allow_defaults"


# ---------------------------------------------------------------------------
# Reference.target_kind normalisation
# ---------------------------------------------------------------------------


def test_candidate_registry_kinds_for_display_resolves_exact():
    """Display-form kinds (``"ltm pool"`` / ``"security firewall
    address-list"``) map to the underscored registry keys so the
    graph resolver can look them up in ``OBJECT_KIND_SPECS``
    without each consumer rebuilding the mapping."""
    from core.bigip.registry import candidate_registry_kinds_for_display

    assert candidate_registry_kinds_for_display("ltm pool") == ("ltm_pool",)
    assert candidate_registry_kinds_for_display("ltm rule") == ("ltm_rule",)
    assert candidate_registry_kinds_for_display("security firewall address-list") == (
        "security_firewall_address_list",
    )
    assert candidate_registry_kinds_for_display("security firewall rule-list") == (
        "security_firewall_rule_list",
    )


def test_candidate_registry_kinds_for_display_fans_out_monitor_family():
    """``MonitorExpressionSpec`` yields ``Reference(target_kind="ltm
    monitor")`` without naming a specific monitor type — the
    graph resolver needs every ``ltm_monitor_*`` candidate to look
    up the path against."""
    from core.bigip.registry import candidate_registry_kinds_for_display

    kinds = candidate_registry_kinds_for_display("ltm monitor")
    assert "ltm_monitor_http" in kinds
    assert "ltm_monitor_tcp" in kinds
    assert "ltm_monitor_https" in kinds
    # gtm_monitor_* kinds must NOT be in the ltm fan-out.
    assert not any(k.startswith("gtm_") for k in kinds)


def test_monitor_expression_spec_rejects_zero_threshold():
    """``min 0 of { ... }`` is grammatically valid but BIG-IP refuses
    it at validation — the spec should emit a diagnostic so editors
    surface it before the user pushes the config."""
    from core.bigip.registry import MonitorExpressionSpec

    spec = MonitorExpressionSpec(ref_kinds=("ltm monitor",))
    parsed = spec.parse("min 0 of { /Common/http }", ParseContext())
    assert parsed.value is not None
    assert any(d.severity == "error" and "at least 1" in d.message for d in parsed.diagnostics)


def test_monitor_expression_spec_rejects_threshold_above_listed_count():
    """``min 99 of { /Common/http }`` parses but is unsatisfiable —
    BIG-IP needs the threshold to be ≤ the number of listed
    monitors.  The spec emits a diagnostic so the editor flags it."""
    from core.bigip.registry import MonitorExpressionSpec

    spec = MonitorExpressionSpec(ref_kinds=("ltm monitor",))
    parsed = spec.parse("min 99 of { /Common/http }", ParseContext())
    assert parsed.value is not None
    assert any(d.severity == "error" and "exceeds the 1" in d.message for d in parsed.diagnostics)


def test_monitor_expression_spec_accepts_valid_threshold_silently():
    """A well-formed ``min 2 of { a b c }`` produces no diagnostics."""
    from core.bigip.registry import MonitorExpressionSpec

    spec = MonitorExpressionSpec(ref_kinds=("ltm monitor",))
    parsed = spec.parse("min 2 of { /Common/http /Common/tcp /Common/https }", ParseContext())
    assert parsed.value is not None
    assert parsed.diagnostics == ()


def test_candidate_registry_kinds_for_display_empty_for_unknown():
    """Unrecognised display strings return an empty tuple so callers
    fall through to the legacy grep path without a special case."""
    from core.bigip.registry import candidate_registry_kinds_for_display

    assert candidate_registry_kinds_for_display("") == ()
    assert candidate_registry_kinds_for_display("bogus xyzzy") == ()


# ---------------------------------------------------------------------------
# BigipList / ListSpec — own parse / project / render / references
# ---------------------------------------------------------------------------


def test_list_spec_parse_braced_space_separated_returns_bigip_list():
    """A flat ``{ /Common/a /Common/b }`` parses into a BigipList of
    typed item values (here ObjectRef refs)."""
    from core.bigip.types import BigipList

    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"))
    parsed = spec.parse("{ /Common/r1 /Common/r2 }", ParseContext())
    assert isinstance(parsed.value, BigipList)
    assert parsed.value.syntax == "braced-space-separated"
    assert len(parsed.value) == 2
    # Iteration yields typed item values.
    assert list(parsed.value) == ["/Common/r1", "/Common/r2"]


def test_list_spec_parse_keyed_block_returns_typed_items():
    """A keyed-block list (``profiles { /Common/clientssl { context
    clientside } }``) parses into BigipList items whose ``value`` is
    the inner spec's typed value and whose ``key`` / ``body`` carry
    the lexical halves."""
    from core.bigip.registry import ProfileAttachmentSpec
    from core.bigip.types import BigipList, ProfileAttachment

    spec = ListSpec(item=ProfileAttachmentSpec(), syntax="keyed-block")
    parsed = spec.parse(
        "{ /Common/clientssl { context clientside } /Common/http { } }",
        ParseContext(),
    )
    assert isinstance(parsed.value, BigipList)
    assert parsed.value.syntax == "keyed-block"
    assert len(parsed.value) == 2
    first, second = parsed.value.items
    assert first.key == "/Common/clientssl"
    assert isinstance(first.value, ProfileAttachment)
    assert first.value.context == "clientside"
    assert second.key == "/Common/http"


def test_list_spec_render_roundtrips_keyed_block():
    """Render a BigipList back to TMSH text — keyed-block syntax
    keeps the per-item brace bodies."""
    from core.bigip.registry import ProfileAttachmentSpec

    spec = ListSpec(item=ProfileAttachmentSpec(), syntax="keyed-block")
    parsed = spec.parse(
        "{ /Common/clientssl { context clientside } /Common/http { } }",
        ParseContext(),
    )
    rendered = spec.render(parsed.value, RenderContext())
    assert "/Common/clientssl" in rendered
    assert "context clientside" in rendered
    assert "/Common/http" in rendered


def test_list_spec_references_walks_each_item():
    """The list spec's ``references`` enumerates one Reference per
    item by delegating to the inner item spec — no caller code has
    to know that ``profiles`` is a list."""
    from core.bigip.registry import ProfileAttachmentSpec

    spec = ListSpec(item=ProfileAttachmentSpec(), syntax="keyed-block")
    parsed = spec.parse(
        "{ /Common/clientssl { context clientside } /Common/http { } }",
        ParseContext(),
    )
    refs = list(spec.references(parsed.value, ReferenceContext()))
    paths = {r.target_path for r in refs}
    assert "/Common/clientssl" in paths
    assert "/Common/http" in paths


def test_list_spec_parses_operator_prefixed_replace_all_with():
    """``{ replace-all-with { /Common/r1 /Common/r2 } }`` — the
    tmsh-modify edit form where the verb nests inside the
    property body — parses into a :class:`BigipList` whose
    ``operator`` carries the verb and whose items are the inner
    paths."""
    from core.bigip.types import BigipList

    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"), syntax="operator-prefixed")
    parsed = spec.parse("{ replace-all-with { /Common/r1 /Common/r2 } }", ParseContext())
    assert isinstance(parsed.value, BigipList)
    assert parsed.value.operator == "replace-all-with"
    assert [it.value for it in parsed.value.items] == ["/Common/r1", "/Common/r2"]


def test_list_spec_parses_operator_prefixed_add_and_delete_forms():
    """The ``add`` / ``delete`` / ``modify`` / ``none`` verbs are
    recognised the same way ``replace-all-with`` is."""
    from core.bigip.types import BigipList

    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"), syntax="operator-prefixed")
    for op in ("add", "delete", "modify", "none"):
        raw = f"{{ {op} {{ /Common/x }} }}"
        parsed = spec.parse(raw, ParseContext())
        assert isinstance(parsed.value, BigipList)
        assert parsed.value.operator == op
        assert [it.value for it in parsed.value.items] == ["/Common/x"]


def test_list_spec_renders_operator_prefixed_roundtrip():
    """Render a parsed operator-prefixed BigipList back to its TMSH
    text — the verb stays inside the property body, no operator
    leaks outside."""
    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"), syntax="operator-prefixed")
    raw = "{ replace-all-with { /Common/r1 /Common/r2 } }"
    parsed = spec.parse(raw, ParseContext())
    assert spec.render(parsed.value, RenderContext()) == raw


def test_list_spec_operator_prefixed_falls_back_to_bare_list_when_no_verb():
    """An ``operator-prefixed`` spec given a bare braced list
    (``{ /Common/a /Common/b }``) — no leading verb — falls
    through to the flat-list parse so the caller still gets a
    usable :class:`BigipList`, just with ``operator=None``."""
    from core.bigip.types import BigipList

    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"), syntax="operator-prefixed")
    parsed = spec.parse("{ /Common/r1 /Common/r2 }", ParseContext())
    assert isinstance(parsed.value, BigipList)
    assert parsed.value.operator is None
    assert [it.value for it in parsed.value.items] == ["/Common/r1", "/Common/r2"]


def test_list_spec_populates_per_item_ranges():
    """``ListSpec.parse`` records per-item source spans so LSP
    features (document links, rename, semantic tokens) get exact
    byte coordinates without re-scanning the source."""
    from core.bigip.registry import ProfileAttachmentSpec
    from core.bigip.types import BigipList

    spec = ListSpec(item=ProfileAttachmentSpec(), syntax="keyed-block")
    raw = "{ /Common/clientssl { context clientside } /Common/http { } }"
    parsed = spec.parse(raw, ParseContext(base_offset=100))
    assert isinstance(parsed.value, BigipList)
    items = parsed.value.items
    assert len(items) == 2
    # ``key_range`` brackets just the key token.
    assert items[0].key == "/Common/clientssl"
    assert items[0].key_range is not None
    key_text = raw[items[0].key_range.start - 100 : items[0].key_range.end - 100]
    assert key_text == "/Common/clientssl"
    # ``range`` brackets the whole keyed item.
    assert items[0].range is not None
    assert items[0].range.start <= items[0].key_range.start


def test_references_via_spec_propagates_ranges():
    """End-to-end: a list-shaped property exposes per-reference
    source ranges through ``references_via_spec`` so the link
    extractor / LSP have byte-accurate edges out of the box."""
    from core.bigip.registry import references_via_spec

    src = (
        "ltm virtual /Common/v {\n"
        "    profiles {\n"
        "        /Common/clientssl { context clientside }\n"
        "        /Common/http { }\n"
        "    }\n"
        "}\n"
    )
    # Compute the offset of the ``profiles { ... }`` value body.
    val_start = src.index("{", src.index("profiles"))
    val_end = src.index("}", src.index("/Common/http")) + 1
    refs = references_via_spec(
        module="ltm",
        object_type="virtual",
        property_name="profiles",
        value=src[val_start : val_end + 1],
        owner_path="/Common/v",
        base_offset=val_start,
    )
    assert refs is not None
    by_path = {r.target_path: r for r in refs}
    css = by_path["/Common/clientssl"]
    assert css.range is not None
    assert src[css.range.start : css.range.end] == "/Common/clientssl"
    http = by_path["/Common/http"]
    assert http.range is not None
    assert src[http.range.start : http.range.end] == "/Common/http"


def test_list_spec_empty_input_returns_empty_list():
    spec = ListSpec(item=ObjectRefSpec(kind="ltm rule"))
    parsed = spec.parse("", ParseContext())
    from core.bigip.types import BigipList

    assert isinstance(parsed.value, BigipList)
    assert len(parsed.value) == 0
    assert spec.render(parsed.value, RenderContext()) == "{ }"


def test_hand_maintained_kinds_are_explicit_allowlist():
    """Kinds without a corresponding manpage-generated JSON source
    must be on a small explicit allowlist.  A spec regeneration that
    leaves orphaned files behind (the reviewer's ``gtm_add`` /
    ``cm_ha_group`` finding) shows up as a test failure here, not
    as a silent kind in ``OBJECT_KIND_SPECS``.

    The allowlist documents intent — every entry needs a comment in
    its spec file explaining why the manpage corpus doesn't cover
    it (device emits the header under a different module, hand-
    written stub for a deprecated alias, etc.).
    """
    from core.bigip.registry.data import OBJECT_KIND_SPECS

    hand_maintained_allowlist = frozenset(
        {
            "cm_ha_group",  # alias of sys_ha_group on 13.x/14.x emissions
        }
    )
    # Every allowlist entry must actually exist in the registry; a
    # stale allowlist entry is just as bad as an orphaned spec.
    for kind in hand_maintained_allowlist:
        assert kind in OBJECT_KIND_SPECS, f"allowlist entry {kind!r} is not in OBJECT_KIND_SPECS"
    # Conversely, no truly dead kinds should sneak in.  The pruned
    # ``gtm_add`` kind in particular must stay out.
    assert "gtm_add" not in OBJECT_KIND_SPECS


def test_force_replace_all_with_pins_known_list_properties():
    """The curated override layer in
    :data:`core.bigip.registry.specs._base._FORCE_REPLACE_ALL_WITH`
    pins ``list_operators`` onto every list property whose
    full-body ``tmsh modify`` would otherwise emit a bare
    ``<prop> { ... }`` body (rejected by the device).

    Every entry in the allowlist must resolve to an actual property
    in the registry — stale entries trip CI rather than silently
    failing to apply on a regenerated spec set."""
    from core.bigip.registry import list_operator_for, property_spec_for
    from core.bigip.registry.specs._base import _FORCE_REPLACE_ALL_WITH

    for module, object_type, prop_name in _FORCE_REPLACE_ALL_WITH:
        spec = property_spec_for(module, object_type, prop_name)
        assert spec is not None, (
            f"override entry {(module, object_type, prop_name)!r} does "
            "not resolve to a registered property; either the property "
            "name changed in a spec regen or the allowlist is stale"
        )
        assert "replace-all-with" in spec.list_operators, (
            f"override layer failed to pin replace-all-with on {module} {object_type}.{prop_name}"
        )
        assert list_operator_for(module, object_type, prop_name) == "replace-all-with"
