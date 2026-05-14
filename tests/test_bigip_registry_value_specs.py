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
