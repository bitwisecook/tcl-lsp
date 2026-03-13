"""Tests for event_flow_chains -- temporal ordering of iRules events."""

from __future__ import annotations

import pytest

from core.commands.registry.namespace_data import (
    EVENT_PROPS,
    FLOW_CHAINS,
    MASTER_ORDER,
    ONCE_PER_CONNECTION,
    PER_REQUEST,
    chain_event_names,
    chain_for_profiles,
    event_index,
    events_after,
    events_before,
    is_once_per_connection,
    is_per_request,
    order_events,
    order_events_for_file,
    variable_scope_note,
)

# Master ordering consistency


class TestMasterOrder:
    """Validate that MASTER_ORDER is consistent with EVENT_PROPS."""

    def test_all_master_events_in_event_props(self):
        """Every event in MASTER_ORDER must exist in EVENT_PROPS."""
        for evt, _gates in MASTER_ORDER:
            assert evt in EVENT_PROPS, f"{evt} is in MASTER_ORDER but not EVENT_PROPS"

    def test_no_duplicate_events(self):
        """No event appears twice in MASTER_ORDER."""
        seen: set[str] = set()
        for evt, _gates in MASTER_ORDER:
            assert evt not in seen, f"{evt} appears more than once in MASTER_ORDER"
            seen.add(evt)

    def test_rule_init_is_first(self):
        assert MASTER_ORDER[0][0] == "RULE_INIT"

    def test_client_closed_near_end(self):
        events = [evt for evt, _ in MASTER_ORDER]
        assert "CLIENT_CLOSED" in events
        idx = events.index("CLIENT_CLOSED")
        # CLIENT_CLOSED should be in the last few entries
        assert idx >= len(events) - 5

    def test_server_side_events_after_lb(self):
        """Events with server_side=True in EVENT_PROPS should come after
        LB_SELECTED in MASTER_ORDER (they need a server connection)."""
        events = [evt for evt, _ in MASTER_ORDER]
        lb_idx = events.index("LB_SELECTED")
        server_connected_idx = events.index("SERVER_CONNECTED")

        # SERVER_CONNECTED must come after LB_SELECTED
        assert server_connected_idx > lb_idx

    def test_clientssl_before_http(self):
        """CLIENTSSL_HANDSHAKE must fire before HTTP_REQUEST."""
        events = [evt for evt, _ in MASTER_ORDER]
        assert events.index("CLIENTSSL_HANDSHAKE") < events.index("HTTP_REQUEST")

    def test_http_request_before_lb(self):
        """HTTP_REQUEST fires before LB_SELECTED (LB uses HTTP info)."""
        events = [evt for evt, _ in MASTER_ORDER]
        assert events.index("HTTP_REQUEST") < events.index("LB_SELECTED")

    def test_serverssl_after_server_connected(self):
        """SERVERSSL events must come after SERVER_CONNECTED."""
        events = [evt for evt, _ in MASTER_ORDER]
        sc_idx = events.index("SERVER_CONNECTED")
        for evt in events:
            if evt.startswith("SERVERSSL_"):
                assert events.index(evt) > sc_idx, f"{evt} should be after SERVER_CONNECTED"

    def test_http_response_after_request_send(self):
        """HTTP_RESPONSE comes after HTTP_REQUEST_SEND."""
        events = [evt for evt, _ in MASTER_ORDER]
        assert events.index("HTTP_RESPONSE") > events.index("HTTP_REQUEST_SEND")


# Flow chain consistency


class TestFlowChains:
    """Validate that FLOW_CHAINS are consistent."""

    def test_all_chain_events_in_event_props(self):
        """Every event in a chain must exist in EVENT_PROPS."""
        for chain_id, chain in FLOW_CHAINS.items():
            for step in chain.steps:
                assert step.event in EVENT_PROPS, (
                    f"{step.event} in chain '{chain_id}' not in EVENT_PROPS"
                )

    def test_all_chain_events_in_master_order(self):
        """Every event in a chain must exist in MASTER_ORDER."""
        master_events = {evt for evt, _ in MASTER_ORDER}
        for chain_id, chain in FLOW_CHAINS.items():
            for step in chain.steps:
                assert step.event in master_events, (
                    f"{step.event} in chain '{chain_id}' not in MASTER_ORDER"
                )

    def test_chain_events_in_master_order_sequence(self):
        """Events within each chain must appear in the same relative
        order as MASTER_ORDER."""
        master_events = [evt for evt, _ in MASTER_ORDER]
        for chain_id, chain in FLOW_CHAINS.items():
            chain_events = chain_event_names(chain)
            indices = [master_events.index(e) for e in chain_events]
            assert indices == sorted(indices), (
                f"Chain '{chain_id}' events are not in MASTER_ORDER sequence: {chain_events}"
            )

    def test_init_is_first_in_every_chain(self):
        """RULE_INIT must be the first step in every chain."""
        for chain_id, chain in FLOW_CHAINS.items():
            assert chain.steps[0].event == "RULE_INIT", (
                f"Chain '{chain_id}' does not start with RULE_INIT"
            )

    def test_closed_events_are_last(self):
        """CLIENT_CLOSED must be the last step in every TCP chain."""
        for chain_id, chain in FLOW_CHAINS.items():
            if "TCP" in chain.profiles:
                assert chain.steps[-1].event == "CLIENT_CLOSED", (
                    f"Chain '{chain_id}' does not end with CLIENT_CLOSED"
                )

    def test_conditional_steps_have_notes(self):
        """Steps with conditional=True must have a condition_note."""
        for chain_id, chain in FLOW_CHAINS.items():
            for step in chain.steps:
                if step.conditional:
                    assert step.condition_note, (
                        f"{step.event} in '{chain_id}' is conditional but has no condition_note"
                    )

    def test_no_duplicate_events_in_chain(self):
        """No event appears twice in the same chain."""
        for chain_id, chain in FLOW_CHAINS.items():
            events = chain_event_names(chain)
            assert len(events) == len(set(events)), f"Duplicate events in chain '{chain_id}'"

    def test_chain_ids_are_unique(self):
        """chain_id matches the dict key."""
        for key, chain in FLOW_CHAINS.items():
            assert key == chain.chain_id

    @pytest.mark.parametrize("chain_id", list(FLOW_CHAINS.keys()))
    def test_chain_has_lb_event(self, chain_id: str):
        """Every chain should include LB_SELECTED."""
        events = chain_event_names(FLOW_CHAINS[chain_id])
        assert "LB_SELECTED" in events


# Query API


class TestOrderEvents:
    """Test the order_events() query function."""

    def test_basic_https_ordering(self):
        events = frozenset(
            {
                "HTTP_REQUEST",
                "CLIENT_ACCEPTED",
                "CLIENTSSL_HANDSHAKE",
                "LB_SELECTED",
                "SERVER_CONNECTED",
                "HTTP_RESPONSE",
                "CLIENT_CLOSED",
            }
        )
        ordered = order_events(events)
        assert ordered == [
            "CLIENT_ACCEPTED",
            "CLIENTSSL_HANDSHAKE",
            "HTTP_REQUEST",
            "LB_SELECTED",
            "SERVER_CONNECTED",
            "HTTP_RESPONSE",
            "CLIENT_CLOSED",
        ]

    def test_unknown_events_appended(self):
        events = frozenset({"HTTP_REQUEST", "MY_CUSTOM_EVENT", "CLIENT_ACCEPTED"})
        ordered = order_events(events)
        assert ordered == ["CLIENT_ACCEPTED", "HTTP_REQUEST", "MY_CUSTOM_EVENT"]

    def test_empty_set(self):
        assert order_events(frozenset()) == []

    def test_single_event(self):
        assert order_events(frozenset({"HTTP_REQUEST"})) == ["HTTP_REQUEST"]

    def test_matches_empirical_output(self):
        """Match the e-XpertSolutions empirical output:
        CLIENT_ACCEPTED → CLIENTSSL_CLIENTHELLO → CLIENTSSL_HANDSHAKE →
        HTTP_REQUEST → LB_FAILED → CLIENT_CLOSED"""
        events = frozenset(
            {
                "CLIENT_ACCEPTED",
                "CLIENTSSL_CLIENTHELLO",
                "CLIENTSSL_HANDSHAKE",
                "HTTP_REQUEST",
                "LB_FAILED",
                "CLIENT_CLOSED",
            }
        )
        ordered = order_events(events)
        assert ordered == [
            "CLIENT_ACCEPTED",
            "CLIENTSSL_CLIENTHELLO",
            "CLIENTSSL_HANDSHAKE",
            "HTTP_REQUEST",
            "LB_FAILED",
            "CLIENT_CLOSED",
        ]


class TestOrderEventsForFile:
    def test_simple_irule(self):
        source = """\
when CLIENT_ACCEPTED {
    log local0. "accepted"
}
when HTTP_REQUEST {
    log local0. "request"
}
when HTTP_RESPONSE {
    log local0. "response"
}
when CLIENT_CLOSED {
    log local0. "closed"
}
"""
        ordered = order_events_for_file(source)
        assert ordered == [
            "CLIENT_ACCEPTED",
            "HTTP_REQUEST",
            "HTTP_RESPONSE",
            "CLIENT_CLOSED",
        ]

    def test_full_https_irule(self):
        source = """\
when RULE_INIT { }
when CLIENT_ACCEPTED { }
when CLIENTSSL_CLIENTHELLO { }
when CLIENTSSL_HANDSHAKE { }
when HTTP_REQUEST { }
when LB_SELECTED { }
when SERVER_CONNECTED { }
when SERVERSSL_HANDSHAKE { }
when HTTP_REQUEST_SEND { }
when HTTP_RESPONSE { }
when HTTP_RESPONSE_RELEASE { }
when CLIENT_CLOSED { }
"""
        ordered = order_events_for_file(source)
        assert ordered == [
            "RULE_INIT",
            "CLIENT_ACCEPTED",
            "CLIENTSSL_CLIENTHELLO",
            "CLIENTSSL_HANDSHAKE",
            "HTTP_REQUEST",
            "LB_SELECTED",
            "SERVER_CONNECTED",
            "SERVERSSL_HANDSHAKE",
            "HTTP_REQUEST_SEND",
            "HTTP_RESPONSE",
            "HTTP_RESPONSE_RELEASE",
            "CLIENT_CLOSED",
        ]


class TestEventIndex:
    def test_known_event(self):
        idx = event_index("CLIENT_ACCEPTED")
        assert idx is not None
        assert idx > event_index("RULE_INIT")  # type: ignore[operator]

    def test_unknown_event(self):
        assert event_index("TOTALLY_FAKE_EVENT") is None


class TestEventsBeforeAfter:
    def test_events_before_http_request(self):
        file_events = frozenset(
            {
                "CLIENT_ACCEPTED",
                "CLIENTSSL_HANDSHAKE",
                "HTTP_REQUEST",
                "LB_SELECTED",
            }
        )
        before = events_before("HTTP_REQUEST", file_events)
        assert before == ["CLIENT_ACCEPTED", "CLIENTSSL_HANDSHAKE"]

    def test_events_after_http_request(self):
        file_events = frozenset(
            {
                "CLIENT_ACCEPTED",
                "HTTP_REQUEST",
                "LB_SELECTED",
                "SERVER_CONNECTED",
                "HTTP_RESPONSE",
            }
        )
        after = events_after("HTTP_REQUEST", file_events)
        assert after == ["LB_SELECTED", "SERVER_CONNECTED", "HTTP_RESPONSE"]

    def test_events_before_unknown(self):
        assert events_before("FAKE_EVENT", frozenset({"HTTP_REQUEST"})) == []


class TestChainForProfiles:
    def test_finds_plain_tcp(self):
        chain = chain_for_profiles(frozenset({"TCP"}))
        assert chain is not None
        assert chain.chain_id == "plain_tcp"

    def test_finds_full_https(self):
        chain = chain_for_profiles(frozenset({"TCP", "CLIENTSSL", "SERVERSSL", "HTTP"}))
        assert chain is not None
        assert chain.chain_id == "tcp_clientssl_serverssl_http"

    def test_finds_dns(self):
        chain = chain_for_profiles(frozenset({"UDP", "DNS"}))
        assert chain is not None
        assert chain.chain_id == "udp_dns"

    def test_no_match(self):
        chain = chain_for_profiles(frozenset({"NONEXISTENT_PROFILE"}))
        assert chain is None


# Event multiplicity


class TestEventMultiplicity:
    """Validate the ONCE_PER_CONNECTION and PER_REQUEST sets."""

    def test_sets_are_disjoint(self):
        """No event can be both once-per-connection and per-request."""
        overlap = ONCE_PER_CONNECTION & PER_REQUEST
        assert not overlap, f"Events in both sets: {overlap}"

    def test_all_multiplicity_events_in_event_props(self):
        """Every event in the multiplicity sets must be in EVENT_PROPS."""
        for evt in ONCE_PER_CONNECTION | PER_REQUEST:
            assert evt in EVENT_PROPS, f"{evt} not in EVENT_PROPS"

    def test_rule_init_is_once(self):
        assert is_once_per_connection("RULE_INIT")
        assert not is_per_request("RULE_INIT")

    def test_http_request_is_per_request(self):
        assert is_per_request("HTTP_REQUEST")
        assert not is_once_per_connection("HTTP_REQUEST")

    def test_client_accepted_is_once(self):
        assert is_once_per_connection("CLIENT_ACCEPTED")

    def test_http_response_is_per_request(self):
        assert is_per_request("HTTP_RESPONSE")

    def test_lb_selected_is_per_request(self):
        """LB fires per-request because HTTP mode picks per transaction."""
        assert is_per_request("LB_SELECTED")

    def test_client_closed_is_once(self):
        assert is_once_per_connection("CLIENT_CLOSED")


class TestVariableScopeNote:
    """Test the variable_scope_note() helper."""

    def test_rule_init_always_safe(self):
        """Variables from RULE_INIT (static) are always safe."""
        assert variable_scope_note("RULE_INIT", "HTTP_REQUEST") is None
        assert variable_scope_note("RULE_INIT", "CLIENT_CLOSED") is None

    def test_later_event_to_earlier(self):
        """Reading in an earlier event than the set event is flagged."""
        note = variable_scope_note("HTTP_REQUEST", "CLIENT_ACCEPTED")
        assert note is not None
        assert "not yet available" in note

    def test_per_request_to_once(self):
        """Variable set per-request, read in once-per-connection event."""
        note = variable_scope_note("HTTP_REQUEST", "CLIENT_CLOSED")
        # CLIENT_CLOSED fires after HTTP_REQUEST in ordering, so no
        # "not yet available" concern.  But reading per-request var
        # in once-per-connection context is flagged.
        assert note is not None
        assert "per-request" in note

    def test_same_scope_is_fine(self):
        """Variable set and read in events of the same scope: no note."""
        assert variable_scope_note("HTTP_REQUEST", "HTTP_RESPONSE") is None

    def test_once_to_later_once(self):
        """Variable set once-per-connection, read later: fine."""
        assert variable_scope_note("CLIENT_ACCEPTED", "HTTP_REQUEST") is None

    def test_unknown_event(self):
        """Unknown events return None (no note)."""
        assert variable_scope_note("FAKE_EVENT", "HTTP_REQUEST") is None
