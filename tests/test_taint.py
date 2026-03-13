"""Tests for taint analysis and collect/release pairing."""

from __future__ import annotations

from core.commands.registry.taint_hints import TaintColour
from core.compiler.taint import (
    CollectWithoutReleaseWarning,
    ReleaseWithoutCollectWarning,
    TaintLattice,
    TaintWarning,
    find_taint_warnings,
    taint_join,
)


def _codes(source: str) -> list[str]:
    """Return sorted list of diagnostic codes from taint analysis."""
    return sorted(w.code for w in find_taint_warnings(source))


def _taint_warnings(source: str, code: str | None = None) -> list[TaintWarning]:
    """Return T1xx taint warnings, optionally filtered by code."""
    return [
        w
        for w in find_taint_warnings(source)
        if isinstance(w, TaintWarning) and (code is None or w.code == code)
    ]


def _collect_warnings(source: str) -> list[CollectWithoutReleaseWarning]:
    """Return T200 collect-without-release warnings."""
    return [w for w in find_taint_warnings(source) if isinstance(w, CollectWithoutReleaseWarning)]


def _release_warnings(source: str) -> list[ReleaseWithoutCollectWarning]:
    """Return T201 release-without-collect warnings."""
    return [w for w in find_taint_warnings(source) if isinstance(w, ReleaseWithoutCollectWarning)]


# Lattice join

_UNTAINTED = TaintLattice.untainted()
_TAINTED = TaintLattice.of(TaintColour.TAINTED)
_PATH = TaintLattice.of(TaintColour.TAINTED | TaintColour.PATH_PREFIXED)
_IP = TaintLattice.of(TaintColour.TAINTED | TaintColour.IP_ADDRESS)
_PORT = TaintLattice.of(TaintColour.TAINTED | TaintColour.PORT)
_FQDN = TaintLattice.of(TaintColour.TAINTED | TaintColour.FQDN)


class TestTaintJoin:
    def test_untainted_untainted(self):
        r = taint_join(_UNTAINTED, _UNTAINTED)
        assert not r.tainted

    def test_tainted_untainted(self):
        r = taint_join(_TAINTED, _UNTAINTED)
        assert r.tainted

    def test_untainted_tainted(self):
        r = taint_join(_UNTAINTED, _TAINTED)
        assert r.tainted

    def test_tainted_tainted(self):
        r = taint_join(_TAINTED, _TAINTED)
        assert r.tainted

    def test_path_prefixed_join_preserves_colour(self):
        r = taint_join(_PATH, _PATH)
        assert r.tainted
        assert bool(r.colour & TaintColour.PATH_PREFIXED)

    def test_path_prefixed_join_with_generic_loses_colour(self):
        r = taint_join(_PATH, _TAINTED)
        assert r.tainted
        assert not bool(r.colour & TaintColour.PATH_PREFIXED)

    def test_path_prefixed_join_with_untainted(self):
        r = taint_join(_PATH, _UNTAINTED)
        assert r.tainted
        assert bool(r.colour & TaintColour.PATH_PREFIXED)

    def test_ip_join_preserves_colour(self):
        r = taint_join(_IP, _IP)
        assert r.tainted
        assert bool(r.colour & TaintColour.IP_ADDRESS)

    def test_ip_join_with_generic_loses_colour(self):
        r = taint_join(_IP, _TAINTED)
        assert r.tainted
        assert not bool(r.colour & TaintColour.IP_ADDRESS)

    def test_port_join_preserves_colour(self):
        r = taint_join(_PORT, _PORT)
        assert r.tainted
        assert bool(r.colour & TaintColour.PORT)

    def test_fqdn_join_with_untainted(self):
        r = taint_join(_FQDN, _UNTAINTED)
        assert r.tainted
        assert bool(r.colour & TaintColour.FQDN)

    def test_different_colours_lose_both(self):
        r = taint_join(_IP, _PORT)
        assert r.tainted
        # IP_ADDRESS & PORT = 0 (different flags)
        assert not bool(r.colour & TaintColour.IP_ADDRESS)
        assert not bool(r.colour & TaintColour.PORT)


# No false positives


class TestNoFalsePositives:
    """Clean code should produce no taint warnings."""

    def test_literal_set(self):
        assert _codes("set x 42") == []

    def test_incr(self):
        assert _codes("set x 0\nincr x") == []

    def test_safe_eval(self):
        assert _codes('set x "puts hello"\neval $x') == []

    def test_safe_expr(self):
        assert _codes("set x 42\nexpr {$x + 1}") == []

    def test_list_operations(self):
        assert _codes("set x [list a b c]\nset n [llength $x]") == []


# Taint sources — Tcl core


class TestTclTaintSources:
    """Commands that read from I/O should taint their results."""

    def test_read_taints(self):
        ws = _taint_warnings("set data [read $fd]\neval $data", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "data"

    def test_gets_taints(self):
        # gets return value (when used without a varName arg)
        ws = _taint_warnings("set line [gets $fd]\neval $line", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "line"

    def test_exec_taints(self):
        ws = _taint_warnings("set output [exec ls]\neval $output", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "output"

    def test_socket_taints(self):
        ws = _taint_warnings("set s [socket localhost 80]\neval $s", "T100")
        assert len(ws) >= 1

    def test_chan_read_taints(self):
        ws = _taint_warnings("set data [chan read $fd]\neval $data", "T100")
        assert len(ws) >= 1

    def test_chan_gets_taints(self):
        ws = _taint_warnings("set data [chan gets $fd]\neval $data", "T100")
        assert len(ws) >= 1

    def test_chan_configure_does_not_taint(self):
        # chan configure is not a taint source
        assert _codes("set x [chan configure $fd]\neval $x") == []


# Taint sources — iRules


class TestIRulesTaintSources:
    """iRules I/O commands should taint their results."""

    def test_http_payload_taints(self):
        ws = _taint_warnings(
            "set body [HTTP::payload]\neval $body",
            "T100",
        )
        assert len(ws) >= 1
        assert ws[0].variable == "body"

    def test_http_header_taints(self):
        ws = _taint_warnings(
            'set hdr [HTTP::header "Host"]\neval $hdr',
            "T100",
        )
        assert len(ws) >= 1

    def test_http_uri_taints(self):
        ws = _taint_warnings(
            "set uri [HTTP::uri]\neval $uri",
            "T100",
        )
        assert len(ws) >= 1

    def test_tcp_payload_taints(self):
        ws = _taint_warnings(
            "set data [TCP::payload]\neval $data",
            "T100",
        )
        assert len(ws) >= 1

    def test_ip_client_addr_taints_with_colour(self):
        # IP addresses are tainted but with IP_ADDRESS colour.
        ws = _taint_warnings("set addr [IP::client_addr]\neval $addr", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "addr"


# Taint propagation


class TestTaintPropagation:
    """Taint should propagate through assignments and interpolation."""

    def test_variable_copy_propagates(self):
        ws = _taint_warnings("set x [read $fd]\nset y $x\neval $y", "T100")
        assert any(w.variable == "y" for w in ws)

    def test_string_interpolation_propagates(self):
        ws = _taint_warnings(
            'set x [read $fd]\nset y "prefix${x}suffix"\neval $y',
            "T100",
        )
        assert any(w.variable == "y" for w in ws)

    def test_command_subst_concat_propagates(self):
        ws = _taint_warnings(
            "set x [read $fd]/suffix\neval $x",
            "T100",
        )
        assert any(w.variable == "x" for w in ws)

    def test_sanitiser_blocks_taint(self):
        # string length returns INT — sanitises taint
        assert _codes("set x [read $fd]\nset n [string length $x]\nexpr $n") == []

    def test_llength_sanitises(self):
        assert _codes("set x [read $fd]\nset n [llength $x]\nexpr $n") == []

    def test_taint_through_if_branch(self):
        source = """\
set x [read $fd]
if {1} {
    set y $x
} else {
    set y "safe"
}
eval $y
"""
        ws = _taint_warnings(source, "T100")
        assert any(w.variable == "y" for w in ws)


class TestInterproceduralTaint:
    """Interprocedural taint should use proc summaries (IDE-style)."""

    def test_helper_sanitiser_suppresses_taint(self):
        source = """\
proc safe_len {x} { return [string length $x] }
set raw [read $fd]
set n [safe_len $raw]
expr $n
"""
        assert _codes(source) == []

    def test_tainted_return_from_helper_reaches_sink(self):
        source = """\
proc payload {} { return [HTTP::payload] }
set data [payload]
eval $data
"""
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1
        assert any(w.variable == "data" for w in ws)

    def test_tainted_argument_into_sinking_helper_warns(self):
        source = """\
proc do_eval {x} { eval $x }
set data [read $fd]
do_eval $data
"""
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1
        assert any(w.sink_command == "eval" for w in ws)


# Dangerous sinks (T100)


class TestDangerousSinks:
    """Tainted data in eval/expr/exec/uplevel/subst should warn."""

    def test_eval_sink(self):
        ws = _taint_warnings("set x [read $fd]\neval $x", "T100")
        assert len(ws) >= 1
        assert ws[0].sink_command == "eval"

    def test_expr_sink(self):
        ws = _taint_warnings("set x [read $fd]\nexpr $x", "T100")
        assert len(ws) >= 1

    def test_exec_sink(self):
        ws = _taint_warnings("set x [read $fd]\nexec $x", "T100")
        assert len(ws) >= 1
        assert ws[0].sink_command == "exec"

    def test_uplevel_sink(self):
        ws = _taint_warnings("set x [read $fd]\nuplevel $x", "T100")
        assert len(ws) >= 1

    def test_subst_sink(self):
        ws = _taint_warnings("set x [read $fd]\nsubst $x", "T100")
        assert len(ws) >= 1


# Collect / release pairing (T200)


class TestCollectRelease:
    """collect without matching release should warn."""

    def test_collect_without_release(self):
        ws = _collect_warnings("HTTP::collect")
        assert len(ws) == 1
        assert ws[0].command == "HTTP::collect"

    def test_collect_with_release(self):
        assert _codes("HTTP::collect\nHTTP::release") == []

    def test_tcp_collect_without_release(self):
        ws = _collect_warnings("TCP::collect")
        assert len(ws) == 1
        assert ws[0].command == "TCP::collect"

    def test_tcp_collect_with_release(self):
        assert _codes("TCP::collect\nTCP::release") == []

    def test_protocol_mismatch(self):
        # HTTP::collect needs HTTP::release, not TCP::release
        ws = _collect_warnings("HTTP::collect\nTCP::release")
        assert len(ws) >= 1
        assert ws[0].command == "HTTP::collect"

    def test_ssl_collect_without_release(self):
        ws = _collect_warnings("SSL::collect\nSSL::release")
        assert len(ws) == 0

    def test_multiple_protocols(self):
        source = "HTTP::collect\nTCP::collect\nHTTP::release"
        ws = _collect_warnings(source)
        # TCP::collect has no release
        assert len(ws) == 1
        assert ws[0].command == "TCP::collect"


# Release without collect (T201)


class TestReleaseWithoutCollect:
    """release without matching collect should warn."""

    def test_release_without_collect(self):
        ws = _release_warnings("HTTP::release")
        assert len(ws) == 1
        assert ws[0].command == "HTTP::release"
        assert ws[0].code == "T201"

    def test_release_with_collect(self):
        assert _release_warnings("HTTP::collect\nHTTP::release") == []

    def test_tcp_release_without_collect(self):
        ws = _release_warnings("TCP::release")
        assert len(ws) == 1
        assert ws[0].command == "TCP::release"

    def test_protocol_mismatch(self):
        # TCP::release needs TCP::collect, not HTTP::collect
        ws = _release_warnings("HTTP::collect\nTCP::release")
        assert len(ws) == 1
        assert ws[0].command == "TCP::release"

    def test_multiple_protocols(self):
        source = "HTTP::release\nTCP::release\nTCP::collect"
        ws = _release_warnings(source)
        # HTTP::release has no collect
        assert len(ws) == 1
        assert ws[0].command == "HTTP::release"

    def test_release_warning_message(self):
        ws = _release_warnings("HTTP::release")
        assert len(ws) == 1
        assert "HTTP::release" in ws[0].message
        assert "HTTP::collect" in ws[0].message


# Warning message content


class TestWarningMessages:
    def test_taint_warning_message(self):
        ws = _taint_warnings("set x [read $fd]\neval $x", "T100")
        assert len(ws) >= 1
        assert "eval" in ws[0].message
        assert "$x" in ws[0].message or "x" in ws[0].message

    def test_collect_warning_message(self):
        ws = _collect_warnings("HTTP::collect")
        assert len(ws) == 1
        assert "HTTP::collect" in ws[0].message
        assert "HTTP::release" in ws[0].message


# Output sinks (T101)


class TestOutputSinks:
    """Tainted data flowing into output commands should warn."""

    def test_puts_tainted_data(self):
        ws = _taint_warnings("set x [read $fd]\nputs $x", "T101")
        assert len(ws) >= 1
        assert ws[0].variable == "x"
        assert "puts" in ws[0].sink_command

    def test_puts_literal_clean(self):
        ws = _taint_warnings('puts "hello world"', "T101")
        assert len(ws) == 0

    def test_puts_sanitised_clean(self):
        ws = _taint_warnings(
            "set x [read $fd]\nset n [string length $x]\nputs $n",
            "T101",
        )
        assert len(ws) == 0

    def test_puts_interpolation_propagates(self):
        ws = _taint_warnings(
            'set x [read $fd]\nputs "data: $x"',
            "T101",
        )
        assert len(ws) >= 1


# Option injection (T102)


class TestOptionInjection:
    """Tainted data in option position without '--' should warn."""

    def test_regexp_tainted_without_terminator(self):
        ws = _taint_warnings(
            "set x [read $fd]\nregexp $x test",
            "T102",
        )
        assert len(ws) >= 1
        assert "regexp" in ws[0].sink_command

    def test_regexp_tainted_with_terminator(self):
        ws = _taint_warnings(
            "set x [read $fd]\nregexp -- $x test",
            "T102",
        )
        assert len(ws) == 0

    def test_glob_tainted_without_terminator(self):
        ws = _taint_warnings(
            "set x [read $fd]\nglob $x",
            "T102",
        )
        assert len(ws) >= 1

    def test_glob_tainted_with_terminator(self):
        ws = _taint_warnings(
            "set x [read $fd]\nglob -- $x",
            "T102",
        )
        assert len(ws) == 0

    def test_literal_no_warning(self):
        """Literal values should not trigger T102."""
        ws = _taint_warnings("regexp {^/api} test", "T102")
        assert len(ws) == 0

    def test_untainted_var_no_warning(self):
        """Non-tainted variable should not trigger T102."""
        ws = _taint_warnings('set x "pattern"\nregexp $x test', "T102")
        assert len(ws) == 0


# iRules HTTP output sinks (IRULE3001/3002)


class TestIRulesOutputSinks:
    """Tainted data in HTTP responses should warn in iRules dialect."""

    def test_http_respond_tainted_body(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set data [HTTP::payload]\nHTTP::respond 200 content $data",
            "IRULE3001",
        )
        assert len(ws) >= 1
        assert "HTTP::respond" in ws[0].sink_command

    def test_http_respond_literal_clean(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            'HTTP::respond 200 content "ok"',
            "IRULE3001",
        )
        assert len(ws) == 0

    def test_http_header_insert_tainted(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set val [HTTP::header Host]\nHTTP::header insert X-Foo $val",
            "IRULE3002",
        )
        assert len(ws) >= 1
        assert "header" in ws[0].sink_command.lower()

    def test_http_header_remove_clean(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "HTTP::header remove X-Bad",
            "IRULE3002",
        )
        assert len(ws) == 0

    def test_http_cookie_insert_tainted(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            'set val [HTTP::cookie value session]\nHTTP::cookie insert name "track" value $val',
            "IRULE3002",
        )
        assert len(ws) >= 1

    def test_not_in_tcl86_dialect(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl8.6")
        ws = _taint_warnings(
            "set data [read $fd]\nHTTP::respond 200 content $data",
            "IRULE3001",
        )
        assert len(ws) == 0


# Log injection (IRULE3003)


class TestLogInjection:
    """Tainted data in log commands should warn in iRules dialect."""

    def test_log_tainted_data(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set x [HTTP::uri]\nlog local0. $x",
            "IRULE3003",
        )
        assert len(ws) >= 1
        assert "log" in ws[0].sink_command

    def test_log_literal_clean(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings('log local0. "static message"', "IRULE3003")
        assert len(ws) == 0

    def test_log_not_in_tcl86(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl8.6")
        ws = _taint_warnings(
            "set x [read $fd]\nlog local0. $x",
            "IRULE3003",
        )
        assert len(ws) == 0


# Taint colour propagation


class TestTaintColours:
    """PATH_PREFIXED colour should propagate through variable copies."""

    def test_http_uri_getter_has_path_colour(self):
        """HTTP::uri getter (0 args) should produce PATH_PREFIXED taint."""
        # T100 is still raised (it's a dangerous sink regardless of colour).
        ws = _taint_warnings("set uri [HTTP::uri]\neval $uri", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "uri"

    def test_http_path_getter_has_path_colour(self):
        """HTTP::path getter should also produce PATH_PREFIXED taint."""
        ws = _taint_warnings("set p [HTTP::path]\neval $p", "T100")
        assert len(ws) >= 1
        assert ws[0].variable == "p"

    def test_colour_propagates_through_copy(self):
        """PATH_PREFIXED should propagate through a variable copy."""
        # HTTP::uri → x → y, both should still be tainted (T100 at eval).
        ws = _taint_warnings(
            "set x [HTTP::uri]\nset y $x\neval $y",
            "T100",
        )
        assert any(w.variable == "y" for w in ws)

    def test_non_dash_literal_prefix_stays_t102_safe(self):
        """Interpolation with fixed non-dash prefix stays safe for T102."""
        # "prefix${uri}" no longer starts with "/", but still cannot start with "-".
        ws = _taint_warnings(
            'set uri [HTTP::uri]\nset z "prefix${uri}"\nregexp $z test',
            "T102",
        )
        assert len(ws) == 0

    def test_colour_kept_when_dynamic_leading_piece_is_path_prefixed(self):
        """${uri}/... keeps PATH_PREFIXED because the leading piece is uri."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nset z ${uri}/suffix\nregexp $z test",
            "T102",
        )
        assert len(ws) == 0

    def test_leading_slash_concat_sets_path_prefixed(self):
        """A literal leading slash keeps option-injection-safe PATH_PREFIXED."""
        ws = _taint_warnings(
            "set h [HTTP::header Host]\nset z /${h}\nregexp $z test",
            "T102",
        )
        assert len(ws) == 0


# T102 suppression for PATH_PREFIXED


class TestT102Suppression:
    """T102 should be suppressed when tainted value has PATH_PREFIXED colour."""

    def test_http_uri_no_t102(self):
        """HTTP::uri always starts with '/' — no option injection possible."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nregexp $uri test",
            "T102",
        )
        assert len(ws) == 0

    def test_http_path_no_t102(self):
        """HTTP::path always starts with '/' — no option injection possible."""
        ws = _taint_warnings(
            "set p [HTTP::path]\nregexp $p test",
            "T102",
        )
        assert len(ws) == 0

    def test_literal_prefix_concat_no_t102(self):
        """A fixed non-dash literal prefix remains option-injection-safe."""
        ws = _taint_warnings(
            'set foo "path_[HTTP::path]"\nregexp $foo test',
            "T102",
        )
        assert len(ws) == 0

    def test_literal_dash_prefix_concat_still_warns(self):
        """A fixed '-' prefix is still option-like and should warn."""
        ws = _taint_warnings(
            'set foo "-[HTTP::path]"\nregexp $foo test',
            "T102",
        )
        assert len(ws) >= 1

    def test_generic_taint_still_warns(self):
        """Generic tainted data should still trigger T102."""
        ws = _taint_warnings(
            "set x [read $fd]\nregexp $x test",
            "T102",
        )
        assert len(ws) >= 1

    def test_http_header_still_warns(self):
        """HTTP::header is generic-tainted → should trigger T102."""
        ws = _taint_warnings(
            "set h [HTTP::header Host]\nregexp $h test",
            "T102",
        )
        assert len(ws) >= 1

    def test_path_prefixed_copy_suppresses(self):
        """PATH_PREFIXED propagated through copy should still suppress T102."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nset x $uri\nregexp $x test",
            "T102",
        )
        assert len(ws) == 0

    def test_ip_client_addr_no_t102(self):
        """IP::client_addr starts with digit/colon — no option injection."""
        ws = _taint_warnings(
            "set addr [IP::client_addr]\nregexp $addr test",
            "T102",
        )
        assert len(ws) == 0

    def test_ip_remote_addr_no_t102(self):
        """IP::remote_addr suppresses T102."""
        ws = _taint_warnings(
            "set addr [IP::remote_addr]\nregexp $addr test",
            "T102",
        )
        assert len(ws) == 0

    def test_tcp_client_port_no_t102(self):
        """TCP::client_port is always numeric — no option injection."""
        ws = _taint_warnings(
            "set port [TCP::client_port]\nregexp $port test",
            "T102",
        )
        assert len(ws) == 0

    def test_tcp_remote_port_no_t102(self):
        """TCP::remote_port suppresses T102."""
        ws = _taint_warnings(
            "set port [TCP::remote_port]\nregexp $port test",
            "T102",
        )
        assert len(ws) == 0

    def test_ssl_sni_no_t102(self):
        """SSL::sni is an FQDN — starts with letter/digit, not '-'."""
        ws = _taint_warnings(
            "set sni [SSL::sni]\nregexp $sni test",
            "T102",
        )
        assert len(ws) == 0

    def test_ip_addr_copy_preserves_colour(self):
        """IP_ADDRESS propagated through copy still suppresses T102."""
        ws = _taint_warnings(
            "set addr [IP::client_addr]\nset x $addr\nregexp $x test",
            "T102",
        )
        assert len(ws) == 0

    def test_ip_addr_still_fires_t100(self):
        """IP_ADDRESS does NOT suppress T100 — still dangerous for eval."""
        ws = _taint_warnings(
            "set addr [IP::client_addr]\neval $addr",
            "T100",
        )
        assert len(ws) >= 1

    def test_port_still_fires_t100(self):
        """PORT does NOT suppress T100."""
        ws = _taint_warnings(
            "set port [TCP::client_port]\neval $port",
            "T100",
        )
        assert len(ws) >= 1

    def test_phi_same_colour_preserves(self):
        """Both branches IP_ADDRESS → colour preserved, no T102."""
        source = """\
set a [IP::client_addr]
set b [IP::remote_addr]
if {1} {
    set x $a
} else {
    set x $b
}
regexp $x test
"""
        ws = _taint_warnings(source, "T102")
        assert len(ws) == 0

    def test_phi_mixed_colours_preserves_non_dash(self):
        """IP_ADDRESS and PORT both augment to NON_DASH_PREFIXED; T102 safe."""
        source = """\
set a [IP::client_addr]
set p [TCP::client_port]
if {$cond} {
    set x $a
} else {
    set x $p
}
regexp $x test
"""
        ws = _taint_warnings(source, "T102")
        assert len(ws) == 0

    def test_phi_generic_with_coloured_loses(self):
        """One branch generic taint, other IP_ADDRESS → T102 fires."""
        source = """\
set a [IP::client_addr]
set g [read $fd]
if {$cond} {
    set x $a
} else {
    set x $g
}
regexp $x test
"""
        ws = _taint_warnings(source, "T102")
        assert len(ws) >= 1


# Setter constraints (IRULE3101)


class TestSetterConstraints:
    """IRULE3101: HTTP::uri / HTTP::path set to value without '/' prefix."""

    def test_literal_slash_prefix_clean(self):
        """Literal value starting with '/' should not warn."""
        ws = _taint_warnings("HTTP::uri /newpath", "IRULE3101")
        assert len(ws) == 0

    def test_literal_no_slash_warns(self):
        """Literal value not starting with '/' should warn."""
        ws = _taint_warnings("HTTP::uri newpath", "IRULE3101")
        assert len(ws) == 1
        assert "HTTP::uri" in ws[0].sink_command

    def test_path_prefixed_var_clean(self):
        """Variable with PATH_PREFIXED colour should not warn."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nHTTP::uri $uri",
            "IRULE3101",
        )
        assert len(ws) == 0

    def test_generic_tainted_var_warns(self):
        """Generic tainted variable should warn."""
        ws = _taint_warnings(
            "set x [read $fd]\nHTTP::uri $x",
            "IRULE3101",
        )
        assert len(ws) == 1

    def test_http_path_literal_clean(self):
        """HTTP::path with '/' prefix literal should not warn."""
        ws = _taint_warnings("HTTP::path /foo/bar", "IRULE3101")
        assert len(ws) == 0

    def test_http_path_literal_warns(self):
        """HTTP::path without '/' prefix should warn."""
        ws = _taint_warnings("HTTP::path relative/path", "IRULE3101")
        assert len(ws) == 1

    def test_path_to_path_clean(self):
        """HTTP::path set from HTTP::path getter is clean."""
        ws = _taint_warnings(
            "set p [HTTP::path]\nHTTP::path $p",
            "IRULE3101",
        )
        assert len(ws) == 0

    def test_untainted_var_warns(self):
        """Untainted variable without known prefix warns (can't prove '/')."""
        ws = _taint_warnings(
            'set x "something"\nHTTP::uri $x',
            "IRULE3101",
        )
        assert len(ws) == 1


# Lattice join — new colours

_CRLF_FREE = TaintLattice.of(TaintColour.TAINTED | TaintColour.CRLF_FREE)
_SHELL_ATOM = TaintLattice.of(TaintColour.TAINTED | TaintColour.SHELL_ATOM)
_LIST_CANONICAL = TaintLattice.of(TaintColour.TAINTED | TaintColour.LIST_CANONICAL)
_REGEX_LITERAL = TaintLattice.of(TaintColour.TAINTED | TaintColour.REGEX_LITERAL)
_PATH_NORMALISED = TaintLattice.of(TaintColour.TAINTED | TaintColour.PATH_NORMALISED)
_HEADER_TOKEN_SAFE = TaintLattice.of(TaintColour.TAINTED | TaintColour.HEADER_TOKEN_SAFE)
_HTML_ESCAPED = TaintLattice.of(TaintColour.TAINTED | TaintColour.HTML_ESCAPED)
_URL_ENCODED = TaintLattice.of(TaintColour.TAINTED | TaintColour.URL_ENCODED)
_NON_DASH = TaintLattice.of(TaintColour.TAINTED | TaintColour.NON_DASH_PREFIXED)


class TestLatticeJoinNewColours:
    """Lattice join semantics for the new taint colours."""

    def test_crlf_free_self(self):
        r = taint_join(_CRLF_FREE, _CRLF_FREE)
        assert r.tainted and bool(r.colour & TaintColour.CRLF_FREE)

    def test_crlf_free_with_generic_loses(self):
        r = taint_join(_CRLF_FREE, _TAINTED)
        assert r.tainted and not bool(r.colour & TaintColour.CRLF_FREE)

    def test_crlf_free_with_untainted(self):
        r = taint_join(_CRLF_FREE, _UNTAINTED)
        assert r.tainted and bool(r.colour & TaintColour.CRLF_FREE)

    def test_shell_atom_self(self):
        r = taint_join(_SHELL_ATOM, _SHELL_ATOM)
        assert r.tainted and bool(r.colour & TaintColour.SHELL_ATOM)

    def test_shell_atom_with_generic_loses(self):
        r = taint_join(_SHELL_ATOM, _TAINTED)
        assert not bool(r.colour & TaintColour.SHELL_ATOM)

    def test_list_canonical_self(self):
        r = taint_join(_LIST_CANONICAL, _LIST_CANONICAL)
        assert bool(r.colour & TaintColour.LIST_CANONICAL)

    def test_regex_literal_with_html_escaped_loses_both(self):
        r = taint_join(_REGEX_LITERAL, _HTML_ESCAPED)
        assert not bool(r.colour & TaintColour.REGEX_LITERAL)
        assert not bool(r.colour & TaintColour.HTML_ESCAPED)

    def test_path_normalised_self(self):
        r = taint_join(_PATH_NORMALISED, _PATH_NORMALISED)
        assert bool(r.colour & TaintColour.PATH_NORMALISED)

    def test_header_token_safe_self(self):
        r = taint_join(_HEADER_TOKEN_SAFE, _HEADER_TOKEN_SAFE)
        assert bool(r.colour & TaintColour.HEADER_TOKEN_SAFE)

    def test_html_escaped_self(self):
        r = taint_join(_HTML_ESCAPED, _HTML_ESCAPED)
        assert bool(r.colour & TaintColour.HTML_ESCAPED)

    def test_url_encoded_self(self):
        r = taint_join(_URL_ENCODED, _URL_ENCODED)
        assert bool(r.colour & TaintColour.URL_ENCODED)

    def test_url_encoded_with_generic_loses(self):
        r = taint_join(_URL_ENCODED, _TAINTED)
        assert not bool(r.colour & TaintColour.URL_ENCODED)

    def test_multi_colour_join_preserves_shared(self):
        """Two values with CRLF_FREE|URL_ENCODED keep both at join."""
        both = TaintLattice.of(
            TaintColour.TAINTED | TaintColour.CRLF_FREE | TaintColour.URL_ENCODED,
        )
        r = taint_join(both, both)
        assert bool(r.colour & TaintColour.CRLF_FREE)
        assert bool(r.colour & TaintColour.URL_ENCODED)

    def test_multi_colour_partial_overlap(self):
        """One has CRLF_FREE|URL_ENCODED, other only CRLF_FREE → keep CRLF_FREE."""
        a = TaintLattice.of(
            TaintColour.TAINTED | TaintColour.CRLF_FREE | TaintColour.URL_ENCODED,
        )
        r = taint_join(a, _CRLF_FREE)
        assert bool(r.colour & TaintColour.CRLF_FREE)
        assert not bool(r.colour & TaintColour.URL_ENCODED)


# CRLF_FREE — producers, consumers, interpolation


class TestCrlfFree:
    """CRLF_FREE colour: suppresses IRULE3002 and IRULE3003."""

    def test_ip_client_addr_augments_crlf_free(self):
        """IP::client_addr → augmented with CRLF_FREE; suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set addr [IP::client_addr]\nlog local0. $addr",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_tcp_client_port_augments_crlf_free(self):
        """TCP::client_port → augmented with CRLF_FREE; suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set port [TCP::client_port]\nlog local0. $port",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_ssl_sni_augments_crlf_free(self):
        """SSL::sni (FQDN) → augmented with CRLF_FREE; suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set sni [SSL::sni]\nlog local0. $sni",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_uri_encode_adds_crlf_free(self):
        """URI::encode strips CR/LF; suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_html_encode_adds_crlf_free(self):
        """HTML::encode strips CR/LF; suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nlog local0. $enc",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_generic_taint_irule3003_still_warns(self):
        """Generic tainted data in log without CRLF_FREE still fires."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nlog local0. $raw",
            "IRULE3003",
        )
        assert len(ws) >= 1

    def test_crlf_free_suppresses_irule3002(self):
        """CRLF_FREE value in header insert suppresses IRULE3002."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::header Host]\nset enc [URI::encode $raw]\n"
            "HTTP::header insert X-Fwd $enc",
            "IRULE3002",
        )
        assert len(ws) == 0

    def test_crlf_free_survives_safe_concat(self):
        """CRLF_FREE from IP addr survives safe prefix/suffix interpolation."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            'set addr [IP::client_addr]\nset msg "src=${addr}:80"\nlog local0. $msg',
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_interpolation_without_crlf_preserves(self):
        """Interpolation without literal CRLF preserves CRLF_FREE."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            'set addr [IP::client_addr]\nset x "client:${addr}"\nlog local0. $x',
            "IRULE3003",
        )
        assert len(ws) == 0


# SHELL_ATOM — producer augmentation


class TestShellAtom:
    """SHELL_ATOM colour: augmented from IP/PORT/FQDN sources."""

    def test_ip_addr_augments_shell_atom(self):
        """IP addresses are shell-safe atoms."""
        ws = _taint_warnings(
            "set addr [IP::client_addr]\neval $addr",
            "T100",
        )
        # SHELL_ATOM doesn't suppress T100 (still dangerous)
        assert len(ws) >= 1

    def test_port_augments_shell_atom(self):
        """Port numbers are shell-safe atoms."""
        ws = _taint_warnings(
            "set port [TCP::client_port]\neval $port",
            "T100",
        )
        assert len(ws) >= 1

    def test_fqdn_augments_shell_atom(self):
        """FQDNs are shell-safe atoms."""
        ws = _taint_warnings(
            "set sni [SSL::sni]\neval $sni",
            "T100",
        )
        assert len(ws) >= 1

    def test_shell_atom_lost_at_phi_with_generic(self):
        """SHELL_ATOM lost when joined with generic taint."""
        source = """\
set a [IP::client_addr]
set g [read $fd]
if {$cond} {
    set x $a
} else {
    set x $g
}
eval $x
"""
        # Both paths tainted → T100 fires regardless
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1


# LIST_CANONICAL — producers and interpolation invalidation


class TestListCanonical:
    """LIST_CANONICAL colour: produced by list/concat, lost on interpolation."""

    def test_list_command_propagates_taint_to_eval_suppressed(self):
        """[list] wrapping → LIST_CANONICAL → eval suppressed (T100)."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset lst [list $raw]\neval $lst",
            "T100",
        )
        assert len(ws) == 0  # suppressed by LIST_CANONICAL

    def test_list_command_propagates_taint_to_puts(self):
        """[list] wrapping → taint still flows to non-eval sinks (T101)."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset lst [list $raw]\nputs $lst",
            "T101",
        )
        assert len(ws) >= 1

    def test_concat_of_canonical_lists_eval_suppressed(self):
        """concat of two canonical lists keeps LIST_CANONICAL → eval suppressed."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset a [list $raw]\n"
            "set raw2 [read $fd2]\nset b [list $raw2]\n"
            "set c [concat $a $b]\neval $c",
            "T100",
        )
        assert len(ws) == 0  # suppressed by LIST_CANONICAL

    def test_interpolation_preserves_taint_from_list(self):
        """String interpolation of list-wrapped tainted data keeps taint."""
        ws = _taint_warnings(
            'set raw [read $fd]\nset lst [list $raw]\nset broken "prefix $lst"\neval $broken',
            "T100",
        )
        assert len(ws) >= 1

    def test_list_canonical_copy_preserves_colour(self):
        """LIST_CANONICAL propagates through variable copy → eval suppressed."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\neval $copy",
            "T100",
        )
        assert len(ws) == 0  # LIST_CANONICAL preserved through copy

    def test_list_canonical_copy_still_tainted(self):
        """LIST_CANONICAL copy is still tainted for non-eval sinks."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset lst [list $raw]\nset copy $lst\nputs $copy",
            "T101",
        )
        assert len(ws) >= 1  # taint flows to puts


# REGEX_LITERAL — producer and interpolation invalidation


class TestRegexLiteral:
    """REGEX_LITERAL colour: produced by regex::quote, lost on interpolation."""

    def test_regex_quote_produces_colour(self):
        """regex::quote on tainted input adds REGEX_LITERAL."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset safe [regex::quote $raw]\neval $safe",
            "T100",
        )
        assert len(ws) >= 1

    def test_regexp_quote_produces_colour(self):
        """regexp::quote on tainted input adds REGEX_LITERAL."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset safe [regexp::quote $raw]\neval $safe",
            "T100",
        )
        assert len(ws) >= 1

    def test_interpolation_invalidates_regex_literal(self):
        """String interpolation destroys REGEX_LITERAL guarantee."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset safe [regex::quote $raw]\n"
            'set pat "prefix${safe}suffix"\neval $pat',
            "T100",
        )
        assert len(ws) >= 1

    def test_regex_literal_propagates_through_copy(self):
        """REGEX_LITERAL propagates through variable copy."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset safe [regex::quote $raw]\nset copy $safe\neval $copy",
            "T100",
        )
        assert len(ws) >= 1


# PATH_NORMALISED — producer and interpolation invalidation


class TestPathNormalised:
    """PATH_NORMALISED colour: produced by file normalize."""

    def test_file_normalize_produces_colour(self):
        """file normalize adds PATH_NORMALISED colour."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset norm [file normalize $raw]\neval $norm",
            "T100",
        )
        assert len(ws) >= 1

    def test_interpolation_invalidates_path_normalised(self):
        """Interpolation destroys PATH_NORMALISED guarantee."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset norm [file normalize $raw]\n"
            'set broken "${norm}/extra"\neval $broken',
            "T100",
        )
        assert len(ws) >= 1

    def test_path_normalised_propagates_through_copy(self):
        """PATH_NORMALISED propagates through variable copy."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset norm [file normalize $raw]\nset copy $norm\neval $copy",
            "T100",
        )
        assert len(ws) >= 1


# HTML_ESCAPED — producer and IRULE3001 suppression


class TestHtmlEscaped:
    """HTML_ESCAPED colour: suppresses IRULE3001 (XSS in HTTP::respond)."""

    def test_html_encode_produces_colour(self):
        """HTML::encode adds HTML_ESCAPED colour."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::respond 200 content $safe",
            "IRULE3001",
        )
        assert len(ws) == 0

    def test_generic_taint_irule3001_fires(self):
        """Generic taint in HTTP::respond body fires IRULE3001."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nHTTP::respond 200 content $raw",
            "IRULE3001",
        )
        assert len(ws) >= 1

    def test_interpolation_invalidates_html_escaped(self):
        """Interpolation destroys HTML_ESCAPED — IRULE3001 fires again."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\n"
            'set broken "<b>${safe}</b>"\n'
            "HTTP::respond 200 content $broken",
            "IRULE3001",
        )
        assert len(ws) >= 1

    def test_html_escaped_propagates_through_copy(self):
        """HTML_ESCAPED propagates through variable copy."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\n"
            "set copy $safe\nHTTP::respond 200 content $copy",
            "IRULE3001",
        )
        assert len(ws) == 0

    def test_html_encode_also_sets_crlf_free(self):
        """HTML::encode also adds CRLF_FREE — suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nlog local0. $safe",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_html_encode_recognised_as_sanitiser(self):
        """html_encode (portable helper) produces HTML_ESCAPED colour."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [html_encode $raw]\nHTTP::respond 200 content $safe",
            "IRULE3001",
        )
        assert len(ws) == 0

    def test_html_encode_produces_crlf_free(self):
        """html_encode also adds CRLF_FREE — suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset safe [html_encode $raw]\nlog local0. $safe",
            "IRULE3003",
        )
        assert len(ws) == 0

    def test_html_encode_double_encode_detected(self):
        """T106 fires for html_encode on already HTML_ESCAPED data."""
        source = "set raw [read $fd]\nset safe [HTML::encode $raw]\nset dup [html_encode $safe]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1


# URL_ENCODED — producer and interpolation


class TestUrlEncoded:
    """URL_ENCODED colour: produced by URI::encode."""

    def test_uri_encode_produces_colour(self):
        """URI::encode adds URL_ENCODED colour."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset enc [URI::encode $raw]\neval $enc",
            "T100",
        )
        assert len(ws) >= 1

    def test_uri_encode_component_produces_colour(self):
        """URI::encode_component also adds URL_ENCODED."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset enc [URI::encode_component $raw]\neval $enc",
            "T100",
        )
        assert len(ws) >= 1

    def test_uri_escape_produces_colour(self):
        """URI::escape also adds URL_ENCODED."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset enc [URI::escape $raw]\neval $enc",
            "T100",
        )
        assert len(ws) >= 1

    def test_interpolation_invalidates_url_encoded(self):
        """Interpolation destroys URL_ENCODED guarantee."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset enc [URI::encode $raw]\n"
            'set broken "prefix${enc}"\neval $broken',
            "T100",
        )
        assert len(ws) >= 1

    def test_url_encoded_propagates_through_copy(self):
        """URL_ENCODED propagates through variable copy."""
        ws = _taint_warnings(
            "set raw [read $fd]\nset enc [URI::encode $raw]\nset copy $enc\neval $copy",
            "T100",
        )
        assert len(ws) >= 1

    def test_uri_encode_also_sets_crlf_free(self):
        """URI::encode adds both URL_ENCODED and CRLF_FREE."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set raw [HTTP::query]\nset enc [URI::encode $raw]\nlog local0. $enc",
            "IRULE3003",
        )
        assert len(ws) == 0


# HEADER_TOKEN_SAFE — IRULE3002 position-aware suppression


class TestHeaderTokenSafe:
    """HEADER_TOKEN_SAFE: suppresses IRULE3002 only in header-name position."""

    # Note: HEADER_TOKEN_SAFE has no built-in producer yet — test via
    # CRLF_FREE which universally suppresses IRULE3002.

    def test_crlf_free_in_header_value_suppresses_irule3002(self):
        """CRLF_FREE in value position of HTTP::header insert suppresses."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set val [IP::client_addr]\nHTTP::header insert X-Fwd $val",
            "IRULE3002",
        )
        assert len(ws) == 0

    def test_crlf_free_in_cookie_value_suppresses_irule3002(self):
        """CRLF_FREE in cookie value suppresses IRULE3002."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            'set val [IP::client_addr]\nHTTP::cookie insert name "track" value $val',
            "IRULE3002",
        )
        assert len(ws) == 0

    def test_generic_taint_in_header_value_warns(self):
        """Generic taint in header value position fires IRULE3002."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        ws = _taint_warnings(
            "set val [HTTP::header Host]\nHTTP::header insert X-Fwd $val",
            "IRULE3002",
        )
        assert len(ws) >= 1


# Source colour augmentation (IP/PORT/FQDN derived properties)


class TestSourceColourAugmentation:
    """IP_ADDRESS, PORT, FQDN sources get NON_DASH_PREFIXED + CRLF_FREE + SHELL_ATOM."""

    def test_ip_client_addr_non_dash(self):
        """IP::client_addr → NON_DASH_PREFIXED (suppresses T102)."""
        ws = _taint_warnings(
            "set addr [IP::client_addr]\nregexp $addr test",
            "T102",
        )
        assert len(ws) == 0

    def test_tcp_client_port_non_dash(self):
        """TCP::client_port → NON_DASH_PREFIXED (suppresses T102)."""
        ws = _taint_warnings(
            "set port [TCP::client_port]\nregexp $port test",
            "T102",
        )
        assert len(ws) == 0

    def test_ssl_sni_non_dash(self):
        """SSL::sni → NON_DASH_PREFIXED (suppresses T102)."""
        ws = _taint_warnings(
            "set sni [SSL::sni]\nregexp $sni test",
            "T102",
        )
        assert len(ws) == 0

    def test_path_prefixed_augments_non_dash(self):
        """PATH_PREFIXED (HTTP::uri) → augmented with NON_DASH_PREFIXED."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nregexp $uri test",
            "T102",
        )
        assert len(ws) == 0

    def test_generic_taint_no_augmentation(self):
        """Generic taint (read) gets no augmentation — T102 fires."""
        ws = _taint_warnings(
            "set raw [read $fd]\nregexp $raw test",
            "T102",
        )
        assert len(ws) >= 1


# NON_DASH_PREFIXED — interpolation edge cases


class TestNonDashPrefixedInterpolation:
    """NON_DASH_PREFIXED tracking through string interpolation."""

    def test_literal_alpha_prefix(self):
        """'key_${tainted}' starts with 'k' → NON_DASH_PREFIXED."""
        ws = _taint_warnings(
            'set x [HTTP::header Host]\nset y "key_${x}"\nregexp $y test',
            "T102",
        )
        assert len(ws) == 0

    def test_literal_digit_prefix(self):
        """'0${tainted}' starts with '0' → NON_DASH_PREFIXED."""
        ws = _taint_warnings(
            'set x [HTTP::header Host]\nset y "0${x}"\nregexp $y test',
            "T102",
        )
        assert len(ws) == 0

    def test_literal_slash_prefix(self):
        """'/${tainted}' starts with '/' → PATH_PREFIXED + NON_DASH_PREFIXED."""
        ws = _taint_warnings(
            'set x [HTTP::header Host]\nset y "/${x}"\nregexp $y test',
            "T102",
        )
        assert len(ws) == 0

    def test_literal_dash_prefix_warns(self):
        """'-${tainted}' starts with '-' → T102 fires."""
        ws = _taint_warnings(
            'set x [HTTP::header Host]\nset y "-${x}"\nregexp $y test',
            "T102",
        )
        assert len(ws) >= 1

    def test_dynamic_leading_piece_inherits_original(self):
        """'${uri}/suffix' with PATH_PREFIXED keeps PATH_PREFIXED."""
        ws = _taint_warnings(
            "set uri [HTTP::uri]\nset z ${uri}/suffix\nregexp $z test",
            "T102",
        )
        assert len(ws) == 0

    def test_command_subst_prefix(self):
        """'path_[HTTP::path]' starts with 'p' → NON_DASH_PREFIXED."""
        ws = _taint_warnings(
            'set z "path_[HTTP::path]"\nregexp $z test',
            "T102",
        )
        assert len(ws) == 0


# Interprocedural taint with new colours


class TestInterproceduralColours:
    """Colour-aware interprocedural taint propagation."""

    def test_helper_returning_uri_encode_suppresses_irule3003(self):
        """Proc that URI::encode's its arg returns CRLF_FREE."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = """\
proc encode_it {x} { return [URI::encode $x] }
set raw [HTTP::query]
set safe [encode_it $raw]
log local0. $safe
"""
        ws = _taint_warnings(source, "IRULE3003")
        assert len(ws) == 0

    def test_helper_returning_html_encode_suppresses_irule3001(self):
        """Proc that HTML::encode's its arg returns HTML_ESCAPED."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = """\
proc html_safe {x} { return [HTML::encode $x] }
set raw [HTTP::query]
set safe [html_safe $raw]
HTTP::respond 200 content $safe
"""
        ws = _taint_warnings(source, "IRULE3001")
        assert len(ws) == 0

    def test_helper_passthrough_generic_taint_fires_t102(self):
        """Proc that passes through generic taint fires T102."""
        source = """\
proc identity {x} { return $x }
set raw [read $fd]
set x [identity $raw]
regexp $x test
"""
        ws = _taint_warnings(source, "T102")
        assert len(ws) >= 1

    def test_helper_list_wrapper_adds_canonical(self):
        """Proc that wraps in list adds LIST_CANONICAL — suppresses T100 for eval."""
        source = """\
proc wrap_list {x} { return [list $x] }
set raw [read $fd]
set lst [wrap_list $raw]
eval $lst
"""
        # LIST_CANONICAL from [list] propagated through proc → suppresses T100 for eval
        ws = _taint_warnings(source, "T100")
        assert len(ws) == 0

    def test_helper_list_wrapper_still_tainted(self):
        """Proc that wraps in list still propagates taint to non-eval sinks."""
        source = """\
proc wrap_list {x} { return [list $x] }
set raw [read $fd]
set lst [wrap_list $raw]
puts $lst
"""
        ws = _taint_warnings(source, "T101")
        assert len(ws) >= 1

    def test_helper_with_ip_addr_param_augmented(self):
        """Proc receiving IP_ADDRESS param still has augmented colours."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = """\
proc log_addr {addr} { log local0. $addr }
set a [IP::client_addr]
log_addr $a
"""
        ws = _taint_warnings(source, "IRULE3003")
        assert len(ws) == 0


# Interpolation colour invalidation — comprehensive


class TestInterpolationColourInvalidation:
    """Structural colours are stripped on interpolation; CRLF_FREE preserved."""

    def test_list_canonical_stripped(self):
        """LIST_CANONICAL lost on interpolation."""
        source = 'set x [list [read $fd]]\nset y "pre $x suf"\neval $y'
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_path_normalised_stripped(self):
        """PATH_NORMALISED lost on interpolation."""
        source = 'set x [file normalize [read $fd]]\nset y "pre $x"\neval $y'
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_html_escaped_stripped(self):
        """HTML_ESCAPED lost on interpolation."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            "set raw [HTTP::query]\nset safe [HTML::encode $raw]\n"
            'set broken "<b>$safe</b>"\n'
            "HTTP::respond 200 content $broken"
        )
        ws = _taint_warnings(source, "IRULE3001")
        assert len(ws) >= 1

    def test_url_encoded_stripped(self):
        """URL_ENCODED lost on interpolation."""
        source = 'set raw [read $fd]\nset enc [URI::encode $raw]\nset y "pre$enc"\neval $y'
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_regex_literal_stripped(self):
        """REGEX_LITERAL lost on interpolation."""
        source = 'set raw [read $fd]\nset q [regex::quote $raw]\nset y "^$q"\neval $y'
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_shell_atom_stripped(self):
        """SHELL_ATOM lost on interpolation."""
        source = 'set addr [IP::client_addr]\nset y "host:$addr"\neval $y'
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_crlf_free_preserved_without_literal_crlf(self):
        """CRLF_FREE preserved when interpolation adds no literal CR/LF."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set addr [IP::client_addr]\nset msg "client:${addr}"\nlog local0. $msg'
        ws = _taint_warnings(source, "IRULE3003")
        assert len(ws) == 0


# Transform colour producers — edge cases


class TestTransformColourEdgeCases:
    """Edge cases for _derive_transform_colours."""

    def test_list_with_literal_untainted(self):
        """[list] with literal input → untainted."""
        ws = _taint_warnings("set x [list hello]\neval $x", "T100")
        assert len(ws) == 0  # untainted

    def test_concat_of_mixed_canonicality(self):
        """concat of one canonical and one non-canonical → taint still propagates."""
        source = (
            "set raw [read $fd]\nset a [list $raw]\n"
            "set b [read $fd2]\n"
            "set c [concat $a $b]\n"
            "eval $c"
        )
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_uri_encode_with_untainted_no_colour(self):
        """URI::encode on untainted data → still untainted."""
        ws = _taint_warnings(
            'set enc [URI::encode "hello"]\neval $enc',
            "T100",
        )
        assert len(ws) == 0

    def test_file_normalize_with_untainted_no_colour(self):
        """file normalize on untainted → still untainted."""
        ws = _taint_warnings(
            'set norm [file normalize "/tmp/foo"]\neval $norm',
            "T100",
        )
        assert len(ws) == 0


# IRULE3001/3002/3003 suppression matrix — comprehensive


class TestSinkSuppressionMatrix:
    """Verify colour → diagnostic suppression matrix comprehensively."""

    def test_irule3001_not_suppressed_by_crlf_free(self):
        """CRLF_FREE does NOT suppress IRULE3001 (XSS)."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set raw [HTTP::query]\nset enc [URI::encode $raw]\nHTTP::respond 200 content $enc"
        ws = _taint_warnings(source, "IRULE3001")
        assert len(ws) >= 1  # URL_ENCODED doesn't suppress IRULE3001

    def test_irule3001_suppressed_by_html_escaped(self):
        """HTML_ESCAPED suppresses IRULE3001."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            "set raw [HTTP::query]\nset enc [HTML::encode $raw]\nHTTP::respond 200 content $enc"
        )
        ws = _taint_warnings(source, "IRULE3001")
        assert len(ws) == 0

    def test_irule3002_suppressed_by_ip_address(self):
        """IP_ADDRESS (augmented CRLF_FREE) suppresses IRULE3002."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set addr [IP::client_addr]\nHTTP::header insert X-Client $addr"
        ws = _taint_warnings(source, "IRULE3002")
        assert len(ws) == 0

    def test_irule3002_suppressed_by_port(self):
        """PORT (augmented CRLF_FREE) suppresses IRULE3002."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set port [TCP::client_port]\nHTTP::header insert X-Port $port"
        ws = _taint_warnings(source, "IRULE3002")
        assert len(ws) == 0

    def test_irule3003_suppressed_by_fqdn(self):
        """FQDN (augmented CRLF_FREE) suppresses IRULE3003."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set sni [SSL::sni]\nlog local0. $sni"
        ws = _taint_warnings(source, "IRULE3003")
        assert len(ws) == 0

    def test_t102_not_suppressed_by_crlf_free(self):
        """CRLF_FREE alone does NOT suppress T102."""
        source = "set raw [read $fd]\nset enc [URI::encode $raw]\nregexp $enc test"
        ws = _taint_warnings(source, "T102")
        assert len(ws) >= 1

    def test_t102_not_suppressed_by_html_escaped(self):
        """HTML_ESCAPED alone does NOT suppress T102."""
        source = "set raw [read $fd]\nset enc [HTML::encode $raw]\nregexp $enc test"
        ws = _taint_warnings(source, "T102")
        assert len(ws) >= 1

    def test_t100_not_suppressed_by_path_prefixed(self):
        """T100 (dangerous sinks) is not suppressed by PATH_PREFIXED."""
        source = "set raw [HTTP::uri]\neval $raw"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1


# T100 sink suppression via SHELL_ATOM / LIST_CANONICAL


class TestT100SinkSuppression:
    """Test that T100 is suppressed for exec+SHELL_ATOM and eval+LIST_CANONICAL."""

    def test_exec_with_shell_atom_suppressed(self):
        """exec with SHELL_ATOM tainted data → T100 suppressed."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        # IP::client_addr produces IP_ADDRESS which augments to SHELL_ATOM
        source = "set addr [IP::client_addr]\nexec ping $addr"
        ws = _taint_warnings(source, "T100")
        assert len(ws) == 0

    def test_exec_with_generic_taint_not_suppressed(self):
        """exec with generic tainted data → T100 fires."""
        source = "set raw [read $fd]\nexec echo $raw"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_eval_with_list_canonical_suppressed(self):
        """eval with LIST_CANONICAL tainted data → T100 suppressed."""
        source = "set raw [read $fd]\nset safe [list $raw]\neval $safe"
        ws = _taint_warnings(source, "T100")
        assert len(ws) == 0

    def test_eval_with_generic_taint_not_suppressed(self):
        """eval with generic tainted data → T100 fires."""
        source = "set raw [read $fd]\neval $raw"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_uplevel_with_list_canonical_suppressed(self):
        """uplevel with LIST_CANONICAL tainted data → T100 suppressed."""
        source = "set raw [read $fd]\nset safe [list $raw]\nuplevel $safe"
        ws = _taint_warnings(source, "T100")
        assert len(ws) == 0

    def test_uplevel_with_generic_taint_not_suppressed(self):
        """uplevel with generic tainted data → T100 fires."""
        source = "set raw [read $fd]\nuplevel $raw"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_subst_not_suppressed_by_shell_atom(self):
        """subst is never suppressed by SHELL_ATOM."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set addr [IP::client_addr]\nsubst $addr"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_expr_not_suppressed_by_list_canonical(self):
        """expr is never suppressed by LIST_CANONICAL."""
        source = "set raw [read $fd]\nset safe [list $raw]\nexpr $safe"
        ws = _taint_warnings(source, "T100")
        assert len(ws) >= 1

    def test_exec_with_port_suppressed(self):
        """exec with PORT tainted data → T100 suppressed (PORT augments to SHELL_ATOM)."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set port [TCP::client_port]\nexec firewall-cmd $port"
        ws = _taint_warnings(source, "T100")
        assert len(ws) == 0


# T103 — regexp/regsub pattern injection


class TestT103RegexpPatternInjection:
    """Test tainted data in regexp/regsub pattern position."""

    def test_tainted_regexp_pattern_fires(self):
        """Tainted data in regexp pattern position → T103."""
        source = "set pat [read $fd]\nregexp $pat teststring"
        ws = _taint_warnings(source, "T103")
        assert len(ws) >= 1
        assert ws[0].code == "T103"

    def test_tainted_regsub_pattern_fires(self):
        """Tainted data in regsub pattern position → T103."""
        source = "set pat [read $fd]\nregsub $pat teststring replacement"
        ws = _taint_warnings(source, "T103")
        assert len(ws) >= 1

    def test_tainted_regexp_with_options_fires(self):
        """Tainted pattern after options → T103."""
        source = "set pat [read $fd]\nregexp -nocase -- $pat teststring"
        ws = _taint_warnings(source, "T103")
        assert len(ws) >= 1

    def test_regex_literal_suppresses_t103(self):
        """REGEX_LITERAL colour suppresses T103."""
        source = "set raw [read $fd]\nset safe [regex::quote $raw]\nregexp $safe teststring"
        ws = _taint_warnings(source, "T103")
        assert len(ws) == 0

    def test_untainted_regexp_no_warning(self):
        """Literal pattern → no T103."""
        source = "regexp {^[a-z]+$} teststring"
        ws = _taint_warnings(source, "T103")
        assert len(ws) == 0

    def test_tainted_non_pattern_arg_no_t103(self):
        """Tainted data in string position (not pattern) → no T103."""
        source = "set input [read $fd]\nregexp {^\\d+$} $input"
        ws = _taint_warnings(source, "T103")
        assert len(ws) == 0

    def test_tainted_regsub_with_options_fires(self):
        """Tainted pattern after regsub -all -nocase -- → T103."""
        source = "set pat [read $fd]\nregsub -all -nocase -- $pat teststring replacement"
        ws = _taint_warnings(source, "T103")
        assert len(ws) >= 1

    def test_t103_message_format(self):
        """T103 warning message references the command and variable."""
        source = "set pat [read $fd]\nregexp $pat teststring"
        ws = _taint_warnings(source, "T103")
        assert len(ws) >= 1
        assert "pat" in ws[0].message
        assert "regexp" in ws[0].message or "regex" in ws[0].message.lower()


# PATH_NORMALISED in setter constraints (IRULE3101)


class TestPathNormalisedSetterConstraint:
    """PATH_NORMALISED satisfies the IRULE3101 setter constraint."""

    def test_path_normalised_suppresses_irule3101(self):
        """file normalize → PATH_NORMALISED → IRULE3101 suppressed."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set raw [HTTP::uri]\nset norm [file normalize $raw]\nHTTP::uri $norm"
        ws = _taint_warnings(source, "IRULE3101")
        assert len(ws) == 0

    def test_generic_taint_still_fires_irule3101(self):
        """Generic tainted variable without PATH_NORMALISED → IRULE3101 fires."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set raw [HTTP::query]\nHTTP::uri $raw"
        ws = _taint_warnings(source, "IRULE3101")
        assert len(ws) >= 1

    def test_path_prefixed_still_suppresses_irule3101(self):
        """PATH_PREFIXED (existing behaviour) still suppresses IRULE3101."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set path [HTTP::path]\nHTTP::uri $path"
        ws = _taint_warnings(source, "IRULE3101")
        assert len(ws) == 0


# IRULE3004 — Open redirect (HTTP::redirect)


class TestIrule3004OpenRedirect:
    """HTTP::redirect with tainted URL should warn about open redirect."""

    def test_tainted_redirect_fires(self):
        """Generic tainted data in HTTP::redirect fires IRULE3004."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set dest [HTTP::query]\nHTTP::redirect $dest"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) >= 1
        assert "redirect" in ws[0].message.lower()

    def test_tainted_header_redirect_fires(self):
        """Tainted header value in redirect fires IRULE3004."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set url [HTTP::header value Redirect-To]\nHTTP::redirect $url"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) >= 1

    def test_path_prefixed_suppresses_irule3004(self):
        """PATH_PREFIXED (relative redirect) suppresses IRULE3004."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set p [HTTP::path]\nHTTP::redirect $p"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) == 0

    def test_path_normalised_suppresses_irule3004(self):
        """PATH_NORMALISED suppresses IRULE3004."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set raw [HTTP::query]\nset norm [file normalize $raw]\nHTTP::redirect $norm"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) == 0

    def test_html_escaped_does_not_suppress_irule3004(self):
        """HTML_ESCAPED is wrong encoding for redirect — doesn't suppress."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set raw [HTTP::query]\nset safe [HTML::encode $raw]\nHTTP::redirect $safe"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) >= 1

    def test_literal_redirect_clean(self):
        """Literal redirect URL produces no warning."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'HTTP::redirect "https://example.com/home"'
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) == 0

    def test_not_in_tcl86(self):
        """IRULE3004 only fires in iRules dialect."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl8.6")
        source = "set dest [read $fd]\nHTTP::redirect $dest"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) == 0

    def test_message_format(self):
        """IRULE3004 message includes variable name and command."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = "set url [HTTP::query]\nHTTP::redirect $url"
        ws = _taint_warnings(source, "IRULE3004")
        assert len(ws) >= 1
        assert "$url" in ws[0].message
        assert "HTTP::redirect" in ws[0].message


# T106 — Double encoding


class TestT106DoubleEncoding:
    """Detect redundant double-encoding of already-encoded data."""

    def test_double_html_encode(self):
        """HTML::encode on HTML_ESCAPED data fires T106."""
        source = "set raw [read $fd]\nset safe [HTML::encode $raw]\nset double [HTML::encode $safe]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1
        assert "HTML-escaped" in ws[0].message

    def test_double_uri_encode(self):
        """URI::encode on URL_ENCODED data fires T106."""
        source = "set raw [read $fd]\nset enc [URI::encode $raw]\nset double [URI::encode $enc]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1
        assert "URL-encoded" in ws[0].message

    def test_double_regex_quote(self):
        """regex::quote on REGEX_LITERAL data fires T106."""
        source = "set raw [read $fd]\nset esc [regex::quote $raw]\nset double [regex::quote $esc]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1
        assert "regex-escaped" in ws[0].message

    def test_cross_encode_no_fire(self):
        """HTML::encode on URL_ENCODED data does NOT fire T106."""
        source = "set raw [read $fd]\nset enc [URI::encode $raw]\nset html [HTML::encode $enc]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) == 0

    def test_first_encode_no_fire(self):
        """First encoding of generic tainted data does NOT fire T106."""
        source = "set raw [read $fd]\nset safe [HTML::encode $raw]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) == 0

    def test_double_encode_through_copy(self):
        """Double-encoding detected even through variable copy."""
        source = (
            "set raw [read $fd]\n"
            "set enc [URI::encode $raw]\n"
            "set copy $enc\n"
            "set double [URI::encode $copy]"
        )
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1

    def test_untainted_no_fire(self):
        """Literal data through encoder does not fire T106."""
        source = 'set x [HTML::encode "hello"]\nset y [HTML::encode $x]'
        ws = _taint_warnings(source, "T106")
        assert len(ws) == 0

    def test_message_format(self):
        """T106 message includes variable, colour label, and command."""
        source = "set raw [read $fd]\nset enc [HTML::encode $raw]\nset dup [HTML::encode $enc]"
        ws = _taint_warnings(source, "T106")
        assert len(ws) >= 1
        assert "$enc" in ws[0].message
        assert "HTML::encode" in ws[0].message
        assert "HTML-escaped" in ws[0].message


# T104: SSRF via socket/network commands


class TestT104NetworkSinks:
    """T104 -- tainted data in network address argument (SSRF risk)."""

    def test_socket_tainted_host(self):
        """Tainted host in socket → T104."""
        source = "set host [read $fd]\nsocket $host 80"
        ws = _taint_warnings(source, "T104")
        assert len(ws) >= 1
        assert any(w.sink_command == "socket" for w in ws)

    def test_socket_literal_clean(self):
        """Literal host in socket → no T104."""
        ws = _taint_warnings("socket localhost 80", "T104")
        assert len(ws) == 0

    def test_http_geturl_tainted_url(self):
        """Tainted URL in http::geturl → T104."""
        source = "set url [read $fd]\nhttp::geturl $url"
        ws = _taint_warnings(source, "T104")
        assert len(ws) >= 1
        assert any(w.sink_command == "http::geturl" for w in ws)

    def test_http_geturl_literal_clean(self):
        """Literal URL in http::geturl → no T104."""
        ws = _taint_warnings('http::geturl "http://example.com"', "T104")
        assert len(ws) == 0

    def test_socket_propagation_through_copy(self):
        """Taint propagates through variable copy to socket."""
        source = "set host [read $fd]\nset h2 $host\nsocket $h2 443"
        ws = _taint_warnings(source, "T104")
        assert any(w.variable == "h2" for w in ws)

    def test_message_format(self):
        """T104 message mentions the variable and command."""
        source = "set host [read $fd]\nsocket $host 80"
        ws = _taint_warnings(source, "T104")
        assert len(ws) >= 1
        msg = ws[0].message
        assert "SSRF" in msg
        assert "socket" in msg


# T105: interp eval injection (taint-based)


class TestT105InterpEvalSinks:
    """T105 -- tainted data in interp eval script argument."""

    def test_interp_eval_tainted(self):
        """Tainted data in interp eval → T105."""
        source = "set script [read $fd]\ninterp eval $child $script"
        ws = _taint_warnings(source, "T105")
        assert len(ws) >= 1
        assert any("interp eval" in w.sink_command for w in ws)

    def test_interp_eval_literal_clean(self):
        """Literal script in interp eval → no T105."""
        ws = _taint_warnings('interp eval $child "puts hello"', "T105")
        assert len(ws) == 0

    def test_interp_invokehidden_tainted(self):
        """Tainted data in interp invokehidden → T105."""
        source = "set cmd [read $fd]\ninterp invokehidden $child $cmd"
        ws = _taint_warnings(source, "T105")
        assert len(ws) >= 1

    def test_list_suppresses_t105(self):
        """LIST_CANONICAL colour suppresses T105."""
        source = "set x [read $fd]\nset safe [list $x]\ninterp eval $child $safe"
        ws = _taint_warnings(source, "T105")
        assert len(ws) == 0

    def test_message_format(self):
        """T105 message mentions variable and interp eval."""
        source = "set data [read $fd]\ninterp eval $child $data"
        ws = _taint_warnings(source, "T105")
        assert len(ws) >= 1
        msg = ws[0].message
        assert "interp eval" in msg
        assert "injection" in msg.lower()


# IRULE3103 — HTTP::uri split on ? or &


class TestIrule3103UriSplit:
    """IRULE3103: splitting HTTP::uri on '?' or '&' instead of using HTTP::path/query."""

    def test_split_uri_on_question_mark(self):
        """Splitting HTTP::uri on '?' fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset parts [split $uri "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "HTTP::query" in ws[0].message

    def test_split_uri_on_ampersand(self):
        """Splitting HTTP::uri on '&' fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset parts [split $uri "&"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_split_uri_on_question_and_ampersand(self):
        """Splitting HTTP::uri on '?&' fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset parts [split $uri "?&"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "HTTP::query" in ws[0].message

    def test_inline_command_substitution(self):
        """Inline [HTTP::uri] in split fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set parts [split [HTTP::uri] "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1

    def test_split_non_uri_clean(self):
        """Splitting a non-HTTP::uri variable does not fire IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set x "foo?bar"\nset parts [split $x "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_split_http_path_clean(self):
        """Splitting HTTP::path on '?' does not fire IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set p [HTTP::path]\nset parts [split $p "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_split_uri_on_slash_clean(self):
        """Splitting HTTP::uri on '/' does not fire IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset parts [split $uri "/"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_copy_propagation(self):
        """Tracing through a variable copy still detects HTTP::uri."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset copy $uri\nset parts [split $copy "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1

    def test_tcl_dialect_clean(self):
        """Non-iRules dialect does not fire IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl8.6")
        source = 'set x "foo"\nset parts [split $x "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_uri_setter_not_flagged(self):
        """HTTP::uri setter form (with arg) is not a getter origin."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'HTTP::uri "/new"\nset parts [split [HTTP::uri /path] "?"]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0


class TestIrule3103ExprOperators:
    """IRULE3103: expression operators on HTTP::uri suggesting HTTP::path/query."""

    def test_starts_with_path_pattern(self):
        """HTTP::uri starts_with '/api' fires IRULE3103 suggesting HTTP::path."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] starts_with "/api" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "starts_with" in ws[0].message

    def test_starts_with_via_variable(self):
        """Variable tracing: $uri starts_with '/api' fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nif { $uri starts_with "/api" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_ends_with_extension(self):
        """HTTP::uri ends_with '.html' fires IRULE3103 suggesting HTTP::path."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] ends_with ".html" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_contains_query_param(self):
        """HTTP::uri contains '&key=' fires IRULE3103 suggesting HTTP::query."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] contains "&key=" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_contains_equals_query(self):
        """HTTP::uri contains 'param=val' fires IRULE3103 suggesting HTTP::query."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] contains "user=test" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_matches_glob_path(self):
        """HTTP::uri matches_glob '/api/*' fires IRULE3103 suggesting HTTP::path."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] matches_glob "/api/*" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_matches_glob_query(self):
        """HTTP::uri matches_glob '*&key=*' fires IRULE3103 suggesting HTTP::query."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] matches_glob "*&key=*" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_non_uri_clean(self):
        """Non-HTTP::uri variable in expression does not fire IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set p [HTTP::path]\nif { $p starts_with "/api" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_ambiguous_operand_clean(self):
        """Ambiguous operand (not clearly path or query) does not fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] contains "something" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0


class TestIrule3103StringMatch:
    """IRULE3103: string match / string first on HTTP::uri."""

    def test_string_match_path_pattern(self):
        """string match '/api/*' $uri fires IRULE3103 suggesting HTTP::path."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset m [string match "/api/*" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_string_match_query_pattern(self):
        """string match '*&key=*' $uri fires IRULE3103 suggesting HTTP::query."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset m [string match "*&key=*" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_string_first_question_mark(self):
        """string first '?' $uri fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset pos [string first "?" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message or "HTTP::query" in ws[0].message

    def test_string_match_non_uri_clean(self):
        """string match on non-HTTP::uri does not fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set p [HTTP::path]\nset m [string match "/api/*" $p]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_string_match_ambiguous_pattern_clean(self):
        """string match with ambiguous pattern does not fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset m [string match "*something*" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_string_match_in_if_condition(self):
        """[string match ...] inside if condition fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nif { [string match "/static/*" $uri] } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_string_first_in_if_condition(self):
        """[string first "?" ...] inside if condition fires IRULE3103."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nif { [string first "?" $uri] >= 0 } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1


class TestIrule3103EdgeCases:
    """IRULE3103: edge cases for glob/regex classifiers and SSA correctness."""

    def test_glob_question_mark_is_wildcard_not_query(self):
        """Glob ? is a single-char wildcard — classified as path, not query."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        # /api/?? uses ? as glob wildcard, should be path-like (not query)
        source = 'set uri [HTTP::uri]\nset m [string match "/api/??" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_glob_bare_question_not_query(self):
        """Glob with bare ? wildcard and no path prefix — no fire (ambiguous)."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset m [string match "??" $uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_glob_question_path_with_wildcard(self):
        """Glob /api/? should not be classified as query-like."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] matches_glob "/api/?" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        # Should fire as path (starts with /) not query
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_regex_question_is_quantifier_not_query(self):
        """Regex ? is a quantifier, not a query delimiter — no fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'if { [HTTP::uri] matches_regex "^/api/v[0-9]+/?$" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        # /api/v[0-9]+/? is path-like (starts with /), ? is a quantifier
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_regex_escaped_question_is_query(self):
        r"""Regex \? is a literal question mark — signals query matching."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = r'if { [HTTP::uri] matches_regex "\\?key=" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message

    def test_later_reassignment_no_false_positive(self):
        """Variable reassigned to URI after expr check should not fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri "foo"\nset ok [expr { $uri starts_with "/api" }]\nset uri [HTTP::uri]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_string_match_nocase_in_if_condition(self):
        """[string match -nocase ...] inside if condition is detected."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            'set uri [HTTP::uri]\nif { [string match -nocase "/STATIC/*" $uri] } { log local0. x }'
        )
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_split_separator_from_constant_variable(self):
        """SCCP-resolved separator variable still triggers split warning."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nset sep "?"\nset parts [split $uri $sep]'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "HTTP::query" in ws[0].message

    def test_phi_mixed_uri_and_non_uri_no_warning(self):
        """Mixed-origin phi node (URI + non-URI) must not produce a warning."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            "set cond 0\n"
            "if { $cond } {\n"
            "    set candidate [HTTP::uri]\n"
            "} else {\n"
            '    set candidate "/not-uri"\n'
            "}\n"
            'set out [string match "/api/*" $candidate]'
        )
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 0

    def test_string_first_with_ampersand_in_if_condition(self):
        """[string first "&" ...] in if condition should be detected."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = 'set uri [HTTP::uri]\nif { [string first "&" $uri] >= 0 } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "HTTP::query" in ws[0].message

    def test_split_separator_from_copied_constant_variable(self):
        """SCCP separator propagation through variable copy remains detectable."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            'set uri [HTTP::uri]\nset sep "?"\nset sep_copy $sep\nset parts [split $uri $sep_copy]'
        )
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message
        assert "HTTP::query" in ws[0].message

    def test_nested_boolean_expression_emits_one_hit_per_uri_use(self):
        """Nested expressions should emit one warning per concrete URI-pattern use."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            "set uri [HTTP::uri]\n"
            'if { ([string match "/api/*" $uri] && ([HTTP::uri] contains "&id=")) } {\n'
            "    log local0. ok\n"
            "}"
        )
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 2
        sinks = sorted(warning.sink_command for warning in ws)
        assert sinks == ["contains", "string match"]

    def test_phi_both_branches_uri_still_warns(self):
        """When all phi inputs are URI-derived, warning should still be produced."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = (
            "set cond 1\n"
            "if { $cond } {\n"
            "    set candidate [HTTP::uri]\n"
            "} else {\n"
            "    set tmp [HTTP::uri]\n"
            "    set candidate $tmp\n"
            "}\n"
            'set out [string match "/api/*" $candidate]'
        )
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::path" in ws[0].message

    def test_regex_escaped_ampersand_is_query(self):
        r"""Regex \& is a literal ampersand and should classify as query-like."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        source = r'if { [HTTP::uri] matches_regex "key\\&id" } { log local0. x }'
        ws = _taint_warnings(source, "IRULE3103")
        assert len(ws) == 1
        assert "HTTP::query" in ws[0].message
