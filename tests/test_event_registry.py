"""Tests for the EventRegistry facade."""

from __future__ import annotations

from core.commands.registry.namespace_data import EVENT_PROPS
from core.commands.registry.namespace_models import EventRequires
from core.commands.registry.namespace_registry import (
    NAMESPACE_REGISTRY as EVENT_REGISTRY,
)
from core.commands.registry.namespace_registry import (
    NamespaceRegistry as EventRegistry,
)


class TestBuildDefault:
    def test_loads_all_events(self):
        assert EVENT_REGISTRY.all_event_names() == frozenset(EVENT_PROPS)

    def test_singleton_is_event_registry(self):
        assert isinstance(EVENT_REGISTRY, EventRegistry)


class TestPropertyQueries:
    def test_get_props_known(self):
        props = EVENT_REGISTRY.get_props("HTTP_REQUEST")
        assert props is not None
        assert props.client_side
        assert "HTTP" in props.implied_profiles

    def test_get_props_unknown(self):
        assert EVENT_REGISTRY.get_props("NONEXISTENT") is None

    def test_is_known(self):
        assert EVENT_REGISTRY.is_known("HTTP_REQUEST")
        assert not EVENT_REGISTRY.is_known("NONEXISTENT")

    def test_is_flow_event(self):
        assert EVENT_REGISTRY.is_flow_event("HTTP_REQUEST")
        assert EVENT_REGISTRY.is_flow_event("CLIENT_ACCEPTED")
        assert not EVENT_REGISTRY.is_flow_event("RULE_INIT")
        assert not EVENT_REGISTRY.is_flow_event("PERSIST_DOWN")

    def test_is_flow_event_unknown_returns_true(self):
        """Unknown events default to flow=True (safe default)."""
        assert EVENT_REGISTRY.is_flow_event("NONEXISTENT")

    def test_non_flow_events(self):
        non_flow = EVENT_REGISTRY.non_flow_events()
        assert "RULE_INIT" in non_flow
        assert "PERSIST_DOWN" in non_flow
        assert "ACCESS_SESSION_CLOSED" in non_flow
        assert "IP_GTM" in non_flow
        assert "HTTP_REQUEST" not in non_flow
        assert "CLIENT_ACCEPTED" not in non_flow

    def test_events_matching_delegates(self):
        matched = EVENT_REGISTRY.events_matching(EventRequires(init_only=True))
        assert matched == ["RULE_INIT"]

    def test_event_satisfies_delegates(self):
        props = EVENT_REGISTRY.get_props("HTTP_REQUEST")
        assert props is not None
        req = EventRequires(client_side=True)
        assert EVENT_REGISTRY.event_satisfies(props, req)


class TestDescriptionQueries:
    def test_get_description(self):
        desc = EVENT_REGISTRY.get_description("HTTP_REQUEST")
        assert desc is not None
        assert len(desc) > 0

    def test_get_description_unknown(self):
        assert EVENT_REGISTRY.get_description("NONEXISTENT") is None

    def test_get_detail(self):
        detail = EVENT_REGISTRY.get_detail("HTTP_REQUEST")
        assert isinstance(detail, str)

    def test_side_label(self):
        label = EVENT_REGISTRY.side_label("HTTP_REQUEST")
        assert "client" in label.lower()

    def test_side_label_unknown(self):
        assert EVENT_REGISTRY.side_label("NONEXISTENT") == "global"

    def test_side_label_short(self):
        label = EVENT_REGISTRY.side_label_short("HTTP_REQUEST")
        assert isinstance(label, str)
        assert len(label) > 0


class TestFlowOrderingQueries:
    def test_event_index(self):
        idx = EVENT_REGISTRY.event_index("CLIENT_ACCEPTED")
        assert idx is not None
        assert isinstance(idx, int)

    def test_event_index_unknown(self):
        assert EVENT_REGISTRY.event_index("NONEXISTENT") is None

    def test_order_events(self):
        events = frozenset({"HTTP_REQUEST", "CLIENT_ACCEPTED", "HTTP_RESPONSE"})
        ordered = EVENT_REGISTRY.order_events(events)
        assert isinstance(ordered, list)
        # CLIENT_ACCEPTED fires before HTTP_REQUEST
        ca_idx = ordered.index("CLIENT_ACCEPTED")
        hr_idx = ordered.index("HTTP_REQUEST")
        assert ca_idx < hr_idx

    def test_events_before(self):
        file_events = frozenset({"CLIENT_ACCEPTED", "HTTP_REQUEST", "HTTP_RESPONSE"})
        before = EVENT_REGISTRY.events_before("HTTP_REQUEST", file_events)
        assert "CLIENT_ACCEPTED" in before
        assert "HTTP_REQUEST" not in before

    def test_events_after(self):
        file_events = frozenset({"CLIENT_ACCEPTED", "HTTP_REQUEST", "HTTP_RESPONSE"})
        after = EVENT_REGISTRY.events_after("HTTP_REQUEST", file_events)
        assert "HTTP_RESPONSE" in after
        assert "HTTP_REQUEST" not in after

    def test_is_per_request(self):
        assert EVENT_REGISTRY.is_per_request("HTTP_REQUEST")
        assert not EVENT_REGISTRY.is_per_request("CLIENT_ACCEPTED")

    def test_is_once_per_connection(self):
        assert EVENT_REGISTRY.is_once_per_connection("CLIENT_ACCEPTED")
        assert not EVENT_REGISTRY.is_once_per_connection("HTTP_REQUEST")


class TestFileLevelHelpers:
    def test_scan_file_events(self):
        src = "when HTTP_REQUEST {\n}\nwhen CLIENT_ACCEPTED {\n}"
        events = EVENT_REGISTRY.scan_file_events(src)
        assert "HTTP_REQUEST" in events
        assert "CLIENT_ACCEPTED" in events

    def test_compute_file_profiles(self):
        src = "when CLIENTSSL_HANDSHAKE {\n    SSL::cert 0\n}"
        profiles = EVENT_REGISTRY.compute_file_profiles(src)
        assert "CLIENTSSL" in profiles
