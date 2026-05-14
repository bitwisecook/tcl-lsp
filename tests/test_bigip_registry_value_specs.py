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
    StringSpec,
)

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
    assert spec.render("hello", None) == "hello"  # type: ignore[arg-type]


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
    assert spec.render(True, None) == "enabled"  # type: ignore[arg-type]
    assert spec.render(False, None) == "disabled"  # type: ignore[arg-type]
    yes_no = BoolSpec(style="yes")
    assert yes_no.render(True, None) == "yes"  # type: ignore[arg-type]
    assert yes_no.render(False, None) == "no"  # type: ignore[arg-type]


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
