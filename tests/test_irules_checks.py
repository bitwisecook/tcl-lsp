"""Tests for iRules-specific event-aware diagnostics."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol.types import MarkupContent

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity
from core.commands.registry.runtime import configure_signatures
from core.compiler.irules_flow import find_irules_flow_warnings
from lsp.features.completion import get_completions
from lsp.features.hover import get_hover


def _diag_with_code(source: str, code: str):
    """Return all diagnostics matching a specific code."""
    configure_signatures(dialect="f5-irules")
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


# IRULE1001: Command invalid/ineffective in this event


class TestIrule1001:
    """IRULE1001: command in wrong event."""

    def test_http_respond_in_http_request(self):
        """HTTP::respond is valid in HTTP_REQUEST — no warning."""
        src = 'when HTTP_REQUEST {\n    HTTP::respond 200 content "ok"\n}'
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_http_respond_in_client_accepted(self):
        """HTTP::respond is NOT valid in CLIENT_ACCEPTED — warning."""
        src = 'when CLIENT_ACCEPTED {\n    HTTP::respond 200 content "ok"\n}'
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "HTTP::respond" in diags[0].message
        assert "CLIENT_ACCEPTED" in diags[0].message

    def test_table_in_rule_init_warns(self):
        """table requires flow — should warn in RULE_INIT (non-flow)."""
        src = "when RULE_INIT {\n    table set mykey myval\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "table" in diags[0].message

    # K14320: commands invalid in non-flow events

    def test_table_in_access_session_closed_warns(self):
        """table requires flow — should warn in ACCESS_SESSION_CLOSED."""
        src = "when ACCESS_SESSION_CLOSED {\n    table set mykey myval\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "table" in diags[0].message

    def test_connect_in_rule_init_warns(self):
        """connect requires flow — should warn in RULE_INIT."""
        src = "when RULE_INIT {\n    connect\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "connect" in diags[0].message

    def test_lb_select_in_rule_init_warns(self):
        """LB::select requires flow — should warn in RULE_INIT."""
        src = "when RULE_INIT {\n    LB::select pool mypool\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "LB::select" in diags[0].message

    def test_resolv_lookup_in_rule_init_warns(self):
        """RESOLV::lookup requires flow — should warn in RULE_INIT."""
        src = "when RULE_INIT {\n    RESOLV::lookup @192.168.1.1 a example.com\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "RESOLV::lookup" in diags[0].message

    def test_persist_in_persist_down_no_warning(self):
        """persist has also_in={PERSIST_DOWN} — should NOT warn."""
        src = "when PERSIST_DOWN {\n    persist uie mykey\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_session_in_persist_down_no_warning(self):
        """session has also_in={PERSIST_DOWN} — should NOT warn."""
        src = "when PERSIST_DOWN {\n    session add uie mykey myval\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_session_in_access_session_closed_warns(self):
        """session requires client_side + flow — warns in ACCESS_SESSION_CLOSED (non-flow)."""
        src = "when ACCESS_SESSION_CLOSED {\n    session add uie [IP::client_addr] myval\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert "session" in diags[0].message

    def test_table_in_http_request_no_warning(self):
        """table is valid in HTTP_REQUEST (flow event) — no warning."""
        src = "when HTTP_REQUEST {\n    table set mykey myval\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_connect_in_client_accepted_no_warning(self):
        """connect is valid in CLIENT_ACCEPTED (flow event) — no warning."""
        src = "when CLIENT_ACCEPTED {\n    connect\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_command_outside_when_no_warning(self):
        """Commands outside when blocks should not trigger IRULE1001."""
        src = 'HTTP::respond 200 content "ok"'
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_any_event_command_no_warning(self):
        """Commands valid in ANY_EVENT should never warn."""
        src = "when CLIENT_ACCEPTED {\n    IP::idle_timeout 300\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_no_valid_events_no_warning(self):
        """Commands with no valid_events metadata should not warn."""
        src = "when HTTP_REQUEST {\n    set x 1\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_severity_is_warning(self):
        src = "when CLIENT_ACCEPTED {\n    HTTP::respond 200\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 1
        assert diags[0].severity is Severity.WARNING

    def test_tcp_collect_in_client_data_no_warning(self):
        src = "when CLIENT_DATA {\n    TCP::collect\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0

    def test_ssl_release_in_clientssl_data_no_warning(self):
        src = "when CLIENTSSL_DATA {\n    SSL::release\n}"
        diags = _diag_with_code(src, "IRULE1001")
        assert len(diags) == 0


# IRULE2001: Deprecated matchclass


class TestIrule2001:
    """IRULE2001: deprecated matchclass."""

    def test_matchclass_warns(self):
        src = "when HTTP_REQUEST {\n    matchclass [HTTP::uri] equals my_class\n}"
        diags = _diag_with_code(src, "IRULE2001")
        assert len(diags) == 1
        assert "matchclass" in diags[0].message
        assert "class match" in diags[0].message

    def test_matchclass_has_fix(self):
        src = "when HTTP_REQUEST {\n    matchclass [HTTP::uri] my_class\n}"
        diags = _diag_with_code(src, "IRULE2001")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert "class match" in fix.new_text

    def test_no_matchclass_no_warning(self):
        src = "when HTTP_REQUEST {\n    class match [HTTP::uri] equals my_class\n}"
        diags = _diag_with_code(src, "IRULE2001")
        assert len(diags) == 0


# IRULE2101: Heavy regex in hot event


class TestIrule2101:
    """IRULE2101: regexp in hot event."""

    def test_regexp_in_http_request(self):
        src = "when HTTP_REQUEST {\n    regexp {^/api/} [HTTP::uri]\n}"
        diags = _diag_with_code(src, "IRULE2101")
        assert len(diags) == 1
        assert "regexp" in diags[0].message
        assert diags[0].severity is Severity.HINT

    def test_regexp_in_rule_init_no_warning(self):
        """regexp in RULE_INIT is not a hot event — no warning."""
        src = 'when RULE_INIT {\n    regexp {test} "test string"\n}'
        diags = _diag_with_code(src, "IRULE2101")
        assert len(diags) == 0

    def test_regexp_outside_when_no_warning(self):
        src = 'regexp {test} "test string"'
        diags = _diag_with_code(src, "IRULE2101")
        assert len(diags) == 0


# IRULE5001: Ungated log in hot event


class TestIrule5001:
    """IRULE5001: ungated log in hot event."""

    def test_log_in_http_request(self):
        src = 'when HTTP_REQUEST {\n    log local0. "request"\n}'
        diags = _diag_with_code(src, "IRULE5001")
        assert len(diags) == 1
        assert "log" in diags[0].message
        assert diags[0].severity is Severity.HINT

    def test_log_in_rule_init_no_warning(self):
        src = 'when RULE_INIT {\n    log local0. "init"\n}'
        diags = _diag_with_code(src, "IRULE5001")
        assert len(diags) == 0

    def test_log_outside_when_no_warning(self):
        src = 'log local0. "test"'
        diags = _diag_with_code(src, "IRULE5001")
        assert len(diags) == 0


# Example regressions


class TestIruleExamples:
    """Regression checks for shipped iRules examples."""

    def test_cookbook_tcp_collect_has_no_warnings(self):
        configure_signatures(dialect="f5-irules")
        example = Path(__file__).resolve().parent.parent / "samples" / "irules" / "cookbook_tcp_collect.irul"
        src = example.read_text()
        result = analyse(src)
        warnings = [d for d in result.diagnostics if d.severity is Severity.WARNING]
        assert warnings == []


# IRULE1201: HTTP command after response committed


def _flow_with_code(source: str, code: str):
    """Return iRules flow warnings matching a specific code."""
    configure_signatures(dialect="f5-irules")
    return [w for w in find_irules_flow_warnings(source) if w.code == code]


class TestIrule1201:
    """IRULE1201: HTTP command after respond/redirect."""

    def test_http_header_after_respond(self):
        src = (
            "when HTTP_REQUEST {\n"
            '    HTTP::respond 200 content "ok"\n'
            '    HTTP::header insert X-Debug "yes"\n'
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 1
        assert "HTTP::header" in warnings[0].message

    def test_respond_alone_no_warning(self):
        src = "when HTTP_REQUEST {\n    HTTP::respond 200\n}"
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 0

    def test_redirect_then_header(self):
        src = (
            "when HTTP_REQUEST {\n"
            '    HTTP::redirect "https://example.com"\n'
            "    HTTP::header insert X-Foo bar\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 1
        assert "HTTP::header" in warnings[0].message

    def test_header_after_conditional_respond_warns(self):
        src = (
            "when HTTP_REQUEST {\n"
            "    if {$debug} {\n"
            "        HTTP::respond 200\n"
            "    }\n"
            "    HTTP::header insert X-Debug 1\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 1
        assert "HTTP::header" in warnings[0].message

    def test_non_http_event_no_warning(self):
        src = "when CLIENT_ACCEPTED {\n    set x 1\n    set y 2\n}"
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 0

    def test_return_resets_state(self):
        src = (
            "when HTTP_REQUEST {\n"
            "    HTTP::respond 200\n"
            "    return\n"
            '    HTTP::header insert X-Debug "yes"\n'
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1201")
        assert len(warnings) == 0


# IRULE1202: Multiple respond/redirect calls


class TestIrule1202:
    """IRULE1202: multiple respond/redirect possible."""

    def test_double_respond(self):
        src = "when HTTP_REQUEST {\n    HTTP::respond 200\n    HTTP::respond 302\n}"
        warnings = _flow_with_code(src, "IRULE1202")
        assert len(warnings) == 1
        assert "Multiple" in warnings[0].message

    def test_respond_then_redirect(self):
        src = (
            "when HTTP_REQUEST {\n"
            "    HTTP::respond 200\n"
            '    HTTP::redirect "https://example.com"\n'
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1202")
        assert len(warnings) == 1

    def test_single_respond_no_warning(self):
        src = "when HTTP_REQUEST {\n    HTTP::respond 200\n}"
        warnings = _flow_with_code(src, "IRULE1202")
        assert len(warnings) == 0


# IRULE1005: *_DATA event without matching *::collect


class TestIrule1005:
    """IRULE1005: DATA event without matching collect."""

    def test_client_data_without_collect(self):
        """CLIENT_DATA without TCP::collect → warning."""
        src = "when CLIENT_DATA {\n    set data [TCP::payload]\n}"
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 1
        assert "CLIENT_DATA" in warnings[0].message
        assert "TCP::collect" in warnings[0].message

    def test_client_data_with_tcp_collect(self):
        """CLIENT_DATA with TCP::collect elsewhere → no warning."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            "    TCP::collect\n"
            "}\n"
            "when CLIENT_DATA {\n"
            "    set data [TCP::payload]\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_server_data_without_collect(self):
        """SERVER_DATA without TCP::collect → warning."""
        src = "when SERVER_DATA {\n    TCP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 1
        assert "SERVER_DATA" in warnings[0].message

    def test_server_data_with_collect(self):
        """SERVER_DATA with TCP::collect in SERVER_CONNECTED → no warning."""
        src = (
            "when SERVER_CONNECTED {\n    TCP::collect\n}\nwhen SERVER_DATA {\n    TCP::payload\n}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_client_data_with_server_side_collect_still_warns(self):
        """SERVER_CONNECTED TCP::collect should not satisfy CLIENT_DATA."""
        src = (
            "when SERVER_CONNECTED {\n    TCP::collect\n}\nwhen CLIENT_DATA {\n    TCP::payload\n}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 1
        assert "CLIENT_DATA" in warnings[0].message

    def test_client_data_with_clientside_collect_in_server_event(self):
        """clientside { TCP::collect } in SERVER_CONNECTED should satisfy CLIENT_DATA."""
        src = (
            "when SERVER_CONNECTED {\n"
            "    clientside { TCP::collect }\n"
            "}\n"
            "when CLIENT_DATA {\n"
            "    TCP::payload\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_http_request_data_without_collect(self):
        """HTTP_REQUEST_DATA without HTTP::collect → warning."""
        src = "when HTTP_REQUEST_DATA {\n    HTTP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 1
        assert "HTTP::collect" in warnings[0].message

    def test_http_request_data_with_collect(self):
        """HTTP_REQUEST_DATA with HTTP::collect → no warning."""
        src = (
            "when HTTP_REQUEST {\n"
            "    HTTP::collect 1048576\n"
            "}\n"
            "when HTTP_REQUEST_DATA {\n"
            "    HTTP::payload\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_collect_inside_if(self):
        """Collect nested inside an if block should still count."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            "    if {[TCP::local_port] == 80} {\n"
            "        TCP::collect\n"
            "    }\n"
            "}\n"
            "when CLIENT_DATA {\n"
            "    TCP::payload\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_udp_collect_satisfies_client_data(self):
        """UDP::collect should satisfy CLIENT_DATA requirement."""
        src = "when CLIENT_ACCEPTED {\n    UDP::collect\n}\nwhen CLIENT_DATA {\n    UDP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_non_data_event_no_warning(self):
        """Non-DATA events should not trigger IRULE1005."""
        src = "when HTTP_REQUEST {\n    HTTP::uri\n}"
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 0

    def test_ssl_data_events(self):
        """CLIENTSSL_DATA and SERVERSSL_DATA need SSL::collect."""
        src = (
            "when CLIENTSSL_DATA {\n    SSL::payload\n}\nwhen SERVERSSL_DATA {\n    SSL::payload\n}"
        )
        warnings = _flow_with_code(src, "IRULE1005")
        assert len(warnings) == 2


# IRULE1006: *::payload without matching *::collect


class TestIrule1006:
    """IRULE1006: payload access without matching collect."""

    def test_tcp_payload_without_collect(self):
        """TCP::payload without TCP::collect → warning."""
        src = "when CLIENT_DATA {\n    TCP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 1
        assert "TCP::payload" in warnings[0].message
        assert "TCP::collect" in warnings[0].message

    def test_tcp_payload_with_collect(self):
        """TCP::payload with TCP::collect elsewhere → no warning."""
        src = "when CLIENT_ACCEPTED {\n    TCP::collect\n}\nwhen CLIENT_DATA {\n    TCP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 0

    def test_tcp_payload_with_only_server_side_collect_warns(self):
        """Server-side collect should not satisfy payload read on client side."""
        src = (
            "when SERVER_CONNECTED {\n    TCP::collect\n}\nwhen CLIENT_DATA {\n    TCP::payload\n}"
        )
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 1

    def test_http_payload_without_collect(self):
        """HTTP::payload without HTTP::collect → warning."""
        src = "when HTTP_REQUEST_DATA {\n    HTTP::payload\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 1
        assert "HTTP::payload" in warnings[0].message

    def test_http_payload_with_collect(self):
        """HTTP::payload with HTTP::collect → no warning."""
        src = (
            "when HTTP_REQUEST {\n"
            "    HTTP::collect 1048576\n"
            "}\n"
            "when HTTP_REQUEST_DATA {\n"
            "    HTTP::payload\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 0

    def test_payload_in_bracket_substitution(self):
        """TCP::payload inside [set data [TCP::payload]] should be detected."""
        src = "when CLIENT_DATA {\n    set data [TCP::payload]\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 1

    def test_multiple_payloads_without_collect(self):
        """Multiple payload accesses should produce multiple warnings."""
        src = 'when CLIENT_DATA {\n    TCP::payload\n    TCP::payload replace 0 5 "hello"\n}'
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 2

    def test_no_payload_no_warning(self):
        """No payload access → no warning."""
        src = "when HTTP_REQUEST {\n    HTTP::uri\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 0

    def test_ssl_payload_without_collect(self):
        """SSL::payload without SSL::collect → warning."""
        src = "when CLIENTSSL_DATA {\n    SSL::payload\n}"
        warnings = _flow_with_code(src, "IRULE1006")
        assert len(warnings) == 1
        assert "SSL::payload" in warnings[0].message
        assert "SSL::collect" in warnings[0].message


# IRULE2102: Retired — subsumed by O105 (GVN/CSE)


class TestIrule2102:
    """IRULE2102 has been retired in favour of O105 (GVN/CSE).

    Repeated expensive command detection now lives in
    ``server.compiler.gvn._scan_when_bodies`` which handles both
    standalone and embedded command invocations.
    See ``tests/test_gvn.py::TestGVNIrulesWhenBodies`` for coverage.
    """

    def test_no_longer_emitted(self):
        src = "when HTTP_REQUEST {\n    HTTP::uri\n    HTTP::uri\n}"
        warnings = _flow_with_code(src, "IRULE2102")
        assert len(warnings) == 0


# IRULE4001: Write to static:: outside RULE_INIT


class TestIrule4001:
    """IRULE4001: static:: write outside RULE_INIT."""

    def test_set_static_in_http_request(self):
        src = "when HTTP_REQUEST {\n    set static::debug 1\n}"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 1
        assert "static::debug" in diags[0].message
        assert "outside RULE_INIT" in diags[0].message
        assert diags[0].severity is Severity.WARNING

    def test_set_static_in_rule_init_no_warning(self):
        src = "when RULE_INIT {\n    set static::debug 1\n}"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 0

    def test_array_set_static_outside_rule_init(self):
        src = "when HTTP_REQUEST {\n    array set static::config {k v}\n}"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 1
        assert "static::config" in diags[0].message

    def test_set_global_outside_rule_init_no_warning(self):
        """::var writes should not trigger IRULE4001."""
        src = "when HTTP_REQUEST {\n    set ::my_var 1\n}"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 0

    def test_set_local_no_warning(self):
        src = "when HTTP_REQUEST {\n    set local_var 1\n}"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 0

    def test_outside_when_no_warning(self):
        """Commands outside when blocks have no event context."""
        src = "set static::debug 1"
        diags = _diag_with_code(src, "IRULE4001")
        assert len(diags) == 0


# IRULE4002: Generic static:: variable name — collision likely


class TestIrule4002:
    """IRULE4002: generic static:: variable names that risk collision."""

    def test_static_debug_in_rule_init(self):
        src = "when RULE_INIT {\n    set static::debug 0\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1
        assert "generic" in diags[0].message.lower()
        assert "every irule" in diags[0].message.lower()
        assert diags[0].severity is Severity.HINT

    def test_static_debug_in_http_request(self):
        """Should still fire (in addition to IRULE4001)."""
        src = "when HTTP_REQUEST {\n    set static::debug 1\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_prefixed_name_no_warning(self):
        """A properly prefixed name should not trigger."""
        src = "when RULE_INIT {\n    set static::myapp_debug 1\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 0

    def test_array_set_static_debug(self):
        src = "when RULE_INIT {\n    array set static::debug {on 1 off 0}\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_log_level(self):
        """log_level is a commonly reused generic name."""
        src = "when RULE_INIT {\n    set static::log_level 3\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1
        assert "static::log_level" in diags[0].message

    def test_generic_timeout(self):
        src = "when RULE_INIT {\n    set static::timeout 5000\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_enabled(self):
        src = "when RULE_INIT {\n    set static::enabled 1\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_verbose(self):
        src = "when RULE_INIT {\n    set static::verbose 1\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_server(self):
        src = 'when RULE_INIT {\n    set static::server "10.0.0.1"\n}'
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_pool(self):
        src = 'when RULE_INIT {\n    set static::pool "my_pool"\n}'
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_generic_counter(self):
        src = "when RULE_INIT {\n    set static::counter 0\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_non_generic_name_no_warning(self):
        """Application-prefixed names should not trigger."""
        src = "when RULE_INIT {\n    set static::myapp_log_level 3\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 0

    def test_case_insensitive(self):
        """Generic name detection is case-insensitive."""
        src = "when RULE_INIT {\n    set static::DEBUG 0\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1

    def test_message_suggests_prefix(self):
        """Diagnostic message should suggest prefixing with app name."""
        src = "when RULE_INIT {\n    set static::debug 0\n}"
        diags = _diag_with_code(src, "IRULE4002")
        assert len(diags) == 1
        assert "prefix" in diags[0].message.lower()
        assert "<app>" in diags[0].message


# Event-aware completions


class TestEventAwareCompletions:
    """Completions should rank commands valid in the current event higher."""

    def test_valid_commands_sorted_first(self):
        configure_signatures(dialect="f5-irules")
        # Cursor inside when HTTP_REQUEST body, typing "HTTP::"
        src = "when HTTP_REQUEST {\n    HTTP::\n}"
        # line=1, character=10 is after "HTTP::"
        items = get_completions(src, 1, 10)
        http_items = [i for i in items if i.label.startswith("HTTP::")]
        # HTTP commands valid in HTTP_REQUEST should get event-priority rank "B0"
        valid_items = [i for i in http_items if i.sort_text and i.sort_text.startswith("B0")]
        assert len(valid_items) > 0, "Expected some HTTP commands valid in HTTP_REQUEST"

    def test_outside_when_neutral_event_rank(self):
        configure_signatures(dialect="f5-irules")
        src = "HTTP::\n"
        items = get_completions(src, 0, 6)
        # Outside a when block, command ranking should use neutral event bucket "B1".
        http_items = [i for i in items if i.label.startswith("HTTP::")]
        neutral_sort = [i for i in http_items if i.sort_text and i.sort_text.startswith("B1")]
        assert len(neutral_sort) == len(http_items), "Outside when, expected neutral event rank"


# Enhanced hovers


class TestEnhancedHovers:
    """Hovers should show valid events for iRules commands."""

    def test_http_respond_hover_has_valid_events(self):
        configure_signatures(dialect="f5-irules")
        src = "HTTP::respond 200"
        hover = get_hover(src, 0, 5)
        assert hover is not None
        contents = hover.contents
        assert isinstance(contents, MarkupContent)
        assert "Valid events" in contents.value
        # With event_requires the computed list is alphabetical; verify
        # the Requires section mentions the HTTP profile instead of
        # looking for a specific event in the (possibly truncated) list.
        assert "Requires" in contents.value
        assert "HTTP" in contents.value

    def test_http_respond_lifecycle_note_in_signature_help(self):
        configure_signatures(dialect="f5-irules")
        src = "HTTP::respond 200 "
        from lsp.features.signature_help import get_signature_help

        result = get_signature_help(src, 0, 18)
        assert result is not None
        sig = result.signatures[0]
        doc = sig.documentation
        assert doc is not None
        doc_text = doc.value if not isinstance(doc, str) else doc
        assert "event completes" in doc_text


# IRULE4003: Variable scoping across events


class TestIrule4003:
    """IRULE4003: variable scoping concerns across events."""

    def test_set_in_http_request_read_in_client_accepted(self):
        """Variable set in HTTP_REQUEST read in CLIENT_ACCEPTED (earlier) → fires."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "uri=$uri"\n'
            "}\n"
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 1
        assert "uri" in diags[0].message
        assert "CLIENT_ACCEPTED" in diags[0].message
        assert diags[0].severity is Severity.HINT

    def test_set_in_http_request_read_in_http_response_no_warning(self):
        """Variable set in HTTP_REQUEST read in HTTP_RESPONSE (later) → safe."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE {\n"
            '    log local0. "uri=$uri"\n'
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_set_in_rule_init_no_warning(self):
        """Variables set in RULE_INIT are always safe."""
        src = (
            "when RULE_INIT {\n"
            "    set debug_mode 0\n"
            "}\n"
            "when HTTP_REQUEST {\n"
            '    if {$debug_mode} { log local0. "debug" }\n'
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_static_var_no_irule4003(self):
        """static:: vars are handled by IRULE4001/4002, not 4003."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set static::uri [HTTP::uri]\n"
            "}\n"
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "$static::uri"\n'
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_global_var_no_warning(self):
        """Global (::) vars should not trigger IRULE4003."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set ::uri [HTTP::uri]\n"
            "}\n"
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "$::uri"\n'
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_per_request_to_per_connection_multiplicity(self):
        """Variable set in per-request event read in per-connection event → fires."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set method [HTTP::method]\n"
            "}\n"
            "when CLIENT_CLOSED {\n"
            '    log local0. "last method=$method"\n'
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        # HTTP_REQUEST is PER_REQUEST, CLIENT_CLOSED is ONCE_PER_CONNECTION
        # variable_scope_note fires a multiplicity concern
        assert len(diags) == 1
        assert "per-request" in diags[0].message
        assert "per-connection" in diags[0].message

    def test_set_read_form_no_warning(self):
        """set with one arg is a read, not a write — no warning."""
        src = (
            'when HTTP_REQUEST {\n    set uri\n}\nwhen CLIENT_ACCEPTED {\n    log local0. "$uri"\n}'
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_single_event_file_no_warning(self):
        """A file with only one event cannot have cross-event scoping issues."""
        src = 'when HTTP_REQUEST {\n    set uri [HTTP::uri]\n    log local0. "uri=$uri"\n}'
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0

    def test_braced_var_reference(self):
        """${varname} style references should be detected."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "uri=${uri}"\n'
            "}\n"
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 1
        assert "uri" in diags[0].message

    def test_multiple_concerns_combined(self):
        """Multiple cross-event concerns should be combined in one diagnostic."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "val=$myval"\n'
            "}\n"
            "when LB_SELECTED {\n"
            '    log local0. "val=$myval"\n'
            "}\n"
            "when HTTP_RESPONSE {\n"
            "    set myval [HTTP::header Content-Type]\n"
            "}"
        )
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 1
        # Should mention at least both concerned events
        assert "CLIENT_ACCEPTED" in diags[0].message or "LB_SELECTED" in diags[0].message

    def test_outside_when_no_warning(self):
        """set outside any when block has no event context."""
        src = "set myvar 123"
        diags = _diag_with_code(src, "IRULE4003")
        assert len(diags) == 0


# IRULE5002: drop/reject/discard without event disable all or return


class TestIrule5002:
    """IRULE5002: drop/reject/discard without event disable all."""

    def test_drop_without_event_disable(self):
        src = "when CLIENT_ACCEPTED {\n    drop\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1
        assert "drop" in warnings[0].message
        assert "event disable all" in warnings[0].message

    def test_drop_with_event_disable_all(self):
        src = "when CLIENT_ACCEPTED {\n    drop\n    event disable all\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 0

    def test_drop_with_return(self):
        src = "when CLIENT_ACCEPTED {\n    drop\n    return\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 0

    def test_reject_without_event_disable(self):
        src = "when CLIENT_ACCEPTED {\n    reject\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1
        assert "reject" in warnings[0].message

    def test_discard_without_event_disable(self):
        src = "when CLIENT_ACCEPTED {\n    discard\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1
        assert "discard" in warnings[0].message

    def test_drop_in_if_with_event_disable(self):
        src = (
            "when CLIENT_ACCEPTED {\n"
            "    if {$x} {\n"
            "        drop\n"
            "        event disable all\n"
            "    }\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 0

    def test_drop_in_if_without_event_disable(self):
        src = "when CLIENT_ACCEPTED {\n    if {$x} {\n        drop\n    }\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1

    def test_plain_event_disable_still_warns(self):
        """'event disable' (without 'all') should NOT suppress the warning."""
        src = "when CLIENT_ACCEPTED {\n    drop\n    event disable\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1

    def test_no_drop_no_warning(self):
        src = "when HTTP_REQUEST {\n    HTTP::respond 200\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 0

    def test_drop_with_event_disable_all_and_return(self):
        """Full safe pattern: drop + event disable all + return."""
        src = "when HTTP_REQUEST {\n    drop\n    event disable all\n    return\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 0

    def test_drop_in_switch_branch(self):
        src = (
            "when HTTP_REQUEST {\n"
            "    switch -- [HTTP::method] {\n"
            '        "DELETE" {\n'
            "            drop\n"
            "        }\n"
            "    }\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1

    def test_has_codefix(self):
        src = "when CLIENT_ACCEPTED {\n    reject\n}"
        warnings = _flow_with_code(src, "IRULE5002")
        assert len(warnings) == 1
        assert len(warnings[0].fixes) > 0
        assert "event disable all" in warnings[0].fixes[0].new_text


# IRULE5004: DNS::return without return


class TestIrule5004:
    """IRULE5004: DNS::return without return."""

    def test_dns_return_without_return(self):
        src = "when DNS_REQUEST {\n    DNS::return\n}"
        warnings = _flow_with_code(src, "IRULE5004")
        assert len(warnings) == 1
        assert "DNS::return" in warnings[0].message

    def test_dns_return_with_return(self):
        src = "when DNS_REQUEST {\n    DNS::return\n    return\n}"
        warnings = _flow_with_code(src, "IRULE5004")
        assert len(warnings) == 0

    def test_dns_return_in_branch_without_return(self):
        src = "when DNS_REQUEST {\n    if {$x} {\n        DNS::return\n    }\n}"
        warnings = _flow_with_code(src, "IRULE5004")
        assert len(warnings) == 1

    def test_dns_return_in_branch_with_return(self):
        src = "when DNS_REQUEST {\n    if {$x} {\n        DNS::return\n        return\n    }\n}"
        warnings = _flow_with_code(src, "IRULE5004")
        assert len(warnings) == 0

    def test_has_codefix(self):
        src = "when DNS_REQUEST {\n    DNS::return\n}"
        warnings = _flow_with_code(src, "IRULE5004")
        assert len(warnings) == 1
        assert len(warnings[0].fixes) > 0
        assert "return" in warnings[0].fixes[0].new_text


class TestIrule5003:
    """IRULE5003: while {$var != 0} with decrement can miss zero."""

    def test_ne_zero_with_decrement(self):
        src = "while {$count != 0} { incr count -1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 1
        assert "$count != 0" in diags[0].message
        assert diags[0].severity == Severity.HINT

    def test_ne_literal_with_decrement(self):
        src = "while {$count ne 0} { incr count -1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 1

    def test_reversed_operands(self):
        src = "while {0 != $count} { incr count -1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 1

    def test_gt_zero_no_warning(self):
        src = "while {$count > 0} { incr count -1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 0

    def test_ne_zero_with_increment_no_warning(self):
        src = "while {$count != 0} { incr count 1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 0

    def test_different_var_no_warning(self):
        src = "while {$count != 0} { incr other -1 }"
        diags = _diag_with_code(src, "IRULE5003")
        assert len(diags) == 0


# IRULE4004: Hoist constant set from per-request to per-connection


class TestIrule4004:
    """IRULE4004: constant set in per-request event hoistable to per-connection."""

    def test_literal_integer_fires(self):
        """set timeout 30 in HTTP_REQUEST with CLIENT_ACCEPTED → fires."""
        src = (
            "when CLIENT_ACCEPTED {\n"
            '    log local0. "accepted"\n'
            "}\n"
            "when HTTP_REQUEST {\n"
            "    set timeout 30\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "timeout" in warnings[0].message
        assert "CLIENT_ACCEPTED" in warnings[0].message

    def test_braced_literal_fires(self):
        """set x {hello} with braced literal → fires."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set pool {my_pool}\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1

    def test_ip_client_addr_hoistable(self):
        """[IP::client_addr] is available at CLIENT_ACCEPTED → fires."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set client_ip [IP::client_addr]\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "client_ip" in warnings[0].message

    def test_http_uri_not_hoistable(self):
        """[HTTP::uri] requires HTTP profile → not available at CLIENT_ACCEPTED."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_nested_http_in_string_cmd_not_hoistable(self):
        """[string tolower [HTTP::uri]] has nested HTTP dep → not hoistable."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set uri [string tolower [HTTP::uri]]\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_variable_ref_not_hoistable(self):
        """set method $m has variable ref → not hoistable."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set method $m\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_dollar_in_cmd_not_hoistable(self):
        """[expr {$a + 1}] has $ inside command → not hoistable."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set x [expr {$a + 1}]\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_already_once_per_connection_no_fire(self):
        """set timeout 30 in CLIENT_ACCEPTED → no warning."""
        src = "when CLIENT_ACCEPTED {\n    set timeout 30\n}"
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_no_existing_target_still_fires(self):
        """set timeout 30 in HTTP_REQUEST with no once-per-connection event
        should still fire, suggesting the best predecessor event."""
        src = "when HTTP_REQUEST {\n    set timeout 30\n}"
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "event not yet in this iRule" in warnings[0].message

    def test_prefers_existing_event(self):
        """When the target event exists in the file, no 'not yet' note."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set timeout 30\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "CLIENT_ACCEPTED" in warnings[0].message
        assert "event not yet" not in warnings[0].message

    def test_static_var_skipped(self):
        """static:: variables are excluded (IRULE4001 handles them)."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set static::x 1\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_inside_if_not_top_level(self):
        """set inside if block → not top-level → no fire."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n"
            "    if {1} {\n"
            "        set timeout 30\n"
            "    }\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 0

    def test_clock_seconds_hoistable(self):
        """[clock seconds] has no profile requirement → hoistable."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n    set ts [clock seconds]\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1

    def test_multiple_constants_multiple_warnings(self):
        """Multiple constant sets in one body → multiple warnings."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when HTTP_REQUEST {\n"
            "    set timeout 30\n"
            "    set pool_name {my_pool}\n"
            "    set uri [HTTP::uri]\n"
            "}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 2  # timeout and pool_name, not uri

    def test_dns_request_no_access_policy(self):
        """DNS_REQUEST must not suggest ACCESS_POLICY_COMPLETED — incompatible profiles."""
        src = "when DNS_REQUEST {\n    set rdtracker 0\n}"
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "ACCESS_POLICY_COMPLETED" not in warnings[0].message
        assert "CLIENT_ACCEPTED" in warnings[0].message
        assert "event not yet in this iRule" in warnings[0].message

    def test_dns_request_with_client_accepted(self):
        """DNS_REQUEST with CLIENT_ACCEPTED in file → hoists to CLIENT_ACCEPTED."""
        src = (
            'when CLIENT_ACCEPTED {\n    log local0. "ok"\n}\n'
            "when DNS_REQUEST {\n    set rdtracker 0\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        assert len(warnings) == 1
        assert "CLIENT_ACCEPTED" in warnings[0].message
        assert "event not yet" not in warnings[0].message

    def test_dns_request_ignores_access_in_file(self):
        """DNS_REQUEST must not hoist to ACCESS_POLICY_COMPLETED even if present."""
        src = (
            'when ACCESS_POLICY_COMPLETED {\n    log local0. "ok"\n}\n'
            "when DNS_REQUEST {\n    set rdtracker 0\n}"
        )
        warnings = _flow_with_code(src, "IRULE4004")
        # Should suggest CLIENT_ACCEPTED (from flow chain), not
        # ACCESS_POLICY_COMPLETED (incompatible profile).
        for w in warnings:
            assert "ACCESS_POLICY_COMPLETED" not in w.message


# IRULE6001: Global namespace variable causes TMM pinning


class TestIrule6001:
    """IRULE6001: global namespace variable (::var) forces CMP compatibility / TMM pinning."""

    def test_set_global_var_warns(self):
        """set ::myvar value → warning."""
        src = "when HTTP_REQUEST {\n    set ::myvar 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::myvar" in diags[0].message
        assert "TMM" in diags[0].message
        assert "static::myvar" in diags[0].message
        assert diags[0].severity is Severity.WARNING

    def test_set_global_var_read_warns(self):
        """set ::myvar (read form, one arg) → warning."""
        src = "when HTTP_REQUEST {\n    set ::myvar\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::myvar" in diags[0].message

    def test_incr_global_var_warns(self):
        """incr ::counter → warning."""
        src = "when HTTP_REQUEST {\n    incr ::counter\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::counter" in diags[0].message
        assert "static::counter" in diags[0].message

    def test_append_global_var_warns(self):
        """append ::log_data value → warning."""
        src = 'when HTTP_REQUEST {\n    append ::log_data "entry"\n}'
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::log_data" in diags[0].message

    def test_lappend_global_var_warns(self):
        """lappend ::items value → warning."""
        src = 'when HTTP_REQUEST {\n    lappend ::items "item"\n}'
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::items" in diags[0].message

    def test_unset_global_var_warns(self):
        """unset ::myvar → warning."""
        src = "when HTTP_REQUEST {\n    unset ::myvar\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::myvar" in diags[0].message

    def test_array_set_global_var_warns(self):
        """array set ::config {k v} → warning."""
        src = "when HTTP_REQUEST {\n    array set ::config {k v}\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "::config" in diags[0].message
        assert "static::config" in diags[0].message

    def test_global_command_warns(self):
        """global varname → warning."""
        src = "when HTTP_REQUEST {\n    global myvar\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "global myvar" in diags[0].message
        assert "TMM" in diags[0].message
        assert "static::myvar" in diags[0].message

    def test_local_var_no_warning(self):
        """set localvar value → no warning."""
        src = "when HTTP_REQUEST {\n    set localvar 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_static_var_no_warning(self):
        """set static::var value → no IRULE6001 (handled by IRULE4001/4002)."""
        src = "when HTTP_REQUEST {\n    set static::myvar 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_outside_when_no_warning(self):
        """Commands outside when blocks have no event context → no warning."""
        src = "set ::myvar 1"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_has_codefix(self):
        """Diagnostic should carry a CodeFix to replace :: with static::."""
        src = "when HTTP_REQUEST {\n    set ::myvar 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text == "static::myvar"
        assert "Replace" in fix.description

    def test_global_command_no_codefix(self):
        """The 'global' command form has no simple replacement fix."""
        src = "when HTTP_REQUEST {\n    global myvar\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 0

    def test_in_rule_init_still_warns(self):
        """Global namespace usage in RULE_INIT still causes TMM pinning."""
        src = "when RULE_INIT {\n    set ::key [AES::key 128]\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "static::key" in diags[0].message

    def test_message_mentions_cmp(self):
        """Diagnostic message should mention CMP compatibility mode."""
        src = "when HTTP_REQUEST {\n    set ::x 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "CMP" in diags[0].message

    # Implicit globals in RULE_INIT

    def test_plain_set_in_rule_init_warns(self):
        """set var value in RULE_INIT → implicit global, warning."""
        src = "when RULE_INIT {\n    set mykey [AES::key 128]\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "implicitly global" in diags[0].message
        assert "RULE_INIT" in diags[0].message
        assert "static::mykey" in diags[0].message
        assert diags[0].severity is Severity.WARNING

    def test_plain_set_in_rule_init_has_codefix(self):
        """Implicit global in RULE_INIT should carry a CodeFix."""
        src = "when RULE_INIT {\n    set mykey 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text == "static::mykey"

    def test_plain_incr_in_rule_init_warns(self):
        """incr var in RULE_INIT → implicit global."""
        src = "when RULE_INIT {\n    set counter 0\n    incr counter\n}"
        diags = _diag_with_code(src, "IRULE6001")
        # Both set and incr should fire
        assert len(diags) == 2

    def test_plain_append_in_rule_init_warns(self):
        """append var in RULE_INIT → implicit global."""
        src = 'when RULE_INIT {\n    append buf "data"\n}'
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "static::buf" in diags[0].message

    def test_plain_lappend_in_rule_init_warns(self):
        """lappend var in RULE_INIT → implicit global."""
        src = 'when RULE_INIT {\n    lappend items "item"\n}'
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "static::items" in diags[0].message

    def test_plain_array_set_in_rule_init_warns(self):
        """array set var in RULE_INIT → implicit global."""
        src = "when RULE_INIT {\n    array set config {k v}\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "static::config" in diags[0].message

    def test_plain_set_read_in_rule_init_no_warning(self):
        """set var (read form, one arg) in RULE_INIT → no warning."""
        src = "when RULE_INIT {\n    set mykey\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_plain_set_outside_rule_init_no_warning(self):
        """set var value outside RULE_INIT → local, no IRULE6001."""
        src = "when HTTP_REQUEST {\n    set localvar 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_static_set_in_rule_init_no_warning(self):
        """set static::var in RULE_INIT → already scoped, no IRULE6001."""
        src = "when RULE_INIT {\n    set static::mykey [AES::key 128]\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 0

    def test_implicit_global_message_mentions_cmp(self):
        """Implicit global diagnostic should mention CMP."""
        src = "when RULE_INIT {\n    set mykey 1\n}"
        diags = _diag_with_code(src, "IRULE6001")
        assert len(diags) == 1
        assert "CMP" in diags[0].message
