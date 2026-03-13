"""Tests for event tree: EventProps, EventRequires, validation, and profiles."""

from __future__ import annotations

from core.commands.registry.namespace_data import (
    EVENT_PROPS,
    compute_file_profiles,
    event_satisfies,
    events_matching,
    get_event_props,
    infer_profiles_from_events,
    missing_requirements_description,
    parse_profile_directive,
    profile_info_description,
    scan_file_events,
)
from core.commands.registry.namespace_models import EventRequires

# EVENT_PROPS table coverage


class TestEventPropsTable:
    """Verify the EVENT_PROPS table has correct properties for key events."""

    def test_rule_init_has_nothing(self):
        props = EVENT_PROPS["RULE_INIT"]
        assert not props.client_side
        assert not props.server_side
        assert props.transport is None
        assert not props.implied_profiles

    def test_client_accepted(self):
        props = EVENT_PROPS["CLIENT_ACCEPTED"]
        assert props.client_side
        assert not props.server_side
        assert props.transport == "tcp"

    def test_server_connected_has_both_sides(self):
        props = EVENT_PROPS["SERVER_CONNECTED"]
        assert props.client_side
        assert props.server_side
        assert props.transport == "tcp"

    def test_http_request_implies_http_profile(self):
        props = EVENT_PROPS["HTTP_REQUEST"]
        assert props.client_side
        assert not props.server_side
        assert props.transport == "tcp"
        assert "HTTP" in props.implied_profiles
        assert "FASTHTTP" in props.implied_profiles

    def test_http_request_send_is_both_sides(self):
        """HTTP_REQUEST_SEND is on the clientside of the proxy but server-side
        connection is available (LB has happened)."""
        props = EVENT_PROPS["HTTP_REQUEST_SEND"]
        assert props.client_side
        assert props.server_side

    def test_http_request_release_is_both_sides(self):
        """HTTP_REQUEST_RELEASE is on the serverside of the proxy."""
        props = EVENT_PROPS["HTTP_REQUEST_RELEASE"]
        assert props.client_side
        assert props.server_side

    def test_clientssl_handshake_implies_clientssl(self):
        props = EVENT_PROPS["CLIENTSSL_HANDSHAKE"]
        assert props.client_side
        assert not props.server_side
        assert props.transport == "tcp"
        assert "CLIENTSSL" in props.implied_profiles

    def test_serverssl_handshake_implies_serverssl(self):
        props = EVENT_PROPS["SERVERSSL_HANDSHAKE"]
        assert props.client_side
        assert props.server_side
        assert "SERVERSSL" in props.implied_profiles

    def test_dns_request_is_udp(self):
        props = EVENT_PROPS["DNS_REQUEST"]
        assert props.transport == "udp"
        assert "DNS" in props.implied_profiles

    def test_lb_selected_is_client_only(self):
        props = EVENT_PROPS["LB_SELECTED"]
        assert props.client_side
        assert not props.server_side
        assert props.transport == "tcp"

    def test_ip_gtm_has_no_connection(self):
        """IP_GTM is a GTM event — no standard connection context."""
        props = EVENT_PROPS["IP_GTM"]
        assert not props.client_side
        assert not props.server_side

    # Flow property (K14320)

    def test_rule_init_is_non_flow(self):
        assert not EVENT_PROPS["RULE_INIT"].flow

    def test_persist_down_is_non_flow(self):
        assert not EVENT_PROPS["PERSIST_DOWN"].flow

    def test_access_session_closed_is_non_flow(self):
        assert not EVENT_PROPS["ACCESS_SESSION_CLOSED"].flow

    def test_ip_gtm_is_non_flow(self):
        assert not EVENT_PROPS["IP_GTM"].flow

    def test_client_accepted_is_flow(self):
        assert EVENT_PROPS["CLIENT_ACCEPTED"].flow

    def test_http_request_is_flow(self):
        assert EVENT_PROPS["HTTP_REQUEST"].flow

    def test_server_connected_is_flow(self):
        assert EVENT_PROPS["SERVER_CONNECTED"].flow

    def test_get_event_props_known(self):
        assert get_event_props("HTTP_REQUEST") is EVENT_PROPS["HTTP_REQUEST"]

    def test_get_event_props_unknown(self):
        assert get_event_props("NONEXISTENT_EVENT") is None


# EventRequires matching


class TestEventSatisfies:
    def test_no_requirements_matches_anything(self):
        req = EventRequires()
        assert event_satisfies(EVENT_PROPS["RULE_INIT"], req)
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)

    def test_client_side_requirement(self):
        req = EventRequires(client_side=True)
        assert event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req)

    def test_server_side_requirement(self):
        req = EventRequires(server_side=True)
        assert event_satisfies(EVENT_PROPS["SERVER_CONNECTED"], req)
        assert event_satisfies(EVENT_PROPS["HTTP_RESPONSE"], req)
        assert not event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req)

    def test_transport_tcp(self):
        req = EventRequires(client_side=True, transport="tcp")
        assert event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)
        assert not event_satisfies(EVENT_PROPS["DNS_REQUEST"], req)

    def test_transport_udp(self):
        req = EventRequires(client_side=True, transport="udp")
        assert event_satisfies(EVENT_PROPS["DNS_REQUEST"], req)
        assert not event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)

    def test_profile_requirement(self):
        req = EventRequires(profiles=frozenset({"HTTP", "FASTHTTP"}))
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)
        assert not event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)

    def test_ssl_profile_requirement(self):
        req = EventRequires(profiles=frozenset({"CLIENTSSL", "SERVERSSL"}))
        assert event_satisfies(EVENT_PROPS["CLIENTSSL_HANDSHAKE"], req)
        assert event_satisfies(EVENT_PROPS["SERVERSSL_HANDSHAKE"], req)
        assert not event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)

    def test_combined_requirements(self):
        req = EventRequires(
            client_side=True,
            transport="tcp",
            profiles=frozenset({"HTTP", "FASTHTTP"}),
        )
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)
        assert not event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)
        assert not event_satisfies(EVENT_PROPS["DNS_REQUEST"], req)

    def test_init_only_matches_rule_init(self):
        req = EventRequires(init_only=True)
        assert event_satisfies(EVENT_PROPS["RULE_INIT"], req, event_name="RULE_INIT")
        assert not event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req, event_name="HTTP_REQUEST")
        assert not event_satisfies(
            EVENT_PROPS["CLIENT_ACCEPTED"], req, event_name="CLIENT_ACCEPTED"
        )

    def test_init_only_requires_event_name(self):
        req = EventRequires(init_only=True)
        # Without event_name, can't match
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req)

    def test_also_in_unconditional_match(self):
        req = EventRequires(
            profiles=frozenset({"MQTT"}),
            also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
        )
        # Profile-based match
        assert event_satisfies(
            EVENT_PROPS["MQTT_CLIENT_INGRESS"], req, event_name="MQTT_CLIENT_INGRESS"
        )
        # also_in match (CLIENT_ACCEPTED has no MQTT profile)
        assert event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req, event_name="CLIENT_ACCEPTED")
        assert event_satisfies(EVENT_PROPS["SERVER_CONNECTED"], req, event_name="SERVER_CONNECTED")
        # Neither profile nor also_in
        assert not event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req, event_name="HTTP_REQUEST")
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req, event_name="RULE_INIT")

    def test_also_in_needs_event_name(self):
        req = EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"}))
        # Without event_name, also_in can't match; falls through to
        # normal checks which all pass (no requirements set).
        assert event_satisfies(EVENT_PROPS["RULE_INIT"], req)

    def test_multi_profile_or_semantics(self):
        """profiles uses OR — event matches if ANY of its profiles overlap."""
        req = EventRequires(profiles=frozenset({"DIAMETER", "MR"}))
        assert event_satisfies(EVENT_PROPS["DIAMETER_INGRESS"], req, event_name="DIAMETER_INGRESS")
        assert event_satisfies(EVENT_PROPS["MR_INGRESS"], req, event_name="MR_INGRESS")
        assert not event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req, event_name="HTTP_REQUEST")

    # Flow requirement (K14320)

    def test_flow_requirement_blocks_non_flow_event(self):
        req = EventRequires(flow=True)
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req)
        assert not event_satisfies(EVENT_PROPS["PERSIST_DOWN"], req)
        assert not event_satisfies(EVENT_PROPS["ACCESS_SESSION_CLOSED"], req)
        assert not event_satisfies(EVENT_PROPS["IP_GTM"], req)

    def test_flow_requirement_passes_flow_event(self):
        req = EventRequires(flow=True)
        assert event_satisfies(EVENT_PROPS["CLIENT_ACCEPTED"], req)
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)
        assert event_satisfies(EVENT_PROPS["SERVER_CONNECTED"], req)

    def test_no_flow_requirement_passes_anywhere(self):
        """Commands without flow=True work in all events."""
        req = EventRequires()
        assert event_satisfies(EVENT_PROPS["RULE_INIT"], req)
        assert event_satisfies(EVENT_PROPS["HTTP_REQUEST"], req)
        assert event_satisfies(EVENT_PROPS["ACCESS_SESSION_CLOSED"], req)

    def test_also_in_bypasses_flow_check(self):
        """also_in is checked before flow — persist/session work in PERSIST_DOWN."""
        req = EventRequires(
            client_side=True,
            flow=True,
            also_in=frozenset({"PERSIST_DOWN"}),
        )
        # PERSIST_DOWN is non-flow but in also_in — should pass
        assert event_satisfies(EVENT_PROPS["PERSIST_DOWN"], req, event_name="PERSIST_DOWN")
        # RULE_INIT is non-flow and NOT in also_in — should fail
        assert not event_satisfies(EVENT_PROPS["RULE_INIT"], req, event_name="RULE_INIT")

    def test_events_matching_init_only(self):
        req = EventRequires(init_only=True)
        assert events_matching(req) == ["RULE_INIT"]

    def test_events_matching_also_in(self):
        req = EventRequires(
            profiles=frozenset({"FIX"}),
            also_in=frozenset({"RULE_INIT"}),
        )
        matched = events_matching(req)
        assert "FIX_HEADER" in matched
        assert "FIX_MESSAGE" in matched
        assert "RULE_INIT" in matched
        assert "HTTP_REQUEST" not in matched


# Requirement descriptions


class TestMissingRequirementsDescription:
    def test_client_side_missing(self):
        req = EventRequires(client_side=True)
        props = EVENT_PROPS["RULE_INIT"]
        desc = missing_requirements_description("RULE_INIT", props, req)
        assert "client-side" in desc

    def test_transport_mismatch(self):
        req = EventRequires(client_side=True, transport="tcp")
        props = EVENT_PROPS["DNS_REQUEST"]
        desc = missing_requirements_description("DNS_REQUEST", props, req)
        assert "tcp" in desc

    def test_profile_missing(self):
        req = EventRequires(profiles=frozenset({"HTTP"}))
        props = EVENT_PROPS["CLIENT_ACCEPTED"]
        desc = missing_requirements_description("CLIENT_ACCEPTED", props, req)
        assert "HTTP" in desc

    def test_flow_missing(self):
        req = EventRequires(flow=True)
        props = EVENT_PROPS["RULE_INIT"]
        desc = missing_requirements_description("RULE_INIT", props, req)
        assert "non-flow" in desc


class TestProfileInfoDescription:
    def test_no_profiles_required(self):
        req = EventRequires(client_side=True)
        assert profile_info_description(req, frozenset()) is None

    def test_profiles_covered(self):
        req = EventRequires(profiles=frozenset({"HTTP"}))
        assert profile_info_description(req, frozenset({"HTTP"})) is None

    def test_profiles_unconfirmed(self):
        req = EventRequires(profiles=frozenset({"HTTP"}))
        desc = profile_info_description(req, frozenset())
        assert desc is not None
        assert "HTTP" in desc

    def test_partial_coverage_suppresses(self):
        req = EventRequires(profiles=frozenset({"HTTP", "FASTHTTP"}))
        assert profile_info_description(req, frozenset({"HTTP"})) is None


# events_matching


class TestEventsMatching:
    def test_empty_requirements_matches_all(self):
        matched = events_matching(EventRequires())
        assert "RULE_INIT" in matched
        assert "HTTP_REQUEST" in matched
        assert len(matched) == len(EVENT_PROPS)

    def test_client_side_excludes_rule_init(self):
        matched = events_matching(EventRequires(client_side=True))
        assert "RULE_INIT" not in matched
        assert "CLIENT_ACCEPTED" in matched
        assert "HTTP_REQUEST" in matched

    def test_server_side_excludes_client_only(self):
        matched = events_matching(EventRequires(server_side=True))
        assert "CLIENT_ACCEPTED" not in matched
        assert "SERVER_CONNECTED" in matched

    def test_http_profile_requirement(self):
        matched = events_matching(EventRequires(profiles=frozenset({"HTTP", "FASTHTTP"})))
        assert "HTTP_REQUEST" in matched
        assert "CLIENT_ACCEPTED" not in matched

    def test_flow_excludes_non_flow_events(self):
        matched = events_matching(EventRequires(flow=True))
        assert "CLIENT_ACCEPTED" in matched
        assert "HTTP_REQUEST" in matched
        assert "RULE_INIT" not in matched
        assert "PERSIST_DOWN" not in matched
        assert "ACCESS_SESSION_CLOSED" not in matched
        assert "IP_GTM" not in matched


# Profile directive


class TestParseProfileDirective:
    def test_basic(self):
        src = "# profiles: HTTP CLIENTSSL\nwhen HTTP_REQUEST { }"
        assert parse_profile_directive(src) == frozenset({"HTTP", "CLIENTSSL"})

    def test_case_insensitive(self):
        src = "# Profiles: http clientssl\n"
        assert parse_profile_directive(src) == frozenset({"HTTP", "CLIENTSSL"})

    def test_no_directive(self):
        src = "when HTTP_REQUEST {\n    set x 1\n}"
        assert parse_profile_directive(src) == frozenset()

    def test_stops_at_first_non_comment(self):
        src = "when HTTP_REQUEST { }\n# profiles: HTTP\n"
        assert parse_profile_directive(src) == frozenset()

    def test_multiple_comment_lines(self):
        src = "# This is a header\n# profiles: HTTP SERVERSSL\n# More comments\n"
        assert parse_profile_directive(src) == frozenset({"HTTP", "SERVERSSL"})

    def test_singular_profile(self):
        src = "# profile: DNS\n"
        assert parse_profile_directive(src) == frozenset({"DNS"})

    def test_comma_separated(self):
        src = "# Profiles: HTTP, CLIENTSSL\nwhen HTTP_REQUEST { }"
        assert parse_profile_directive(src) == frozenset({"HTTP", "CLIENTSSL"})

    def test_comma_separated_no_spaces(self):
        src = "# Profiles: HTTP,CLIENTSSL,DNS\n"
        assert parse_profile_directive(src) == frozenset({"HTTP", "CLIENTSSL", "DNS"})

    def test_mixed_comma_and_space(self):
        src = "# profiles: HTTP, CLIENTSSL SERVERSSL\n"
        assert parse_profile_directive(src) == frozenset({"HTTP", "CLIENTSSL", "SERVERSSL"})


# Profile inference from events


class TestScanFileEvents:
    def test_basic(self):
        src = "when HTTP_REQUEST {\n}\nwhen CLIENTSSL_HANDSHAKE {\n}"
        events = scan_file_events(src)
        assert "HTTP_REQUEST" in events
        assert "CLIENTSSL_HANDSHAKE" in events

    def test_no_events(self):
        src = "set x 1\nlog local0. hello\n"
        assert scan_file_events(src) == frozenset()


class TestInferProfilesFromEvents:
    def test_clientssl_event(self):
        events = frozenset({"CLIENTSSL_HANDSHAKE", "HTTP_REQUEST"})
        profiles = infer_profiles_from_events(events)
        assert "CLIENTSSL" in profiles
        assert "HTTP" in profiles or "FASTHTTP" in profiles

    def test_serverssl_event(self):
        profiles = infer_profiles_from_events(frozenset({"SERVERSSL_HANDSHAKE"}))
        assert "SERVERSSL" in profiles

    def test_unknown_event_ignored(self):
        profiles = infer_profiles_from_events(frozenset({"FAKE_EVENT"}))
        assert profiles == frozenset()


class TestComputeFileProfiles:
    def test_combines_directive_and_inference(self):
        src = "# profiles: SERVERSSL\nwhen CLIENTSSL_HANDSHAKE {\n}\nwhen HTTP_REQUEST {\n}"
        profiles = compute_file_profiles(src)
        assert "SERVERSSL" in profiles
        assert "CLIENTSSL" in profiles
        assert "HTTP" in profiles or "FASTHTTP" in profiles

    def test_directive_only(self):
        src = "# profiles: HTTP CLIENTSSL\nwhen CLIENT_ACCEPTED { }"
        profiles = compute_file_profiles(src)
        assert "HTTP" in profiles
        assert "CLIENTSSL" in profiles

    def test_inference_only(self):
        src = "when CLIENTSSL_HANDSHAKE {\n    SSL::cert 0\n}\n"
        profiles = compute_file_profiles(src)
        assert "CLIENTSSL" in profiles
