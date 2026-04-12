"""Tests for diagram data extraction from iRule IR."""

from __future__ import annotations

import textwrap

from core.diagram.extract import extract_diagram_data


class TestSimpleExtraction:
    """Basic extraction from single-event iRules."""

    def test_simple_pool_selection(self):
        source = "when HTTP_REQUEST { pool my_pool }"
        data = extract_diagram_data(source)
        assert len(data["events"]) == 1
        event = data["events"][0]
        assert event["name"] == "HTTP_REQUEST"
        assert event["multiplicity"] == "per_request"
        assert len(event["flow"]) == 1
        assert event["flow"][0]["kind"] == "action"
        assert event["flow"][0]["command"] == "pool"

    def test_switch_on_uri(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                switch -glob -- [HTTP::uri] {
                    "/api/*"    { pool api_pool }
                    "/static/*" { pool static_pool }
                    default     { HTTP::respond 404 content "Not Found" }
                }
            }
        """)
        data = extract_diagram_data(source)
        assert len(data["events"]) == 1
        flow = data["events"][0]["flow"]
        assert len(flow) == 1
        sw = flow[0]
        assert sw["kind"] == "switch"
        assert "[HTTP::uri]" in sw["subject"]
        assert len(sw["arms"]) == 3
        assert sw["arms"][0]["pattern"] == "/api/*"
        assert sw["arms"][1]["pattern"] == "/static/*"
        assert sw["arms"][2]["pattern"] == "default"
        # Check actions inside arms.
        assert sw["arms"][0]["body"][0]["command"] == "pool"
        assert sw["arms"][2]["body"][0]["command"] == "HTTP::respond"

    def test_if_else_chain(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {[HTTP::method] eq "OPTIONS"} {
                    HTTP::respond 204
                    return
                } elseif {[HTTP::method] eq "GET"} {
                    pool get_pool
                } else {
                    pool post_pool
                }
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        assert len(flow) == 1
        ifn = flow[0]
        assert ifn["kind"] == "if"
        assert len(ifn["branches"]) == 3
        # First branch has HTTP::respond + return.
        first_body = ifn["branches"][0]["body"]
        assert any(n["kind"] == "action" for n in first_body)
        assert any(n["kind"] == "return" for n in first_body)
        # Last branch is else.
        assert ifn["branches"][2]["condition"] == "else"

    def test_empty_event_body(self):
        source = "when HTTP_REQUEST { }"
        data = extract_diagram_data(source)
        assert len(data["events"]) == 1
        assert data["events"][0]["flow"] == []


class TestMultipleEvents:
    """Tests for multi-event iRules."""

    def test_events_in_canonical_order(self):
        source = textwrap.dedent("""\
            when HTTP_RESPONSE { HTTP::header remove Server }
            when CLIENT_ACCEPTED { log local0. "connected" }
            when HTTP_REQUEST { pool default_pool }
        """)
        data = extract_diagram_data(source)
        event_names = [e["name"] for e in data["events"]]
        assert event_names == ["CLIENT_ACCEPTED", "HTTP_REQUEST", "HTTP_RESPONSE"]

    def test_multiplicity_annotations(self):
        source = textwrap.dedent("""\
            when CLIENT_ACCEPTED { log local0. "hi" }
            when HTTP_REQUEST { pool my_pool }
            when RULE_INIT { set static::debug 0 }
        """)
        data = extract_diagram_data(source)
        mults = {e["name"]: e["multiplicity"] for e in data["events"]}
        assert mults["CLIENT_ACCEPTED"] == "once_per_connection"
        assert mults["HTTP_REQUEST"] == "per_request"
        assert mults["RULE_INIT"] == "init"


class TestPriority:
    """Priority extraction from when blocks."""

    def test_priority_extracted(self):
        source = "when HTTP_REQUEST priority 200 { pool my_pool }"
        data = extract_diagram_data(source)
        assert data["events"][0]["priority"] == 200

    def test_default_priority_is_none(self):
        source = "when HTTP_REQUEST { pool my_pool }"
        data = extract_diagram_data(source)
        assert data["events"][0]["priority"] is None


class TestProcedures:
    """Procedure definition and call extraction."""

    def test_procedure_in_procedures_list(self):
        source = textwrap.dedent("""\
            proc select_pool {uri} {
                if {[string match "/api/*" $uri]} {
                    return "api_pool"
                }
                return "default_pool"
            }
            when HTTP_REQUEST {
                pool [select_pool [HTTP::uri]]
            }
        """)
        data = extract_diagram_data(source)
        assert len(data["procedures"]) == 1
        assert data["procedures"][0]["name"] == "select_pool"
        assert data["procedures"][0]["params"] == ["uri"]
        # Procedure body should have if and return nodes.
        proc_flow = data["procedures"][0]["flow"]
        assert any(n["kind"] == "if" for n in proc_flow)

    def test_proc_call_detected_in_event(self):
        source = textwrap.dedent("""\
            proc log_it {msg} { log local0. $msg }
            when HTTP_REQUEST {
                log_it "hello"
                pool my_pool
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        kinds = [n["kind"] for n in flow]
        assert "proc_call" in kinds
        assert "action" in kinds


class TestNestedControlFlow:
    """Nested decision structures."""

    def test_switch_inside_if(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {[HTTP::header exists "X-Custom"]} {
                    switch [HTTP::header value "X-Custom"] {
                        "a" { pool pool_a }
                        "b" { pool pool_b }
                    }
                }
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        assert flow[0]["kind"] == "if"
        inner = flow[0]["branches"][0]["body"]
        assert inner[0]["kind"] == "switch"

    def test_if_inside_switch(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                switch -glob -- [HTTP::uri] {
                    "/api/*" {
                        if {[HTTP::method] eq "POST"} {
                            pool api_write_pool
                        } else {
                            pool api_read_pool
                        }
                    }
                    default { pool default_pool }
                }
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        sw = flow[0]
        assert sw["kind"] == "switch"
        arm_body = sw["arms"][0]["body"]
        assert arm_body[0]["kind"] == "if"


class TestSwitchFallthrough:
    """Switch arms with fallthrough (-)."""

    def test_fallthrough_patterns_merged(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                switch [HTTP::uri] {
                    "/a" -
                    "/b" { pool ab_pool }
                    "/c" { pool c_pool }
                }
            }
        """)
        data = extract_diagram_data(source)
        arms = data["events"][0]["flow"][0]["arms"]
        # The fallthrough arms /a and /b should be merged.
        assert len(arms) == 2
        assert "/a" in arms[0]["pattern"]
        assert "/b" in arms[0]["pattern"]
        assert arms[1]["pattern"] == "/c"


class TestActionCommands:
    """Various action command types are extracted."""

    def test_common_actions(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::header insert X-Forwarded-For [IP::client_addr]
                pool my_pool
                log local0. "request"
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        commands = [n.get("command") for n in flow if n.get("command")]
        assert "HTTP::header" in commands
        assert "pool" in commands
        assert "log" in commands

    def test_redirect_and_respond(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {[HTTP::uri] eq "/old"} {
                    HTTP::redirect "https://example.com/new"
                } else {
                    HTTP::respond 200 content "OK"
                }
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        if_node = flow[0]
        branch0_cmds = [n.get("command") for n in if_node["branches"][0]["body"]]
        branch1_cmds = [n.get("command") for n in if_node["branches"][1]["body"]]
        assert "HTTP::redirect" in branch0_cmds
        assert "HTTP::respond" in branch1_cmds


class TestAssignments:
    """Variable assignments with command substitutions."""

    def test_notable_assign_extracted(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set uri [HTTP::uri]
                pool my_pool
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        assigns = [n for n in flow if n["kind"] == "assign"]
        assert len(assigns) == 1
        assert assigns[0]["var"] == "uri"
        assert "[HTTP::uri]" in assigns[0]["value"]

    def test_plain_assign_skipped(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set x 42
                pool my_pool
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        # The plain `set x 42` should be skipped (no command substitution).
        kinds = [n["kind"] for n in flow]
        assert "assign" not in kinds


class TestLoops:
    """Loop extraction."""

    def test_foreach_loop(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                foreach header {X-A X-B X-C} {
                    HTTP::header remove $header
                }
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        assert flow[0]["kind"] == "loop"
        assert "foreach" in flow[0]["label"]
        assert len(flow[0]["body"]) >= 1


class TestReturn:
    """Return node extraction."""

    def test_return_with_value(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::respond 403
                return
            }
        """)
        data = extract_diagram_data(source)
        flow = data["events"][0]["flow"]
        returns = [n for n in flow if n["kind"] == "return"]
        assert len(returns) == 1
        assert returns[0]["label"] == "return"


class TestMermaidSafeOperators:
    """Symbolic logical operators are replaced with words for Mermaid."""

    def test_and_operator_replaced(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {[HTTP::uri] starts_with "/api" && [HTTP::method] eq "POST"} {
                    pool api_pool
                }
            }
        """)
        data = extract_diagram_data(source)
        cond = data["events"][0]["flow"][0]["branches"][0]["condition"]
        assert "&&" not in cond
        assert "and" in cond

    def test_or_operator_replaced(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {[HTTP::method] eq "GET" || [HTTP::method] eq "HEAD"} {
                    pool read_pool
                }
            }
        """)
        data = extract_diagram_data(source)
        cond = data["events"][0]["flow"][0]["branches"][0]["condition"]
        assert "||" not in cond
        assert "or" in cond

    def test_not_operator_replaced(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {![HTTP::header exists "Host"]} {
                    HTTP::respond 400
                }
            }
        """)
        data = extract_diagram_data(source)
        cond = data["events"][0]["flow"][0]["branches"][0]["condition"]
        assert cond.startswith("not ")
        assert "!" not in cond


class TestEdgeCases:
    """Edge cases and error tolerance."""

    def test_no_when_blocks(self):
        source = "proc foo {} { return 1 }"
        data = extract_diagram_data(source)
        assert data["events"] == []
        assert len(data["procedures"]) == 1

    def test_empty_source(self):
        data = extract_diagram_data("")
        assert data["events"] == []
        assert data["procedures"] == []

    def test_syntax_error_does_not_crash(self):
        source = "when HTTP_REQUEST { if { incomplete"
        data = extract_diagram_data(source)
        assert "events" in data

    def test_unknown_event_still_included(self):
        source = "when MY_CUSTOM_EVENT { pool custom_pool }"
        data = extract_diagram_data(source)
        assert len(data["events"]) == 1
        assert data["events"][0]["name"] == "MY_CUSTOM_EVENT"
        assert data["events"][0]["multiplicity"] == "unknown"
